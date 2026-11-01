//! Окно истории GPU (спека I1d §4): по какой метке подписан хост в источнике истории, шаблоны
//! полей, не больше 4 запросов одновременно (общий предел источника), сборка рядов; сбой одного
//! запроса — `partial`, всех — ошибка. На всё окно — срок источника, как у окна сервера.

use futures::StreamExt;
use serde::Serialize;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::gpu::bind::same_host;
use crate::gpu::promql::{hosts_expr, queries};
use crate::gpu::series::{GpuSeries, assemble, series_host};
use crate::history::client::SourceKind;
use crate::history::read::{CONCURRENCY, Source, permit};
use crate::history::series::Partial;
use crate::history::targets::{escape, parse_selector, render};
use crate::history::window::{Window, range};

/// Ответ `read_gpu_history`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuHistoryAnswer {
    pub window: Window,
    pub step_sec: u64,
    pub source: SourceKind,
    pub series: Vec<GpuSeries>,
    pub partial: Vec<Partial>,
}

fn host_not_in_history(host: &str) -> AppError {
    app_err!(
        ErrorCode::NotFound,
        "errors.gpu.hostNotInHistory",
        "The metrics history has no GPU series of host {{host}}",
        host = host
    )
}

/// Базовые условия окна из селектора (`None` — без условий, все карты источника).
pub fn base(selector: Option<&str>) -> AppResult<String> {
    selector.map_or(Ok(String::new()), |s| Ok(render(&parse_selector(s)?)))
}

/// Условия серий хоста `host` под базовыми условиями `bases` (`base`): первая база, где хост
/// нашёлся, плюс точное условие на метку, которой он подписан (`hostname`, `Hostname` или
/// `instance`).
async fn host_matchers(source: &Source, bases: &[String], host: &str) -> AppResult<String> {
    for matchers in bases {
        let found = {
            let _permit = permit(source).await;
            source.client.query(&hosts_expr(matchers)).await?
        };
        for s in found {
            if !series_host(&s.labels).is_some_and(|h| same_host(&h, host)) {
                continue;
            }
            let get = |k: &str| s.labels.get(k).filter(|v| !v.is_empty());
            let (label, value) = match (get("hostname"), get("Hostname"), get("instance")) {
                (Some(v), _, _) => ("hostname", v),
                (None, Some(v), _) => ("Hostname", v),
                (None, None, Some(v)) => ("instance", v),
                _ => continue,
            };
            let exact = format!("{label}=\"{}\"", escape(value));
            return Ok(if matchers.is_empty() {
                exact
            } else {
                format!("{matchers},{exact}")
            });
        }
    }
    Err(host_not_in_history(host))
}

/// Окно `window` карт хоста `host` (одной карты `gpu` или всех), которое кончается в `now_sec`.
pub async fn read_history(
    source: &Source,
    bases: &[String],
    host: &str,
    gpu: Option<u32>,
    window: Window,
    now_sec: u64,
) -> AppResult<GpuHistoryAnswer> {
    let read = read_window(source, bases, host, gpu, window, now_sec);
    match tokio::time::timeout(source.deadline(), read).await {
        Ok(answer) => answer,
        Err(_) => Err(app_err!(
            ErrorCode::Timeout,
            "errors.history.deadline",
            "The metrics history source did not return the window in {{seconds}} s: choose a shorter window or raise the request timeout",
            seconds = source.deadline().as_secs()
        )),
    }
}

