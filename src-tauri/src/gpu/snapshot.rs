//! Карты GPU из серий DCGM (спека I1d §2, §4): хост, номер, метки карты и значения полей.
//! Серии приходят из `/metrics` экспортёра или из мгновенного запроса к источнику истории — в обоих
//! случаях это `metrics::parse::Sample`.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::gpu::fields::{Combine, Field, Group, Kind, SM_ACTIVE, SmSource, spec_of};
use crate::metrics::parse::Sample;

/// Сколько карт может быть у инсталляции (спека I1d §1).
pub const MAX_GPUS: usize = 64;

/// Нагрузка на карте по меткам Kubernetes экспортёра.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Workload {
    pub namespace: String,
    pub pod: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
}

/// Экземпляр MIG карты: профиль и номер.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigInstance {
    pub profile: String,
    pub instance: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuCard {
    /// Хост карты; в JSON — у группы хоста.
    #[serde(skip)]
    pub host: String,
    /// Версия драйвера; в JSON — у группы хоста.
    #[serde(skip)]
    pub driver: Option<String>,
    pub index: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pci_bus_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workload: Option<Workload>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mig: Vec<MigInstance>,
    /// Адрес экспортёра (режим `direct`): подпись у второй карты с тем же хостом и номером.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exporter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sm_source: Option<SmSource>,
    pub values: BTreeMap<Field, f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuHost {
    pub host: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,
    pub cards: Vec<GpuCard>,
}

/// Метка непуста.
fn label<'a>(s: &'a Sample, name: &str) -> Option<&'a str> {
    s.label(name).filter(|v| !v.is_empty())
}

/// Хост карты: метка `hostname` (dcgm-exporter 4.8), `Hostname` (до 4.1), иначе `fallback`.
pub fn host_of(s: &Sample, fallback: &dyn Fn(&Sample) -> String) -> String {
    label(s, "hostname")
        .or_else(|| label(s, "Hostname"))
        .map(str::to_string)
        .unwrap_or_else(|| fallback(s))
}

/// Карты из серий DCGM. `fallback` — хост серии без метки хоста (адрес экспортёра, `instance`);
/// `exporter` — адрес экспортёра для подписи карты. Серии без номера карты и не из таблицы полей
/// пропускаются; серии экземпляров MIG (`GPU_I_ID`) только перечисляют экземпляры.
pub fn cards_from(
    samples: &[Sample],
    fallback: &dyn Fn(&Sample) -> String,
    exporter: Option<&str>,
) -> Vec<GpuCard> {
    // (хост, номер) → карта и номер кандидата, которым взято поле First.
    let mut cards: BTreeMap<(String, u32), (GpuCard, BTreeMap<Field, usize>)> = BTreeMap::new();
    for s in samples {
        if !s.name.starts_with("DCGM_") {
            continue;
        }
        let Some(index) = s.label("gpu").and_then(|g| g.parse::<u32>().ok()) else {
            continue;
        };
        let host = host_of(s, fallback);
        let (card, taken) = cards.entry((host.clone(), index)).or_insert_with(|| {
            let card = GpuCard {
                host,
                driver: None,
                index,
                uuid: None,
                model: None,
                pci_bus_id: None,
                workload: None,
                mig: Vec::new(),
                exporter: exporter.map(str::to_string),
                sm_source: None,
                values: BTreeMap::new(),
            };
            (card, BTreeMap::new())
        });
        fill_labels(card, s);
        if let Some(instance) = label(s, "GPU_I_ID") {
            let m = MigInstance {
                profile: label(s, "GPU_I_PROFILE").unwrap_or_default().to_string(),
                instance: instance.to_string(),
            };
            if !card.mig.contains(&m) {
                card.mig.push(m);
            }
            continue;
        }
        let Some((spec, rank)) = spec_of(&s.name) else {
            continue;
        };
        if !s.value.is_finite() {
            continue;
        }
        let v = if spec.kind == Kind::Ratio {
            s.value * 100.0
        } else {
            s.value
        };
        match spec.combine {
            Combine::Sum => *card.values.entry(spec.field).or_insert(0.0) += v,
            Combine::First => {
                if taken.get(&spec.field).is_none_or(|r| rank < *r) {
                    taken.insert(spec.field, rank);
                    card.values.insert(spec.field, v);
                    if spec.field == Field::SmActive {
                        card.sm_source = Some(if s.name == SM_ACTIVE {
                            SmSource::Sm
                        } else {
                            SmSource::Gr
                        });
                    }
                }
            }
        }
    }
    cards
        .into_values()
        .map(|(mut card, _)| {
            let v = &card.values;
            if let (Some(used), Some(free)) = (v.get(&Field::MemUsed), v.get(&Field::MemFree)) {
                let total = used + free + v.get(&Field::MemReserved).copied().unwrap_or(0.0);
                card.values.insert(Field::MemTotal, total);
            }
            card.mig.sort();
            card
        })
        .collect()
}

