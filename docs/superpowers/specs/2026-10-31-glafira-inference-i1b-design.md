# Glafira for LLM Inference — спека этапа I1b «Сервер: чтение и метрики»

Уточнения поверх общей спеки `docs/superpowers/specs/2026-10-29-glafira-inference-design.md` (дальше — «общая
спека»). Где эта спека молчит, действует общая. Решения брейншторма 2026-10-30.

## 1. Решения и границы

- **Этап I1b из §20 общей спеки делится на два.** I1b «Сервер» — адаптеры, канонические метрики, поток метрик
  «сейчас», экраны сервера. I1c «История и GPU» — источник Prometheus/VictoriaMetrics, сопоставление серий, окна
  1 ч / 24 ч / 7 д / 30 д, DCGM (`direct`, `prometheus`), экран GPU. Строка I1b в §20 общей спеки заменяется двумя
  строками (раздел 9).
- **Нижние границы версий:** vLLM ≥ 0.10 (июль 2025), SGLang ≥ 0.5 (август 2025) вместо 0.8 и 0.4 (§1, §4.3 общей
  спеки). Старее — тип «OpenAI-совместимый» с предупреждением, как сейчас у vLLM 0.7.3.
- **Стенда с GPU нет.** Таблицы метрик — по исходникам на тегах релизов и по снимкам; настоящий снимок vLLM 0.32.0
  снят на Apple Silicon (`vllm-metal` 0.32.0, MLX, модель `mlx-community/Qwen2.5-0.5B-Instruct-4bit`). Смоук на стенде
  переносится (§18 общей спеки это разрешает).
- **Поток метрик — подход A:** опрашивает фронтенд, считает Rust (раздел 4).
- **Не входит в I1b:** история, GPU, логи и фильтр промптов (I3), правила и находки (I2), кнопка «Артефакт» (I4).

## 2. Факты о серверах (на 2026-10-30)

- Последние релизы: vLLM 0.32.0 (2026-10-18), SGLang 0.5.22 (2026-10-19).
- **vLLM 0.32.0, снимок:** `/version` (`{"version":"0.32.0"}`), `/health`, `/ping`, `/load` (`{"server_load":0}`) и
  `/metrics` открыты без ключа API; `/v1/models` требует ключ; `/server_info` — 404 без dev-режима. У серий метка
  `engine="<n>"` (по движку data parallel). `vllm:cache_config_info` несёт в метках `block_size`, `cache_dtype`,
  `enable_prefix_caching`, `gpu_memory_utilization`, `num_gpu_blocks`, `kv_cache_size_tokens`,
  `kv_cache_max_concurrency` и другие. Журнал запуска пишет `non-default args` с ключом API как `'***'` и строку
  `Initializing a V1 LLM engine … with config: …` (источник аргументов через логи — I3); журнал доступа — метод, путь,
  код, без текстов запросов.
- **vLLM, переименования (по `vllm/v1/metrics/loggers.py` на тегах):** `gpu_cache_usage_perc` → `kv_cache_usage_perc`
  и `gpu_prefix_cache_{queries,hits}` → `prefix_cache_{queries,hits}` — оба имени в 0.10.x–0.11.x, старые убраны в
  0.12; `time_per_output_token_seconds` → `inter_token_latency_seconds` — новое с 0.10.2, старое убрано к 0.16
  (с 0.11 есть отдельная `request_time_per_output_token_seconds` — TPOT на запрос, не ITL). В 0.11 скрытые
  устаревшие серии включаются флагом `--show-hidden-metrics-for-version=0.10`.
- **SGLang** (`python/sglang/srt/metrics/collector.py`, с 0.5.12 — `srt/observability/metrics_collector.py`): метрики
  только с `--enable-metrics`. Имена стабильны с 0.5.0: `num_running_reqs`, `num_queue_reqs`, `token_usage`,
  `prompt_tokens_total`, `generation_tokens_total`, `cached_tokens_total`, `num_requests_total`,
  `time_to_first_token_seconds`, `inter_token_latency_seconds`, `e2e_request_latency_seconds`; `queue_time_seconds` —
  с 0.5.4; `num_aborted_requests` → `num_aborted_requests_total` в 0.5.4; `num_retracted_requests_total` (счётчик
  вытеснений) — с 0.5.12, раньше только мгновенная `num_retracted_reqs`.

## 3. Адаптеры и канонические метрики

