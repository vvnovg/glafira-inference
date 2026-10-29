use std::collections::BTreeMap;

use serde::Deserialize;
use tauri::State;

use super::blocking;
use crate::error::AppResult;
use crate::secrets::{API_KEY, secret_key};
use crate::servers::client::{Timeouts, ensure_allowed};
use crate::servers::connectivity::{TestReport, test_server};
use crate::servers::pool::InstallationInfo;
use crate::state::AppState;
use crate::storage::profile::InstallationProfile;

/// Ключи API из формы: id сервера → значение. Сервера нет в списке — оставить сохранённый,
/// `""` — удалить, иное — сохранить. `Debug` печатает вместо значений `***`.
#[derive(Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretsInput {
    #[serde(default)]
    pub api_keys: BTreeMap<String, String>,
}

/// Ключи не печатаются: структура может попасть в лог или сообщение об ошибке.
impl std::fmt::Debug for SecretsInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(self.api_keys.keys().map(|k| (k, "***")))
            .finish()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallationInput {
    pub profile: InstallationProfile,
    #[serde(default)]
    pub secrets: SecretsInput,
}

/// Что сделать с ключом при сохранении.
#[derive(Debug, PartialEq, Eq)]
enum SecretOp {
    Keep,
    Set(String),
    Delete,
}

/// Ключи серверов профиля — по данным формы; ключи серверов, которых больше нет, удаляются.
fn secret_ops(
    saved: &InstallationProfile,
    previous: Option<&InstallationProfile>,
    s: &SecretsInput,
) -> Vec<(String, SecretOp)> {
    let mut ops: Vec<(String, SecretOp)> = saved
        .servers
        .iter()
        .map(|server| {
            let op = match s.api_keys.get(&server.id).map(|v| v.trim()) {
                None => SecretOp::Keep,
                Some("") => SecretOp::Delete,
                Some(v) => SecretOp::Set(v.to_string()),
            };
            (server.id.clone(), op)
        })
        .collect();
    for old in previous.map(|p| p.servers.as_slice()).unwrap_or_default() {
        if saved.server(&old.id).is_none() {
            ops.push((old.id.clone(), SecretOp::Delete));
        }
    }
    ops
}

#[tauri::command]
pub fn list_installations(state: State<'_, AppState>) -> AppResult<Vec<InstallationProfile>> {
    Ok(state.installations.list())
}

#[tauri::command]
pub async fn save_installation(
    state: State<'_, AppState>,
    input: InstallationInput,
) -> AppResult<InstallationProfile> {
    let st = state.inner().clone();
    blocking(move || save_installation_blocking(&st, input)).await
}

/// Сохраняет профиль и ключи согласованно: если keychain не принял изменение, профиль
/// возвращается к прежнему состоянию (или удаляется, если он новый).
pub fn save_installation_blocking(
    st: &AppState,
    input: InstallationInput,
) -> AppResult<InstallationProfile> {
    let InstallationInput {
        mut profile,
        secrets,
    } = input;
    for s in &profile.servers {
        ensure_allowed(s, &st.policy.status)?;
    }
    if profile.id.is_empty() {
        profile.id = uuid::Uuid::new_v4().to_string();
    }
    let previous = st.installations.get(&profile.id).ok();
    // upsert проверяет профиль — ключи трогаем только после успешного сохранения.
    let saved = st.installations.upsert(profile)?;
    // Сначала `Set`, потом `Delete`: если keychain не примет новый ключ, удалённые ещё на
    // месте, и откат профиля не оставляет серверы без ключей.
    let mut ops = secret_ops(&saved, previous.as_ref(), &secrets);
    ops.sort_by_key(|(_, op)| *op == SecretOp::Delete);
    let result = ops.into_iter().try_for_each(|(server, op)| {
        let key = secret_key(&saved.id, &server, API_KEY);
        match op {
            SecretOp::Keep => Ok(()),
            SecretOp::Set(v) => st.secrets.set(&key, &v),
            SecretOp::Delete => st.secrets.delete(&key),
        }
    });
    if result.is_err() {
        let rollback = match previous {
            Some(p) => st.installations.upsert(p).map(|_| ()),
            None => st.installations.delete(&saved.id),
        };
        if let Err(r) = rollback {
            tracing::error!("failed to roll back profile {}: {r}", saved.id);
        }
    }
    // Клиенты работают со старыми адресами и ключами: следующий вызов подключится заново.
    st.evict_installation(&saved.id);
    result.map(|()| saved)
}

