//! Приросты счётчиков карт за последнюю минуту опросов (спека I1d §4): ошибки ECC, повторы PCIe,
//! ошибки NVLink и доля времени троттлинга. Снимки за минуту хранит пул подключений на каждую
//! инсталляцию, как `servers/live.rs`. Уменьшение счётчика — перезапуск экспортёра или драйвера,
//! прирост 0.

use std::collections::{BTreeMap, HashMap, VecDeque};

use serde::Serialize;

use crate::gpu::fields::Field;
use crate::gpu::source::GpuSnapshot;
use crate::metrics::derive::MINUTE_MS;

/// Предел истории: при опросе из нескольких вкладок снимков за минуту больше обычного.
pub const HISTORY_MAX: usize = 64;

/// Счётчики, у которых считается прирост.
pub const COUNTERS: [Field; 4] = [
    Field::EccDbe,
    Field::EccSbe,
    Field::PcieReplay,
    Field::NvlinkErrors,
];

/// Время троттлинга (нс): прирост — доля времени, %.
pub const VIOLATIONS: [Field; 2] = [Field::PowerViolation, Field::ThermalViolation];

/// Карта в истории: хост, номер, экспортёр.
type CardKey = (String, u32, Option<String>);

/// Счётчики карт на момент опроса.
#[derive(Debug, Clone)]
pub struct Timed {
    pub at_ms: u64,
    counters: HashMap<CardKey, BTreeMap<Field, f64>>,
}

/// Ответ `read_gpu_status`: снимок с приростами, время опроса и длина окна приростов (`None` —
/// первый опрос, приростов нет).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuStatus {
    pub at: u64,
    #[serde(flatten)]
    pub snapshot: GpuSnapshot,
    pub delta_sec: Option<f64>,
}

fn timed(at_ms: u64, snapshot: &GpuSnapshot) -> Timed {
    let mut counters = HashMap::new();
    for host in &snapshot.hosts {
        for c in &host.cards {
            let values: BTreeMap<Field, f64> = COUNTERS
                .iter()
                .chain(&VIOLATIONS)
                .filter_map(|f| c.values.get(f).map(|v| (*f, *v)))
                .collect();
            counters.insert((host.host.clone(), c.index, c.exporter.clone()), values);
        }
    }
    Timed { at_ms, counters }
}

/// Приросты нового снимка относительно самого старого снимка за минуту; снимок уходит в историю,
/// снимки старше минуты отбрасываются.
pub fn record(history: &mut VecDeque<Timed>, snapshot: GpuSnapshot, at_ms: u64) -> GpuStatus {
    let cur = timed(at_ms, &snapshot);
    let base = history
        .iter()
        .find(|t| t.at_ms < at_ms && at_ms - t.at_ms <= MINUTE_MS)
        .cloned();
    let mut snapshot = snapshot;
    let delta_sec = base.as_ref().map(|b| (at_ms - b.at_ms) as f64 / 1000.0);
    if let Some(base) = &base {
        let span_ns = (at_ms - base.at_ms) as f64 * 1e6;
        for host in &mut snapshot.hosts {
            for c in &mut host.cards {
                let key = (host.host.clone(), c.index, c.exporter.clone());
                let Some(old) = base.counters.get(&key) else {
                    continue;
                };
                for f in COUNTERS.iter().chain(&VIOLATIONS) {
                    let (Some(now), Some(was)) = (c.values.get(f), old.get(f)) else {
                        continue;
                    };
                    let grew = if now < was { 0.0 } else { now - was };
                    let v = if VIOLATIONS.contains(f) {
                        (grew / span_ns * 100.0).clamp(0.0, 100.0)
                    } else {
                        grew
                    };
                    c.deltas.insert(*f, v);
                }
            }
        }
    }
    history.push_back(cur);
    while history
        .front()
        .is_some_and(|t| at_ms.saturating_sub(t.at_ms) > MINUTE_MS)
        || history.len() > HISTORY_MAX
    {
        history.pop_front();
    }
    GpuStatus {
        at: at_ms,
        snapshot,
        delta_sec,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::snapshot::tests::{FULL, cards};
    use crate::gpu::source::snapshot;

    const FULL_NEXT: &str = include_str!("testdata/dcgm-4.8.4/full-next.txt");

    fn snap(text: &str) -> GpuSnapshot {
        snapshot(cards(text, "x"), Vec::new())
    }

    #[test]
    fn first_poll_has_no_deltas_the_next_one_counts_growth() {
        let mut h = VecDeque::new();
        let first = record(&mut h, snap(FULL), 100_000);
        assert_eq!(first.delta_sec, None);
        assert!(first.snapshot.hosts[0].cards[0].deltas.is_empty());
        let next = record(&mut h, snap(FULL_NEXT), 105_000);
        assert_eq!(next.delta_sec, Some(5.0));
        let d = &next.snapshot.hosts[0].cards[0].deltas;
        // Мок на опросе 5 против 4: повторы PCIe +3, ошибки NVLink +3 (три счётчика по +1),
        // ECC SBE +1, DBE без изменений; троттлинг по мощности +2 с, по температуре +3 с за 5 с.
        assert_eq!(d[&Field::PcieReplay], 3.0);
        assert_eq!(d[&Field::NvlinkErrors], 3.0);
        assert_eq!(d[&Field::EccSbe], 1.0);
        assert_eq!(d[&Field::EccDbe], 0.0);
        assert_eq!(d[&Field::PowerViolation], 40.0);
        assert_eq!(d[&Field::ThermalViolation], 60.0);
        assert_eq!(
            next.snapshot.hosts[0].cards[1].deltas[&Field::EccSbe],
            0.0,
            "ECC — только у карты 0"
        );
    }

    #[test]
    fn a_counter_that_went_down_is_a_restart_and_old_snapshots_expire() {
        let mut h = VecDeque::new();
        record(&mut h, snap(FULL_NEXT), 0);
        let restarted = record(&mut h, snap(FULL), 5_000);
        assert_eq!(
            restarted.snapshot.hosts[0].cards[0].deltas[&Field::PcieReplay],
            0.0
        );
        let late = record(&mut h, snap(FULL_NEXT), 200_000);
        assert_eq!(late.delta_sec, None, "снимки старше минуты отброшены");
        assert_eq!(h.len(), 1);
    }

    #[test]
    fn json_flattens_the_snapshot() {
        let mut h = VecDeque::new();
        record(&mut h, snap(FULL), 0);
        let v = serde_json::to_value(record(&mut h, snap(FULL_NEXT), 5_000)).unwrap();
        assert_eq!(v["at"], 5000);
        assert_eq!(v["deltaSec"], 5.0);
        assert_eq!(v["hosts"][0]["cards"][0]["deltas"]["pcieReplay"], 3.0);
        assert_eq!(v["missing"], serde_json::json!([]));
        assert_eq!(v["errors"], serde_json::json!([]));
    }
}
