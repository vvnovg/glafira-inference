//! Команды GPU (спека I1d §3–4): проверка компонента из формы, снимок карт для окна «Сейчас» и
//! окно истории карт хоста.

use std::sync::Arc;

use tauri::State;

use super::blocking;
use super::history::{form_secret, history_source};
use super::installations::{InstallationInput, SecretOp, gpu_op};
use crate::error::{AppResult, ErrorCode};
use crate::gpu::check::{GpuCheck, check_gpu};
use crate::gpu::live::{GpuStatus, record};
use crate::gpu::read::{GpuHistoryAnswer, base, read_history};
use crate::gpu::source::{OpenGpu, snapshot};
use crate::history::check::no_source;
use crate::history::window::Window;
use crate::secrets::gpu_secret_key;
use crate::servers::client::Timeouts;
use crate::servers::live::now_ms;
use crate::state::AppState;
use crate::storage::profile::GpuSource;

/// Проверяет компонент GPU несохранённого профиля. Пароль экспортёров и секрет источника
/// истории — как при сохранении (`gpu_op`, `history_op`): из формы, без значения — сохранённый,
/// только если вид входа не менялся.
#[tauri::command]
pub async fn check_gpu_source(
    state: State<'_, AppState>,
    input: InstallationInput,
) -> AppResult<GpuCheck> {
    let st = state.inner();
    let password = form_password(st, &input).await?;
    let history = form_secret(st, &input).await?;
    Ok(check_gpu(&input.profile, password, history, &st.policy.status).await)
}

/// Пароль экспортёров для проверки: `gpu_op` над сохранённым профилем инсталляции.
async fn form_password(st: &AppState, input: &InstallationInput) -> AppResult<Option<String>> {
    let previous = st.installations.get(&input.profile.id).ok();
    match gpu_op(&input.profile, previous.as_ref(), &input.secrets) {
        SecretOp::Set(v) => Ok(Some(v)),
        SecretOp::Keep => {
            let (st, key) = (st.clone(), gpu_secret_key(&input.profile.id));
            blocking(move || st.secrets.get(&key)).await
        }
        SecretOp::Delete => Ok(None),
    }
}

fn not_configured() -> crate::error::AppError {
    app_err!(
        ErrorCode::InvalidInput,
        "errors.gpu.notConfigured",
        "The installation has no GPU component"
    )
}

/// Компонент GPU инсталляции из пула; нет — открывается (пароль из keychain, источник истории —
/// из пула) и кладётся, если инсталляцию за это время не сбросили.
async fn open_gpu(st: &AppState, installation_id: &str) -> AppResult<Arc<OpenGpu>> {
    if let Some(open) = st.pool.gpu(installation_id) {
        return Ok(open);
    }
    let generation = st.pool.generation(installation_id);
    let profile = st.installations.get(installation_id)?;
    let open = match profile.gpu.clone().ok_or_else(not_configured)? {
        GpuSource::Direct(d) => {
            let password = {
                let (st, key) = (st.clone(), gpu_secret_key(installation_id));
                blocking(move || st.secrets.get(&key)).await?
            };
            OpenGpu::direct(&d, Timeouts::from(&profile), password, &st.policy.status)?
        }
        GpuSource::Prometheus(g) => {
            OpenGpu::prometheus(&g, history_source(st, installation_id).await?)
        }
    };
    let open = Arc::new(open);
    st.pool
        .insert_gpu_if_current(installation_id, generation, open.clone());
    Ok(open)
}

/// Снимок карт инсталляции с приростами счётчиков за последнюю минуту опросов.
#[tauri::command]
pub async fn read_gpu_status(
    state: State<'_, AppState>,
    installation_id: String,
) -> AppResult<GpuStatus> {
    read_gpu_status_inner(&state, &installation_id).await
}

