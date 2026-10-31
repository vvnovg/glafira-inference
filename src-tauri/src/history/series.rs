//! Сборка точек истории (спека I1c §4): матрицы запросов выравниваются по меткам времени окна,
//! точка — `ServerMetrics` той же формы, что в окне «Сейчас», поэтому графики не меняются.

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::history::client::RangeSeries;
use crate::history::promql::{Query, QueryKey};
use crate::history::window::Range;
use crate::metrics::derive::{Gauges, Latency, LatencySet, Rates, ServerMetrics};
use crate::servers::adapters::Canonical;

/// Точка истории — как точка буфера «Сейчас» во фронтенде: время (мс) и метрики.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HistoryPoint {
    pub at: u64,
    pub m: ServerMetrics,
}

/// Ряд, который не удалось прочитать: график и ошибка.
#[derive(Debug, Clone, Serialize)]
pub struct Partial {
    pub chart: &'static str,
    pub error: AppError,
}

/// Запрос и его результат: ряды или ошибка.
pub type QueryResult = (Query, AppResult<Vec<RangeSeries>>);

/// Значения ряда по метке времени (с); у разбитого ряда — по значению метки.
type Values = HashMap<u64, Option<f64>>;

fn values(series: &RangeSeries) -> Values {
    series.points.iter().copied().collect()
}

/// Значение по ключу ряда в момент `t`.
struct Table {
    plain: HashMap<String, Values>,
    grouped: HashMap<String, BTreeMap<String, Values>>,
}

fn key_id(key: QueryKey) -> String {
    format!("{key:?}")
}

impl Table {
    fn get(&self, key: QueryKey, t: u64) -> Option<f64> {
        self.plain.get(&key_id(key))?.get(&t).copied().flatten()
    }

    fn latency(&self, c: Canonical, t: u64) -> Option<Latency> {
        let q = |q: f64| {
            self.get(QueryKey::Quantile { canonical: c, q }, t)
                .map(|v| v * 1000.0)
        };
        let (p50, p95, p99) = (q(0.5), q(0.95), q(0.99));
        // Число наблюдений история не запрашивает: `count` у её точек — 0.
        (p50.is_some() || p95.is_some() || p99.is_some()).then_some(Latency {
            p50,
            p95,
            p99,
            count: 0.0,
        })
    }
}

