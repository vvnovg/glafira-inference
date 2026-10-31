//! Метрики «сейчас» (спец. I1b §4): снимок `/metrics` сервера по карте адаптера и производные
//! по истории снимков за последнюю минуту. Опрашивает фронтенд, пока вкладка видна.

use std::collections::VecDeque;

use crate::error::AppResult;
use crate::metrics::canonical::{Snapshot, snapshot};
use crate::metrics::derive::{MINUTE_MS, ServerMetrics, Timed, derive};
use crate::metrics::parse::{parse, too_large};
use crate::servers::adapters::adapter;
use crate::servers::client::{METRICS_LIMIT, ServerClient};
use crate::servers::detect::DetectedKind;

/// Предел истории: при опросе из двух вкладок снимков за минуту вдвое больше обычного.
pub const HISTORY_MAX: usize = 64;

/// Канонический снимок `/metrics` сервера. Ответ не 2xx — ошибка по коду; больше 8 МБ —
/// `errors.metrics.tooLarge`.
pub async fn fetch(c: &ServerClient, kind: DetectedKind) -> AppResult<Snapshot> {
    let reply = c.get_ok(c.metrics_path(), METRICS_LIMIT).await?;
    if reply.truncated {
        return Err(too_large());
    }
    let parsed = parse(&String::from_utf8_lossy(&reply.body))?;
    Ok(snapshot(adapter(kind).metric_map(), &parsed))
}

/// Производные нового снимка; снимок уходит в историю. Перезапуск сервера (`reset`) начинает
/// историю заново; снимки старше минуты отбрасываются.
pub fn record(history: &mut VecDeque<Timed>, cur: Timed) -> ServerMetrics {
    let m = derive(history.make_contiguous(), &cur);
    if m.reset {
        history.clear();
    }
    let now = cur.at_ms;
    history.push_back(cur);
    while history
        .front()
        .is_some_and(|t| now.saturating_sub(t.at_ms) > MINUTE_MS)
        || history.len() > HISTORY_MAX
    {
        history.pop_front();
    }
    m
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::canonical::snapshot;
    use crate::servers::adapters::Canonical;

    const BEFORE: &str = include_str!("testdata/vllm-0.32.0/metrics-before.txt");
    const AFTER: &str = include_str!("testdata/vllm-0.32.0/metrics-after.txt");

    fn at(ms: u64, text: &str) -> Timed {
        Timed {
            at_ms: ms,
            snap: snapshot(
                adapter(DetectedKind::Vllm).metric_map(),
                &parse(text).unwrap(),
            ),
        }
    }

    #[test]
    fn history_keeps_one_minute() {
        let mut h = VecDeque::new();
        record(&mut h, at(0, BEFORE));
        let m = record(&mut h, at(5_000, AFTER));
        assert_eq!(m.rates.unwrap().generation_tokens, Some(101.0));
        for i in 2..=14u64 {
            record(&mut h, at(i * 5_000, AFTER));
        }
        assert_eq!(h.len(), 13);
        assert!(h.iter().all(|t| 70_000 - t.at_ms <= MINUTE_MS));
    }

    #[test]
    fn restart_starts_the_history_again() {
        let mut h = VecDeque::new();
        record(&mut h, at(0, AFTER));
        let m = record(&mut h, at(5_000, BEFORE));
        assert!(m.reset);
        assert_eq!(h.len(), 1);
        let m = record(&mut h, at(10_000, AFTER));
        assert!(!m.reset);
        assert_eq!(m.latency.ttft.unwrap().count, 30.0);
    }

    #[test]
    fn history_is_capped_for_fast_polling() {
        let mut h = VecDeque::new();
        for i in 0..100u64 {
            record(&mut h, at(i * 100, AFTER));
        }
        assert_eq!(h.len(), HISTORY_MAX);
    }

    #[tokio::test]
    async fn fetch_reads_the_metrics_path_of_the_profile() {
        use crate::policy::PolicyStatus;
        use crate::servers::client::Timeouts;
        use crate::servers::testing::{Reply, TestServer};
        use crate::storage::profile::{ProxyConfig, sample_server};
        let srv = TestServer::start(vec![("/stats", Reply::text(AFTER))]).await;
        let mut s = sample_server("s1");
        s.base_url = srv.url();
        s.proxy = ProxyConfig::None;
        s.metrics_path = "/stats".into();
        let t = Timeouts {
            connect_ms: 2_000,
            request_ms: 2_000,
        };
        let c = ServerClient::new(&s, t, None, &PolicyStatus::default()).unwrap();
        let snap = fetch(&c, DetectedKind::Vllm).await.unwrap();
        assert_eq!(snap.counters[&Canonical::PromptTokensTotal], 1041.0);
        s.metrics_path = "/missing".into();
        let c = ServerClient::new(&s, t, None, &PolicyStatus::default()).unwrap();
        let e = fetch(&c, DetectedKind::Vllm).await.unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::NotFound);
    }
}
