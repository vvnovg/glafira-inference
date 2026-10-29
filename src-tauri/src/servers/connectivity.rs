//! Пошаговая проверка подключения сервера (спец. §5.8): конфигурация → DNS/TCP → TLS → `/health` →
//! вход → тип и версия → модели. Останавливается на первом неуспешном шаге и называет его;
//! предупреждения (старая версия, другой тип в профиле, нет метрик) успеха не отменяют.

use std::time::Duration;

use serde::Serialize;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::i18n::LocalizedText;
use crate::policy::PolicyStatus;
use crate::servers::client::{JSON_LIMIT, ServerClient, Timeouts};
use crate::servers::detect::{ServerFacts, analyze};
use crate::servers::errors::timeout;
use crate::servers::probe::probe;
use crate::storage::profile::{ProxyConfig, ServerConfig, parse_base_url, validate_server};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestStep {
    pub name: &'static str,
    pub ok: bool,
    pub message: LocalizedText,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AppError>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestReport {
    pub ok: bool,
    pub steps: Vec<TestStep>,
    pub warnings: Vec<LocalizedText>,
}

fn passed(name: &'static str, message: LocalizedText) -> TestStep {
    TestStep {
        name,
        ok: true,
        message,
        error: None,
    }
}

fn failed_step(name: &'static str, e: AppError) -> TestStep {
    TestStep {
        name,
        ok: false,
        message: e.text(),
        error: Some(e),
    }
}

fn report(steps: Vec<TestStep>, warnings: Vec<LocalizedText>) -> TestReport {
    TestReport {
        ok: steps.iter().all(|s| s.ok),
        steps,
        warnings,
    }
}

/// Куда открыть TCP: прокси из профиля или сервер (порт по умолчанию — 80 или 443). `None` —
/// системный прокси из окружения: прямое соединение ничего бы не доказало.
pub fn tcp_target(s: &ServerConfig, url: &url::Url) -> Option<(String, u16, bool)> {
    let proxy = match &s.proxy {
        ProxyConfig::Url { url } => url::Url::parse(url.trim()).ok(),
        ProxyConfig::System if system_proxy_set(url.scheme()) => return None,
        _ => None,
    };
    let (target, via_proxy) = match &proxy {
        Some(p) => (p, true),
        None => (url, false),
    };
    let host = target
        .host_str()?
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_string();
    Some((host, target.port_or_known_default()?, via_proxy))
}

/// Задан ли системный прокси для схемы адреса (`HTTPS_PROXY` / `HTTP_PROXY`, `ALL_PROXY`).
fn system_proxy_set(scheme: &str) -> bool {
    let names: &[&str] = if scheme == "https" {
        &["HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"]
    } else {
        &["HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"]
    };
    names
        .iter()
        .any(|n| std::env::var(n).is_ok_and(|v| !v.trim().is_empty()))
}

/// DNS и TCP-соединение с `host:port` за отведённое время. Успех — адрес, к которому подключились.
pub async fn tcp_probe(host: &str, port: u16, limit: Duration) -> AppResult<String> {
    let addrs: Vec<_> =
        match tokio::time::timeout(limit, tokio::net::lookup_host((host, port))).await {
            Ok(Ok(a)) => a.collect(),
            Ok(Err(e)) => {
                return Err(app_err!(
                    ErrorCode::ConnectionFailed,
                    "errors.server.dns",
                    "Cannot resolve the server host name"
                )
                .with_details(e.to_string()));
            }
            Err(_) => return Err(timeout()),
        };
    let mut last = None;
    for addr in addrs {
        match tokio::time::timeout(limit, tokio::net::TcpStream::connect(addr)).await {
            Ok(Ok(_)) => return Ok(addr.to_string()),
            Ok(Err(e)) => last = Some(e.to_string()),
            Err(_) => last = Some("timed out".into()),
        }
    }
    Err(app_err!(
        ErrorCode::ConnectionFailed,
        "errors.server.connect",
        "Cannot connect to the server"
    )
    .with_details(last.unwrap_or_else(|| format!("{host}: no addresses"))))
}

/// Сколько моделей шаг «Модели» называет по имени; остальные — числом.
const SHOWN_MODELS: usize = 5;

fn models_message(f: &ServerFacts) -> LocalizedText {
    let mut list = f
        .models
        .iter()
        .take(SHOWN_MODELS)
        .map(|m| match m.max_model_len {
            Some(len) => format!("{} ({len})", m.id),
            None => m.id.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ");
    if f.models.len() > SHOWN_MODELS {
        list.push_str(&format!(", +{}", f.models.len() - SHOWN_MODELS));
    }
    ltext!(
        "messages.connectionTest.models",
        "Models: {{models}}",
        models = list
    )
}

fn kind_message(f: &ServerFacts) -> LocalizedText {
    match &f.version {
        Some(v) => ltext!(
            "messages.connectionTest.kindVersion",
            "{{kind}} {{version}}",
            kind = f.detected.label(),
            version = v
        ),
        None => ltext!(
            "messages.connectionTest.kind",
            "{{kind}}",
            kind = f.detected.label()
        ),
    }
}

/// Проверка одного сервера. `api_key` — из формы или keychain; пустой — без ключа.
pub async fn test_server(
    s: &ServerConfig,
    timeouts: Timeouts,
    api_key: Option<String>,
    policy: &PolicyStatus,
) -> TestReport {
    let mut steps = Vec::new();
    let has_key = api_key.as_deref().is_some_and(|k| !k.trim().is_empty());
    let prepared = validate_server(s)
        .and_then(|()| parse_base_url(s))
        .and_then(|url| Ok((url, ServerClient::new(s, timeouts, api_key, policy)?)));
    let (url, client) = match prepared {
        Ok(p) => p,
        Err(e) => return report(vec![failed_step("config", e)], Vec::new()),
    };
    steps.push(passed(
        "config",
        ltext!("messages.connectionTest.configOk", "Configuration is valid"),
    ));

    let limit = Duration::from_millis(timeouts.connect_ms);
    match tcp_target(s, &url) {
        None => steps.push(passed(
            "tcp",
            ltext!(
                "messages.connectionTest.tcpSystemProxy",
                "The system proxy is used: the connection is checked by the requests below"
            ),
        )),
        Some((host, port, via_proxy)) => match tcp_probe(&host, port, limit).await {
            Ok(address) if via_proxy => steps.push(passed(
                "tcp",
                ltext!(
                    "messages.connectionTest.tcpProxyOk",
                    "TCP connection to the proxy {{address}} established",
                    address = address
                ),
            )),
            Ok(address) => steps.push(passed(
                "tcp",
                ltext!(
                    "messages.connectionTest.tcpOk",
                    "TCP connection to {{address}} established",
                    address = address
                ),
            )),
            Err(e) => {
                steps.push(failed_step("tcp", e));
                return report(steps, Vec::new());
            }
        },
    }

    // `/health`: любой ответ сервера доказывает, что TLS-рукопожатие прошло.
    let https = url.scheme() == "https";
    let health = match client.get("/health", JSON_LIMIT).await {
        Ok(r) => r,
        Err(e) => {
            let step = if https && e.code == ErrorCode::TlsError {
                "tls"
            } else {
                "health"
            };
            steps.push(failed_step(step, e));
            return report(steps, Vec::new());
        }
    };
    if https {
        steps.push(passed(
            "tls",
            ltext!(
                "messages.connectionTest.tlsOk",
                "TLS connection established, the certificate is trusted"
            ),
        ));
    }
    match health.status {
        200..=299 => steps.push(passed(
            "health",
            ltext!("messages.connectionTest.healthOk", "The server is healthy"),
        )),
        404 => steps.push(passed(
            "health",
            ltext!(
                "messages.connectionTest.healthMissing",
                "The server has no /health — skipped"
            ),
        )),
        401 | 403 => steps.push(passed(
            "health",
            ltext!(
                "messages.connectionTest.healthNeedsKey",
                "/health requires an API key — checked by the next step"
            ),
        )),
        status => {
            steps.push(failed_step(
                "health",
                app_err!(
                    ErrorCode::ConnectionFailed,
                    "errors.server.unhealthy",
                    "The server is not healthy (/health answered {{status}}): it may still be loading the model",
                    status = status
                ),
            ));
            return report(steps, Vec::new());
        }
    }

    let snapshot = match probe(&client, &s.metrics_path).await {
        Ok(p) => p,
        Err(e) if e.code == ErrorCode::AuthFailed && !has_key => {
            steps.push(failed_step(
                "auth",
                app_err!(
                    ErrorCode::AuthFailed,
                    "errors.server.keyRequired",
                    "The server requires an API key — enter it in the server settings"
                )
                .with_details(e.message),
            ));
            return report(steps, Vec::new());
        }
        Err(e) if e.code == ErrorCode::AuthFailed => {
            steps.push(failed_step("auth", e));
            return report(steps, Vec::new());
        }
        Err(e) if e.code == ErrorCode::NotFound => {
            steps.push(failed_step(
                "auth",
                app_err!(
                    ErrorCode::NotFound,
                    "errors.server.notOpenAi",
                    "The server has no /v1/models — is this the address of an OpenAI-compatible server?"
                ),
            ));
            return report(steps, Vec::new());
        }
        Err(e) => {
            steps.push(failed_step("auth", e));
            return report(steps, Vec::new());
        }
    };
    steps.push(passed(
        "auth",
        if has_key {
            ltext!(
                "messages.connectionTest.keyAccepted",
                "The API key is accepted"
            )
        } else {
            ltext!(
                "messages.connectionTest.noKeyNeeded",
                "The server answers without an API key"
            )
        },
    ));

    let facts = match analyze(s.kind, &snapshot) {
        Ok(f) => f,
        Err(e) => {
            steps.push(failed_step("kind", e));
            return report(steps, Vec::new());
        }
    };
    steps.push(passed("kind", kind_message(&facts)));
    if facts.models.is_empty() {
        steps.push(failed_step(
            "models",
            app_err!(
                ErrorCode::NotFound,
                "errors.server.noModels",
                "The server serves no models"
            ),
        ));
    } else {
        steps.push(passed("models", models_message(&facts)));
    }
    report(steps, facts.warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::servers::testing::{Reply, TEST_CA_PEM, TestServer};
    use crate::storage::profile::{CaConfig, sample_server};

    const TIMEOUTS: Timeouts = Timeouts {
        connect_ms: 2_000,
        request_ms: 2_000,
    };

    fn names(r: &TestReport) -> Vec<(&str, bool)> {
        r.steps.iter().map(|s| (s.name, s.ok)).collect()
    }

    fn server(url: &str) -> ServerConfig {
        let mut s = sample_server("s1");
        s.base_url = url.into();
        s.proxy = ProxyConfig::None;
        s
    }

    fn vllm(health: Reply) -> Vec<(&'static str, Reply)> {
        vec![
            ("/health", health),
            (
                "/v1/models",
                Reply::json(include_str!("testdata/vllm-0.11.0/models.json")),
            ),
            (
                "/version",
                Reply::json(include_str!("testdata/vllm-0.11.0/version.json")),
            ),
            (
                "/metrics",
                Reply::text(include_str!("testdata/vllm-0.11.0/metrics.txt")),
            ),
        ]
    }

    async fn run(s: &ServerConfig, key: Option<&str>) -> TestReport {
        test_server(s, TIMEOUTS, key.map(String::from), &PolicyStatus::default()).await
    }

    #[tokio::test]
    async fn all_steps_pass_on_vllm_over_http() {
        let srv = TestServer::start(vllm(Reply::status(200))).await;
        let r = run(&server(&srv.url()), Some("sk-1")).await;
        assert!(r.ok);
        assert_eq!(
            names(&r),
            [
                ("config", true),
                ("tcp", true),
                ("health", true),
                ("auth", true),
                ("kind", true),
                ("models", true)
            ]
        );
        assert_eq!(r.steps[1].message.params["address"], srv.addr.to_string());
        assert_eq!(
            r.steps[3].message.key,
            "messages.connectionTest.keyAccepted"
        );
        assert_eq!(r.steps[4].message.params["kind"], "vLLM");
        assert_eq!(r.steps[4].message.params["version"], "0.11.0");
        assert_eq!(r.steps[5].message.params["models"], "qwen3-32b (32768)");
        assert!(r.warnings.is_empty());
    }

    #[tokio::test]
    async fn https_adds_a_tls_step_and_stops_on_an_untrusted_certificate() {
        let srv = TestServer::start_tls(vllm(Reply::status(200))).await;
        let mut s = server(&srv.url());
        s.ca = CaConfig::Pem {
            pem: TEST_CA_PEM.into(),
        };
        let r = run(&s, None).await;
        assert!(r.ok);
        assert_eq!(r.steps[2].name, "tls");
        assert_eq!(
            r.steps[4].message.key,
            "messages.connectionTest.noKeyNeeded"
        );
        s.ca = CaConfig::System;
        let r = run(&s, None).await;
        assert_eq!(names(&r), [("config", true), ("tcp", true), ("tls", false)]);
        assert_eq!(r.steps[2].error.as_ref().unwrap().code, ErrorCode::TlsError);
    }

    #[tokio::test]
    async fn config_and_tcp_failures_stop_early() {
        let mut s = server("https://vllm:8000");
        s.ca = CaConfig::Insecure;
        let policy = PolicyStatus {
            allow_insecure_tls: false,
            ..PolicyStatus::default()
        };
        let r = test_server(&s, TIMEOUTS, None, &policy).await;
        assert_eq!(names(&r), [("config", false)]);
        assert_eq!(
            r.steps[0].message.key,
            "errors.installations.insecureForbidden"
        );
        let r = run(&server("ftp://h"), None).await;
        assert_eq!(r.steps[0].message.key, "errors.installations.badServerUrl");
        let closed = Timeouts {
            connect_ms: 10_000,
            request_ms: 10_000,
        };
        let r = test_server(
            &server("http://127.0.0.1:1"),
            closed,
            None,
            &PolicyStatus::default(),
        )
        .await;
        assert_eq!(names(&r), [("config", true), ("tcp", false)]);
    }

    #[tokio::test]
    async fn health_answers() {
        let srv = TestServer::start(vllm(Reply::status(503))).await;
        let r = run(&server(&srv.url()), None).await;
        assert_eq!(names(&r).last(), Some(&("health", false)));
        assert_eq!(r.steps[2].message.key, "errors.server.unhealthy");
        // Ollama: /health нет — шаг пропущен, проверка идёт дальше.
        let srv = TestServer::start(vec![(
            "/v1/models",
            Reply::json(include_str!("testdata/ollama/models.json")),
        )])
        .await;
        let r = run(&server(&srv.url()), None).await;
        assert!(r.ok);
        assert_eq!(
            r.steps[2].message.key,
            "messages.connectionTest.healthMissing"
        );
        assert_eq!(r.steps[4].message.params["kind"], "OpenAI-compatible");
    }

    #[tokio::test]
    async fn missing_and_rejected_keys_are_told_apart() {
        let srv = TestServer::start(vec![
            ("/health", Reply::status(401)),
            ("/v1/models", Reply::status(401)),
        ])
        .await;
        let r = run(&server(&srv.url()), None).await;
        assert_eq!(names(&r).last(), Some(&("auth", false)));
        assert_eq!(
            r.steps[2].message.key,
            "messages.connectionTest.healthNeedsKey"
        );
        assert_eq!(r.steps[3].message.key, "errors.server.keyRequired");
        let r = run(&server(&srv.url()), Some("sk-wrong")).await;
        assert_eq!(r.steps[3].message.key, "errors.server.unauthorized");
    }

    #[tokio::test]
    async fn not_an_openai_server_and_no_models() {
        let srv = TestServer::start(vec![("/health", Reply::status(200))]).await;
        let r = run(&server(&srv.url()), None).await;
        assert_eq!(
            r.steps.last().unwrap().message.key,
            "errors.server.notOpenAi"
        );
        let srv = TestServer::start(vec![(
            "/v1/models",
            Reply::json(r#"{"object":"list","data":[]}"#),
        )])
        .await;
        let r = run(&server(&srv.url()), None).await;
        assert_eq!(names(&r).last(), Some(&("models", false)));
        assert!(!r.ok);
    }

    #[tokio::test]
    async fn warnings_do_not_fail_the_test() {
        let srv = TestServer::start(vec![
            (
                "/v1/models",
                Reply::json(include_str!("testdata/vllm-0.7.3/models.json")),
            ),
            (
                "/version",
                Reply::json(include_str!("testdata/vllm-0.7.3/version.json")),
            ),
        ])
        .await;
        let r = run(&server(&srv.url()), None).await;
        assert!(r.ok);
        assert_eq!(r.warnings[0].key, "messages.servers.unsupportedVersion");
        assert_eq!(r.steps[4].message.params["version"], "0.7.3");
    }

    #[tokio::test]
    async fn tcp_goes_to_the_proxy_of_the_profile() {
        let proxy = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = proxy.local_addr().unwrap().port();
        let mut s = server("http://vllm.internal:8000");
        s.proxy = ProxyConfig::Url {
            url: format!("http://127.0.0.1:{port}"),
        };
        let url = parse_base_url(&s).unwrap();
        assert_eq!(tcp_target(&s, &url), Some(("127.0.0.1".into(), port, true)));
        let s = server("https://vllm.internal");
        let url = parse_base_url(&s).unwrap();
        assert_eq!(
            tcp_target(&s, &url),
            Some(("vllm.internal".into(), 443, false))
        );
    }

    #[tokio::test]
    async fn tcp_probe_reaches_listener_and_reports_refused_port() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let addr = tcp_probe("127.0.0.1", port, Duration::from_secs(2))
            .await
            .unwrap();
        assert_eq!(addr, format!("127.0.0.1:{port}"));
        // Порт 1 закрыт; срок с запасом: на Windows отказ приходит примерно через 2 с.
        let e = tcp_probe("127.0.0.1", 1, Duration::from_secs(10))
            .await
            .unwrap_err();
        assert_eq!(e.code, ErrorCode::ConnectionFailed);
    }
}
