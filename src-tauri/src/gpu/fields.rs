//! Канонические поля карты GPU (спека I1d §4): имена серий DCGM-кандидатов по порядку, вид серии
//! и единица. Отсутствующая серия — «нет данных», а не ноль. Набор счётчиков dcgm-exporter по
//! умолчанию не собирает активность SM, ECC и причины троттлинга, поэтому у полей есть запасные
//! серии, а группы без единой серии называет шаг проверки «полнота набора».

use serde::Serialize;

/// Поле карты. JSON — camelCase (`smActive`, `memUsed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Field {
    /// Утилизация, %.
    Util,
    /// Активность SM, % (без `PROF_SM_ACTIVE` — активность graphics engine, `SmSource::Gr`).
    SmActive,
    TensorActive,
    DramActive,
    MemCopyUtil,
    /// Память, МиБ.
    MemUsed,
    MemFree,
    MemReserved,
    /// Всего памяти: занято + свободно + зарезервировано, МиБ (считается, серии нет).
    MemTotal,
    /// Температура карты и памяти, °C.
    Temp,
    MemTemp,
    /// Мощность, Вт.
    Power,
    /// Такты SM и памяти, МГц.
    SmClock,
    MemClock,
    /// Битовая маска причин снижения тактов (`CLOCKS_EVENT_REASONS`).
    Throttle,
    /// Время троттлинга по мощности и температуре, нс (счётчики).
    PowerViolation,
    ThermalViolation,
    /// Код последней XID-ошибки (0 — не было) и число XID за окно экспортёра.
    Xid,
    XidCount,
    /// Ошибки ECC (volatile) — некорректируемые и корректируемые (счётчики).
    EccDbe,
    EccSbe,
    /// Переназначенные строки памяти и сбой переназначения.
    RemapUncorrectable,
    RemapCorrectable,
    RemapFailure,
    /// Повторы PCIe (счётчик) и трафик PCIe, Б/с.
    PcieReplay,
    PcieTx,
    PcieRx,
    /// Трафик NVLink, МБ/с, и ошибки NVLink (сумма счётчиков).
    NvlinkBandwidth,
    NvlinkErrors,
}

/// Вид серии: датчик, доля (на выходе — проценты), битовая маска, счётчик.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Gauge,
    Ratio,
    Mask,
    Counter,
}

/// Как брать значение из кандидатов: первый найденный или сумма всех найденных.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combine {
    First,
    Sum,
}

#[derive(Debug, Clone, Copy)]
pub struct FieldSpec {
    pub field: Field,
    /// Полные имена серий по порядку предпочтения.
    pub names: &'static [&'static str],
    pub kind: Kind,
    pub combine: Combine,
}

const fn spec(field: Field, names: &'static [&'static str], kind: Kind) -> FieldSpec {
    FieldSpec {
        field,
        names,
        kind,
        combine: Combine::First,
    }
}

/// Серия активности SM; без неё `SmActive` берётся из активности graphics engine.
pub const SM_ACTIVE: &str = "DCGM_FI_PROF_SM_ACTIVE";
/// Серия, по которой узнаётся экспортёр DCGM и считаются карты: она есть в наборе по умолчанию и
/// в режиме MIG, где утилизации карты нет.
pub const PRESENCE: &str = "DCGM_FI_DEV_FB_USED";

