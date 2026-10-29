//! Профиль инсталляции инференса (спец. §5.1, §5.2): окружение, серверы и их настройки. Ключи API
//! серверов в профиле не хранятся, они лежат в keychain (`secrets::API_KEY`). Проверки PEM, адресов
//! и прокси — те же, что у профиля подключения Glafira for Kubernetes.

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult, ErrorCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Local,
    Dev,
    Test,
    Stage,
    Prod,
}

/// Тип сервера (спец. §4.3): `auto` — определить по ответам сервера при подключении.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServerKind {
    #[default]
    Auto,
    Vllm,
    Sglang,
    Openai,
}

/// Чему доверять при проверке сертификата сервера (только для `https://`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum CaConfig {
    /// Системное хранилище сертификатов ОС.
    System,
    /// Только перечисленные сертификаты (один или несколько PEM): системные корни не используются.
    Pem { pem: String },
    /// Без проверки сертификата (`insecure-skip-tls-verify`); может быть запрещено политикой.
    Insecure,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum ProxyConfig {
    /// Переменные окружения `HTTPS_PROXY` / `NO_PROXY`.
    System,
    None,
    Url {
        url: String,
    },
}

fn default_ca() -> CaConfig {
    CaConfig::System
}

fn default_proxy() -> ProxyConfig {
    ProxyConfig::System
}

fn default_metrics_path() -> String {
    "/metrics".into()
}

fn default_request_timeout() -> u64 {
    30_000
}

fn default_connect_timeout() -> u64 {
    10_000
}

/// Сколько серверов может быть в инсталляции: столько охватывает проверка (спец. §7.1).
pub const MAX_SERVERS: usize = 32;

/// Сервер инференса — компонент инсталляции (спец. §5.2). Реплики одной модели за балансировщиком
/// добавляются по отдельности.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    /// Идентификатор компонента: часть ключа keychain (`<инсталляция>/<сервер>/apiKey`).
    pub id: String,
    pub name: String,
    /// Базовый URL: `http://` или `https://`, хост и порт; допускается путь (обратный прокси).
    pub base_url: String,
    #[serde(default)]
    pub kind: ServerKind,
    #[serde(default = "default_metrics_path")]
    pub metrics_path: String,
    #[serde(default = "default_ca")]
    pub ca: CaConfig,
    #[serde(default = "default_proxy")]
    pub proxy: ProxyConfig,
    /// Поля из более новой версии Glafira: сохраняются при записи как есть.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallationProfile {
    pub id: String,
    pub name: String,
    pub color: String,
    pub environment: Environment,
    pub read_only: bool,
    #[serde(default)]
    pub servers: Vec<ServerConfig>,
    #[serde(default = "default_request_timeout")]
    pub request_timeout_ms: u64,
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_ms: u64,
    #[serde(default)]
    pub folder_id: Option<String>,
    #[serde(default)]
    pub order: i64,
    /// Поля из более новой версии Glafira: сохраняются при записи как есть.
    /// `InstallationStore::upsert` входящий `extra` не принимает: у существующего профиля
    /// остаётся сохранённый, новый получает пустой.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl InstallationProfile {
    pub fn server(&self, id: &str) -> Option<&ServerConfig> {
        self.servers.iter().find(|s| s.id == id)
    }
}

/// Приводит поля к каноническому виду до проверки: имена и адреса без пробелов по краям, адрес
/// без `/` на конце, путь метрик — с `/` в начале.
pub fn normalize_profile(p: &mut InstallationProfile) {
    p.name = p.name.trim().to_string();
    for s in &mut p.servers {
        s.name = s.name.trim().to_string();
        s.base_url = s.base_url.trim().trim_end_matches('/').to_string();
        let path = s.metrics_path.trim();
        s.metrics_path = if path.is_empty() {
            default_metrics_path()
        } else if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{path}")
        };
    }
}

/// Сертификаты из PEM-текста (DER). Ошибка — если текст не PEM или в нём нет ни одного сертификата.
pub fn pem_certificates(text: &str) -> Result<Vec<Vec<u8>>, String> {
    let blocks = pem::parse_many(text.as_bytes()).map_err(|e| e.to_string())?;
    let certs: Vec<Vec<u8>> = blocks
        .into_iter()
        .filter(|b| b.tag() == "CERTIFICATE")
        .map(|b| b.into_contents())
        .collect();
    if certs.is_empty() {
        return Err("no CERTIFICATE blocks".into());
    }
    Ok(certs)
}