#[tauri::command]
pub async fn delete_installation(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let st = state.inner().clone();
    blocking(move || st.delete_installation_full(&id)).await
}

/// Проверяет сервер и несохранённого профиля: ключ из формы накладывается поверх keychain
/// (с той же семантикой, что и при сохранении).
#[tauri::command]
pub async fn test_server_connection(
    state: State<'_, AppState>,
    input: InstallationInput,
    server_id: String,
) -> AppResult<TestReport> {
    let st = state.inner().clone();
    let profile = input.profile.clone();
    let server = profile.server(&server_id).cloned().ok_or_else(|| {
        app_err!(
            crate::error::ErrorCode::NotFound,
            "errors.installations.serverNotFound",
            "Server {{id}} is not in the installation",
            id = server_id
        )
    })?;
    let typed = input.secrets.api_keys.get(&server_id).cloned();
    let key = match typed {
        Some(v) => Some(v),
        None => {
            let key = secret_key(&profile.id, &server_id, API_KEY);
            blocking(move || st.secrets.get(&key)).await?
        }
    };
    Ok(test_server(&server, Timeouts::from(&profile), key, &state.policy.status).await)
}

/// Подключается (или берёт подключение из пула) и возвращает сведения о серверах.
/// `fresh` — подключиться заново («Обновить», после смены ключа).
#[tauri::command]
pub async fn connect_installation(
    state: State<'_, AppState>,
    installation_id: String,
    fresh: bool,
) -> AppResult<InstallationInfo> {
    let c = state.connected(&installation_id, fresh).await?;
    Ok(c.info.clone())
}