/// Метки карты из первой серии, где они есть.
fn fill_labels(card: &mut GpuCard, s: &Sample) {
    let set = |slot: &mut Option<String>, v: Option<&str>| {
        if slot.is_none() {
            *slot = v.map(str::to_string);
        }
    };
    set(
        &mut card.uuid,
        label(s, "UUID").or_else(|| label(s, "uuid")),
    );
    set(&mut card.model, label(s, "modelName"));
    set(&mut card.pci_bus_id, label(s, "pci_bus_id"));
    set(&mut card.driver, label(s, "DCGM_FI_DRIVER_VERSION"));
    if card.workload.is_none()
        && let (Some(namespace), Some(pod)) = (label(s, "namespace"), label(s, "pod"))
    {
        card.workload = Some(Workload {
            namespace: namespace.to_string(),
            pod: pod.to_string(),
            container: label(s, "container").map(str::to_string),
        });
    }
}

/// Группы счётчиков, которых нет ни у одной карты. Ошибки NVLink без трафика NVLink не нужны:
/// у карт без NVLink их и не бывает.
pub fn missing(cards: &[GpuCard]) -> Vec<Group> {
    let any = |f: &dyn Fn(&GpuCard) -> bool| cards.iter().any(f);
    let has = |field: Field| move |c: &GpuCard| c.values.contains_key(&field);
    let mut out = Vec::new();
    if !any(&|c| c.sm_source == Some(SmSource::Sm)) {
        out.push(Group::Sm);
    }
    if !any(&has(Field::EccDbe)) {
        out.push(Group::Ecc);
    }
    if !any(&|c| {
        [
            Field::Throttle,
            Field::PowerViolation,
            Field::ThermalViolation,
        ]
        .iter()
        .any(|f| c.values.contains_key(f))
    }) {
        out.push(Group::Throttle);
    }
    if !any(&has(Field::NvlinkErrors))
        && any(&|c| {
            c.values
                .get(&Field::NvlinkBandwidth)
                .is_some_and(|v| *v > 0.0)
        })
    {
        out.push(Group::NvlinkErrors);
    }
    out
}