pub async fn read_gpu_status_inner(st: &AppState, installation_id: &str) -> AppResult<GpuStatus> {
    let open = open_gpu(st, installation_id).await?;
    let live = st.pool.gpu_live(installation_id);
    // Опросы из нескольких вкладок идут по очереди: приросты считаются по порядку снимков.
    let mut history = live.lock().await;
    let (cards, errors) = open.read().await?;
    Ok(record(&mut history, snapshot(cards, errors), now_ms()))
}

/// Окно истории карт хоста `host`: одной карты `gpu` или всех (не больше 16).
#[tauri::command]
pub async fn read_gpu_history(
    state: State<'_, AppState>,
    installation_id: String,
    window: Window,
    host: String,
    gpu: Option<u32>,
) -> AppResult<GpuHistoryAnswer> {
    read_gpu_history_inner(&state, &installation_id, window, &host, gpu).await
}

pub async fn read_gpu_history_inner(
    st: &AppState,
    installation_id: &str,
    window: Window,
    host: &str,
    gpu: Option<u32>,
) -> AppResult<GpuHistoryAnswer> {
    let profile = st.installations.get(installation_id)?;
    if profile.history.is_none() {
        return Err(no_source());
    }
    let bases = match profile.gpu.as_ref().ok_or_else(not_configured)? {
        GpuSource::Prometheus(g) => vec![base(g.selector.as_deref())?],
        GpuSource::Direct(d) => d
            .exporters
            .iter()
            .filter_map(|e| e.history_selector.as_deref())
            .map(|s| base(Some(s)))
            .collect::<AppResult<Vec<_>>>()?,
    };
    if bases.is_empty() {
        return Err(app_err!(
            ErrorCode::InvalidInput,
            "errors.gpu.notMatched",
            "The DCGM exporters are not matched with the metrics history: choose their targets in the installation"
        ));
    }
    let source = history_source(st, installation_id).await?;
    read_history(&source, &bases, host, gpu, window, now_ms() / 1000).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::installations::SecretsInput;
    use crate::gpu::fields::Field;
    use crate::gpu::snapshot::tests::FULL;
    use crate::secrets::MemoryStore;
    use crate::servers::testing::{Reply, TestServer};
    use crate::storage::profile::{
        DirectGpu, GpuAuth, PrometheusGpu, ProxyConfig, sample_exporter, sample_gpu,
        sample_history, sample_profile,
    };

    fn state(dir: &std::path::Path) -> AppState {
        AppState::new(dir, Arc::new(MemoryStore::default())).unwrap()
    }

    fn direct(urls: &[String], auth: GpuAuth) -> GpuSource {
        let GpuSource::Direct(d) = sample_gpu() else {
            unreachable!()
        };
        GpuSource::Direct(DirectGpu {
            exporters: urls.iter().map(|u| sample_exporter(u)).collect(),
            auth,
            proxy: ProxyConfig::None,
            ..d
        })
    }

    fn glafira() -> GpuAuth {
        GpuAuth::Basic {
            username: "glafira".into(),
        }
    }

    #[tokio::test]
    async fn the_form_password_follows_the_save_rules() {
        let dir = tempfile::tempdir().unwrap();
        let st = state(dir.path());
        st.secrets.set(&gpu_secret_key("a"), "saved").unwrap();
        let mut p = sample_profile("a");
        p.gpu = Some(direct(&["http://gpu:9400/metrics".into()], glafira()));
        let input = |secret: Option<&str>, p: &crate::storage::profile::InstallationProfile| {
            InstallationInput {
                profile: p.clone(),
                secrets: SecretsInput {
                    gpu_secret: secret.map(str::to_string),
                    ..Default::default()
                },
            }
        };
        assert_eq!(
            form_password(&st, &input(None, &p)).await.unwrap(),
            None,
            "новой инсталляции сохранённый не положен"
        );
        st.installations.upsert(p.clone()).unwrap();
        assert_eq!(
            form_password(&st, &input(None, &p))
                .await
                .unwrap()
                .as_deref(),
            Some("saved")
        );
        assert_eq!(
            form_password(&st, &input(Some(" typed "), &p))
                .await
                .unwrap()
                .as_deref(),
            Some("typed")
        );
        let mut open = p.clone();
        open.gpu = Some(direct(&["http://gpu:9400/metrics".into()], GpuAuth::None));
        assert_eq!(form_password(&st, &input(None, &open)).await.unwrap(), None);
    }

    #[tokio::test]
    async fn status_reads_the_cards_and_counts_deltas_between_polls() {
        let exporter = TestServer::start(vec![("/metrics", Reply::text(FULL))]).await;
        let dir = tempfile::tempdir().unwrap();
        let st = state(dir.path());
        let mut p = sample_profile("a");
        p.gpu = Some(direct(
            &[format!("{}/metrics", exporter.url())],
            GpuAuth::None,
        ));
        st.installations.upsert(p).unwrap();
        let first = read_gpu_status_inner(&st, "a").await.unwrap();
        assert_eq!(first.snapshot.hosts[0].cards.len(), 8);
        assert_eq!(first.delta_sec, None);
        let second = read_gpu_status_inner(&st, "a").await.unwrap();
        assert!(second.delta_sec.is_some(), "второй опрос — с приростами");
        assert_eq!(
            second.snapshot.hosts[0].cards[0].deltas[&Field::PcieReplay],
            0.0,
            "снимок тот же — прироста нет"
        );
        let open = st.pool.gpu("a").unwrap();
        st.evict_installation("a");
        assert!(st.pool.gpu("a").is_none());
        let third = read_gpu_status_inner(&st, "a").await.unwrap();
        assert_eq!(
            third.delta_sec, None,
            "после сброса инсталляции приросты начинаются заново"
        );
        assert!(!Arc::ptr_eq(&open, &st.pool.gpu("a").unwrap()));
    }

    #[tokio::test]
    async fn without_the_component_or_history_there_is_nothing_to_read() {
        let dir = tempfile::tempdir().unwrap();
        let st = state(dir.path());
        st.installations.upsert(sample_profile("a")).unwrap();
        assert_eq!(
            read_gpu_status_inner(&st, "a").await.unwrap_err().key,
            "errors.gpu.notConfigured"
        );
        let e = read_gpu_history_inner(&st, "a", Window::Hour, "h", None)
            .await
            .unwrap_err();
        assert_eq!(e.key, "errors.history.noSource");
        let mut p = sample_profile("a");
        p.history = Some(sample_history());
        p.gpu = Some(direct(&["http://gpu:9400/metrics".into()], GpuAuth::None));
        st.installations.upsert(p).unwrap();
        let e = read_gpu_history_inner(&st, "a", Window::Hour, "h", None)
            .await
            .unwrap_err();
        assert_eq!(
            e.key, "errors.gpu.notMatched",
            "экспортёры без селекторов истории"
        );
    }

    #[tokio::test]
    async fn prometheus_mode_reads_status_through_the_history_source() {
        let prom = TestServer::start(vec![(
            "/api/v1/query",
            Reply::json(&crate::gpu::source::tests::snapshot_file("instant.json")),
        )])
        .await;
        let dir = tempfile::tempdir().unwrap();
        let st = state(dir.path());
        let mut p = sample_profile("a");
        p.history = Some(crate::history::client::tests::source(
            &prom.url(),
            crate::storage::profile::HistoryAuth::None,
        ));
        p.gpu = Some(GpuSource::Prometheus(PrometheusGpu {
            selector: None,
            extra: Default::default(),
        }));
        st.installations.upsert(p).unwrap();
        let status = read_gpu_status_inner(&st, "a").await.unwrap();
        assert_eq!(status.snapshot.hosts.len(), 2);
        assert!(
            st.pool.history("a").is_some(),
            "источник истории открыт общим для окон сервера"
        );
    }
}
