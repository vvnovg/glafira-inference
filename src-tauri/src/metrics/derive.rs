//! Производные величины (спец. I1b §4): скорости в секунду, доля попаданий в prefix-cache и
//! перцентили задержек по разнице двух снимков. Перцентиль — как `histogram_quantile`
//! Prometheus: линейная интерполяция внутри корзины. Уменьшение любого счётчика — перезапуск
//! сервера: производных за интервал нет (`reset`), снимок становится новой базой.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::metrics::canonical::{Histogram, Snapshot};
use crate::servers::adapters::Canonical;

/// Окно перцентилей «за минуту» (плитки «Обзора»).
pub const MINUTE_MS: u64 = 60_000;

/// Снимок со временем получения (мс Unix).
#[derive(Debug, Clone, PartialEq)]
pub struct Timed {
    pub at_ms: u64,
    pub snap: Snapshot,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Latency {
    pub p50: Option<f64>,
    pub p95: Option<f64>,
    pub p99: Option<f64>,
    /// Наблюдений за окно.
    pub count: f64,
}

/// Задержки в миллисекундах; `None` — серии нет или окно не посчитать.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatencySet {
    pub ttft: Option<Latency>,
    pub itl: Option<Latency>,
    pub e2e: Option<Latency>,
    pub queue_time: Option<Latency>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gauges {
    pub requests_running: Option<f64>,
    pub requests_waiting: Option<f64>,
    pub kv_usage: Option<f64>,
    pub kv_usage_avg: Option<f64>,
}

/// Скорости в секунду за интервал.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rates {
    pub prompt_tokens: Option<f64>,
    pub generation_tokens: Option<f64>,
    pub requests: Option<f64>,
    pub preemptions: Option<f64>,
    pub aborted: Option<f64>,
    pub finished_by_reason: BTreeMap<String, f64>,
}

/// Ответ `read_server_metrics`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerMetrics {
    pub at: u64,
    /// Секунд с предыдущего снимка; `None` у первого.
    pub interval_sec: Option<f64>,
    /// Счётчики уменьшились — сервер перезапущен, производных за интервал нет.
    pub reset: bool,
    pub engines: usize,
    pub gauges: Gauges,
    pub rates: Option<Rates>,
    pub prefix_cache_hit_ratio: Option<f64>,
    pub latency: LatencySet,
    /// То же за последнюю минуту — по самому старому снимку не старше 60 с.
    #[serde(rename = "latency1m")]
    pub latency_1m: LatencySet,
    pub totals: BTreeMap<Canonical, f64>,
    pub missing: Vec<Canonical>,
    pub series: BTreeMap<Canonical, String>,
}

/// Перцентиль по накопленным корзинам `(le, count)`, как `histogram_quantile`.
pub fn quantile(q: f64, buckets: &[(f64, f64)]) -> Option<f64> {
    let total = buckets.last()?.1;
    if total <= 0.0 {
        return None;
    }
    let rank = q * total;
    let (mut lower, mut below) = (0.0, 0.0);
    for &(le, cum) in buckets {
        if cum >= rank {
            if le.is_infinite() {
                return Some(lower);
            }
            if cum == below {
                return Some(le);
            }
            return Some(lower + (le - lower) * (rank - below) / (cum - below));
        }
        lower = le;
        below = cum;
    }
    Some(lower)
}

/// Разность гистограмм: корзины по границам `le`; граница, которой не было, — с нуля.
fn diff(cur: &Histogram, prev: &Histogram) -> Histogram {
    let before = |le: f64| {
        prev.buckets
            .iter()
            .find(|(l, _)| *l == le)
            .map_or(0.0, |(_, c)| *c)
    };
    Histogram {
        buckets: cur
            .buckets
            .iter()
            .map(|&(le, c)| (le, c - before(le)))
            .collect(),
        sum: cur.sum - prev.sum,
        count: cur.count - prev.count,
    }
}

fn latency(h: &Histogram) -> Latency {
    let ms = |q| quantile(q, &h.buckets).map(|v| v * 1000.0);
    Latency {
        p50: ms(0.5),
        p95: ms(0.95),
        p99: ms(0.99),
        count: h.count,
    }
}

fn latency_set(cur: &Snapshot, base: &Snapshot) -> LatencySet {
    let one = |c: Canonical| {
        let (Some(a), Some(b)) = (cur.histograms.get(&c), base.histograms.get(&c)) else {
            return None;
        };
        Some(latency(&diff(a, b)))
    };
    LatencySet {
        ttft: one(Canonical::Ttft),
        itl: one(Canonical::Itl),
        e2e: one(Canonical::E2e),
        queue_time: one(Canonical::QueueTime),
    }
}

/// Счётчики снимка как плоский список: сброс — если хоть один уменьшился.
fn counter_values(s: &Snapshot) -> BTreeMap<String, f64> {
    let mut m: BTreeMap<String, f64> = s
        .counters
        .iter()
        .map(|(c, v)| (format!("{c:?}"), *v))
        .collect();
    for (c, g) in &s.groups {
        for (k, v) in g {
            m.insert(format!("{c:?}/{k}"), *v);
        }
    }
    for (c, h) in &s.histograms {
        m.insert(format!("{c:?}/count"), h.count);
    }
    m
}

