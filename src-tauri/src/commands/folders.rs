use std::path::Path;

use tauri::State;
use time::format_description::well_known::Rfc3339;

use super::blocking;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::storage::exchange::{
    ExportSummary, ImportOptions, ImportPreview, StoredSecrets, build_export, parse_exchange,
    plan_import,
};
use crate::storage::fs::write_json;
use crate::storage::installations::InstallationFolder;
use crate::storage::tree::TreeItemRef;

#[tauri::command]
pub fn list_folders(state: State<'_, AppState>) -> AppResult<Vec<InstallationFolder>> {
    Ok(state.installations.list_folders())
}

#[tauri::command]
pub async fn create_folder(
    state: State<'_, AppState>,
    name: String,
    parent_id: Option<String>,
) -> AppResult<InstallationFolder> {
    let st = state.inner().clone();
    blocking(move || st.installations.create_folder(&name, parent_id.as_deref())).await
}

#[tauri::command]
pub async fn rename_folder(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> AppResult<InstallationFolder> {
    let st = state.inner().clone();
    blocking(move || st.installations.rename_folder(&id, &name)).await
}

#[tauri::command]
pub async fn delete_folder(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let st = state.inner().clone();
    blocking(move || st.installations.delete_folder(&id)).await
}

#[tauri::command]
pub async fn move_items(
    state: State<'_, AppState>,
    items: Vec<TreeItemRef>,
    parent_id: Option<String>,
    before: Option<TreeItemRef>,
) -> AppResult<()> {
    let st = state.inner().clone();
    blocking(move || {
        st.installations
            .move_items(&items, parent_id.as_deref(), before.as_ref())
    })
    .await
}

/// Пишет файл `glafira-inference-installations` (всё дерево или папку `folder_id`) без секретов.
#[tauri::command]
pub async fn export_installations(
    state: State<'_, AppState>,
    path: String,
    folder_id: Option<String>,
) -> AppResult<ExportSummary> {
    let st = state.inner().clone();
    blocking(move || {
        let now = time::OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .map_err(|e| AppError::internal_with(format!("exportedAt: {e}")))?;
        export_to_file(&st, Path::new(&path), folder_id.as_deref(), &now)
    })
    .await
}

fn export_to_file(
    st: &AppState,
    path: &Path,
    folder_id: Option<&str>,
    now: &str,
) -> AppResult<ExportSummary> {
    let out = build_export(
        &st.installations,
        st.secrets.as_ref(),
        folder_id,
        env!("CARGO_PKG_VERSION"),
        now,
    )?;
    write_json(path, &out)?;
    Ok(ExportSummary {
        installations: out.installations.len(),
        folders: out.folders.len(),
    })
}

/// Предпросмотр импорта файла `path` под папку `target_folder_id` (или в корень); ничего не меняет.
#[tauri::command]
pub async fn preview_import(
    state: State<'_, AppState>,
    path: String,
    target_folder_id: Option<String>,
    update_existing: bool,
) -> AppResult<ImportPreview> {
    let st = state.inner().clone();
    blocking(move || {
        import_from_file(
            &st,
            Path::new(&path),
            target_folder_id.as_deref(),
            update_existing,
            false,
        )
    })
    .await
}

/// Применяет импорт: план пересчитывается на свежих данных и пишется одной записью.
#[tauri::command]
pub async fn apply_import(
    state: State<'_, AppState>,
    path: String,
    target_folder_id: Option<String>,
    update_existing: bool,
) -> AppResult<ImportPreview> {
    let st = state.inner().clone();
    blocking(move || {
        import_from_file(
            &st,
            Path::new(&path),
            target_folder_id.as_deref(),
            update_existing,
            true,
        )
    })
    .await
}

fn import_from_file(
    st: &AppState,
    path: &Path,
    target: Option<&str>,
    update_existing: bool,
    apply: bool,
) -> AppResult<ImportPreview> {
    let parsed = parse_exchange(&std::fs::read(path)?)?;
    let opts = ImportOptions {
        target,
        update_existing,
        allow_insecure_tls: st.policy.status.allow_insecure_tls,
    };
    let snapshot = st.installations.snapshot();
    // Keychain читается до `update`: запрос доступа (на macOS — окно с паролем) не держит
    // блокировку записи хранилища, под которой ждали бы `list_installations` и `installations.get`.
    // Без «Обновлять существующие» сохранённые инсталляции не меняются — предупреждать не о чем,
    // и keychain не читается вовсе.
    let stored = if update_existing {
        StoredSecrets::read(st.secrets.as_ref(), &snapshot, &parsed)
    } else {
        StoredSecrets::default()
    };
    if !apply {
        return Ok(plan_import(&snapshot, &parsed, &opts, &stored)?.preview);
    }
    let (preview, removed) = st.installations.update(|data| {
        let plan = plan_import(data, &parsed, &opts, &stored)?;
        *data = plan.next;
        Ok((plan.preview, plan.removed_servers))
    })?;
    // Секреты серверов, которых больше нет в профиле, не нужны: удаляются все, как при
    // сохранении и удалении инсталляции. Сбой keychain импорт не отменяет: профиль уже
    // записан, а лишняя запись в keychain ничего не открывает.
    for (installation, server) in removed {
        if let Err(e) = st.delete_server_secrets(&installation, &server) {
            tracing::warn!(
                "secrets of server {server} of installation {installation} were not deleted: {e}"
            );
        }
    }
    // Обновлённые инсталляции подключатся заново с новым профилем — как при сохранении формы.
    for item in &preview.updated {
        st.evict_installation(&item.id);
    }
    Ok(preview)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::secrets::{API_KEY, MemoryStore, SecretStore, secret_key};
    use crate::storage::installations::sample_profile;

    #[test]
    fn export_writes_file_without_secrets() {
        let dir = tempfile::tempdir().unwrap();
        let secrets = Arc::new(MemoryStore::default());
        secrets
            .set(&secret_key("a", "s1", API_KEY), "s3cr3t")
            .unwrap();
        let st = AppState::new(dir.path(), secrets).unwrap();
        let f = st.installations.create_folder("ИФТ", None).unwrap();
        let mut p = sample_profile("a");
        p.folder_id = Some(f.id.clone());
        st.installations.upsert(p).unwrap();
        st.installations.upsert(sample_profile("b")).unwrap();

        let path = dir.path().join("out.glafira-inference-installations.json");
        let sum = export_to_file(&st, &path, Some(&f.id), "2026-10-29T12:00:00Z").unwrap();
        assert_eq!(
            sum,
            ExportSummary {
                installations: 1,
                folders: 1
            }
        );
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("s3cr3t"));
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["format"], "glafira-inference-installations");
        assert_eq!(v["appVersion"], env!("CARGO_PKG_VERSION"));
        assert_eq!(v["installations"][0]["id"], "a");
        assert_eq!(v["installations"][0]["needsSecret"]["apiKeys"][0], "s1");

        let err = export_to_file(&st, &path, Some("nope"), "now").unwrap_err();
        assert_eq!(err.key, "errors.folders.notFound");
    }

    fn write_file(dir: &Path, base_url: &str) -> std::path::PathBuf {
        let mut p = serde_json::to_value(sample_profile("a")).unwrap();
        p["servers"][0]["baseUrl"] = base_url.into();
        let mut n = serde_json::to_value(sample_profile("n")).unwrap();
        n["environment"] = "prod".into();
        let path = dir.join("in.glafira-inference-installations.json");
        std::fs::write(
            &path,
            serde_json::to_vec(&serde_json::json!({
                "format": "glafira-inference-installations", "version": 1,
                "folders": [], "installations": [p, n],
            }))
            .unwrap(),
        )
        .unwrap();
        path
    }

    #[test]
    fn preview_does_not_write_and_apply_writes_once() {
        let dir = tempfile::tempdir().unwrap();
        let st = AppState::new(dir.path(), Arc::new(MemoryStore::default())).unwrap();
        st.installations.upsert(sample_profile("a")).unwrap();
        let input = tempfile::tempdir().unwrap();
        let path = write_file(input.path(), "http://changed:8000");
        let conns = dir.path().join("installations.json");
        let before = std::fs::read(&conns).unwrap();

        st.pool.insert(
            "a",
            Arc::new(crate::servers::pool::Connected {
                info: crate::servers::pool::InstallationInfo {
                    installation_id: "a".into(),
                    servers: Vec::new(),
                },
                clients: Default::default(),
            }),
        );
        let preview = import_from_file(&st, &path, None, true, false).unwrap();
        assert_eq!(preview.updated.len(), 1);
        assert_eq!(preview.added.len(), 1);
        assert!(st.pool.get("a").is_some(), "предпросмотр ничего не меняет");
        // prod-инсталляция импортируется как в файле: без принудительного read-only.
        assert!(preview.warnings.is_empty());
        assert_eq!(std::fs::read(&conns).unwrap(), before);

        let applied = import_from_file(&st, &path, None, true, true).unwrap();
        assert_eq!(applied, preview);
        assert!(
            st.pool.get("a").is_none(),
            "обновлённая инсталляция подключится заново"
        );
        assert_eq!(
            st.installations.get("a").unwrap().servers[0].base_url,
            "http://changed:8000"
        );
        assert!(!st.installations.get("n").unwrap().read_only);
        let reopened = crate::storage::installations::InstallationStore::open(dir.path()).unwrap();
        assert_eq!(reopened.list().len(), 2);
    }

    #[test]
    fn changed_address_of_a_server_with_an_api_key_is_warned_about() {
        let dir = tempfile::tempdir().unwrap();
        let secrets = Arc::new(MemoryStore::default());
        secrets
            .set(&secret_key("a", "s1", API_KEY), "sk-t")
            .unwrap();
        let st = AppState::new(dir.path(), secrets.clone()).unwrap();
        st.installations.upsert(sample_profile("a")).unwrap();
        let path = write_file(dir.path(), "http://changed:8000");

        let preview = import_from_file(&st, &path, None, true, false).unwrap();
        assert_eq!(preview.warnings.len(), 1);
        assert_eq!(preview.warnings[0].key, "messages.import.addressChanged");
        assert_eq!(preview.warnings[0].params["name"], "inst-a");
        // Результат применения содержит то же предупреждение, ключ остаётся на месте.
        let applied = import_from_file(&st, &path, None, true, true).unwrap();
        assert_eq!(applied, preview);
        assert_eq!(
            secrets
                .get(&secret_key("a", "s1", API_KEY))
                .unwrap()
                .as_deref(),
            Some("sk-t")
        );
    }

    /// Keychain, который запоминает для каждого чтения, держал ли кто-то в этот момент
    /// блокировку записи хранилища инсталляций.
    #[derive(Default)]
    struct LockProbe {
        inner: MemoryStore,
        store: std::sync::OnceLock<Arc<crate::storage::installations::InstallationStore>>,
        reads: std::sync::Mutex<Vec<bool>>,
    }

    impl SecretStore for LockProbe {
        fn get(&self, key: &str) -> AppResult<Option<String>> {
            let locked = self.store.get().is_some_and(|s| s.is_write_locked());
            self.reads.lock().unwrap().push(locked);
            self.inner.get(key)
        }
        fn set(&self, key: &str, value: &str) -> AppResult<()> {
            self.inner.set(key, value)
        }
        fn delete(&self, key: &str) -> AppResult<()> {
            self.inner.delete(key)
        }
    }

    /// Применение импорта читает keychain до блокировки записи хранилища: запрос доступа к
    /// keychain не задерживает `list_installations` и `installations.get`.
    #[test]
    fn import_reads_the_keychain_outside_the_store_lock() {
        let dir = tempfile::tempdir().unwrap();
        let probe = Arc::new(LockProbe::default());
        probe
            .inner
            .set(&secret_key("a", "s1", API_KEY), "sk-t")
            .unwrap();
        let st = AppState::new(dir.path(), probe.clone()).unwrap();
        assert!(probe.store.set(st.installations.clone()).is_ok());
        st.installations.upsert(sample_profile("a")).unwrap();
        probe.reads.lock().unwrap().clear();
        let path = write_file(dir.path(), "http://changed:8000");

        let applied = import_from_file(&st, &path, None, true, true).unwrap();
        assert_eq!(applied.warnings[0].key, "messages.import.addressChanged");
        let reads = probe.reads.lock().unwrap().clone();
        assert!(!reads.is_empty(), "ключ прочитан");
        assert!(reads.iter().all(|locked| !locked), "{reads:?}");
    }

    /// Без «Обновлять существующие» предупреждать не о чем: ни предпросмотр, ни применение
    /// keychain не читают (на macOS — без лишнего запроса доступа).
    #[test]
    fn import_without_updating_existing_does_not_touch_the_keychain() {
        let dir = tempfile::tempdir().unwrap();
        let probe = Arc::new(LockProbe::default());
        let st = AppState::new(dir.path(), probe.clone()).unwrap();
        st.installations.upsert(sample_profile("a")).unwrap();
        probe.reads.lock().unwrap().clear();
        let path = write_file(dir.path(), "http://changed:8000");

        let preview = import_from_file(&st, &path, None, false, false).unwrap();
        assert!(preview.updated.is_empty() && preview.warnings.is_empty());
        let applied = import_from_file(&st, &path, None, false, true).unwrap();
        assert_eq!(applied.added.len(), 1);
        assert!(probe.reads.lock().unwrap().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn failed_apply_changes_nothing() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let st = AppState::new(&data, Arc::new(MemoryStore::default())).unwrap();
        st.installations.upsert(sample_profile("a")).unwrap();
        let path = write_file(dir.path(), "http://changed:8000");
        let conns = data.join("installations.json");
        let before = std::fs::read(&conns).unwrap();

        // Каталог данных только для чтения: временный файл не создать, запись падает.
        std::fs::set_permissions(&data, std::fs::Permissions::from_mode(0o555)).unwrap();
        let result = import_from_file(&st, &path, None, true, true);
        std::fs::set_permissions(&data, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(result.is_err());
        assert_eq!(std::fs::read(&conns).unwrap(), before);
        assert_eq!(st.installations.list().len(), 1);
        assert_eq!(
            st.installations.get("a").unwrap().servers[0].base_url,
            "https://s1.dev.example.com:8000"
        );
    }

    #[test]
    fn policy_forbidding_insecure_tls_skips_such_installations() {
        let dir = tempfile::tempdir().unwrap();
        let mut policy = crate::policy::Policy::default();
        policy.status.allow_insecure_tls = false;
        let st = AppState::new(dir.path(), Arc::new(MemoryStore::default()))
            .unwrap()
            .with_policy(policy);
        let mut insecure = serde_json::to_value(sample_profile("i")).unwrap();
        insecure["servers"][0]["ca"] = serde_json::json!({"mode": "insecure"});
        let path = dir.path().join("in.glafira-inference-installations.json");
        std::fs::write(
            &path,
            serde_json::to_vec(&serde_json::json!({
                "format": "glafira-inference-installations", "version": 1,
                "folders": [], "installations": [insecure],
            }))
            .unwrap(),
        )
        .unwrap();
        let applied = import_from_file(&st, &path, None, true, true).unwrap();
        assert!(applied.added.is_empty());
        assert_eq!(
            applied.skipped[0].reason.key,
            "errors.import.insecureForbidden"
        );
        assert!(st.installations.list().is_empty());
    }

    #[test]
    fn foreign_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let st = AppState::new(dir.path(), Arc::new(MemoryStore::default())).unwrap();
        let path = dir.path().join("x.json");
        std::fs::write(&path, br#"{"format":"other","version":1}"#).unwrap();
        let err = import_from_file(&st, &path, None, true, false).unwrap_err();
        assert_eq!(err.key, "errors.import.notGlafiraFile");
    }

    #[test]
    fn keys_of_servers_removed_by_import_are_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let secrets = Arc::new(MemoryStore::default());
        for server in ["s1", "s2"] {
            secrets
                .set(&secret_key("a", server, API_KEY), "sk-t")
                .unwrap();
        }
        let st = AppState::new(dir.path(), secrets.clone()).unwrap();
        let mut two = sample_profile("a");
        two.servers
            .push(crate::storage::profile::sample_server("s2"));
        st.installations.upsert(two).unwrap();
        let path = write_file(dir.path(), "https://s1.dev.example.com:8000");

        let preview = import_from_file(&st, &path, None, true, false).unwrap();
        assert_eq!(preview.updated.len(), 1);
        assert!(
            secrets
                .get(&secret_key("a", "s2", API_KEY))
                .unwrap()
                .is_some(),
            "предпросмотр ничего не удаляет"
        );
        import_from_file(&st, &path, None, true, true).unwrap();
        assert_eq!(st.installations.get("a").unwrap().servers.len(), 1);
        assert_eq!(secrets.get(&secret_key("a", "s2", API_KEY)).unwrap(), None);
        assert!(
            secrets
                .get(&secret_key("a", "s1", API_KEY))
                .unwrap()
                .is_some()
        );
    }

    /// Keychain, который ничего не удаляет; остальное — как `MemoryStore`.
    struct DenyDelete(MemoryStore);

    impl SecretStore for DenyDelete {
        fn get(&self, key: &str) -> AppResult<Option<String>> {
            self.0.get(key)
        }
        fn set(&self, key: &str, value: &str) -> AppResult<()> {
            self.0.set(key, value)
        }
        fn delete(&self, _key: &str) -> AppResult<()> {
            Err(AppError::internal_with("denied"))
        }
    }

    /// Журнал, записанный за время `f`.
    fn logged(f: impl FnOnce()) -> String {
        use tracing_subscriber::fmt::MakeWriter;

        #[derive(Clone, Default)]
        struct Buf(Arc<std::sync::Mutex<Vec<u8>>>);

        impl std::io::Write for Buf {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        impl<'a> MakeWriter<'a> for Buf {
            type Writer = Buf;
            fn make_writer(&'a self) -> Buf {
                self.clone()
            }
        }

        let buf = Buf::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buf.clone())
            .with_ansi(false)
            .finish();
        tracing::subscriber::with_default(subscriber, f);
        let out = buf.0.lock().unwrap().clone();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn failed_cleanup_of_a_removed_server_is_logged_with_its_ids_and_does_not_fail_the_import() {
        let dir = tempfile::tempdir().unwrap();
        let st = AppState::new(dir.path(), Arc::new(DenyDelete(MemoryStore::default()))).unwrap();
        let mut two = sample_profile("a");
        two.servers
            .push(crate::storage::profile::sample_server("s2"));
        st.installations.upsert(two).unwrap();
        let path = write_file(dir.path(), "https://s1.dev.example.com:8000");

        let mut result = None;
        let log = logged(|| result = Some(import_from_file(&st, &path, None, true, true)));
        assert!(
            result.unwrap().is_ok(),
            "сбой keychain импорт не отменяет: профиль уже записан"
        );
        assert_eq!(st.installations.get("a").unwrap().servers.len(), 1);
        assert!(
            log.contains("server s2") && log.contains("installation a"),
            "в журнале id инсталляции и сервера: {log}"
        );
    }
}