/// Точки окна `range` из результатов запросов и список рядов, которые не прочитались. Точка есть
/// на каждый шаг окна; без значения — `None` (на графике разрыв). Метрика, у которой нет ни
/// одного значения во всём окне, — в `missing` каждой точки (как отсутствующая серия в I1b).
pub fn assemble(range: &Range, results: Vec<QueryResult>) -> (Vec<HistoryPoint>, Vec<Partial>) {
    let mut table = Table {
        plain: HashMap::new(),
        grouped: HashMap::new(),
    };
    let mut partial: Vec<Partial> = Vec::new();
    // Метрика → есть ли у неё хоть одно значение (у KV — в любом из рядов: максимум, среднее).
    let mut present: BTreeMap<Canonical, bool> = BTreeMap::new();
    for (query, result) in results {
        let series = match result {
            Ok(s) => s,
            Err(error) => {
                let chart = query.key.chart();
                if !partial.iter().any(|p| p.chart == chart) {
                    partial.push(Partial { chart, error });
                }
                continue;
            }
        };
        let any = series
            .iter()
            .any(|s| s.points.iter().any(|(_, v)| v.is_some()));
        *present.entry(query.key.canonical()).or_default() |= any;
        match query.group_by {
            Some(label) => {
                let groups = table.grouped.entry(key_id(query.key)).or_default();
                for s in &series {
                    let name = s
                        .labels
                        .get(label)
                        .cloned()
                        .unwrap_or_else(|| "unknown".into());
                    groups.insert(name, values(s));
                }
            }
            None => {
                if let Some(s) = series.first() {
                    table.plain.insert(key_id(query.key), values(s));
                }
            }
        }
    }
    let missing: Vec<Canonical> = present
        .into_iter()
        .filter(|(_, any)| !any)
        .map(|(c, _)| c)
        .collect();

    let finished = table.grouped.get(&key_id(QueryKey::FinishedByReason));
    let mut points = Vec::new();
    let mut t = range.start;
    while t <= range.end {
        let get = |k: QueryKey| table.get(k, t);
        let finished_by_reason: BTreeMap<String, f64> = finished
            .map(|g| {
                g.iter()
                    .filter_map(|(reason, v)| Some((reason.clone(), (*v.get(&t)?)?)))
                    .collect()
            })
            .unwrap_or_default();
        let requests =
            (!finished_by_reason.is_empty()).then(|| finished_by_reason.values().sum::<f64>());
        let preemptions = get(QueryKey::Preemptions);
        let latency = LatencySet {
            ttft: table.latency(Canonical::Ttft, t),
            itl: table.latency(Canonical::Itl, t),
            e2e: table.latency(Canonical::E2e, t),
            queue_time: table.latency(Canonical::QueueTime, t),
        };
        let m = ServerMetrics {
            at: t * 1000,
            interval_sec: Some(range.step as f64),
            reset: false,
            engines: 0,
            gauges: Gauges {
                requests_running: get(QueryKey::RequestsRunning),
                requests_waiting: get(QueryKey::RequestsWaiting),
                kv_usage: get(QueryKey::KvUsage),
                kv_usage_avg: get(QueryKey::KvUsageAvg),
            },
            rates: Some(Rates {
                prompt_tokens: get(QueryKey::PromptTokens),
                generation_tokens: get(QueryKey::GenerationTokens),
                requests,
                preemptions,
                aborted: get(QueryKey::Aborted),
                finished_by_reason,
            }),
            prefix_cache_hit_ratio: get(QueryKey::PrefixCacheHitRatio),
            latency_1m: latency.clone(),
            latency,
            // Скорость за интервал `rate` (не меньше минуты) — в вытеснения за минуту.
            preemptions_1m: preemptions.map(|r| r * 60.0),
            totals: BTreeMap::new(),
            missing: missing.clone(),
            series: BTreeMap::new(),
        };
        points.push(HistoryPoint { at: t * 1000, m });
        t += range.step;
    }
    (points, partial)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::client::tests::snapshot;
    use crate::history::client::{parse_api, parse_matrix};
    use crate::history::promql::queries;
    use crate::history::window::duration;
    use crate::servers::adapters::adapter;
    use crate::servers::client::HttpReply;
    use crate::servers::detect::DetectedKind;

    /// Окно снимка `range-<цель>.json` и результаты шаблонов по настоящим ответам.
    fn captured(name: &str, kind: DetectedKind, selector: &str) -> (Range, Vec<QueryResult>) {
        let doc: serde_json::Value =
            serde_json::from_str(&snapshot(&format!("range-{name}.json"))).unwrap();
        let range = Range {
            start: doc["start"].as_u64().unwrap(),
            end: doc["end"].as_u64().unwrap(),
            step: doc["step"].as_u64().unwrap(),
            rate: 60,
        };
        assert_eq!(doc["rate"], duration(range.rate));
        let answers: HashMap<String, serde_json::Value> = doc["queries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|q| {
                (
                    q["expr"].as_str().unwrap().to_string(),
                    q["response"].clone(),
                )
            })
            .collect();
        let results = queries(adapter(kind).metric_map(), selector, "1m")
            .into_iter()
            .filter_map(|q| {
                let answer = answers.get(&q.expr)?;
                let reply = HttpReply {
                    status: 200,
                    content_type: None,
                    body: answer.to_string().into_bytes(),
                    truncated: false,
                };
                let parsed =
                    parse_api(&reply, "/api/v1/query_range").and_then(|d| parse_matrix(&d));
                Some((q, parsed))
            })
            .collect();
        (range, results)
    }

    #[test]
    fn vllm_window_from_a_real_prometheus() {
        let (range, results) = captured(
            "vllm-8000",
            DetectedKind::Vllm,
            r#"job="vllm",instance="127.0.0.1:8000""#,
        );
        assert_eq!(results.len(), 20, "все шаблоны сняты");
        let (points, partial) = assemble(&range, results);
        assert!(partial.is_empty());
        assert_eq!(points.len(), 61);
        assert!(points.windows(2).all(|w| w[1].at - w[0].at == 15_000));
        // До запуска Prometheus значений нет: разрыв, а не ноль.
        let first = &points[0].m;
        assert_eq!(first.gauges.kv_usage, None);
        assert_eq!(first.rates.as_ref().unwrap().prompt_tokens, None);
        assert!(first.missing.is_empty(), "{:?}", first.missing);
        let last = &points[60].m;
        let rates = last.rates.as_ref().unwrap();
        // Мок растит счётчики на 10 % снимка за опрос раз в 5 с: 20,8 токена/с на входе.
        assert!(
            (rates.prompt_tokens.unwrap() - 20.82).abs() < 0.05,
            "{rates:?}"
        );
        assert!((rates.generation_tokens.unwrap() - 10.1).abs() < 0.05);
        assert_eq!(
            rates.finished_by_reason.keys().collect::<Vec<_>>(),
            ["abort", "error", "length", "repetition", "stop"]
        );
        assert!((rates.requests.unwrap() - 0.6).abs() < 0.01, "{rates:?}");
        let ttft = last.latency.ttft.as_ref().unwrap();
        assert!((ttft.p95.unwrap() - 235.0).abs() < 1.0, "{ttft:?}");
        assert_eq!(last.latency_1m, last.latency);
        assert_eq!(last.latency.queue_time.as_ref().unwrap().p50, None);
        assert!(last.gauges.kv_usage.unwrap() > 0.0);
        assert_eq!(last.gauges.kv_usage, last.gauges.kv_usage_avg);
        assert!((last.prefix_cache_hit_ratio.unwrap() - 0.4457).abs() < 0.001);
        assert_eq!(last.preemptions_1m, Some(0.0));
        assert_eq!((last.engines, last.reset), (0, false));
    }

    #[test]
    fn sglang_reasons_without_a_label_are_unknown() {
        let (range, results) = captured(
            "sglang-30000",
            DetectedKind::Sglang,
            r#"job="sglang",instance="127.0.0.1:30000""#,
        );
        let (points, partial) = assemble(&range, results);
        assert!(partial.is_empty());
        let rates = points[60].m.rates.clone().unwrap();
        assert_eq!(
            rates.finished_by_reason.keys().collect::<Vec<_>>(),
            ["unknown"]
        );
        assert!(rates.aborted.unwrap() > 0.0);
        assert!(points[60].m.preemptions_1m.unwrap() > 0.0);
    }

    #[test]
    fn restart_has_no_negative_rates() {
        let (range, results) = captured(
            "vllm-8003",
            DetectedKind::Vllm,
            r#"job="vllm",instance="127.0.0.1:8003""#,
        );
        assert_eq!(results.len(), 2);
        let (points, _) = assemble(&range, results);
        let rates: Vec<f64> = points
            .iter()
            .filter_map(|p| p.m.rates.as_ref()?.prompt_tokens)
            .collect();
        assert!(rates.len() > 40);
        assert!(rates.iter().all(|r| *r >= 0.0), "{rates:?}");
        // Этих рядов не снимали: метрики — «нет данных» (missing не трогается).
        assert_eq!(points[60].m.gauges.kv_usage, None);
    }

    #[test]
    fn failed_query_is_partial_and_empty_series_is_missing() {
        let range = Range {
            start: 1_000,
            end: 1_030,
            step: 15,
            rate: 60,
        };
        let map = adapter(DetectedKind::Vllm).metric_map();
        let qs = queries(map, r#"job="j""#, "1m");
        let series = |v: Option<f64>| {
            vec![RangeSeries {
                labels: BTreeMap::new(),
                points: vec![(1_000, v), (1_015, v), (1_030, v)],
            }]
        };
        let results = qs
            .into_iter()
            .map(|q| {
                let r = match q.key {
                    QueryKey::Quantile {
                        canonical: Canonical::Ttft,
                        ..
                    } => Err(crate::history::client::parse_api(
                        &HttpReply {
                            status: 503,
                            content_type: None,
                            body: br#"{"status":"error","errorType":"timeout","error":"x"}"#
                                .to_vec(),
                            truncated: false,
                        },
                        "/q",
                    )
                    .unwrap_err()),
                    QueryKey::Preemptions => Ok(Vec::new()),
                    QueryKey::KvUsageAvg => Ok(series(None)),
                    _ => Ok(series(Some(1.0))),
                };
                (q, r)
            })
            .collect();
        let (points, partial) = assemble(&range, results);
        assert_eq!(points.len(), 3);
        assert_eq!(partial.len(), 1, "три квантиля TTFT — одна запись");
        assert_eq!(partial[0].chart, "ttft");
        assert_eq!(partial[0].error.key, "errors.history.timeout");
        let m = &points[1].m;
        assert_eq!(m.latency.ttft, None);
        assert_eq!(m.missing, [Canonical::PreemptionsTotal]);
        assert_eq!(m.gauges.kv_usage, Some(1.0));
        assert_eq!(m.gauges.kv_usage_avg, None);
        let json = serde_json::to_value(&partial[0]).unwrap();
        assert_eq!(json["chart"], "ttft");
        assert_eq!(json["error"]["key"], "errors.history.timeout");
    }
}
