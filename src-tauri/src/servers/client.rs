//! HTTP-клиент сервера инференса из профиля (спец. §5.2): CA, прокси, таймауты и ключ API. Только
//! GET; перенаправления не выполняются — ключ API не уходит на другой адрес.

use std::time::Duration;

use serde::de::DeserializeOwned;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::policy::PolicyStatus;
use crate::servers::errors::{chain_text, from_reqwest, from_status};
use crate::storage::profile::{
    CaConfig, InstallationProfile, ProxyConfig, ServerConfig, parse_base_url, pem_certificates,
    redact_url,
};

/// Сколько читается из ответа JSON: модели, версия, сведения о сервере.
pub const JSON_LIMIT: usize = 1024 * 1024;
/// Сколько читается из `/metrics`: у vLLM с десятком моделей LoRA это сотни килобайт.
pub const METRICS_LIMIT: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeouts {
    pub connect_ms: u64,
    pub request_ms: u64,
}

impl From<&InstallationProfile> for Timeouts {
    fn from(p: &InstallationProfile) -> Self {
        Self {
            connect_ms: p.connect_timeout_ms,
            request_ms: p.request_timeout_ms,
        }
    }
}

/// Размер тела в `Debug`: `<N bytes>`. В `/server_info` SGLang бывает ключ API, поэтому сырые
/// тела ответов в логи и сообщения об ошибках не попадают.
pub(crate) struct BodyLen(pub usize);

impl std::fmt::Debug for BodyLen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<{} bytes>", self.0)
    }
}

/// Ответ сервера: код, тело (не больше лимита запроса) и тип содержимого.
#[derive(Clone, PartialEq)]
pub struct HttpReply {
    pub status: u16,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
    /// Тело длиннее лимита: прочитано только начало.
    pub truncated: bool,
}

/// Тело не печатается, только его длина.
impl std::fmt::Debug for HttpReply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpReply")
            .field("status", &self.status)
            .field("content_type", &self.content_type)
            .field("body", &BodyLen(self.body.len()))
            .field("truncated", &self.truncated)
            .finish()
    }
}

impl HttpReply {
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

/// Сервер без проверки сертификата нельзя открыть, если политика это запрещает (спец. §13).
pub fn ensure_allowed(s: &ServerConfig, policy: &PolicyStatus) -> AppResult<()> {
    if s.ca == CaConfig::Insecure && !policy.allow_insecure_tls {
        return Err(app_err!(
            ErrorCode::InvalidInput,
            "errors.installations.insecureForbidden",
            "Servers without TLS certificate verification are forbidden by your organization"
        ));
    }
    Ok(())
}

pub struct ServerClient {
    http: reqwest::Client,
    base: url::Url,
    api_key: Option<String>,
}

/// Ключ API не печатается: структура может попасть в лог или сообщение об ошибке.
impl std::fmt::Debug for ServerClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerClient")
            .field("base", &self.base.as_str())
            .field("api_key", &self.api_key.as_ref().map(|_| "***"))
            .finish()
    }
}