`servers/adapters/` с трейтом `ServerAdapter` (§4.3 общей спеки) и реализациями `vllm`, `sglang`, `openai`. В I1b —
`metric_map()` и `config()`; `request_log_patterns` — в I3. Определение типа (`detect.rs`) остаётся, меняются
границы версий.

**Карта метрик — упорядоченные кандидаты, а не диапазоны версий.** Для каждой канонической метрики адаптер задаёт
список имён серий; берётся первое, которое есть в снимке. Так переход с перекрытием имён (раздел 2) не требует
знать точную версию, а `--show-hidden-metrics-for-version` не даёт двойного счёта. Отсутствующая серия — «нет данных»
(`null`), не ноль.

| Каноническая | vLLM (кандидаты по порядку) | SGLang |
|---|---|---|
| `requests_running` | `vllm:num_requests_running` | `sglang:num_running_reqs` |
| `requests_waiting` | `vllm:num_requests_waiting` | `sglang:num_queue_reqs` |
| `kv_usage` (0…1) | `vllm:kv_cache_usage_perc`, `vllm:gpu_cache_usage_perc` | `sglang:token_usage` |
| `preemptions_total` | `vllm:num_preemptions_total` | `sglang:num_retracted_requests_total` |
| `prompt_tokens_total` | `vllm:prompt_tokens_total` | `sglang:prompt_tokens_total` |
| `generation_tokens_total` | `vllm:generation_tokens_total` | `sglang:generation_tokens_total` |
| `prefix_cache_queries_total` | `vllm:prefix_cache_queries_total`, `vllm:gpu_prefix_cache_queries_total` | `sglang:prompt_tokens_total` |
| `prefix_cache_hits_total` | `vllm:prefix_cache_hits_total`, `vllm:gpu_prefix_cache_hits_total` | `sglang:cached_tokens_total` |
| `request_success_total{finished_reason}` | `vllm:request_success_total` | — (`num_requests_total` без причины → `finished_reason="unknown"`) |
| `request_aborted_total` | из `request_success_total{finished_reason="abort"}` | `sglang:num_aborted_requests_total`, `sglang:num_aborted_requests` |
| `ttft` (гистограмма, с) | `vllm:time_to_first_token_seconds` | `sglang:time_to_first_token_seconds` |
| `itl` | `vllm:inter_token_latency_seconds`, `vllm:time_per_output_token_seconds` | `sglang:inter_token_latency_seconds` |
| `e2e` | `vllm:e2e_request_latency_seconds` | `sglang:e2e_request_latency_seconds` |
| `queue_time` | `vllm:request_queue_time_seconds` | `sglang:queue_time_seconds` |
| `prompt_len` | `vllm:request_prompt_tokens` | `sglang:prompt_tokens_histogram` |
| `gen_len` | `vllm:request_generation_tokens` | `sglang:generation_tokens_histogram` |

Тип `openai` метрик не читает: у llama.cpp и Ollama своих канонических серий нет — экран показывает «нет данных».

**Сводка серий.** Серии одной метрики с разными метками, кроме `finished_reason`, сводятся в одно значение: счётчики,
`requests_running`, `requests_waiting` и корзины гистограмм складываются; `kv_usage` — максимум (на графике — ещё и
среднее). Метки движков (`engine` у vLLM, `tp_rank`/`dp_rank` у SGLang) в интерфейс не идут: в I1b показывается
сервер целиком, число движков — в «Обзоре».

**Разбор `/metrics`** — свой модуль `metrics/parse.rs` (текстовый формат Prometheus 0.0.4): `# TYPE`, имена, метки
с экранированием `\"`, `\\`, `\n`, значения `NaN`, `+Inf`, `-Inf`, суффиксы `_bucket`/`_sum`/`_count`; `_created`
и `# HELP` пропускаются; строки длиннее 64 КБ и ответ больше 8 МБ — ошибка `errors.metrics.tooLarge`. OpenMetrics
(`# EOF`) принимается так же.

## 4. Поток метрик «сейчас»

- **Команда** `read_server_metrics(installationId, serverId) -> ServerMetrics`: `GET <metricsPath>` клиентом сервера
  (ключ API, CA, прокси — как в I1a), разбор, карта адаптера, сводка; затем производные по предыдущему снимку этого
  сервера. Снимки за последние 60 с (не больше 13) хранятся в `Connected` (пул, I1a); переподключение и
  «Подключиться заново» их сбрасывают.
