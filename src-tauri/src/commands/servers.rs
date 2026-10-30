//! Команды сервера (спец. I1b §4–5): метрики «сейчас» и аргументы запуска.

use std::sync::Arc;

use tauri::State;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::metrics::derive::{ServerMetrics, Timed};
use crate::servers::client::ServerClient;
use crate::servers::config::{self, ConfigEntry};
use crate::servers::detect::DetectedKind;
use crate::servers::live::{fetch, now_ms, record};
use crate::state::AppState;
use crate::sync::MutexExt;

/// Метрики сервера с производными по предыдущим снимкам. Сервер, который не подключился,
/// возвращает свою ошибку подключения.
#[tauri::command]
pub async fn read_server_metrics(
    state: State<'_, AppState>,
    installation_id: String,
    server_id: String,
) -> AppResult<ServerMetrics> {
    read_server_metrics_inner(&state, &installation_id, &server_id).await
}

/// Аргументы запуска сервера с источниками (спец. I1b §5).
#[tauri::command]
pub async fn read_server_config(
    state: State<'_, AppState>,
    installation_id: String,
    server_id: String,
) -> AppResult<Vec<ConfigEntry>> {
    let (client, kind) = connected_server(&state, &installation_id, &server_id).await?;
    config::fetch(&client, kind).await
}

/// Клиент и тип подключённого сервера; не подключился — его ошибка подключения.
async fn connected_server(
    state: &AppState,
    installation_id: &str,
    server_id: &str,
) -> AppResult<(Arc<ServerClient>, DetectedKind)> {
    let c = state.connected(installation_id, false).await?;
    let server = c
        .info
        .servers
        .iter()
        .find(|s| s.server_id == server_id)
        .ok_or_else(|| {
            app_err!(
                ErrorCode::NotFound,
                "errors.installations.serverNotFound",
                "Server {{id}} is not in the installation",
                id = server_id
            )
        })?;
    if let Some(e) = &server.error {
        return Err(e.clone());
    }
    match (&server.facts, c.client(server_id)) {
        (Some(facts), Some(client)) => Ok((client, facts.kind)),
        _ => Err(AppError::internal_with(format!(
            "server {server_id} has neither facts nor an error"
        ))),
    }
}

pub async fn read_server_metrics_inner(
    state: &AppState,
    installation_id: &str,
    server_id: &str,
) -> AppResult<ServerMetrics> {
    let (client, kind) = connected_server(state, installation_id, server_id).await?;
    let snap = fetch(&client, kind).await?;
    let live = state.pool.live(installation_id, server_id);
    let mut history = live.lock_safe();
    Ok(record(
        &mut history,
        Timed {
            at_ms: now_ms(),
            snap,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::MemoryStore;
    use crate::servers::testing::{Reply, TestServer};
    use crate::storage::profile::{ProxyConfig, sample_profile, sample_server};

    const AFTER: &str = include_str!("../servers/testdata/vllm-0.32.0/metrics-after.txt");

    fn vllm_routes() -> Vec<(&'static str, Reply)> {
        vec![
            (
                "/v1/models",
                Reply::json(include_str!("../servers/testdata/vllm-0.32.0/models.json")),
            ),
            (
                "/version",
                Reply::json(include_str!("../servers/testdata/vllm-0.32.0/version.json")),
            ),
            ("/metrics", Reply::text(AFTER)),
        ]
    }

    fn state_with(urls: &[&str]) -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let st = AppState::new(dir.path(), Arc::new(MemoryStore::default())).unwrap();
        let mut p = sample_profile("a");
        p.servers.clear();
        for (i, url) in urls.iter().enumerate() {
            let mut s = sample_server(&format!("s{}", i + 1));
            s.base_url = (*url).into();
            s.proxy = ProxyConfig::None;
            p.servers.push(s);
        }
        st.installations.upsert(p).unwrap();
        (dir, st)
    }

    #[tokio::test]
    async fn first_read_has_gauges_then_rates_follow() {
        let srv = TestServer::start(vllm_routes()).await;
        let (_d, st) = state_with(&[&srv.url()]);
        let m = read_server_metrics_inner(&st, "a", "s1").await.unwrap();
        assert_eq!(m.interval_sec, None);
        assert_eq!(m.gauges.requests_running, Some(0.0));
        assert_eq!(m.engines, 1);
        let m = read_server_metrics_inner(&st, "a", "s1").await.unwrap();
        assert!(m.interval_sec.is_some());
        assert_eq!(m.rates.unwrap().prompt_tokens, Some(0.0));
        st.evict_installation("a");
        let m = read_server_metrics_inner(&st, "a", "s1").await.unwrap();
        assert_eq!(
            m.interval_sec, None,
            "переподключение начинает историю заново"
        );
    }

    #[tokio::test]
    async fn server_that_did_not_connect_returns_its_error() {
        let srv = TestServer::start(vllm_routes()).await;
        let dead = TestServer::start(vec![("/v1/models", Reply::status(401))]).await;
        let (_d, st) = state_with(&[&srv.url(), &dead.url()]);
        let e = read_server_metrics_inner(&st, "a", "s2").await.unwrap_err();
        assert_eq!(e.code, ErrorCode::AuthFailed);
        let e = read_server_metrics_inner(&st, "a", "nope")
            .await
            .unwrap_err();
        assert_eq!(e.key, "errors.installations.serverNotFound");
        assert!(read_server_metrics_inner(&st, "a", "s1").await.is_ok());
    }

    #[tokio::test]
    async fn openai_compatible_server_has_no_canonical_metrics() {
        let srv = TestServer::start(vec![
            (
                "/v1/models",
                Reply::json(r#"{"data":[{"id":"m","owned_by":"llamacpp"}]}"#),
            ),
            ("/metrics", Reply::text("llamacpp:prompt_tokens_total 5\n")),
        ])
        .await;
        let (_d, st) = state_with(&[&srv.url()]);
        let m = read_server_metrics_inner(&st, "a", "s1").await.unwrap();
        assert_eq!(m.gauges.requests_running, None);
        assert!(m.missing.is_empty() && m.series.is_empty());
    }

    #[tokio::test]
    async fn config_of_a_connected_server() {
        let srv = TestServer::start(vllm_routes()).await;
        let (_d, st) = state_with(&[&srv.url()]);
        let (client, kind) = connected_server(&st, "a", "s1").await.unwrap();
        let e = config::fetch(&client, kind).await.unwrap();
        assert!(e.iter().any(|x| x.name == "block_size" && x.value == "16"));
    }
}
