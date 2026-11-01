//! Шаблоны PromQL окон истории GPU (спека I1d §4) — чистые функции над таблицей полей. Ряды
//! группируются по `hostname`, `Hostname`, `instance` и `gpu`: хост карты Rust берёт из ответа
//! по тому же порядку, что у снимка. Серии экземпляров MIG (`GPU_I_ID`) в окна не входят.
//! Сумма нескольких счётчиков (ошибки NVLink) — подзапросом: `increase` по каждому убрал бы имя
//! серии, и одинаковые наборы меток разных счётчиков столкнулись бы (Prometheus отвечает 422).

use crate::gpu::fields::{Combine, Field, Kind, PRESENCE, SPECS};
use crate::history::targets::escape;
use crate::history::window::duration;

/// Группировка рядов окна.
pub const BY: &str = "hostname, Hostname, instance, gpu";

/// Поля окон истории: графики вкладки GPU и счётчики проблем.
pub const HISTORY_FIELDS: [Field; 14] = [
    Field::Util,
    Field::SmActive,
    Field::MemUsed,
    Field::MemFree,
    Field::MemReserved,
    Field::Temp,
    Field::MemTemp,
    Field::Power,
    Field::SmClock,
    Field::MemClock,
    Field::Throttle,
    Field::EccDbe,
    Field::PcieReplay,
    Field::NvlinkErrors,
];

/// График вкладки GPU, которому принадлежит поле (для записи `partial`).
pub fn chart_of(field: Field) -> &'static str {
    match field {
        Field::Util | Field::SmActive => "util",
        Field::MemUsed | Field::MemFree | Field::MemReserved | Field::MemTotal => "memory",
        Field::Temp | Field::MemTemp => "temp",
        Field::Power => "power",
        Field::SmClock | Field::MemClock => "clocks",
        _ => "errors",
    }
}

/// Запрос окна: поле и выражение.
#[derive(Debug, Clone, PartialEq)]
pub struct GpuQuery {
    pub field: Field,
    pub expr: String,
}

/// Условия селектора серий карт: `matchers` (без скобок, может быть пустым) и запрет экземпляров
/// MIG.
fn selector(matchers: &str) -> String {
    if matchers.is_empty() {
        "GPU_I_ID=\"\"".into()
    } else {
        format!("{matchers},GPU_I_ID=\"\"")
    }
}

/// Запросы окна для условий `matchers` и интервала `rate_sec` (`rate` окна, I1c §4).
pub fn queries(matchers: &str, rate_sec: u64) -> Vec<GpuQuery> {
    let s = selector(matchers);
    let rate = duration(rate_sec);
    // Разрешение подзапроса суммы счётчиков: четыре точки на интервал.
    let resolution = duration((rate_sec / 4).max(1));
    HISTORY_FIELDS
        .iter()
        .map(|field| {
            let spec = SPECS
                .iter()
                .find(|x| x.field == *field)
                .expect("every history field is in the table");
            let one = |name: &str| match spec.kind {
                Kind::Gauge | Kind::Ratio => {
                    format!("avg by ({BY}) (avg_over_time({name}{{{s}}}[{rate}]))")
                }
                Kind::Mask => format!("max by ({BY}) (max_over_time({name}{{{s}}}[{rate}]))"),
                Kind::Counter => format!("sum by ({BY}) (increase({name}{{{s}}}[{rate}]))"),
            };
            let expr = match spec.combine {
                Combine::First => spec
                    .names
                    .iter()
                    .map(|n| one(n))
                    .collect::<Vec<_>>()
                    .join(" or "),
                Combine::Sum => format!(
                    "increase((sum by ({BY}) ({{__name__=~\"{}\",{s}}}))[{rate}:{resolution}])",
                    escape(&spec.names.join("|"))
                ),
            };
            GpuQuery {
                field: *field,
                expr,
            }
        })
        .collect()
}

/// Поиск хостов под базовым селектором окна: по какой метке подписан нужный хост.
pub fn hosts_expr(matchers: &str) -> String {
    format!("count by (hostname, Hostname, instance) ({PRESENCE}{{{matchers}}})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_have_exact_text() {
        let q = queries(r#"job="dcgm",hostname="gpu-node-1""#, 60);
        assert_eq!(q.len(), 14);
        let expr = |f: Field| q.iter().find(|x| x.field == f).unwrap().expr.clone();
        let s = r#"{job="dcgm",hostname="gpu-node-1",GPU_I_ID=""}"#;
        assert_eq!(
            expr(Field::Util),
            format!(
                "avg by (hostname, Hostname, instance, gpu) (avg_over_time(DCGM_FI_DEV_GPU_UTIL{s}[1m]))"
            )
        );
        assert_eq!(
            expr(Field::SmActive),
            format!(
                "avg by (hostname, Hostname, instance, gpu) (avg_over_time(DCGM_FI_PROF_SM_ACTIVE{s}[1m])) or avg by (hostname, Hostname, instance, gpu) (avg_over_time(DCGM_FI_PROF_GR_ENGINE_ACTIVE{s}[1m]))"
            )
        );
        assert_eq!(
            expr(Field::Throttle),
            format!(
                "max by (hostname, Hostname, instance, gpu) (max_over_time(DCGM_FI_DEV_CLOCKS_EVENT_REASONS{s}[1m])) or max by (hostname, Hostname, instance, gpu) (max_over_time(DCGM_FI_DEV_CLOCK_THROTTLE_REASONS{s}[1m]))"
            )
        );
        assert_eq!(
            expr(Field::PcieReplay),
            format!(
                "sum by (hostname, Hostname, instance, gpu) (increase(DCGM_FI_DEV_PCIE_REPLAY_COUNTER{s}[1m]))"
            )
        );
        assert!(
            expr(Field::NvlinkErrors).starts_with(
                r#"increase((sum by (hostname, Hostname, instance, gpu) ({__name__=~"DCGM_FI_DEV_NVLINK_CRC_FLIT_ERROR_TOTAL|"#
            ),
            "{}",
            expr(Field::NvlinkErrors)
        );
        assert!(expr(Field::NvlinkErrors).ends_with(r#"|DCGM_FI_DEV_NVLINK_REPLAY_TOTAL",job="dcgm",hostname="gpu-node-1",GPU_I_ID=""}))[1m:15s])"#));
        let month = queries("", 2700);
        assert_eq!(
            month[0].expr,
            r#"avg by (hostname, Hostname, instance, gpu) (avg_over_time(DCGM_FI_DEV_GPU_UTIL{GPU_I_ID=""}[45m]))"#
        );
        assert!(
            month[13].expr.ends_with("[45m:675s])"),
            "{}",
            month[13].expr
        );
        assert_eq!(
            hosts_expr(r#"job="dcgm""#),
            r#"count by (hostname, Hostname, instance) (DCGM_FI_DEV_FB_USED{job="dcgm"})"#
        );
    }

    #[test]
    fn every_field_has_a_chart() {
        assert_eq!(chart_of(Field::SmActive), "util");
        assert_eq!(chart_of(Field::MemReserved), "memory");
        assert_eq!(chart_of(Field::MemClock), "clocks");
        assert_eq!(chart_of(Field::NvlinkErrors), "errors");
        let charts: std::collections::BTreeSet<_> =
            HISTORY_FIELDS.iter().map(|f| chart_of(*f)).collect();
        assert_eq!(charts.len(), 6);
    }
}
