# vLLM 0.10.0 — собран по исходникам

Не снят с сервера: собран по `vllm/v1/metrics/loggers.py` на теге `v0.10.0`. Два движка data parallel
(`engine="0"` и `"1"`), устаревшие имена рядом с новыми, как их отдаёт 0.10.0:
`gpu_cache_usage_perc` и `kv_cache_usage_perc`, `gpu_prefix_cache_*` и `prefix_cache_*`; ITL — ещё
`time_per_output_token_seconds` (`inter_token_latency_seconds` появилась в 0.10.2). Значения выдуманы.
