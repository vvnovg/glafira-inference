//! Сборка рядов окна истории GPU (спека I1d §4): ряд на карту, точка на каждый шаг окна, значения
//! полей — только те, что есть (на графике разрыв). Активность SM — в процентах, всего памяти —
//! сумма занятой, свободной и зарезервированной.

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;

use crate::error::AppResult;
use crate::gpu::fields::Field;
use crate::gpu::promql::{GpuQuery, chart_of};
use crate::gpu::source::instance_host;
use crate::history::client::RangeSeries;
use crate::history::series::Partial;
use crate::history::window::Range;

/// Запрос окна и его результат: ряды или ошибка.
pub type QueryResult = (GpuQuery, AppResult<Vec<RangeSeries>>);

/// Сколько рядов (карт) может быть в окне: на графике хоста — ряд на карту.
pub const MAX_SERIES: usize = 16;

/// Точка ряда: время (мс) и значения полей.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GpuPoint {
    pub at: u64,
    pub values: BTreeMap<Field, f64>,
}

/// Ряд карты в окне.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuSeries {
    pub host: String,
    pub gpu: u32,
    pub points: Vec<GpuPoint>,
}

/// Хост ряда — как у снимка: `hostname`, `Hostname`, иначе `instance` без порта.
pub fn series_host(labels: &BTreeMap<String, String>) -> Option<String> {
    let get = |k: &str| labels.get(k).filter(|v| !v.is_empty());
    get("hostname")
        .or_else(|| get("Hostname"))
        .cloned()
        .or_else(|| get("instance").map(|i| instance_host(i)))
}

