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
#[derive(PartialEq, Eq)]
enum SecretOp {
    Keep,
    Set(String),
    Delete,
}

/// Значение ключа не печатается: операция может попасть в лог или сообщение об ошибке.
impl std::fmt::Debug for SecretOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Keep => f.write_str("Keep"),
            Self::Set(_) => f.debug_tuple("Set").field(&"***").finish(),
            Self::Delete => f.write_str("Delete"),
        }
    }
}

/// Ключи серверов профиля — по данным формы. Серверу, которого не было в прежнем профиле,
/// сохранённый ключ взять неоткуда: без ключа в форме запись с его id — осиротевшая (от
/// прежнего сбоя или другой установки), и она удаляется, а не прикрепляется к новому серверу.
fn secret_ops(
    saved: &InstallationProfile,
    previous: Option<&InstallationProfile>,
    s: &SecretsInput,
) -> Vec<(String, SecretOp)> {
    saved
        .servers
        .iter()
        .map(|server| {
            let op = match s.api_keys.get(&server.id).map(|v| v.trim()) {
                Some("") => SecretOp::Delete,
                Some(v) => SecretOp::Set(v.to_string()),
                None if previous.is_some_and(|p| p.server(&server.id).is_some()) => SecretOp::Keep,
                None => SecretOp::Delete,
            };
            (server.id.clone(), op)
        })
        .collect()
}

/// Серверы прежнего профиля, которых нет в сохранённом: их ключи больше не нужны.
fn removed_servers(
    saved: &InstallationProfile,
    previous: Option<&InstallationProfile>,
) -> Vec<String> {
    previous
        .map(|p| p.servers.as_slice())
        .unwrap_or_default()
        .iter()
        .filter(|old| saved.server(&old.id).is_none())
        .map(|old| old.id.clone())
        .collect()
}

/// Применяет операции над ключами серверов профиля. Перед каждым `Set` и `Delete` читается
/// прежнее значение: если любая операция откажет, уже изменённые ключи возвращаются к нему
/// (`Some` — `set`, `None` — `delete`), и keychain остаётся таким, каким был. Сбой самого
/// восстановления пишется в журнал (без значения), вызвавший получает исходную ошибку.
fn apply_secret_ops(
    st: &AppState,
    installation_id: &str,
    ops: Vec<(String, SecretOp)>,
) -> AppResult<()> {
    // Изменённые ключи вместе с прежним значением (`None` — ключа не было).
    let mut done: Vec<(String, Option<String>)> = Vec::new();
    // `Keep` ключ не трогает — keychain для него не читается.
    for (server, op) in ops.into_iter().filter(|(_, op)| *op != SecretOp::Keep) {
        let key = secret_key(installation_id, &server, API_KEY);
        let step = st.secrets.get(&key).and_then(|before| {
            match &op {
                SecretOp::Set(v) => st.secrets.set(&key, v),
                SecretOp::Delete => st.secrets.delete(&key),
                SecretOp::Keep => Ok(()),
            }
            .map(|()| before)
        });
        match step {
            Ok(before) => done.push((key, before)),
            Err(e) => {
                restore_secrets(st, done);
                return Err(e);
            }
        }
    }
    Ok(())
}

