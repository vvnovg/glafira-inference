//! Окно истории сервера (спека I1c §4): шаблоны по карте адаптера, не больше 4 запросов к
//! источнику одновременно, сборка точек; сбой одного запроса — `partial`, всех — ошибка.

use futures::StreamExt;
use serde::Serialize;

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
/// до `evict` инсталляции.
#[derive(Debug)]
pub struct Source {
    pub client: HistoryClient,
    pub kind: SourceKind,
}

/// Значение метки селектора с условием `=`: по `job` и `instance` ищется интервал опроса цели.
fn exact(matchers: &[crate::history::targets::Matcher], label: &str) -> Option<String> {
    matchers
        .iter()
        .find(|m| m.label == label && m.op == MatchOp::Eq)
        .map(|m| m.value.clone())
}

/// Окно `window` сервера типа `kind` с селектором `selector`, которое кончается в `now_sec`.
pub async fn read_history(
    client: &HistoryClient,
    source: SourceKind,
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
        (Some(job), Some(instance)) => client.scrape_interval(&job, &instance).await,
        _ => None,
    };
    let r = range(window, now_sec, scrape);
    let qs = queries(map, &render(&matchers), &duration(r.rate));
    let results: Vec<_> = futures::stream::iter(qs)
        .map(|q| async move {
            let result = client.query_range(&q.expr, &r).await;
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
        source,
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
                (range_path(expr, start, end, 15), reply)
            })
            .collect();
        let mut routes: Vec<(&str, Reply)> =
            paths.iter().map(|(p, r)| (p.as_str(), r.clone())).collect();
        let targets = snapshot("targets.json");
        routes.push(("/api/v1/targets", Reply::json(&targets)));
        (TestServer::start(routes).await, end)
    }

    fn client(server: &TestServer) -> HistoryClient {
        HistoryClient::new(
            &source(&server.url(), HistoryAuth::None),
            TIMEOUTS,
            None,
            &PolicyStatus::default(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn now_window_of_a_vllm_server() {
        let (server, end) = captured_source("vllm-8000", None).await;
        let a = read_history(
            &client(&server),
            SourceKind::Prometheus,
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
    async fn failed_query_is_partial_and_all_failed_is_an_error() {
        let (server, end) = captured_source("vllm-8000", Some("time_to_first_token")).await;
        let a = read_history(
            &client(&server),
            SourceKind::Prometheus,
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
            SourceKind::Prometheus,
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
            async move { read_history(&c, SourceKind::Prometheus, kind, sel, Window::Hour, 0).await }
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