impl ServerClient {
    /// Клиент сервера `s`. Пустой ключ API — то же, что его отсутствие.
    pub fn new(
        s: &ServerConfig,
        timeouts: Timeouts,
        api_key: Option<String>,
        policy: &PolicyStatus,
    ) -> AppResult<Self> {
        ensure_allowed(s, policy)?;
        let base = parse_base_url(s)?;
        crate::tls::ensure_crypto_provider();
        let mut b = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(timeouts.connect_ms))
            .timeout(Duration::from_millis(timeouts.request_ms))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("glafira-inference/", env!("CARGO_PKG_VERSION")));
        match &s.ca {
            CaConfig::System => {}
            CaConfig::Pem { pem } => {
                let bad_pem = || {
                    app_err!(
                        ErrorCode::InvalidInput,
                        "errors.installations.caPemInvalid",
                        "The CA certificate of server “{{server}}” must be in PEM format (-----BEGIN CERTIFICATE-----)",
                        server = s.name
                    )
                };
                let certs = pem_certificates(pem)
                    .map_err(|_| bad_pem())?
                    .iter()
                    .map(|der| reqwest::Certificate::from_der(der))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| bad_pem())?;
                b = b.tls_certs_only(certs);
            }
            CaConfig::Insecure => b = b.danger_accept_invalid_certs(true),
        }
        b = match &s.proxy {
            ProxyConfig::System => b,
            ProxyConfig::None => b.no_proxy(),
            ProxyConfig::Url { url } => b.proxy(reqwest::Proxy::all(url.trim()).map_err(|_| {
                app_err!(
                    ErrorCode::InvalidInput,
                    "errors.installations.badProxyUrl",
                    "Invalid proxy address “{{url}}”: use http://host:port",
                    url = redact_url(url)
                )
            })?),
        };
        let http = b
            .build()
            .map_err(|e| AppError::internal_with(format!("http client: {}", chain_text(&e))))?;
        Ok(Self {
            http,
            base,
            api_key: api_key.filter(|k| !k.trim().is_empty()),
        })
    }

    pub fn has_api_key(&self) -> bool {
        self.api_key.is_some()
    }

    /// Адрес запроса: путь базового адреса сервера + `path` (`https://gw/llm` + `/v1/models`).
    pub fn url(&self, path: &str) -> url::Url {
        let mut u = self.base.clone();
        let joined = format!(
            "{}/{}",
            u.path().trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        u.set_path(&joined);
        u
    }

    /// GET `path`: ответ с любым кодом, тело — не больше `limit` байт.
    pub async fn get(&self, path: &str, limit: usize) -> AppResult<HttpReply> {
        let mut req = self.http.get(self.url(path));
        if let Some(key) = &self.api_key {
            let mut value =
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", key.trim())).map_err(
                    |_| {
                        app_err!(
                            ErrorCode::InvalidInput,
                            "errors.installations.badApiKey",
                            "The API key contains characters that cannot be sent in a header"
                        )
                    },
                )?;
            value.set_sensitive(true);
            req = req.header(reqwest::header::AUTHORIZATION, value);
        }
        let mut resp = req.send().await.map_err(|e| from_reqwest(&e))?;
        let status = resp.status().as_u16();
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(String::from);
        let mut body = Vec::new();
        let mut truncated = false;
        while let Some(chunk) = resp.chunk().await.map_err(|e| from_reqwest(&e))? {
            let room = limit.saturating_sub(body.len());
            if chunk.len() > room {
                body.extend_from_slice(&chunk[..room]);
                truncated = true;
                break;
            }
            body.extend_from_slice(&chunk);
        }
        Ok(HttpReply {
            status,
            content_type,
            body,
            truncated,
        })
    }

    /// GET `path`, ответ только 2xx: иначе ошибка по коду (`from_status`).
    pub async fn get_ok(&self, path: &str, limit: usize) -> AppResult<HttpReply> {
        let reply = self.get(path, limit).await?;
        if !reply.ok() {
            return Err(from_status(reply.status, path));
        }
        Ok(reply)
    }

    /// GET `path` и разбор JSON. Ошибка разбора не цитирует тело ответа.
    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> AppResult<T> {
        let reply = self.get_ok(path, JSON_LIMIT).await?;
        parse_json(&reply.body, path)
    }
}