- **`ServerMetrics`:** `at` (время снимка), `intervalSec` (с предыдущего, `null` у первого), `engines` (число
  движков), `gauges` (`requestsRunning`, `requestsWaiting`, `kvUsage`, `kvUsageAvg`), `rates` в секунду
  (`promptTokens`, `generationTokens`, `requests`, `preemptions`, `aborted`, `finishedByReason`), `prefixCacheHitRatio`
  за интервал, `latency` (`ttft`, `itl`, `e2e`, `queueTime` — `{ p50, p95, p99, count }` в миллисекундах за
  интервал; `latency1m` — то же за последнюю минуту, по самому старому снимку не старше 60 с), `totals` (счётчики как есть — для «Обзора»), `missing` (канонические имена без серии), `series`
  (какая серия питает каждую каноническую метрику — для подсказок).
- **Производные (`metrics/derive.rs`, чистые функции):** скорость = Δсчётчика / Δt; доля попаданий = Δhits / Δqueries
  (`null`, если Δqueries = 0); перцентиль — по разности корзин гистограммы за интервал с линейной интерполяцией
  внутри корзины, как `histogram_quantile` Prometheus (корзина `+Inf` — верхняя граница предыдущей; Δcount = 0 —
  `null`). Уменьшение любого счётчика — сброс (перезапуск сервера): все производные интервала `null`, снимок
  становится новой базой, в ответе `reset: true`.
- **Фронтенд:** react-query, `refetchInterval` 5 с только при видимой вкладке (`tabActive`); кольцевой буфер в памяти
  вкладки — точки за последний час (до 720); ошибка опроса — точка-разрыв и строка «Нет ответа сервера: …» в шапке,
  опрос продолжается. Метрики на диск не пишутся (§6.6 общей спеки).
- **Команда** `read_server_config(installationId, serverId) -> ConfigEntry[]` (раздел 5), без кэша: читается при
  открытии «Конфигурации» и по кнопке «Обновить».

## 5. Аргументы запуска

`{ name, value, source, alternatives? }`, источники I1b: `server_info`, `models`, `metrics`; `pod`, `cmdline`,
`docker` — с I3. Приоритет и правило «значение по умолчанию не подставляется» — §4.3 общей спеки; при расхождении
источников в `alternatives` — значения других источников.

- **vLLM:** `models` — `served_model_name` (`id`), `model` (`root`), `max_model_len`; `metrics` — метки
  `vllm:cache_config_info` (`block_size`, `cache_dtype`, `enable_prefix_caching`, `gpu_memory_utilization`,
  `num_gpu_blocks`, `kv_cache_size_tokens`, `kv_cache_max_concurrency`, `prefix_caching_hash_algo`; метки со значением
  `None` пропускаются), `lora_requests_info` (`max_lora`); `server_info` — если отвечает (dev-режим), его поля.
- **SGLang:** `server_info` — `/get_server_info` (с 0.5.4 и `/server_info`) целиком, кроме полей с секретами (`api_key`,
  `admin_api_key` и других по списку адаптера — значение заменяется на `«задан»`), `models` — `/v1/models` и
  `/get_model_info` (`model_path`, `is_generation`).
- Порядок строк — по таблице известных аргументов адаптера (модель, параллелизм, память и KV, батч, кэш), остальные —
  по алфавиту.

## 6. Интерфейс

- **Вкладка сервера** (`server:<installationId>:<serverId>`): открывается из дерева (клик по серверу) и кнопкой
  «Открыть» на карточке во вкладке инсталляции; заголовок «сервер · инсталляция»; подвкладки «Обзор», «Конфигурация»,
  «Метрики» (подвкладка «Логи» — с I3).
- **«Обзор»:** статус и тип/версия (как карточка I1a), модели (`id`, `root`, `max_model_len`), число движков; плитки —
  running / waiting, KV-кэш %, токены/с (вход и выход), p95 TTFT, p95 ITL, prefix-cache hit %, вытеснения за минуту;
  перцентили на плитках — за последнюю минуту; у плитки без серии — «нет данных» и подсказка (какая серия нужна; у SGLang без метрик — «запустите с
  --enable-metrics»).
- **«Конфигурация»:** таблица «Аргумент · Значение · Источник» (бейдж источника, ⚠ и все значения при расхождении);
  пояснение «По HTTP сервер отдаёт часть аргументов; остальные — из пода или процесса (с этапа I3)»; кнопка
  «Обновить».
