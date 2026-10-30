//! Снимок ответов сервера для определения типа (спец. §4.3). Запросы идут параллельно;
//! обязателен только `/v1/models`, ошибка остальных путей снимок не прерывает.

use crate::error::AppResult;
use crate::servers::client::{JSON_LIMIT, ServerClient};
use crate::servers::detect::{Answer, Probe, ServerFacts, analyze};
use crate::storage::profile::ServerConfig;

/// Сколько читается из `/metrics` для определения типа: признаки — в первых сериях.
pub const METRICS_PROBE_LIMIT: usize = 256 * 1024;

async fn optional(c: &ServerClient, path: &str) -> Option<Answer> {
    let r = c.get(path, JSON_LIMIT).await.ok()?;
    Some(Answer {
        status: r.status,
        body: r.body,
    })
}

/// Ответ первого пути, который есть у сервера: новые имена SGLang, затем устаревшие.
async fn first_found(c: &ServerClient, paths: [&str; 2]) -> Option<Answer> {
    match optional(c, paths[0]).await {
        Some(a) if a.status == 404 => optional(c, paths[1]).await.or(Some(a)),
        other => other,
    }
}

/// Снимок ответов сервера. Ошибка — только если `/v1/models` не ответил 2xx.
pub async fn probe(c: &ServerClient, metrics_path: &str) -> AppResult<Probe> {
    let (models, version, model_info, server_info, metrics) = tokio::join!(
        c.get_ok("/v1/models", JSON_LIMIT),
        optional(c, "/version"),
        first_found(c, ["/model_info", "/get_model_info"]),
        first_found(c, ["/server_info", "/get_server_info"]),
        c.get(metrics_path, METRICS_PROBE_LIMIT),
    );
    let metrics = metrics
        .ok()
        .filter(|r| r.ok())
        .map(|r| String::from_utf8_lossy(&r.body).into_owned());
    Ok(Probe {
        version,
        model_info,
        server_info,
        metrics,
        models: models?.body,
    })
}

/// Тип, версия и модели сервера.
pub async fn facts(c: &ServerClient, s: &ServerConfig) -> AppResult<ServerFacts> {
    analyze(s.kind, &probe(c, &s.metrics_path).await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use crate::policy::PolicyStatus;
    use crate::servers::client::Timeouts;
    use crate::servers::detect::DetectedKind;
    use crate::servers::testing::{Reply, TestServer};
    use crate::storage::profile::{ProxyConfig, ServerKind, sample_server};

    fn config(url: &str) -> ServerConfig {
        let mut s = sample_server("s1");
        s.base_url = url.into();
        s.proxy = ProxyConfig::None;
        s
    }

    fn client(s: &ServerConfig) -> ServerClient {
        let t = Timeouts {
            connect_ms: 2_000,
            request_ms: 2_000,
        };
        ServerClient::new(s, t, None, &PolicyStatus::default()).unwrap()
    }

    fn sglang_0_4_routes() -> Vec<(&'static str, Reply)> {
        vec![
            (
                "/v1/models",
                Reply::json(include_str!("testdata/sglang-0.4.6/models.json")),
            ),
            (
                "/get_model_info",
                Reply::json(include_str!("testdata/sglang-0.4.6/model_info.json")),
            ),
            (
                "/get_server_info",
                Reply::json(include_str!("testdata/sglang-0.4.6/server_info.json")),
            ),
        ]
    }

    #[tokio::test]
    async fn vllm_server_is_recognized_over_http() {
        let srv = TestServer::start(vec![
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
        ])
        .await;
        let s = config(&srv.url());
        let f = facts(&client(&s), &s).await.unwrap();
        assert_eq!(
            (f.kind, f.version.as_deref(), f.metrics),
            (DetectedKind::Vllm, Some("0.11.0"), true)
        );
        assert_eq!(f.models[0].id, "qwen3-32b");
        let mut paths = srv.paths();
        paths.sort();
        assert_eq!(
            paths,
            [
                "/get_model_info",
                "/get_server_info",
                "/metrics",
                "/model_info",
                "/server_info",
                "/v1/models",
                "/version"
            ]
        );
    }

    #[tokio::test]
    async fn old_sglang_paths_are_used_when_new_ones_are_missing() {
        let srv = TestServer::start(sglang_0_4_routes()).await;
        let s = config(&srv.url());
        let f = facts(&client(&s), &s).await.unwrap();
        // Версию даёт `/get_server_info`; SGLang 0.4 ниже границы и работает как openai.
        assert_eq!(
            (f.detected, f.version.as_deref()),
            (DetectedKind::Sglang, Some("0.4.6.post1"))
        );
        assert_eq!(f.kind, DetectedKind::Openai);
    }

    #[tokio::test]
    async fn metrics_path_comes_from_the_profile() {
        let mut routes = sglang_0_4_routes();
        routes.push((
            "/stats",
            Reply::text(include_str!("testdata/sglang-0.5.4/metrics.txt")),
        ));
        let srv = TestServer::start(routes).await;
        let mut s = config(&srv.url());
        s.metrics_path = "/stats".into();
        facts(&client(&s), &s).await.unwrap();
        assert!(srv.paths().contains(&"/stats".to_string()));
        assert!(!srv.paths().contains(&"/metrics".to_string()));
    }

    #[tokio::test]
    async fn models_are_required() {
        let srv = TestServer::start(vec![("/v1/models", Reply::status(401))]).await;
        let s = config(&srv.url());
        let e = facts(&client(&s), &s).await.unwrap_err();
        assert_eq!(e.code, ErrorCode::AuthFailed);
        let srv = TestServer::start(vec![]).await;
        let s = config(&srv.url());
        let e = facts(&client(&s), &s).await.unwrap_err();
        assert_eq!(e.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn configured_kind_is_respected() {
        let srv = TestServer::start(sglang_0_4_routes()).await;
        let mut s = config(&srv.url());
        s.kind = ServerKind::Openai;
        let f = facts(&client(&s), &s).await.unwrap();
        assert_eq!(
            (f.kind, f.detected),
            (DetectedKind::Openai, DetectedKind::Sglang)
        );
        assert_eq!(f.warnings[0].key, "messages.servers.kindMismatch");
    }
}
