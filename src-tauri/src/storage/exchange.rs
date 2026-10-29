//! Файл обмена инсталляциями (`glafira-inference-installations`): экспорт без секретов, предпросмотр и
//! применение импорта.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::error::{AppResult, ErrorCode};
use crate::i18n::LocalizedText;
use crate::secrets::{API_KEY, SecretStore, secret_key};
use crate::storage::installations::{
    InstallationFolder, InstallationProfile, InstallationStore, StoreData, check_tree,
    folder_not_found, unique_name, validate_profile,
};
use crate::storage::profile::{CaConfig, ServerConfig, normalize_profile, strip_private_keys};
use crate::storage::tree::TreeItemKind;

pub const EXCHANGE_FORMAT: &str = "glafira-inference-installations";
pub const EXCHANGE_VERSION: u32 = 1;

/// Какие секреты были сохранены у инсталляции при экспорте: получатель вводит свои.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NeedsSecret {
    /// Серверы (id), у которых был сохранён ключ API.
    #[serde(default)]
    pub api_keys: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeFileOut<'a> {
    pub format: &'a str,
    pub version: u32,
    pub exported_at: String,
    pub app_version: &'a str,
    pub folders: Vec<InstallationFolder>,
    pub installations: Vec<ExportedInstallation>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ExportedInstallation {
    #[serde(flatten)]
    pub profile: InstallationProfile,
    #[serde(default)]
    pub needs_secret: NeedsSecret,
}

/// Итог экспорта для фронтенда.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExportSummary {
    pub installations: usize,
    pub folders: usize,
}

/// Сохранён ли непустой ключ API сервера `server` инсталляции `id`.
fn has_api_key(secrets: &dyn SecretStore, id: &str, server: &str) -> AppResult<bool> {
    Ok(secrets
        .get(&secret_key(id, server, API_KEY))?
        .is_some_and(|v| !v.is_empty()))
}

/// Серверы профиля, у которых сохранён ключ API: устаревшие записи keychain (сервер удалён)
/// не требуют ввода у получателя.
fn needs_secret(secrets: &dyn SecretStore, p: &InstallationProfile) -> AppResult<NeedsSecret> {
    let mut api_keys = Vec::new();
    for s in &p.servers {
        if has_api_key(secrets, &p.id, &s.id)? {
            api_keys.push(s.id.clone());
        }
    }
    Ok(NeedsSecret { api_keys })
}

/// Неизвестные поля профиля и серверов — локальные: в файл обмена и из него не переносятся.
fn clear_extra(p: &mut InstallationProfile) {
    p.extra.clear();
    for s in &mut p.servers {
        s.extra.clear();
    }
}

/// Приватный ключ из поля CA в файл обмена не попадает, сертификаты остаются (спец. §5.1:
/// секреты — только в keychain). Профили читаются с диска без повторной проверки (файл могли
/// править руками), поэтому ключ вырезается и здесь.
fn strip_pem_keys_for_export(p: &mut InstallationProfile) {
    for s in &mut p.servers {
        if let CaConfig::Pem { pem } = &mut s.ca {
            *pem = strip_private_keys(pem);
        }
    }
}

/// Собирает файл экспорта: всё дерево или поддерево `root` (сама папка становится корнем).
/// Секреты keychain не попадают в файл; неизвестные локальные поля (`extra`) не выносятся.
pub fn build_export(
    store: &InstallationStore,
    secrets: &dyn SecretStore,
    root: Option<&str>,
    app_version: &'static str,
    now: &str,
) -> AppResult<ExchangeFileOut<'static>> {
    let all_folders = store.list_folders();
    let included: Option<HashSet<String>> = match root {
        None => None,
        Some(root) => {
            if !all_folders.iter().any(|f| f.id == root) {
                return Err(folder_not_found(root));
            }
            let mut set = HashSet::from([root.to_string()]);
            // Дерево без циклов (инвариант хранилища), поэтому достаточно расширять до неподвижной точки.
            loop {
                let before = set.len();
                for f in &all_folders {
                    if f.parent_id.as_deref().is_some_and(|p| set.contains(p)) {
                        set.insert(f.id.clone());
                    }
                }
                if set.len() == before {
                    break;
                }
            }
            Some(set)
        }
    };
    let in_scope = |folder: Option<&str>| match &included {
        None => true,
        Some(set) => folder.is_some_and(|id| set.contains(id)),
    };

    let folders = all_folders
        .into_iter()
        .filter(|f| in_scope(Some(&f.id)))
        .map(|mut f| {
            if Some(f.id.as_str()) == root {
                f.parent_id = None;
            }
            f.extra.clear();
            f
        })
        .collect();

    let installations = store
        .list()
        .into_iter()
        .filter(|p| in_scope(p.folder_id.as_deref()))
        .map(|mut profile| {
            clear_extra(&mut profile);
            strip_pem_keys_for_export(&mut profile);
            let needs_secret = needs_secret(secrets, &profile)?;
            Ok(ExportedInstallation {
                profile,
                needs_secret,
            })
        })
        .collect::<AppResult<Vec<_>>>()?;

    Ok(ExchangeFileOut {
        format: EXCHANGE_FORMAT,
        version: EXCHANGE_VERSION,
        exported_at: now.to_string(),
        app_version,
        folders,
        installations,
    })
}

/// Инсталляция файла, которая не будет импортирована, и причина.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Skipped {
    pub name: String,
    pub reason: LocalizedText,
}

/// Прочитанный файл обмена: папки, инсталляции (с флагами секретов) и пропущенные инсталляции.
#[derive(Debug, Clone, Default)]
pub struct ParsedExchange {
    pub folders: Vec<InstallationFolder>,
    pub installations: Vec<(InstallationProfile, NeedsSecret)>,
    pub skipped: Vec<Skipped>,
}

fn not_glafira_file() -> crate::error::AppError {
    app_err!(
        ErrorCode::InvalidInput,
        "errors.import.notGlafiraFile",
        "This is not a Glafira installations file"
    )
}

fn invalid_json(error: impl std::fmt::Display) -> crate::error::AppError {
    app_err!(
        ErrorCode::InvalidInput,
        "errors.import.invalidJson",
        "The file is damaged or is not JSON: {{error}}",
        error = error
    )
}

