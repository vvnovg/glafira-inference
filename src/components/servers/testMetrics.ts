import type { ServerMetrics } from "../../api/types";

/** Метрики сервера для тестов: vLLM с одним движком, все серии на месте. */
export function metrics(at: number, over: Partial<ServerMetrics> = {}): ServerMetrics {
  return {
    at,
    intervalSec: 5,
    reset: false,
    engines: 1,
    gauges: { requestsRunning: 3, requestsWaiting: 1, kvUsage: 0.42, kvUsageAvg: 0.4 },
    rates: { promptTokens: 120, generationTokens: 48, requests: 2, preemptions: 0, aborted: 0, finishedByReason: { stop: 2 } },
    prefixCacheHitRatio: 0.35,
    latency: { ttft: { p50: 80, p95: 240, p99: 400, count: 10 }, itl: null, e2e: null, queueTime: null },
    latency1m: { ttft: { p50: 90, p95: 250, p99: 410, count: 120 }, itl: null, e2e: null, queueTime: null },
    preemptions1m: 0,
    totals: { promptTokensTotal: 1000 },
    missing: [],
    series: { kvUsage: "vllm:kv_cache_usage_perc" },
    ...over,
  };
}