/// Таблица полей (спека I1d §4).
pub const SPECS: &[FieldSpec] = &[
    spec(Field::Util, &["DCGM_FI_DEV_GPU_UTIL"], Kind::Gauge),
    spec(
        Field::SmActive,
        &[SM_ACTIVE, "DCGM_FI_PROF_GR_ENGINE_ACTIVE"],
        Kind::Ratio,
    ),
    spec(
        Field::TensorActive,
        &["DCGM_FI_PROF_PIPE_TENSOR_ACTIVE"],
        Kind::Ratio,
    ),
    spec(
        Field::DramActive,
        &["DCGM_FI_PROF_DRAM_ACTIVE"],
        Kind::Ratio,
    ),
    spec(
        Field::MemCopyUtil,
        &["DCGM_FI_DEV_MEM_COPY_UTIL"],
        Kind::Gauge,
    ),
    spec(Field::MemUsed, &[PRESENCE], Kind::Gauge),
    spec(Field::MemFree, &["DCGM_FI_DEV_FB_FREE"], Kind::Gauge),
    spec(
        Field::MemReserved,
        &["DCGM_FI_DEV_FB_RESERVED"],
        Kind::Gauge,
    ),
    spec(Field::Temp, &["DCGM_FI_DEV_GPU_TEMP"], Kind::Gauge),
    spec(Field::MemTemp, &["DCGM_FI_DEV_MEMORY_TEMP"], Kind::Gauge),
    spec(Field::Power, &["DCGM_FI_DEV_POWER_USAGE"], Kind::Gauge),
    spec(Field::SmClock, &["DCGM_FI_DEV_SM_CLOCK"], Kind::Gauge),
    spec(Field::MemClock, &["DCGM_FI_DEV_MEM_CLOCK"], Kind::Gauge),
    spec(
        Field::Throttle,
        &[
            "DCGM_FI_DEV_CLOCKS_EVENT_REASONS",
            "DCGM_FI_DEV_CLOCK_THROTTLE_REASONS",
        ],
        Kind::Mask,
    ),
    spec(
        Field::PowerViolation,
        &["DCGM_FI_DEV_POWER_VIOLATION"],
        Kind::Counter,
    ),
    spec(
        Field::ThermalViolation,
        &["DCGM_FI_DEV_THERMAL_VIOLATION"],
        Kind::Counter,
    ),
    spec(Field::Xid, &["DCGM_FI_DEV_XID_ERRORS"], Kind::Gauge),
    FieldSpec {
        field: Field::XidCount,
        names: &["DCGM_EXP_XID_ERRORS_COUNT"],
        kind: Kind::Gauge,
        combine: Combine::Sum,
    },
    spec(
        Field::EccDbe,
        &["DCGM_FI_DEV_ECC_DBE_VOL_TOTAL"],
        Kind::Counter,
    ),
    spec(
        Field::EccSbe,
        &["DCGM_FI_DEV_ECC_SBE_VOL_TOTAL"],
        Kind::Counter,
    ),
    spec(
        Field::RemapUncorrectable,
        &["DCGM_FI_DEV_UNCORRECTABLE_REMAPPED_ROWS"],
        Kind::Gauge,
    ),
    spec(
        Field::RemapCorrectable,
        &["DCGM_FI_DEV_CORRECTABLE_REMAPPED_ROWS"],
        Kind::Gauge,
    ),
    spec(
        Field::RemapFailure,
        &["DCGM_FI_DEV_ROW_REMAP_FAILURE"],
        Kind::Gauge,
    ),
    spec(
        Field::PcieReplay,
        &["DCGM_FI_DEV_PCIE_REPLAY_COUNTER"],
        Kind::Counter,
    ),
    spec(Field::PcieTx, &["DCGM_FI_PROF_PCIE_TX_BYTES"], Kind::Gauge),
    spec(Field::PcieRx, &["DCGM_FI_PROF_PCIE_RX_BYTES"], Kind::Gauge),
    spec(
        Field::NvlinkBandwidth,
        &["DCGM_FI_DEV_NVLINK_BANDWIDTH_TOTAL"],
        Kind::Gauge,
    ),
    FieldSpec {
        field: Field::NvlinkErrors,
        names: &[
            // До Hopper.
            "DCGM_FI_DEV_NVLINK_CRC_FLIT_ERROR_TOTAL",
            "DCGM_FI_DEV_NVLINK_CRC_DATA_ERROR_TOTAL",
            "DCGM_FI_DEV_NVLINK_REPLAY_ERROR_TOTAL",
            "DCGM_FI_DEV_NVLINK_RECOVERY_ERROR_TOTAL",
            // Hopper.
            "DCGM_FI_DEV_NVLINK_CRC_ERROR_TOTAL",
            "DCGM_FI_DEV_NVLINK_RECOVERY_TOTAL",
            "DCGM_FI_DEV_NVLINK_REPLAY_TOTAL",
        ],
        kind: Kind::Counter,
        combine: Combine::Sum,
    },
];

/// Поле, которому принадлежит серия `name`, и номер кандидата.
pub fn spec_of(name: &str) -> Option<(&'static FieldSpec, usize)> {
    SPECS
        .iter()
        .find_map(|s| s.names.iter().position(|n| *n == name).map(|i| (s, i)))
}

