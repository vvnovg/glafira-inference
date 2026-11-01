//! Клиент источника истории (спека I1c §4): HTTP API Prometheus и VictoriaMetrics поверх механики
//! `ServerClient` — CA, прокси, таймауты инсталляции, лимит ответа, без перенаправлений. Токен или
//! пароль источника не попадает ни в логи, ни в ошибки.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::history::window::Range;
use crate::policy::PolicyStatus;
use crate::servers::client::{HttpReply, METRICS_LIMIT, ServerClient, Timeouts};
use crate::storage::profile::{
    HISTORY_NAME, HistoryAuth, HistorySource, ServerConfig, ServerKind, parse_history_url,
};

/// Сколько символов текста ошибки источника показывается (спека I1c §3).
pub const ERROR_TEXT_LIMIT: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Prometheus,
    Victoriametrics,
}

/// Сведения `buildinfo`: тип источника и версия.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildInfo {
    pub kind: SourceKind,
    pub version: Option<String>,
}

/// Значение мгновенного запроса: метки и значение (`NaN`, `±Inf` — `None`).
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub labels: BTreeMap<String, String>,
    pub value: Option<f64>,
}

/// Ряд `query_range`: метки и точки (секунды Unix, значение).
#[derive(Debug, Clone, PartialEq)]
pub struct RangeSeries {
    pub labels: BTreeMap<String, String>,
    pub points: Vec<(u64, Option<f64>)>,
}

pub struct HistoryClient {
    http: ServerClient,
}

/// Секрет не печатается: клиент может попасть в лог.
impl std::fmt::Debug for HistoryClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HistoryClient")
            .field("http", &self.http)
            .finish()
    }
}

fn history_err(key_and_text: crate::i18n::LocalizedText, code: ErrorCode) -> AppError {
    AppError::localized(code, key_and_text)
}

fn unexpected(path: &str) -> AppError {
    history_err(
        ltext!(
            "errors.history.unexpected",
            "Unexpected answer of the metrics history source to {{path}}: not the Prometheus HTTP API",
            path = path
        ),
        ErrorCode::Internal,
    )
}

/// Ошибка соединения общего HTTP-клиента (`errors.server.*`) говорит «сервер»; у источника истории —
/// свой текст с теми же кодом, параметрами и подробностями. Остальные ошибки — как есть.
pub fn as_source_error(e: AppError) -> AppError {
    let p = |k: &str| e.params.get(k).cloned().unwrap_or_default();
    let text = match e.key.as_str() {
        "errors.server.dns" => ltext!(
            "errors.historySource.dns",
            "Cannot resolve the host name of the metrics history source"
        ),
        "errors.server.connect" => ltext!(
            "errors.historySource.connect",
            "Cannot connect to the metrics history source"
        ),
        "errors.server.timeout" => ltext!(
            "errors.historySource.timeout",
            "The metrics history source did not respond in time"
        ),
        "errors.server.hostnameMismatch" => ltext!(
            "errors.historySource.hostnameMismatch",
            "The certificate of the metrics history source does not match its address — check the address"
        ),
        "errors.server.certExpired" => ltext!(
            "errors.historySource.certExpired",
            "The certificate of the metrics history source has expired or is not valid yet"
        ),
        "errors.server.caUntrusted" => ltext!(
            "errors.historySource.caUntrusted",
            "The certificate of the metrics history source is not signed by a trusted CA — specify the CA of the source"
        ),
        "errors.server.redirect" => ltext!(
            "errors.historySource.redirect",
            "The metrics history source redirects {{path}} to another address ({{status}}) — use the final address of the source",
            path = p("path"),
            status = p("status")
        ),
        _ => return e,
    };
    let details = e.details.clone();
    let mut out = AppError::localized(e.code, text);
    out.details = details;
    out
}

/// Текст ошибки источника, не длиннее `ERROR_TEXT_LIMIT` символов.
fn clip(text: &str) -> String {
    match text.char_indices().nth(ERROR_TEXT_LIMIT) {
        Some((i, _)) => format!("{}…", &text[..i]),
        None => text.to_string(),
    }
}

