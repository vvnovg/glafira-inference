# Metric history

History comes from the Prometheus or VictoriaMetrics that already scrapes the inference servers. Glafira does not
store history itself: it only reads it with PromQL queries while a server tab is open, and writes nothing to disk.

## Source {#source}

1. Open the installation, click [[ui:installations.panel.edit]], then [[ui:installations.history.add]].
2. Enter the HTTP API address, for example `http://prometheus.monitoring:9090` (a path is allowed:
   `https://gw.example.com/prometheus`).
3. Choose the sign-in: none, a token, or a user name and password. The token and the password are kept only in the
   system secret store; they never go to the installation file or to an export.
4. CA and proxy work as for servers. A source without certificate verification can be opened only if the policy
   allows it.
5. Click [[ui:installations.history.check]]: the check goes through DNS/TCP, sign-in, the `up` query, the search for
   vLLM and SGLang targets and the matching of every server.

If the source is unavailable, the installation still connects: only the history windows are unavailable.

## Matching servers {#matching}

A server needs a series selector — the labels its metrics have in the source, for example
`{job="vllm", instance="10.0.0.5:8000"}`. After the check, every server in [[ui:installations.history.series]] has a
list of the found targets. Only the check fills the lists: in a reopened form a saved target shows as an own selector
until the check runs again:

- **auto-match** — the target whose `instance` equals the host and port of the server address (`localhost` and
  `127.0.0.1` are one address); auto-match never overwrites a choice made by a person; if you change the server address after auto-match, the selector is cleared: run the check again;
- a target from the list — when the server address and `instance` differ: in Kubernetes the source scrapes the pod
  IP while the installation holds the service address;
- [[ui:installations.history.custom]] — any labels in double quotes, without `__name__`: Glafira adds the series
  names itself;
- [[ui:installations.history.noMatch]] — the server has no history.

The [[ui:installations.panel.history]] row on the server card shows the chosen target.

## Windows {#windows}

On the server tab, in the metrics section, the 1 h, 24 h, 7 d and 30 d windows are read from the source:

| Window | Step |
|---|---|
| 1 h | 1 min |
| 24 h | 90 s |
| 7 d | 12 min |
| 30 d | 45 min |

Rates come from `rate`, percentiles from `histogram_quantile` over the same histograms as in the “Now” window; `rate`
handles server restarts itself. A window is read when chosen, a fresh one with [[ui:common.refresh]]; choosing a
window again within a minute does not load the source. If some series are not read, the other charts are still
shown, with a note above them about which series are missing and why.

## The “Now” window {#now}

If the server has history, the “Now” window is filled with the last 15 minutes of it as soon as the tab opens, and
the `/metrics` polling continues from there. A dashed line on the charts marks where the history ends. Until the
second poll, the rates and percentiles on the overview tiles come from the history and are marked “from history”.

## If nothing matched {#troubleshooting}

- “The source does not collect vLLM or SGLang metrics” — the Prometheus configuration has no targets of the servers
  (example below) or SGLang runs without `--enable-metrics`.
- “No target with the address of the server” — choose a target from the list or write a selector: look up the `job`
  and `instance` labels on the Targets page of the Prometheus web interface.
- “There are no series under the selector” — the selector does not match the labels or the server has not been
  scraped yet.
- The window is empty at its start — the source did not keep metrics for that time (for example, it was started
  later).

## Prometheus configuration example {#prometheus-config}

```yaml
scrape_configs:
  - job_name: vllm
    static_configs:
      - targets: ["10.0.0.5:8000"]
  - job_name: sglang
    static_configs:
      - targets: ["10.0.0.7:30000"]
```

vLLM `/metrics` is open without an API key; SGLang serves metrics only when started with `--enable-metrics`.
