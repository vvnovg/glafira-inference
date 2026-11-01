//! Проверка источника истории (спека I1c §3): конфигурация → DNS/TCP → вход (`buildinfo`) →
//! запрос `up` → цели инференса → сопоставление каждого сервера. Сбой источника не мешает
//! подключению серверов: проверка только сообщает его.

use std::time::Duration;

use serde::Serialize;

use crate::error::{AppError, ErrorCode};
use crate::history::client::{BuildInfo, HistoryClient, Sample};
use crate::history::targets::{Target, TargetKind, matching_targets, parse_selector, render};
use crate::i18n::LocalizedText;
use crate::policy::PolicyStatus;
use crate::servers::client::Timeouts;
use crate::servers::connectivity::{
    TestReport, TestStep, failed_step, passed, report, tcp_failure, tcp_probe, tcp_target,
};
use crate::storage::profile::{
    InstallationProfile, ServerConfig, parse_base_url, parse_history_url, validate_history,
};

/// Запрос целей инференса: пары `job`, `instance` с сериями vLLM или SGLang.
pub const DISCOVER: &str = r#"count by (job, instance) ({__name__=~"vllm:.+|sglang:.+"})"#;
/// Те же цели SGLang: остальные — vLLM.
pub const DISCOVER_SGLANG: &str = r#"count by (job, instance) ({__name__=~"sglang:.+"})"#;

/// Селектор, который форма может поставить серверу.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub server_id: String,
    /// Селектор подобранной цели; `None` — цель не найдена.
    pub selector: Option<String>,
    /// Подходящие цели (по `host:port`), первая — подобранная.
    pub targets: Vec<Target>,
}

/// Итог проверки для формы и проверки инсталляции.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryCheck {
    pub report: TestReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<BuildInfo>,
    /// Найденные цели инференса (пусто, если до шага поиска проверка не дошла).
    pub targets: Vec<Target>,
    /// Автоподбор для серверов без селектора.
    pub suggestions: Vec<Suggestion>,
}

/// Цели из ответов поиска: все и цели SGLang.
pub fn targets_from(all: &[Sample], sglang: &[Sample]) -> Vec<Target> {
    let pair = |s: &Sample| {
        Some((
            s.labels.get("job")?.clone(),
            s.labels.get("instance")?.clone(),
        ))
    };
    let sglang: Vec<_> = sglang.iter().filter_map(pair).collect();
    let mut out: Vec<Target> = all
        .iter()
        .filter_map(pair)
        .map(|(job, instance)| {
            let kind = if sglang.contains(&(job.clone(), instance.clone())) {
                TargetKind::Sglang
            } else {
                TargetKind::Vllm
            };
            Target {
                job,
                instance,
                kind,
            }
        })
        .collect();
    out.sort();
    out
}

fn warning_step(name: &'static str, message: LocalizedText) -> TestStep {
    TestStep {
        name,
        ok: true,
        message,
        error: None,
    }
}