async fn read_window(
    source: &Source,
    bases: &[String],
    host: &str,
    gpu: Option<u32>,
    window: Window,
    now_sec: u64,
) -> AppResult<GpuHistoryAnswer> {
    let mut matchers = host_matchers(source, bases, host).await?;
    if let Some(g) = gpu {
        matchers.push_str(&format!(",gpu=\"{g}\""));
    }
    let r = range(window, now_sec, None);
    let results: Vec<_> = futures::stream::iter(queries(&matchers, r.rate))
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
        return Err(first.unwrap_or_else(|| AppError::internal_with("no GPU history queries")));
    }
    let (series, mut partial, truncated) = assemble(&r, results);
    if truncated {
        partial.push(Partial {
            chart: "all",
            error: app_err!(
                ErrorCode::InvalidInput,
                "errors.gpu.tooManySeries",
                "The host has more than {{max}} GPUs: the first {{max}} are shown",
                max = crate::gpu::series::MAX_SERIES
            ),
        });
    }
    Ok(GpuHistoryAnswer {
        window,
        step_sec: r.step,
        source: source.kind,
        series,
        partial,
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::history::client::HistoryClient;
    use crate::history::client::tests::source as history_source;
    use crate::policy::PolicyStatus;
    use crate::servers::client::Timeouts;
    use crate::servers::testing::{Reply, TestServer};
    use crate::storage::profile::HistoryAuth;

    fn snap(name: &str) -> String {
        crate::gpu::source::tests::snapshot_file(name)
    }

    async fn open(url: &str) -> Source {
        let t = Timeouts {
            connect_ms: 2_000,
            request_ms: 2_000,
        };
        let client = HistoryClient::new(
            &history_source(url, HistoryAuth::None),
            t,
            None,
            &PolicyStatus::default(),
        )
        .unwrap();
        Source::new(client, SourceKind::Prometheus, Duration::from_secs(5))
    }

    /// Ответ `query_range` одного шаблона из снятого окна; поиск хостов — снятые карты.
    fn routes() -> Vec<(&'static str, Reply)> {
        let raw: serde_json::Value = serde_json::from_str(&snap("range-gpu-node-1.json")).unwrap();
        let util = raw["queries"][0]["response"].to_string();
        vec![
            ("/api/v1/query", Reply::json(&snap("cards.json"))),
            ("/api/v1/query_range", Reply::json(&util)),
        ]
    }

    #[tokio::test]
    async fn window_of_a_host_finds_its_label_and_reads_every_field() {
        let p = TestServer::start(routes()).await;
        let source = open(&p.url()).await;
        let a = read_history(
            &source,
            &[r#"job="dcgm""#.into()],
            "gpu-node-1",
            Some(3),
            Window::Hour,
            1_793_556_000,
        )
        .await
        .unwrap();
        assert_eq!((a.window, a.step_sec), (Window::Hour, 60));
        assert!(a.partial.is_empty());
        assert_eq!(
            a.series.len(),
            8,
            "тестовый сервер отдаёт один снятый ряд на любой шаблон"
        );
        let paths = p.paths();
        assert_eq!(paths.len(), 15, "поиск хоста и 14 шаблонов");
        let range = paths.iter().find(|x| x.contains("query_range")).unwrap();
        let decoded = url::form_urlencoded::parse(range.split_once('?').unwrap().1.as_bytes())
            .find(|(k, _)| k == "query")
            .unwrap()
            .1
            .to_string();
        assert!(
            decoded.contains(r#"{job="dcgm",hostname="gpu-node-1",gpu="3",GPU_I_ID=""}"#),
            "{decoded}"
        );
        assert!(p.peak_concurrency() <= CONCURRENCY);
    }

    #[tokio::test]
    async fn unknown_host_and_failed_source_are_errors() {
        let p = TestServer::start(routes()).await;
        let source = open(&p.url()).await;
        let e = read_history(
            &source,
            &[r#"job="dcgm""#.into()],
            "gpu-node-9",
            None,
            Window::Hour,
            1_793_556_000,
        )
        .await
        .unwrap_err();
        assert_eq!(
            (e.key.as_str(), e.params["host"].as_str()),
            ("errors.gpu.hostNotInHistory", "gpu-node-9")
        );
        let e = read_history(&source, &[], "gpu-node-1", None, Window::Hour, 0)
            .await
            .unwrap_err();
        assert_eq!(
            e.key, "errors.gpu.hostNotInHistory",
            "без базовых селекторов хоста не найти"
        );

        let down = TestServer::start(vec![
            ("/api/v1/query", Reply::json(&snap("cards.json"))),
            ("/api/v1/query_range", Reply::status(503)),
        ])
        .await;
        let source = open(&down.url()).await;
        let e = read_history(
            &source,
            &[r#"job="dcgm""#.into()],
            "gpu-node-1",
            None,
            Window::Hour,
            0,
        )
        .await
        .unwrap_err();
        assert_eq!(
            e.key, "errors.history.status",
            "все запросы упали — ошибка первого"
        );
    }

    #[tokio::test]
    async fn slow_source_misses_the_deadline() {
        let slow = TestServer::start(vec![(
            "/api/v1/query",
            Reply::json(&snap("cards.json")).delayed(Duration::from_millis(300)),
        )])
        .await;
        let t = Timeouts {
            connect_ms: 2_000,
            request_ms: 2_000,
        };
        let client = HistoryClient::new(
            &history_source(&slow.url(), HistoryAuth::None),
            t,
            None,
            &PolicyStatus::default(),
        )
        .unwrap();
        let source = Source::new(client, SourceKind::Prometheus, Duration::from_millis(100));
        let e = read_history(
            &source,
            &[r#"job="dcgm""#.into()],
            "gpu-node-1",
            None,
            Window::Hour,
            0,
        )
        .await
        .unwrap_err();
        assert_eq!(e.key, "errors.history.deadline");
    }

    #[test]
    fn base_conditions_come_from_the_selector() {
        assert_eq!(base(None).unwrap(), "");
        assert_eq!(
            base(Some(r#"{job="dcgm", hostname=~"a|b"}"#)).unwrap(),
            r#"job="dcgm",hostname=~"a|b""#
        );
        assert_eq!(
            base(Some("job=dcgm")).unwrap_err().key,
            "errors.history.selector"
        );
    }
}
