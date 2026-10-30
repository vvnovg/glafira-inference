//! vLLM ≥ 0.10 (движок V1). Имена — по `vllm/v1/metrics/loggers.py` на тегах 0.10.0–0.32.0:
//! `gpu_cache_usage_perc` и `gpu_prefix_cache_*` — до 0.11 включительно, `inter_token_latency_seconds`
//! — с 0.10.2 (раньше `time_per_output_token_seconds`, убран к 0.16).

use super::{Canonical as C, MetricKind as K, MetricSpec as S, ServerAdapter};

pub struct Vllm;

const MAP: &[S] = &[
    S::new(C::RequestsRunning, K::Gauge, &["vllm:num_requests_running"]),
    S::new(C::RequestsWaiting, K::Gauge, &["vllm:num_requests_waiting"]),
    S::new(
        C::KvUsage,
        K::Gauge,
        &["vllm:kv_cache_usage_perc", "vllm:gpu_cache_usage_perc"],
    )
    .max_avg(),
    S::new(
        C::PreemptionsTotal,
        K::Counter,
        &["vllm:num_preemptions_total"],
    ),
    S::new(
        C::PromptTokensTotal,
        K::Counter,
        &["vllm:prompt_tokens_total"],
    ),
    S::new(
        C::GenerationTokensTotal,
        K::Counter,
        &["vllm:generation_tokens_total"],
    ),
    S::new(
        C::PrefixCacheQueriesTotal,
        K::Counter,
        &[
            "vllm:prefix_cache_queries_total",
            "vllm:gpu_prefix_cache_queries_total",
        ],
    ),
    S::new(
        C::PrefixCacheHitsTotal,
        K::Counter,
        &[
            "vllm:prefix_cache_hits_total",
            "vllm:gpu_prefix_cache_hits_total",
        ],
    ),
    S::new(
        C::RequestSuccessTotal,
        K::Counter,
        &["vllm:request_success_total"],
    )
    .by("finished_reason"),
    S::new(
        C::RequestAbortedTotal,
        K::Counter,
        &["vllm:request_success_total"],
    )
    .only("finished_reason", "abort"),
    S::new(C::Ttft, K::Histogram, &["vllm:time_to_first_token_seconds"]),
    S::new(
        C::Itl,
        K::Histogram,
        &[
            "vllm:inter_token_latency_seconds",
            "vllm:time_per_output_token_seconds",
        ],
    ),
    S::new(C::E2e, K::Histogram, &["vllm:e2e_request_latency_seconds"]),
    S::new(
        C::QueueTime,
        K::Histogram,
        &["vllm:request_queue_time_seconds"],
    ),
    S::new(C::PromptLen, K::Histogram, &["vllm:request_prompt_tokens"]),
    S::new(C::GenLen, K::Histogram, &["vllm:request_generation_tokens"]),
];

impl ServerAdapter for Vllm {
    fn metric_map(&self) -> &'static [S] {
        MAP
    }
}