/// Карты по хостам: хосты и карты по порядку, не больше `limit` карт (по хосту и номеру); второй
/// результат — карт было больше.
pub fn group(mut cards: Vec<GpuCard>, limit: usize) -> (Vec<GpuHost>, bool) {
    cards.sort_by(|a, b| (&a.host, a.index, &a.exporter).cmp(&(&b.host, b.index, &b.exporter)));
    let truncated = cards.len() > limit;
    cards.truncate(limit);
    let mut hosts: Vec<GpuHost> = Vec::new();
    for card in cards {
        match hosts.last_mut() {
            Some(h) if h.host == card.host => {
                if h.driver.is_none() {
                    h.driver.clone_from(&card.driver);
                }
                h.cards.push(card);
            }
            _ => hosts.push(GpuHost {
                host: card.host.clone(),
                driver: card.driver.clone(),
                cards: vec![card],
            }),
        }
    }
    (hosts, truncated)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::metrics::parse::parse;

    pub const DEFAULT: &str = include_str!("testdata/dcgm-4.8.4/default.txt");
    pub const FULL: &str = include_str!("testdata/dcgm-4.8.4/full.txt");
    pub const OLD: &str = include_str!("testdata/dcgm-4.8.4/4.1.txt");
    pub const MIG: &str = include_str!("testdata/dcgm-4.8.4/mig.txt");

    pub fn cards(text: &str, fallback: &str) -> Vec<GpuCard> {
        let f = fallback.to_string();
        cards_from(&parse(text).unwrap().samples, &move |_| f.clone(), None)
    }

    #[test]
    fn default_counter_set_gives_cards_with_fallback_sm_and_missing_groups() {
        let cards = cards(DEFAULT, "exporter-host");
        assert_eq!(cards.len(), 2);
        let c = &cards[0];
        assert_eq!(
            (c.host.as_str(), c.index),
            ("localhost", 0),
            "метка hostname, а не адрес"
        );
        assert_eq!(c.model.as_deref(), Some("NVIDIA H100 80GB HBM3"));
        assert_eq!(
            c.uuid.as_deref(),
            Some("GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00")
        );
        assert_eq!(c.pci_bus_id.as_deref(), Some("00000000:18:00.0"));
        assert_eq!(c.driver.as_deref(), Some("570.124.06"));
        assert_eq!(c.workload, None);
        assert_eq!(
            c.sm_source,
            Some(SmSource::Gr),
            "без SM_ACTIVE — graphics engine"
        );
        let v = &c.values;
        assert_eq!(v[&Field::MemTotal], 81559.0);
        assert!(
            v[&Field::SmActive] > 1.0 && v[&Field::SmActive] <= 100.0,
            "доля → проценты: {}",
            v[&Field::SmActive]
        );
        assert!(
            v.contains_key(&Field::Util)
                && v.contains_key(&Field::Power)
                && v.contains_key(&Field::PcieReplay)
        );
        for absent in [
            Field::Throttle,
            Field::EccDbe,
            Field::XidCount,
            Field::NvlinkErrors,
            Field::PowerViolation,
        ] {
            assert!(
                !v.contains_key(&absent),
                "{absent:?}: нет данных, а не ноль"
            );
        }
        assert_eq!(v[&Field::Xid], 0.0);
        assert_eq!(
            missing(&cards),
            [Group::Sm, Group::Ecc, Group::Throttle, Group::NvlinkErrors]
        );
    }

    #[test]
    fn full_counter_set_has_every_field_and_kubernetes_workload() {
        let cards = cards(FULL, "x");
        assert_eq!(cards.len(), 8);
        let c = &cards[0];
        assert_eq!(c.host, "gpu-node-1");
        assert_eq!(c.sm_source, Some(SmSource::Sm));
        assert_eq!(
            c.workload,
            Some(Workload {
                namespace: "llm".into(),
                pod: "vllm-0".into(),
                container: Some("vllm".into())
            })
        );
        let v = &c.values;
        assert_eq!(v[&Field::Throttle], 72.0, "HW_SLOWDOWN | HW_THERMAL");
        assert_eq!(v[&Field::Xid], 79.0);
        assert_eq!(v[&Field::XidCount], 1.0);
        assert_eq!(v[&Field::EccDbe], 2.0);
        assert_eq!(
            v[&Field::NvlinkErrors],
            12.0,
            "сумма трёх счётчиков Hopper на опросе 4"
        );
        assert_eq!(cards[1].values[&Field::Xid], 0.0);
        assert!(!cards[1].values.contains_key(&Field::XidCount));
        assert_eq!(missing(&cards), Vec::<Group>::new());
    }

    #[test]
    fn old_exporter_hostname_label_mig_and_fallback_host() {
        let old = cards(OLD, "x");
        assert_eq!(old[0].host, "gpu-old", "метка Hostname до 4.1");
        assert_eq!(old[0].model.as_deref(), Some("NVIDIA A100-SXM4-80GB"));
        let mig = cards(MIG, "x");
        assert_eq!(mig.len(), 2, "экземпляры MIG — не отдельные карты");
        let c = &mig[0];
        assert_eq!(
            c.mig,
            [
                MigInstance {
                    profile: "3g.40gb".into(),
                    instance: "1".into()
                },
                MigInstance {
                    profile: "3g.40gb".into(),
                    instance: "2".into()
                }
            ]
        );
        assert!(
            !c.values.contains_key(&Field::Util) && !c.values.contains_key(&Field::MemUsed),
            "{:?}",
            c.values
        );
        assert!(c.values.contains_key(&Field::Temp), "поля карты остаются");
        assert!(mig[1].mig.is_empty() && mig[1].values.contains_key(&Field::Util));
        let text = DEFAULT.replace("hostname=\"localhost\",", "");
        let fallback = cards(&text, "10.0.0.5");
        assert_eq!(
            fallback[0].host, "10.0.0.5",
            "без метки хоста — адрес экспортёра"
        );
    }

    #[test]
    fn group_sorts_hosts_and_cards_and_limits_the_count() {
        let mut all = cards(FULL, "x");
        all.extend(cards(DEFAULT, "x"));
        all.reverse();
        let (hosts, truncated) = group(all.clone(), MAX_GPUS);
        assert!(!truncated);
        assert_eq!(
            hosts
                .iter()
                .map(|h| (h.host.as_str(), h.cards.len()))
                .collect::<Vec<_>>(),
            [("gpu-node-1", 8), ("localhost", 2)]
        );
        assert_eq!(hosts[0].driver.as_deref(), Some("570.124.06"));
        assert_eq!(
            hosts[0].cards.iter().map(|c| c.index).collect::<Vec<_>>(),
            (0..8).collect::<Vec<_>>()
        );
        let (hosts, truncated) = group(all, 9);
        assert!(truncated);
        assert_eq!(hosts.iter().map(|h| h.cards.len()).sum::<usize>(), 9);
        assert_eq!(hosts[1].cards.len(), 1);
    }

    #[test]
    fn json_shape() {
        let (hosts, _) = group(cards(FULL, "x"), 1);
        let v = serde_json::to_value(&hosts[0]).unwrap();
        assert_eq!(v["host"], "gpu-node-1");
        assert_eq!(v["driver"], "570.124.06");
        let card = &v["cards"][0];
        assert_eq!(card["index"], 0);
        assert_eq!(card["smSource"], "sm");
        assert_eq!(card["workload"]["namespace"], "llm");
        assert_eq!(card["values"]["throttle"], 72.0);
        assert!(
            card.get("host").is_none()
                && card.get("mig").is_none()
                && card.get("exporter").is_none(),
            "{card}"
        );
    }
}
