use std::path::Path;
use std::sync::Arc;

use crate::error::AppResult;
use crate::secrets::SecretStore;
use crate::storage::installations::InstallationStore;

/// Общее состояние приложения: хранилища и политика. Клонируется дёшево (всё за `Arc`).
#[derive(Clone)]
pub struct AppState {
    pub installations: Arc<InstallationStore>,
    pub secrets: Arc<dyn SecretStore>,
    pub policy: Arc<crate::policy::Policy>,
    pub settings: Arc<crate::settings::SettingsStore>,
}

impl AppState {
    pub fn new(data_dir: &Path, secrets: Arc<dyn SecretStore>) -> AppResult<Self> {
        Ok(Self {
            installations: Arc::new(InstallationStore::open(data_dir)?),
            secrets,
            policy: Arc::new(crate::policy::Policy::default()),
            settings: Arc::new(crate::settings::SettingsStore::open(data_dir)),
        })
    }

    /// Политика администратора (читается при запуске в lib.rs).
    pub fn with_policy(mut self, policy: crate::policy::Policy) -> Self {
        self.policy = Arc::new(policy);
        self
    }
}