/// Все имена серий таблицы — для мгновенного запроса к источнику истории.
pub fn all_names() -> impl Iterator<Item = &'static str> {
    SPECS.iter().flat_map(|s| s.names.iter().copied())
}

/// Откуда взята активность SM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SmSource {
    /// `DCGM_FI_PROF_SM_ACTIVE`.
    Sm,
    /// `DCGM_FI_PROF_GR_ENGINE_ACTIVE` — запасная серия.
    Gr,
}

/// Группа счётчиков, которой нет ни у одной карты (шаг проверки «полнота набора»).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Group {
    Sm,
    Ecc,
    Throttle,
    NvlinkErrors,
}

/// Разряды маски причин снижения тактов (`DCGM_CLOCKS_EVENT_REASON_*`).
pub const REASONS: &[(u64, &str)] = &[
    (0x1, "GPU_IDLE"),
    (0x2, "CLOCKS_SETTING"),
    (0x4, "SW_POWER_CAP"),
    (0x8, "HW_SLOWDOWN"),
    (0x10, "SYNC_BOOST"),
    (0x20, "SW_THERMAL"),
    (0x40, "HW_THERMAL"),
    (0x80, "HW_POWER_BRAKE"),
    (0x100, "DISPLAY_CLOCKS"),
];

/// Причины, которые отмечаются значком «троттлинг»; остальные — только в подсказке.
pub const ALERT_REASONS: &[&str] = &[
    "SW_POWER_CAP",
    "HW_SLOWDOWN",
    "SW_THERMAL",
    "HW_THERMAL",
    "HW_POWER_BRAKE",
];

/// Причины из маски по порядку разрядов; неизвестные разряды не называются.
pub fn reasons(mask: u64) -> Vec<&'static str> {
    REASONS
        .iter()
        .filter(|(bit, _)| mask & bit != 0)
        .map(|(_, name)| *name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_series_belongs_to_one_field() {
        let mut seen = std::collections::HashSet::new();
        for name in all_names() {
            assert!(seen.insert(name), "{name} дважды");
            assert!(name.starts_with("DCGM_"), "{name}");
        }
        assert_eq!(
            spec_of("DCGM_FI_PROF_GR_ENGINE_ACTIVE").map(|(s, i)| (s.field, i)),
            Some((Field::SmActive, 1))
        );
        assert_eq!(
            spec_of(SM_ACTIVE).map(|(s, i)| (s.field, i)),
            Some((Field::SmActive, 0))
        );
        assert_eq!(
            spec_of("DCGM_FI_DEV_NVLINK_REPLAY_TOTAL").map(|(s, _)| s.combine),
            Some(Combine::Sum)
        );
        assert!(
            spec_of("DCGM_FI_DEV_ENC_UTIL").is_none(),
            "кодировщик экрану не нужен"
        );
        assert!(
            SPECS.iter().all(|s| s.field != Field::MemTotal),
            "всего памяти считается"
        );
    }

    #[test]
    fn throttle_mask_names_reasons_in_bit_order() {
        assert_eq!(reasons(0), Vec::<&str>::new());
        assert_eq!(reasons(0x48), ["HW_SLOWDOWN", "HW_THERMAL"]);
        assert_eq!(
            reasons(0x1 | 0x4 | 0x80 | 0x100),
            [
                "GPU_IDLE",
                "SW_POWER_CAP",
                "HW_POWER_BRAKE",
                "DISPLAY_CLOCKS"
            ]
        );
        assert_eq!(reasons(0x1000), Vec::<&str>::new(), "неизвестный разряд");
        assert!(
            ALERT_REASONS
                .iter()
                .all(|a| REASONS.iter().any(|(_, r)| r == a))
        );
    }

    #[test]
    fn json_names() {
        assert_eq!(serde_json::to_value(Field::SmActive).unwrap(), "smActive");
        assert_eq!(
            serde_json::to_value(Field::NvlinkErrors).unwrap(),
            "nvlinkErrors"
        );
        assert_eq!(
            serde_json::to_value(Group::NvlinkErrors).unwrap(),
            "nvlinkErrors"
        );
        assert_eq!(serde_json::to_value(SmSource::Gr).unwrap(), "gr");
    }
}
