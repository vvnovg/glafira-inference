//! Проверка компонента GPU (спека I1d §3): конфигурация → экспортёры (DNS/TCP, `/metrics`, карты)
//! или карты в источнике истории → привязка каждого сервера → сопоставление экспортёров с
//! историей → полнота набора счётчиков. Сбой GPU не мешает подключению серверов: проверка только
//! сообщает его.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::Serialize;

use crate::error::{AppError, ErrorCode};
use crate::gpu::bind::{
    BindingState, DcgmTarget, FoundCard, HostLabel, binding_state, matching_exporter_targets,
    same_host, server_host, suggest_binding, suggest_selector,
};
use crate::gpu::fields::{Group, PRESENCE};
use crate::gpu::snapshot::{GpuCard, MAX_GPUS, missing};
use crate::gpu::source::{Exporter, exporter_server, instance_host, too_many_gpus};
use crate::history::client::{HistoryClient, Sample};
use crate::history::targets::{parse_selector, render, selector_label};
use crate::i18n::LocalizedText;
use crate::policy::PolicyStatus;
use crate::servers::client::Timeouts;
use crate::servers::connectivity::{
    TestReport, TestStep, failed_step, passed, report, tcp_failure, tcp_probe, tcp_target,
};
use crate::storage::profile::{
    DirectGpu, GpuBinding, GpuSource, InstallationProfile, PrometheusGpu, parse_exporter_url,
    redact_url, validate_gpu,
};

/// Карты в источнике истории: хост (по метке), метка хоста и номер.
pub const CARDS: &str = "count by (hostname, Hostname, instance, gpu) (DCGM_FI_DEV_FB_USED)";
/// Цели экспортёров DCGM в источнике истории.
pub const TARGETS: &str = "count by (job, instance) (DCGM_FI_DEV_FB_USED)";

/// Хост и номера его карт.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FoundHost {
    pub host: String,
    pub cards: Vec<u32>,
}

/// Привязка, которую форма может поставить серверу без привязки.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BindingSuggestion {
    pub server_id: String,
    pub binding: Option<GpuBinding>,
}

/// Селектор экспортёра в истории, который может поставить форма, и подходящие цели.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExporterSuggestion {
    pub url: String,
    pub history_selector: Option<String>,
    pub targets: Vec<DcgmTarget>,
}

/// Итог проверки для формы.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuCheck {
    pub report: TestReport,
    /// Хосты карт компонента (для выбора привязки сервера).
    pub hosts: Vec<FoundHost>,
    /// Все хосты карт источника истории (режим `prometheus`, для выбора хостов селектора).
    pub available: Vec<String>,
    /// Автоподбор селектора карт (режим `prometheus`, селектор пуст).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
    pub bindings: Vec<BindingSuggestion>,
    pub exporters: Vec<ExporterSuggestion>,
}

/// Шаг с предупреждением: проверку не валит, показывается ⚠.
fn warning_step(name: &'static str, message: LocalizedText) -> TestStep {
    TestStep {
        name,
        ok: true,
        warning: true,
        message,
        error: None,
    }
}

/// Хосты и номера карт по порядку из пар «хост, номер».
fn hosts_of<'a>(pairs: impl Iterator<Item = (&'a str, u32)>) -> Vec<FoundHost> {
    let mut by_host: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    for (host, index) in pairs {
        by_host.entry(host.to_string()).or_default().push(index);
    }
    by_host
        .into_iter()
        .map(|(host, mut cards)| {
            cards.sort_unstable();
            cards.dedup();
            FoundHost { host, cards }
        })
        .collect()
}