/// Диапазоны PEM-текста, занятые блоками с приватным ключом: тег содержит `PRIVATE KEY`
/// (`PRIVATE KEY`, `RSA PRIVATE KEY`, `EC PRIVATE KEY`, `ENCRYPTED PRIVATE KEY`). Блок идёт от
/// `-----BEGIN` до конца `-----END …-----`, незавершённый — до конца текста. Разбор не зависит
/// от того, корректно ли тело блока: ключ находится и в повреждённом PEM.
fn private_key_ranges(text: &str) -> Vec<std::ops::Range<usize>> {
    const BEGIN: &str = "-----BEGIN ";
    const END: &str = "-----END ";
    const DASHES: &str = "-----";
    let mut ranges = Vec::new();
    let mut pos = 0;
    while let Some(found) = text[pos..].find(BEGIN) {
        let start = pos + found;
        let label_start = start + BEGIN.len();
        let Some(label_len) = text[label_start..].find(DASHES) else {
            break;
        };
        let body = label_start + label_len + DASHES.len();
        if !text[label_start..label_start + label_len].contains("PRIVATE KEY") {
            // Следующий блок ищем сразу за заголовком этого: так не пропускается ключ,
            // стоящий внутри незавершённого блока другого вида.
            pos = body;
            continue;
        }
        let end = text[body..]
            .find(END)
            .and_then(|e| {
                let tail = body + e + END.len();
                text[tail..].find(DASHES).map(|d| tail + d + DASHES.len())
            })
            .unwrap_or(text.len());
        ranges.push(start..end);
        pos = end;
    }
    ranges
}

/// В PEM-тексте есть блок с приватным ключом (секреты хранятся только в keychain, спец. §5.1).
pub fn pem_has_private_key(text: &str) -> bool {
    !private_key_ranges(text).is_empty()
}

/// PEM-текст без блоков с приватным ключом; остальное (сертификаты) остаётся как есть.
pub fn strip_private_keys(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;
    for range in private_key_ranges(text) {
        out.push_str(&text[pos..range.start]);
        pos = range.end;
    }
    out.push_str(&text[pos..]);
    out
}

/// Идентификатор сервера: латинские буквы, цифры, `-` и `_`, от 1 до 64 символов — он входит
/// в ключ keychain и в имена файлов.
pub fn is_valid_server_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn invalid(key_and_text: crate::i18n::LocalizedText) -> AppError {
    AppError::localized(ErrorCode::InvalidInput, key_and_text)
}

/// В адресе есть имя пользователя или пароль (`https://user:pass@host`).
fn has_userinfo(u: &url::Url) -> bool {
    !u.username().is_empty() || u.password().is_some()
}

/// Адрес без имени и пароля и без завершающего «/» — для текста ошибки.
pub(crate) fn without_userinfo(u: &url::Url) -> String {
    let mut clean = u.clone();
    // Ошибка возможна только для адреса без хоста; вызывающий код такие адреса сюда не передаёт.
    let _ = clean.set_username("");
    let _ = clean.set_password(None);
    clean.as_str().trim_end_matches('/').to_string()
}

/// Адрес для текста ошибки без имени и пароля: ни один адрес, который вводил человек или который
/// читается из профиля и окружения, не показывается как есть. Если адрес не разбирается как URL
/// с хостом (нет схемы, пробел в хосте, пустой хост), отбрасывается всё до последней «@».
pub(crate) fn redact_url(raw: &str) -> String {
    match url::Url::parse(raw) {
        Ok(u) if u.host().is_some() => without_userinfo(&u),
        _ => match raw.rsplit_once('@') {
            Some((head, tail)) => match head.split_once("://") {
                Some((scheme, _)) => format!("{scheme}://{tail}"),
                None => tail.to_string(),
            },
            None => raw.to_string(),
        },
    }
}

/// Ошибка «учётные данные в адресе»; в тексте адрес показан без имени и пароля.
fn credentials_in_url(u: &url::Url) -> AppError {
    invalid(ltext!(
        "errors.installations.credentialsInUrl",
        "Remove the user name and password from “{{url}}”: credentials do not belong in the address (a proxy with a password is set via the system HTTPS_PROXY)",
        url = without_userinfo(u)
    ))
}

