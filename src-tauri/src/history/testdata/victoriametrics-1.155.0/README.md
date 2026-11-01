# Ответы VictoriaMetrics 1.155.0

Настоящие ответы VictoriaMetrics 1.155.0 (single-node, Homebrew, macOS), сняты 2026-11-01. VictoriaMetrics
опрашивала мок-серверы Glafira по конфигу смоука I1c (`scripts/prometheus/smoke.yml`).

| Файл | Запрос |
|---|---|
| `buildinfo.json` | `/api/v1/status/buildinfo` — версия совместимого Prometheus API (`2.24.0`), а не своя |
| `up.json` | `query=up` |
| `probe.json` | `query=vector(1)` — поле `stats.seriesFetched` есть только у VictoriaMetrics |
| `metrics.txt` | строки `go_info` и `vm_app_version` из `/metrics` — настоящая версия |

`probe.json` Prometheus 3.15.1 того же запроса лежит в `../prometheus-3.15.1/probe.json`: поля `stats` в нём нет.