- **«Метрики»:** графики uPlot — running/waiting; KV-кэш (макс. и средн.); вытеснения/с; токены/с (вход, выход);
  prefix-cache hit %; TTFT, ITL, E2E — p50/p95/p99; очередь p95; завершения по `finished_reason` и прерывания/с.
  Переключатель окна: «Сейчас» активно, 1 ч / 24 ч / 7 д / 30 д — неактивны с подсказкой «История — с этапа I1c».
  График без данных — «нет данных», разрывы при ошибках опроса.
- **Компонент графика** переносится из Kubernetes-версии (`src/components/resources/UsageChart.tsx`, uPlot) с
  записью в `PORTED.md`.
- **Вкладка инсталляции:** на карточке сервера — строка «running · waiting · KV %» (тот же `read_server_metrics`
  раз в 5 с при видимой вкладке); кнопка «Открыть».
- **Дерево:** у сервера — имя первой модели (как сейчас) и переход на вкладку сервера.
- **Справка:** глава `servers` (en, ru): вкладка сервера, что означает каждая метрика и плитка, перцентили за интервал,
  источники аргументов, «нет данных», `--enable-metrics` у SGLang; F1 на вкладке сервера открывает её.
- **Словари:** все тексты — в en и ru; ключи ошибок `errors.metrics.*`.

## 7. Тестирование

- **Снимки** `src-tauri/src/servers/testdata/`: `vllm-0.32.0` (настоящий, Apple Silicon; `metrics-before.txt`,
  `metrics-after.txt` — после 30 запросов, `models.json`, `version.json`, `load.json`), `vllm-0.10.0` и `vllm-0.11.0`
  (дополняется полным `/metrics`) — по исходникам тегов, `sglang-0.5.4` (дополняется) и `sglang-0.5.22` — по исходникам;
  у собранных по исходникам — `README.md` в каталоге: тег, файл, что не настоящее. Ключи API и тексты запросов
  в снимки не попадают.
- **Rust:** разбор Prometheus (экранирование, `NaN`/`±Inf`, `_created`, OpenMetrics, лимиты); карта метрик на каждом
  снимке — каждая каноническая метрика найдена или в `missing`, без двойного счёта при перекрытии имён; сводка движков
  (снимок с `engine="0"` и `"1"`); производные — скорости, доля попаданий, перцентили против эталонных значений
  `histogram_quantile` на известных корзинах, Δcount = 0, сброс счётчиков; аргументы — приоритет, расхождения,
  `None`, скрытие секретов SGLang; новые границы версий (vLLM 0.9.x и SGLang 0.4.x — «OpenAI-совместимый» с
  предупреждением).
- **Мок-сервер:** `--kind vllm` и `--kind sglang` — снимки свежих версий со счётчиками и гистограммами, растущими от
  запроса к запросу; новый `--kind vllm-0.10` (перекрытие старых и новых имён); `--engines N` — N движков;
  `--reset-after N` — счётчики обнуляются после N опросов `/metrics`.
- **Фронтенд (Vitest):** подвкладки, плитки и «нет данных», таблица аргументов с расхождением, буфер (обрезка по
  часу, разрывы), остановка опроса на скрытой вкладке, неактивные окна истории.
- **Смоук `docs/smoke/i1b.md`:** моки vLLM и SGLang, `--engines 2`, `--reset-after`, SGLang без метрик; по желанию —
  локальный vLLM через `vllm-metal` (раздел 2); пункты для GPU-стенда — отложены до стенда.

## 8. Риски этапа

| Риск | Мера |
|---|---|
| Таблицы по исходникам расходятся с настоящими серверами | Кандидаты по порядку вместо диапазонов; `missing` и «нет данных»; смоук на стенде, как только он появится |
| Apple Silicon-снимок отличается от CUDA | Различия — в метках `cache_config_info` и устройстве; канонические серии те же (сверено с исходниками) |
| Перцентили за 5 с при малом трафике шумят | `count` в ответе; плитки показывают p95 за последнюю минуту (`latency1m`), графики — за интервал |
| Большой `/metrics` (сотни серий, LoRA, DP) | Лимит 8 МБ, разбор потоком строк, сводка без хранения лишних серий |

## 9. Изменения общей спеки

- §1 и §4.3: нижние границы — vLLM 0.10, SGLang 0.5.
- §20: строка I1b заменяется на «I1b. Сервер: чтение и метрики» (адаптеры, канонические метрики, поток «сейчас»,
  экраны сервера, графики) и «I1c. История и GPU» (источник истории и сопоставление серий, окна истории, DCGM
  `direct` и `prometheus`, экран GPU).
