//! Команды истории метрик (спека I1c §3–4): проверка источника из формы и окно истории сервера.

use std::sync::Arc;
use std::time::Duration;

use tauri::State;

use super::blocking;
use super::installations::{InstallationInput, SecretOp, history_op};
use crate::error::{AppResult, ErrorCode};
use crate::history::check::{HistoryCheck, check_source, no_source};
use crate::history::client::{HistoryClient, SourceKind};
use crate::history::read::{HistoryAnswer, Source, read_history};
use crate::history::window::Window;
use crate::secrets::history_secret_key;
use crate::servers::client::Timeouts;
use crate::servers::live::now_ms;
use crate::state::AppState;

/// Проверяет источник истории несохранённого профиля. Секрет определяется так же, как при
/// сохранении (`history_op`): значение из формы обрезается, пустое — без секрета; без значения
/// берётся сохранённый в keychain, только если вид входа не изменился относительно сохранённого
/// профиля инсталляции.
#[tauri::command]
pub async fn check_history_source(
    state: State<'_, AppState>,
    input: InstallationInput,
) -> AppResult<HistoryCheck> {
    let secret = form_secret(state.inner(), &input).await?;
    Ok(check_source(&input.profile, secret, &state.policy.status).await)
}

/// Секрет источника для проверки: `history_op` над сохранённым профилем инсталляции (новой
/// инсталляции в хранилище ещё нет — прежнего профиля нет, и сохранённый секрет не берётся).
async fn form_secret(st: &AppState, input: &InstallationInput) -> AppResult<Option<String>> {
    let previous = st.installations.get(&input.profile.id).ok();
    match history_op(&input.profile, previous.as_ref(), &input.secrets) {
        SecretOp::Set(v) => Ok(Some(v)),
        SecretOp::Keep => {
            let (st, key) = (st.clone(), history_secret_key(&input.profile.id));
            blocking(move || st.secrets.get(&key)).await
        }
        SecretOp::Delete => Ok(None),
    }
}

/// Окно истории сервера: `now` (15 минут для «Сейчас»), `1h`, `24h`, `7d`, `30d`.
#[tauri::command]
pub async fn read_server_history(
    state: State<'_, AppState>,
    installation_id: String,
    server_id: String,
    window: Window,
) -> AppResult<HistoryAnswer> {
    read_server_history_inner(&state, &installation_id, &server_id, window).await
}

/// Клиент источника инсталляции из пула; нет — создаётся (секрет из keychain, тип источника по
/// `buildinfo`) и кладётся, если инсталляцию за это время не сбросили.
async fn history_source(st: &AppState, installation_id: &str) -> AppResult<Arc<Source>> {
    if let Some(s) = st.pool.history(installation_id) {
        return Ok(s);
    }
    let generation = st.pool.generation(installation_id);
    let profile = st.installations.get(installation_id)?;
    let h = profile.history.clone().ok_or_else(no_source)?;
    let secret = {
        let (st, key) = (st.clone(), history_secret_key(installation_id));
        blocking(move || st.secrets.get(&key)).await?
    };
    let timeouts = Timeouts::from(&profile);
    let client = HistoryClient::new(&h, timeouts, secret, &st.policy.status)?;
    let kind = client
        .buildinfo()
        .await?
        .map_or(SourceKind::Prometheus, |b| b.kind);
    let deadline = Duration::from_millis(timeouts.request_ms);
    let source = Arc::new(Source::new(client, kind, deadline));
    st.pool
        .insert_history_if_current(installation_id, generation, source.clone());
    Ok(source)
}