/// Шаг сопоставления сервера: у сервера с селектором — есть ли под ним серии; без селектора —
/// подобранная цель.
async fn match_step(
    client: &HistoryClient,
    server: &ServerConfig,
    targets: &[Target],
    warnings: &mut Vec<LocalizedText>,
) -> (TestStep, Option<Suggestion>) {
    let name = &server.name;
    if let Some(selector) = &server.history_selector {
        let matchers = match parse_selector(selector) {
            Ok(m) => m,
            Err(e) => return (failed_step("match", e), None),
        };
        let expr = format!(
            r#"count({{{},__name__=~"vllm:.+|sglang:.+"}})"#,
            render(&matchers)
        );
        return match client.query(&expr).await {
            Ok(found) if found.iter().any(|s| s.value.is_some_and(|v| v > 0.0)) => (
                passed(
                    "match",
                    ltext!(
                        "messages.historyTest.matchSelector",
                        "“{{server}}”: {{selector}}",
                        server = name,
                        selector = selector
                    ),
                ),
                None,
            ),
            Ok(_) => {
                let w = ltext!(
                    "messages.historyTest.noSeries",
                    "“{{server}}”: there are no vLLM or SGLang series under the selector {{selector}}",
                    server = name,
                    selector = selector
                );
                warnings.push(w.clone());
                (warning_step("match", w), None)
            }
            Err(e) => (failed_step("match", e), None),
        };
    }
    let found: Vec<Target> = parse_base_url(server)
        .map(|url| {
            matching_targets(&url, targets)
                .into_iter()
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    match found.first() {
        Some(t) => (
            passed(
                "match",
                ltext!(
                    "messages.historyTest.matchAuto",
                    "“{{server}}”: {{job}} · {{instance}} (auto-match)",
                    server = name,
                    job = t.job,
                    instance = t.instance
                ),
            ),
            Some(Suggestion {
                server_id: server.id.clone(),
                selector: Some(t.selector()),
                targets: found.clone(),
            }),
        ),
        None => {
            let w = ltext!(
                "messages.historyTest.noTarget",
                "“{{server}}”: no target with the address of the server — choose the target manually",
                server = name
            );
            warnings.push(w.clone());
            (
                warning_step("match", w),
                Some(Suggestion {
                    server_id: server.id.clone(),
                    selector: None,
                    targets: Vec::new(),
                }),
            )
        }
    }
}

/// Проверка источника истории профиля `p`; `secret` — токен или пароль из формы или keychain.
pub async fn check_source(
    p: &InstallationProfile,
    secret: Option<String>,
    policy: &PolicyStatus,
) -> HistoryCheck {
    let finish = |steps, warnings, source, targets, suggestions| HistoryCheck {
        report: report(steps, warnings),
        source,
        targets,
        suggestions,
    };
    let Some(h) = &p.history else {
        let e = app_err!(
            ErrorCode::InvalidInput,
            "errors.history.noSource",
            "The installation has no metrics history source"
        );
        return finish(
            vec![failed_step("config", e)],
            Vec::new(),
            None,
            Vec::new(),
            Vec::new(),
        );
    };
    let timeouts = Timeouts::from(p);
    let prepared = validate_history(h)
        .and_then(|()| parse_history_url(h))
        .and_then(|url| Ok((url, HistoryClient::new(h, timeouts, secret, policy)?)));
    let (url, client) = match prepared {
        Ok(x) => x,
        Err(e) => {
            return finish(
                vec![failed_step("config", e)],
                Vec::new(),
                None,
                Vec::new(),
                Vec::new(),
            );
        }
    };
    let mut steps = vec![passed(
        "config",
        ltext!("messages.connectionTest.configOk", "Configuration is valid"),
    )];
    let mut warnings = Vec::new();

    let as_server = crate::history::client::as_server(h);
    if let Some((host, port, via_proxy)) = tcp_target(&as_server, &url) {
        let limit = Duration::from_millis(timeouts.connect_ms);
        match tcp_probe(&host, port, limit).await {
            Ok(address) => steps.push(passed(
                "tcp",
                ltext!(
                    "messages.connectionTest.tcpOk",
                    "TCP connection to {{address}} established",
                    address = address
                ),
            )),
            Err(e) => {
                steps.push(failed_step("tcp", tcp_failure(e, &host, port, via_proxy)));
                return finish(steps, warnings, None, Vec::new(), Vec::new());
            }
        }
    }

    let source = match client.buildinfo().await {
        Ok(info) => {
            let message = match &info {
                Some(BuildInfo {
                    version: Some(v), ..
                }) => ltext!(
                    "messages.historyTest.source",
                    "{{kind}} {{version}}",
                    kind = source_label(info.as_ref()),
                    version = v
                ),
                Some(_) => ltext!(
                    "messages.historyTest.sourceKind",
                    "{{kind}}",
                    kind = source_label(info.as_ref())
                ),
                None => ltext!(
                    "messages.historyTest.noBuildinfo",
                    "The source has no buildinfo — sign-in is checked by the next step"
                ),
            };
            steps.push(passed("auth", message));
            info
        }
        Err(e) => {
            let step = if e.code == ErrorCode::TlsError {
                "tls"
            } else {
                "auth"
            };
            steps.push(failed_step(step, e));
            return finish(steps, warnings, None, Vec::new(), Vec::new());
        }
    };

    match client.query("up").await {
        Ok(up) => {
            let alive = up.iter().filter(|s| s.value == Some(1.0)).count();
            steps.push(passed(
                "up",
                ltext!(
                    "messages.historyTest.up",
                    "Targets up: {{alive}} of {{total}}",
                    alive = alive,
                    total = up.len()
                ),
            ));
        }
        Err(e) => {
            steps.push(failed_step("up", e));
            return finish(steps, warnings, source, Vec::new(), Vec::new());
        }
    }

    let targets = match (
        client.query(DISCOVER).await,
        client.query(DISCOVER_SGLANG).await,
    ) {
        (Ok(all), Ok(sglang)) => targets_from(&all, &sglang),
        (Err(e), _) | (_, Err(e)) => {
            steps.push(failed_step("targets", e));
            return finish(steps, warnings, source, Vec::new(), Vec::new());
        }
    };
    if targets.is_empty() {
        let w = ltext!(
            "messages.historyTest.noTargets",
            "The source does not collect vLLM or SGLang metrics"
        );
        warnings.push(w.clone());
        steps.push(warning_step("targets", w));
    } else {
        steps.push(passed(
            "targets",
            ltext!(
                "messages.historyTest.targets",
                "Inference targets found: {{count}}",
                count = targets.len()
            ),
        ));
    }

    let mut suggestions = Vec::new();
    for server in &p.servers {
        let (step, suggestion) = match_step(&client, server, &targets, &mut warnings).await;
        steps.push(step);
        suggestions.extend(suggestion);
    }
    finish(steps, warnings, source, targets, suggestions)
}

/// Название источника для шага «Вход».
fn source_label(info: Option<&BuildInfo>) -> &'static str {
    match info.map(|i| i.kind) {
        Some(crate::history::client::SourceKind::Victoriametrics) => "VictoriaMetrics",
        _ => "Prometheus",
    }
}

/// Ошибка «нет источника истории» — для команд чтения.
pub fn no_source() -> AppError {
    app_err!(
        ErrorCode::InvalidInput,
        "errors.history.noSource",
        "The installation has no metrics history source"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::client::tests::{snapshot, source};
    use crate::servers::testing::{Reply, TestServer};
    use crate::storage::profile::{HistoryAuth, ProxyConfig, sample_profile, sample_server};

    fn query_path(expr: &str) -> String {
        let q = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("query", expr)
            .finish();
        format!("/api/v1/query?{q}")
    }

    fn server(id: &str, url: &str, selector: Option<&str>) -> ServerConfig {
        let mut s = sample_server(id);
        s.name = id.into();
        s.base_url = url.into();
        s.proxy = ProxyConfig::None;
        s.history_selector = selector.map(String::from);
        s
    }

    async fn prometheus() -> TestServer {
        let one = r#"{"status":"success","data":{"resultType":"vector","result":[{"metric":{},"value":[1,"57"]}]}}"#;
        let routes: Vec<(String, Reply)> = vec![
            (
                "/api/v1/status/buildinfo".into(),
                Reply::json(&snapshot("buildinfo.json")),
            ),
            (query_path("up"), Reply::json(&snapshot("up.json"))),
            (
                query_path(DISCOVER),
                Reply::json(&snapshot("discover.json")),
            ),
            (
                query_path(DISCOVER_SGLANG),
                Reply::json(&snapshot("discover-sglang.json")),
            ),
            (
                query_path(
                    r#"count({job="vllm",instance="127.0.0.1:8001",__name__=~"vllm:.+|sglang:.+"})"#,
                ),
                Reply::json(one),
            ),
            (
                query_path(r#"count({job="gone",__name__=~"vllm:.+|sglang:.+"})"#),
                Reply::json(&snapshot("empty.json")),
            ),
        ];
        let routes: Vec<(&str, Reply)> = routes
            .iter()
            .map(|(p, r)| (p.as_str(), r.clone()))
            .collect();
        TestServer::start(routes).await
    }

    #[tokio::test]
    async fn steps_targets_and_suggestions() {
        let prom = prometheus().await;
        let mut p = sample_profile("a");
        p.history = Some(source(&prom.url(), HistoryAuth::None));
        p.servers = vec![
            server("auto", "http://localhost:8000", None),
            server(
                "chosen",
                "http://vllm-svc:8000",
                Some(r#"{job="vllm", instance="127.0.0.1:8001"}"#),
            ),
            server("stale", "http://10.0.0.9:8000", Some(r#"{job="gone"}"#)),
            server("unknown", "http://10.0.0.9:8000", None),
        ];
        let c = check_source(&p, None, &PolicyStatus::default()).await;
        let names: Vec<_> = c.report.steps.iter().map(|s| s.name).collect();
        assert_eq!(
            names,
            [
                "config", "tcp", "auth", "up", "targets", "match", "match", "match", "match"
            ]
        );
        assert!(c.report.ok, "{:?}", c.report.steps);
        assert_eq!(c.report.steps[2].message.params["version"], "3.15.1");
        assert_eq!(c.report.steps[3].message.params["alive"], "4");
        assert_eq!(c.targets.len(), 4);
        assert_eq!(
            c.targets
                .iter()
                .filter(|t| t.kind == TargetKind::Sglang)
                .count(),
            1
        );
        let keys: Vec<_> = c.report.steps[5..]
            .iter()
            .map(|s| s.message.key.as_str())
            .collect();
        assert_eq!(
            keys,
            [
                "messages.historyTest.matchAuto",
                "messages.historyTest.matchSelector",
                "messages.historyTest.noSeries",
                "messages.historyTest.noTarget"
            ]
        );
        assert_eq!(c.report.warnings.len(), 2);
        assert_eq!(
            c.suggestions,
            [
                Suggestion {
                    server_id: "auto".into(),
                    selector: Some(r#"{job="vllm", instance="127.0.0.1:8000"}"#.into()),
                    targets: vec![Target {
                        job: "vllm".into(),
                        instance: "127.0.0.1:8000".into(),
                        kind: TargetKind::Vllm
                    }],
                },
                Suggestion {
                    server_id: "unknown".into(),
                    selector: None,
                    targets: vec![],
                },
            ]
        );
    }

    #[tokio::test]
    async fn unauthorized_source_stops_at_sign_in() {
        let prom = TestServer::start(vec![("/api/v1/status/buildinfo", Reply::status(401))]).await;
        let mut p = sample_profile("a");
        p.history = Some(source(&prom.url(), HistoryAuth::Bearer));
        let c = check_source(&p, Some("t".into()), &PolicyStatus::default()).await;
        let last = c.report.steps.last().unwrap();
        assert_eq!((last.name, last.ok), ("auth", false));
        assert_eq!(
            last.error.as_ref().unwrap().key,
            "errors.history.unauthorized"
        );
        assert!(!c.report.ok);
    }

    #[tokio::test]
    async fn no_source_and_bad_config_fail_the_first_step() {
        let c = check_source(&sample_profile("a"), None, &PolicyStatus::default()).await;
        assert_eq!(
            c.report.steps[0].error.as_ref().unwrap().key,
            "errors.history.noSource"
        );
        let mut p = sample_profile("a");
        p.history = Some(source("prom:9090", HistoryAuth::None));
        let c = check_source(&p, None, &PolicyStatus::default()).await;
        assert_eq!(
            c.report.steps[0].error.as_ref().unwrap().key,
            "errors.history.badUrl"
        );
    }

    #[test]
    fn targets_are_typed_by_the_sglang_query() {
        let v = |name: &str| {
            let data: serde_json::Value = serde_json::from_str(&snapshot(name)).unwrap();
            crate::history::client::parse_vector(&data["data"]).unwrap()
        };
        let t = targets_from(&v("discover.json"), &v("discover-sglang.json"));
        assert_eq!(
            t.iter()
                .map(|t| (t.job.as_str(), t.kind))
                .collect::<Vec<_>>(),
            [
                ("sglang", TargetKind::Sglang),
                ("vllm", TargetKind::Vllm),
                ("vllm", TargetKind::Vllm),
                ("vllm", TargetKind::Vllm)
            ]
        );
    }
}