#[tauri::command]
pub fn disconnect_installation(
    state: State<'_, AppState>,
    installation_id: String,
) -> AppResult<()> {
    state.evict_installation(&installation_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::error::{AppError, ErrorCode};
    use crate::secrets::{MemoryStore, SecretStore};
    use crate::storage::profile::{CaConfig, sample_profile, sample_server};

    fn state(dir: &std::path::Path) -> AppState {
        AppState::new(dir, Arc::new(MemoryStore::default())).unwrap()
    }

    fn input(profile: InstallationProfile, keys: &[(&str, &str)]) -> InstallationInput {
        InstallationInput {
            profile,
            secrets: SecretsInput {
                api_keys: keys
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            },
        }
    }

    fn key(st: &AppState, id: &str, server: &str) -> Option<String> {
        st.secrets.get(&secret_key(id, server, API_KEY)).unwrap()
    }

    #[test]
    fn secret_ops_matrix() {
        let mut prev = sample_profile("a");
        prev.servers.push(sample_server("gone"));
        let mut saved = sample_profile("a");
        saved.servers.push(sample_server("s2"));
        saved.servers.push(sample_server("s3"));
        let s = SecretsInput {
            api_keys: [("s2", " sk-2 "), ("s3", " ")]
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        };
        assert_eq!(
            secret_ops(&saved, Some(&prev), &s),
            [
                ("s1".to_string(), SecretOp::Keep),
                ("s2".to_string(), SecretOp::Set("sk-2".into())),
                ("s3".to_string(), SecretOp::Delete),
                ("gone".to_string(), SecretOp::Delete),
            ]
        );
        assert_eq!(format!("{s:?}"), r#"{"s2": "***", "s3": "***"}"#);
    }

    #[test]
    fn new_installation_gets_an_id_and_keys() {
        let dir = tempfile::tempdir().unwrap();
        let st = state(dir.path());
        let mut p = sample_profile("");
        p.id.clear();
        let saved = save_installation_blocking(&st, input(p, &[("s1", "sk-1")])).unwrap();
        assert!(!saved.id.is_empty());
        assert_eq!(key(&st, &saved.id, "s1").as_deref(), Some("sk-1"));
        let raw = std::fs::read_to_string(dir.path().join("installations.json")).unwrap();
        assert!(!raw.contains("sk-1"));
    }

    #[test]
    fn keys_are_kept_replaced_and_removed_with_their_servers() {
        let dir = tempfile::tempdir().unwrap();
        let st = state(dir.path());
        let mut two = sample_profile("a");
        two.servers.push(sample_server("s2"));
        save_installation_blocking(&st, input(two, &[("s1", "sk-1"), ("s2", "sk-2")])).unwrap();
        // Без ключей в форме — сохранённые остаются; s2 удалён из профиля — его ключ тоже.
        save_installation_blocking(&st, input(sample_profile("a"), &[])).unwrap();
        assert_eq!(key(&st, "a", "s1").as_deref(), Some("sk-1"));
        assert_eq!(key(&st, "a", "s2"), None);
        save_installation_blocking(&st, input(sample_profile("a"), &[("s1", "")])).unwrap();
        assert_eq!(key(&st, "a", "s1"), None);
    }

    /// Keychain, который отказывает в записи.
    struct ReadOnlyKeychain(MemoryStore);

    impl SecretStore for ReadOnlyKeychain {
        fn get(&self, key: &str) -> AppResult<Option<String>> {
            self.0.get(key)
        }
        fn set(&self, _: &str, _: &str) -> AppResult<()> {
            Err(AppError::internal_with("denied"))
        }
        fn delete(&self, _: &str) -> AppResult<()> {
            Err(AppError::internal_with("denied"))
        }
    }

    #[test]
    fn keychain_failure_rolls_the_profile_back() {
        let dir = tempfile::tempdir().unwrap();
        let st = AppState::new(
            dir.path(),
            Arc::new(ReadOnlyKeychain(MemoryStore::default())),
        )
        .unwrap();
        let e = save_installation_blocking(&st, input(sample_profile("a"), &[("s1", "sk")]));
        assert!(e.is_err());
        assert!(st.installations.get("a").is_err(), "новый профиль удалён");
        st.installations.upsert(sample_profile("b")).unwrap();
        let mut renamed = sample_profile("b");
        renamed.name = "renamed".into();
        assert!(save_installation_blocking(&st, input(renamed, &[("s1", "sk")])).is_err());
        assert_eq!(st.installations.get("b").unwrap().name, "inst-b");
    }

    #[test]
    fn policy_forbids_saving_insecure_servers() {
        let dir = tempfile::tempdir().unwrap();
        let mut policy = crate::policy::Policy::default();
        policy.status.allow_insecure_tls = false;
        let st = state(dir.path()).with_policy(policy);
        let mut p = sample_profile("a");
        p.servers[0].ca = CaConfig::Insecure;
        let e = save_installation_blocking(&st, input(p, &[])).unwrap_err();
        assert_eq!(
            (e.code, e.key.as_str()),
            (
                ErrorCode::InvalidInput,
                "errors.installations.insecureForbidden"
            )
        );
        assert!(st.installations.list().is_empty());
    }

    #[test]
    fn saving_and_deleting_forget_the_connection() {
        let dir = tempfile::tempdir().unwrap();
        let st = state(dir.path());
        let fake = || {
            Arc::new(crate::servers::pool::Connected {
                info: InstallationInfo {
                    installation_id: "a".into(),
                    servers: Vec::new(),
                },
                clients: Default::default(),
            })
        };
        save_installation_blocking(&st, input(sample_profile("a"), &[("s1", "sk")])).unwrap();
        st.pool.insert("a", fake());
        save_installation_blocking(&st, input(sample_profile("a"), &[])).unwrap();
        assert!(st.pool.get("a").is_none());
        st.pool.insert("a", fake());
        st.delete_installation_full("a").unwrap();
        assert!(st.pool.get("a").is_none());
        assert!(st.installations.get("a").is_err());
        assert_eq!(key(&st, "a", "s1"), None);
    }
}