/// Карты из ответа `CARDS`: метка хоста — `hostname`, `Hostname`, иначе `instance` без порта.
pub fn found_cards(samples: &[Sample]) -> Vec<FoundCard> {
    let mut out: Vec<FoundCard> = samples
        .iter()
        .filter_map(|s| {
            let get = |k: &str| s.labels.get(k).filter(|v| !v.is_empty());
            let index = get("gpu")?.parse().ok()?;
            let (host, label) = if let Some(h) = get("hostname") {
                (h.clone(), HostLabel::Lower)
            } else if let Some(h) = get("Hostname") {
                (h.clone(), HostLabel::Upper)
            } else {
                (instance_host(get("instance")?), HostLabel::Instance)
            };
            Some(FoundCard { host, label, index })
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Цели из ответа `TARGETS`.
fn targets_from(samples: &[Sample]) -> Vec<DcgmTarget> {
    let mut out: Vec<DcgmTarget> = samples
        .iter()
        .filter_map(|s| {
            Some(DcgmTarget {
                job: s.labels.get("job")?.clone(),
                instance: s.labels.get("instance")?.clone(),
            })
        })
        .collect();
    out.sort();
    out
}

fn hosts_text(hosts: &[FoundHost]) -> String {
    let mut names: Vec<&str> = hosts.iter().map(|h| h.host.as_str()).take(20).collect();
    if hosts.len() > 20 {
        names.push("…");
    }
    names.join(", ")
}

/// Шаги одного экспортёра: DNS/TCP, `/metrics`, карты. Карты — при успехе.
async fn exporter_steps(
    e: &crate::storage::profile::GpuExporter,
    d: &DirectGpu,
    timeouts: Timeouts,
    password: Option<String>,
    policy: &PolicyStatus,
    steps: &mut Vec<TestStep>,
) -> Vec<GpuCard> {
    let shown = redact_url(&e.url);
    let opened = exporter_server(e, d).and_then(|server| {
        let url = parse_exporter_url(e)?;
        Ok((
            server,
            url,
            Exporter::new(e, d, timeouts, password, policy)?,
        ))
    });
    let (server, url, exporter) = match opened {
        Ok(x) => x,
        Err(err) => {
            steps.push(failed_step("config", err));
            return Vec::new();
        }
    };
    if let Some((host, port, via_proxy)) = tcp_target(&server, &url) {
        let limit = Duration::from_millis(timeouts.connect_ms);
        if let Err(err) = tcp_probe(&host, port, limit).await {
            steps.push(failed_step("tcp", tcp_failure(err, &host, port, via_proxy)));
            return Vec::new();
        }
    }
    let cards = match exporter.cards().await {
        Ok(c) => c,
        Err(err) => {
            let step = match err.code {
                ErrorCode::AuthFailed => "auth",
                ErrorCode::TlsError => "tls",
                _ => "metrics",
            };
            steps.push(failed_step(step, err));
            return Vec::new();
        }
    };
    if cards.is_empty() {
        steps.push(failed_step(
            "metrics",
            app_err!(
                ErrorCode::InvalidInput,
                "errors.gpu.noDcgm",
                "{{url}} answers, but there are no DCGM metrics (DCGM_FI_DEV_FB_USED): check the exporter address",
                url = shown
            ),
        ));
        return Vec::new();
    }
    let hosts = hosts_of(cards.iter().map(|c| (c.host.as_str(), c.index)));
    let driver = cards
        .iter()
        .find_map(|c| c.driver.clone())
        .unwrap_or_else(|| "—".into());
    steps.push(passed(
        "cards",
        ltext!(
            "messages.gpuTest.exporterCards",
            "{{url}}: {{count}} GPUs on {{hosts}}, driver {{driver}}",
            url = shown,
            count = cards.len(),
            hosts = hosts_text(&hosts),
            driver = driver
        ),
    ));
    cards
}

/// Ошибка «нет источника истории» для режима `prometheus`.
fn no_history() -> AppError {
    app_err!(
        ErrorCode::InvalidInput,
        "errors.gpu.noHistorySource",
        "GPU metrics from the history source need a metrics history source: add it to the installation"
    )
}

/// Шаги режима `prometheus`: карты под селектором (или под подобранным), их число.
async fn prometheus_steps(
    g: &PrometheusGpu,
    p: &InstallationProfile,
    client: &HistoryClient,
    steps: &mut Vec<TestStep>,
    out: &mut GpuCheck,
) -> Vec<FoundCard> {
    let all = match client.query(CARDS).await {
        Ok(s) => found_cards(&s),
        Err(e) => {
            steps.push(failed_step("history", e));
            return Vec::new();
        }
    };
    let mut available: Vec<String> = all.iter().map(|c| c.host.clone()).collect();
    available.dedup();
    out.available = available;
    let chosen: Vec<FoundCard> = match &g.selector {
        Some(selector) => {
            let matchers = match parse_selector(selector) {
                Ok(m) => m,
                Err(e) => {
                    steps.push(failed_step("cards", e));
                    return Vec::new();
                }
            };
            let expr = format!(
                "count by (hostname, Hostname, instance, gpu) ({PRESENCE}{{{}}})",
                render(&matchers)
            );
            match client.query(&expr).await {
                Ok(s) => found_cards(&s),
                Err(e) => {
                    steps.push(failed_step("cards", e));
                    return Vec::new();
                }
            }
        }
        None => match suggest_selector(&p.servers, &all) {
            Some(selector) => {
                let own: Vec<String> = p.servers.iter().filter_map(server_host).collect();
                let hosts: Vec<String> = all
                    .iter()
                    .filter(|c| own.iter().any(|h| same_host(h, &c.host)))
                    .map(|c| c.host.clone())
                    .collect();
                out.selector = Some(selector.clone());
                steps.push(passed(
                    "cards",
                    ltext!(
                        "messages.gpuTest.selectorAuto",
                        "Card selector {{selector}} (auto-match)",
                        selector = selector
                    ),
                ));
                all.iter()
                    .filter(|c| hosts.contains(&c.host))
                    .cloned()
                    .collect()
            }
            None if all.is_empty() => Vec::new(),
            None => {
                let hosts = hosts_of(all.iter().map(|c| (c.host.as_str(), c.index)));
                steps.push(warning_step(
                    "cards",
                    ltext!(
                        "messages.gpuTest.noServerHosts",
                        "The hosts of the servers are not among the GPU hosts — choose the hosts: {{hosts}}",
                        hosts = hosts_text(&hosts)
                    ),
                ));
                all.clone()
            }
        },
    };
    if chosen.is_empty() {
        let w = if g.selector.is_some() {
            ltext!(
                "messages.gpuTest.noCardsUnderSelector",
                "There are no GPUs under the card selector"
            )
        } else {
            ltext!(
                "messages.gpuTest.noDcgmInHistory",
                "The history source does not collect DCGM metrics"
            )
        };
        steps.push(warning_step("cards", w));
        return chosen;
    }
    if chosen.len() > MAX_GPUS {
        steps.push(failed_step("cards", too_many_gpus(chosen.len())));
        return Vec::new();
    }
    let hosts = hosts_of(chosen.iter().map(|c| (c.host.as_str(), c.index)));
    steps.push(passed(
        "cards",
        ltext!(
            "messages.gpuTest.historyCards",
            "{{count}} GPUs on {{hosts}}",
            count = chosen.len(),
            hosts = hosts_text(&hosts)
        ),
    ));
    chosen
}

/// Шаг привязки сервера.
fn bind_step(
    server: &crate::storage::profile::ServerConfig,
    hosts: &[FoundHost],
    out: &mut Vec<BindingSuggestion>,
) -> TestStep {
    let name = &server.name;
    let pairs: Vec<(String, Vec<u32>)> = hosts
        .iter()
        .map(|h| (h.host.clone(), h.cards.clone()))
        .collect();
    if let Some(b) = &server.gpus {
        return match binding_state(b, &pairs) {
            BindingState::Ok => passed(
                "bind",
                ltext!(
                    "messages.gpuTest.bound",
                    "“{{server}}”: {{host}}, {{cards}}",
                    server = name,
                    host = b.host,
                    cards = cards_text(b)
                ),
            ),
            BindingState::NoHost => warning_step(
                "bind",
                ltext!(
                    "messages.gpuTest.boundHostMissing",
                    "“{{server}}”: the bound host {{host}} is not among the GPU hosts",
                    server = name,
                    host = b.host
                ),
            ),
            BindingState::NoIndices(absent) => warning_step(
                "bind",
                ltext!(
                    "messages.gpuTest.boundCardsMissing",
                    "“{{server}}”: host {{host}} has no GPUs {{cards}}",
                    server = name,
                    host = b.host,
                    cards = absent
                        .iter()
                        .map(u32::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ),
        };
    }
    let names: Vec<String> = hosts.iter().map(|h| h.host.clone()).collect();
    let binding = suggest_binding(server, &names);
    out.push(BindingSuggestion {
        server_id: server.id.clone(),
        binding: binding.clone(),
    });
    match binding {
        Some(b) => passed(
            "bind",
            ltext!(
                "messages.gpuTest.boundAuto",
                "“{{server}}”: {{host}}, all GPUs (auto-match)",
                server = name,
                host = b.host
            ),
        ),
        None => warning_step(
            "bind",
            ltext!(
                "messages.gpuTest.unbound",
                "“{{server}}”: the host of the server is not among the GPU hosts — choose the GPU host manually",
                server = name
            ),
        ),
    }
}

/// Номера карт привязки для текста шага.
fn cards_text(b: &GpuBinding) -> String {
    match &b.indices {
        None => "*".into(),
        Some(list) => list
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// Шаг сопоставления экспортёра с историей: селектор или автоподбор цели.
async fn history_step(
    client: &HistoryClient,
    e: &crate::storage::profile::GpuExporter,
    targets: &[DcgmTarget],
    out: &mut Vec<ExporterSuggestion>,
) -> TestStep {
    let shown = redact_url(&e.url);
    if let Some(selector) = &e.history_selector {
        let matchers = match parse_selector(selector) {
            Ok(m) => m,
            Err(err) => return failed_step("history", err),
        };
        let expr = format!("count({PRESENCE}{{{}}})", render(&matchers));
        return match client.query(&expr).await {
            Ok(found) if found.iter().any(|s| s.value.is_some_and(|v| v > 0.0)) => passed(
                "history",
                ltext!(
                    "messages.gpuTest.historySelector",
                    "{{url}}: {{selector}}",
                    url = shown,
                    selector = selector_label(selector)
                ),
            ),
            Ok(_) => warning_step(
                "history",
                ltext!(
                    "messages.gpuTest.historyNoSeries",
                    "{{url}}: there are no DCGM series under the selector {{selector}}",
                    url = shown,
                    selector = selector_label(selector)
                ),
            ),
            Err(err) => failed_step("history", err),
        };
    }
    let found: Vec<DcgmTarget> = parse_exporter_url(e)
        .map(|url| {
            matching_exporter_targets(&url, targets)
                .into_iter()
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    out.push(ExporterSuggestion {
        url: e.url.clone(),
        history_selector: found.first().map(DcgmTarget::selector),
        targets: found.clone(),
    });
    match found.first() {
        Some(t) => passed(
            "history",
            ltext!(
                "messages.gpuTest.historyAuto",
                "{{url}}: {{job}} · {{instance}} (auto-match)",
                url = shown,
                job = t.job,
                instance = t.instance
            ),
        ),
        None => warning_step(
            "history",
            ltext!(
                "messages.gpuTest.historyNoTarget",
                "{{url}}: no history target with the address of the exporter — GPU history windows are unavailable",
                url = shown
            ),
        ),
    }
}

/// Предупреждения о картах с одним хостом и номером от разных экспортёров (один экспортёр,
/// записанный дважды — по IP и по имени): по шагу на хост; показываются обе карты (спека I1d §2).
fn duplicate_steps(cards: &[GpuCard]) -> Vec<TestStep> {
    let mut by_card: BTreeMap<(&str, u32), Vec<&str>> = BTreeMap::new();
    for c in cards {
        let list = by_card.entry((&c.host, c.index)).or_default();
        if let Some(e) = c.exporter.as_deref().filter(|e| !list.contains(e)) {
            list.push(e);
        }
    }
    // Хост → число повторных карт и адреса экспортёров, от которых они пришли.
    let mut by_host: BTreeMap<&str, (usize, Vec<&str>)> = BTreeMap::new();
    for ((host, _), exporters) in by_card.iter().filter(|(_, e)| e.len() > 1) {
        let entry = by_host.entry(host).or_default();
        entry.0 += 1;
        for e in exporters {
            if !entry.1.contains(e) {
                entry.1.push(e);
            }
        }
    }
    by_host
        .into_iter()
        .map(|(host, (count, exporters))| {
            warning_step(
                "cards",
                ltext!(
                    "messages.gpuTest.duplicateCards",
                    "Host {{host}}: {{count}} GPUs are reported by several exporters ({{exporters}}) — is one exporter listed twice? Both cards are shown",
                    host = host,
                    count = count,
                    exporters = exporters.join(", ")
                ),
            )
        })
        .collect()
}

/// Шаги-подсказки о группах счётчиков, которых нет ни у одной карты.
fn counter_steps(groups: &[Group]) -> Vec<TestStep> {
    groups
        .iter()
        .map(|g| {
            let message = match g {
                Group::Sm => ltext!(
                    "messages.gpuTest.missingSm",
                    "The exporter does not collect SM activity (DCGM_FI_PROF_SM_ACTIVE): graphics engine activity is shown instead"
                ),
                Group::Ecc => ltext!(
                    "messages.gpuTest.missingEcc",
                    "The exporter does not collect ECC errors (DCGM_FI_DEV_ECC_DBE_VOL_TOTAL, DCGM_FI_DEV_ECC_SBE_VOL_TOTAL)"
                ),
                Group::Throttle => ltext!(
                    "messages.gpuTest.missingThrottle",
                    "The exporter does not collect throttle reasons (DCGM_FI_DEV_CLOCKS_EVENT_REASONS)"
                ),
                Group::NvlinkErrors => ltext!(
                    "messages.gpuTest.missingNvlink",
                    "The exporter does not collect NVLink errors (DCGM_FI_DEV_NVLINK_*_ERROR_TOTAL)"
                ),
            };
            warning_step("counters", message)
        })
        .collect()
}

/// Проверка компонента GPU профиля `p`; `password` — пароль экспортёров, `history_secret` —
/// секрет источника истории (из формы или keychain).
pub async fn check_gpu(
    p: &InstallationProfile,
    password: Option<String>,
    history_secret: Option<String>,
    policy: &PolicyStatus,
) -> GpuCheck {
    let mut out = GpuCheck {
        report: report(Vec::new(), Vec::new()),
        hosts: Vec::new(),
        available: Vec::new(),
        selector: None,
        bindings: Vec::new(),
        exporters: Vec::new(),
    };
    let finish = |mut out: GpuCheck, steps: Vec<TestStep>| {
        out.report = report(steps, Vec::new());
        out
    };
    let Some(g) = &p.gpu else {
        let e = app_err!(
            ErrorCode::InvalidInput,
            "errors.gpu.notConfigured",
            "The installation has no GPU component"
        );
        return finish(out, vec![failed_step("config", e)]);
    };
    if let Err(e) = validate_gpu(g, p.history.is_some()) {
        return finish(out, vec![failed_step("config", e)]);
    }
    let timeouts = Timeouts::from(p);
    // Сбой клиента истории в режиме `direct` — шаг `history` ниже: экспортёры проверяются и без него.
    let mut history_error = None;
    let history = match &p.history {
        Some(h) => match HistoryClient::new(h, timeouts, history_secret, policy) {
            Ok(c) => Some(c),
            Err(e) if matches!(g, GpuSource::Prometheus(_)) => {
                return finish(out, vec![failed_step("config", e)]);
            }
            Err(e) => {
                history_error = Some(e);
                None
            }
        },
        None => None,
    };
    let mut steps = vec![passed(
        "config",
        ltext!("messages.connectionTest.configOk", "Configuration is valid"),
    )];
    match g {
        GpuSource::Direct(d) => {
            let mut cards = Vec::new();
            for e in &d.exporters {
                cards.extend(
                    exporter_steps(e, d, timeouts, password.clone(), policy, &mut steps).await,
                );
            }
            if cards.len() > MAX_GPUS {
                steps.push(failed_step("cards", too_many_gpus(cards.len())));
            }
            steps.extend(duplicate_steps(&cards));
            let groups = missing(&cards);
            let hosts = hosts_of(cards.iter().map(|c| (c.host.as_str(), c.index)));
            out.hosts = hosts.clone();
            if !cards.is_empty() {
                for server in &p.servers {
                    let step = bind_step(server, &hosts, &mut out.bindings);
                    steps.push(step);
                }
            }
            if let Some(e) = history_error {
                steps.push(failed_step("history", e));
            }
            if let Some(client) = &history {
                match client.query(TARGETS).await {
                    Ok(t) => {
                        let targets = targets_from(&t);
                        for e in &d.exporters {
                            let step = history_step(client, e, &targets, &mut out.exporters).await;
                            steps.push(step);
                        }
                    }
                    Err(e) => steps.push(failed_step("history", e)),
                }
            }
            if !cards.is_empty() {
                steps.extend(counter_steps(&groups));
            }
        }
        GpuSource::Prometheus(pg) => {
            let Some(client) = &history else {
                return finish(out, vec![failed_step("config", no_history())]);
            };
            let chosen = prometheus_steps(pg, p, client, &mut steps, &mut out).await;
            let hosts = hosts_of(chosen.iter().map(|c| (c.host.as_str(), c.index)));
            out.hosts = hosts.clone();
            if !chosen.is_empty() {
                for server in &p.servers {
                    let step = bind_step(server, &hosts, &mut out.bindings);
                    steps.push(step);
                }
            }
        }
    }
    finish(out, steps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::snapshot::tests::{DEFAULT, FULL};
    use crate::history::client::tests::source as history_source;
    use crate::servers::testing::{Reply, TestServer};
    use crate::storage::profile::{
        GpuAuth, HistoryAuth, ProxyConfig, ServerConfig, sample_exporter, sample_gpu,
        sample_profile, sample_server,
    };

    fn snap(name: &str) -> String {
        crate::gpu::source::tests::snapshot_file(name)
    }

    fn server(id: &str, url: &str) -> ServerConfig {
        ServerConfig {
            name: format!("vllm-{id}"),
            base_url: url.into(),
            ..sample_server(id)
        }
    }

    fn direct_profile(urls: &[String]) -> InstallationProfile {
        let mut p = sample_profile("a");
        let GpuSource::Direct(d) = sample_gpu() else {
            unreachable!()
        };
        p.gpu = Some(GpuSource::Direct(DirectGpu {
            exporters: urls.iter().map(|u| sample_exporter(u)).collect(),
            auth: GpuAuth::None,
            proxy: ProxyConfig::None,
            ..d
        }));
        p.servers = vec![
            server("s1", "http://gpu-node-1.corp:8000"),
            server("s2", "http://lb.corp:8000"),
        ];
        p
    }

    fn names(c: &GpuCheck) -> Vec<(&'static str, bool, bool)> {
        c.report
            .steps
            .iter()
            .map(|s| (s.name, s.ok, s.warning))
            .collect()
    }

    fn keys(c: &GpuCheck) -> Vec<&str> {
        c.report
            .steps
            .iter()
            .map(|s| s.message.key.as_str())
            .collect()
    }

    #[tokio::test]
    async fn direct_exporters_cards_bindings_and_missing_counters() {
        let a = TestServer::start(vec![("/metrics", Reply::text(FULL))]).await;
        let b = TestServer::start(vec![("/metrics", Reply::text(DEFAULT))]).await;
        let p = direct_profile(&[
            format!("{}/metrics", a.url()),
            format!("{}/metrics", b.url()),
        ]);
        let c = check_gpu(&p, None, None, &PolicyStatus::default()).await;
        assert!(c.report.ok, "{:?}", c.report);
        assert_eq!(
            names(&c),
            [
                ("config", true, false),
                ("cards", true, false),
                ("cards", true, false),
                ("bind", true, false),
                ("bind", true, true),
            ],
            "полный набор у одного экспортёра — подсказок о счётчиках нет"
        );
        assert_eq!(c.report.steps[1].message.params["count"], "8");
        assert_eq!(c.report.steps[1].message.params["hosts"], "gpu-node-1");
        assert_eq!(c.report.steps[1].message.params["driver"], "570.124.06");
        assert_eq!(
            c.hosts,
            [
                FoundHost {
                    host: "gpu-node-1".into(),
                    cards: (0..8).collect()
                },
                FoundHost {
                    host: "localhost".into(),
                    cards: vec![0, 1]
                }
            ]
        );
        assert_eq!(
            c.bindings,
            [
                BindingSuggestion {
                    server_id: "s1".into(),
                    binding: Some(GpuBinding {
                        host: "gpu-node-1".into(),
                        indices: None
                    })
                },
                BindingSuggestion {
                    server_id: "s2".into(),
                    binding: None
                },
            ]
        );
        assert_eq!(keys(&c)[4], "messages.gpuTest.unbound");

        let only_default = direct_profile(&[format!("{}/metrics", b.url())]);
        let c = check_gpu(&only_default, None, None, &PolicyStatus::default()).await;
        let counters: Vec<&str> = c
            .report
            .steps
            .iter()
            .filter(|s| s.name == "counters")
            .map(|s| s.message.key.as_str())
            .collect();
        assert_eq!(
            counters,
            [
                "messages.gpuTest.missingSm",
                "messages.gpuTest.missingEcc",
                "messages.gpuTest.missingThrottle",
                "messages.gpuTest.missingNvlink"
            ]
        );
        assert!(c.report.ok, "подсказки — не ошибки");
    }

    #[tokio::test]
    async fn address_without_a_path_or_with_a_slash_reads_metrics() {
        let a = TestServer::start(vec![("/metrics", Reply::text(FULL))]).await;
        for url in [
            a.url(),
            format!("{}/", a.url()),
            format!("{}/metrics/", a.url()),
        ] {
            let p = direct_profile(std::slice::from_ref(&url));
            let c = check_gpu(&p, None, None, &PolicyStatus::default()).await;
            assert!(c.report.ok, "{url}: {:?}", c.report);
            assert_eq!(keys(&c)[1], "messages.gpuTest.exporterCards", "{url}");
        }
    }

    #[tokio::test]
    async fn the_same_exporter_twice_warns_about_repeated_cards() {
        let a = TestServer::start(vec![("/metrics", Reply::text(FULL))]).await;
        let b = TestServer::start(vec![("/metrics", Reply::text(FULL))]).await;
        let (ua, ub) = (
            format!("{}/metrics", a.url()),
            format!("{}/metrics", b.url()),
        );
        let p = direct_profile(&[ua.clone(), ub.clone()]);
        let c = check_gpu(&p, None, None, &PolicyStatus::default()).await;
        assert!(
            c.report.ok,
            "предупреждение не валит проверку: {:?}",
            c.report
        );
        let dup: Vec<&TestStep> = c
            .report
            .steps
            .iter()
            .filter(|s| s.message.key == "messages.gpuTest.duplicateCards")
            .collect();
        assert_eq!(dup.len(), 1);
        assert_eq!(
            (dup[0].name, dup[0].ok, dup[0].warning),
            ("cards", true, true)
        );
        assert_eq!(dup[0].message.params["host"], "gpu-node-1");
        assert_eq!(dup[0].message.params["count"], "8");
        assert_eq!(dup[0].message.params["exporters"], format!("{ua}, {ub}"));

        let different = direct_profile(&[ua]);
        let c = check_gpu(&different, None, None, &PolicyStatus::default()).await;
        assert!(!keys(&c).contains(&"messages.gpuTest.duplicateCards"));
    }

    #[tokio::test]
    async fn a_history_client_error_is_a_history_step_in_direct_mode() {
        let a = TestServer::start(vec![("/metrics", Reply::text(FULL))]).await;
        let mut p = direct_profile(&[format!("{}/metrics", a.url())]);
        let mut h = history_source("http://prom:9090", HistoryAuth::None);
        h.url = "ftp://prom".into();
        p.history = Some(h);
        let c = check_gpu(&p, None, None, &PolicyStatus::default()).await;
        assert!(!c.report.ok);
        let step = c.report.steps.iter().find(|s| s.name == "history").unwrap();
        assert!(!step.ok && step.error.is_some(), "{step:?}");
        assert_eq!(
            keys(&c)[1],
            "messages.gpuTest.exporterCards",
            "экспортёры проверены"
        );
    }

    #[tokio::test]
    async fn exporter_failures_are_steps_and_saved_bindings_are_checked() {
        let ok = TestServer::start(vec![("/metrics", Reply::text(FULL))]).await;
        let refused = TestServer::start(vec![("/metrics", Reply::status(401))]).await;
        let other = TestServer::start(vec![("/metrics", Reply::text("# no dcgm\nup 1\n"))]).await;
        let mut p = direct_profile(&[
            format!("{}/metrics", ok.url()),
            format!("{}/metrics", refused.url()),
            format!("{}/metrics", other.url()),
            "http://127.0.0.1:1/metrics".into(),
        ]);
        p.servers[0].gpus = Some(GpuBinding {
            host: "gpu-node-1".into(),
            indices: Some(vec![1, 9]),
        });
        p.servers[1].gpus = Some(GpuBinding {
            host: "gpu-node-7".into(),
            indices: None,
        });
        let c = check_gpu(&p, None, None, &PolicyStatus::default()).await;
        assert!(!c.report.ok);
        assert_eq!(
            names(&c),
            [
                ("config", true, false),
                ("cards", true, false),
                ("auth", false, false),
                ("metrics", false, false),
                ("tcp", false, false),
                ("bind", true, true),
                ("bind", true, true),
            ]
        );
        assert_eq!(
            c.report.steps[2].error.as_ref().unwrap().key,
            "errors.gpu.unauthorized"
        );
        assert_eq!(
            c.report.steps[3].error.as_ref().unwrap().key,
            "errors.gpu.noDcgm"
        );
        assert_eq!(keys(&c)[5], "messages.gpuTest.boundCardsMissing");
        assert_eq!(c.report.steps[5].message.params["cards"], "9");
        assert_eq!(keys(&c)[6], "messages.gpuTest.boundHostMissing");
        assert!(
            c.bindings.is_empty(),
            "сохранённые привязки не подбираются заново"
        );
    }

    #[tokio::test]
    async fn exporters_are_matched_with_history_targets() {
        let a = TestServer::start(vec![("/metrics", Reply::text(FULL))]).await;
        let prom =
            TestServer::start(vec![("/api/v1/query", Reply::json(&snap("targets.json")))]).await;
        let mut p = direct_profile(&["http://127.0.0.1:9400/metrics".into()]);
        // Адрес экспортёра в профиле — как цель Prometheus; ответы — тестового сервера.
        p.gpu = match p.gpu.take() {
            Some(GpuSource::Direct(mut d)) => {
                d.exporters = vec![
                    sample_exporter(&format!("{}/metrics", a.url())),
                    sample_exporter("http://127.0.0.1:9401/metrics"),
                ];
                d.exporters[1].history_selector = None;
                Some(GpuSource::Direct(d))
            }
            other => other,
        };
        p.history = Some(history_source(&prom.url(), HistoryAuth::None));
        let c = check_gpu(&p, None, None, &PolicyStatus::default()).await;
        let history: Vec<&str> = c
            .report
            .steps
            .iter()
            .filter(|s| s.name == "history")
            .map(|s| s.message.key.as_str())
            .collect();
        assert_eq!(
            history,
            [
                "messages.gpuTest.historyNoTarget",
                "messages.gpuTest.historyAuto"
            ]
        );
        assert_eq!(
            c.exporters[1].history_selector.as_deref(),
            Some(r#"{job="dcgm", instance="127.0.0.1:9401"}"#)
        );
        assert_eq!(c.exporters[0].history_selector, None);
    }

    #[tokio::test]
    async fn prometheus_mode_auto_matches_the_selector_and_limits_the_count() {
        let prom =
            TestServer::start(vec![("/api/v1/query", Reply::json(&snap("cards.json")))]).await;
        let mut p = sample_profile("a");
        p.history = Some(history_source(&prom.url(), HistoryAuth::None));
        p.gpu = Some(GpuSource::Prometheus(PrometheusGpu {
            selector: None,
            extra: Default::default(),
        }));
        p.servers = vec![server("s1", "http://gpu-node-1:8000")];
        let c = check_gpu(&p, None, None, &PolicyStatus::default()).await;
        assert!(c.report.ok, "{:?}", c.report);
        assert_eq!(
            keys(&c),
            [
                "messages.connectionTest.configOk",
                "messages.gpuTest.selectorAuto",
                "messages.gpuTest.historyCards",
                "messages.gpuTest.boundAuto"
            ]
        );
        assert_eq!(c.selector.as_deref(), Some(r#"{hostname=~"gpu-node-1"}"#));
        assert_eq!(c.available, ["gpu-node-1", "localhost"]);
        assert_eq!(
            c.hosts,
            [FoundHost {
                host: "gpu-node-1".into(),
                cards: (0..8).collect()
            }]
        );

        p.servers = vec![server("s1", "http://lb:8000")];
        let c = check_gpu(&p, None, None, &PolicyStatus::default()).await;
        assert_eq!(keys(&c)[1], "messages.gpuTest.noServerHosts");
        assert_eq!(c.selector, None);
        assert_eq!(c.hosts.len(), 2, "без селектора — все карты источника");

        let many: Vec<serde_json::Value> = (0..65)
            .map(|i| serde_json::json!({ "metric": { "hostname": format!("h{}", i / 8), "gpu": (i % 8).to_string() }, "value": [0, "1"] }))
            .collect();
        let body = serde_json::json!({ "status": "success", "data": { "resultType": "vector", "result": many } }).to_string();
        let big = TestServer::start(vec![("/api/v1/query", Reply::json(&body))]).await;
        p.history = Some(history_source(&big.url(), HistoryAuth::None));
        p.gpu = Some(GpuSource::Prometheus(PrometheusGpu {
            selector: Some(r#"{job="dcgm"}"#.into()),
            extra: Default::default(),
        }));
        let c = check_gpu(&p, None, None, &PolicyStatus::default()).await;
        assert!(!c.report.ok);
        assert_eq!(
            c.report.steps.last().unwrap().error.as_ref().unwrap().key,
            "errors.gpu.tooManyGpus"
        );
    }

    #[tokio::test]
    async fn configuration_errors_stop_the_check() {
        let mut p = sample_profile("a");
        let c = check_gpu(&p, None, None, &PolicyStatus::default()).await;
        assert_eq!(
            c.report.steps[0].error.as_ref().unwrap().key,
            "errors.gpu.notConfigured"
        );
        p.gpu = Some(GpuSource::Prometheus(PrometheusGpu {
            selector: None,
            extra: Default::default(),
        }));
        let c = check_gpu(&p, None, None, &PolicyStatus::default()).await;
        assert_eq!(
            c.report.steps[0].error.as_ref().unwrap().key,
            "errors.gpu.noHistorySource"
        );
        assert_eq!(c.report.steps.len(), 1);
    }

    #[test]
    fn found_cards_take_the_host_label_in_order() {
        let samples: Vec<Sample> = serde_json::from_str::<serde_json::Value>(&snap("cards.json"))
            .map(|v| crate::history::client::parse_vector(&v["data"]).unwrap())
            .unwrap();
        let cards = found_cards(&samples);
        assert_eq!(cards.len(), 10);
        assert!(cards.iter().all(|c| c.label == HostLabel::Lower));
        let s = |labels: &[(&str, &str)]| Sample {
            labels: labels
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            value: Some(1.0),
        };
        let mixed = found_cards(&[
            s(&[("Hostname", "old"), ("gpu", "0")]),
            s(&[("instance", "10.0.0.5:9400"), ("gpu", "1")]),
        ]);
        assert_eq!(
            mixed,
            [
                FoundCard {
                    host: "10.0.0.5".into(),
                    label: HostLabel::Instance,
                    index: 1
                },
                FoundCard {
                    host: "old".into(),
                    label: HostLabel::Upper,
                    index: 0
                }
            ]
        );
    }
}