pub fn is_reset(cur: &Snapshot, prev: &Snapshot) -> bool {
    let before = counter_values(prev);
    counter_values(cur)
        .iter()
        .any(|(k, v)| before.get(k).is_some_and(|b| v < b))
}

fn rates(cur: &Snapshot, prev: &Snapshot, dt: f64) -> Rates {
    let rate = |c: Canonical| Some((cur.counters.get(&c)? - prev.counters.get(&c)?) / dt);
    let mut finished_by_reason = BTreeMap::new();
    let mut requests = None;
    if let (Some(a), Some(b)) = (
        cur.groups.get(&Canonical::RequestSuccessTotal),
        prev.groups.get(&Canonical::RequestSuccessTotal),
    ) {
        for (k, v) in a {
            finished_by_reason.insert(k.clone(), (v - b.get(k).copied().unwrap_or(0.0)) / dt);
        }
        requests = Some(finished_by_reason.values().sum());
    }
    Rates {
        prompt_tokens: rate(Canonical::PromptTokensTotal),
        generation_tokens: rate(Canonical::GenerationTokensTotal),
        requests,
        preemptions: rate(Canonical::PreemptionsTotal),
        aborted: rate(Canonical::RequestAbortedTotal),
        finished_by_reason,
    }
}

fn hit_ratio(cur: &Snapshot, prev: &Snapshot) -> Option<f64> {
    let d = |c: Canonical| Some(cur.counters.get(&c)? - prev.counters.get(&c)?);
    let (q, h) = (
        d(Canonical::PrefixCacheQueriesTotal)?,
        d(Canonical::PrefixCacheHitsTotal)?,
    );
    (q > 0.0).then(|| h / q)
}