/// Имя инсталляции для списка пропущенных: из файла, иначе id, иначе номер в списке.
fn display_name(v: &serde_json::Value, index: usize) -> String {
    ["name", "id"]
        .iter()
        .filter_map(|k| v.get(*k).and_then(|x| x.as_str()))
        .map(str::trim)
        .find(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_else(|| format!("#{}", index + 1))
}

/// Читает файл обмена. Ошибка — только для чужого или повреждённого файла и более новой версии;
/// неизвестные поля игнорируются, неподдерживаемая или невалидная инсталляция попадает в `skipped`.
pub fn parse_exchange(bytes: &[u8]) -> AppResult<ParsedExchange> {
    let root: serde_json::Value = serde_json::from_slice(bytes).map_err(invalid_json)?;
    if root.get("format").and_then(|f| f.as_str()) != Some(EXCHANGE_FORMAT) {
        return Err(not_glafira_file());
    }
    let version = root
        .get("version")
        .and_then(|v| v.as_u64())
        .ok_or_else(not_glafira_file)?;
    if version > u64::from(EXCHANGE_VERSION) {
        return Err(app_err!(
            ErrorCode::InvalidInput,
            "errors.import.incompatibleVersion",
            "The file was created by an incompatible version of Glafira — update the app"
        ));
    }
    let list = |key: &str| -> AppResult<Vec<serde_json::Value>> {
        match root.get(key) {
            None | Some(serde_json::Value::Null) => Ok(Vec::new()),
            Some(serde_json::Value::Array(items)) => Ok(items.clone()),
            Some(_) => Err(invalid_json(format!("“{key}” is not a list"))),
        }
    };

    let mut seen = HashSet::new();
    let mut folders = Vec::new();
    for v in list("folders")? {
        let mut f: InstallationFolder = serde_json::from_value(v).map_err(invalid_json)?;
        // Папка без id пропускается (её инсталляции попадут в цель); из повторов id берётся первая.
        if f.id.trim().is_empty() || !seen.insert(f.id.clone()) {
            continue;
        }
        f.extra.clear();
        let name = f.name.trim();
        f.name = if name.is_empty() { f.id.trim() } else { name }.to_string();
        folders.push(f);
    }

    let mut seen = HashSet::new();
    let mut installations = Vec::new();
    let mut skipped = Vec::new();
    for (i, v) in list("installations")?.into_iter().enumerate() {
        let name = display_name(&v, i);
        let mut c: ExportedInstallation = match serde_json::from_value(v) {
            Ok(c) => c,
            Err(e) => {
                skipped.push(Skipped {
                    name,
                    reason: ltext!(
                        "errors.import.unsupportedInstallation",
                        "Not supported by this version of Glafira (update the app): {{error}}",
                        error = e
                    ),
                });
                continue;
            }
        };
        // Неизвестные поля файла не переносятся в хранилище.
        clear_extra(&mut c.profile);
        // Как при сохранении в форме: имена и адреса без пробелов, адрес без «/» на конце.
        normalize_profile(&mut c.profile);
        if let Err(e) = validate_profile(&c.profile) {
            skipped.push(Skipped {
                name,
                reason: e.text(),
            });
            continue;
        }
        if !seen.insert(c.profile.id.clone()) {
            skipped.push(Skipped {
                name,
                reason: ltext!(
                    "errors.import.duplicateId",
                    "Duplicate installation id in the file"
                ),
            });
            continue;
        }
        installations.push((c.profile, c.needs_secret));
    }
    Ok(ParsedExchange {
        folders,
        installations,
        skipped,
    })
}

pub struct ImportOptions<'a> {
    /// Папка, под которую попадают корневые элементы файла; `None` — корень.
    pub target: Option<&'a str>,
    pub update_existing: bool,
    /// `false` — политика запрещает серверы без проверки сертификата (спец. §13).
    pub allow_insecure_tls: bool,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewItem {
    pub kind: TreeItemKind,
    pub id: String,
    pub name: String,
    /// Имена папок от корня до места элемента (сам элемент не входит).
    pub path: Vec<String>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub added: Vec<PreviewItem>,
    pub updated: Vec<PreviewItem>,
    pub unchanged: Vec<PreviewItem>,
    pub skipped: Vec<Skipped>,
    pub warnings: Vec<LocalizedText>,
    pub needs_secrets: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct ImportPlan {
    pub next: StoreData,
    pub preview: ImportPreview,
    /// Серверы обновлённых инсталляций, которых нет в файле: `(инсталляция, сервер)`. Их ключи
    /// API удаляются из keychain после записи.
    pub removed_servers: Vec<(String, String)>,
}

/// Имена папок от корня до `folder` включительно.
fn path_of(folders: &[InstallationFolder], folder: Option<&str>) -> Vec<String> {
    let mut path = Vec::new();
    let mut seen = HashSet::new();
    let mut cur = folder;
    while let Some(id) = cur {
        if !seen.insert(id) {
            break;
        }
        let Some(f) = folders.iter().find(|f| f.id == id) else {
            break;
        };
        path.push(f.name.clone());
        cur = f.parent_id.as_deref();
    }
    path.reverse();
    path
}

/// Папки файла в порядке «родитель раньше потомка» (внутри уровня — по `order`).
/// Папка с родителем не из файла — корневая в файле; цикл разрывается на первой оставшейся папке.
fn parents_first(folders: &[InstallationFolder]) -> Vec<(&InstallationFolder, bool)> {
    let ids: HashSet<&str> = folders.iter().map(|f| f.id.as_str()).collect();
    let mut rest: Vec<&InstallationFolder> = folders.iter().collect();
    rest.sort_by_key(|f| f.order);
    let mut done: HashSet<&str> = HashSet::new();
    let mut out = Vec::new();
    while !rest.is_empty() {
        let ready = |f: &&InstallationFolder| match f.parent_id.as_deref() {
            None => true,
            Some(p) => !ids.contains(p) || done.contains(p),
        };
        let (mut now, later): (Vec<_>, Vec<_>) = rest.into_iter().partition(ready);
        let mut forced_root = None;
        if now.is_empty() {
            // Цикл в файле: первая оставшаяся папка становится корневой.
            forced_root = Some(later[0].id.as_str());
            now.push(later[0]);
        }
        rest = later
            .into_iter()
            .filter(|f| Some(f.id.as_str()) != forced_root)
            .collect();
        for f in now {
            let root = Some(f.id.as_str()) == forced_root
                || f.parent_id.as_deref().is_none_or(|p| !ids.contains(p));
            out.push((f, root));
            done.insert(f.id.as_str());
        }
    }
    out
}

/// Настройки инсталляции без места в дереве и локальных неизвестных полей.
fn same_settings(local: &InstallationProfile, incoming: &InstallationProfile) -> bool {
    let strip = |p: &InstallationProfile| {
        let mut p = InstallationProfile {
            id: String::new(),
            folder_id: None,
            order: 0,
            ..p.clone()
        };
        clear_extra(&mut p);
        p
    };
    strip(local) == strip(incoming)
}

/// Меняется ли то, куда уходит ключ API сервера: адрес, прокси или проверка сертификата.
fn destination_changes(local: &ServerConfig, new: &ServerConfig) -> bool {
    local.base_url != new.base_url || local.proxy != new.proxy || local.ca != new.ca
}

/// Пары «сервер до импорта — сервер из файла» с тем же id, у которых меняется направление.
fn moved_servers<'a>(
    local: &'a InstallationProfile,
    new: &'a InstallationProfile,
) -> impl Iterator<Item = (&'a ServerConfig, &'a ServerConfig)> {
    new.servers.iter().filter_map(|n| {
        local
            .server(&n.id)
            .filter(|l| destination_changes(l, n))
            .map(|l| (l, n))
    })
}

/// Сохранены ли в keychain ключи API серверов, которые импорт обновит с новым направлением
/// (`destination_changes`). Читается до `InstallationStore::update`: запрос доступа к keychain
/// (на macOS — окно с паролем) не должен ждать под блокировкой записи хранилища, иначе ждали бы
/// и `list_installations` на главном потоке, и `installations.get`.
#[derive(Debug, Default)]
pub(crate) struct StoredSecrets {
    /// (id инсталляции, id сервера) → есть ли сохранённый ключ API.
    by_server: HashMap<(String, String), bool>,
}

impl StoredSecrets {
    /// Читает только нужное: серверы инсталляций, которые есть и в `data`, и в файле, у которых
    /// направление в файле другое. Сбой keychain считается «есть»: лишнее предупреждение лучше
    /// пропущенного.
    pub(crate) fn read(
        secrets: &dyn SecretStore,
        data: &StoreData,
        parsed: &ParsedExchange,
    ) -> Self {
        let mut by_server = HashMap::new();
        for (incoming, _) in &parsed.installations {
            let Some(local) = data.installations.iter().find(|l| l.id == incoming.id) else {
                continue;
            };
            for (_, server) in moved_servers(local, incoming) {
                let stored = has_api_key(secrets, &incoming.id, &server.id).unwrap_or(true);
                by_server.insert((incoming.id.clone(), server.id.clone()), stored);
            }
        }
        Self { by_server }
    }

    /// Сохранён ли ключ сервера `server` инсталляции `id`. Не прочитанное заранее (хранилище
    /// изменилось между чтением и записью) считается сохранённым.
    fn has(&self, id: &str, server: &str) -> bool {
        self.by_server
            .get(&(id.to_string(), server.to_string()))
            .copied()
            .unwrap_or(true)
    }
}

/// Предупреждения об обновлении инсталляции, у сервера которой меняется то, куда уходит
/// сохранённый ключ API: адрес, прокси или проверка сертификата.
fn destination_warnings(
    stored: &StoredSecrets,
    local: &InstallationProfile,
    new: &InstallationProfile,
) -> Vec<LocalizedText> {
    let mut out = Vec::new();
    for (old, server) in moved_servers(local, new) {
        if !stored.has(&new.id, &server.id) {
            continue;
        }
        let (name, srv) = (&new.name, &server.name);
        if old.base_url != server.base_url {
            out.push(ltext!(
                "messages.import.addressChanged",
                "“{{name}}”, server “{{server}}”: the address changes from {{from}} to {{to}}. The saved API key will be sent to the new address.",
                name = name,
                server = srv,
                from = old.base_url,
                to = server.base_url
            ));
        }
        if old.proxy != server.proxy {
            out.push(ltext!(
                "messages.import.proxyChanged",
                "“{{name}}”, server “{{server}}”: the proxy settings change. The saved API key will go through the new proxy.",
                name = name,
                server = srv
            ));
        }
        if old.ca != server.ca {
            out.push(ltext!(
                "messages.import.tlsChanged",
                "“{{name}}”, server “{{server}}”: the certificate verification settings change. The saved API key will be used with the new settings.",
                name = name,
                server = srv
            ));
        }
    }
    out
}

/// Строит новое состояние хранилища и предпросмотр импорта. Функция чистая: разрешение
/// политики приходит в `opts`, а сохранённые секреты — в `stored` (`StoredSecrets::read`, до
/// блокировки хранилища), для предупреждений об обновлении сервера с ключом API.
pub(crate) fn plan_import(
    data: &StoreData,
    parsed: &ParsedExchange,
    opts: &ImportOptions,
    stored: &StoredSecrets,
) -> AppResult<ImportPlan> {
    let target = opts.target.map(str::to_string);
    if let Some(t) = &target
        && !data.folders.iter().any(|f| &f.id == t)
    {
        return Err(folder_not_found(t));
    }
    let mut next = data.clone();
    let mut preview = ImportPreview {
        skipped: parsed.skipped.clone(),
        ..Default::default()
    };
    let mut removed_servers = Vec::new();
    // id папок файла совпадают с id в хранилище: существующие берутся как есть, новые создаются с тем же id.
    let file_folders: HashSet<&str> = parsed.folders.iter().map(|f| f.id.as_str()).collect();
    let local_folders: HashSet<&str> = data.folders.iter().map(|f| f.id.as_str()).collect();
    // (вид, id) в порядке появления, чтобы пути в предпросмотре считались по итоговому дереву.
    let mut added = Vec::new();
    let mut updated = Vec::new();
    let mut unchanged = Vec::new();

    for (f, is_root) in parents_first(&parsed.folders) {
        if local_folders.contains(f.id.as_str()) {
            unchanged.push((TreeItemKind::Folder, f.id.clone()));
            continue;
        }
        let parent = if is_root {
            target.clone()
        } else {
            f.parent_id.clone()
        };
        let siblings: Vec<&str> = next
            .folders
            .iter()
            .filter(|s| s.parent_id == parent)
            .map(|s| s.name.as_str())
            .collect();
        let name = unique_name(&siblings, &f.name);
        let order = next
            .folders
            .iter()
            .filter(|s| s.parent_id == parent)
            .map(|s| s.order + 1)
            .max()
            .unwrap_or(0);
        next.folders.push(InstallationFolder {
            id: f.id.clone(),
            name,
            parent_id: parent,
            order,
            extra: Default::default(),
        });
        added.push((TreeItemKind::Folder, f.id.clone()));
    }

    let mut installations: Vec<&(InstallationProfile, NeedsSecret)> =
        parsed.installations.iter().collect();
    installations.sort_by_key(|(p, _)| p.order);
    for (incoming, needs) in installations {
        if !opts.allow_insecure_tls && incoming.servers.iter().any(|s| s.ca == CaConfig::Insecure) {
            preview.skipped.push(Skipped {
                name: incoming.name.clone(),
                reason: ltext!(
                    "errors.import.insecureForbidden",
                    "Servers without TLS certificate verification are forbidden by your organization"
                ),
            });
            continue;
        }
        let mut profile = incoming.clone();
        if let Some(slot) = next.installations.iter_mut().find(|c| c.id == profile.id) {
            if same_settings(slot, incoming) || !opts.update_existing {
                unchanged.push((TreeItemKind::Installation, profile.id.clone()));
                continue;
            }
            // Место в дереве и неизвестные поля — локальные; ключи API получателя не трогаются.
            profile.folder_id = slot.folder_id.clone();
            profile.order = slot.order;
            profile.extra = std::mem::take(&mut slot.extra);
            for server in &mut profile.servers {
                if let Some(old) = slot.server(&server.id) {
                    server.extra = old.extra.clone();
                }
            }
            removed_servers.extend(
                slot.servers
                    .iter()
                    .filter(|old| profile.server(&old.id).is_none())
                    .map(|old| (profile.id.clone(), old.id.clone())),
            );
            // Адрес, прокси и CA берутся из файла, а ключи API остаются: файл из чужих рук
            // отправил бы их на другой сервер, и человек должен это увидеть.
            preview
                .warnings
                .extend(destination_warnings(stored, slot, &profile));
            updated.push((TreeItemKind::Installation, profile.id.clone()));
            *slot = profile;
            continue;
        }
        let folder = profile
            .folder_id
            .as_deref()
            .filter(|f| file_folders.contains(f))
            .map(str::to_string)
            .or_else(|| target.clone());
        let siblings: Vec<&str> = next
            .installations
            .iter()
            .filter(|c| c.folder_id == folder)
            .map(|c| c.name.as_str())
            .collect();
        let wanted = profile.name.trim();
        let taken = siblings
            .iter()
            .any(|s| s.trim().to_lowercase() == wanted.to_lowercase());
        let name = if taken {
            unique_name(&siblings, wanted)
        } else {
            profile.name.clone()
        };
        profile.order = next
            .installations
            .iter()
            .filter(|c| c.folder_id == folder)
            .map(|c| c.order + 1)
            .max()
            .unwrap_or(0);
        profile.folder_id = folder;
        profile.name = name;
        clear_extra(&mut profile);
        if !needs.api_keys.is_empty() {
            preview.needs_secrets += 1;
        }
        added.push((TreeItemKind::Installation, profile.id.clone()));
        next.installations.push(profile);
    }

    check_tree(&next)?;

    let folders_by_id: HashMap<&str, &InstallationFolder> =
        next.folders.iter().map(|f| (f.id.as_str(), f)).collect();
    let item = |(kind, id): (TreeItemKind, String)| -> PreviewItem {
        let (name, location) = match kind {
            TreeItemKind::Folder => {
                let f = folders_by_id[id.as_str()];
                (f.name.clone(), f.parent_id.as_deref())
            }
            TreeItemKind::Installation => {
                let c = next
                    .installations
                    .iter()
                    .find(|c| c.id == id)
                    .expect("planned installation exists");
                (c.name.clone(), c.folder_id.as_deref())
            }
        };
        PreviewItem {
            kind,
            path: path_of(&next.folders, location),
            id,
            name,
        }
    };
    preview.added = added.into_iter().map(item).collect();
    preview.updated = updated.into_iter().map(item).collect();
    preview.unchanged = unchanged.into_iter().map(item).collect();
    Ok(ImportPlan {
        next,
        preview,
        removed_servers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::MemoryStore;
    use crate::storage::installations::sample_profile;
    use crate::storage::profile::{TEST_CERT_PEM, pem_certificates, sample_server};

    fn store() -> (tempfile::TempDir, InstallationStore) {
        let d = tempfile::tempdir().unwrap();
        let s = InstallationStore::open(d.path()).unwrap();
        (d, s)
    }

    fn in_folder(id: &str, folder: Option<&str>) -> InstallationProfile {
        let mut p = sample_profile(id);
        p.folder_id = folder.map(String::from);
        p
    }

    /// ИФТ/{gpu-1}, ПСИ; инсталляции: a в ИФТ, b в gpu-1, c в ПСИ, d в корне.
    fn tree(s: &InstallationStore) -> (String, String, String) {
        let ift = s.create_folder("ИФТ", None).unwrap();
        let g1 = s.create_folder("gpu-1", Some(&ift.id)).unwrap();
        let psi = s.create_folder("ПСИ", None).unwrap();
        s.upsert(in_folder("a", Some(&ift.id))).unwrap();
        s.upsert(in_folder("b", Some(&g1.id))).unwrap();
        s.upsert(in_folder("c", Some(&psi.id))).unwrap();
        s.upsert(in_folder("d", None)).unwrap();
        (ift.id, g1.id, psi.id)
    }

    fn ids<T>(items: &[T], f: impl Fn(&T) -> &str) -> Vec<String> {
        let mut v: Vec<String> = items.iter().map(|i| f(i).to_string()).collect();
        v.sort();
        v
    }

    #[test]
    fn exports_whole_tree() {
        let (_d, s) = store();
        let (ift, g1, psi) = tree(&s);
        let out = build_export(
            &s,
            &MemoryStore::default(),
            None,
            "9.9.9",
            "2026-10-29T12:00:00Z",
        )
        .unwrap();
        assert_eq!(out.format, "glafira-inference-installations");
        assert_eq!(out.version, 1);
        assert_eq!(out.app_version, "9.9.9");
        assert_eq!(out.exported_at, "2026-10-29T12:00:00Z");
        let mut want = vec![ift, g1, psi];
        want.sort();
        assert_eq!(ids(&out.folders, |f| &f.id), want);
        assert_eq!(
            ids(&out.installations, |c| &c.profile.id),
            ["a", "b", "c", "d"]
        );

        let v = serde_json::to_value(&out).unwrap();
        assert_eq!(v["format"], "glafira-inference-installations");
        assert!(v["exportedAt"].is_string());
        assert!(v["installations"][0]["needsSecret"]["apiKeys"].is_array());
        assert!(v["installations"][0]["servers"][0]["baseUrl"].is_string());
    }

    #[test]
    fn exports_subtree_with_root_detached() {
        let (_d, s) = store();
        let (ift, g1, _psi) = tree(&s);
        let out = build_export(&s, &MemoryStore::default(), Some(&ift), "1", "now").unwrap();
        let mut want = vec![ift.clone(), g1.clone()];
        want.sort();
        assert_eq!(ids(&out.folders, |f| &f.id), want);
        let root = out.folders.iter().find(|f| f.id == ift).unwrap();
        assert_eq!(root.parent_id, None);
        let child = out.folders.iter().find(|f| f.id == g1).unwrap();
        assert_eq!(child.parent_id.as_deref(), Some(ift.as_str()));
        assert_eq!(ids(&out.installations, |c| &c.profile.id), ["a", "b"]);
    }

    #[test]
    fn api_keys_are_never_exported_but_flagged_per_server() {
        let (_d, s) = store();
        let mut two = sample_profile("two");
        two.servers.push(sample_server("s2"));
        s.upsert(two).unwrap();
        s.upsert(sample_profile("empty")).unwrap();
        let secrets = MemoryStore::default();
        secrets
            .set(&secret_key("two", "s2", API_KEY), "sk-s3cr3t")
            .unwrap();
        // Пустое значение и ключ удалённого сервера флаг не поднимают.
        secrets.set(&secret_key("two", "s1", API_KEY), "").unwrap();
        secrets
            .set(&secret_key("empty", "gone", API_KEY), "sk-old")
            .unwrap();
        let out = build_export(&s, &secrets, None, "1", "now").unwrap();
        let json = serde_json::to_string(&out).unwrap();
        assert!(
            !json.contains("s3cr3t") && !json.contains("sk-old"),
            "{json}"
        );
        let flags = |id: &str| {
            out.installations
                .iter()
                .find(|c| c.profile.id == id)
                .unwrap()
                .needs_secret
                .clone()
        };
        assert_eq!(flags("two").api_keys, ["s2"]);
        assert_eq!(flags("empty"), NeedsSecret::default());
    }

    #[test]
    fn unknown_local_fields_are_not_exported() {
        let (_d, s) = store();
        s.upsert(sample_profile("a")).unwrap();
        s.update(|data| {
            data.installations[0]
                .extra
                .insert("futureField".into(), serde_json::json!("local"));
            data.installations[0].servers[0]
                .extra
                .insert("futureServerField".into(), serde_json::json!("local"));
            Ok(())
        })
        .unwrap();
        let out = build_export(&s, &MemoryStore::default(), None, "1", "now").unwrap();
        assert!(out.installations[0].profile.extra.is_empty());
        let text = serde_json::to_string(&out).unwrap();
        assert!(!text.contains("futureField") && !text.contains("futureServerField"));
    }

    #[test]
    fn unknown_root_is_an_error() {
        let (_d, s) = store();
        tree(&s);
        let err = build_export(&s, &MemoryStore::default(), Some("nope"), "1", "now")
            .err()
            .unwrap();
        assert_eq!(err.key, "errors.folders.notFound");
    }

    // ---- импорт ----

    fn inst_json(id: &str, name: &str, folder: Option<&str>) -> serde_json::Value {
        let mut p = sample_profile(id);
        p.name = name.into();
        p.folder_id = folder.map(String::from);
        let mut v = serde_json::to_value(&p).unwrap();
        v["needsSecret"] = serde_json::json!({"apiKeys": []});
        v
    }

    fn folder_json(id: &str, name: &str, parent: Option<&str>, order: i64) -> serde_json::Value {
        serde_json::json!({"id": id, "name": name, "parentId": parent, "order": order})
    }

    fn file(folders: Vec<serde_json::Value>, insts: Vec<serde_json::Value>) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "format": "glafira-inference-installations",
            "version": 1,
            "exportedAt": "now",
            "appVersion": "1",
            "folders": folders,
            "installations": insts,
        }))
        .unwrap()
    }

    fn local_folder(id: &str, name: &str, parent: Option<&str>) -> InstallationFolder {
        InstallationFolder {
            id: id.into(),
            name: name.into(),
            parent_id: parent.map(String::from),
            order: 0,
            extra: Default::default(),
        }
    }

    fn opts(target: Option<&str>) -> ImportOptions<'_> {
        ImportOptions {
            target,
            update_existing: true,
            allow_insecure_tls: true,
        }
    }

    fn plan(data: &StoreData, bytes: &[u8], o: &ImportOptions) -> ImportPlan {
        let parsed = parse_exchange(bytes).unwrap();
        plan_with(data, &parsed, o, &MemoryStore::default()).unwrap()
    }

    /// План импорта, как в приложении: сохранённые секреты читаются до плана.
    fn plan_with(
        data: &StoreData,
        parsed: &ParsedExchange,
        o: &ImportOptions,
        secrets: &dyn SecretStore,
    ) -> AppResult<ImportPlan> {
        plan_import(data, parsed, o, &StoredSecrets::read(secrets, data, parsed))
    }

    fn inst<'a>(data: &'a StoreData, id: &str) -> &'a InstallationProfile {
        data.installations.iter().find(|c| c.id == id).unwrap()
    }

    fn folder<'a>(data: &'a StoreData, id: &str) -> &'a InstallationFolder {
        data.folders.iter().find(|f| f.id == id).unwrap()
    }

    fn preview_ids(items: &[PreviewItem]) -> Vec<&str> {
        items.iter().map(|i| i.id.as_str()).collect()
    }

    #[test]
    fn new_folders_and_installations_go_under_target_keeping_structure() {
        let data = StoreData {
            folders: vec![local_folder("t", "Target", None)],
            ..Default::default()
        };
        let bytes = file(
            vec![
                folder_json("g1", "gpu-1", Some("ift"), 0),
                folder_json("ift", "ИФТ", None, 0),
            ],
            vec![
                inst_json("a", "A", Some("ift")),
                inst_json("b", "B", Some("g1")),
                inst_json("c", "C", None),
            ],
        );
        let p = plan(&data, &bytes, &opts(Some("t")));
        let next = &p.next;
        assert_eq!(folder(next, "ift").parent_id.as_deref(), Some("t"));
        assert_eq!(folder(next, "g1").parent_id.as_deref(), Some("ift"));
        assert_eq!(inst(next, "a").folder_id.as_deref(), Some("ift"));
        assert_eq!(inst(next, "b").folder_id.as_deref(), Some("g1"));
        assert_eq!(inst(next, "c").folder_id.as_deref(), Some("t"));
        assert_eq!(preview_ids(&p.preview.added), ["ift", "g1", "a", "b", "c"]);
        let b = p.preview.added.iter().find(|i| i.id == "b").unwrap();
        assert_eq!(b.kind, TreeItemKind::Installation);
        assert_eq!(b.path, ["Target", "ИФТ", "gpu-1"]);
        assert!(p.preview.updated.is_empty() && p.preview.unchanged.is_empty());
        assert!(p.removed_servers.is_empty());
        check_tree(next).unwrap();
    }

    #[test]
    fn unknown_target_is_an_error() {
        let parsed = parse_exchange(&file(vec![], vec![])).unwrap();
        let err = plan_with(
            &StoreData::default(),
            &parsed,
            &opts(Some("nope")),
            &MemoryStore::default(),
        )
        .err()
        .unwrap();
        assert_eq!(err.key, "errors.folders.notFound");
    }

    #[test]
    fn matched_installation_is_updated_only_with_update_existing() {
        let mut local = sample_profile("a");
        local.folder_id = Some("f".into());
        local.order = 7;
        local.extra.insert("pinned".into(), serde_json::json!(true));
        local.servers[0]
            .extra
            .insert("gpuNode".into(), serde_json::json!("n1"));
        let data = StoreData {
            folders: vec![local_folder("f", "F", None)],
            installations: vec![local.clone()],
            ..Default::default()
        };
        let mut changed = inst_json("a", "inst-a", None);
        changed["servers"][0]["baseUrl"] = "http://other:8000".into();
        let bytes = file(vec![], vec![changed]);

        let p = plan(&data, &bytes, &opts(None));
        let a = inst(&p.next, "a");
        assert_eq!(a.servers[0].base_url, "http://other:8000");
        assert_eq!(a.folder_id.as_deref(), Some("f"));
        assert_eq!(a.order, 7);
        assert_eq!(a.extra.get("pinned"), Some(&serde_json::json!(true)));
        assert_eq!(
            a.servers[0].extra["gpuNode"], "n1",
            "поле сервера — локальное"
        );
        assert_eq!(preview_ids(&p.preview.updated), ["a"]);
        assert_eq!(p.preview.updated[0].path, ["F"]);

        let p = plan(
            &data,
            &bytes,
            &ImportOptions {
                update_existing: false,
                ..opts(None)
            },
        );
        assert_eq!(inst(&p.next, "a"), &local);
        assert_eq!(preview_ids(&p.preview.unchanged), ["a"]);

        // Те же настройки — «без изменений» и при update_existing.
        let same = file(vec![], vec![inst_json("a", "inst-a", None)]);
        let p = plan(&data, &same, &opts(None));
        assert_eq!(preview_ids(&p.preview.unchanged), ["a"]);
        assert_eq!(inst(&p.next, "a"), &local);
    }

    #[test]
    fn servers_missing_from_the_file_are_reported_for_key_removal() {
        let mut local = sample_profile("a");
        local.servers.push(sample_server("s2"));
        let data = StoreData {
            installations: vec![local],
            ..Default::default()
        };
        let mut incoming = inst_json("a", "inst-a", None);
        incoming["servers"][0]["name"] = "renamed".into();
        let p = plan(&data, &file(vec![], vec![incoming]), &opts(None));
        assert_eq!(inst(&p.next, "a").servers.len(), 1);
        assert_eq!(p.removed_servers, [("a".to_string(), "s2".to_string())]);
    }

    /// Локальное хранилище с инсталляцией `a` и план импорта файла, где `a` изменена `edit`.
    fn plan_update(
        local: InstallationProfile,
        secrets: &dyn SecretStore,
        edit: impl FnOnce(&mut serde_json::Value),
    ) -> ImportPreview {
        let data = StoreData {
            installations: vec![local],
            ..Default::default()
        };
        let mut changed = inst_json("a", "inst-a", None);
        edit(&mut changed);
        let parsed = parse_exchange(&file(vec![], vec![changed])).unwrap();
        let p = plan_with(&data, &parsed, &opts(None), secrets).unwrap();
        assert_eq!(preview_ids(&p.preview.updated), ["a"]);
        p.preview
    }

    fn with_key() -> MemoryStore {
        let secrets = MemoryStore::default();
        secrets
            .set(&secret_key("a", "s1", API_KEY), "sk-t")
            .unwrap();
        secrets
    }

    fn warning_keys(preview: &ImportPreview) -> Vec<&str> {
        preview.warnings.iter().map(|w| w.key.as_str()).collect()
    }

    #[test]
    fn address_change_with_a_stored_api_key_warns() {
        let p = plan_update(sample_profile("a"), &with_key(), |c| {
            c["servers"][0]["baseUrl"] = "http://other:8000".into();
        });
        assert_eq!(warning_keys(&p), ["messages.import.addressChanged"]);
        let w = &p.warnings[0];
        assert_eq!(w.params["name"], "inst-a");
        assert_eq!(w.params["server"], "vllm-s1");
        assert_eq!(w.params["from"], "https://s1.dev.example.com:8000");
        assert_eq!(w.params["to"], "http://other:8000");
    }

    /// Хранилище секретов, которое не читается (keychain заблокирован или отказал) и
    /// записывает, какие ключи у него спрашивали.
    #[derive(Default)]
    struct FailingStore {
        asked: std::sync::Mutex<Vec<String>>,
    }

    impl SecretStore for FailingStore {
        fn get(&self, key: &str) -> AppResult<Option<String>> {
            self.asked.lock().unwrap().push(key.to_string());
            Err(crate::error::AppError::internal_with("keychain is locked"))
        }
        fn set(&self, _: &str, _: &str) -> AppResult<()> {
            Err(crate::error::AppError::internal_with("keychain is locked"))
        }
        fn delete(&self, _: &str) -> AppResult<()> {
            Err(crate::error::AppError::internal_with("keychain is locked"))
        }
    }

    /// Сбой keychain считается «ключ есть»: лишнее предупреждение лучше пропущенного.
    #[test]
    fn keychain_error_counts_as_a_stored_key() {
        let secrets = FailingStore::default();
        let p = plan_update(sample_profile("a"), &secrets, |c| {
            c["servers"][0]["baseUrl"] = "http://other:8000".into();
        });
        assert_eq!(warning_keys(&p), ["messages.import.addressChanged"]);
        assert_eq!(
            *secrets.asked.lock().unwrap(),
            [secret_key("a", "s1", API_KEY)]
        );
    }

    /// Ключи читаются только у серверов, которые обновятся с новым направлением; то, чего
    /// не прочитали заранее, считается сохранённым.
    #[test]
    fn stored_keys_are_read_only_where_a_warning_is_possible() {
        let mut two = sample_profile("a");
        two.servers.push(sample_server("s2"));
        let data = StoreData {
            installations: vec![two, sample_profile("same")],
            ..Default::default()
        };
        let mut a = inst_json("a", "inst-a", None);
        a["servers"] = serde_json::json!([
            serde_json::to_value(sample_server("s1")).unwrap(),
            {"id": "s2", "name": "vllm-s2", "baseUrl": "http://moved:8000"},
            {"id": "s3", "name": "vllm-s3", "baseUrl": "http://new:8000"},
        ]);
        let mut same = inst_json("same", "inst-same", None);
        same["readOnly"] = true.into();
        let parsed = parse_exchange(&file(vec![], vec![a, same])).unwrap();
        let secrets = FailingStore::default();
        let stored = StoredSecrets::read(&secrets, &data, &parsed);
        assert_eq!(
            *secrets.asked.lock().unwrap(),
            [secret_key("a", "s2", API_KEY)]
        );
        assert!(stored.has("a", "s2"), "сбой keychain — «есть»");
        assert!(
            stored.has("a", "not-read"),
            "не прочитанное заранее — «есть»"
        );
        let p = plan_import(&data, &parsed, &opts(None), &stored).unwrap();
        assert_eq!(preview_ids(&p.preview.updated), ["a", "same"]);
        assert_eq!(warning_keys(&p.preview), ["messages.import.addressChanged"]);
        assert_eq!(p.preview.warnings[0].params["server"], "vllm-s2");
    }

    #[test]
    fn address_change_without_a_stored_key_does_not_warn() {
        let p = plan_update(sample_profile("a"), &MemoryStore::default(), |c| {
            c["servers"][0]["baseUrl"] = "http://other:8000".into();
        });
        assert!(p.warnings.is_empty());
        // Ключ другого сервера не в счёт.
        let secrets = MemoryStore::default();
        secrets
            .set(&secret_key("a", "s9", API_KEY), "sk-t")
            .unwrap();
        let p = plan_update(sample_profile("a"), &secrets, |c| {
            c["servers"][0]["baseUrl"] = "http://other:8000".into();
        });
        assert!(p.warnings.is_empty());
    }

    #[test]
    fn unchanged_address_does_not_warn() {
        let p = plan_update(sample_profile("a"), &with_key(), |c| {
            c["environment"] = "prod".into();
            c["readOnly"] = true.into();
        });
        assert!(p.warnings.is_empty());
        // Адрес с «/» на конце тот же адрес.
        let p = plan_update(sample_profile("a"), &with_key(), |c| {
            c["servers"][0]["baseUrl"] = "https://s1.dev.example.com:8000/".into();
            c["environment"] = "prod".into();
        });
        assert!(p.warnings.is_empty());
    }

    #[test]
    fn proxy_and_certificate_changes_warn_separately() {
        let p = plan_update(sample_profile("a"), &with_key(), |c| {
            c["servers"][0]["proxy"] =
                serde_json::json!({"mode": "url", "url": "http://proxy.other:3128"});
        });
        assert_eq!(warning_keys(&p), ["messages.import.proxyChanged"]);
        assert!(
            !p.warnings[0].text.contains("proxy.other"),
            "адрес прокси не печатается"
        );
        let p = plan_update(sample_profile("a"), &with_key(), |c| {
            c["servers"][0]["ca"] = serde_json::json!({"mode": "pem", "pem": TEST_CERT_PEM});
        });
        assert_eq!(warning_keys(&p), ["messages.import.tlsChanged"]);
        let p = plan_update(sample_profile("a"), &with_key(), |c| {
            c["servers"][0]["baseUrl"] = "http://other:8000".into();
            c["servers"][0]["proxy"] = serde_json::json!({"mode": "none"});
            c["servers"][0]["ca"] = serde_json::json!({"mode": "insecure"});
        });
        assert_eq!(
            warning_keys(&p),
            [
                "messages.import.addressChanged",
                "messages.import.proxyChanged",
                "messages.import.tlsChanged"
            ]
        );
    }

    #[test]
    fn added_installations_and_disabled_updates_do_not_warn() {
        let mut c = inst_json("a", "inst-a", None);
        c["servers"][0]["baseUrl"] = "http://other:8000".into();
        let bytes = file(vec![], vec![c.clone()]);
        let parsed = parse_exchange(&bytes).unwrap();
        let p = plan_with(&StoreData::default(), &parsed, &opts(None), &with_key()).unwrap();
        assert_eq!(preview_ids(&p.preview.added), ["a"]);
        assert!(p.preview.warnings.is_empty());
        let data = StoreData {
            installations: vec![sample_profile("a")],
            ..Default::default()
        };
        let off = ImportOptions {
            update_existing: false,
            ..opts(None)
        };
        let p = plan_with(&data, &parsed, &off, &with_key()).unwrap();
        assert_eq!(preview_ids(&p.preview.unchanged), ["a"]);
        assert!(p.preview.warnings.is_empty());
    }

    #[test]
    fn matched_folder_is_not_renamed_or_moved_and_receives_new_installations() {
        let data = StoreData {
            folders: vec![
                local_folder("p", "Parent", None),
                local_folder("f", "Local name", Some("p")),
            ],
            ..Default::default()
        };
        let bytes = file(
            vec![folder_json("f", "Renamed", None, 0)],
            vec![inst_json("n", "New", Some("f"))],
        );
        let p = plan(&data, &bytes, &opts(None));
        let f = folder(&p.next, "f");
        assert_eq!(f.name, "Local name");
        assert_eq!(f.parent_id.as_deref(), Some("p"));
        assert_eq!(inst(&p.next, "n").folder_id.as_deref(), Some("f"));
        assert_eq!(preview_ids(&p.preview.unchanged), ["f"]);
        assert_eq!(p.preview.added[0].path, ["Parent", "Local name"]);
    }

    #[test]
    fn name_conflicts_get_suffixes() {
        let mut x = sample_profile("x");
        x.name = "Qwen".into();
        let mut y = sample_profile("y");
        y.name = "qwen (2)".into();
        let data = StoreData {
            folders: vec![local_folder("l1", "ИФТ", None)],
            installations: vec![x, y],
            ..Default::default()
        };
        let bytes = file(
            vec![folder_json("n1", "ифт", None, 0)],
            vec![inst_json("n", "Qwen", None)],
        );
        let p = plan(&data, &bytes, &opts(None));
        assert_eq!(folder(&p.next, "n1").name, "ифт (2)");
        assert_eq!(inst(&p.next, "n").name, "Qwen (3)");
    }

    #[test]
    fn foreign_local_installations_are_untouched() {
        let mut mine = sample_profile("mine");
        mine.order = 3;
        let data = StoreData {
            installations: vec![mine.clone()],
            ..Default::default()
        };
        let p = plan(
            &data,
            &file(vec![], vec![inst_json("a", "A", None)]),
            &opts(None),
        );
        assert_eq!(inst(&p.next, "mine"), &mine);
        assert_eq!(inst(&p.next, "a").order, 4);
    }

    #[test]
    fn prod_read_only_comes_from_the_file() {
        let mut prod = inst_json("p", "P", None);
        prod["environment"] = "prod".into();
        let bytes = file(vec![], vec![prod]);
        let p = plan(&StoreData::default(), &bytes, &opts(None));
        assert!(!inst(&p.next, "p").read_only);
        assert!(p.preview.warnings.is_empty());
    }

    #[test]
    fn duplicate_installation_ids_are_skipped_with_reason() {
        let bytes = file(
            vec![],
            vec![
                inst_json("a", "First", None),
                inst_json("a", "Second", None),
            ],
        );
        let parsed = parse_exchange(&bytes).unwrap();
        assert_eq!(parsed.installations.len(), 1);
        assert_eq!(parsed.installations[0].0.name, "First");
        assert_eq!(parsed.skipped[0].name, "Second");
        assert_eq!(parsed.skipped[0].reason.key, "errors.import.duplicateId");
    }

    #[test]
    fn insecure_servers_are_skipped_when_policy_forbids() {
        let mut insecure = inst_json("i", "Insecure", None);
        insecure["servers"][0]["ca"] = serde_json::json!({"mode": "insecure"});
        let bytes = file(vec![], vec![insecure, inst_json("ok", "OK", None)]);
        let parsed = parse_exchange(&bytes).unwrap();
        let p = plan_with(
            &StoreData::default(),
            &parsed,
            &opts(None),
            &MemoryStore::default(),
        )
        .unwrap();
        assert_eq!(preview_ids(&p.preview.added), ["i", "ok"]);
        let forbid = ImportOptions {
            allow_insecure_tls: false,
            ..opts(None)
        };
        let p = plan_with(
            &StoreData::default(),
            &parsed,
            &forbid,
            &MemoryStore::default(),
        )
        .unwrap();
        assert_eq!(preview_ids(&p.preview.added), ["ok"]);
        assert_eq!(p.preview.skipped[0].name, "Insecure");
        assert_eq!(
            p.preview.skipped[0].reason.key,
            "errors.import.insecureForbidden"
        );
    }

    #[test]
    fn needs_secrets_counts_new_installations_only() {
        let mut a = inst_json("a", "A", None);
        a["needsSecret"]["apiKeys"] = serde_json::json!(["s1"]);
        let mut b = inst_json("b", "B", None);
        b["needsSecret"]["apiKeys"] = serde_json::json!(["s1"]);
        b["servers"][0]["baseUrl"] = "http://changed:8000".into();
        let data = StoreData {
            installations: vec![sample_profile("b")],
            ..Default::default()
        };
        let p = plan(&data, &file(vec![], vec![a, b]), &opts(None));
        assert_eq!(preview_ids(&p.preview.updated), ["b"]);
        assert_eq!(p.preview.needs_secrets, 1);
    }

    #[test]
    fn parse_exchange_tolerates_unknown_fields_and_skips_unsupported() {
        let mut ok = inst_json("ok", "OK", None);
        ok["futureField"] = serde_json::json!({"x": 1});
        ok["servers"][0]["futureServerField"] = 1.into();
        let mut future = inst_json("future", "Future", None);
        future["environment"] = "qa".into();
        let mut invalid = inst_json("bad", "Bad", None);
        invalid["servers"][0]["baseUrl"] = "ftp://h:21".into();
        let mut v: serde_json::Value =
            serde_json::from_slice(&file(vec![], vec![ok, future, invalid])).unwrap();
        v["somethingNew"] = serde_json::json!([1, 2]);
        let parsed = parse_exchange(&serde_json::to_vec(&v).unwrap()).unwrap();
        assert_eq!(parsed.installations.len(), 1);
        assert_eq!(parsed.installations[0].0.id, "ok");
        assert!(parsed.installations[0].0.extra.is_empty());
        assert!(parsed.installations[0].0.servers[0].extra.is_empty());
        assert_eq!(parsed.skipped.len(), 2);
        assert_eq!(parsed.skipped[0].name, "Future");
        assert_eq!(
            parsed.skipped[0].reason.key,
            "errors.import.unsupportedInstallation"
        );
        assert_eq!(parsed.skipped[1].name, "Bad");
        assert_eq!(
            parsed.skipped[1].reason.key,
            "errors.installations.badServerUrl"
        );

        v["version"] = 2.into();
        let err = parse_exchange(&serde_json::to_vec(&v).unwrap())
            .err()
            .unwrap();
        assert_eq!(err.key, "errors.import.incompatibleVersion");
        v["version"] = 1.into();
        v["format"] = "glafira-kubernetes-connections".into();
        let err = parse_exchange(&serde_json::to_vec(&v).unwrap())
            .err()
            .unwrap();
        assert_eq!(err.key, "errors.import.notGlafiraFile");
        let err = parse_exchange(b"{not json").err().unwrap();
        assert_eq!(err.key, "errors.import.invalidJson");
        let err = parse_exchange(b"[1, 2]").err().unwrap();
        assert_eq!(err.key, "errors.import.notGlafiraFile");
    }

    /// Импортируемый профиль приводится к каноническому виду до проверки, как при сохранении
    /// в форме.
    #[test]
    fn imported_profiles_are_normalized_before_validation() {
        let mut c = inst_json("n", "N", None);
        c["servers"][0]["baseUrl"] = " http://gpu-1:8000/ ".into();
        c["servers"][0]["metricsPath"] = "stats".into();
        let parsed = parse_exchange(&file(vec![], vec![c])).unwrap();
        assert!(parsed.skipped.is_empty(), "{:?}", parsed.skipped);
        let s = &parsed.installations[0].0.servers[0];
        assert_eq!(s.base_url, "http://gpu-1:8000");
        assert_eq!(s.metrics_path, "/stats");
    }

    #[test]
    fn export_then_import_into_empty_store_round_trips() {
        let (_d, s) = store();
        let (_ift, _g1, psi) = tree(&s);
        let mut x = sample_profile("x");
        x.folder_id = Some(psi.clone());
        s.upsert(x).unwrap();
        let secrets = MemoryStore::default();
        secrets
            .set(&secret_key("x", "s1", API_KEY), "sk-s3cr3t")
            .unwrap();
        let out = build_export(&s, &secrets, None, "1", "now").unwrap();
        let bytes = serde_json::to_vec(&out).unwrap();

        let parsed = parse_exchange(&bytes).unwrap();
        assert!(parsed.skipped.is_empty());
        let p = plan_with(
            &StoreData::default(),
            &parsed,
            &opts(None),
            &MemoryStore::default(),
        )
        .unwrap();
        assert_eq!(p.preview.needs_secrets, 1);
        let sort_f = |mut v: Vec<InstallationFolder>| {
            v.sort_by(|a, b| a.id.cmp(&b.id));
            v
        };
        let sort_c = |mut v: Vec<InstallationProfile>| {
            v.sort_by(|a, b| a.id.cmp(&b.id));
            v
        };
        assert_eq!(sort_f(p.next.folders.clone()), sort_f(s.list_folders()));
        assert_eq!(sort_c(p.next.installations.clone()), sort_c(s.list()));
    }

    #[test]
    fn private_keys_in_stored_ca_fields_never_reach_the_export() {
        let key = include_str!("testdata/test-key.pem");
        let mut p = sample_profile("k");
        p.servers[0].ca = CaConfig::Pem {
            pem: format!("{key}\n{TEST_CERT_PEM}"),
        };
        // Профиль, записанный в обход проверки (файл правили руками).
        let dir = tempfile::tempdir().unwrap();
        let stored = serde_json::json!({
            "schemaVersion": 1,
            "folders": [],
            "installations": [serde_json::to_value(&p).unwrap()],
        });
        std::fs::write(
            dir.path().join("installations.json"),
            serde_json::to_vec(&stored).unwrap(),
        )
        .unwrap();
        let s = InstallationStore::open(dir.path()).unwrap();
        assert_eq!(s.get("k").unwrap().servers[0].ca, p.servers[0].ca);

        let out = build_export(&s, &MemoryStore::default(), None, "1", "now").unwrap();
        let text = serde_json::to_string(&out).unwrap();
        assert!(!text.contains("PRIVATE KEY"), "{text}");
        let CaConfig::Pem { pem } = &out.installations[0].profile.servers[0].ca else {
            panic!("pem")
        };
        assert_eq!(
            pem_certificates(pem).unwrap(),
            pem_certificates(TEST_CERT_PEM).unwrap()
        );
        let parsed = parse_exchange(&serde_json::to_vec(&out).unwrap()).unwrap();
        assert!(parsed.skipped.is_empty());
    }
}