/// Разбор JSON ответа `path`; в тексте ошибки — только путь.
pub fn parse_json<T: DeserializeOwned>(body: &[u8], path: &str) -> AppResult<T> {
    serde_json::from_slice(body).map_err(|_| {
        app_err!(
            ErrorCode::Internal,
            "errors.server.badJson",
            "Unexpected answer to {{path}}: not the JSON this server type returns",
            path = path
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::servers::testing::{Reply, TEST_CA_PEM, TestServer};
    use crate::storage::profile::sample_server;

    const TIMEOUTS: Timeouts = Timeouts {
        connect_ms: 2_000,
        request_ms: 2_000,
    };

    fn server(url: &str) -> ServerConfig {
        let mut s = sample_server("s1");
        s.base_url = url.into();
        s.proxy = ProxyConfig::None;
        s
    }

    fn client(s: &ServerConfig, key: Option<&str>) -> ServerClient {
        ServerClient::new(s, TIMEOUTS, key.map(String::from), &PolicyStatus::default()).unwrap()
    }

    #[test]
    fn url_keeps_the_base_path() {
        let c = client(&server("https://gw.corp/llm/qwen"), None);
        assert_eq!(
            c.url("/v1/models").as_str(),
            "https://gw.corp/llm/qwen/v1/models"
        );
        let c = client(&server("http://10.0.0.5:8000"), None);
        assert_eq!(c.url("/metrics").as_str(), "http://10.0.0.5:8000/metrics");
        assert_eq!(c.url("health").as_str(), "http://10.0.0.5:8000/health");
    }

    #[tokio::test]
    async fn bearer_is_sent_only_with_a_key() {
        let srv = TestServer::start(vec![("/v1/models", Reply::json(r#"{"data":[]}"#))]).await;
        let with = client(&server(&srv.url()), Some(" sk-test "));
        let v: serde_json::Value = with.get_json("/v1/models").await.unwrap();
        assert_eq!(v["data"], serde_json::json!([]));
        client(&server(&srv.url()), Some(""))
            .get("/v1/models", JSON_LIMIT)
            .await
            .unwrap();
        let seen = srv.seen.lock().unwrap().clone();
        assert_eq!(seen[0].authorization.as_deref(), Some("Bearer sk-test"));
        assert_eq!(seen[1].authorization, None, "пустой ключ не отправляется");
        assert!(
            seen[0]
                .user_agent
                .as_deref()
                .is_some_and(|ua| ua.starts_with("glafira-inference/"))
        );
        assert!(!format!("{with:?}").contains("sk-test"), "Debug без ключа");
    }

    #[tokio::test]
    async fn statuses_and_limits() {
        let srv = TestServer::start(vec![
            ("/v1/models", Reply::status(401)),
            ("/metrics", Reply::text(&"x".repeat(100))),
            ("/bad", Reply::json("<html>")),
        ])
        .await;
        let c = client(&server(&srv.url()), None);
        let e = c.get_ok("/v1/models", JSON_LIMIT).await.unwrap_err();
        assert_eq!(
            (e.code, e.key.as_str()),
            (ErrorCode::AuthFailed, "errors.server.unauthorized")
        );
        let r = c.get("/v1/models", JSON_LIMIT).await.unwrap();
        assert_eq!(r.status, 401, "get отдаёт любой код");
        let r = c.get_ok("/metrics", 10).await.unwrap();
        assert_eq!((r.body.len(), r.truncated), (10, true));
        assert!(r.content_type.unwrap().starts_with("text/plain"));
        let r = c.get_ok("/metrics", 1000).await.unwrap();
        assert_eq!((r.body.len(), r.truncated), (100, false));
        let e = c.get_json::<serde_json::Value>("/bad").await.unwrap_err();
        assert_eq!(e.key, "errors.server.badJson");
        assert!(!serde_json::to_string(&e).unwrap().contains("html"));
        let e = c.get_ok("/nope", JSON_LIMIT).await.unwrap_err();
        assert_eq!(e.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn redirects_are_not_followed() {
        let srv = TestServer::start(vec![(
            "/v1/models",
            Reply::status(302).header("Location", "http://127.0.0.1:1/steal"),
        )])
        .await;
        let c = client(&server(&srv.url()), Some("sk-test"));
        let e = c.get_ok("/v1/models", JSON_LIMIT).await.unwrap_err();
        assert_eq!(e.key, "errors.server.redirect");
        assert_eq!(srv.paths(), ["/v1/models"]);
    }

    #[tokio::test]
    async fn slow_answer_is_a_timeout_and_closed_port_a_connection_error() {
        let srv = TestServer::start(vec![(
            "/health",
            Reply::status(200).delayed(Duration::from_secs(5)),
        )])
        .await;
        let quick = Timeouts {
            connect_ms: 2_000,
            request_ms: 1_000,
        };
        let c =
            ServerClient::new(&server(&srv.url()), quick, None, &PolicyStatus::default()).unwrap();
        let e = c.get("/health", JSON_LIMIT).await.unwrap_err();
        assert_eq!(e.code, ErrorCode::Timeout, "{:?}", e.details);
        // Порт 1 на localhost закрыт. С запасом: на Windows отказ приходит через ~2 с.
        let closed = Timeouts {
            connect_ms: 10_000,
            request_ms: 10_000,
        };
        let c = ServerClient::new(
            &server("http://127.0.0.1:1"),
            closed,
            None,
            &PolicyStatus::default(),
        )
        .unwrap();
        let e = c.get("/health", JSON_LIMIT).await.unwrap_err();
        assert_eq!(e.code, ErrorCode::ConnectionFailed, "{:?}", e.details);
    }

    #[tokio::test]
    async fn request_address_does_not_affect_classification_or_details() {
        // Текст ошибки reqwest содержит адрес запроса, а в пути могут быть слова, по которым
        // классификатор узнаёт TLS («tls», «handshake», «certificate»). Класс ошибки и `details`
        // от адреса зависеть не должны.
        const TRAP: &str = "/tls/handshake/certificate";
        let closed = Timeouts {
            connect_ms: 10_000,
            request_ms: 10_000,
        };
        let c = ServerClient::new(
            &server("http://127.0.0.1:1"),
            closed,
            None,
            &PolicyStatus::default(),
        )
        .unwrap();
        let e = c.get(TRAP, JSON_LIMIT).await.unwrap_err();
        assert_eq!(
            (e.code, e.key.as_str()),
            (ErrorCode::ConnectionFailed, "errors.server.connect"),
            "{:?}",
            e.details
        );
        let details = e.details.unwrap_or_default();
        assert!(
            !details.contains("127.0.0.1") && !details.contains("handshake"),
            "в details адрес запроса: {details}"
        );
        // Ветка таймаута: адрес не должен попасть в details и здесь.
        let srv = TestServer::start(vec![(
            TRAP,
            Reply::status(200).delayed(Duration::from_secs(5)),
        )])
        .await;
        let quick = Timeouts {
            connect_ms: 2_000,
            request_ms: 500,
        };
        let c =
            ServerClient::new(&server(&srv.url()), quick, None, &PolicyStatus::default()).unwrap();
        let e = c.get(TRAP, JSON_LIMIT).await.unwrap_err();
        assert_eq!(e.code, ErrorCode::Timeout, "{:?}", e.details);
        let details = e.details.unwrap_or_default();
        assert!(
            !details.contains("127.0.0.1") && !details.contains("handshake"),
            "в details адрес запроса: {details}"
        );
    }

    #[tokio::test]
    async fn tls_trusts_only_the_given_ca_or_nothing_in_insecure_mode() {
        let srv = TestServer::start_tls(vec![("/health", Reply::status(200))]).await;
        let mut s = server(&srv.url());
        // Системные корни тестовый CA не знают.
        let e = client(&s, None)
            .get("/health", JSON_LIMIT)
            .await
            .unwrap_err();
        assert_eq!(e.code, ErrorCode::TlsError, "{:?}", e.details);
        s.ca = CaConfig::Pem {
            pem: TEST_CA_PEM.into(),
        };
        let r = client(&s, None).get("/health", JSON_LIMIT).await.unwrap();
        assert_eq!(r.status, 200);
        // Чужой CA вместо нужного — снова ошибка TLS.
        s.ca = CaConfig::Pem {
            pem: crate::storage::profile::TEST_CERT_PEM.into(),
        };
        let e = client(&s, None)
            .get("/health", JSON_LIMIT)
            .await
            .unwrap_err();
        assert_eq!(e.code, ErrorCode::TlsError);
        s.ca = CaConfig::Insecure;
        let r = client(&s, None).get("/health", JSON_LIMIT).await.unwrap();
        assert_eq!(r.status, 200);
    }

    #[tokio::test]
    async fn https_to_a_plain_http_server_is_a_tls_error() {
        // Самая частая ошибка адреса: в профиле `https://`, а сервер отвечает по обычному HTTP.
        // Настоящая ошибка reqwest здесь — «received corrupt message of type InvalidContentType»,
        // слов «tls» и «certificate» в ней нет.
        let srv = TestServer::start(vec![("/health", Reply::status(200))]).await;
        let url = srv.url().replacen("http://", "https://", 1);
        let e = client(&server(&url), None)
            .get("/health", JSON_LIMIT)
            .await
            .unwrap_err();
        assert_eq!(
            (e.code, e.key.as_str()),
            (ErrorCode::TlsError, "errors.server.tlsError"),
            "{:?}",
            e.details
        );
    }

    #[test]
    fn policy_forbids_insecure_servers() {
        let mut s = server("https://vllm:8000");
        s.ca = CaConfig::Insecure;
        let policy = PolicyStatus {
            allow_insecure_tls: false,
            ..PolicyStatus::default()
        };
        let e = ServerClient::new(&s, TIMEOUTS, None, &policy).unwrap_err();
        assert_eq!(
            (e.code, e.key.as_str()),
            (
                ErrorCode::InvalidInput,
                "errors.installations.insecureForbidden"
            )
        );
        s.ca = CaConfig::System;
        assert!(ServerClient::new(&s, TIMEOUTS, None, &policy).is_ok());
    }

    #[test]
    fn bad_api_key_characters_are_an_input_error() {
        let c = client(&server("http://h:8000"), Some("bad\nkey"));
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let e = rt.block_on(c.get("/v1/models", JSON_LIMIT)).unwrap_err();
        assert_eq!(e.key, "errors.installations.badApiKey");
    }

    /// Тело ответа может содержать ключ API (`/server_info` SGLang): `Debug` его не печатает.
    #[test]
    fn debug_of_a_reply_hides_the_body() {
        let reply = HttpReply {
            status: 200,
            content_type: Some("application/json".into()),
            body: br#"{"api_key":"sk-secret"}"#.to_vec(),
            truncated: true,
        };
        let text = format!("{reply:?}");
        assert!(!text.contains("sk-secret"), "{text}");
        assert!(!text.contains("115, 107"), "байты тела тоже скрыты: {text}");
        for part in ["200", "application/json", "<23 bytes>", "true"] {
            assert!(text.contains(part), "{part} в {text}");
        }
        assert_eq!(reply.clone(), reply);
    }
}