/// Поле `data` ответа API или ошибка по коду и телу (спека I1c §3).
pub fn parse_api(reply: &HttpReply, path: &str) -> AppResult<Value> {
    if reply.truncated {
        return Err(history_err(
            ltext!(
                "errors.history.tooLarge",
                "The answer of the metrics history source is larger than 8 MB: choose a shorter window"
            ),
            // Не внутренняя ошибка и не повод повторять: окно слишком велико для предела ответа.
            ErrorCode::InvalidInput,
        ));
    }
    match reply.status {
        401 => {
            return Err(history_err(
                ltext!(
                    "errors.history.unauthorized",
                    "The metrics history source rejected the sign-in (401): check the token or the user name and password"
                ),
                ErrorCode::AuthFailed,
            ));
        }
        403 => {
            return Err(history_err(
                ltext!(
                    "errors.history.forbidden",
                    "The metrics history source denied access (403): the account may not read metrics"
                ),
                ErrorCode::AuthFailed,
            ));
        }
        _ => {}
    }
    let body: Option<Value> = serde_json::from_slice(&reply.body).ok();
    let field = |k: &str| body.as_ref().and_then(|b| b.get(k)).and_then(Value::as_str);
    match field("status") {
        Some("success") if reply.ok() => body
            .and_then(|mut b| b.get_mut("data").map(Value::take))
            .ok_or_else(|| unexpected(path)),
        Some("error") => {
            let text = clip(field("error").unwrap_or_default());
            Err(match field("errorType") {
                Some("timeout") => history_err(
                    ltext!(
                        "errors.history.timeout",
                        "The metrics history source did not finish the query in time: choose a shorter window"
                    ),
                    ErrorCode::Timeout,
                ),
                Some("bad_data") if text.contains("maximum resolution") => history_err(
                    ltext!(
                        "errors.history.tooManyPoints",
                        "The metrics history source refused the query: too many points per series"
                    ),
                    ErrorCode::InvalidInput,
                ),
                Some("bad_data") => history_err(
                    ltext!(
                        "errors.history.badData",
                        "The metrics history source rejected the query: {{message}}",
                        message = text
                    ),
                    ErrorCode::InvalidInput,
                ),
                _ => history_err(
                    ltext!(
                        "errors.history.failed",
                        "The metrics history source failed the query: {{message}}",
                        message = text
                    ),
                    ErrorCode::ConnectionFailed,
                ),
            })
        }
        _ if !reply.ok() => Err(history_err(
            ltext!(
                "errors.history.status",
                "The metrics history source answered {{path}} with {{status}}",
                path = path,
                status = reply.status
            ),
            ErrorCode::ConnectionFailed,
        )),
        _ => Err(unexpected(path)),
    }
}

/// Значение точки: строка числа; `NaN` и `±Inf` — отсутствие значения.
fn value(v: &Value) -> Option<f64> {
    v.as_str()?.parse::<f64>().ok().filter(|x| x.is_finite())
}

fn labels(v: &Value) -> BTreeMap<String, String> {
    v.get("metric")
        .and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

fn result<'a>(data: &'a Value, kind: &str, path: &str) -> AppResult<&'a Vec<Value>> {
    if data.get("resultType").and_then(Value::as_str) != Some(kind) {
        return Err(unexpected(path));
    }
    data.get("result")
        .and_then(Value::as_array)
        .ok_or_else(|| unexpected(path))
}

/// `data` мгновенного запроса (`resultType: vector`).
pub fn parse_vector(data: &Value) -> AppResult<Vec<Sample>> {
    let path = "/api/v1/query";
    result(data, "vector", path)?
        .iter()
        .map(|s| {
            let v = s.get("value").and_then(Value::as_array);
            Ok(Sample {
                labels: labels(s),
                value: v.and_then(|v| v.get(1)).and_then(value),
            })
        })
        .collect()
}

