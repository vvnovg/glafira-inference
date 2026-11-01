# Ответы Prometheus 3.15.1 с сериями DCGM

Настоящие ответы HTTP API Prometheus 3.15.1 (Homebrew, macOS), сняты 2026-11-01. Prometheus опрашивал раз в 5 с
мок-серверы Glafira (`scripts/prometheus/smoke.yml`, job `dcgm`): `dcgm-full --gpus 8 --hostname gpu-node-1` на 9400
и `dcgm --gpus 2` на 9401; цель 9402 не запущена.

| Файл | Запрос |
|---|---|
| `instant.json` | мгновенный запрос карт `gpu::source::instant_expr` с селектором `{job="dcgm"}` |
| `cards.json` | `count by (hostname, Hostname, instance, gpu) (DCGM_FI_DEV_FB_USED)` |
| `targets.json` | `count by (job, instance) (DCGM_FI_DEV_FB_USED)` |
| `range-gpu-node-1.json` | `query_range` шаблонов `gpu::promql::queries` для `job="dcgm",hostname="gpu-node-1"` (окно `now`: 15 мин, шаг 15 с, интервал 1m) |

`range-gpu-node-1.json` — `{ start, end, step, rate, queries: [{ expr, response }] }`. Prometheus запущен примерно за
6 мин до конца окна, поэтому первые точки окна пустые.
