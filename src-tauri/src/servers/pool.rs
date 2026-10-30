//! Подключённые инсталляции (спец. §4.1): клиенты серверов и сведения о них. Подключение
//! инсталляции — это подключение каждого её сервера; сервер с ошибкой не мешает остальным.
//! Подключения одной инсталляции идут по очереди: кто ждал, получает готовый результат.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::metrics::derive::Timed;
use crate::policy::PolicyStatus;
use crate::secrets::{API_KEY, SecretStore, secret_key};
use crate::servers::client::{ServerClient, Timeouts};
use crate::servers::detect::ServerFacts;
use crate::storage::profile::{InstallationProfile, ServerConfig};
use crate::sync::MutexExt;

/// Итог подключения сервера: сведения или ошибка.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerState {
    pub server_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facts: Option<ServerFacts>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AppError>,
}

/// Подключённая инсталляция для фронтенда: серверы в порядке профиля.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallationInfo {
    pub installation_id: String,
    pub servers: Vec<ServerState>,
}

/// Подключённая инсталляция: сведения и клиенты серверов, которые подключились.
pub struct Connected {
    pub info: InstallationInfo,
    pub clients: HashMap<String, Arc<ServerClient>>,
}

impl Connected {
    pub fn client(&self, server_id: &str) -> Option<Arc<ServerClient>> {
        self.clients.get(server_id).cloned()
    }
}

/// Снимки метрик одного сервера за последнюю минуту.
pub type LiveHistory = Arc<tokio::sync::Mutex<VecDeque<Timed>>>;

#[derive(Default)]
pub struct ServerPool {
    connected: Mutex<HashMap<String, Arc<Connected>>>,
    /// Поколения инсталляций: растут при каждом `evict`. Порядок блокировок всегда
    /// `generations`, затем `connected`, чтобы сверка поколения и вставка шли одним шагом.
    generations: Mutex<HashMap<String, u64>>,
    locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    /// Снимки метрик серверов за последнюю минуту (спец. I1b §4), по (инсталляция, сервер).
    live: Mutex<HashMap<(String, String), LiveHistory>>,
}

impl ServerPool {
    pub fn get(&self, id: &str) -> Option<Arc<Connected>> {
        self.connected.lock_safe().get(id).cloned()
    }

    pub fn insert(&self, id: &str, c: Arc<Connected>) {
        self.connected.lock_safe().insert(id.to_string(), c);
    }

    /// Забывает инсталляцию: следующее чтение подключится заново. Подключение, начатое
    /// раньше, в пул уже не попадёт (`insert_if_current`).
    pub fn evict(&self, id: &str) {
        let mut generations = self.generations.lock_safe();
        *generations.entry(id.to_string()).or_default() += 1;
        self.connected.lock_safe().remove(id);
        self.live
            .lock_safe()
            .retain(|(installation, _), _| installation != id);
    }

    /// История снимков метрик сервера; `evict` инсталляции её сбрасывает.
    pub fn live(&self, installation_id: &str, server_id: &str) -> LiveHistory {
        self.live
            .lock_safe()
            .entry((installation_id.to_string(), server_id.to_string()))
            .or_default()
            .clone()
    }

    /// Текущее поколение инсталляции: запоминается до чтения профиля и ключей.
    pub fn generation(&self, id: &str) -> u64 {
        self.generations.lock_safe().get(id).copied().unwrap_or(0)
    }

    /// Кладёт подключение, только если с момента `generation` инсталляцию не сбрасывали:
    /// иначе оно собрано по устаревшим профилю и ключам. Возвращает, положено ли.
    pub fn insert_if_current(&self, id: &str, generation: u64, c: Arc<Connected>) -> bool {
        let generations = self.generations.lock_safe();
        if generations.get(id).copied().unwrap_or(0) != generation {
            return false;
        }
        self.connected.lock_safe().insert(id.to_string(), c);
        true
    }

    /// Очередь подключений инсталляции `id`.
    pub async fn connect_lock(&self, id: &str) -> tokio::sync::OwnedMutexGuard<()> {
        let lock = self
            .locks
            .lock_safe()
            .entry(id.to_string())
            .or_default()
            .clone();
        lock.lock_owned().await
    }
}

/// Ключи API серверов из keychain: блокирующий вызов, выполнять вне async-потоков. Сбой
/// keychain — ошибка только этого сервера.
pub fn read_api_keys(
    secrets: &dyn SecretStore,
    p: &InstallationProfile,
) -> HashMap<String, AppResult<Option<String>>> {
    p.servers
        .iter()
        .map(|s| {
            (
                s.id.clone(),
                secrets.get(&secret_key(&p.id, &s.id, API_KEY)),
            )
        })
        .collect()
}

async fn open_server(
    s: &ServerConfig,
    timeouts: Timeouts,
    key: AppResult<Option<String>>,
    policy: &PolicyStatus,
) -> AppResult<(ServerClient, ServerFacts)> {
    let client = ServerClient::new(s, timeouts, key?, policy)?;
    let facts = crate::servers::probe::facts(&client, s).await?;
    Ok((client, facts))
}