/// `data` запроса за окно (`resultType: matrix`).
pub fn parse_matrix(data: &Value) -> AppResult<Vec<RangeSeries>> {
    let path = "/api/v1/query_range";
    result(data, "matrix", path)?
        .iter()
        .map(|s| {
            let points = s
                .get("values")
                .and_then(Value::as_array)
                .ok_or_else(|| unexpected(path))?
                .iter()
                .map(|p| {
                    let t = p
                        .get(0)
                        .and_then(Value::as_f64)
                        .ok_or_else(|| unexpected(path))?;
                    Ok((t.round() as u64, p.get(1).and_then(value)))
                })
                .collect::<AppResult<Vec<_>>>()?;
            Ok(RangeSeries {
                labels: labels(s),
                points,
            })
        })
        .collect()
}

/// Длительность Go/Prometheus (`5s`, `1m`, `1m30s`, `1h`) в секундах; дробные и `ms` — `None`.
pub fn parse_duration(s: &str) -> Option<u64> {
    let mut total = 0u64;
    let mut num = String::new();
    let mut any = false;
    for c in s.chars() {
        if c.is_ascii_digit() {
            num.push(c);
            continue;
        }
        let n: u64 = num.parse().ok()?;
        num.clear();
        total += n * match c {
            's' => 1,
            'm' => 60,
            'h' => 3_600,
            'd' => 86_400,
            _ => return None,
        };
        any = true;
    }
    (any && num.is_empty()).then_some(total)
}

/// `scrapeInterval` цели `job`, `instance` из `data` ответа `/api/v1/targets`.
pub fn scrape_interval_of(data: &Value, job: &str, instance: &str) -> Option<u64> {
    data.get("activeTargets")?
        .as_array()?
        .iter()
        .find(|t| {
            let l = t.get("labels");
            let get = |k: &str| l.and_then(|l| l.get(k)).and_then(Value::as_str);
            get("job") == Some(job) && get("instance") == Some(instance)
        })?
        .get("scrapeInterval")?
        .as_str()
        .and_then(parse_duration)
}

/// Пробный запрос, по ответу которого VictoriaMetrics отличают от Prometheus.
pub const PROBE: &str = "vector(1)";

/// В ответе мгновенного запроса есть `stats.seriesFetched` — так отвечает VictoriaMetrics.
pub fn has_query_stats(body: &[u8]) -> bool {
    serde_json::from_slice::<Value>(body)
        .ok()
        .is_some_and(|b| b.pointer("/stats/seriesFetched").is_some())
}

/// Версия из строки `vm_app_version{version="1.155.0", …}` текста `/metrics` VictoriaMetrics.
pub fn vm_version(metrics: &str) -> Option<String> {
    let line = metrics.lines().find(|l| l.starts_with("vm_app_version{"))?;
    let rest = &line[line.find("version=\"")? + "version=\"".len()..];
    let v = &rest[..rest.find('"')?];
    (!v.is_empty()).then(|| v.to_string())
}

/// Источник истории как адрес с настройками соединения: так его открывают общий HTTP-клиент и
/// шаг «DNS/TCP» проверки.
pub fn as_server(h: &HistorySource) -> ServerConfig {
    ServerConfig {
        id: crate::secrets::HISTORY.into(),
        name: HISTORY_NAME.into(),
        base_url: h.url.clone(),
        kind: ServerKind::Auto,
        metrics_path: "/metrics".into(),
        ca: h.ca.clone(),
        proxy: h.proxy.clone(),
        history_selector: None,
        gpus: None,
        extra: Default::default(),
    }
}

impl HistoryClient {
    /// Клиент источника `h`; `secret` — токен или пароль из формы или keychain.
    pub fn new(
        h: &HistorySource,
        timeouts: Timeouts,
        secret: Option<String>,
        policy: &PolicyStatus,
    ) -> AppResult<Self> {
        parse_history_url(h)?;
        let server = as_server(h);
        let secret = secret.filter(|s| !s.is_empty());
        let http = match (&h.auth, secret) {
            (HistoryAuth::Bearer, token) => ServerClient::new(&server, timeouts, token, policy)?,
            (HistoryAuth::Basic { username }, Some(password)) => {
                ServerClient::new(&server, timeouts, None, policy)?
                    .with_basic_auth(username, password)
            }
            _ => ServerClient::new(&server, timeouts, None, policy)?,
        };
        Ok(Self { http })
    }

