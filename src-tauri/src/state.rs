use std::path::Path;
use std::sync::Arc;

use crate::error::AppResult;
use crate::secrets::SecretStore;
use crate::servers::pool::{Connected, ServerPool, open, read_api_keys};
use crate::storage::installations::InstallationStore;

/// Общее состояние приложения: хранилища, политика и подключённые инсталляции.
/// Клонируется дёшево (всё за `Arc`).
#[derive(Clone)]
pub struct AppState {
    pub installations: Arc<InstallationStore>,
    pub secrets: Arc<dyn SecretStore>,
    pub policy: Arc<crate::policy::Policy>,
    pub settings: Arc<crate::settings::SettingsStore>,
    pub pool: Arc<ServerPool>,
}

impl AppState {
    pub fn new(data_dir: &Path, secrets: Arc<dyn SecretStore>) -> AppResult<Self> {
        Ok(Self {
            installations: Arc::new(InstallationStore::open(data_dir)?),
            secrets,
            policy: Arc::new(crate::policy::Policy::default()),
            settings: Arc::new(crate::settings::SettingsStore::open(data_dir)),
            pool: Arc::new(ServerPool::default()),
        })
    }

    /// Политика администратора (читается при запуске в lib.rs).
    pub fn with_policy(mut self, policy: crate::policy::Policy) -> Self {
        self.policy = Arc::new(policy);
        self
    }

    /// Подключённая инсталляция: из пула или новое подключение (`fresh` — всегда новое).
    /// Подключения одной инсталляции идут по очереди: кто ждал, получает готовый результат.
    /// Ключи API читаются из keychain вне async-потоков.
    pub async fn connected(&self, id: &str, fresh: bool) -> AppResult<Arc<Connected>> {
        let _turn = self.pool.connect_lock(id).await;
        if fresh {
            self.pool.evict(id);
        } else if let Some(c) = self.pool.get(id) {
            return Ok(c);
        }
        // Поколение до чтения профиля: сброс во время подключения делает результат устаревшим.
        let generation = self.pool.generation(id);
        let profile = self.installations.get(id)?;
        let keys = {
            let (secrets, p) = (self.secrets.clone(), profile.clone());
            crate::commands::blocking(move || Ok(read_api_keys(secrets.as_ref(), &p))).await?
        };
        let c = Arc::new(open(&profile, keys, &self.policy.status).await);
        // Устаревшее подключение вызвавший получает, но в пул оно не попадает.
        self.pool.insert_if_current(id, generation, c.clone());
        Ok(c)
    }

    /// Забывает подключение инсталляции: профиль или ключи изменились, удалена.
    pub fn evict_installation(&self, id: &str) {
        self.pool.evict(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use crate::secrets::{API_KEY, MemoryStore, secret_key};
    use crate::servers::testing::{Reply, TestServer};
    use crate::storage::profile::{ProxyConfig, sample_profile};

    async fn state_with(url: &str) -> (tempfile::TempDir, AppState, Arc<MemoryStore>) {
        let dir = tempfile::tempdir().unwrap();
        let secrets = Arc::new(MemoryStore::default());
        let st = AppState::new(dir.path(), secrets.clone()).unwrap();
        let mut p = sample_profile("a");
        p.servers[0].base_url = url.into();
        p.servers[0].proxy = ProxyConfig::None;
        st.installations.upsert(p).unwrap();
        (dir, st, secrets)
    }

    #[tokio::test]
    async fn connection_is_reused_until_fresh_or_evicted() {
        let srv = TestServer::start(vec![(
            "/v1/models",
            Reply::json(r#"{"data":[{"id":"m","owned_by":"llamacpp"}]}"#),
        )])
        .await;
        let (_d, st, secrets) = state_with(&srv.url()).await;
        secrets
            .set(&secret_key("a", "s1", API_KEY), "sk-a")
            .unwrap();
        let c = st.connected("a", false).await.unwrap();
        assert_eq!(c.info.servers[0].facts.as_ref().unwrap().models[0].id, "m");
        let models = |srv: &TestServer| srv.paths().iter().filter(|p| *p == "/v1/models").count();
        assert_eq!(models(&srv), 1);
        st.connected("a", false).await.unwrap();
        assert_eq!(models(&srv), 1, "из пула");
        st.connected("a", true).await.unwrap();
        assert_eq!(models(&srv), 2, "fresh — заново");
        st.evict_installation("a");
        st.connected("a", false).await.unwrap();
        assert_eq!(models(&srv), 3, "после сброса — заново");
        assert_eq!(
            srv.seen.lock().unwrap()[0].authorization.as_deref(),
            Some("Bearer sk-a")
        );
    }

    #[tokio::test]
    async fn unknown_installation_is_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let st = AppState::new(dir.path(), Arc::new(MemoryStore::default())).unwrap();
        let e = st.connected("nope", false).await.err().unwrap();
        assert_eq!(e.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn concurrent_connects_probe_once() {
        let srv = TestServer::start(vec![(
            "/v1/models",
            Reply::json(r#"{"data":[]}"#).delayed(std::time::Duration::from_millis(200)),
        )])
        .await;
        let (_d, st, _) = state_with(&srv.url()).await;
        let (a, b) = tokio::join!(st.connected("a", false), st.connected("a", false));
        assert!(Arc::ptr_eq(&a.unwrap(), &b.unwrap()));
        assert_eq!(srv.paths().iter().filter(|p| *p == "/v1/models").count(), 1);
    }

    #[tokio::test]
    async fn connect_started_before_evict_is_not_cached() {
        let srv = TestServer::start(vec![(
            "/v1/models",
            Reply::json(r#"{"data":[]}"#).delayed(std::time::Duration::from_millis(300)),
        )])
        .await;
        let (_d, st, _) = state_with(&srv.url()).await;
        let (connected, ()) = tokio::join!(st.connected("a", false), async {
            // Ждём, пока подключение уйдёт в сеть, и сбрасываем его: профиль «изменился».
            while !srv.paths().contains(&"/v1/models".to_string()) {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            st.evict_installation("a");
        });
        assert!(connected.is_ok(), "вызвавший всё равно получает результат");
        assert!(
            st.pool.get("a").is_none(),
            "устаревшее подключение в пул не попадает"
        );
        st.connected("a", false).await.unwrap();
        assert!(
            st.pool.get("a").is_some(),
            "следующее подключение кешируется"
        );
    }
}
