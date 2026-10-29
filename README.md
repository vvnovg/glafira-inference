# Glafira for LLM Inference

Desktop-приложение (Tauri 2 + Rust + React) для подключения к корпоративному инференсу open-source LLM, его
мониторинга, диагностики и аудита; в фокусе — vLLM и SGLang на GPU NVIDIA.

vLLM, SGLang, LiteLLM, NVIDIA and other product names are trademarks of their respective owners. This project is not
affiliated with them.

Спецификация: `docs/superpowers/specs/2026-10-29-glafira-inference-design.md`. Код, общий с Glafira for Kubernetes
(а через неё — с Glafira for Apache Kafka), перечислен в `PORTED.md`.

## Требования
- Rust stable (`rustup`), Xcode Command Line Tools (macOS)
- Node 22+, pnpm 11
- GPU не нужен: для разработки и смоука есть мок-сервер (`pnpm mock-server`, ниже)

## Запуск
```bash
pnpm install
pnpm tauri dev
```
Dev-сервер Vite слушает порт 1440 (1420 и 1430 заняты Kafka- и Kubernetes-версиями), приложения семейства можно
запускать рядом.

## Тесты
```bash
pnpm test                                                 # фронтенд
pnpm test:scripts                                         # скрипты scripts/*.mjs, в том числе мок-сервер
pnpm i18n:check                                           # словари, ключи Rust, руководство
cargo test --manifest-path src-tauri/Cargo.toml --lib     # Rust: unit-тесты и тестовые HTTP/HTTPS-серверы на локальных портах
```

## Мок-сервер
`pnpm mock-server --kind <вид> [--port 8000] [--api-key sk-dev] [--health 503]` отвечает как сервер инференса
снимками ответов из `src-tauri/src/servers/testdata` — теми же, что проверяют тесты Rust. Виды: `vllm` (0.11),
`vllm-0.8` (модель с адаптером LoRA, старые имена метрик), `vllm-old` (0.7 — ниже границы поддержки), `sglang` (0.5),
`sglang-0.4` (устаревшие пути `/get_*`, без метрик), `llamacpp`, `ollama` (без `/health`). `/health` отвечает без
ключа, остальные пути — только с `Authorization: Bearer <ключ>`, если ключ задан.

## Инсталляции и серверы
- Инсталляция — серверы инференса одной системы (`storage/profile.rs`): имя, окружение, только чтение, таймауты и
  серверы. Сервер — имя, адрес (`http://` или `https://`, путь допускается; `user:pass@` запрещён), тип (`auto`,
  `vllm`, `sglang`, `openai`), путь метрик, CA (системные корни, только перечисленные PEM, без проверки) и прокси.
  Не больше 32 серверов; id сервера генерирует форма, он входит в ключ keychain.
- Клиент сервера (`servers/client.rs`): только GET, перенаправления не выполняются (ключ API не уходит на другой адрес),
  ключ — `Authorization: Bearer` с пометкой «чувствительный», лимит тела — 1 МБ для JSON, 8 МБ для метрик.
- Определение типа (`servers/detect.rs`, чистые функции над снимком ответов): SGLang — `/model_info` и `/server_info`
  (у старых версий — `/get_model_info`, `/get_server_info`), префикс `sglang:` в метриках или `owned_by: sglang`;
  vLLM — префикс `vllm:`, `owned_by: vllm` или JSON `/version`; иначе `openai`. Тип из профиля побеждает признаки, но
  расхождение — предупреждение. vLLM ниже 0.8 и SGLang ниже 0.4 работают как `openai` с предупреждением. Из
  `/server_info` берётся только версия: в ответе SGLang бывает ключ API.
- Подключение инсталляции (`servers/pool.rs`, `AppState::connected`): все серверы параллельно, ошибка сервера — в его
  состоянии; подключения одной инсталляции идут по очереди. Сохранение профиля, удаление и импорт забывают подключение.
- Проверка сервера (`servers/connectivity.rs`): конфигурация → DNS/TCP (или прокси из профиля; при системном прокси
  TCP-шаг пропускается) → TLS (только `https://`) → `/health` (404 — пропуск, 401/403 — проверит следующий шаг) → вход
  (`/v1/models` с ключом) → тип и версия → модели. Предупреждения успеха не отменяют.

## Где хранятся данные
- Профили и папки: `installations.json` в каталоге данных (macOS:
  `~/Library/Application Support/io.github.vvnovg.glafira.inference/`), права 0600.
- Ключи API — в системном хранилище секретов, сервис `glafira-inference`, ключ `<id инсталляции>/<id сервера>/apiKey`.
- Настройки: `settings.json`.
- Экспорт — файл `*.glafira-inference-installations.json` без ключей API; у каждой инсталляции — список серверов, чей
  ключ был сохранён (`needsSecret.apiKeys`). Импорт с «Обновлять существующие» предупреждает, если у сервера с
  сохранённым ключом меняется адрес, прокси или проверка сертификата; ключи серверов, которых нет в файле, удаляются.
- Логи: `RUST_LOG=debug pnpm tauri dev`; файлы — в папке логов ОС (путь — в «О программе»).

## Политика администратора
Ключи `updatesEnabled`, `updateUrl`, `language`, `mcpEnabled`, `allowInsecureTls`; примеры — `docs/admin/`.
`allowExecAuth` появится в I3 вместе с Kubernetes.

## Языки
Интерфейс и руководство — на английском и русском; остальные 10 языков семейства добавляет этап I7.
Как добавить строку — `docs/i18n/README.md`.

## Лицензия и поддержка проекта
- Код открыт под [Apache License 2.0](LICENSE). Имя Glafira и значок лицензией не покрываются — см. `TRADEMARK.md`.
- Пункт «Поддержать проект» ведёт на https://github.com/vvnovg/glafira-inference-releases/blob/main/SUPPORT.md.
- Как участвовать — `CONTRIBUTING.md`; об уязвимостях — `SECURITY.md`.
