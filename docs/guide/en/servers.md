# Servers

Open a server tab by clicking the server in the tree or with [[ui:servers.panel.open]] on its card in the installation
tab. Metrics are read from the server's `/metrics` every 5 seconds and only while the tab is shown; they are never
written to disk. Glafira changes nothing on the server and never reads request texts.

## Overview {#overview}

The kind, version and models are the same as on the installation card; a server with several engines (data parallel)
shows their number. The tiles show the current state:

- [[ui:servers.tiles.requests]] — requests in progress and in the queue. A growing queue with a full KV cache is the
  first sign of a lack of GPU memory.
- [[ui:servers.tiles.kv]] — how full the KV cache is; with several engines, the fullest one.
- [[ui:servers.tiles.promptTokens]] and [[ui:servers.tiles.generationTokens]] — tokens per second over the last
  polling interval.
- [[ui:servers.tiles.ttft]] and [[ui:servers.tiles.itl]] — the 95th percentile of time to first token and of
  inter-token latency over the last minute.
- [[ui:servers.tiles.prefixCache]] — the share of request tokens found in the prefix cache.
- [[ui:servers.tiles.preemptions]] — requests preempted from the batch: the server freed KV cache for other requests.

"No data" means the server does not expose the series (the tile's tooltip names it) or there were no requests in the
interval. Glafira never shows zero in place of an unknown value.

## Percentiles {#percentiles}

Percentiles come from the server's histograms, the same way as `histogram_quantile` in Prometheus, with linear
interpolation inside a bucket. The tiles show percentiles over the last minute, the charts over the interval between
polls. Their precision is limited by the server's bucket bounds. The minute never spans a server restart. If the
counters went down, the server restarted: Glafira starts over from the next poll and says so.

## Configuration {#config}

A table of launch arguments with their values and sources. Over HTTP a server reports only part of its arguments:

- [[ui:servers.sources.server_info]] — `/server_info` of SGLang (vLLM serves it only in development mode);
- [[ui:servers.sources.models]] — `/v1/models`: the model name, its path and the context length;
- [[ui:servers.sources.metrics]] — labels of info series in `/metrics`, such as `vllm:cache_config_info`: block size,
  GPU memory share, KV cache blocks, prefix caching.

When sources disagree, the most direct one wins and the other values are shown under it with ⚠. Values that look
like keys, tokens, passwords, and addresses with a login are never shown, only that the value is set. The rest of the
arguments, from the pod or the process, come with the Kubernetes and SSH connections.

## Metrics {#metrics}

Charts for the "Now" window, up to an hour kept in the tab's memory: requests, KV cache, preemptions, tokens per
second, prefix cache hits, TTFT, ITL and end-to-end latency (p50, p95, p99), queue time and finished requests by
reason. A failed poll is a gap in the chart, not a zero. The 1 h, 24 h, 7 d and 30 d windows come with a history
source (Prometheus or VictoriaMetrics).

## Server versions {#versions}

Metrics are read from vLLM 0.10 and later and from SGLang 0.5 and later; SGLang exposes them only when started with
`--enable-metrics`. Series names changed between versions: Glafira takes the new name and falls back to the old one
when the new one has no data.
OpenAI-compatible servers (Ollama, llama.cpp) have no metrics: their tab shows the models and arguments from
`/v1/models`.
