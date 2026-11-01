# Ответы Prometheus 3.15.1

Настоящие ответы HTTP API Prometheus 3.15.1 (Homebrew, macOS), сняты 2026-10-31. Prometheus опрашивал
раз в 5 с мок-серверы Glafira (`scripts/prometheus/smoke.yml`): `vllm` на 8000, `vllm --engines 2` на 8001,
`vllm --reset-after 20` на 8003 (счётчики сбрасываются каждые 100 с), `sglang` на 30000. Имя хоста в
`globalUrl` ответа `targets` заменено на `prometheus-host.local`.

| Файл | Запрос |
|---|---|
| `buildinfo.json` | `/api/v1/status/buildinfo` |
| `targets.json` | `/api/v1/targets?state=active` |
| `up.json` | `query=up` |
| `discover.json` | `count by (job, instance) ({__name__=~"vllm:.+\|sglang:.+"})` |
| `discover-sglang.json` | `count by (job, instance) ({__name__=~"sglang:.+"})` |
| `bad-data.json` | неполное выражение — ответ 400 `bad_data` |
| `empty.json` | `up{job="nothing"}` — пустой результат |
| `range-<цель>.json` | `query_range` шаблонов `promql::queries` (окно `now`: 15 мин, шаг 15 с, `rate` 1m) |

`range-<цель>.json` — `{ start, end, step, rate, queries: [{ expr, response }] }`; цели: `vllm-8000` и
`sglang-30000` — все 20 шаблонов, `vllm-8003` — скорость токенов на входе и TTFT p95 (перезапуски). Prometheus
запущен примерно через 2,5 мин после начала окна, поэтому первые точки окна пустые.