/// Разобранный базовый URL сервера; ошибка называет сервер.
pub fn parse_base_url(s: &ServerConfig) -> AppResult<url::Url> {
    let bad = || {
        invalid(ltext!(
            "errors.installations.badServerUrl",
            "Invalid address “{{url}}” of server “{{server}}”: use http://host:port or https://host:port",
            url = redact_url(&s.base_url),
            server = s.name
        ))
    };
    let url = url::Url::parse(s.base_url.trim()).map_err(|_| bad())?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none_or(str::is_empty)
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(bad());
    }
    if has_userinfo(&url) {
        return Err(credentials_in_url(&url));
    }
    Ok(url)
}

/// Проверка одного сервера: и при сохранении профиля, и при проверке подключения из формы.
pub fn validate_server(s: &ServerConfig) -> AppResult<()> {
    if !is_valid_server_id(&s.id) {
        return Err(invalid(ltext!(
            "errors.installations.badServerId",
            "Server “{{server}}” has an invalid ID",
            server = s.name
        )));
    }
    if s.name.trim().is_empty() {
        return Err(invalid(ltext!(
            "errors.installations.emptyServerName",
            "Server name cannot be empty"
        )));
    }
    parse_base_url(s)?;
    let path = &s.metrics_path;
    if !path.starts_with('/') || path.contains(['?', '#']) || path.chars().any(char::is_whitespace)
    {
        return Err(invalid(ltext!(
            "errors.installations.badMetricsPath",
            "Invalid metrics path “{{path}}” of server “{{server}}”: use a path such as /metrics",
            path = path.escape_debug(),
            server = s.name
        )));
    }
    if let CaConfig::Pem { pem } = &s.ca {
        if pem_has_private_key(pem) {
            return Err(invalid(ltext!(
                "errors.installations.pemContainsKey",
                "A certificate field contains a private key — remove it: Glafira needs only the CA certificate"
            )));
        }
        if pem_certificates(pem).is_err() {
            return Err(invalid(ltext!(
                "errors.installations.caPemInvalid",
                "The CA certificate of server “{{server}}” must be in PEM format (-----BEGIN CERTIFICATE-----)",
                server = s.name
            )));
        }
    }
    if let ProxyConfig::Url { url } = &s.proxy {
        let proxy = url::Url::parse(url.trim())
            .ok()
            .filter(|u| matches!(u.scheme(), "http" | "https") && u.host_str().is_some());
        let Some(proxy) = proxy else {
            return Err(invalid(ltext!(
                "errors.installations.badProxyUrl",
                "Invalid proxy address “{{url}}”: use http://host:port",
                url = redact_url(url)
            )));
        };
        if has_userinfo(&proxy) {
            return Err(credentials_in_url(&proxy));
        }
    }
    Ok(())
}

pub fn validate_profile(p: &InstallationProfile) -> AppResult<()> {
    if p.id.trim().is_empty() {
        return Err(invalid(ltext!(
            "errors.installations.noId",
            "The installation has no ID"
        )));
    }
    if p.name.trim().is_empty() {
        return Err(invalid(ltext!(
            "errors.installations.emptyName",
            "Installation name cannot be empty"
        )));
    }
    if p.servers.is_empty() {
        return Err(invalid(ltext!(
            "errors.installations.noServers",
            "Add at least one inference server"
        )));
    }
    if p.servers.len() > MAX_SERVERS {
        return Err(invalid(ltext!(
            "errors.installations.tooManyServers",
            "An installation can have at most {{max}} servers",
            max = MAX_SERVERS
        )));
    }
    let mut ids = std::collections::HashSet::new();
    let mut names = std::collections::HashSet::new();
    for s in &p.servers {
        validate_server(s)?;
        if !ids.insert(s.id.as_str()) {
            return Err(invalid(ltext!(
                "errors.installations.duplicateServerId",
                "Two servers have the same ID “{{id}}”",
                id = s.id
            )));
        }
        if !names.insert(s.name.trim().to_lowercase()) {
            return Err(invalid(ltext!(
                "errors.installations.duplicateServerName",
                "Two servers are named “{{server}}”: give them different names",
                server = s.name.trim()
            )));
        }
    }
    if p.request_timeout_ms < 1_000 || p.connect_timeout_ms < 1_000 {
        return Err(invalid(ltext!(
            "errors.installations.timeoutTooSmall",
            "Timeouts must be at least 1000 ms"
        )));
    }
    Ok(())
}

