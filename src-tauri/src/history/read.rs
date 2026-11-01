//! Окно истории сервера (спека I1c §4): шаблоны по карте адаптера, не больше 4 запросов к
//! источнику одновременно, сборка точек; сбой одного запроса — `partial`, всех — ошибка. На всё
//! окно — один срок, таймаут запроса инсталляции.

use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use serde::Serialize;
use tokio::sync::Semaphore;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::history::client::{HistoryClient, SourceKind};
use crate::history::promql::queries;
use crate::history::series::{HistoryPoint, Partial, assemble};
use crate::history::targets::{MatchOp, parse_selector, render};
use crate::history::window::{Window, duration, range};
use crate::servers::adapters::adapter;
use crate::servers::detect::DetectedKind;

/// Сколько запросов к источнику идёт одновременно (спека I1c §4).
pub const CONCURRENCY: usize = 4;

/// Ответ `read_server_history`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryAnswer {
    pub window: Window,
    pub step_sec: u64,
    pub source: SourceKind,
    pub points: Vec<HistoryPoint>,
    pub partial: Vec<Partial>,
}

/// Открытый источник истории инсталляции: клиент и тип по `buildinfo`. Живёт в пуле подключений
/// до `evict` инсталляции. Предел одновременных запросов (`CONCURRENCY`) общий для всех окон
/// инсталляции, поэтому хранится здесь, а не в вызове. `deadline` — срок чтения одного окна:
/// у каждого запроса свой таймаут, и без общего срока окно из ~20 запросов волнами по 4 ждало бы
/// медленный источник впятеро дольше.
#[derive(Debug)]
pub struct Source {
    pub client: HistoryClient,
    pub kind: SourceKind,
    limit: Arc<Semaphore>,
    deadline: Duration,
}

impl Source {
    pub fn new(client: HistoryClient, kind: SourceKind, deadline: Duration) -> Self {
        Self {
            client,
            kind,
            limit: Arc::new(Semaphore::new(CONCURRENCY)),
            deadline,
        }
    }
}

/// Значение метки селектора с условием `=`: по `job` и `instance` ищется интервал опроса цели.
fn exact(matchers: &[crate::history::targets::Matcher], label: &str) -> Option<String> {
    matchers
        .iter()
        .find(|m| m.label == label && m.op == MatchOp::Eq)
        .map(|m| m.value.clone())
}

/// Разрешение на один запрос к источнику; держится, пока запрос идёт.
pub(crate) async fn permit(source: &Source) -> tokio::sync::OwnedSemaphorePermit {
    source
        .limit
        .clone()
        .acquire_owned()
        .await
        .expect("the semaphore is never closed")
}

/// Окно `window` сервера типа `kind` с селектором `selector`, которое кончается в `now_sec`.
/// Не уложилось в срок источника — ошибка таймаута, недоделанные запросы отменяются.
pub async fn read_history(
    source: &Source,
    kind: DetectedKind,
    selector: &str,
    window: Window,
    now_sec: u64,
) -> AppResult<HistoryAnswer> {
    let read = read_window(source, kind, selector, window, now_sec);
    match tokio::time::timeout(source.deadline, read).await {
        Ok(answer) => answer,
        Err(_) => Err(app_err!(
            ErrorCode::Timeout,
            "errors.history.deadline",
            "The metrics history source did not return the window in {{seconds}} s: choose a shorter window or raise the request timeout",
            seconds = source.deadline.as_secs()
        )),
    }
}

