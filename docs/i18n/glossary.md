# Глоссарий переводов Glafira for LLM Inference

Единые решения по терминам. Эталон ключей и параметров — `en.json`, эталон смысла — `ru.json`. Меняя решение,
исправьте его во всех словарях одним коммитом. Общие правила стиля — как в семействе Glafira: коротко и нейтрально,
параметры `{{…}}` и теги `<code>` не переводятся, многоточие — `…`, тире — `—`.

## Не переводится
Glafira, vLLM, SGLang, LiteLLM, Ollama, llama.cpp, Triton, TGI, NVIDIA, OpenAI, Prometheus, LoRA, TLS, PEM, DNS, TCP,
CA, API, URL, JSON, HTTP, MCP, LLM; названия метрик и путей (`/metrics`, `/health`, `/v1/models`), аргументов и
переменных (`--api-key`, `--enable-metrics`, `HTTPS_PROXY`, `NO_PROXY`); TTFT, ITL, TPOT, goodput, prefill, decode,
TP/PP.

## Термины

| en | ru | Примечание |
|---|---|---|
| installation | инсталляция | серверы инференса одной системы; женский род: «новая инсталляция», «её серверы» |
| inference server / server | сервер инференса / сервер | компонент инсталляции |
| inference | инференс | |
| server type | тип сервера | vLLM, SGLang, OpenAI-совместимый |
| OpenAI-compatible | OpenAI-совместимый | |
| detect automatically | определить автоматически | тип сервера |
| API key | ключ API | не «API-ключ» |
| model | модель | |
| context length | длина контекста | `max_model_len` |
| adapter (LoRA) | адаптер LoRA | |
| metrics | метрики | «отдаёт метрики» |
| metrics path | путь метрик | |
| health | здоровье | шаг проверки `/health`: «Сервер здоров» |
| server check / check the server | проверка сервера / проверить сервер | пошаговая проверка в форме |
| sign-in (step) | вход | шаг проверки с ключом API |
| reconnect | подключиться заново | |
| certificate verification | проверка сертификата | |
| trusted CA | доверенный CA | |
| proxy | прокси | |
| read-only | только чтение | |
| environment | окружение | local, dev, test, stage, prod не переводятся |
| folder | папка | |
| KV cache | KV-кэш | с I1b |
| preemption | вытеснение | с I1b |
| containment (of the perimeter) | замкнутость контура | с I5 |
| egress probe | проба выхода | с I5 |
| load test | нагрузочный замер | с I6 |
| artifact | артефакт | аргументы запуска, фрагменты манифестов (с I4) |