pub async fn read_server_history_inner(
    st: &AppState,
    installation_id: &str,
    server_id: &str,
    window: Window,
) -> AppResult<HistoryAnswer> {
    let profile = st.installations.get(installation_id)?;
    if profile.history.is_none() {
        return Err(no_source());
    }
    let selector = profile
        .server(server_id)
        .and_then(|s| s.history_selector.clone())
        .ok_or_else(|| {
            app_err!(
                ErrorCode::InvalidInput,
                "errors.history.notMatched",
                "The series of the server are not matched with the metrics history: choose a target in the installation"
            )
        })?;
    let (_, kind) = super::servers::connected_server(st, installation_id, server_id).await?;
    let source = history_source(st, installation_id).await?;
    read_history(&source, kind, &selector, window, now_ms() / 1000).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::MemoryStore;
    use crate::storage::profile::{sample_history, sample_profile};

    fn state(dir: &std::path::Path) -> AppState {
        AppState::new(dir, Arc::new(MemoryStore::default())).unwrap()
    }

    fn input(
        history: crate::storage::profile::HistoryAuth,
        secret: Option<&str>,
    ) -> InstallationInput {
        let mut p = sample_profile("a");
        let mut h = sample_history();
        h.auth = history;
        p.history = Some(h);
        InstallationInput {
            profile: p,
            secrets: crate::commands::installations::SecretsInput {
                history_secret: secret.map(str::to_string),
                ..Default::default()
            },
        }
    }

    #[tokio::test]
    async fn check_trims_the_form_secret_and_drops_a_blank_one() {
        use crate::storage::profile::HistoryAuth::Bearer;
        let dir = tempfile::tempdir().unwrap();
        let st = state(dir.path());
        st.secrets.set(&history_secret_key("a"), "saved").unwrap();
        assert_eq!(
            form_secret(
                &st,
                &input(
                    Bearer,
                    Some(
                        "  tok 
"
                    )
                )
            )
            .await
            .unwrap(),
            Some("tok".to_string())
        );
        assert_eq!(
            form_secret(&st, &input(Bearer, Some("   "))).await.unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn check_uses_the_saved_secret_only_for_the_same_auth_mode() {
        use crate::storage::profile::HistoryAuth::{Basic, Bearer};
        let dir = tempfile::tempdir().unwrap();
        let st = state(dir.path());
        st.secrets.set(&history_secret_key("a"), "saved").unwrap();
        // Инсталляции в хранилище нет: прежнего профиля нет.
        assert_eq!(form_secret(&st, &input(Bearer, None)).await.unwrap(), None);
        let mut p = sample_profile("a");
        let mut h = sample_history();
        h.auth = Bearer;
        p.history = Some(h);
        st.installations.upsert(p).unwrap();
        assert_eq!(
            form_secret(&st, &input(Bearer, None)).await.unwrap(),
            Some("saved".to_string())
        );
        // Вид входа сменился: токен не уходит паролем Basic.
        let basic = Basic {
            username: "u".into(),
        };
        assert_eq!(form_secret(&st, &input(basic, None)).await.unwrap(), None);
    }

    #[tokio::test]
    async fn without_a_source_or_a_selector_history_is_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let st = state(dir.path());
        st.installations.upsert(sample_profile("a")).unwrap();
        let e = read_server_history_inner(&st, "a", "s1", Window::Hour)
            .await
            .unwrap_err();
        assert_eq!(e.key, "errors.history.noSource");
        let mut p = sample_profile("a");
        p.history = Some(sample_history());
        st.installations.upsert(p).unwrap();
        let e = read_server_history_inner(&st, "a", "s1", Window::Hour)
            .await
            .unwrap_err();
        assert_eq!(e.key, "errors.history.notMatched");
    }

    #[tokio::test]
    async fn source_is_cached_until_the_installation_is_evicted() {
        let prom = crate::servers::testing::TestServer::start(vec![]).await;
        let dir = tempfile::tempdir().unwrap();
        let st = state(dir.path());
        let mut p = sample_profile("a");
        p.history = Some(crate::history::client::tests::source(
            &prom.url(),
            crate::storage::profile::HistoryAuth::None,
        ));
        st.installations.upsert(p).unwrap();
        let first = history_source(&st, "a").await.unwrap();
        // Без buildinfo (404) — Prometheus.
        assert_eq!(first.kind, SourceKind::Prometheus);
        let again = history_source(&st, "a").await.unwrap();
        assert!(Arc::ptr_eq(&first, &again));
        st.evict_installation("a");
        let fresh = history_source(&st, "a").await.unwrap();
        assert!(!Arc::ptr_eq(&first, &fresh));
        assert_eq!(prom.paths(), ["/api/v1/status/buildinfo"; 2]);
    }
}