    async fn api(&self, path: &str, query: &[(&str, &str)]) -> AppResult<Value> {
        let reply = self
            .http
            .get_query(path, query, METRICS_LIMIT)
            .await
            .map_err(as_source_error)?;
        parse_api(&reply, path)
    }

    /// Тип и версия источника; `None` — у источника нет `buildinfo` (404). VictoriaMetrics отдаёт
    /// в `buildinfo` версию совместимого API (`2.24.0`), поэтому её узнают по полю
    /// `stats.seriesFetched` в ответе пробного запроса, а версию берут из `vm_app_version` в
    /// `/metrics` (`/metrics` недоступен — без версии).
    pub async fn buildinfo(&self) -> AppResult<Option<BuildInfo>> {
        let path = "/api/v1/status/buildinfo";
        let reply = self
            .http
            .get(path, METRICS_LIMIT)
            .await
            .map_err(as_source_error)?;
        if reply.status == 404 {
            return Ok(None);
        }
        let data = parse_api(&reply, path)?;
        let version = data
            .get("version")
            .and_then(Value::as_str)
            .map(String::from);
        let named = version
            .as_deref()
            .is_some_and(|v| v.to_lowercase().contains("victoria"));
        if !named && !self.answers_like_victoria().await {
            return Ok(Some(BuildInfo {
                kind: SourceKind::Prometheus,
                version,
            }));
        }
        let own = match self.http.get("/metrics", METRICS_LIMIT).await {
            Ok(r) if r.ok() => vm_version(&String::from_utf8_lossy(&r.body)),
            _ => None,
        };
        Ok(Some(BuildInfo {
            kind: SourceKind::Victoriametrics,
            version: own.or(version.filter(|_| named)),
        }))
    }

    /// Пробный запрос: в ответе VictoriaMetrics есть `stats.seriesFetched`; сбой — не она.
    async fn answers_like_victoria(&self) -> bool {
        match self
            .http
            .get_query("/api/v1/query", &[("query", PROBE)], METRICS_LIMIT)
            .await
        {
            Ok(r) if r.ok() => has_query_stats(&r.body),
            _ => false,
        }
    }

    /// Мгновенный запрос.
    pub async fn query(&self, expr: &str) -> AppResult<Vec<Sample>> {
        parse_vector(&self.api("/api/v1/query", &[("query", expr)]).await?)
    }

    /// Запрос за окно `range`.
    pub async fn query_range(&self, expr: &str, range: &Range) -> AppResult<Vec<RangeSeries>> {
        let (start, end, step) = (
            range.start.to_string(),
            range.end.to_string(),
            range.step.to_string(),
        );
        let data = self
            .api(
                "/api/v1/query_range",
                &[
                    ("query", expr),
                    ("start", &start),
                    ("end", &end),
                    ("step", &step),
                ],
            )
            .await?;
        parse_matrix(&data)
    }