#[cfg(test)]
pub fn sample_server(id: &str) -> ServerConfig {
    ServerConfig {
        id: id.to_string(),
        name: format!("vllm-{id}"),
        base_url: format!("https://{id}.dev.example.com:8000"),
        kind: ServerKind::Auto,
        metrics_path: "/metrics".into(),
        ca: CaConfig::System,
        proxy: ProxyConfig::System,
        extra: Default::default(),
    }
}

#[cfg(test)]
pub fn sample_profile(id: &str) -> InstallationProfile {
    InstallationProfile {
        id: id.to_string(),
        name: format!("inst-{id}"),
        color: "#4f8cff".into(),
        environment: Environment::Dev,
        read_only: false,
        servers: vec![sample_server("s1")],
        request_timeout_ms: 30_000,
        connect_timeout_ms: 10_000,
        folder_id: None,
        order: 0,
        extra: Default::default(),
    }
}

/// Самоподписанный сертификат для тестов (CN=glafira-test).
#[cfg(test)]
pub const TEST_CERT_PEM: &str = include_str!("testdata/test-cert.pem");

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_invalid(p: &InstallationProfile, key: &str) -> AppError {
        let e = validate_profile(p).unwrap_err();
        assert_eq!((e.code, e.key.as_str()), (ErrorCode::InvalidInput, key));
        e
    }

    fn cert_with_key() -> String {
        format!("{TEST_CERT_PEM}\n{}", include_str!("testdata/test-key.pem"))
    }

    #[test]
    fn json_shape_matches_frontend_contract() {
        let v = serde_json::to_value(sample_profile("a")).unwrap();
        assert_eq!(
            v,
            serde_json::json!({
                "id": "a",
                "name": "inst-a",
                "color": "#4f8cff",
                "environment": "dev",
                "readOnly": false,
                "servers": [{
                    "id": "s1",
                    "name": "vllm-s1",
                    "baseUrl": "https://s1.dev.example.com:8000",
                    "kind": "auto",
                    "metricsPath": "/metrics",
                    "ca": { "mode": "system" },
                    "proxy": { "mode": "system" }
                }],
                "requestTimeoutMs": 30000,
                "connectTimeoutMs": 10000,
                "folderId": null,
                "order": 0
            })
        );
    }

    #[test]
    fn minimal_json_gets_defaults_and_keeps_unknown_fields() {
        let p: InstallationProfile = serde_json::from_value(serde_json::json!({
            "id": "a", "name": "n", "color": "#fff", "environment": "prod", "readOnly": true,
            "servers": [{ "id": "s", "name": "x", "baseUrl": "http://h:8000", "future": 1 }],
            "slo": { "ttftP95Ms": 500 }
        }))
        .unwrap();
        let s = &p.servers[0];
        assert_eq!(s.kind, ServerKind::Auto);
        assert_eq!(s.metrics_path, "/metrics");
        assert_eq!((&s.ca, &s.proxy), (&CaConfig::System, &ProxyConfig::System));
        assert_eq!(
            (p.request_timeout_ms, p.connect_timeout_ms),
            (30_000, 10_000)
        );
        assert_eq!(s.extra["future"], 1, "поле сервера из новой версии");
        assert_eq!(
            p.extra["slo"]["ttftP95Ms"], 500,
            "поле профиля из новой версии"
        );
        let back = serde_json::to_value(&p).unwrap();
        assert_eq!(back["slo"]["ttftP95Ms"], 500);
        assert_eq!(back["servers"][0]["future"], 1);
    }

    #[test]
    fn normalization_trims_names_urls_and_metrics_path() {
        let mut p = sample_profile("a");
        p.name = "  prod LLM ".into();
        p.servers[0].name = " gpu-1 ".into();
        p.servers[0].base_url = " http://gpu-1:8000/v/ ".into();
        p.servers[0].metrics_path = " stats ".into();
        normalize_profile(&mut p);
        assert_eq!(p.name, "prod LLM");
        assert_eq!(p.servers[0].name, "gpu-1");
        assert_eq!(p.servers[0].base_url, "http://gpu-1:8000/v");
        assert_eq!(p.servers[0].metrics_path, "/stats");
        p.servers[0].metrics_path = "   ".into();
        normalize_profile(&mut p);
        assert_eq!(p.servers[0].metrics_path, "/metrics");
    }

    #[test]
    fn server_ids() {
        for ok in ["s1", "a-b_C", &"x".repeat(64)] {
            assert!(is_valid_server_id(ok), "{ok}");
        }
        for bad in ["", "a/b", "a b", "сервер", &"x".repeat(65), "../x"] {
            assert!(!is_valid_server_id(bad), "{bad}");
        }
    }

    #[test]
    fn validation_accepts_sample_and_http_and_rejects_bad_fields() {
        assert!(validate_profile(&sample_profile("a")).is_ok());
        let mut p = sample_profile("a");
        p.servers[0].base_url = "http://10.0.0.5:8000".into();
        assert!(
            validate_profile(&p).is_ok(),
            "http — частый случай в контуре"
        );
        p.servers[0].base_url = "https://gw.corp/llm/qwen".into();
        assert!(validate_profile(&p).is_ok(), "путь за обратным прокси");

        type Change = Box<dyn Fn(&mut InstallationProfile)>;
        let cases: Vec<(&str, Change)> = vec![
            ("errors.installations.noId", Box::new(|p| p.id = " ".into())),
            (
                "errors.installations.emptyName",
                Box::new(|p| p.name = "".into()),
            ),
            (
                "errors.installations.noServers",
                Box::new(|p| p.servers.clear()),
            ),
            (
                "errors.installations.tooManyServers",
                Box::new(|p| {
                    p.servers = (0..=MAX_SERVERS)
                        .map(|i| sample_server(&format!("s{i}")))
                        .collect()
                }),
            ),
            (
                "errors.installations.badServerId",
                Box::new(|p| p.servers[0].id = "a/b".into()),
            ),
            (
                "errors.installations.emptyServerName",
                Box::new(|p| p.servers[0].name = " ".into()),
            ),
            (
                "errors.installations.badServerUrl",
                Box::new(|p| p.servers[0].base_url = "ftp://h:21".into()),
            ),
            (
                "errors.installations.badServerUrl",
                Box::new(|p| p.servers[0].base_url = "vllm:8000".into()),
            ),
            (
                "errors.installations.badServerUrl",
                Box::new(|p| p.servers[0].base_url = "http://h:8000/?x=1".into()),
            ),
            (
                "errors.installations.badServerUrl",
                Box::new(|p| p.servers[0].base_url = "http://h:8000/#f".into()),
            ),
            (
                "errors.installations.badMetricsPath",
                Box::new(|p| p.servers[0].metrics_path = "metrics".into()),
            ),
            (
                "errors.installations.badMetricsPath",
                Box::new(|p| p.servers[0].metrics_path = "/metrics?x=1".into()),
            ),
            (
                "errors.installations.badMetricsPath",
                Box::new(|p| p.servers[0].metrics_path = "/a b".into()),
            ),
            (
                "errors.installations.caPemInvalid",
                Box::new(|p| p.servers[0].ca = CaConfig::Pem { pem: "nope".into() }),
            ),
            (
                "errors.installations.badProxyUrl",
                Box::new(|p| {
                    p.servers[0].proxy = ProxyConfig::Url {
                        url: "proxy:3128".into(),
                    }
                }),
            ),
            (
                "errors.installations.duplicateServerId",
                Box::new(|p| {
                    let mut s = sample_server("s1");
                    s.name = "other".into();
                    p.servers.push(s)
                }),
            ),
            (
                "errors.installations.duplicateServerName",
                Box::new(|p| {
                    let mut s = sample_server("s2");
                    s.name = " VLLM-S1 ".into();
                    p.servers.push(s)
                }),
            ),
            (
                "errors.installations.timeoutTooSmall",
                Box::new(|p| p.connect_timeout_ms = 999),
            ),
        ];
        for (key, change) in cases {
            let mut p = sample_profile("a");
            change(&mut p);
            assert_invalid(&p, key);
        }
    }

    #[test]
    fn server_errors_name_the_server() {
        let mut p = sample_profile("a");
        p.servers[0].base_url = "ftp://h".into();
        let e = assert_invalid(&p, "errors.installations.badServerUrl");
        assert_eq!(e.params["server"], "vllm-s1");
        assert_eq!(e.params["url"], "ftp://h");
    }

    #[test]
    fn private_key_in_the_ca_field_is_rejected() {
        for tag in [
            "PRIVATE KEY",
            "RSA PRIVATE KEY",
            "EC PRIVATE KEY",
            "ENCRYPTED PRIVATE KEY",
        ] {
            let mut p = sample_profile("a");
            p.servers[0].ca = CaConfig::Pem {
                pem: format!("{TEST_CERT_PEM}\n-----BEGIN {tag}-----\nAAAA\n-----END {tag}-----\n"),
            };
            assert_invalid(&p, "errors.installations.pemContainsKey");
        }
        let mut p = sample_profile("a");
        p.servers[0].ca = CaConfig::Pem {
            pem: cert_with_key(),
        };
        assert_invalid(&p, "errors.installations.pemContainsKey");
        p.servers[0].ca = CaConfig::Pem {
            pem: format!("{TEST_CERT_PEM}\n{TEST_CERT_PEM}"),
        };
        assert!(validate_profile(&p).is_ok());
    }

    #[test]
    fn credentials_in_urls_are_rejected_without_echoing_them() {
        let mut p = sample_profile("a");
        p.servers[0].base_url = "https://admin:s3cr3t@vllm:8000".into();
        let e = assert_invalid(&p, "errors.installations.credentialsInUrl");
        assert_eq!(e.params["url"], "https://vllm:8000");
        let mut p = sample_profile("a");
        p.servers[0].proxy = ProxyConfig::Url {
            url: "http://u:s3cr3t@proxy:3128".into(),
        };
        let e = assert_invalid(&p, "errors.installations.credentialsInUrl");
        assert_eq!(e.params["url"], "http://proxy:3128");
        for (url, shown) in [
            ("ftp://u:s3cr3t@vllm:8000", "ftp://vllm:8000"),
            ("u:s3cr3t@vllm:8000", "vllm:8000"),
            ("http://u:s3cr3t@vl lm:8000", "http://vl lm:8000"),
        ] {
            let mut p = sample_profile("a");
            p.servers[0].base_url = url.into();
            let e = assert_invalid(&p, "errors.installations.badServerUrl");
            assert_eq!(e.params["url"], shown, "{url}");
            let all = serde_json::to_string(&e).unwrap();
            assert!(!all.contains("s3cr3t"), "{url}: {all}");
        }
    }

    #[test]
    fn strip_private_keys_keeps_only_the_certificates() {
        let key = include_str!("testdata/test-key.pem");
        let body = key.lines().nth(1).unwrap();
        let want = pem_certificates(TEST_CERT_PEM).unwrap();
        for text in [
            cert_with_key(),
            format!("{key}\n{TEST_CERT_PEM}"),
            format!("{TEST_CERT_PEM}\n-----BEGIN EC PRIVATE KEY-----\nAAAA\n"),
            format!("{TEST_CERT_PEM}\n-----BEGIN RSA PRIVATE KEY-----\nAAAA\n-----END"),
        ] {
            assert!(pem_has_private_key(&text), "{text}");
            let stripped = strip_private_keys(&text);
            assert!(
                !stripped.contains("PRIVATE KEY") && !stripped.contains(body),
                "{stripped}"
            );
            assert_eq!(pem_certificates(&stripped).unwrap(), want, "{stripped}");
        }
        let plain = format!("# CA\n{TEST_CERT_PEM}\n{TEST_CERT_PEM}");
        assert!(!pem_has_private_key(&plain));
        assert_eq!(strip_private_keys(&plain), plain);
        let nested = format!("-----BEGIN CERTIFICATE-----\nAAAA\n{key}");
        assert!(pem_has_private_key(&nested));
        assert!(!strip_private_keys(&nested).contains(body));
    }

    #[test]
    fn profile_json_never_contains_secrets() {
        let v = serde_json::to_string(&sample_profile("a")).unwrap();
        assert!(!v.contains("apiKey") && !v.contains("token"), "{v}");
    }
}