async fn read_window(
    source: &Source,
    kind: DetectedKind,
    selector: &str,
    window: Window,
    now_sec: u64,
) -> AppResult<HistoryAnswer> {
    let map = adapter(kind).metric_map();
    if map.is_empty() {
        return Err(app_err!(
            ErrorCode::InvalidInput,
            "errors.history.noMetrics",
            "Glafira reads the metrics history only of vLLM and SGLang"
        ));
    }
    let matchers = parse_selector(selector)?;
    let scrape = match (exact(&matchers, "job"), exact(&matchers, "instance")) {
        (Some(job), Some(instance)) => {
            let _permit = permit(source).await;
            source.client.scrape_interval(&job, &instance).await
        }
        _ => None,
    };
    let r = range(window, now_sec, scrape);
    let qs = queries(map, &render(&matchers), &duration(r.rate));
    let results: Vec<_> = futures::stream::iter(qs)
        .map(|q| async move {
            let _permit = permit(source).await;
            let result = source.client.query_range(&q.expr, &r).await;
            (q, result)
        })
        .buffered(CONCURRENCY)
        .collect()
        .await;
    if results.iter().all(|(_, res)| res.is_err()) {
        let first: Option<AppError> = results.into_iter().find_map(|(_, res)| res.err());
        return Err(first.unwrap_or_else(|| AppError::internal_with("no history queries")));
    }
    let (points, partial) = assemble(&r, results);
    Ok(HistoryAnswer {
        window,
        step_sec: r.step,
        source: source.kind,
        points,
        partial,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::client::tests::{snapshot, source};
    use crate::policy::PolicyStatus;
    use crate::servers::client::Timeouts;
    use crate::servers::testing::{Reply, TestServer};
    use crate::storage::profile::HistoryAuth;

    const TIMEOUTS: Timeouts = Timeouts {
        connect_ms: 2_000,
        request_ms: 2_000,
    };

    /// Путь запроса окна, как его строит клиент.
    fn range_path(expr: &str, start: u64, end: u64, step: u64) -> String {
        let q = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs([
                ("query", expr),
                ("start", &start.to_string()),
                ("end", &end.to_string()),
                ("step", &step.to_string()),
            ])
            .finish();
        format!("/api/v1/query_range?{q}")
    }

    /// Тестовый источник, который отвечает снятыми ответами на запросы окна `now` цели.
    async fn captured_source(name: &str, broken: Option<&str>) -> (TestServer, u64) {
        captured_source_delayed(name, broken, None).await
    }

    async fn captured_source_delayed(
        name: &str,
        broken: Option<&str>,
        delay: Option<std::time::Duration>,
    ) -> (TestServer, u64) {
        let doc: serde_json::Value =
            serde_json::from_str(&snapshot(&format!("range-{name}.json"))).unwrap();
        let (start, end) = (doc["start"].as_u64().unwrap(), doc["end"].as_u64().unwrap());
        let paths: Vec<(String, Reply)> = doc["queries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|q| {
                let expr = q["expr"].as_str().unwrap();
                let reply = if broken.is_some_and(|b| expr.contains(b)) {
                    Reply {
                        status: 422,
                        ..Reply::json(
                            r#"{"status":"error","errorType":"execution","error":"boom"}"#,
                        )
                    }
                } else {
                    Reply::json(&q["response"].to_string())
                };
                let reply = Reply { delay, ..reply };
                (range_path(expr, start, end, 15), reply)
            })
            .collect();
        let mut routes: Vec<(&str, Reply)> =
            paths.iter().map(|(p, r)| (p.as_str(), r.clone())).collect();
        let targets = snapshot("targets.json");
        routes.push(("/api/v1/targets", Reply::json(&targets)));
        (TestServer::start(routes).await, end)
    }

    fn client(server: &TestServer) -> Source {
        client_with_deadline(server, Duration::from_millis(TIMEOUTS.request_ms))
    }

    fn client_with_deadline(server: &TestServer, deadline: Duration) -> Source {
        let c = HistoryClient::new(
            &source(&server.url(), HistoryAuth::None),
            TIMEOUTS,
            None,
            &PolicyStatus::default(),
        )
        .unwrap();
        Source::new(c, SourceKind::Prometheus, deadline)
    }

    #[tokio::test]
    async fn slow_source_fails_the_window_by_one_deadline() {
        // Каждый ответ — 300 мс, сам по себе в таймаут запроса (2 с) укладывается; 20 запросов
        // волнами по 4 — 1,5 с, а срок окна — 0,5 с.
        let (server, end) =
            captured_source_delayed("vllm-8000", None, Some(Duration::from_millis(300))).await;
        let source = client_with_deadline(&server, Duration::from_millis(500));
        let started = std::time::Instant::now();
        let e = read_history(
            &source,
            DetectedKind::Vllm,
            r#"{job="vllm", instance="127.0.0.1:8000"}"#,
            Window::Now,
            end,
        )
        .await
        .unwrap_err();
        assert_eq!(
            (e.code, e.key.as_str()),
            (ErrorCode::Timeout, "errors.history.deadline")
        );
        assert!(started.elapsed() < Duration::from_millis(1_000));
        // Отменённые запросы вернули разрешения: следующее окно снова получает все четыре.
        assert_eq!(source.limit.available_permits(), CONCURRENCY);
    }

    #[tokio::test]
    async fn now_window_of_a_vllm_server() {
        let (server, end) = captured_source("vllm-8000", None).await;
        let a = read_history(
            &client(&server),
            DetectedKind::Vllm,
            r#"{job="vllm", instance="127.0.0.1:8000"}"#,
            Window::Now,
            end + 7,
        )
        .await
        .unwrap();
        assert_eq!((a.step_sec, a.points.len()), (15, 61));
        assert!(a.partial.is_empty(), "{:?}", a.partial);
        let rates = a.points[60].m.rates.clone().unwrap();
        assert!((rates.prompt_tokens.unwrap() - 20.82).abs() < 0.05);
        let json = serde_json::to_value(&a).unwrap();
        assert_eq!(json["window"], "now");
        assert_eq!(json["source"], "prometheus");
        assert_eq!(json["points"][60]["at"], json["points"][60]["m"]["at"]);
        // Интервал опроса цели — 5 с: rate по минуте, и все 20 запросов окна совпали со снятыми
        // (иначе были бы `partial`); плюс один запрос targets.
        let paths = server.paths();
        assert_eq!(
            paths.iter().filter(|p| p.contains("query_range")).count(),
            20
        );
        assert_eq!(paths.len(), 21);
    }

    #[tokio::test]
    async fn concurrent_windows_of_one_source_share_the_limit() {
        let (server, end) = captured_source_delayed(
            "vllm-8000",
            None,
            Some(std::time::Duration::from_millis(40)),
        )
        .await;
        let source = client(&server);
        let one = || {
            read_history(
                &source,
                DetectedKind::Vllm,
                r#"{job="vllm", instance="127.0.0.1:8000"}"#,
                Window::Now,
                end,
            )
        };
        let (a, b, c) = tokio::join!(one(), one(), one());
        for r in [a, b, c] {
            assert!(r.unwrap().partial.is_empty());
        }
        let paths = server.paths();
        assert_eq!(
            paths.iter().filter(|p| p.contains("query_range")).count(),
            60
        );
        assert_eq!(server.peak_concurrency(), CONCURRENCY);
    }

    #[tokio::test]
    async fn failed_query_is_partial_and_all_failed_is_an_error() {
        let (server, end) = captured_source("vllm-8000", Some("time_to_first_token")).await;
        let a = read_history(
            &client(&server),
            DetectedKind::Vllm,
            r#"{job="vllm", instance="127.0.0.1:8000"}"#,
            Window::Now,
            end,
        )
        .await
        .unwrap();
        assert_eq!(a.partial.len(), 1);
        assert_eq!(a.partial[0].chart, "ttft");
        assert_eq!(a.partial[0].error.key, "errors.history.failed");

        // Другой селектор — у теста нет ни одного ответа: все запросы — 404, окно — ошибка.
        let e = read_history(
            &client(&server),
            DetectedKind::Vllm,
            r#"{job="other"}"#,
            Window::Now,
            end,
        )
        .await
        .unwrap_err();
        assert_eq!(e.key, "errors.history.status");
    }

    #[tokio::test]
    async fn openai_and_bad_selector_are_errors() {
        let server = TestServer::start(vec![]).await;
        let read = |kind, sel: &'static str| {
            let c = client(&server);
            async move { read_history(&c, kind, sel, Window::Hour, 0).await }
        };
        assert_eq!(
            read(DetectedKind::Openai, r#"{job="x"}"#)
                .await
                .unwrap_err()
                .key,
            "errors.history.noMetrics"
        );
        assert_eq!(
            read(DetectedKind::Vllm, "job=x").await.unwrap_err().key,
            "errors.history.selector"
        );
        assert!(server.paths().is_empty());
    }
}
