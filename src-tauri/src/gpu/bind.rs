//! Автоподбор и проверка привязок GPU (спека I1d §3): хост сервера среди хостов карт, селектор
//! карт режима `prometheus`, селектор экспортёра в источнике истории, номера карт привязки.

use serde::Serialize;

use crate::history::targets::{escape, instance_matches};
use crate::storage::profile::{GpuBinding, ServerConfig, parse_base_url};

/// Хост для сравнения: без скобок IPv6, в нижнем регистре, `localhost`, `127.0.0.1` и `::1` — один
/// адрес (как автоподбор целей в I1c).
pub fn canonical_host(h: &str) -> String {
    let h = h
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_lowercase();
    match h.as_str() {
        "localhost" | "127.0.0.1" | "::1" => "loopback".into(),
        _ => h,
    }
}

fn is_ip(h: &str) -> bool {
    h.parse::<std::net::IpAddr>().is_ok()
}

/// Один ли это хост: имена без учёта регистра; короткое имя совпадает с полным, если это его
/// первая часть (`gpu-node-1` и `gpu-node-1.corp.local`: метка `hostname` экспортёра — обычно
/// короткое имя узла). Адреса IP сравниваются только целиком.
pub fn same_host(a: &str, b: &str) -> bool {
    let (a, b) = (canonical_host(a), canonical_host(b));
    if a == b {
        return true;
    }
    if is_ip(&a) || is_ip(&b) {
        return false;
    }
    let short =
        |full: &str, short: &str| !short.contains('.') && full.split('.').next() == Some(short);
    short(&a, &b) || short(&b, &a)
}

/// Хост адреса сервера.
pub fn server_host(s: &ServerConfig) -> Option<String> {
    parse_base_url(s).ok()?.host_str().map(str::to_string)
}

/// Хост карт, который совпал с хостом сервера (первый по порядку `hosts`).
pub fn matching_host<'a>(s: &ServerConfig, hosts: &'a [String]) -> Option<&'a String> {
    let own = server_host(s)?;
    hosts.iter().find(|h| same_host(h, &own))
}

/// Привязка для сервера без привязки: все карты хоста, совпавшего с хостом сервера.
pub fn suggest_binding(s: &ServerConfig, hosts: &[String]) -> Option<GpuBinding> {
    if s.gpus.is_some() {
        return None;
    }
    matching_host(s, hosts).map(|host| GpuBinding {
        host: host.clone(),
        indices: None,
    })
}

/// Что не так с привязкой сервера среди найденных карт.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingState {
    Ok,
    /// Хоста привязки нет среди карт.
    NoHost,
    /// Номеров нет на хосте.
    NoIndices(Vec<u32>),
}

/// Сверка привязки с картами: `cards` — хост и номера его карт.
pub fn binding_state(b: &GpuBinding, cards: &[(String, Vec<u32>)]) -> BindingState {
    let Some((_, have)) = cards.iter().find(|(h, _)| same_host(h, &b.host)) else {
        return BindingState::NoHost;
    };
    let absent: Vec<u32> = b
        .indices
        .iter()
        .flatten()
        .copied()
        .filter(|i| !have.contains(i))
        .collect();
    if absent.is_empty() {
        BindingState::Ok
    } else {
        BindingState::NoIndices(absent)
    }
}

/// Карты привязки: номера карт хоста, которые относятся к серверу (все или перечисленные).
pub fn bound_indices(b: &GpuBinding, have: &[u32]) -> Vec<u32> {
    match &b.indices {
        None => have.to_vec(),
        Some(list) => have.iter().copied().filter(|i| list.contains(i)).collect(),
    }
}

/// Какой меткой подписан хост карты в источнике истории.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum HostLabel {
    /// `hostname` (dcgm-exporter 4.8).
    #[serde(rename = "hostname")]
    Lower,
    /// `Hostname` (до 4.1).
    #[serde(rename = "Hostname")]
    Upper,
    /// Нет метки хоста — хост из `instance` без порта.
    #[serde(rename = "instance")]
    Instance,
}

impl HostLabel {
    pub fn name(self) -> &'static str {
        match self {
            Self::Lower => "hostname",
            Self::Upper => "Hostname",
            Self::Instance => "instance",
        }
    }
}

/// Карта, найденная в источнике истории: хост, метка хоста и номер.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FoundCard {
    pub host: String,
    pub label: HostLabel,
    pub index: u32,
}