/// Производные текущего снимка по истории (старые — первыми, текущий в неё не входит).
pub fn derive(history: &[Timed], cur: &Timed) -> ServerMetrics {
    let s = &cur.snap;
    let prev = history.last().filter(|p| p.at_ms < cur.at_ms);
    let reset = prev.is_some_and(|p| is_reset(s, &p.snap));
    let (interval_sec, rates_, ratio, lat) = match prev {
        Some(p) if !reset => {
            let dt = (cur.at_ms - p.at_ms) as f64 / 1000.0;
            (
                Some(dt),
                Some(rates(s, &p.snap, dt)),
                hit_ratio(s, &p.snap),
                latency_set(s, &p.snap),
            )
        }
        Some(p) => (
            Some((cur.at_ms - p.at_ms) as f64 / 1000.0),
            None,
            None,
            LatencySet::default(),
        ),
        None => (None, None, None, LatencySet::default()),
    };
    let minute_base = (!reset)
        .then(|| {
            history.iter().find(|t| {
                t.at_ms < cur.at_ms && cur.at_ms - t.at_ms <= MINUTE_MS && !is_reset(s, &t.snap)
            })
        })
        .flatten();
    let mut totals = s.counters.clone();
    if let Some(g) = s.groups.get(&Canonical::RequestSuccessTotal) {
        totals.insert(Canonical::RequestSuccessTotal, g.values().sum());
    }
    ServerMetrics {
        at: cur.at_ms,
        interval_sec,
        reset,
        engines: s.engines,
        gauges: Gauges {
            requests_running: s.gauges.get(&Canonical::RequestsRunning).copied(),
            requests_waiting: s.gauges.get(&Canonical::RequestsWaiting).copied(),
            kv_usage: s.gauges.get(&Canonical::KvUsage).copied(),
            kv_usage_avg: s.gauge_avg.get(&Canonical::KvUsage).copied(),
        },
        rates: rates_,
        prefix_cache_hit_ratio: ratio,
        latency: lat,
        latency_1m: minute_base.map_or_else(LatencySet::default, |b| latency_set(s, &b.snap)),
        totals,
        missing: s.missing.clone(),
        series: s.series.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::canonical::snapshot;
    use crate::metrics::parse::parse;
    use crate::servers::adapters::adapter;
    use crate::servers::detect::DetectedKind;

    fn vllm(at_ms: u64, text: &str) -> Timed {
        Timed {
            at_ms,
            snap: snapshot(
                adapter(DetectedKind::Vllm).metric_map(),
                &parse(text).unwrap(),
            ),
        }
    }

    const BEFORE: &str = include_str!("../servers/testdata/vllm-0.32.0/metrics-before.txt");
    const AFTER: &str = include_str!("../servers/testdata/vllm-0.32.0/metrics-after.txt");

    #[test]
    fn quantile_matches_prometheus_histogram_quantile() {
        let b = [(0.1, 10.0), (0.5, 30.0), (1.0, 40.0), (f64::INFINITY, 40.0)];
        assert!((quantile(0.5, &b).unwrap() - 0.3).abs() < 1e-12);
        assert!((quantile(0.99, &b).unwrap() - 0.98).abs() < 1e-12);
        assert!((quantile(0.1, &b).unwrap() - 0.04).abs() < 1e-12);
        // Ранг в корзине +Inf — нижняя граница этой корзины.
        assert_eq!(
            quantile(0.95, &[(1.0, 5.0), (f64::INFINITY, 10.0)]),
            Some(1.0)
        );
        assert_eq!(quantile(0.5, &[(1.0, 0.0), (f64::INFINITY, 0.0)]), None);
        assert_eq!(quantile(0.5, &[]), None);
    }

    #[test]
    fn first_snapshot_has_gauges_but_no_rates() {
        let m = derive(&[], &vllm(1_000, AFTER));
        assert_eq!(m.interval_sec, None);
        assert_eq!(m.rates, None);
        assert_eq!(m.latency, LatencySet::default());
        assert_eq!(m.gauges.requests_running, Some(0.0));
        assert_eq!(m.totals[&Canonical::RequestSuccessTotal], 30.0);
        assert_eq!(m.engines, 1);
    }

    #[test]
    fn rates_ratio_and_latency_over_the_interval_of_a_real_server() {
        let m = derive(&[vllm(0, BEFORE)], &vllm(10_000, AFTER));
        assert_eq!(m.interval_sec, Some(10.0));
        assert!(!m.reset);
        let r = m.rates.unwrap();
        assert_eq!(r.prompt_tokens, Some(104.1));
        assert_eq!(r.generation_tokens, Some(50.5));
        assert_eq!(r.requests, Some(3.0));
        assert_eq!(r.finished_by_reason["stop"], 2.5);
        assert_eq!(r.aborted, Some(0.0));
        let ratio = m.prefix_cache_hit_ratio.unwrap();
        assert!((0.0..=1.0).contains(&ratio));
        let ttft = m.latency.ttft.clone().unwrap();
        assert_eq!(ttft.count, 30.0);
        let (p50, p95, p99) = (ttft.p50.unwrap(), ttft.p95.unwrap(), ttft.p99.unwrap());
        assert!(0.0 < p50 && p50 <= p95 && p95 <= p99, "{p50} {p95} {p99}");
        assert_eq!(
            m.latency_1m, m.latency,
            "единственный снимок минуты — тот же интервал"
        );
    }

    #[test]
    fn no_observations_in_the_interval_is_none_not_zero() {
        let m = derive(&[vllm(0, AFTER)], &vllm(5_000, AFTER));
        let ttft = m.latency.ttft.unwrap();
        assert_eq!((ttft.count, ttft.p50), (0.0, None));
        assert_eq!(m.prefix_cache_hit_ratio, None);
        assert_eq!(m.rates.unwrap().prompt_tokens, Some(0.0));
    }

    #[test]
    fn counter_going_down_is_a_server_restart() {
        let m = derive(&[vllm(0, AFTER)], &vllm(5_000, BEFORE));
        assert!(m.reset);
        assert_eq!(m.rates, None);
        assert_eq!(m.latency_1m, LatencySet::default());
        assert_eq!(m.interval_sec, Some(5.0));
    }

    #[test]
    fn minute_window_never_spans_a_server_restart() {
        // Перезапуск был на 10 с: снимок 0 с — до него, 10 с — после.
        let m = derive(
            &[vllm(0, AFTER), vllm(10_000, BEFORE)],
            &vllm(20_000, BEFORE),
        );
        assert!(!m.reset);
        assert_eq!(m.latency_1m.ttft.as_ref().unwrap().count, 0.0);
    }

    #[test]
    fn minute_window_uses_the_oldest_snapshot_within_sixty_seconds() {
        let old = vllm(0, BEFORE);
        let recent = vllm(55_000, AFTER);
        let m = derive(&[old.clone(), recent.clone()], &vllm(60_000, AFTER));
        assert_eq!(m.latency.ttft.as_ref().unwrap().count, 0.0);
        assert_eq!(m.latency_1m.ttft.as_ref().unwrap().count, 30.0);
        let m = derive(&[old, recent], &vllm(61_000, AFTER));
        assert_eq!(
            m.latency_1m.ttft.as_ref().unwrap().count,
            0.0,
            "снимок старше минуты не берётся"
        );
    }

    #[test]
    fn json_shape_for_the_frontend() {
        let m = derive(&[vllm(0, BEFORE)], &vllm(10_000, AFTER));
        let j = serde_json::to_value(&m).unwrap();
        assert!(j["latency1m"]["ttft"]["p95"].is_number());
        assert_eq!(j["gauges"]["kvUsage"], 0.0);
        assert_eq!(j["series"]["kvUsage"], "vllm:kv_cache_usage_perc");
        assert_eq!(j["totals"]["promptTokensTotal"], 1041.0);
        assert!(j["missing"].as_array().unwrap().is_empty());
    }
}