/// Ряды окна `range` из результатов запросов: не больше `MAX_SERIES` карт (по хосту и номеру);
/// третий результат — карт было больше. Сбой запроса — запись `partial` его графика (одна на
/// график).
pub fn assemble(range: &Range, results: Vec<QueryResult>) -> (Vec<GpuSeries>, Vec<Partial>, bool) {
    // (хост, номер) → поле → время (с) → значение.
    let mut table: BTreeMap<(String, u32), HashMap<Field, HashMap<u64, f64>>> = BTreeMap::new();
    let mut partial: Vec<Partial> = Vec::new();
    for (query, result) in results {
        let series = match result {
            Ok(s) => s,
            Err(error) => {
                let chart = chart_of(query.field);
                if !partial.iter().any(|p| p.chart == chart) {
                    partial.push(Partial { chart, error });
                }
                continue;
            }
        };
        for s in series {
            let (Some(host), Some(gpu)) = (
                series_host(&s.labels),
                s.labels.get("gpu").and_then(|g| g.parse::<u32>().ok()),
            ) else {
                continue;
            };
            let field = table
                .entry((host, gpu))
                .or_default()
                .entry(query.field)
                .or_default();
            for (t, v) in s.points {
                if let Some(v) = v.filter(|v| v.is_finite()) {
                    let v = if query.field == Field::SmActive {
                        v * 100.0
                    } else {
                        v
                    };
                    field.insert(t, v);
                }
            }
        }
    }
    let truncated = table.len() > MAX_SERIES;
    let times: Vec<u64> = (range.start..=range.end)
        .step_by(range.step.max(1) as usize)
        .collect();
    let series = table
        .into_iter()
        .take(MAX_SERIES)
        .map(|((host, gpu), fields)| {
            let points = times
                .iter()
                .map(|t| {
                    let mut values: BTreeMap<Field, f64> = fields
                        .iter()
                        .filter_map(|(f, by_t)| by_t.get(t).map(|v| (*f, *v)))
                        .collect();
                    if let (Some(used), Some(free)) =
                        (values.get(&Field::MemUsed), values.get(&Field::MemFree))
                    {
                        let total =
                            used + free + values.get(&Field::MemReserved).copied().unwrap_or(0.0);
                        values.insert(Field::MemTotal, total);
                    }
                    GpuPoint {
                        at: t * 1000,
                        values,
                    }
                })
                .collect();
            GpuSeries { host, gpu, points }
        })
        .collect();
    (series, partial, truncated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{AppError, ErrorCode};
    use crate::history::client::parse_matrix;

    /// Снятое окно `now` хоста gpu-node-1: `{ start, end, step, rate, queries: [{ expr, response }] }`.
    fn captured() -> (Range, Vec<QueryResult>) {
        let raw: serde_json::Value = serde_json::from_str(
            &crate::gpu::source::tests::snapshot_file("range-gpu-node-1.json"),
        )
        .unwrap();
        let n = |k: &str| raw[k].as_u64().unwrap();
        let range = Range {
            start: n("start"),
            end: n("end"),
            step: n("step"),
            rate: n("rate"),
        };
        let queries = crate::gpu::promql::queries(r#"job="dcgm",hostname="gpu-node-1""#, 60);
        let results = raw["queries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|q| {
                let query = queries
                    .iter()
                    .find(|x| x.expr == q["expr"].as_str().unwrap())
                    .cloned()
                    .expect("снят текущий шаблон");
                (query, parse_matrix(&q["response"]["data"]))
            })
            .collect();
        (range, results)
    }

    #[test]
    fn a_captured_window_gives_a_series_per_card() {
        let (range, results) = captured();
        assert_eq!(results.len(), 14);
        let (series, partial, truncated) = assemble(&range, results);
        assert!(partial.is_empty() && !truncated);
        assert_eq!(
            series
                .iter()
                .map(|s| (s.host.as_str(), s.gpu))
                .collect::<Vec<_>>(),
            (0..8).map(|g| ("gpu-node-1", g)).collect::<Vec<_>>()
        );
        let points = &series[0].points;
        assert_eq!(points.len(), 61, "15 мин с шагом 15 с, концы включительно");
        assert_eq!(points[0].at, range.start * 1000);
        let last = points.iter().rev().find(|p| !p.values.is_empty()).unwrap();
        assert_eq!(last.values[&Field::MemTotal], 81559.0);
        assert!(last.values[&Field::SmActive] > 1.0 && last.values[&Field::SmActive] <= 100.0);
        assert!(last.values[&Field::Util] > 0.0 && last.values.contains_key(&Field::Power));
        assert_eq!(last.values[&Field::Throttle], 0.0);
        assert_eq!(last.values[&Field::PcieReplay], 0.0);
        assert!(!last.values.contains_key(&Field::MemTemp) || last.values[&Field::MemTemp] > 0.0);
    }

    #[test]
    fn a_failed_query_is_partial_and_many_cards_are_cut() {
        let (range, mut results) = captured();
        let err = || {
            AppError::localized(
                ErrorCode::Timeout,
                ltext!(
                    "errors.history.timeout",
                    "The history source did not answer in time"
                ),
            )
        };
        for (q, r) in results.iter_mut() {
            if matches!(q.field, Field::MemUsed | Field::MemFree) {
                *r = Err(err());
            }
        }
        let (series, partial, _) = assemble(&range, results);
        assert_eq!(
            partial.iter().map(|p| p.chart).collect::<Vec<_>>(),
            ["memory"]
        );
        assert!(
            series[0]
                .points
                .iter()
                .all(|p| !p.values.contains_key(&Field::MemTotal))
        );

        let one = |host: &str, gpu: u32| RangeSeries {
            labels: [
                ("instance".to_string(), format!("{host}:9400")),
                ("gpu".to_string(), gpu.to_string()),
            ]
            .into(),
            points: vec![(range.start, Some(1.0))],
        };
        let many: Vec<RangeSeries> = (0..20).map(|g| one("10.0.0.5", g)).collect();
        let q = crate::gpu::promql::queries("", 60).remove(0);
        let (series, _, truncated) = assemble(&range, vec![(q, Ok(many))]);
        assert!(truncated);
        assert_eq!(series.len(), MAX_SERIES);
        assert_eq!(series[0].host, "10.0.0.5", "хост из instance без порта");
        assert_eq!(series[0].points[0].values[&Field::Util], 1.0);
    }

    #[test]
    fn json_shape() {
        let p = GpuPoint {
            at: 1000,
            values: [(Field::Util, 50.0)].into(),
        };
        assert_eq!(
            serde_json::to_value(&p).unwrap(),
            serde_json::json!({ "at": 1000, "values": { "util": 50.0 } })
        );
    }
}