/// Значение для регулярного выражения PromQL (RE2): метасимволы экранируются.
pub fn regex_escape(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    for c in v.chars() {
        if "\\.+*?()|[]{}^$".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Селектор карт по хостам: `{hostname=~"h1|h2"}` — меткой, которой подписаны найденные карты
/// этих хостов (`hostname` побеждает `Hostname`, `instance` — последней, с любым портом).
pub fn hosts_selector(hosts: &[String], found: &[FoundCard]) -> Option<String> {
    if hosts.is_empty() {
        return None;
    }
    let label = found
        .iter()
        .filter(|c| hosts.iter().any(|h| same_host(h, &c.host)))
        .map(|c| c.label)
        .min()
        .unwrap_or(HostLabel::Lower);
    let mut names: Vec<String> = hosts.iter().map(|h| regex_escape(h)).collect();
    names.sort();
    names.dedup();
    let alt = names.join("|");
    let value = match label {
        HostLabel::Instance => format!("({alt})(:[0-9]+)?"),
        _ => alt,
    };
    Some(format!("{{{}=~\"{}\"}}", label.name(), escape(&value)))
}

/// Автоподбор селектора карт (спека I1d §3): хосты карт, совпавшие с хостами серверов, по
/// найденной метке; ни один не совпал — `None`.
pub fn suggest_selector(servers: &[ServerConfig], found: &[FoundCard]) -> Option<String> {
    let own: Vec<String> = servers.iter().filter_map(server_host).collect();
    let mut hosts: Vec<String> = found
        .iter()
        .filter(|c| own.iter().any(|h| same_host(h, &c.host)))
        .map(|c| c.host.clone())
        .collect();
    hosts.sort();
    hosts.dedup();
    hosts_selector(&hosts, found)
}

/// Цель экспортёра DCGM в источнике истории.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DcgmTarget {
    pub job: String,
    pub instance: String,
}

impl DcgmTarget {
    /// Селектор серий цели: `{job="…", instance="…"}`.
    pub fn selector(&self) -> String {
        format!(
            "{{job=\"{}\", instance=\"{}\"}}",
            escape(&self.job),
            escape(&self.instance)
        )
    }
}

/// Цели с `instance` — хостом и портом адреса экспортёра, по порядку `job`; первая — подобранная.
pub fn matching_exporter_targets<'a>(
    exporter: &url::Url,
    targets: &'a [DcgmTarget],
) -> Vec<&'a DcgmTarget> {
    let mut found: Vec<&DcgmTarget> = targets
        .iter()
        .filter(|t| instance_matches(&t.instance, exporter))
        .collect();
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::profile::sample_server;

    fn server(url: &str) -> ServerConfig {
        ServerConfig {
            base_url: url.into(),
            ..sample_server("s1")
        }
    }

    fn found(host: &str, label: HostLabel, index: u32) -> FoundCard {
        FoundCard {
            host: host.into(),
            label,
            index,
        }
    }

    #[test]
    fn hosts_compare_by_name_short_name_and_loopback() {
        assert!(same_host("GPU-Node-1", "gpu-node-1"));
        assert!(same_host("gpu-node-1", "gpu-node-1.corp.local"));
        assert!(same_host("gpu-node-1.corp.local", "gpu-node-1"));
        assert!(
            !same_host("gpu-node-1.a.local", "gpu-node-1.b.local"),
            "два полных имени — целиком"
        );
        assert!(!same_host("gpu-node-10", "gpu-node-1"));
        assert!(same_host("localhost", "127.0.0.1"));
        assert!(same_host("[::1]", "localhost"));
        assert!(!same_host("10.0.0.5", "10.0.0.50"));
        assert!(!same_host("10.0.0.5", "10"), "адрес IP не короткое имя");
    }

    #[test]
    fn server_binding_is_suggested_by_host() {
        let hosts = vec!["gpu-node-1".to_string(), "localhost".to_string()];
        assert_eq!(
            suggest_binding(&server("http://gpu-node-1.corp.local:8000"), &hosts),
            Some(GpuBinding {
                host: "gpu-node-1".into(),
                indices: None
            })
        );
        assert_eq!(
            suggest_binding(&server("http://127.0.0.1:8000/v1"), &hosts).map(|b| b.host),
            Some("localhost".into())
        );
        assert_eq!(
            suggest_binding(&server("http://lb.corp:8000"), &hosts),
            None,
            "балансировщик"
        );
        let mut bound = server("http://gpu-node-1:8000");
        bound.gpus = Some(GpuBinding {
            host: "other".into(),
            indices: Some(vec![1]),
        });
        assert_eq!(
            suggest_binding(&bound, &hosts),
            None,
            "сохранённая привязка не меняется"
        );
    }

    #[test]
    fn binding_is_checked_against_the_cards() {
        let cards = vec![("gpu-node-1".to_string(), vec![0, 1, 2, 3])];
        let b = |host: &str, indices: Option<Vec<u32>>| GpuBinding {
            host: host.into(),
            indices,
        };
        assert_eq!(
            binding_state(&b("GPU-NODE-1", None), &cards),
            BindingState::Ok
        );
        assert_eq!(
            binding_state(&b("gpu-node-1", Some(vec![1, 3])), &cards),
            BindingState::Ok
        );
        assert_eq!(
            binding_state(&b("gpu-node-1", Some(vec![1, 4, 5])), &cards),
            BindingState::NoIndices(vec![4, 5])
        );
        assert_eq!(
            binding_state(&b("gpu-node-2", None), &cards),
            BindingState::NoHost
        );
        assert_eq!(bound_indices(&b("gpu-node-1", None), &[0, 1, 2]), [0, 1, 2]);
        assert_eq!(
            bound_indices(&b("gpu-node-1", Some(vec![2, 7])), &[0, 1, 2]),
            [2]
        );
    }

    #[test]
    fn selector_is_built_from_server_hosts_with_the_found_label() {
        let servers = vec![
            server("http://gpu-node-2:8000"),
            server("http://gpu-node-1.corp:8000"),
            server("http://lb:8000"),
        ];
        let cards = vec![
            found("gpu-node-1", HostLabel::Lower, 0),
            found("gpu-node-2", HostLabel::Lower, 0),
            found("gpu-node-3", HostLabel::Lower, 0),
        ];
        assert_eq!(
            suggest_selector(&servers, &cards).as_deref(),
            Some(r#"{hostname=~"gpu-node-1|gpu-node-2"}"#)
        );
        let old = vec![found("gpu-node-1", HostLabel::Upper, 0)];
        assert_eq!(
            suggest_selector(&servers, &old).as_deref(),
            Some(r#"{Hostname=~"gpu-node-1"}"#)
        );
        let by_instance = vec![found("10.0.0.5", HostLabel::Instance, 0)];
        assert_eq!(
            suggest_selector(&[server("http://10.0.0.5:8000")], &by_instance).as_deref(),
            Some(r#"{instance=~"(10\\.0\\.0\\.5)(:[0-9]+)?"}"#)
        );
        assert_eq!(suggest_selector(&[server("http://lb:8000")], &cards), None);
        assert_eq!(regex_escape("a.b+c(d)|e"), r"a\.b\+c\(d\)\|e");
        let mixed = vec![
            found("h", HostLabel::Upper, 0),
            found("h", HostLabel::Lower, 1),
        ];
        assert_eq!(
            hosts_selector(&["h".into()], &mixed).as_deref(),
            Some(r#"{hostname=~"h"}"#)
        );
        let parsed =
            crate::history::targets::parse_selector(&suggest_selector(&servers, &cards).unwrap())
                .unwrap();
        assert_eq!(
            parsed[0].value, "gpu-node-1|gpu-node-2",
            "селектор разбирается строгим разбором"
        );
    }

    #[test]
    fn exporter_target_is_matched_by_host_and_port() {
        let t = |job: &str, instance: &str| DcgmTarget {
            job: job.into(),
            instance: instance.into(),
        };
        let targets = vec![
            t("gpu", "10.0.0.5:9400"),
            t("dcgm", "10.0.0.5:9400"),
            t("dcgm", "10.0.0.6:9400"),
            t("dcgm", "localhost:9401"),
        ];
        let url = url::Url::parse("http://10.0.0.5:9400/metrics").unwrap();
        let found = matching_exporter_targets(&url, &targets);
        assert_eq!(
            found.iter().map(|t| t.job.as_str()).collect::<Vec<_>>(),
            ["dcgm", "gpu"]
        );
        assert_eq!(
            found[0].selector(),
            r#"{job="dcgm", instance="10.0.0.5:9400"}"#
        );
        let local = url::Url::parse("http://127.0.0.1:9401/metrics").unwrap();
        assert_eq!(matching_exporter_targets(&local, &targets).len(), 1);
    }
}