/// Подключает все серверы инсталляции параллельно. Ошибка сервера остаётся в его состоянии.
pub async fn open(
    p: &InstallationProfile,
    mut keys: HashMap<String, AppResult<Option<String>>>,
    policy: &PolicyStatus,
) -> Connected {
    let timeouts = Timeouts::from(p);
    let results = futures::future::join_all(p.servers.iter().map(|s| {
        let key = keys.remove(&s.id).unwrap_or(Ok(None));
        async move { (s.id.clone(), open_server(s, timeouts, key, policy).await) }
    }))
    .await;
    let mut clients = HashMap::new();
    let servers = results
        .into_iter()
        .map(|(server_id, r)| match r {
            Ok((client, facts)) => {
                clients.insert(server_id.clone(), Arc::new(client));
                ServerState {
                    server_id,
                    facts: Some(facts),
                    error: None,
                }
            }
            Err(e) => ServerState {
                server_id,
                facts: None,
                error: Some(e),
            },
        })
        .collect();
    Connected {
        info: InstallationInfo {
            installation_id: p.id.clone(),
            servers,
        },
        clients,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use crate::secrets::MemoryStore;
    use crate::servers::detect::DetectedKind;
    use crate::servers::testing::{Reply, TestServer};
    use crate::storage::profile::{CaConfig, ProxyConfig, sample_profile, sample_server};

    pub(crate) fn vllm_routes() -> Vec<(&'static str, Reply)> {
        vec![
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

    fn server(id: &str, url: &str) -> ServerConfig {
        let mut s = sample_server(id);
        s.base_url = url.into();
        s.proxy = ProxyConfig::None;
        s
    }

    #[tokio::test]
    async fn every_server_gets_facts_or_its_own_error() {
        let ok = TestServer::start(vllm_routes()).await;
        let denied = TestServer::start(vec![("/v1/models", Reply::status(401))]).await;
        let mut p = sample_profile("a");
        p.servers = vec![server("s1", &ok.url()), server("s2", &denied.url()), {
            let mut s = server("s3", "https://vllm:8000");
            s.ca = CaConfig::Insecure;
            s
        }];
        let secrets = MemoryStore::default();
        secrets
            .set(&secret_key("a", "s1", API_KEY), "sk-one")
            .unwrap();
        let policy = PolicyStatus {
            allow_insecure_tls: false,
            ..PolicyStatus::default()
        };
        let c = open(&p, read_api_keys(&secrets, &p), &policy).await;
        let s = &c.info.servers;
        assert_eq!(
            s.iter().map(|x| x.server_id.as_str()).collect::<Vec<_>>(),
            ["s1", "s2", "s3"]
        );
        assert_eq!(s[0].facts.as_ref().unwrap().kind, DetectedKind::Vllm);
        assert_eq!(s[1].error.as_ref().unwrap().code, ErrorCode::AuthFailed);
        assert_eq!(
            s[2].error.as_ref().unwrap().key,
            "errors.installations.insecureForbidden"
        );
        assert!(c.client("s1").is_some() && c.client("s2").is_none());
        assert_eq!(
            ok.seen.lock().unwrap()[0].authorization.as_deref(),
            Some("Bearer sk-one")
        );
    }

    #[test]
    fn keychain_failure_is_an_error_of_that_server_only() {
        struct Broken;
        impl SecretStore for Broken {
            fn get(&self, key: &str) -> AppResult<Option<String>> {
                if key.ends_with("/s2/apiKey") {
                    Err(AppError::internal_with("locked"))
                } else {
                    Ok(None)
                }
            }
            fn set(&self, _: &str, _: &str) -> AppResult<()> {
                Ok(())
            }
            fn delete(&self, _: &str) -> AppResult<()> {
                Ok(())
            }
        }
        let mut p = sample_profile("a");
        p.servers.push(sample_server("s2"));
        let keys = read_api_keys(&Broken, &p);
        assert!(matches!(keys["s1"], Ok(None)));
        assert!(keys["s2"].is_err());
    }

    fn connected(id: &str) -> Arc<Connected> {
        Arc::new(Connected {
            info: InstallationInfo {
                installation_id: id.into(),
                servers: Vec::new(),
            },
            clients: HashMap::new(),
        })
    }

    #[test]
    fn insert_get_evict() {
        let pool = ServerPool::default();
        assert!(pool.get("a").is_none());
        pool.insert("a", connected("a"));
        assert_eq!(pool.get("a").unwrap().info.installation_id, "a");
        pool.evict("a");
        assert!(pool.get("a").is_none());

        // Подключение, начатое до evict, в пул не попадает; чужой evict ему не мешает.
        let started = pool.generation("a");
        pool.evict("b");
        assert!(pool.insert_if_current("a", started, connected("a")));
        pool.evict("a");
        assert!(!pool.insert_if_current("a", started, connected("a")));
        assert!(pool.get("a").is_none());
        assert!(pool.insert_if_current("a", pool.generation("a"), connected("a")));
        assert!(pool.get("a").is_some());
    }

    #[tokio::test]
    async fn connect_lock_serializes_one_installation_only() {
        let pool = Arc::new(ServerPool::default());
        let a = pool.connect_lock("a").await;
        let other = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            pool.connect_lock("b"),
        )
        .await;
        assert!(other.is_ok(), "другая инсталляция не ждёт");
        let same = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            pool.connect_lock("a"),
        )
        .await;
        assert!(same.is_err(), "та же инсталляция ждёт");
        drop(a);
        assert!(
            tokio::time::timeout(std::time::Duration::from_secs(1), pool.connect_lock("a"))
                .await
                .is_ok()
        );
    }

    #[test]
    fn info_serializes_facts_or_error() {
        let info = InstallationInfo {
            installation_id: "a".into(),
            servers: vec![ServerState {
                server_id: "s1".into(),
                facts: None,
                error: Some(crate::servers::errors::timeout()),
            }],
        };
        let v = serde_json::to_value(info).unwrap();
        assert_eq!(v["installationId"], "a");
        assert_eq!(v["servers"][0]["serverId"], "s1");
        assert_eq!(v["servers"][0]["error"]["code"], "TIMEOUT");
        assert!(v["servers"][0].get("facts").is_none());
    }
}