/// Возвращает ключи к прежним значениям, начиная с последнего изменённого.
fn restore_secrets(st: &AppState, done: Vec<(String, Option<String>)>) {
    for (key, before) in done.into_iter().rev() {
        let result = match before {
            Some(v) => st.secrets.set(&key, &v),
            None => st.secrets.delete(&key),
        };
        if let Err(e) = result {
            tracing::warn!("api key {key} was not restored after a failed save: {e}");
        }
    }
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

/// Сохраняет профиль и ключи согласованно: если keychain не принял изменение, ключи
/// возвращаются к прежним значениям, а профиль — к прежнему состоянию (или удаляется, если
/// он новый). Ключи убранных серверов чистятся после фиксации и сохранение не отменяют.
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
    // Ключи меняются по одному, прежние значения запоминаются: если keychain откажет посреди
    // изменений, `apply_secret_ops` вернёт уже изменённые ключи, а профиль откатится ниже.
    let ops = secret_ops(&saved, previous.as_ref(), &secrets);
    let result = apply_secret_ops(st, &saved.id, ops);
    if result.is_err() {
        let rollback = match previous {
            Some(p) => st.installations.upsert(p).map(|_| ()),
            None => st.installations.delete(&saved.id),
        };
        if let Err(r) = rollback {
            tracing::error!("failed to roll back profile {}: {r}", saved.id);
        }
    } else {
        // Ключи убранных серверов больше не нужны. Сбой keychain сохранение не отменяет:
        // профиль уже записан, а лишняя запись в keychain ничего не открывает.
        for server in removed_servers(&saved, previous.as_ref()) {
            if let Err(e) = st.delete_server_secrets(&saved.id, &server) {
                tracing::warn!("secrets of a removed server were not deleted: {e}");
            }
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
    use std::sync::atomic::{AtomicUsize, Ordering};

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
        saved.servers.push(sample_server("s4"));
        let s = SecretsInput {
            api_keys: [("s2", " sk-2 "), ("s3", " ")]
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        };
        // Убранный сервер (`gone`) в операции не входит: его ключ чистится после фиксации.
        // Новый сервер без ключа в форме (`s4`) получает `Delete`, а не `Keep`: осиротевший
        // ключ с тем же id не должен прикрепиться к нему.
        assert_eq!(
            secret_ops(&saved, Some(&prev), &s),
            [
                ("s1".to_string(), SecretOp::Keep),
                ("s2".to_string(), SecretOp::Set("sk-2".into())),
                ("s3".to_string(), SecretOp::Delete),
                ("s4".to_string(), SecretOp::Delete),
            ]
        );
        // У новой инсталляции прежнего профиля нет: все серверы без ключа в форме — `Delete`.
        assert_eq!(
            secret_ops(&sample_profile("a"), None, &SecretsInput::default()),
            [("s1".to_string(), SecretOp::Delete)]
        );
        assert_eq!(format!("{s:?}"), r#"{"s2": "***", "s3": "***"}"#);
    }

    #[test]
    fn secret_op_debug_hides_the_key() {
        let debug = format!("{:?}", SecretOp::Set("sk-secret".into()));
        assert!(!debug.contains("sk-secret"));
        assert!(debug.contains("***"));
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

    /// Keychain с управляемыми сбоями: `set` падает на вызовах с перечисленными номерами
    /// (нумерация с 1, вызовы самого теста мимо обёртки не считаются), `delete` — на
    /// перечисленных ключах. Остальное — как `MemoryStore`.
    struct FlakyKeychain {
        inner: MemoryStore,
        set_calls: AtomicUsize,
        fail_set: Vec<usize>,
        fail_delete: Vec<String>,
    }

    impl FlakyKeychain {
        fn new(fail_set: &[usize], fail_delete: &[String]) -> Arc<Self> {
            Arc::new(Self {
                inner: MemoryStore::default(),
                set_calls: AtomicUsize::new(0),
                fail_set: fail_set.to_vec(),
                fail_delete: fail_delete.to_vec(),
            })
        }
    }

    impl SecretStore for FlakyKeychain {
        fn get(&self, key: &str) -> AppResult<Option<String>> {
            self.inner.get(key)
        }
        fn set(&self, key: &str, value: &str) -> AppResult<()> {
            let n = self.set_calls.fetch_add(1, Ordering::SeqCst) + 1;
            if self.fail_set.contains(&n) {
                return Err(AppError::internal_with(format!("set #{n} denied")));
            }
            self.inner.set(key, value)
        }
        fn delete(&self, key: &str) -> AppResult<()> {
            if self.fail_delete.iter().any(|k| k == key) {
                return Err(AppError::internal_with(format!("delete {key} denied")));
            }
            self.inner.delete(key)
        }
    }

    fn flaky_state(dir: &std::path::Path, kc: &Arc<FlakyKeychain>) -> AppState {
        AppState::new(dir, kc.clone()).unwrap()
    }

    fn two_servers(id: &str) -> InstallationProfile {
        let mut p = sample_profile(id);
        p.servers.push(sample_server("s2"));
        p
    }

    #[test]
    fn failed_second_set_of_a_new_installation_leaves_no_keys() {
        let dir = tempfile::tempdir().unwrap();
        let kc = FlakyKeychain::new(&[2], &[]);
        let st = flaky_state(dir.path(), &kc);
        let keys = [("s1", "sk-1"), ("s2", "sk-2")];
        assert!(save_installation_blocking(&st, input(two_servers("a"), &keys)).is_err());
        assert!(st.installations.get("a").is_err(), "новый профиль удалён");
        assert_eq!(
            key(&st, "a", "s1"),
            None,
            "ключ первого сервера не осиротел"
        );
        assert_eq!(key(&st, "a", "s2"), None);
    }

    #[test]
    fn failed_second_set_of_an_existing_installation_restores_previous_keys() {
        let dir = tempfile::tempdir().unwrap();
        let kc = FlakyKeychain::new(&[2], &[]);
        let st = flaky_state(dir.path(), &kc);
        st.installations.upsert(two_servers("a")).unwrap();
        kc.inner
            .set(&secret_key("a", "s1", API_KEY), "old-1")
            .unwrap();
        kc.inner
            .set(&secret_key("a", "s2", API_KEY), "old-2")
            .unwrap();
        let mut renamed = two_servers("a");
        renamed.name = "renamed".into();
        let keys = [("s1", "new-1"), ("s2", "new-2")];
        assert!(save_installation_blocking(&st, input(renamed, &keys)).is_err());
        assert_eq!(st.installations.get("a").unwrap().name, "inst-a");
        assert_eq!(key(&st, "a", "s1").as_deref(), Some("old-1"));
        assert_eq!(key(&st, "a", "s2").as_deref(), Some("old-2"));
    }

    #[test]
    fn explicit_delete_is_undone_when_a_later_operation_fails() {
        let dir = tempfile::tempdir().unwrap();
        let kc = FlakyKeychain::new(&[1], &[]);
        let st = flaky_state(dir.path(), &kc);
        st.installations.upsert(two_servers("a")).unwrap();
        kc.inner
            .set(&secret_key("a", "s1", API_KEY), "old-1")
            .unwrap();
        // s1: `Delete` проходит, s2: `Set` падает — ключ s1 должен вернуться.
        let keys = [("s1", ""), ("s2", "new-2")];
        assert!(save_installation_blocking(&st, input(two_servers("a"), &keys)).is_err());
        assert_eq!(key(&st, "a", "s1").as_deref(), Some("old-1"));
        assert_eq!(key(&st, "a", "s2"), None);
    }

    #[test]
    fn failed_restore_does_not_hide_the_original_error() {
        let dir = tempfile::tempdir().unwrap();
        // Второй `set` — сбой сохранения, третий — сбой восстановления ключа s1.
        let kc = FlakyKeychain::new(&[2, 3], &[]);
        let st = flaky_state(dir.path(), &kc);
        st.installations.upsert(two_servers("a")).unwrap();
        kc.inner
            .set(&secret_key("a", "s1", API_KEY), "old-1")
            .unwrap();
        let keys = [("s1", "new-1"), ("s2", "new-2")];
        let e = save_installation_blocking(&st, input(two_servers("a"), &keys)).unwrap_err();
        assert_eq!(e.details.as_deref(), Some("set #2 denied"));
        assert!(st.installations.get("a").is_ok(), "профиль на месте");
    }

    #[test]
    fn failed_cleanup_of_a_removed_server_key_does_not_undo_the_save() {
        let dir = tempfile::tempdir().unwrap();
        let kc = FlakyKeychain::new(&[], &[secret_key("a", "s2", API_KEY)]);
        let st = flaky_state(dir.path(), &kc);
        st.installations.upsert(two_servers("a")).unwrap();
        kc.inner
            .set(&secret_key("a", "s1", API_KEY), "sk-1")
            .unwrap();
        kc.inner
            .set(&secret_key("a", "s2", API_KEY), "sk-2")
            .unwrap();
        let mut one = sample_profile("a");
        one.name = "renamed".into();
        let saved = save_installation_blocking(&st, input(one, &[]))
            .expect("сбой очистки не отменяет сохранение");
        assert_eq!(saved.servers.len(), 1);
        let stored = st.installations.get("a").unwrap();
        assert_eq!((stored.name.as_str(), stored.servers.len()), ("renamed", 1));
        assert_eq!(key(&st, "a", "s1").as_deref(), Some("sk-1"));
    }

    #[test]
    fn orphan_key_is_not_adopted_by_a_new_server() {
        let dir = tempfile::tempdir().unwrap();
        let st = state(dir.path());
        // Осиротевшие ключи: у сервера, которого в профиле ещё нет, и у новой инсталляции.
        st.secrets
            .set(&secret_key("a", "s2", API_KEY), "orphan-a")
            .unwrap();
        st.secrets
            .set(&secret_key("b", "s1", API_KEY), "orphan-b")
            .unwrap();
        st.installations.upsert(sample_profile("a")).unwrap();
        save_installation_blocking(&st, input(two_servers("a"), &[])).unwrap();
        save_installation_blocking(&st, input(sample_profile("b"), &[])).unwrap();
        assert_eq!(key(&st, "a", "s2"), None);
        assert_eq!(key(&st, "b", "s1"), None);
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