    /// Интервал опроса цели; без `/api/v1/targets` или без цели — `None`.
    pub async fn scrape_interval(&self, job: &str, instance: &str) -> Option<u64> {
        let data = self
            .api("/api/v1/targets", &[("state", "active")])
            .await
            .ok()?;
        scrape_interval_of(&data, job, instance)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::servers::testing::{Reply, TestServer};
    use crate::storage::profile::{ProxyConfig, sample_history};

    pub(crate) fn snapshot(name: &str) -> String {
        std::fs::read_to_string(format!(
            "{}/src/history/testdata/prometheus-3.15.1/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    fn reply(status: u16, body: &str) -> HttpReply {
        HttpReply {
            status,
            content_type: Some("application/json".into()),
            body: body.as_bytes().to_vec(),
            truncated: false,
        }
    }

    const TIMEOUTS: Timeouts = Timeouts {
        connect_ms: 2_000,
        request_ms: 2_000,
    };

    pub(crate) fn source(url: &str, auth: HistoryAuth) -> HistorySource {
        HistorySource {
            url: url.into(),
            auth,
            proxy: ProxyConfig::None,
            ..sample_history()
        }
    }

    #[test]
    fn real_answers_are_parsed() {
        let up = parse_api(&reply(200, &snapshot("up.json")), "/api/v1/query").unwrap();
        let up = parse_vector(&up).unwrap();
        assert_eq!(up.len(), 4);
        assert!(up.iter().all(|s| s.value == Some(1.0)));
        let discover = parse_api(&reply(200, &snapshot("discover.json")), "/api/v1/query").unwrap();
        let mut found: Vec<_> = parse_vector(&discover)
            .unwrap()
            .into_iter()
            .map(|s| format!("{} {}", s.labels["job"], s.labels["instance"]))
            .collect();
        found.sort();
        assert_eq!(
            found,
            [
                "sglang 127.0.0.1:30000",
                "vllm 127.0.0.1:8000",
                "vllm 127.0.0.1:8001",
                "vllm 127.0.0.1:8003"
            ]
        );
        let empty = parse_api(&reply(200, &snapshot("empty.json")), "/api/v1/query").unwrap();
        assert!(parse_vector(&empty).unwrap().is_empty());
        let targets = parse_api(&reply(200, &snapshot("targets.json")), "/api/v1/targets").unwrap();
        assert_eq!(
            scrape_interval_of(&targets, "vllm", "127.0.0.1:8001"),
            Some(5)
        );
        assert_eq!(scrape_interval_of(&targets, "vllm", "127.0.0.1:9"), None);
    }

    #[test]
    fn real_range_answer_is_parsed_with_gaps() {
        let doc: Value = serde_json::from_str(&snapshot("range-vllm-8000.json")).unwrap();
        let first = &doc["queries"][0];
        let reply = reply(200, &first["response"].to_string());
        let series = parse_matrix(&parse_api(&reply, "/api/v1/query_range").unwrap()).unwrap();
        assert_eq!(series.len(), 1);
        let points = &series[0].points;
        let start = doc["start"].as_u64().unwrap();
        // Prometheus запущен позже начала окна: точек меньше 61, первая — не в начале окна.
        assert!(points.len() < 61 && points.len() > 40, "{}", points.len());
        assert!(points[0].0 > start);
        assert!(points.windows(2).all(|w| w[1].0 - w[0].0 == 15));
    }

    #[test]
    fn values_nan_and_infinity_are_missing() {
        let data = serde_json::json!({
            "resultType": "matrix",
            "result": [{ "metric": {}, "values": [[10, "1.5"], [25, "NaN"], [40, "+Inf"], [55, "-Inf"]] }]
        });
        let s = parse_matrix(&data).unwrap();
        assert_eq!(
            s[0].points,
            [(10, Some(1.5)), (25, None), (40, None), (55, None)]
        );
        assert!(s[0].labels.is_empty());
        let e =
            parse_matrix(&serde_json::json!({ "resultType": "vector", "result": [] })).unwrap_err();
        assert_eq!(e.key, "errors.history.unexpected");
    }

    #[test]
    fn errors_by_status_and_body() {
        let key = |r: HttpReply| parse_api(&r, "/api/v1/query").unwrap_err().key;
        assert_eq!(
            key(reply(400, &snapshot("bad-data.json"))),
            "errors.history.badData"
        );
        let e = parse_api(&reply(400, &snapshot("bad-data.json")), "/q").unwrap_err();
        assert!(e.params["message"].contains("unclosed left parenthesis"));
        assert_eq!(key(reply(401, "")), "errors.history.unauthorized");
        assert_eq!(key(reply(403, "{}")), "errors.history.forbidden");
        assert_eq!(
            key(reply(
                503,
                r#"{"status":"error","errorType":"timeout","error":"query timed out"}"#
            )),
            "errors.history.timeout"
        );
        assert_eq!(
            key(reply(
                400,
                r#"{"status":"error","errorType":"bad_data","error":"exceeded maximum resolution of 11,000 points per timeseries"}"#
            )),
            "errors.history.tooManyPoints"
        );
        assert_eq!(
            key(reply(
                422,
                r#"{"status":"error","errorType":"execution","error":"many-to-many matching not allowed"}"#
            )),
            "errors.history.failed"
        );
        assert_eq!(key(reply(502, "<html>")), "errors.history.status");
        assert_eq!(key(reply(200, "<html>")), "errors.history.unexpected");
        let mut big = reply(200, "{");
        big.truncated = true;
        let e = parse_api(&big, "/api/v1/query_range").unwrap_err();
        assert_eq!(
            (e.key.as_str(), e.code, e.retryable),
            ("errors.history.tooLarge", ErrorCode::InvalidInput, false)
        );
        let long = format!(
            r#"{{"status":"error","errorType":"bad_data","error":"{}"}}"#,
            "x".repeat(600)
        );
        let e = parse_api(&reply(400, &long), "/q").unwrap_err();
        assert_eq!(e.params["message"].chars().count(), ERROR_TEXT_LIMIT + 1);
    }

    #[test]
    fn durations() {
        assert_eq!(parse_duration("5s"), Some(5));
        assert_eq!(parse_duration("1m30s"), Some(90));
        assert_eq!(parse_duration("1h"), Some(3_600));
        for bad in ["", "5", "1.5s", "500ms", "x"] {
            assert_eq!(parse_duration(bad), None, "{bad}");
        }
    }

    #[tokio::test]
    async fn requests_carry_the_query_and_the_secret() {
        let server = TestServer::start(vec![
            (
                "/api/v1/status/buildinfo",
                Reply::json(&snapshot("buildinfo.json")),
            ),
            ("/api/v1/query", Reply::json(&snapshot("up.json"))),
            ("/api/v1/targets", Reply::json(&snapshot("targets.json"))),
        ])
        .await;
        let policy = PolicyStatus::default();
        let c = HistoryClient::new(
            &source(&server.url(), HistoryAuth::Bearer),
            TIMEOUTS,
            Some("prom-t0ken".into()),
            &policy,
        )
        .unwrap();
        assert!(!format!("{c:?}").contains("prom-t0ken"));
        let info = c.buildinfo().await.unwrap().unwrap();
        assert_eq!(
            (info.kind, info.version.as_deref()),
            (SourceKind::Prometheus, Some("3.15.1"))
        );
        assert_eq!(c.query("up").await.unwrap().len(), 4);
        assert_eq!(
            c.scrape_interval("sglang", "127.0.0.1:30000").await,
            Some(5)
        );
        let seen = server.seen.lock().unwrap().clone();
        // buildinfo, проба VictoriaMetrics, up.
        assert_eq!(seen[1].path, probe_path());
        assert_eq!(seen[2].path, "/api/v1/query?query=up");
        assert!(
            seen.iter()
                .all(|s| s.authorization.as_deref() == Some("Bearer prom-t0ken"))
        );

        let basic = HistoryClient::new(
            &source(
                &server.url(),
                HistoryAuth::Basic {
                    username: "reader".into(),
                },
            ),
            TIMEOUTS,
            Some("pa55".into()),
            &policy,
        )
        .unwrap();
        basic.query("up").await.unwrap();
        let last = server.seen.lock().unwrap().last().cloned().unwrap();
        // reader:pa55 в base64.
        assert_eq!(
            last.authorization.as_deref(),
            Some("Basic cmVhZGVyOnBhNTU=")
        );
    }

    fn vm_snapshot(name: &str) -> String {
        std::fs::read_to_string(format!(
            "{}/src/history/testdata/victoriametrics-1.155.0/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    pub(crate) fn probe_path() -> String {
        let q = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("query", PROBE)
            .finish();
        format!("/api/v1/query?{q}")
    }

    #[test]
    fn victoria_is_told_by_query_stats_and_metrics() {
        assert!(has_query_stats(vm_snapshot("probe.json").as_bytes()));
        assert!(!has_query_stats(snapshot("probe.json").as_bytes()));
        assert!(!has_query_stats(b"<html>"));
        assert_eq!(
            vm_version(&vm_snapshot("metrics.txt")).as_deref(),
            Some("1.155.0")
        );
        assert_eq!(vm_version("go_info{version=\"go1.27.3\"} 1"), None);
        assert_eq!(vm_version("vm_app_version{version=\"\"} 1"), None);
    }

    #[tokio::test]
    async fn victoriametrics_is_recognised_with_its_own_version() {
        let probe = probe_path();
        let routes = |metrics: bool| {
            let mut r = vec![
                (
                    "/api/v1/status/buildinfo",
                    Reply::json(&vm_snapshot("buildinfo.json")),
                ),
                (probe.as_str(), Reply::json(&vm_snapshot("probe.json"))),
            ];
            if metrics {
                r.push(("/metrics", Reply::text(&vm_snapshot("metrics.txt"))));
            }
            r
        };
        let info = |server: &TestServer| {
            let c = HistoryClient::new(
                &source(&server.url(), HistoryAuth::None),
                TIMEOUTS,
                None,
                &PolicyStatus::default(),
            )
            .unwrap();
            async move { c.buildinfo().await.unwrap().unwrap() }
        };
        let vm = TestServer::start(routes(true)).await;
        assert_eq!(
            info(&vm).await,
            BuildInfo {
                kind: SourceKind::Victoriametrics,
                version: Some("1.155.0".into())
            }
        );
        // /metrics закрыт или не по этому адресу (кластер за префиксом) — тип тот же, версии нет:
        // 2.24.0 из buildinfo — не версия VictoriaMetrics.
        let closed = TestServer::start(routes(false)).await;
        assert_eq!(
            info(&closed).await,
            BuildInfo {
                kind: SourceKind::Victoriametrics,
                version: None
            }
        );
        // Prometheus с настоящим ответом пробы — Prometheus, /metrics не спрашивается.
        let prom = TestServer::start(vec![
            (
                "/api/v1/status/buildinfo",
                Reply::json(&snapshot("buildinfo.json")),
            ),
            (probe.as_str(), Reply::json(&snapshot("probe.json"))),
        ])
        .await;
        assert_eq!(info(&prom).await.kind, SourceKind::Prometheus);
        assert!(!prom.paths().iter().any(|p| p == "/metrics"));
    }

    #[tokio::test]
    async fn connection_errors_name_the_source() {
        // Порт 1 закрыт — ошибка подключения общего клиента, с текстом про источник.
        let c = HistoryClient::new(
            &source("http://127.0.0.1:1", HistoryAuth::None),
            TIMEOUTS,
            None,
            &PolicyStatus::default(),
        )
        .unwrap();
        let e = c.query("up").await.unwrap_err();
        assert_eq!(
            (e.code, e.key.as_str()),
            (ErrorCode::ConnectionFailed, "errors.historySource.connect")
        );
        assert!(e.details.is_some());

        let timeout = as_source_error(crate::servers::errors::timeout().with_details("t"));
        assert_eq!(
            (
                timeout.code,
                timeout.key.as_str(),
                timeout.details.as_deref()
            ),
            (
                ErrorCode::Timeout,
                "errors.historySource.timeout",
                Some("t")
            )
        );
        let redirect = as_source_error(crate::servers::errors::from_status(302, "/api/v1/query"));
        assert_eq!(redirect.key, "errors.historySource.redirect");
        assert_eq!(
            redirect.message,
            "The metrics history source redirects /api/v1/query to another address (302) — use the final address of the source"
        );
        // Не ошибка соединения — без изменений.
        let other = as_source_error(unexpected("/q"));
        assert_eq!(other.key, "errors.history.unexpected");
    }

    #[tokio::test]
    async fn missing_buildinfo_is_not_an_error() {
        let server = TestServer::start(vec![]).await;
        let c = HistoryClient::new(
            &source(&server.url(), HistoryAuth::None),
            TIMEOUTS,
            None,
            &PolicyStatus::default(),
        )
        .unwrap();
        assert_eq!(c.buildinfo().await.unwrap(), None);
        assert_eq!(c.scrape_interval("vllm", "x:1").await, None);
    }
}
