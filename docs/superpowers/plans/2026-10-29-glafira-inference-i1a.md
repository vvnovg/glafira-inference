# I1a «Каркас и подключения» — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Новое приложение Glafira for LLM Inference на основе Glafira for Kubernetes: инсталляции из серверов инференса (vLLM, SGLang, любой OpenAI-совместимый), ключи API в системном хранилище секретов, определение типа, версии и моделей сервера по его ответам, пошаговая проверка сервера, подключение инсталляции и вкладка с серверами, папки, экспорт и импорт, политика `allowInsecureTls`, настройки языка, встроенная справка, «О программе» и мок-сервер для разработки без GPU.

**Architecture:** Tauri 2 + Rust + React, как семейство. Общая основа переносится из Glafira for Kubernetes (`~/projects/glafira-openshift` @ `468e08a`) скриптом `scripts/port.sh` и правится патчами; каждый перенесённый файл записан в `PORTED.md`. Новое в Rust — `storage/profile.rs` (инсталляция и её серверы), `servers/` (HTTP-клиент из профиля, ошибки, определение типа по снимку ответов — чистые функции, снимок, пул подключённых инсталляций, пошаговая проверка), команды `commands/installations.rs`. Фронтенд — форма инсталляции с проверкой каждого сервера, вкладка инсталляции, дерево; справка, настройки, «О программе», раскладка и обмен переносятся.

**Tech Stack:** Rust 2024 (tauri 2, reqwest 0.13 `rustls-no-provider`, rustls 0.23 ring, tokio, futures, serde, keyring 3, url, pem, uuid, time, plist; в тестах — tokio-rustls 0.26 и tempfile), TypeScript (React 19, TanStack Query 5, zustand 5, i18next, react-markdown 9.1, @floating-ui/react), Vitest + Testing Library, pnpm 11, Node 22 (мок-сервер без зависимостей).

**Спецификация:** `docs/superpowers/specs/2026-10-29-glafira-inference-design.md` — строка I1a в §20 и разделы §1, §2, §3, §4.1–4.3, §5.1, §5.2, §5.8, §5.9, §12, §13, §14, §16–19. Этапы I1b и дальше пишутся отдельными планами и опираются на клиент, пул, определение типа и вкладки I1a.

Решения по вопросам, на которые спека не отвечала:
- id сервера генерирует форма (`s-` и 12 шестнадцатеричных знаков из `crypto.getRandomValues`), Rust проверяет его вид и уникальность; id инсталляции — Rust (`uuid`), если форма прислала пустой;
- ключ API в форме: пустое поле — оставить сохранённый, флажок «Удалить сохранённый ключ» — `""`; сохранение — сначала `Set`, потом `Delete`, отказ keychain откатывает профиль;
- импорт с «Обновлять существующие» удаляет из keychain ключи серверов, которых нет в файле; неизвестные поля серверов — локальные, как у профиля;
- значок пока перенесён из Kubernetes-версии (свой — отдельным коммитом, как там `eede48f`); порт dev-сервера — 1440;
- `allowExecAuth` — в I3 вместе с Kubernetes; обновления (updater), заметки к выпуску и раздел MCP в настройках — в I2 вместе с релизным процессом и MCP-шлюзом;
- мок-сервер — Node без зависимостей в `scripts/`, отвечает теми же снимками, что проверяют тесты Rust (сквозная проверка Rust ↔ мок выполнена при подготовке плана, в план не входит).

Уточнения спеки по итогам прототипа (задача 16 вносит их в спеку):
- адрес сервера — `http://` или `https://` (внутренние серверы часто без TLS); по `http://` ключ API уходит открытым текстом — форма предупреждает (§5.2);
- перенаправления HTTP не выполняются, 3xx — ошибка `errors.server.redirect`: ключ API не уходит на другой адрес (§5.2);
- шаги проверки сервера уточнены: TCP к прокси профиля, при системном прокси шаг пропускается; TLS — только для `https://`; `/health` 404 — пропуск, 401/403 — проверит «Вход»; вход без ключа и с неверным ключом различаются; пустой список моделей — ошибка (§5.8);
- экспорт несёт `needsSecret.apiKeys` (id серверов с сохранённым ключом), импорт удаляет ключи убранных серверов (§5.9);
- отказ по политике — код `INVALID_INPUT`, как в семействе, а не `FORBIDDEN`; версия ниже нижней границы — предупреждение `messages.servers.unsupportedVersion`, а не код `SERVER_UNSUPPORTED` (§16).

**Проверено до написания плана.** Весь код плана собран и прогнан в отдельной копии репозитория (по коммиту на задачу, база `0f42a65` — репозиторий только со спекой); повторное применение всех блоков плана в чистом клоне `0f42a65` даёт то же дерево после каждой задачи побайтно (кроме генерируемых `src-tauri/Cargo.lock` и `pnpm-lock.yaml`), проверки после каждой задачи — зелёные:
- Rust — 161 unit-тестов (`cargo test --lib`; было 1 после задачи 1), clippy с `-D warnings`, `cargo fmt --check`; `cargo deny` — `licenses ok`, `advisories ok`; в дереве зависимостей нет `aws-lc-rs` и `openssl-sys`. Тесты клиента и проверки сервера идут против настоящих локальных HTTP- и HTTPS-серверов (тестовый CA);
- фронтенд — 182 тестов в 31 файлах, `tsc --noEmit`, `pnpm build` с проверкой совместимости бандла с WebKit macOS 11, `pnpm i18n:check` (словари en и ru, 95 вызовов из Rust, 2 главы руководства на двух языках);
- скрипты — 67 тестов (`pnpm test:scripts`), в том числе мок-сервер; шаги «Увидеть падение» проверены — тесты фронтенда падают до реализации;
- сквозная проверка: `test_server` из Rust против мок-сервера всех видов (vLLM 0.11 с ключом, SGLang 0.5 и 0.4, Ollama, vLLM 0.7) даёт ожидаемые шаги и предупреждения.
Не проверено: окно приложения (`pnpm tauri dev`) и смоук `docs/smoke/i1a.md`; настоящие vLLM и SGLang (снимки ответов собраны по исходникам и документации серверов, сверка — пункты 11–12 смоука); системное хранилище секретов на Windows и Linux (тесты — на `MemoryStore`); политика из MDM и реестра (проверены разбор примеров и `resolve`).

## Блоки кода

Код переносится как есть. Перед каждым блоком стоит HTML-комментарий `<!-- block: <вид> <id> step=<задача>.<шаг> [path=…] [part=…] [mode=…] -->`: `shell` — команды bash из корня репозитория (в том числе `scripts/port.sh` — перенос файла из Kubernetes-версии); `file` — новый файл по `path` (у Rust-файлов с тестами две части: `part=tests` — модуль тестов, `part=impl` — реализация, итоговый файл = реализация, пустая строка, модуль тестов); `patch` — патч `git diff` (применять `git apply --recount`; патч перенесённого файла — против файла Kubernetes-версии); `locales` — фрагмент словарей для `scripts/merge-locales.mjs`. Ограждения — четыре и больше обратных кавычек: внутри встречаются тройные. Блоки не перепечатывайте: их применяет скрипт ниже, по шагам, в порядке появления в плане. Патчи рассчитаны на дерево `main` @ `0f42a65` с применёнными предыдущими задачами плана.

Перенос читает Kubernetes-версию с коммита `468e08a`: `scripts/port.sh` берёт файл из `$KUBERNETES_REPO` (по умолчанию `~/projects/glafira-openshift`) командой `git show 468e08a:<путь>`, так что рабочее дерево того репозитория может быть любым.

Скрипт извлекается из этого плана один раз, до задачи 1 (план лежит в `docs/superpowers/plans/` репозитория):

```bash
awk '/^<!-- block: helper i1a-blocks.py -->$/{f=1;next} f&&/^````/{n++; if(n==2) exit; next} f' \
  docs/superpowers/plans/2026-10-29-glafira-inference-i1a.md > /tmp/i1a-blocks.py
python3 /tmp/i1a-blocks.py list | head
```

<!-- block: helper i1a-blocks.py -->
````python
#!/usr/bin/env python3
"""Блоки плана I1a: извлечение и применение по шагам.

Использование (из корня репозитория):
  python3 /tmp/i1a-blocks.py list                 — все блоки: id, вид, шаг, путь
  python3 /tmp/i1a-blocks.py get <id>             — содержимое блока в stdout
  python3 /tmp/i1a-blocks.py step <задача>.<шаг>  — применить блоки шага по порядку
План — $PLAN или docs/superpowers/plans/2026-10-29-glafira-inference-i1a.md.

Виды блоков: shell — команды bash из корня репозитория; file — файл по path (part=tests пишет модуль тестов,
part=impl ставит реализацию перед ним через пустую строку; mode — права); patch — `git apply --recount`;
locales — строки словарей для `scripts/merge-locales.mjs`.
"""
import os
import re
import subprocess
import sys

PLAN = os.environ.get("PLAN", "docs/superpowers/plans/2026-10-29-glafira-inference-i1a.md")
BLOCK = re.compile(r"<!-- block: (\w+) (\S+)((?: \w+=\S+)*) -->\n(`{4,})[\w-]*\n(.*?)\n\4\n", re.S)


def blocks():
    text = open(PLAN, encoding="utf-8").read()
    for m in BLOCK.finditer(text):
        attrs = dict(a.split("=", 1) for a in m.group(3).split())
        yield {"kind": m.group(1), "id": m.group(2), "body": m.group(5) + "\n", **attrs}


def write_file(b):
    path = b["path"]
    os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
    part = b.get("part", "whole")
    body = b["body"]
    if part == "impl" and os.path.exists(path):
        body = body + "\n" + open(path, encoding="utf-8").read()
    with open(path, "w", encoding="utf-8") as f:
        f.write(body)
    if "mode" in b:
        os.chmod(path, int(b["mode"], 8))


def apply(b):
    kind = b["kind"]
    if kind == "shell":
        subprocess.run(["bash", "-euo", "pipefail", "-c", b["body"]], check=True)
    elif kind == "file":
        write_file(b)
    elif kind == "patch":
        subprocess.run(["git", "apply", "--recount", "--whitespace=nowarn"], input=b["body"].encode(), check=True)
    elif kind == "locales":
        subprocess.run(["node", "scripts/merge-locales.mjs"], input=b["body"].encode(), check=True)
    else:
        sys.exit(f"неизвестный вид блока {kind}")


def main():
    cmd, *args = sys.argv[1:] or ["list"]
    all_blocks = list(blocks())
    if cmd == "list":
        for b in all_blocks:
            print(b["id"], b["kind"], b.get("step", ""), b.get("path", ""))
    elif cmd == "get":
        sys.stdout.write(next(b for b in all_blocks if b["id"] == args[0])["body"])
    elif cmd == "step":
        chosen = [b for b in all_blocks if b.get("step") == args[0]]
        if not chosen:
            sys.exit(f"в шаге {args[0]} нет блоков")
        for b in chosen:
            print(f"→ {b['id']} ({b['kind']}{', ' + b['path'] if 'path' in b else ''})")
            apply(b)
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
````

## Global Constraints

- Glafira ничего не меняет на серверах инференса: клиент делает только GET (`servers/client.rs`), перенаправления не выполняет, тело ответа читает с лимитом (1 МБ для JSON, 8 МБ для метрик, 256 КБ для определения типа). Тексты запросов и ответов пользователей не читаются нигде.
- Секреты — только в keychain (сервис `glafira-inference`, ключ `<id инсталляции>/<id сервера>/apiKey`); в `installations.json`, экспорте, `Debug` и текстах ошибок ключей нет. Из `/server_info` SGLang берётся только версия: в ответе бывает ключ API.
- Адреса в текстах ошибок — без `user:pass@` (`redact_url`); `user:pass@` в адресе сервера и прокси отклоняется при сохранении.
- TLS — rustls с провайдером ring (reqwest `rustls-no-provider`, `tls::ensure_crypto_provider`); заданный CA сервера заменяет системные корни (`tls_certs_only`); режим без проверки запрещает политика `allowInsecureTls = false` — при сохранении, импорте и подключении (`servers::client::ensure_allowed`). В дереве зависимостей нет `aws-lc-rs` и OpenSSL.
- Пределы: не больше 32 серверов в инсталляции; id сервера — `[A-Za-z0-9_-]{1,64}`, уникален; имена серверов уникальны без учёта регистра; таймауты — от 1000 мс (по умолчанию запрос 30 с, подключение 10 с).
- Версии: vLLM от 0.8, SGLang от 0.4; ниже — работа как `openai` с предупреждением. Имена типов в интерфейсе — «vLLM», «SGLang», «OpenAI-compatible».
- Тексты интерфейса и Rust — ключами в `src/locales/en.json` и `ru.json` (en — эталон ключей, ru — эталон смысла), строки добавляются фрагментами `node scripts/merge-locales.mjs`; в строковых литералах Rust вне тестов кириллицы нет; совпадения ru с en — в `docs/i18n/same-as-en.json`; подсказки — до 200 символов без тегов.
- Код фронтенда — для WebKit macOS 11 (Safari 14): `pnpm build` проверяет бандл (`scripts/check-dist-compat.mjs`), `crypto.randomUUID` и lookbehind недоступны.
- Проверки после каждой задачи (что применимо): `cargo test --manifest-path src-tauri/Cargo.toml --lib`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo fmt --manifest-path src-tauri/Cargo.toml --check`; `pnpm test`, `pnpm exec tsc --noEmit`, `pnpm i18n:check`, `pnpm test:scripts`, `pnpm build`.
- Тесты Rust не ходят в сеть, кроме локальных сокетов: серверы — `servers::testing::TestServer` (HTTP и HTTPS на `127.0.0.1`), keychain — `MemoryStore`. Тесты фронтенда мокают `api` (`vi.mock("../../api/client", …)`) и плагины Tauri.
- `src-tauri/Cargo.lock` и `pnpm-lock.yaml` обновляют cargo и pnpm; они коммитятся.
- Комментарии кода и документы — на русском; **сообщения коммитов — на английском** (Conventional Commits: `feat(storage): …`, `feat(servers): …`, `feat(ui): …`, `docs: …`). Коммит — `git add` по путям задачи (не `git add -A` и не `-a`), `git commit -s`, последняя строка сообщения — `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Push — только с согласия пользователя.

## Структура файлов

| Файл | Ответственность |
|---|---|
| `scripts/port.sh`, `PORTED.md`, каркас Tauri, Vite, Vitest, юридические документы | перенос из Kubernetes-версии, сборка |
| `src-tauri/src/{i18n,error,secrets,logging,sync,tls,dock_name}.rs`, `storage/fs.rs` | основа: тексты, ошибки, секреты, атомарная запись, журнал |
| `src-tauri/src/{policy/,settings.rs,state.rs}`, `commands/{mod,settings}.rs`, `docs/admin/*` | политика `allowInsecureTls`, язык, общее состояние |
| `src-tauri/src/storage/{profile,installations,tree,exchange}.rs`, `commands/folders.rs` | инсталляция и серверы, хранилище, папки, обмен |
| `src-tauri/src/servers/{client,errors,testing}.rs`, `servers/testdata/{ca,server,server-key}.pem` | HTTP-клиент сервера, ошибки, тестовый HTTP/HTTPS-сервер |
| `src-tauri/src/servers/{detect,probe}.rs`, `servers/testdata/<сервер>-<версия>/` | тип, версия и модели по снимку ответов |
| `src-tauri/src/servers/pool.rs`, `state.rs` | подключённые инсталляции |
| `src-tauri/src/servers/connectivity.rs` | пошаговая проверка сервера |
| `src-tauri/src/commands/{installations,app}.rs`, `lib.rs` | команды Tauri |
| `src/api/`, `src/i18n/`, `src/lib/`, `src/components/common/`, `scripts/{i18n-check,merge-locales,changelog}.mjs`, словари | основа фронтенда |
| `src/state/workspace.ts`, `src/components/installations/{profile.ts,InstallationForm.tsx,InstallationFormTab.tsx,tree/folderTree.ts}`, `src/app.css` | форма инсталляции |
| `src/components/installations/InstallationPanel.tsx` | вкладка инсталляции |
| `src/guide/`, `src/components/{help,settings,about}/`, `docs/guide/{en,ru}/` | справка, настройки, «О программе» |
| `src/components/installations/tree/`, `src/components/{Layout,Sidebar,TabContent}.tsx`, `src/lib/{useAppMenu,useHelpShortcut,tabActive}.ts` | дерево, раскладка, меню |
| `scripts/mock-server.mjs`, `.github/` | мок-сервер, CI |
| `README.md`, `docs/i18n/`, `docs/smoke/i1a.md`, спека | документация |

### Task 1: Каркас репозитория, инструменты переноса и сборки

**Files:**
- Port: `.gitattributes`, `.gitignore` (с правками), `CONTRIBUTING.md` (с правками), `LICENSE`, `SECURITY.md` (с правками), `TRADEMARK.md` (с правками), `deny.toml`, `index.html` (с правками), `pnpm-workspace.yaml`, `public/app-icon.svg`, `scripts/check-dist-compat.mjs`, `scripts/check-dist-compat.test.mjs`, `src-tauri/.gitignore`, `src-tauri/build.rs`, `src-tauri/capabilities/default.json`, `src-tauri/icons/128x128.png`, `src-tauri/icons/128x128@2x.png`, `src-tauri/icons/32x32.png`, `src-tauri/icons/icon.icns`, `src-tauri/icons/icon.ico`, `src-tauri/icons/icon.png`, `src/polyfills.test.ts`, `src/polyfills.ts`, `src/styles.css`, `src/vite-env.d.ts`, `tsconfig.json`, `tsconfig.node.json`, `vite.config.ts` (с правками), `vitest.config.ts`
- Create (по Kubernetes-версии): `NOTICE` ← `NOTICE`
- Create: `PORTED.md`, `package.json`, `scripts/port.sh`, `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`, `src-tauri/src/main.rs`, `src-tauri/src/product.rs`, `src-tauri/tauri.conf.json`, `src/App.test.tsx`, `src/App.tsx`, `src/main.tsx`, `src/test/setup.ts`
- Generated: `pnpm-lock.yaml`, `src-tauri/Cargo.lock` (cargo, pnpm)

**Interfaces:**
- Consumes: ничего: репозиторий содержит только спеку.
- Produces: `glafira_inference_lib::product::{PRODUCT_NAME = "Glafira for LLM Inference", IDENTIFIER = "io.github.vvnovg.glafira.inference", KEYCHAIN_SERVICE = "glafira-inference", POLICY_VENDOR = "Glafira", POLICY_PRODUCT = "Inference", CLIENT_NAME = "glafira-inference"}`; бинарь `glafira-inference`, библиотека `glafira_inference_lib`; `scripts/port.sh <путь в glafira-kubernetes> <путь здесь>` копирует файл с коммита `468e08a` (`$KUBERNETES_REPO`, по умолчанию `~/projects/glafira-openshift`); `PORTED.md`; dev-сервер Vite на порту 1440.

Пустой Tauri-проект с именем продукта и перенос инструментов Kubernetes-версии: TypeScript, Vite, Vitest, проверка совместимости бандла с WebKit macOS 11, полифилы, значки, юридические документы с оговоркой о товарных знаках vLLM, SGLang, LiteLLM и NVIDIA, `deny.toml`. Значок пока перенесён из Kubernetes-версии; свой рисуется отдельным коммитом, как было там (`eede48f`). `scripts/port.sh` идёт первым блоком: им переносится всё остальное. `PORTED.md` — таблица перенесённых файлов: исправление общей части нужно перенести в другие проекты семейства. Порт dev-сервера — 1440: 1420 и 1430 заняты Kafka- и Kubernetes-версиями.

- [ ] **Step 1: Перенос из Kubernetes-версии.**

Файлы копируются с коммита `468e08a` репозитория `~/projects/glafira-openshift` (`$KUBERNETES_REPO`) и правятся патчами — против перенесённого файла. Строки о них в `PORTED.md` добавляет шаг реализации.

<!-- block: file t01-scripts-port-sh step=1.1 path=scripts/port.sh mode=755 -->
````bash
#!/usr/bin/env bash
# Переносит файл из Glafira for Kubernetes с зафиксированного коммита (строку о нём в PORTED.md добавляет задача плана).
# Использование: scripts/port.sh <путь в glafira-kubernetes> <путь здесь>
# Репозиторий Kubernetes-версии — $KUBERNETES_REPO (по умолчанию ~/projects/glafira-openshift).
set -euo pipefail
repo="${KUBERNETES_REPO:-$HOME/projects/glafira-openshift}"
commit=468e08a
src="$1"
dst="$2"
mkdir -p "$(dirname "$dst")"
git -C "$repo" show "$commit:$src" > "$dst"
````

<!-- block: shell t01-port step=1.1 -->
````bash
scripts/port.sh .gitattributes .gitattributes
scripts/port.sh .gitignore .gitignore
scripts/port.sh LICENSE LICENSE
scripts/port.sh index.html index.html
scripts/port.sh pnpm-workspace.yaml pnpm-workspace.yaml
scripts/port.sh public/app-icon.svg public/app-icon.svg
scripts/port.sh scripts/check-dist-compat.mjs scripts/check-dist-compat.mjs
scripts/port.sh scripts/check-dist-compat.test.mjs scripts/check-dist-compat.test.mjs
scripts/port.sh src-tauri/.gitignore src-tauri/.gitignore
scripts/port.sh src-tauri/build.rs src-tauri/build.rs
scripts/port.sh src-tauri/capabilities/default.json src-tauri/capabilities/default.json
scripts/port.sh src-tauri/icons/32x32.png src-tauri/icons/32x32.png
scripts/port.sh src-tauri/icons/128x128.png src-tauri/icons/128x128.png
scripts/port.sh src-tauri/icons/128x128@2x.png src-tauri/icons/128x128@2x.png
scripts/port.sh src-tauri/icons/icon.icns src-tauri/icons/icon.icns
scripts/port.sh src-tauri/icons/icon.ico src-tauri/icons/icon.ico
scripts/port.sh src-tauri/icons/icon.png src-tauri/icons/icon.png
scripts/port.sh src/polyfills.ts src/polyfills.ts
scripts/port.sh src/polyfills.test.ts src/polyfills.test.ts
scripts/port.sh src/styles.css src/styles.css
scripts/port.sh src/vite-env.d.ts src/vite-env.d.ts
scripts/port.sh tsconfig.json tsconfig.json
scripts/port.sh tsconfig.node.json tsconfig.node.json
scripts/port.sh vite.config.ts vite.config.ts
scripts/port.sh vitest.config.ts vitest.config.ts
scripts/port.sh TRADEMARK.md TRADEMARK.md
scripts/port.sh SECURITY.md SECURITY.md
scripts/port.sh CONTRIBUTING.md CONTRIBUTING.md
scripts/port.sh deny.toml deny.toml
````

<!-- block: patch t01-port-gitignore step=1.1 -->
````diff
diff --git a/.gitignore b/.gitignore
--- a/.gitignore
+++ b/.gitignore
@@ -24,11 +24,5 @@ dist-ssr
 *.sw?
 
 
-# Экспорты подключений (без паролей, но с адресами реальных кластеров)
-*.glafira-kubernetes-connections.json
-
-# Стенд kind: kubeconfig с ключом администратора и токеном
-dev/kind/out/
-
-# Сборка крейтов MCP-шлюза
-/crates/*/target/
+# Экспорты инсталляций (без ключей, но с адресами реальных серверов)
+*.glafira-inference-installations.json
````

<!-- block: patch t01-port-index-html step=1.1 -->
````diff
diff --git a/index.html b/index.html
--- a/index.html
+++ b/index.html
@@ -4,7 +4,7 @@
     <meta charset="UTF-8" />
     <link rel="icon" type="image/svg+xml" href="/app-icon.svg" />
     <meta name="viewport" content="width=device-width, initial-scale=1.0" />
-    <title>Glafira for Kubernetes</title>
+    <title>Glafira for LLM Inference</title>
   </head>
 
   <body spellcheck="false" autocapitalize="off" autocorrect="off">
````

<!-- block: patch t01-port-vite-config-ts step=1.1 -->
````diff
diff --git a/vite.config.ts b/vite.config.ts
--- a/vite.config.ts
+++ b/vite.config.ts
@@ -18,14 +18,15 @@ export default defineConfig(() => ({
   clearScreen: false,
   // 2. tauri expects a fixed port, fail if that port is not available
   server: {
-    port: 1420,
+    // 1420 и 1430 заняты dev-серверами Kafka- и Kubernetes-версий: приложения семейства можно запускать рядом.
+    port: 1440,
     strictPort: true,
     host: host || false,
     hmr: host
       ? {
           protocol: "ws",
           host,
-          port: 1421,
+          port: 1441,
         }
       : undefined,
     watch: {
````

<!-- block: patch t01-port-trademark-md step=1.1 -->
````diff
diff --git a/TRADEMARK.md b/TRADEMARK.md
--- a/TRADEMARK.md
+++ b/TRADEMARK.md
@@ -1,6 +1,6 @@
 # Trademarks
 
-"Glafira", "Glafira for Kubernetes" and the Glafira application icon are trademarks of Viacheslav Novgorodtsev.
+"Glafira", "Glafira for LLM Inference" and the Glafira application icon are trademarks of Viacheslav Novgorodtsev.
 The Apache License 2.0 that covers the source code does not grant any rights to these names or to the icon
 (see section 6 of the license).
 
@@ -16,6 +16,5 @@ You may not, without written permission:
   give your fork its own name and icon;
 - use the name or the icon in a way that suggests the author endorses your product or service.
 
-Kubernetes is a registered trademark of The Linux Foundation. OpenShift is a trademark of Red Hat, Inc. Platform V
-and DropApp are trademarks of their respective owners. This project is not affiliated with The Linux Foundation,
-Red Hat or SberTech.
+vLLM, SGLang, LiteLLM, NVIDIA and other product names are trademarks of their respective owners. This project is not
+affiliated with them.
````

<!-- block: patch t01-port-security-md step=1.1 -->
````diff
diff --git a/SECURITY.md b/SECURITY.md
--- a/SECURITY.md
+++ b/SECURITY.md
@@ -2,7 +2,7 @@
 
 ## Supported versions
 
-Only the latest release on https://github.com/vvnovg/glafira-kubernetes-releases/releases receives fixes.
+Only the latest release on https://github.com/vvnovg/glafira-inference-releases/releases receives fixes.
 
 ## Reporting a vulnerability
 
@@ -15,6 +15,7 @@ credit to the reporter, unless you ask otherwise.
 
 ## What matters most
 
-Glafira connects to Kubernetes clusters, OpenShift and Platform V DropApp among them, with the user's credentials
-(tokens, client certificates, browser login, exec plugins). Reports about credential handling (system secret store,
-logs), the OAuth login flow, exec plugins and the administrator policy are especially welcome.
+Glafira connects to corporate LLM inference (vLLM, SGLang and other OpenAI-compatible servers) with the user's
+credentials (API keys, and in later versions Kubernetes and SSH access). Reports about credential handling (system
+secret store, logs), anything that could expose the texts of user requests, and the administrator policy are especially
+welcome.
````

<!-- block: patch t01-port-contributing-md step=1.1 -->
````diff
diff --git a/CONTRIBUTING.md b/CONTRIBUTING.md
--- a/CONTRIBUTING.md
+++ b/CONTRIBUTING.md
@@ -1,6 +1,6 @@
 # Contributing
 
-Thank you for your interest in Glafira for Kubernetes.
+Thank you for your interest in Glafira for LLM Inference.
 
 ## Before you start
 
@@ -28,10 +28,10 @@ Every new string goes to all dictionaries in `src/locales/` at once (see `docs/i
 `docs/i18n/glossary.md`); `pnpm i18n:check` enforces it. Changes to the user guide go to every language in
 `docs/guide/`.
 
-## Shared code with Glafira for Apache Kafka
+## Shared code with Glafira for Kubernetes
 
-Files listed in `PORTED.md` were copied from Glafira for Apache Kafka. A fix in such a file is usually needed in
-both projects: mention it in the pull request.
+Files listed in `PORTED.md` were copied from Glafira for Kubernetes (which in turn took much of them from Glafira for
+Apache Kafka). A fix in such a file is usually needed in all of these projects: mention it in the pull request.
 
 ## Developer Certificate of Origin
 
````

- [ ] **Step 2: Тесты.**

<!-- block: file t01-src-tauri-src-product-rs-tests step=1.2 path=src-tauri/src/product.rs part=tests -->
````rust
#[cfg(test)]
mod tests {
    #[test]
    fn matches_tauri_conf() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(conf["identifier"], super::IDENTIFIER);
        assert_eq!(conf["productName"], super::PRODUCT_NAME);
        assert_eq!(conf["app"]["windows"][0]["title"], super::PRODUCT_NAME);
        assert_eq!(conf["mainBinaryName"], super::CLIENT_NAME);
    }
}
````

<!-- block: file t01-src-app-test-tsx step=1.2 path=src/App.test.tsx -->
````tsx
import { render, screen } from "@testing-library/react";
import App from "./App";

test("показывает имя продукта", () => {
  render(<App />);
  expect(screen.getByText("Glafira for LLM Inference")).toBeInTheDocument();
});
````

<!-- block: file t01-src-test-setup-ts step=1.2 path=src/test/setup.ts -->
````ts
import "@testing-library/jest-dom/vitest";
````

- [ ] **Step 3: Реализация.**

<!-- block: file t01-notice step=1.3 path=NOTICE -->
````text
Glafira for LLM Inference
Copyright 2026 Viacheslav Novgorodtsev

This product includes software developed by third parties under their own
licenses; see THIRD_PARTY.md for the list.

vLLM, SGLang, LiteLLM, NVIDIA and other product names are trademarks of their
respective owners. This project is not affiliated with them.
````

<!-- block: file t01-ported-md step=1.3 path=PORTED.md -->
````markdown
# Перенесено из Glafira for Kubernetes

Файлы скопированы из `vvnovg/glafira-kubernetes` (локально `~/projects/glafira-openshift`) скриптом `scripts/port.sh`.
Многие из них Kubernetes-версия сама взяла из Glafira for Apache Kafka (`vvnovg/glafira-kafka`, её `PORTED.md`).
Исправление общей части нужно перенести в другие проекты; строка показывает, откуда взят файл.

| Здесь | В glafira-kubernetes | Коммит | Правки |
|---|---|---|---|
| `.gitattributes` | `.gitattributes` | `468e08a` | как есть |
| `.gitignore` | `.gitignore` | `468e08a` | адаптирован: шаблон файлов экспорта инсталляций, без стенда kind и крейтов MCP |
| `LICENSE` | `LICENSE` | `468e08a` | как есть |
| `index.html` | `index.html` | `468e08a` | адаптирован: заголовок |
| `pnpm-workspace.yaml` | `pnpm-workspace.yaml` | `468e08a` | как есть |
| `public/app-icon.svg` | `public/app-icon.svg` | `468e08a` | как есть (значок Kubernetes-версии до своего значка) |
| `scripts/check-dist-compat.mjs` | `scripts/check-dist-compat.mjs` | `468e08a` | как есть |
| `scripts/check-dist-compat.test.mjs` | `scripts/check-dist-compat.test.mjs` | `468e08a` | как есть |
| `src-tauri/.gitignore` | `src-tauri/.gitignore` | `468e08a` | как есть |
| `src-tauri/build.rs` | `src-tauri/build.rs` | `468e08a` | как есть |
| `src-tauri/capabilities/default.json` | `src-tauri/capabilities/default.json` | `468e08a` | как есть |
| `src-tauri/icons/32x32.png` | `src-tauri/icons/32x32.png` | `468e08a` | как есть (значок Kubernetes-версии до своего значка) |
| `src-tauri/icons/128x128.png` | `src-tauri/icons/128x128.png` | `468e08a` | как есть (значок Kubernetes-версии до своего значка) |
| `src-tauri/icons/128x128@2x.png` | `src-tauri/icons/128x128@2x.png` | `468e08a` | как есть (значок Kubernetes-версии до своего значка) |
| `src-tauri/icons/icon.icns` | `src-tauri/icons/icon.icns` | `468e08a` | как есть (значок Kubernetes-версии до своего значка) |
| `src-tauri/icons/icon.ico` | `src-tauri/icons/icon.ico` | `468e08a` | как есть (значок Kubernetes-версии до своего значка) |
| `src-tauri/icons/icon.png` | `src-tauri/icons/icon.png` | `468e08a` | как есть (значок Kubernetes-версии до своего значка) |
| `src/polyfills.ts` | `src/polyfills.ts` | `468e08a` | как есть |
| `src/polyfills.test.ts` | `src/polyfills.test.ts` | `468e08a` | как есть |
| `src/styles.css` | `src/styles.css` | `468e08a` | как есть |
| `src/vite-env.d.ts` | `src/vite-env.d.ts` | `468e08a` | как есть |
| `tsconfig.json` | `tsconfig.json` | `468e08a` | как есть |
| `tsconfig.node.json` | `tsconfig.node.json` | `468e08a` | как есть |
| `vite.config.ts` | `vite.config.ts` | `468e08a` | адаптирован: порт 1440 (1420 и 1430 — у Kafka- и Kubernetes-версий) |
| `vitest.config.ts` | `vitest.config.ts` | `468e08a` | как есть |
| `NOTICE` | `NOTICE` | `468e08a` | адаптирован: имя продукта и товарные знаки |
| `TRADEMARK.md` | `TRADEMARK.md` | `468e08a` | адаптирован: имя продукта и товарные знаки |
| `SECURITY.md` | `SECURITY.md` | `468e08a` | адаптирован: имя продукта, адрес релизов, что важно для безопасности |
| `CONTRIBUTING.md` | `CONTRIBUTING.md` | `468e08a` | адаптирован: имя продукта, общий код с Kubernetes-версией |
| `deny.toml` | `deny.toml` | `468e08a` | как есть |
````

<!-- block: file t01-package-json step=1.3 path=package.json -->
````json
{
  "name": "glafira-inference",
  "private": true,
  "version": "0.1.0",
  "license": "Apache-2.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build && node scripts/check-dist-compat.mjs dist/assets",
    "check:dist": "node scripts/check-dist-compat.mjs dist/assets",
    "preview": "vite preview",
    "tauri": "tauri",
    "test": "vitest run",
    "test:scripts": "node --test \"scripts/*.test.mjs\""
  },
  "dependencies": {
    "@floating-ui/react": "^0.27.20",
    "@tanstack/react-query": "^5.104.2",
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-dialog": "^2.8.2",
    "@tauri-apps/plugin-opener": "^2",
    "i18next": "^25.10.10",
    "react": "^19.1.0",
    "react-dom": "^19.1.0",
    "react-i18next": "^15.7.4",
    "react-markdown": "9.1.0",
    "zustand": "^5.0.15"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2",
    "@testing-library/jest-dom": "^7.0.1",
    "@testing-library/react": "^16.3.3",
    "@testing-library/user-event": "^14.6.8",
    "@types/react": "^19.1.8",
    "@types/react-dom": "^19.1.6",
    "@vitejs/plugin-react": "^6.0.2",
    "jsdom": "^30.1.2",
    "typescript": "~6.0.3",
    "vite": "^8.0.16",
    "vitest": "^3.2.7"
  }
}
````

<!-- block: file t01-src-tauri-cargo-toml step=1.3 path=src-tauri/Cargo.toml -->
````toml
[package]
name = "glafira-inference"
version = "0.1.0"
description = "Glafira for LLM Inference — desktop-приложение для аудита и диагностики корпоративного инференса LLM"
authors = ["Glafira for LLM Inference contributors"]
edition = "2024"
license = "Apache-2.0"
repository = "https://github.com/vvnovg/glafira-inference"

[lib]
# Суффикс _lib нужен, чтобы имя библиотеки не совпало с именем бинаря (Windows, cargo#8519).
name = "glafira_inference_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
tauri-plugin-dialog = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
tempfile = "3"

[profile.release]
codegen-units = 1
lto = true
opt-level = 3
panic = "abort"
strip = true
````

<!-- block: file t01-src-tauri-src-lib-rs step=1.3 path=src-tauri/src/lib.rs -->
````rust
pub mod product;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
````

<!-- block: file t01-src-tauri-src-main-rs step=1.3 path=src-tauri/src/main.rs -->
````rust
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    glafira_inference_lib::run()
}
````

<!-- block: file t01-src-tauri-src-product-rs-impl step=1.3 path=src-tauri/src/product.rs part=impl -->
````rust
//! Имя продукта и идентификаторы — в одном месте. Переименование меняет только этот файл
//! и `tauri.conf.json`; тест ниже следит, чтобы они совпадали.

pub const PRODUCT_NAME: &str = "Glafira for LLM Inference";
pub const IDENTIFIER: &str = "io.github.vvnovg.glafira.inference";
pub const KEYCHAIN_SERVICE: &str = "glafira-inference";
pub const POLICY_VENDOR: &str = "Glafira";
pub const POLICY_PRODUCT: &str = "Inference";
/// Имя, под которым Glafira видна серверам инференса: `User-Agent` запросов.
pub const CLIENT_NAME: &str = "glafira-inference";
````

<!-- block: file t01-src-tauri-tauri-conf-json step=1.3 path=src-tauri/tauri.conf.json -->
````json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Glafira for LLM Inference",
  "mainBinaryName": "glafira-inference",
  "version": "0.1.0",
  "identifier": "io.github.vvnovg.glafira.inference",
  "build": {
    "beforeDevCommand": "pnpm dev",
    "devUrl": "http://localhost:1440",
    "beforeBuildCommand": "pnpm build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "Glafira for LLM Inference",
        "width": 1400,
        "height": 900,
        "minWidth": 900,
        "minHeight": 600,
        "dragDropEnabled": false
      }
    ],
    "security": {
      "csp": "default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src ipc: http://ipc.localhost"
    }
  },
  "bundle": {
    "active": true,
    "targets": ["app", "dmg", "msi", "deb", "rpm", "appimage"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "publisher": "Glafira for LLM Inference",
    "category": "DeveloperTool",
    "shortDescription": "Glafira for LLM Inference — desktop client for auditing and diagnosing corporate LLM inference",
    "copyright": "© 2026 Viacheslav Novgorodtsev",
    "linux": {
      "deb": { "depends": ["libsecret-1-0"] },
      "rpm": { "depends": ["libsecret"] }
    },
    "macOS": { "minimumSystemVersion": "11.0" },
    "windows": {
      "wix": {
        "upgradeCode": "1ff11da8-a39b-44bb-9338-b6e77c70f9e3",
        "language": ["en-US"]
      }
    }
  }
}
````

<!-- block: file t01-src-app-tsx step=1.3 path=src/App.tsx -->
````tsx
export default function App() {
  return <main className="page">Glafira for LLM Inference</main>;
}
````

<!-- block: file t01-src-main-tsx step=1.3 path=src/main.tsx -->
````tsx
import "./polyfills";
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
````

<!-- block: shell t01-install step=1.3 -->
````bash
pnpm install
````

- [ ] **Step 4: Проверки.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib`  
Expected: PASS — `test result: ok. 1 passed; 0 failed`

Run: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`  
Expected: PASS — без предупреждений

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml --check`  
Expected: PASS — без изменений

Run: `cargo deny --manifest-path src-tauri/Cargo.toml --config deny.toml check licenses advisories`  
Expected: PASS — `advisories ok, licenses ok`

Run: `cargo tree --manifest-path src-tauri/Cargo.toml -i aws-lc-rs`  
Expected: ошибка `did not match any packages` — `aws-lc-rs` в дереве нет (то же для `openssl-sys`)

Run: `pnpm test`  
Expected: PASS — `Test Files 2 passed`, `Tests 4 passed`

Run: `pnpm exec tsc --noEmit`  
Expected: PASS — без ошибок

Run: `pnpm test:scripts`  
Expected: PASS — `# pass 9`, `# fail 0`

Run: `pnpm build`  
Expected: PASS — `✓ dist/assets: чанки совместимы с WebKit macOS 11+`

- [ ] **Step 5: Commit.**

```bash
git add \
  .gitattributes \
  .gitignore \
  CONTRIBUTING.md \
  LICENSE \
  NOTICE \
  PORTED.md \
  SECURITY.md \
  TRADEMARK.md \
  deny.toml \
  index.html \
  package.json \
  pnpm-lock.yaml \
  pnpm-workspace.yaml \
  public/app-icon.svg \
  scripts/check-dist-compat.mjs \
  scripts/check-dist-compat.test.mjs \
  scripts/port.sh \
  src-tauri/.gitignore \
  src-tauri/Cargo.lock \
  src-tauri/Cargo.toml \
  src-tauri/build.rs \
  src-tauri/capabilities/default.json \
  src-tauri/icons/128x128.png \
  src-tauri/icons/128x128@2x.png \
  src-tauri/icons/32x32.png \
  src-tauri/icons/icon.icns \
  src-tauri/icons/icon.ico \
  src-tauri/icons/icon.png \
  src-tauri/src/lib.rs \
  src-tauri/src/main.rs \
  src-tauri/src/product.rs \
  src-tauri/tauri.conf.json \
  src/App.test.tsx \
  src/App.tsx \
  src/main.tsx \
  src/polyfills.test.ts \
  src/polyfills.ts \
  src/styles.css \
  src/test/setup.ts \
  src/vite-env.d.ts \
  tsconfig.json \
  tsconfig.node.json \
  vite.config.ts \
  vitest.config.ts
git commit -s -m "chore: scaffold the Glafira for LLM Inference Tauri app, port tooling from the Kubernetes edition" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 2: Основа Rust: тексты, ошибки, атомарные файлы, секреты, журнал

**Files:**
- Port: `src-tauri/src/dock_name.rs` (с правками), `src-tauri/src/error.rs`, `src-tauri/src/i18n.rs`, `src-tauri/src/logging.rs`, `src-tauri/src/secrets.rs` (с правками), `src-tauri/src/storage/fs.rs`, `src-tauri/src/sync.rs`, `src-tauri/src/tls.rs`
- Create: `src-tauri/src/storage/mod.rs`
- Modify: `PORTED.md`, `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`
- Generated: `src-tauri/Cargo.lock` (cargo, pnpm)

**Interfaces:**
- Consumes: задача 1: `product`.
- Produces: `i18n::{LocalizedText, ltext!, app_err!}`, `error::{ErrorCode, AppError, AppResult}`, `storage::fs::{atomic_write, read_json, read_json_or_quarantine, write_json}`, `secrets::{SecretStore, KeyringStore, MemoryStore, OverlayStore, API_KEY = "apiKey", SERVER_SECRET_FIELDS, secret_key(installation_id, component_id, field) -> "<инсталляция>/<компонент>/<поле>"}`, `logging::init`, `sync::{MutexExt, RwLockExt}`, `tls::ensure_crypto_provider` (rustls с ring).

Модули переносятся как есть, кроме `secrets.rs`: у сервера инференса один секрет — ключ API, и ключ записи keychain строится из инсталляции, компонента и поля (спец. §5.1). `lib.rs` подключает журнал и имя процесса в Dock.

- [ ] **Step 1: Перенос из Kubernetes-версии.**

Файлы копируются с коммита `468e08a` репозитория `~/projects/glafira-openshift` (`$KUBERNETES_REPO`) и правятся патчами — против перенесённого файла. Строки о них в `PORTED.md` добавляет шаг реализации.

<!-- block: shell t02-port step=2.1 -->
````bash
scripts/port.sh src-tauri/src/i18n.rs src-tauri/src/i18n.rs
scripts/port.sh src-tauri/src/error.rs src-tauri/src/error.rs
scripts/port.sh src-tauri/src/storage/fs.rs src-tauri/src/storage/fs.rs
scripts/port.sh src-tauri/src/secrets.rs src-tauri/src/secrets.rs
scripts/port.sh src-tauri/src/logging.rs src-tauri/src/logging.rs
scripts/port.sh src-tauri/src/sync.rs src-tauri/src/sync.rs
scripts/port.sh src-tauri/src/dock_name.rs src-tauri/src/dock_name.rs
scripts/port.sh src-tauri/src/tls.rs src-tauri/src/tls.rs
````

<!-- block: patch t02-port-src-tauri-src-secrets-rs step=2.1 -->
````diff
diff --git a/src-tauri/src/secrets.rs b/src-tauri/src/secrets.rs
--- a/src-tauri/src/secrets.rs
+++ b/src-tauri/src/secrets.rs
@@ -10,16 +10,15 @@ pub trait SecretStore: Send + Sync {
     fn delete(&self, key: &str) -> AppResult<()>;
 }
 
-/// Bearer-токен подключения: вставленный вручную, полученный входом через браузер или из kubeconfig.
-pub const TOKEN: &str = "token";
-/// Ключ клиентского сертификата (PEM).
-pub const CLIENT_KEY: &str = "clientKey";
+/// Ключ API сервера инференса: уходит заголовком `Authorization: Bearer` (спец. §5.2).
+pub const API_KEY: &str = "apiKey";
 
-/// Все секреты подключения в keychain (для удаления).
-pub const CONNECTION_SECRET_FIELDS: [&str; 2] = [TOKEN, CLIENT_KEY];
+/// Все секреты сервера в keychain (для удаления).
+pub const SERVER_SECRET_FIELDS: [&str; 1] = [API_KEY];
 
-pub fn secret_key(connection_id: &str, field: &str) -> String {
-    format!("{connection_id}/{field}")
+/// Ключ записи keychain: `<id инсталляции>/<id компонента>/<поле>` (спец. §5.1).
+pub fn secret_key(installation_id: &str, component_id: &str, field: &str) -> String {
+    format!("{installation_id}/{component_id}/{field}")
 }
 
 /// OS keychain: macOS Keychain, Windows Credential Manager, Secret Service.
@@ -143,8 +142,7 @@ mod tests {
 
     #[test]
     fn key_format() {
-        assert_eq!(secret_key("c1", TOKEN), "c1/token");
-        assert_eq!(secret_key("c1", CLIENT_KEY), "c1/clientKey");
+        assert_eq!(secret_key("i1", "s1", API_KEY), "i1/s1/apiKey");
     }
 
     #[test]
````

<!-- block: patch t02-port-src-tauri-src-dock-name-rs step=2.1 -->
````diff
diff --git a/src-tauri/src/dock_name.rs b/src-tauri/src/dock_name.rs
--- a/src-tauri/src/dock_name.rs
+++ b/src-tauri/src/dock_name.rs
@@ -1,5 +1,5 @@
 //! Имя приложения в Dock и переключателе ⌘Tab при `tauri dev`. Без бандла `.app`
-//! LaunchServices показывает имя исполняемого файла (`glafira-kubernetes`), а вшитый Info.plist
+//! LaunchServices показывает имя исполняемого файла (`glafira-inference`), а вшитый Info.plist
 //! с `CFBundleName` не учитывает. Поэтому в debug-сборке имя задаётся вызовом LaunchServices —
 //! так же переименовывает процессы Chromium. В релизе имя берётся из бандла, вызов не нужен.
 
````

- [ ] **Step 2: Реализация.**

<!-- block: file t02-src-tauri-src-storage-mod-rs step=2.2 path=src-tauri/src/storage/mod.rs -->
````rust
pub mod fs;
````

<!-- block: patch t02-src step=2.2 -->
````diff
diff --git a/PORTED.md b/PORTED.md
index 57805e5..0e4222c 100644
--- a/PORTED.md
+++ b/PORTED.md
@@ -36,3 +36,11 @@
 | `SECURITY.md` | `SECURITY.md` | `468e08a` | адаптирован: имя продукта, адрес релизов, что важно для безопасности |
 | `CONTRIBUTING.md` | `CONTRIBUTING.md` | `468e08a` | адаптирован: имя продукта, общий код с Kubernetes-версией |
 | `deny.toml` | `deny.toml` | `468e08a` | как есть |
+| `src-tauri/src/i18n.rs` | `src-tauri/src/i18n.rs` | `468e08a` | как есть |
+| `src-tauri/src/error.rs` | `src-tauri/src/error.rs` | `468e08a` | как есть |
+| `src-tauri/src/storage/fs.rs` | `src-tauri/src/storage/fs.rs` | `468e08a` | как есть |
+| `src-tauri/src/secrets.rs` | `src-tauri/src/secrets.rs` | `468e08a` | адаптирован: ключ API сервера, ключ keychain из инсталляции, компонента и поля |
+| `src-tauri/src/logging.rs` | `src-tauri/src/logging.rs` | `468e08a` | как есть |
+| `src-tauri/src/sync.rs` | `src-tauri/src/sync.rs` | `468e08a` | как есть |
+| `src-tauri/src/dock_name.rs` | `src-tauri/src/dock_name.rs` | `468e08a` | адаптирован: имя бинаря |
+| `src-tauri/src/tls.rs` | `src-tauri/src/tls.rs` | `468e08a` | как есть |
diff --git a/src-tauri/Cargo.toml b/src-tauri/Cargo.toml
index b9928d0..f1cb6ed 100644
--- a/src-tauri/Cargo.toml
+++ b/src-tauri/Cargo.toml
@@ -21,6 +21,11 @@ tauri-plugin-opener = "2"
 tauri-plugin-dialog = "2"
 serde = { version = "1", features = ["derive"] }
 serde_json = "1"
+thiserror = "2"
+tracing = "0.1"
+tracing-subscriber = { version = "0.3", features = ["env-filter", "registry", "fmt"] }
+keyring = { version = "3", features = ["apple-native", "windows-native", "sync-secret-service"] }
+rustls = { version = "0.23", default-features = false, features = ["ring", "std", "tls12"] }
 
 [dev-dependencies]
 tempfile = "3"
diff --git a/src-tauri/src/lib.rs b/src-tauri/src/lib.rs
index cf62e8b..5bde940 100644
--- a/src-tauri/src/lib.rs
+++ b/src-tauri/src/lib.rs
@@ -1,10 +1,30 @@
+#[macro_use]
+pub mod i18n;
+mod dock_name;
+pub mod error;
+pub mod logging;
 pub mod product;
+pub mod secrets;
+pub mod storage;
+pub mod sync;
+pub mod tls;
+
+use tauri::Manager;
 
 #[cfg_attr(mobile, tauri::mobile_entry_point)]
 pub fn run() {
+    crate::tls::ensure_crypto_provider();
     tauri::Builder::default()
         .plugin(tauri_plugin_opener::init())
         .plugin(tauri_plugin_dialog::init())
+        .setup(|app| {
+            crate::logging::init(&app.path().app_log_dir()?);
+            tracing::info!(version = env!("CARGO_PKG_VERSION"), "starting");
+            if !crate::dock_name::set(crate::product::PRODUCT_NAME) {
+                tracing::warn!("dock name was not set: LaunchServices refused");
+            }
+            Ok(())
+        })
         .run(tauri::generate_context!())
         .expect("error while building tauri application");
 }
````

- [ ] **Step 3: Проверки.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib`  
Expected: PASS — `test result: ok. 20 passed; 0 failed`

Run: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`  
Expected: PASS — без предупреждений

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml --check`  
Expected: PASS — без изменений

Run: `cargo deny --manifest-path src-tauri/Cargo.toml --config deny.toml check licenses advisories`  
Expected: PASS — `advisories ok, licenses ok`

Run: `cargo tree --manifest-path src-tauri/Cargo.toml -i aws-lc-rs`  
Expected: ошибка `did not match any packages` — `aws-lc-rs` в дереве нет (то же для `openssl-sys`)

- [ ] **Step 4: Commit.**

```bash
git add \
  PORTED.md \
  src-tauri/Cargo.lock \
  src-tauri/Cargo.toml \
  src-tauri/src/dock_name.rs \
  src-tauri/src/error.rs \
  src-tauri/src/i18n.rs \
  src-tauri/src/lib.rs \
  src-tauri/src/logging.rs \
  src-tauri/src/secrets.rs \
  src-tauri/src/storage/fs.rs \
  src-tauri/src/storage/mod.rs \
  src-tauri/src/sync.rs \
  src-tauri/src/tls.rs
git commit -s -m "feat(core): Rust foundation — texts, errors, atomic files, secrets, logging" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 3: Политика администратора с `allowInsecureTls`, язык интерфейса

**Files:**
- Port: `docs/admin/policy.mobileconfig` (с правками), `src-tauri/src/commands/settings.rs`, `src-tauri/src/policy/mod.rs` (с правками), `src-tauri/src/policy/sources.rs` (с правками), `src-tauri/src/settings.rs` (с правками)
- Create (по Kubernetes-версии): `docs/admin/policy.json` ← `docs/admin/policy.json`, `docs/admin/policy.reg` ← `docs/admin/policy.reg`
- Create: `src-tauri/src/commands/mod.rs`, `src-tauri/src/state.rs`
- Modify: `PORTED.md`, `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`
- Generated: `src-tauri/Cargo.lock` (cargo, pnpm)

**Interfaces:**
- Consumes: задача 2.
- Produces: `policy::{Policy, PolicyStatus { managed, source, updates_enabled, update_url, language, mcp_enabled, allow_insecure_tls, errors }, resolve, sources::load}`, `settings::{SettingsStore, LANGUAGES = ["en", "ru"], resolve}`, `state::AppState { secrets, policy, settings }` с `new(dir, secrets)` и `with_policy`, команды `get_language_settings`, `set_language`, `get_policy_status`, `commands::blocking`; примеры политики `docs/admin/policy.{json,mobileconfig,reg}`.

Перенос политики и настроек. Ключ `allowExecAuth` убран: он вернётся в I3 вместе с подключением к Kubernetes. `updatesEnabled`, `updateUrl` и `mcpEnabled` остаются: ими пользуются I2 и примеры администратора. Настройки MCP в `settings.rs` переносятся как есть — понадобятся в I2.

- [ ] **Step 1: Перенос из Kubernetes-версии.**

Файлы копируются с коммита `468e08a` репозитория `~/projects/glafira-openshift` (`$KUBERNETES_REPO`) и правятся патчами — против перенесённого файла. Строки о них в `PORTED.md` добавляет шаг реализации.

<!-- block: shell t03-port step=3.1 -->
````bash
scripts/port.sh src-tauri/src/policy/mod.rs src-tauri/src/policy/mod.rs
scripts/port.sh src-tauri/src/policy/sources.rs src-tauri/src/policy/sources.rs
scripts/port.sh src-tauri/src/settings.rs src-tauri/src/settings.rs
scripts/port.sh src-tauri/src/commands/settings.rs src-tauri/src/commands/settings.rs
scripts/port.sh docs/admin/policy.mobileconfig docs/admin/policy.mobileconfig
````

<!-- block: patch t03-port-src-tauri-src-policy-mod-rs step=3.1 -->
````diff
diff --git a/src-tauri/src/policy/mod.rs b/src-tauri/src/policy/mod.rs
--- a/src-tauri/src/policy/mod.rs
+++ b/src-tauri/src/policy/mod.rs
@@ -13,7 +13,6 @@ pub const KEY_UPDATE_URL: &str = "updateUrl";
 pub const KEY_LANGUAGE: &str = "language";
 pub const KEY_MCP_ENABLED: &str = "mcpEnabled";
 pub const KEY_ALLOW_INSECURE_TLS: &str = "allowInsecureTls";
-pub const KEY_ALLOW_EXEC_AUTH: &str = "allowExecAuth";
 
 #[derive(Debug, Clone, PartialEq)]
 pub enum PolicyValue {
@@ -74,10 +73,8 @@ pub struct PolicyStatus {
     pub language: Option<String>,
     /// `false` — MCP-шлюз запрещён администратором (спец. G1 §9).
     pub mcp_enabled: bool,
-    /// `false` — подключения с `insecure-skip-tls-verify` запрещены администратором (спец. §12).
+    /// `false` — серверы с `insecure-skip-tls-verify` запрещены администратором (спец. §5.2, §13).
     pub allow_insecure_tls: bool,
-    /// `false` — подключения с exec-плагином запрещены администратором (спец. §5.2, §12).
-    pub allow_exec_auth: bool,
     pub errors: Vec<LocalizedText>,
 }
 
@@ -91,7 +88,6 @@ impl Default for PolicyStatus {
             language: None,
             mcp_enabled: true,
             allow_insecure_tls: true,
-            allow_exec_auth: true,
             errors: Vec::new(),
         }
     }
@@ -115,7 +111,6 @@ pub fn resolve(source: &str, raw: RawPolicy) -> Policy {
                     updates_enabled: false,
                     mcp_enabled: false,
                     allow_insecure_tls: false,
-                    allow_exec_auth: false,
                     errors: vec![ltext!(
                         "errors.policy.unreadable",
                         "Policy cannot be read: {{reason}}",
@@ -177,13 +172,6 @@ pub fn resolve(source: &str, raw: RawPolicy) -> Policy {
                 status.allow_insecure_tls = false;
                 status.errors.push(other.invalid(&key));
             }
-            (KEY_ALLOW_EXEC_AUTH, PolicyValue::Bool(b)) => status.allow_exec_auth = b,
-            (KEY_ALLOW_EXEC_AUTH, PolicyValue::Int(i @ (0 | 1))) => status.allow_exec_auth = i == 1,
-            // Непонятое значение запрещает exec-плагины.
-            (KEY_ALLOW_EXEC_AUTH, other) => {
-                status.allow_exec_auth = false;
-                status.errors.push(other.invalid(&key));
-            }
             (KEY_UPDATES_ENABLED | KEY_UPDATE_URL, other) => {
                 broken = true;
                 status.errors.push(other.invalid(&key));
@@ -391,28 +379,4 @@ mod tests {
                 .allow_insecure_tls
         );
     }
-
-    #[test]
-    fn exec_auth_policy() {
-        assert!(resolve("src", RawPolicy::Absent).status.allow_exec_auth);
-        for (v, allowed) in [
-            (PolicyValue::Bool(false), false),
-            (PolicyValue::Bool(true), true),
-            (PolicyValue::Int(0), false),
-            (PolicyValue::Int(1), true),
-        ] {
-            let p = resolve("src", values(&[("allowExecAuth", v)]));
-            assert_eq!(p.status.allow_exec_auth, allowed);
-            assert!(p.status.errors.is_empty());
-        }
-        let p = resolve("src", values(&[("allowExecAuth", PolicyValue::Int(2))]));
-        assert!(!p.status.allow_exec_auth, "непонятое значение запрещает");
-        assert!(p.status.updates_enabled);
-        assert!(p.status.allow_insecure_tls, "другие ключи не задеты");
-        assert!(
-            !resolve("src", RawPolicy::Unreadable("x".into()))
-                .status
-                .allow_exec_auth
-        );
-    }
 }
````

<!-- block: patch t03-port-src-tauri-src-policy-sources-rs step=3.1 -->
````diff
diff --git a/src-tauri/src/policy/sources.rs b/src-tauri/src/policy/sources.rs
--- a/src-tauri/src/policy/sources.rs
+++ b/src-tauri/src/policy/sources.rs
@@ -218,7 +218,6 @@ mod tests {
             !p.status.allow_insecure_tls,
             "пример запрещает подключения без проверки TLS"
         );
-        assert!(!p.status.allow_exec_auth, "пример запрещает exec-плагины");
 
         // Профиль MDM: содержимое PayloadContent[0] попадает в Managed Preferences как есть.
         let profile = plist::Value::from_file(format!("{base}/policy.mobileconfig")).unwrap();
@@ -239,7 +238,6 @@ mod tests {
             "language",
             "mcpEnabled",
             "allowInsecureTls",
-            "allowExecAuth",
         ] {
             assert!(content.contains_key(key), "{key}");
         }
@@ -259,6 +257,5 @@ mod tests {
             reg.contains(r#""allowInsecureTls"=dword:00000000"#),
             "{reg}"
         );
-        assert!(reg.contains(r#""allowExecAuth"=dword:00000000"#), "{reg}");
     }
 }
````

<!-- block: patch t03-port-src-tauri-src-settings-rs step=3.1 -->
````diff
diff --git a/src-tauri/src/settings.rs b/src-tauri/src/settings.rs
--- a/src-tauri/src/settings.rs
+++ b/src-tauri/src/settings.rs
@@ -9,8 +9,8 @@ use crate::error::{AppResult, ErrorCode};
 use crate::storage::fs::{read_json, read_json_or_quarantine, write_json};
 use crate::sync::RwLockExt;
 
-/// Языки, для которых есть словарь `src/locales/<код>.json`. Остальные 10 языков Kafka-версии
-/// добавляются на этапе O6 (спец. §16).
+/// Языки, для которых есть словарь `src/locales/<код>.json`. Остальные 10 языков семейства
+/// добавляются на этапе I7 (спец. §17).
 pub const LANGUAGES: [&str; 2] = ["en", "ru"];
 
 fn is_known(code: &str) -> bool {
````

<!-- block: patch t03-port-docs-admin-policy-mobileconfig step=3.1 -->
````diff
diff --git a/docs/admin/policy.mobileconfig b/docs/admin/policy.mobileconfig
--- a/docs/admin/policy.mobileconfig
+++ b/docs/admin/policy.mobileconfig
@@ -5,20 +5,19 @@
   <key>PayloadContent</key>
   <array>
     <dict>
-      <key>PayloadType</key><string>io.github.vvnovg.glafira.kubernetes</string>
-      <key>PayloadIdentifier</key><string>io.github.vvnovg.glafira.kubernetes.policy</string>
+      <key>PayloadType</key><string>io.github.vvnovg.glafira.inference</string>
+      <key>PayloadIdentifier</key><string>io.github.vvnovg.glafira.inference.policy</string>
       <key>PayloadUUID</key><string>00000000-0000-0000-0000-000000000001</string>
       <key>PayloadVersion</key><integer>1</integer>
       <key>updatesEnabled</key><false/>
-      <key>updateUrl</key><string>https://mirror.example.corp/glafira-kubernetes/latest.json</string>
+      <key>updateUrl</key><string>https://mirror.example.corp/glafira-inference/latest.json</string>
       <key>language</key><string>en</string>
       <key>mcpEnabled</key><false/>
       <key>allowInsecureTls</key><false/>
-      <key>allowExecAuth</key><false/>
     </dict>
   </array>
-  <key>PayloadDisplayName</key><string>Glafira for Kubernetes policy</string>
-  <key>PayloadIdentifier</key><string>io.github.vvnovg.glafira.kubernetes.profile</string>
+  <key>PayloadDisplayName</key><string>Glafira for LLM Inference policy</string>
+  <key>PayloadIdentifier</key><string>io.github.vvnovg.glafira.inference.profile</string>
   <key>PayloadType</key><string>Configuration</string>
   <key>PayloadUUID</key><string>00000000-0000-0000-0000-000000000002</string>
   <key>PayloadVersion</key><integer>1</integer>
````

- [ ] **Step 2: Реализация.**

<!-- block: file t03-docs-admin-policy-json step=3.2 path=docs/admin/policy.json -->
````json
{
  "updatesEnabled": false,
  "updateUrl": "https://mirror.example.corp/glafira-inference/latest.json",
  "language": "en",
  "mcpEnabled": false,
  "allowInsecureTls": false
}
````

<!-- block: file t03-docs-admin-policy-reg step=3.2 path=docs/admin/policy.reg -->
````text
Windows Registry Editor Version 5.00

[HKEY_LOCAL_MACHINE\SOFTWARE\Policies\Glafira\Inference]
"updatesEnabled"=dword:00000000
"updateUrl"="https://mirror.example.corp/glafira-inference/latest.json"
"language"="en"
"mcpEnabled"=dword:00000000
"allowInsecureTls"=dword:00000000
````

<!-- block: file t03-src-tauri-src-commands-mod-rs step=3.2 path=src-tauri/src/commands/mod.rs -->
````rust
pub mod settings;

use crate::error::{AppResult, ErrorCode};

/// Блокирующие вызовы (файлы, keychain) не выполняются на async-потоках Tauri.
pub(crate) async fn blocking<T, F>(f: F) -> AppResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> AppResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| {
        app_err!(
            ErrorCode::Internal,
            "errors.app.taskPanicked",
            "Background task failed: {{error}}",
            error = e
        )
    })?
}
````

<!-- block: file t03-src-tauri-src-state-rs step=3.2 path=src-tauri/src/state.rs -->
````rust
use std::path::Path;
use std::sync::Arc;

use crate::error::AppResult;
use crate::secrets::SecretStore;

/// Общее состояние приложения: хранилища и политика. Клонируется дёшево (всё за `Arc`).
#[derive(Clone)]
pub struct AppState {
    pub secrets: Arc<dyn SecretStore>,
    pub policy: Arc<crate::policy::Policy>,
    pub settings: Arc<crate::settings::SettingsStore>,
}

impl AppState {
    pub fn new(data_dir: &Path, secrets: Arc<dyn SecretStore>) -> AppResult<Self> {
        Ok(Self {
            secrets,
            policy: Arc::new(crate::policy::Policy::default()),
            settings: Arc::new(crate::settings::SettingsStore::open(data_dir)),
        })
    }

    /// Политика администратора (читается при запуске в lib.rs).
    pub fn with_policy(mut self, policy: crate::policy::Policy) -> Self {
        self.policy = Arc::new(policy);
        self
    }
}
````

<!-- block: patch t03-src step=3.2 -->
````diff
diff --git a/PORTED.md b/PORTED.md
index 0e4222c..8e86819 100644
--- a/PORTED.md
+++ b/PORTED.md
@@ -44,3 +44,10 @@
 | `src-tauri/src/sync.rs` | `src-tauri/src/sync.rs` | `468e08a` | как есть |
 | `src-tauri/src/dock_name.rs` | `src-tauri/src/dock_name.rs` | `468e08a` | адаптирован: имя бинаря |
 | `src-tauri/src/tls.rs` | `src-tauri/src/tls.rs` | `468e08a` | как есть |
+| `src-tauri/src/policy/mod.rs` | `src-tauri/src/policy/mod.rs` | `468e08a` | адаптирован: без `allowExecAuth` (появится в I3 вместе с Kubernetes) |
+| `src-tauri/src/policy/sources.rs` | `src-tauri/src/policy/sources.rs` | `468e08a` | адаптирован: без `allowExecAuth` в проверке примеров |
+| `src-tauri/src/settings.rs` | `src-tauri/src/settings.rs` | `468e08a` | адаптирован: ссылка на этап I7 |
+| `src-tauri/src/commands/settings.rs` | `src-tauri/src/commands/settings.rs` | `468e08a` | как есть |
+| `docs/admin/policy.json` | `docs/admin/policy.json` | `468e08a` | адаптирован: идентификаторы, без `allowExecAuth` |
+| `docs/admin/policy.mobileconfig` | `docs/admin/policy.mobileconfig` | `468e08a` | адаптирован: идентификаторы, без `allowExecAuth` |
+| `docs/admin/policy.reg` | `docs/admin/policy.reg` | `468e08a` | адаптирован: ветка реестра, без `allowExecAuth` |
diff --git a/src-tauri/Cargo.toml b/src-tauri/Cargo.toml
index f1cb6ed..7bede8e 100644
--- a/src-tauri/Cargo.toml
+++ b/src-tauri/Cargo.toml
@@ -26,6 +26,11 @@ tracing = "0.1"
 tracing-subscriber = { version = "0.3", features = ["env-filter", "registry", "fmt"] }
 keyring = { version = "3", features = ["apple-native", "windows-native", "sync-secret-service"] }
 rustls = { version = "0.23", default-features = false, features = ["ring", "std", "tls12"] }
+plist = "1"
+sys-locale = "0.3"
+
+[target.'cfg(windows)'.dependencies]
+winreg = "0.55"
 
 [dev-dependencies]
 tempfile = "3"
diff --git a/src-tauri/src/lib.rs b/src-tauri/src/lib.rs
index 5bde940..b4ef572 100644
--- a/src-tauri/src/lib.rs
+++ b/src-tauri/src/lib.rs
@@ -1,16 +1,25 @@
 #[macro_use]
 pub mod i18n;
+pub mod commands;
 mod dock_name;
 pub mod error;
 pub mod logging;
+pub mod policy;
 pub mod product;
 pub mod secrets;
+pub mod settings;
+pub mod state;
 pub mod storage;
 pub mod sync;
 pub mod tls;
 
+use std::sync::Arc;
+
 use tauri::Manager;
 
+use crate::secrets::KeyringStore;
+use crate::state::AppState;
+
 #[cfg_attr(mobile, tauri::mobile_entry_point)]
 pub fn run() {
     crate::tls::ensure_crypto_provider();
@@ -23,8 +32,18 @@ pub fn run() {
             if !crate::dock_name::set(crate::product::PRODUCT_NAME) {
                 tracing::warn!("dock name was not set: LaunchServices refused");
             }
+            let dir = app.path().app_data_dir()?;
+            let policy = crate::policy::sources::load();
+            tracing::info!(managed = policy.status.managed, "policy");
+            let st = AppState::new(&dir, Arc::new(KeyringStore::default()))?.with_policy(policy);
+            app.manage(st);
             Ok(())
         })
+        .invoke_handler(tauri::generate_handler![
+            commands::settings::get_language_settings,
+            commands::settings::set_language,
+            commands::settings::get_policy_status,
+        ])
         .run(tauri::generate_context!())
         .expect("error while building tauri application");
 }
````

- [ ] **Step 3: Проверки.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib`  
Expected: PASS — `test result: ok. 46 passed; 0 failed`

Run: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`  
Expected: PASS — без предупреждений

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml --check`  
Expected: PASS — без изменений

Run: `cargo deny --manifest-path src-tauri/Cargo.toml --config deny.toml check licenses advisories`  
Expected: PASS — `advisories ok, licenses ok`

Run: `cargo tree --manifest-path src-tauri/Cargo.toml -i aws-lc-rs`  
Expected: ошибка `did not match any packages` — `aws-lc-rs` в дереве нет (то же для `openssl-sys`)

- [ ] **Step 4: Commit.**

```bash
git add \
  PORTED.md \
  docs/admin/policy.json \
  docs/admin/policy.mobileconfig \
  docs/admin/policy.reg \
  src-tauri/Cargo.lock \
  src-tauri/Cargo.toml \
  src-tauri/src/commands/mod.rs \
  src-tauri/src/commands/settings.rs \
  src-tauri/src/lib.rs \
  src-tauri/src/policy/mod.rs \
  src-tauri/src/policy/sources.rs \
  src-tauri/src/settings.rs \
  src-tauri/src/state.rs
git commit -s -m "feat(policy): administrator policy with allowInsecureTls, interface language settings" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 4: Профиль инсталляции с серверами, хранилище, папки, экспорт и импорт

**Files:**
- Port: `src-tauri/src/commands/folders.rs` (с правками), `src-tauri/src/storage/installations.rs` (с правками), `src-tauri/src/storage/testdata/test-cert.pem`, `src-tauri/src/storage/testdata/test-key.pem`, `src-tauri/src/storage/tree.rs` (с правками)
- Create (по Kubernetes-версии): `src-tauri/src/storage/exchange.rs` ← `src-tauri/src/storage/exchange.rs`, `src-tauri/src/storage/profile.rs` ← `src-tauri/src/storage/profile.rs`
- Modify: `PORTED.md`, `src-tauri/Cargo.toml`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/state.rs`, `src-tauri/src/storage/mod.rs`
- Generated: `src-tauri/Cargo.lock` (cargo, pnpm)

**Interfaces:**
- Consumes: задачи 2–3: `fs`, `secrets`, `AppState`.
- Produces: `storage::profile::{InstallationProfile, ServerConfig { id, name, base_url, kind, metrics_path, ca, proxy, extra }, ServerKind (auto | vllm | sglang | openai), Environment, CaConfig, ProxyConfig, MAX_SERVERS = 32, normalize_profile, validate_profile, validate_server (с задачи 8 — pub), parse_base_url, is_valid_server_id, pem_certificates, pem_has_private_key, strip_private_keys, redact_url}`, `storage::installations::{InstallationStore, InstallationFolder, StoreData}` (`installations.json`), `storage::tree` (папки и перемещение), `storage::exchange` (файл `glafira-inference-installations`, `NeedsSecret { api_keys }`, `ImportPlan.removed_servers`), команды папок, экспорта и импорта; `AppState.installations`.

Профиль пишется заново по образцу профиля подключения Kubernetes-версии (PEM, адреса без `user:pass@`, прокси — как там). Адрес сервера — `http://` или `https://`: внутренние серверы часто без TLS. Хранилище, дерево папок, обмен и команды папок переносятся с переименованием «подключение → инсталляция». В обмене: ключи API по серверам вместо токена и ключа сертификата, предупреждения при смене адреса, прокси или CA у сервера с сохранённым ключом, без exec-плагинов и доступа LLM. Неизвестные поля серверов, как и профиля, — локальные. Импорт с «Обновлять существующие» удаляет из keychain ключи серверов, которых нет в файле.

- [ ] **Step 1: Перенос из Kubernetes-версии.**

Файлы копируются с коммита `468e08a` репозитория `~/projects/glafira-openshift` (`$KUBERNETES_REPO`) и правятся патчами — против перенесённого файла. Строки о них в `PORTED.md` добавляет шаг реализации.

<!-- block: shell t04-port step=4.1 -->
````bash
scripts/port.sh src-tauri/src/storage/testdata/test-cert.pem src-tauri/src/storage/testdata/test-cert.pem
scripts/port.sh src-tauri/src/storage/testdata/test-key.pem src-tauri/src/storage/testdata/test-key.pem
scripts/port.sh src-tauri/src/storage/connections.rs src-tauri/src/storage/installations.rs
scripts/port.sh src-tauri/src/storage/tree.rs src-tauri/src/storage/tree.rs
scripts/port.sh src-tauri/src/commands/folders.rs src-tauri/src/commands/folders.rs
````

<!-- block: patch t04-port-src-tauri-src-storage-installations-rs step=4.1 -->
````diff
diff --git a/src-tauri/src/storage/installations.rs b/src-tauri/src/storage/installations.rs
--- a/src-tauri/src/storage/installations.rs
+++ b/src-tauri/src/storage/installations.rs
@@ -7,12 +7,12 @@ use crate::error::{AppError, AppResult, ErrorCode};
 use crate::storage::fs::{read_json, write_json};
 #[cfg(test)]
 pub use crate::storage::profile::sample_profile;
-pub use crate::storage::profile::{ConnectionProfile, Environment, validate_profile};
+pub use crate::storage::profile::{Environment, InstallationProfile, validate_profile};
 use crate::sync::RwLockExt;
 
 #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
 #[serde(rename_all = "camelCase")]
-pub struct ConnectionFolder {
+pub struct InstallationFolder {
     pub id: String,
     pub name: String,
     #[serde(default)]
@@ -24,11 +24,11 @@ pub struct ConnectionFolder {
     pub extra: serde_json::Map<String, serde_json::Value>,
 }
 
-pub(crate) fn connection_not_found(id: &str) -> AppError {
+pub(crate) fn installation_not_found(id: &str) -> AppError {
     app_err!(
         ErrorCode::NotFound,
-        "errors.connections.notFound",
-        "Connection {{id}} not found",
+        "errors.installations.notFound",
+        "Installation {{id}} not found",
         id = id
     )
 }
@@ -36,15 +36,15 @@ pub(crate) fn connection_not_found(id: &str) -> AppError {
 const CURRENT_SCHEMA_VERSION: u32 = 1;
 const FOLDER_NAME_MAX_CHARS: usize = 100;
 
-/// Папки и подключения читаются по одному: запись, которую эта версия не понимает
-/// (например, новое окружение или механизм аутентификации), не ломает весь файл.
+/// Папки и инсталляции читаются по одному: запись, которую эта версия не понимает
+/// (например, новое окружение или тип сервера), не ломает весь файл.
 #[derive(Deserialize)]
 #[serde(rename_all = "camelCase")]
-struct ConnectionsFile {
+struct InstallationsFile {
     schema_version: u32,
     #[serde(default)]
     folders: Vec<serde_json::Value>,
-    connections: Vec<serde_json::Value>,
+    installations: Vec<serde_json::Value>,
     #[serde(flatten)]
     extra: serde_json::Map<String, serde_json::Value>,
 }
@@ -69,25 +69,25 @@ impl<T: Serialize> Serialize for WithRaw<'_, T> {
 /// Запись файла без копирования данных кэша.
 #[derive(Serialize)]
 #[serde(rename_all = "camelCase")]
-struct ConnectionsFileRef<'a> {
+struct InstallationsFileRef<'a> {
     schema_version: u32,
-    folders: WithRaw<'a, ConnectionFolder>,
-    connections: WithRaw<'a, ConnectionProfile>,
+    folders: WithRaw<'a, InstallationFolder>,
+    installations: WithRaw<'a, InstallationProfile>,
     #[serde(flatten)]
     extra: &'a serde_json::Map<String, serde_json::Value>,
 }
 
-/// Содержимое `connections.json` в памяти: подключения, папки и неизвестные поля файла.
+/// Содержимое `installations.json` в памяти: инсталляции, папки и неизвестные поля файла.
 ///
-/// `raw_connections` и `raw_folders` — записи, которые эта версия не смогла разобрать
+/// `raw_installations` и `raw_folders` — записи, которые эта версия не смогла разобрать
 /// (из более новой Glafira). В приложении они не показываются, `check_tree` их не видит,
 /// ссылки на нечитаемую папку считаются корнем; при записи они возвращаются в файл без изменений.
 #[derive(Debug, Clone, Default)]
 pub(crate) struct StoreData {
-    pub connections: Vec<ConnectionProfile>,
-    pub folders: Vec<ConnectionFolder>,
+    pub installations: Vec<InstallationProfile>,
+    pub folders: Vec<InstallationFolder>,
     pub extra: serde_json::Map<String, serde_json::Value>,
-    pub raw_connections: Vec<serde_json::Value>,
+    pub raw_installations: Vec<serde_json::Value>,
     pub raw_folders: Vec<serde_json::Value>,
 }
 
@@ -105,7 +105,7 @@ fn parse_entries<T: serde::de::DeserializeOwned>(
             Err(_) => {
                 let id = v.get("id").and_then(|id| id.as_str()).unwrap_or("?");
                 tracing::warn!(
-                    "connections.json: {kind} #{i} (id {id}) is not supported by this version and is hidden; it is kept in the file"
+                    "installations.json: {kind} #{i} (id {id}) is not supported by this version and is hidden; it is kept in the file"
                 );
                 raw.push(v);
             }
@@ -176,9 +176,9 @@ pub(crate) fn unique_name(taken: &[&str], wanted: &str) -> String {
 pub(crate) fn check_tree(data: &StoreData) -> AppResult<()> {
     use std::collections::{HashMap, HashSet};
 
-    let by_id: HashMap<&str, &ConnectionFolder> =
+    let by_id: HashMap<&str, &InstallationFolder> =
         data.folders.iter().map(|f| (f.id.as_str(), f)).collect();
-    for c in &data.connections {
+    for c in &data.installations {
         if let Some(fid) = c.folder_id.as_deref()
             && !by_id.contains_key(fid)
         {
@@ -230,13 +230,13 @@ pub(crate) fn check_tree(data: &StoreData) -> AppResult<()> {
 fn repair_on_read(data: &mut StoreData) {
     use std::collections::{HashMap, HashSet};
 
-    if data.connections.len() > 1 && data.connections.iter().all(|c| c.order == 0) {
-        for (i, c) in data.connections.iter_mut().enumerate() {
+    if data.installations.len() > 1 && data.installations.iter().all(|c| c.order == 0) {
+        for (i, c) in data.installations.iter_mut().enumerate() {
             c.order = i as i64;
         }
     }
     let ids: HashSet<String> = data.folders.iter().map(|f| f.id.clone()).collect();
-    for c in &mut data.connections {
+    for c in &mut data.installations {
         if c.folder_id.as_ref().is_some_and(|id| !ids.contains(id)) {
             c.folder_id = None;
         }
@@ -292,16 +292,16 @@ fn repair_on_read(data: &mut StoreData) {
     }
 }
 
-pub struct ConnectionStore {
+pub struct InstallationStore {
     path: PathBuf,
     cache: RwLock<StoreData>,
 }
 
-impl ConnectionStore {
+impl InstallationStore {
     pub fn open(dir: &Path) -> AppResult<Self> {
         std::fs::create_dir_all(dir)?;
-        let path = dir.join("connections.json");
-        let mut data = match read_json::<ConnectionsFile>(&path)? {
+        let path = dir.join("installations.json");
+        let mut data = match read_json::<InstallationsFile>(&path)? {
             Some(file) if file.schema_version > CURRENT_SCHEMA_VERSION => {
                 return Err(crate::storage::fs::newer_schema_error(
                     &path,
@@ -309,17 +309,17 @@ impl ConnectionStore {
                 ));
             }
             Some(file) => {
-                let mut raw_connections = Vec::new();
+                let mut raw_installations = Vec::new();
                 let mut raw_folders = Vec::new();
                 StoreData {
-                    connections: parse_entries(
-                        "connection",
-                        file.connections,
-                        &mut raw_connections,
+                    installations: parse_entries(
+                        "installation",
+                        file.installations,
+                        &mut raw_installations,
                     ),
                     folders: parse_entries("folder", file.folders, &mut raw_folders),
                     extra: file.extra,
-                    raw_connections,
+                    raw_installations,
                     raw_folders,
                 }
             }
@@ -332,22 +332,22 @@ impl ConnectionStore {
         })
     }
 
-    pub fn list(&self) -> Vec<ConnectionProfile> {
-        self.cache.read_safe().connections.clone()
+    pub fn list(&self) -> Vec<InstallationProfile> {
+        self.cache.read_safe().installations.clone()
     }
 
-    pub fn list_folders(&self) -> Vec<ConnectionFolder> {
+    pub fn list_folders(&self) -> Vec<InstallationFolder> {
         self.cache.read_safe().folders.clone()
     }
 
-    pub fn get(&self, id: &str) -> AppResult<ConnectionProfile> {
+    pub fn get(&self, id: &str) -> AppResult<InstallationProfile> {
         self.cache
             .read_safe()
-            .connections
+            .installations
             .iter()
             .find(|c| c.id == id)
             .cloned()
-            .ok_or_else(|| connection_not_found(id))
+            .ok_or_else(|| installation_not_found(id))
     }
 
     /// Копия всех данных хранилища (для расчётов без записи, например предпросмотра импорта).
@@ -377,19 +377,26 @@ impl ConnectionStore {
         Ok(result)
     }
 
-    /// Создаёт или обновляет подключение и возвращает его в сохранённом виде.
+    /// Создаёт или обновляет инсталляцию и возвращает её в сохранённом виде.
     ///
-    /// Новое подключение кладётся в конец папки `folder_id` входящего профиля.
+    /// Новая инсталляция кладётся в конец папки `folder_id` входящего профиля.
     /// У существующего `folder_id` и `order` берутся из сохранённого, входящие значения
-    /// игнорируются: место подключения меняют только команды перемещения. Входящий `extra`
+    /// игнорируются: место инсталляции меняют только команды перемещения. Входящий `extra`
     /// игнорируется всегда: неизвестные поля пишет только сама Glafira (существующий профиль
     /// сохраняет свои, новый получает пустые), поэтому через них нельзя записать секрет в файл.
-    pub fn upsert(&self, mut profile: ConnectionProfile) -> AppResult<ConnectionProfile> {
+    /// То же у серверов: сервер с прежним id сохраняет свои неизвестные поля, новый — пустые.
+    pub fn upsert(&self, mut profile: InstallationProfile) -> AppResult<InstallationProfile> {
         crate::storage::profile::normalize_profile(&mut profile);
         validate_profile(&profile)?;
         self.update(|data| {
-            match data.connections.iter_mut().find(|c| c.id == profile.id) {
+            match data.installations.iter_mut().find(|c| c.id == profile.id) {
                 Some(slot) => {
+                    for server in &mut profile.servers {
+                        server.extra = slot
+                            .server(&server.id)
+                            .map(|old| old.extra.clone())
+                            .unwrap_or_default();
+                    }
                     profile.extra = std::mem::take(&mut slot.extra);
                     profile.folder_id = slot.folder_id.take();
                     profile.order = slot.order;
@@ -397,14 +404,17 @@ impl ConnectionStore {
                 }
                 None => {
                     profile.extra = Default::default();
+                    for server in &mut profile.servers {
+                        server.extra = Default::default();
+                    }
                     profile.order = data
-                        .connections
+                        .installations
                         .iter()
                         .filter(|c| c.folder_id == profile.folder_id)
                         .map(|c| c.order + 1)
                         .max()
                         .unwrap_or(0);
-                    data.connections.push(profile.clone());
+                    data.installations.push(profile.clone());
                 }
             }
             Ok(profile)
@@ -413,10 +423,10 @@ impl ConnectionStore {
 
     pub fn delete(&self, id: &str) -> AppResult<()> {
         self.update(|data| {
-            if !data.connections.iter().any(|c| c.id == id) {
-                return Err(connection_not_found(id));
+            if !data.installations.iter().any(|c| c.id == id) {
+                return Err(installation_not_found(id));
             }
-            data.connections.retain(|c| c.id != id);
+            data.installations.retain(|c| c.id != id);
             Ok(())
         })
     }
@@ -424,10 +434,10 @@ impl ConnectionStore {
     fn persist(&self, data: &StoreData) -> AppResult<()> {
         write_json(
             &self.path,
-            &ConnectionsFileRef {
+            &InstallationsFileRef {
                 schema_version: CURRENT_SCHEMA_VERSION,
                 folders: WithRaw(&data.folders, &data.raw_folders),
-                connections: WithRaw(&data.connections, &data.raw_connections),
+                installations: WithRaw(&data.installations, &data.raw_installations),
                 extra: &data.extra,
             },
         )
@@ -442,17 +452,17 @@ mod tests {
     #[test]
     fn persists_and_reloads() {
         let dir = tempfile::tempdir().unwrap();
-        let store = ConnectionStore::open(dir.path()).unwrap();
+        let store = InstallationStore::open(dir.path()).unwrap();
         store.upsert(sample_profile("a")).unwrap();
         let mut b = sample_profile("b");
-        b.auth = crate::storage::profile::AuthConfig::ClientCert {
-            certificate_pem: crate::storage::profile::TEST_CERT_PEM.into(),
+        b.servers[0].ca = crate::storage::profile::CaConfig::Pem {
+            pem: crate::storage::profile::TEST_CERT_PEM.into(),
         };
-        b.ca = crate::storage::profile::CaConfig::Insecure;
-        b.favorite_namespaces = vec!["team-a".into()];
+        b.servers.push(crate::storage::profile::sample_server("s2"));
+        b.servers[1].ca = crate::storage::profile::CaConfig::Insecure;
         let b = store.upsert(b).unwrap();
 
-        let reopened = ConnectionStore::open(dir.path()).unwrap();
+        let reopened = InstallationStore::open(dir.path()).unwrap();
         assert_eq!(reopened.list().len(), 2);
         assert_eq!(reopened.get("b").unwrap(), b);
     }
@@ -460,7 +470,7 @@ mod tests {
     #[test]
     fn upsert_replaces_and_delete_removes() {
         let dir = tempfile::tempdir().unwrap();
-        let store = ConnectionStore::open(dir.path()).unwrap();
+        let store = InstallationStore::open(dir.path()).unwrap();
         store.upsert(sample_profile("a")).unwrap();
         let mut a2 = sample_profile("a");
         a2.name = "renamed".into();
@@ -475,25 +485,25 @@ mod tests {
     #[test]
     fn file_contains_schema_version_and_no_secrets() {
         let dir = tempfile::tempdir().unwrap();
-        let store = ConnectionStore::open(dir.path()).unwrap();
+        let store = InstallationStore::open(dir.path()).unwrap();
         store.upsert(sample_profile("a")).unwrap();
-        let raw = std::fs::read_to_string(dir.path().join("connections.json")).unwrap();
+        let raw = std::fs::read_to_string(dir.path().join("installations.json")).unwrap();
         assert!(raw.contains("\"schemaVersion\": 1"));
         assert!(!raw.to_lowercase().contains("password"));
-        assert!(!raw.contains("\"token\":"));
+        assert!(!raw.contains("\"token\":") && !raw.contains("apiKey"));
     }
 
     #[test]
-    fn migration_keeps_connections_in_root_with_order() {
+    fn migration_keeps_installations_in_root_with_order() {
         let dir = tempfile::tempdir().unwrap();
         std::fs::write(
-            dir.path().join("connections.json"),
-            r##"{"schemaVersion":1,"connections":[
-          {"id":"a","name":"A","color":"#fff","environment":"dev","readOnly":false,"apiUrl":"https://api.c:6443","auth":{"method":"token"}},
-          {"id":"b","name":"B","color":"#fff","environment":"dev","readOnly":false,"apiUrl":"https://api.c:6443","auth":{"method":"token"}}]}"##,
+            dir.path().join("installations.json"),
+            r##"{"schemaVersion":1,"installations":[
+          {"id":"a","name":"A","color":"#fff","environment":"dev","readOnly":false,"servers":[{"id":"s1","name":"vllm","baseUrl":"http://vllm:8000"}]},
+          {"id":"b","name":"B","color":"#fff","environment":"dev","readOnly":false,"servers":[{"id":"s1","name":"vllm","baseUrl":"http://vllm:8000"}]}]}"##,
         )
         .unwrap();
-        let store = ConnectionStore::open(dir.path()).unwrap();
+        let store = InstallationStore::open(dir.path()).unwrap();
         let list = store.list();
         assert_eq!(
             list.iter()
@@ -508,43 +518,43 @@ mod tests {
     fn unknown_fields_survive_write() {
         let dir = tempfile::tempdir().unwrap();
         std::fs::write(
-            dir.path().join("connections.json"),
+            dir.path().join("installations.json"),
             r##"{"schemaVersion":1,"future":{"x":1},"folders":[{"id":"f","name":"F","parentId":null,"order":0,"icon":"star"}],
-           "connections":[{"id":"a","name":"A","color":"#fff","environment":"dev","readOnly":false,"apiUrl":"https://api.c:6443",
-           "auth":{"method":"token"},"folderId":"f","order":0,"pinned":true}]}"##,
+           "installations":[{"id":"a","name":"A","color":"#fff","environment":"dev","readOnly":false,"servers":[{"id":"s1","name":"vllm","baseUrl":"http://vllm:8000"}],
+           "folderId":"f","order":0,"pinned":true}]}"##,
         )
         .unwrap();
-        let store = ConnectionStore::open(dir.path()).unwrap();
+        let store = InstallationStore::open(dir.path()).unwrap();
         let mut p = store.get("a").unwrap();
         p.name = "A2".into();
         // Входящие неизвестные поля игнорируются: остаются сохранённые.
         p.extra.insert("pinned".into(), serde_json::json!(false));
         store.upsert(p).unwrap();
         let raw: serde_json::Value =
-            serde_json::from_slice(&std::fs::read(dir.path().join("connections.json")).unwrap())
+            serde_json::from_slice(&std::fs::read(dir.path().join("installations.json")).unwrap())
                 .unwrap();
         assert_eq!(raw["future"]["x"], 1);
         assert_eq!(raw["folders"][0]["icon"], "star");
-        assert_eq!(raw["connections"][0]["pinned"], true);
-        assert_eq!(raw["connections"][0]["name"], "A2");
-        assert_eq!(raw["connections"][0]["folderId"], "f");
+        assert_eq!(raw["installations"][0]["pinned"], true);
+        assert_eq!(raw["installations"][0]["name"], "A2");
+        assert_eq!(raw["installations"][0]["folderId"], "f");
     }
 
     #[test]
     fn unparseable_entries_are_kept_but_hidden() {
         let dir = tempfile::tempdir().unwrap();
-        let path = dir.path().join("connections.json");
+        let path = dir.path().join("installations.json");
         let qa = serde_json::json!({"id":"q","name":"QA","color":"#fff","environment":"qa","readOnly":false,
-            "apiUrl":"https://api.c:6443","auth":{"method":"token"},"folderId":null,"order":0,"pinned":true});
+            "servers":[{"id":"s1","name":"vllm","baseUrl":"http://vllm:8000"}],"folderId":null,"order":0,"pinned":true});
         let raw_folder =
             serde_json::json!({"id":"rf","name":{"smart":"x"},"parentId":null,"order":1});
         let file = serde_json::json!({"schemaVersion":1,
             "folders":[raw_folder, {"id":"f","name":"F","parentId":"rf","order":0}],
-            "connections":[qa, {"id":"a","name":"A","color":"#fff","environment":"dev","readOnly":false,
-                "apiUrl":"https://api.c:6443","auth":{"method":"token"},"folderId":"rf","order":1}]});
+            "installations":[qa, {"id":"a","name":"A","color":"#fff","environment":"dev","readOnly":false,
+                "servers":[{"id":"s1","name":"vllm","baseUrl":"http://vllm:8000"}],"folderId":"rf","order":1}]});
         std::fs::write(&path, serde_json::to_vec(&file).unwrap()).unwrap();
 
-        let store = ConnectionStore::open(dir.path()).unwrap();
+        let store = InstallationStore::open(dir.path()).unwrap();
         let list = store.list();
         assert_eq!(list.len(), 1);
         assert_eq!(list[0].id, "a");
@@ -559,7 +569,7 @@ mod tests {
         store.upsert(a).unwrap();
         let written: serde_json::Value =
             serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
-        let conns = written["connections"].as_array().unwrap();
+        let conns = written["installations"].as_array().unwrap();
         assert_eq!(conns.len(), 2);
         assert!(conns.contains(&qa));
         assert!(conns.iter().any(|c| c["id"] == "a" && c["name"] == "A2"));
@@ -568,18 +578,18 @@ mod tests {
         assert!(fl.contains(&raw_folder));
 
         // После перезапуска то же самое: запись ничего не теряет.
-        let reopened = ConnectionStore::open(dir.path()).unwrap();
+        let reopened = InstallationStore::open(dir.path()).unwrap();
         assert_eq!(reopened.list().len(), 1);
         reopened.delete("a").unwrap();
         let written: serde_json::Value =
             serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
-        assert_eq!(written["connections"], serde_json::json!([qa]));
+        assert_eq!(written["installations"], serde_json::json!([qa]));
     }
 
     #[test]
-    fn new_connection_goes_to_end_of_its_folder() {
+    fn new_installation_goes_to_end_of_its_folder() {
         let dir = tempfile::tempdir().unwrap();
-        let store = ConnectionStore::open(dir.path()).unwrap();
+        let store = InstallationStore::open(dir.path()).unwrap();
         store.upsert(sample_profile("a")).unwrap();
         store.upsert(sample_profile("b")).unwrap();
         assert_eq!(store.get("b").unwrap().order, 1);
@@ -588,15 +598,15 @@ mod tests {
     #[test]
     fn dangling_folder_reference_is_rejected_on_write() {
         let dir = tempfile::tempdir().unwrap();
-        let store = ConnectionStore::open(dir.path()).unwrap();
+        let store = InstallationStore::open(dir.path()).unwrap();
         let mut p = sample_profile("a");
         p.folder_id = Some("missing".into());
         let e = store.upsert(p).unwrap_err();
         assert_eq!(e.key, "errors.folders.notFound");
     }
 
-    fn folder(id: &str, name: &str, parent: Option<&str>) -> ConnectionFolder {
-        ConnectionFolder {
+    fn folder(id: &str, name: &str, parent: Option<&str>) -> InstallationFolder {
+        InstallationFolder {
             id: id.into(),
             name: name.into(),
             parent_id: parent.map(String::from),
@@ -607,7 +617,7 @@ mod tests {
 
     #[test]
     fn check_tree_rejects_bad_folders() {
-        let data = |folders: Vec<ConnectionFolder>| StoreData {
+        let data = |folders: Vec<InstallationFolder>| StoreData {
             folders,
             ..Default::default()
         };
@@ -644,15 +654,15 @@ mod tests {
     fn dangling_references_and_cycles_go_to_root_on_read() {
         let dir = tempfile::tempdir().unwrap();
         std::fs::write(
-            dir.path().join("connections.json"),
+            dir.path().join("installations.json"),
             r##"{"schemaVersion":1,"folders":[
               {"id":"f","name":"F","parentId":"gone"},
               {"id":"x","name":"X","parentId":"y"},{"id":"y","name":"Y","parentId":"x"}],
-              "connections":[{"id":"a","name":"A","color":"#fff","environment":"dev","readOnly":false,
-              "apiUrl":"https://api.c:6443","auth":{"method":"token"},"folderId":"gone"}]}"##,
+              "installations":[{"id":"a","name":"A","color":"#fff","environment":"dev","readOnly":false,
+              "servers":[{"id":"s1","name":"vllm","baseUrl":"http://vllm:8000"}],"folderId":"gone"}]}"##,
         )
         .unwrap();
-        let store = ConnectionStore::open(dir.path()).unwrap();
+        let store = InstallationStore::open(dir.path()).unwrap();
         assert_eq!(store.get("a").unwrap().folder_id, None);
         let folders = store.list_folders();
         assert_eq!(folders[0].parent_id, None);
@@ -665,17 +675,17 @@ mod tests {
         let dir = tempfile::tempdir().unwrap();
         let long = "я".repeat(101);
         std::fs::write(
-            dir.path().join("connections.json"),
+            dir.path().join("installations.json"),
             format!(
                 r##"{{"schemaVersion":1,"folders":[
                   {{"id":"e","name":"   "}},{{"id":"l","name":"{long}"}},
                   {{"id":"p1","name":"Prod"}},{{"id":"p2","name":" prod "}},{{"id":"p3","name":"PROD"}},
                   {{"id":"c","name":"prod","parentId":"p1"}}],
-                  "connections":[]}}"##
+                  "installations":[]}}"##
             ),
         )
         .unwrap();
-        let store = ConnectionStore::open(dir.path()).unwrap();
+        let store = InstallationStore::open(dir.path()).unwrap();
         check_tree(&store.cache.read_safe()).unwrap();
         let names: Vec<_> = store.list_folders().into_iter().map(|f| f.name).collect();
         assert_eq!(names[0], "e");
@@ -698,7 +708,7 @@ mod tests {
     #[test]
     fn upsert_of_existing_profile_keeps_its_place() {
         let dir = tempfile::tempdir().unwrap();
-        let store = ConnectionStore::open(dir.path()).unwrap();
+        let store = InstallationStore::open(dir.path()).unwrap();
         store
             .update(|d| {
                 d.folders.push(folder("f", "F", None));
@@ -732,14 +742,14 @@ mod tests {
     fn cycle_repair_detaches_only_folders_on_the_cycle() {
         let dir = tempfile::tempdir().unwrap();
         std::fs::write(
-            dir.path().join("connections.json"),
+            dir.path().join("installations.json"),
             r##"{"schemaVersion":1,"folders":[
               {"id":"z","name":"Z","parentId":"x"},
               {"id":"x","name":"X","parentId":"y"},{"id":"y","name":"Y","parentId":"x"}],
-              "connections":[]}"##,
+              "installations":[]}"##,
         )
         .unwrap();
-        let store = ConnectionStore::open(dir.path()).unwrap();
+        let store = InstallationStore::open(dir.path()).unwrap();
         let f = store.list_folders();
         assert_eq!(f[0].parent_id.as_deref(), Some("x"));
         let roots = f[1..].iter().filter(|f| f.parent_id.is_none()).count();
@@ -750,19 +760,19 @@ mod tests {
     #[test]
     fn incoming_extra_is_ignored_on_save() {
         let dir = tempfile::tempdir().unwrap();
-        let store = ConnectionStore::open(dir.path()).unwrap();
+        let store = InstallationStore::open(dir.path()).unwrap();
         // Новый профиль: неизвестные поля из запроса не пишутся.
         let mut p = sample_profile("a");
         p.extra.insert("token".into(), serde_json::json!("x"));
         p.extra.insert("pinned".into(), serde_json::json!(true));
         assert!(store.upsert(p).unwrap().extra.is_empty());
         assert!(store.get("a").unwrap().extra.is_empty());
-        let raw = std::fs::read_to_string(dir.path().join("connections.json")).unwrap();
+        let raw = std::fs::read_to_string(dir.path().join("installations.json")).unwrap();
         assert!(!raw.contains("\"token\":") && !raw.contains("pinned"));
         // Существующий профиль всегда сохраняет свои неизвестные поля.
         store
             .update(|d| {
-                d.connections[0]
+                d.installations[0]
                     .extra
                     .insert("passwordRotationDays".into(), serde_json::json!(30));
                 Ok(())
@@ -780,7 +790,41 @@ mod tests {
             serde_json::Value::Object(saved.extra),
             serde_json::json!({"passwordRotationDays": 30})
         );
-        let raw = std::fs::read_to_string(dir.path().join("connections.json")).unwrap();
+        let raw = std::fs::read_to_string(dir.path().join("installations.json")).unwrap();
         assert!(!raw.contains("clientSecret"));
     }
+
+    #[test]
+    fn servers_keep_their_unknown_fields_and_new_ones_get_none() {
+        let dir = tempfile::tempdir().unwrap();
+        let store = InstallationStore::open(dir.path()).unwrap();
+        let mut p = sample_profile("a");
+        p.servers[0]
+            .extra
+            .insert("apiKey".into(), serde_json::json!("x"));
+        let saved = store.upsert(p).unwrap();
+        assert!(
+            saved.servers[0].extra.is_empty(),
+            "новый сервер — без чужих полей"
+        );
+        store
+            .update(|d| {
+                d.installations[0].servers[0]
+                    .extra
+                    .insert("gpuNode".into(), serde_json::json!("n1"));
+                Ok(())
+            })
+            .unwrap();
+        let mut edited = store.get("a").unwrap();
+        edited.servers[0].extra.clear();
+        edited.servers[0].name = "renamed".into();
+        let mut added = crate::storage::profile::sample_server("s2");
+        added.extra.insert("secret".into(), serde_json::json!("x"));
+        edited.servers.push(added);
+        let saved = store.upsert(edited).unwrap();
+        assert_eq!(saved.servers[0].extra["gpuNode"], "n1");
+        assert!(saved.servers[1].extra.is_empty());
+        let raw = std::fs::read_to_string(dir.path().join("installations.json")).unwrap();
+        assert!(!raw.contains("secret") && !raw.contains("apiKey"), "{raw}");
+    }
 }
````

<!-- block: patch t04-port-src-tauri-src-storage-tree-rs step=4.1 -->
````diff
diff --git a/src-tauri/src/storage/tree.rs b/src-tauri/src/storage/tree.rs
--- a/src-tauri/src/storage/tree.rs
+++ b/src-tauri/src/storage/tree.rs
@@ -1,13 +1,13 @@
-//! Операции над деревом папок подключений. Каждая операция — одна транзакция
-//! `ConnectionStore::update`: инварианты (ссылки, циклы, имена) проверяет `check_tree`.
+//! Операции над деревом папок инсталляций. Каждая операция — одна транзакция
+//! `InstallationStore::update`: инварианты (ссылки, циклы, имена) проверяет `check_tree`.
 
 use std::collections::HashSet;
 
 use serde::{Deserialize, Serialize};
 
 use crate::error::{AppResult, ErrorCode};
-use crate::storage::connections::{
-    ConnectionFolder, ConnectionStore, StoreData, connection_not_found, folder_not_found,
+use crate::storage::installations::{
+    InstallationFolder, InstallationStore, StoreData, folder_not_found, installation_not_found,
     validate_folder_name,
 };
 
@@ -15,10 +15,10 @@ use crate::storage::connections::{
 #[serde(rename_all = "lowercase")]
 pub enum TreeItemKind {
     Folder,
-    Connection,
+    Installation,
 }
 
-/// Ссылка на элемент дерева: папку или подключение.
+/// Ссылка на элемент дерева: папку или инсталляцию.
 #[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
 #[serde(rename_all = "camelCase")]
 pub struct TreeItemRef {
@@ -30,14 +30,14 @@ impl TreeItemRef {
     fn exists(&self, data: &StoreData) -> AppResult<()> {
         let found = match self.kind {
             TreeItemKind::Folder => data.folders.iter().any(|f| f.id == self.id),
-            TreeItemKind::Connection => data.connections.iter().any(|c| c.id == self.id),
+            TreeItemKind::Installation => data.installations.iter().any(|c| c.id == self.id),
         };
         if found {
             return Ok(());
         }
         Err(match self.kind {
             TreeItemKind::Folder => folder_not_found(&self.id),
-            TreeItemKind::Connection => connection_not_found(&self.id),
+            TreeItemKind::Installation => installation_not_found(&self.id),
         })
     }
 }
@@ -52,13 +52,13 @@ fn arrange(mut level: Vec<String>, moved: Vec<String>, before: Option<&str>) ->
     level
 }
 
-impl ConnectionStore {
+impl InstallationStore {
     /// Создаёт папку в конце уровня `parent_id`.
     pub fn create_folder(
         &self,
         name: &str,
         parent_id: Option<&str>,
-    ) -> AppResult<ConnectionFolder> {
+    ) -> AppResult<InstallationFolder> {
         validate_folder_name(name)?;
         self.update(|data| {
             let parent_id = parent_id.map(str::to_string);
@@ -69,7 +69,7 @@ impl ConnectionStore {
                 .map(|f| f.order + 1)
                 .max()
                 .unwrap_or(0);
-            let folder = ConnectionFolder {
+            let folder = InstallationFolder {
                 id: uuid::Uuid::new_v4().to_string(),
                 name: name.trim().to_string(),
                 parent_id,
@@ -81,7 +81,7 @@ impl ConnectionStore {
         })
     }
 
-    pub fn rename_folder(&self, id: &str, name: &str) -> AppResult<ConnectionFolder> {
+    pub fn rename_folder(&self, id: &str, name: &str) -> AppResult<InstallationFolder> {
         validate_folder_name(name)?;
         self.update(|data| {
             let folder = data
@@ -94,7 +94,7 @@ impl ConnectionStore {
         })
     }
 
-    /// Удаляет только пустую папку: без вложенных папок и подключений.
+    /// Удаляет только пустую папку: без вложенных папок и инсталляций.
     pub fn delete_folder(&self, id: &str) -> AppResult<()> {
         self.update(|data| {
             if !data.folders.iter().any(|f| f.id == id) {
@@ -105,14 +105,14 @@ impl ConnectionStore {
                 .iter()
                 .any(|f| f.parent_id.as_deref() == Some(id))
                 || data
-                    .connections
+                    .installations
                     .iter()
                     .any(|c| c.folder_id.as_deref() == Some(id));
             if busy {
                 return Err(app_err!(
                     ErrorCode::InvalidInput,
                     "errors.folders.notEmpty",
-                    "The folder is not empty: move or delete its connections and folders first"
+                    "The folder is not empty: move or delete its installations and folders first"
                 ));
             }
             data.folders.retain(|f| f.id != id);
@@ -121,7 +121,7 @@ impl ConnectionStore {
     }
 
     /// Переносит элементы в `parent_id` и ставит их перед `before` (или в конец уровня).
-    /// Папки и подключения уровня нумеруются раздельно, `order` = 0..n; `before` другого вида,
+    /// Папки и инсталляции уровня нумеруются раздельно, `order` = 0..n; `before` другого вида,
     /// не лежащий на уровне или сам перемещаемый, означает «в конец».
     pub fn move_items(
         &self,
@@ -147,7 +147,7 @@ impl ConnectionStore {
             let target = parent_id.map(str::to_string);
 
             let moved_folders = moved(TreeItemKind::Folder);
-            let mut level: Vec<&ConnectionFolder> = data
+            let mut level: Vec<&InstallationFolder> = data
                 .folders
                 .iter()
                 .filter(|f| f.parent_id == target && !moved_folders.contains(&f.id))
@@ -162,16 +162,16 @@ impl ConnectionStore {
                 }
             }
 
-            let moved_conns = moved(TreeItemKind::Connection);
+            let moved_conns = moved(TreeItemKind::Installation);
             let mut level: Vec<_> = data
-                .connections
+                .installations
                 .iter()
                 .filter(|c| c.folder_id == target && !moved_conns.contains(&c.id))
                 .collect();
             level.sort_by_key(|c| c.order);
             let level = level.into_iter().map(|c| c.id.clone()).collect();
-            let order = arrange(level, moved_conns, before_of(TreeItemKind::Connection));
-            for c in &mut data.connections {
+            let order = arrange(level, moved_conns, before_of(TreeItemKind::Installation));
+            for c in &mut data.installations {
                 if let Some(i) = order.iter().position(|id| *id == c.id) {
                     c.folder_id = target.clone();
                     c.order = i as i64;
@@ -205,11 +205,11 @@ impl ConnectionStore {
 #[cfg(test)]
 mod tests {
     use super::*;
-    use crate::storage::connections::{ConnectionStore, sample_profile};
+    use crate::storage::installations::{InstallationStore, sample_profile};
 
-    fn store() -> (tempfile::TempDir, ConnectionStore) {
+    fn store() -> (tempfile::TempDir, InstallationStore) {
         let d = tempfile::tempdir().unwrap();
-        let s = ConnectionStore::open(d.path()).unwrap();
+        let s = InstallationStore::open(d.path()).unwrap();
         (d, s)
     }
 
@@ -262,14 +262,14 @@ mod tests {
     }
 
     #[test]
-    fn move_connections_and_reorder() {
+    fn move_installations_and_reorder() {
         let (_d, s) = store();
         for id in ["a", "b", "c"] {
             s.upsert(sample_profile(id)).unwrap();
         }
         let f = s.create_folder("F", None).unwrap();
         let conn = |id: &str| TreeItemRef {
-            kind: TreeItemKind::Connection,
+            kind: TreeItemKind::Installation,
             id: id.into(),
         };
         s.move_items(&[conn("c"), conn("a")], Some(&f.id), None)
@@ -342,15 +342,15 @@ mod tests {
         let (_d, s) = store();
         s.upsert(sample_profile("a")).unwrap();
         let conn = TreeItemRef {
-            kind: TreeItemKind::Connection,
+            kind: TreeItemKind::Installation,
             id: "nope".into(),
         };
         assert_eq!(
             s.move_items(&[conn], None, None).unwrap_err().key,
-            "errors.connections.notFound"
+            "errors.installations.notFound"
         );
         let a = TreeItemRef {
-            kind: TreeItemKind::Connection,
+            kind: TreeItemKind::Installation,
             id: "a".into(),
         };
         assert_eq!(
````

<!-- block: patch t04-port-src-tauri-src-commands-folders-rs step=4.1 -->
````diff
diff --git a/src-tauri/src/commands/folders.rs b/src-tauri/src/commands/folders.rs
--- a/src-tauri/src/commands/folders.rs
+++ b/src-tauri/src/commands/folders.rs
@@ -5,18 +5,19 @@ use time::format_description::well_known::Rfc3339;
 
 use super::blocking;
 use crate::error::{AppError, AppResult};
+use crate::secrets::{API_KEY, secret_key};
 use crate::state::AppState;
-use crate::storage::connections::ConnectionFolder;
 use crate::storage::exchange::{
     ExportSummary, ImportOptions, ImportPreview, StoredSecrets, build_export, parse_exchange,
     plan_import,
 };
 use crate::storage::fs::write_json;
+use crate::storage::installations::InstallationFolder;
 use crate::storage::tree::TreeItemRef;
 
 #[tauri::command]
-pub fn list_folders(state: State<'_, AppState>) -> AppResult<Vec<ConnectionFolder>> {
-    Ok(state.connections.list_folders())
+pub fn list_folders(state: State<'_, AppState>) -> AppResult<Vec<InstallationFolder>> {
+    Ok(state.installations.list_folders())
 }
 
 #[tauri::command]
@@ -24,9 +25,9 @@ pub async fn create_folder(
     state: State<'_, AppState>,
     name: String,
     parent_id: Option<String>,
-) -> AppResult<ConnectionFolder> {
+) -> AppResult<InstallationFolder> {
     let st = state.inner().clone();
-    blocking(move || st.connections.create_folder(&name, parent_id.as_deref())).await
+    blocking(move || st.installations.create_folder(&name, parent_id.as_deref())).await
 }
 
 #[tauri::command]
@@ -34,15 +35,15 @@ pub async fn rename_folder(
     state: State<'_, AppState>,
     id: String,
     name: String,
-) -> AppResult<ConnectionFolder> {
+) -> AppResult<InstallationFolder> {
     let st = state.inner().clone();
-    blocking(move || st.connections.rename_folder(&id, &name)).await
+    blocking(move || st.installations.rename_folder(&id, &name)).await
 }
 
 #[tauri::command]
 pub async fn delete_folder(state: State<'_, AppState>, id: String) -> AppResult<()> {
     let st = state.inner().clone();
-    blocking(move || st.connections.delete_folder(&id)).await
+    blocking(move || st.installations.delete_folder(&id)).await
 }
 
 #[tauri::command]
@@ -54,15 +55,15 @@ pub async fn move_items(
 ) -> AppResult<()> {
     let st = state.inner().clone();
     blocking(move || {
-        st.connections
+        st.installations
             .move_items(&items, parent_id.as_deref(), before.as_ref())
     })
     .await
 }
 
-/// Пишет файл `glafira-kubernetes-connections` (всё дерево или папку `folder_id`) без секретов.
+/// Пишет файл `glafira-inference-installations` (всё дерево или папку `folder_id`) без секретов.
 #[tauri::command]
-pub async fn export_connections(
+pub async fn export_installations(
     state: State<'_, AppState>,
     path: String,
     folder_id: Option<String>,
@@ -84,7 +85,7 @@ fn export_to_file(
     now: &str,
 ) -> AppResult<ExportSummary> {
     let out = build_export(
-        &st.connections,
+        &st.installations,
         st.secrets.as_ref(),
         folder_id,
         env!("CARGO_PKG_VERSION"),
@@ -92,7 +93,7 @@ fn export_to_file(
     )?;
     write_json(path, &out)?;
     Ok(ExportSummary {
-        connections: out.connections.len(),
+        installations: out.installations.len(),
         folders: out.folders.len(),
     })
 }
@@ -151,12 +152,11 @@ fn import_from_file(
         target,
         update_existing,
         allow_insecure_tls: st.policy.status.allow_insecure_tls,
-        allow_exec_auth: st.policy.status.allow_exec_auth,
     };
-    let snapshot = st.connections.snapshot();
+    let snapshot = st.installations.snapshot();
     // Keychain читается до `update`: запрос доступа (на macOS — окно с паролем) не держит
-    // блокировку записи хранилища, под которой ждали бы `list_connections` и `connections.get`.
-    // Без «Обновлять существующие» сохранённые подключения не меняются — предупреждать не о чем,
+    // блокировку записи хранилища, под которой ждали бы `list_installations` и `installations.get`.
+    // Без «Обновлять существующие» сохранённые инсталляции не меняются — предупреждать не о чем,
     // и keychain не читается вовсе.
     let stored = if update_existing {
         StoredSecrets::read(st.secrets.as_ref(), &snapshot, &parsed)
@@ -166,16 +166,20 @@ fn import_from_file(
     if !apply {
         return Ok(plan_import(&snapshot, &parsed, &opts, &stored)?.preview);
     }
-    let preview = st.connections.update(|data| {
+    let (preview, removed) = st.installations.update(|data| {
         let plan = plan_import(data, &parsed, &opts, &stored)?;
         *data = plan.next;
-        Ok(plan.preview)
+        Ok((plan.preview, plan.removed_servers))
     })?;
-    // Обновлённые подключения переподключатся с новым профилем, их сессии на прежнем клиенте
-    // останавливаются — как при сохранении формы.
-    for item in &preview.updated {
-        st.sessions.stop_for(&item.id);
-        st.evict_connection(&item.id);
+    // Ключи API серверов, которых больше нет в профиле, не нужны. Сбой keychain импорт не
+    // отменяет: профиль уже записан, а лишняя запись в keychain ничего не открывает.
+    for (installation, server) in removed {
+        if let Err(e) = st
+            .secrets
+            .delete(&secret_key(&installation, &server, API_KEY))
+        {
+            tracing::warn!("api key of a removed server was not deleted: {e}");
+        }
     }
     Ok(preview)
 }
@@ -185,54 +189,55 @@ mod tests {
     use std::sync::Arc;
 
     use super::*;
-    use crate::secrets::{MemoryStore, SecretStore, secret_key};
-    use crate::storage::connections::sample_profile;
+    use crate::secrets::{MemoryStore, SecretStore};
+    use crate::storage::installations::sample_profile;
 
     #[test]
     fn export_writes_file_without_secrets() {
         let dir = tempfile::tempdir().unwrap();
         let secrets = Arc::new(MemoryStore::default());
         secrets
-            .set(&secret_key("a", crate::secrets::TOKEN), "s3cr3t")
+            .set(&secret_key("a", "s1", API_KEY), "s3cr3t")
             .unwrap();
         let st = AppState::new(dir.path(), secrets).unwrap();
-        let f = st.connections.create_folder("ИФТ", None).unwrap();
+        let f = st.installations.create_folder("ИФТ", None).unwrap();
         let mut p = sample_profile("a");
         p.folder_id = Some(f.id.clone());
-        st.connections.upsert(p).unwrap();
-        st.connections.upsert(sample_profile("b")).unwrap();
+        st.installations.upsert(p).unwrap();
+        st.installations.upsert(sample_profile("b")).unwrap();
 
-        let path = dir.path().join("out.glafira-kubernetes-connections.json");
-        let sum = export_to_file(&st, &path, Some(&f.id), "2026-10-17T12:00:00Z").unwrap();
+        let path = dir.path().join("out.glafira-inference-installations.json");
+        let sum = export_to_file(&st, &path, Some(&f.id), "2026-10-29T12:00:00Z").unwrap();
         assert_eq!(
             sum,
             ExportSummary {
-                connections: 1,
+                installations: 1,
                 folders: 1
             }
         );
         let text = std::fs::read_to_string(&path).unwrap();
         assert!(!text.contains("s3cr3t"));
         let v: serde_json::Value = serde_json::from_str(&text).unwrap();
-        assert_eq!(v["format"], "glafira-kubernetes-connections");
+        assert_eq!(v["format"], "glafira-inference-installations");
         assert_eq!(v["appVersion"], env!("CARGO_PKG_VERSION"));
-        assert_eq!(v["connections"][0]["id"], "a");
+        assert_eq!(v["installations"][0]["id"], "a");
+        assert_eq!(v["installations"][0]["needsSecret"]["apiKeys"][0], "s1");
 
         let err = export_to_file(&st, &path, Some("nope"), "now").unwrap_err();
         assert_eq!(err.key, "errors.folders.notFound");
     }
 
-    fn write_file(dir: &Path, api_url: &str) -> std::path::PathBuf {
+    fn write_file(dir: &Path, base_url: &str) -> std::path::PathBuf {
         let mut p = serde_json::to_value(sample_profile("a")).unwrap();
-        p["apiUrl"] = api_url.into();
+        p["servers"][0]["baseUrl"] = base_url.into();
         let mut n = serde_json::to_value(sample_profile("n")).unwrap();
         n["environment"] = "prod".into();
-        let path = dir.join("in.glafira-kubernetes-connections.json");
+        let path = dir.join("in.glafira-inference-installations.json");
         std::fs::write(
             &path,
             serde_json::to_vec(&serde_json::json!({
-                "format": "glafira-kubernetes-connections", "version": 1,
-                "folders": [], "connections": [p, n],
+                "format": "glafira-inference-installations", "version": 1,
+                "folders": [], "installations": [p, n],
             }))
             .unwrap(),
         )
@@ -244,69 +249,63 @@ mod tests {
     fn preview_does_not_write_and_apply_writes_once() {
         let dir = tempfile::tempdir().unwrap();
         let st = AppState::new(dir.path(), Arc::new(MemoryStore::default())).unwrap();
-        st.connections.upsert(sample_profile("a")).unwrap();
+        st.installations.upsert(sample_profile("a")).unwrap();
         let input = tempfile::tempdir().unwrap();
-        let path = write_file(input.path(), "https://api.changed:6443");
-        let conns = dir.path().join("connections.json");
+        let path = write_file(input.path(), "http://changed:8000");
+        let conns = dir.path().join("installations.json");
         let before = std::fs::read(&conns).unwrap();
 
-        let (_, watch) = st.sessions.register("a");
         let preview = import_from_file(&st, &path, None, true, false).unwrap();
         assert_eq!(preview.updated.len(), 1);
         assert_eq!(preview.added.len(), 1);
-        // prod-подключение импортируется как в файле: без принудительного read-only.
+        // prod-инсталляция импортируется как в файле: без принудительного read-only.
         assert!(preview.warnings.is_empty());
         assert_eq!(std::fs::read(&conns).unwrap(), before);
-        assert!(!watch.is_cancelled(), "предпросмотр ничего не меняет");
 
         let applied = import_from_file(&st, &path, None, true, true).unwrap();
         assert_eq!(applied, preview);
-        assert!(
-            watch.is_cancelled(),
-            "сессии обновлённого подключения остановлены"
-        );
         assert_eq!(
-            st.connections.get("a").unwrap().api_url,
-            "https://api.changed:6443"
+            st.installations.get("a").unwrap().servers[0].base_url,
+            "http://changed:8000"
         );
-        assert!(!st.connections.get("n").unwrap().read_only);
-        let reopened = crate::storage::connections::ConnectionStore::open(dir.path()).unwrap();
+        assert!(!st.installations.get("n").unwrap().read_only);
+        let reopened = crate::storage::installations::InstallationStore::open(dir.path()).unwrap();
         assert_eq!(reopened.list().len(), 2);
     }
 
     #[test]
-    fn changed_address_of_a_connection_with_a_token_is_warned_about() {
+    fn changed_address_of_a_server_with_an_api_key_is_warned_about() {
         let dir = tempfile::tempdir().unwrap();
         let secrets = Arc::new(MemoryStore::default());
         secrets
-            .set(&secret_key("a", crate::secrets::TOKEN), "sha256~t")
+            .set(&secret_key("a", "s1", API_KEY), "sk-t")
             .unwrap();
         let st = AppState::new(dir.path(), secrets.clone()).unwrap();
-        st.connections.upsert(sample_profile("a")).unwrap();
-        let path = write_file(dir.path(), "https://api.changed:6443");
+        st.installations.upsert(sample_profile("a")).unwrap();
+        let path = write_file(dir.path(), "http://changed:8000");
 
         let preview = import_from_file(&st, &path, None, true, false).unwrap();
         assert_eq!(preview.warnings.len(), 1);
         assert_eq!(preview.warnings[0].key, "messages.import.addressChanged");
-        assert_eq!(preview.warnings[0].params["name"], "conn-a");
-        // Результат применения содержит то же предупреждение, токен остаётся на месте.
+        assert_eq!(preview.warnings[0].params["name"], "inst-a");
+        // Результат применения содержит то же предупреждение, ключ остаётся на месте.
         let applied = import_from_file(&st, &path, None, true, true).unwrap();
         assert_eq!(applied, preview);
         assert_eq!(
             secrets
-                .get(&secret_key("a", crate::secrets::TOKEN))
+                .get(&secret_key("a", "s1", API_KEY))
                 .unwrap()
                 .as_deref(),
-            Some("sha256~t")
+            Some("sk-t")
         );
     }
 
     /// Keychain, который запоминает для каждого чтения, держал ли кто-то в этот момент
-    /// блокировку записи хранилища подключений.
+    /// блокировку записи хранилища инсталляций.
     #[derive(Default)]
     struct LockProbe {
         inner: MemoryStore,
-        store: std::sync::OnceLock<Arc<crate::storage::connections::ConnectionStore>>,
+        store: std::sync::OnceLock<Arc<crate::storage::installations::InstallationStore>>,
         reads: std::sync::Mutex<Vec<bool>>,
     }
 
@@ -325,25 +324,25 @@ mod tests {
     }
 
     /// Применение импорта читает keychain до блокировки записи хранилища: запрос доступа к
-    /// keychain не задерживает `list_connections` и `connections.get`.
+    /// keychain не задерживает `list_installations` и `installations.get`.
     #[test]
     fn import_reads_the_keychain_outside_the_store_lock() {
         let dir = tempfile::tempdir().unwrap();
         let probe = Arc::new(LockProbe::default());
         probe
             .inner
-            .set(&secret_key("a", crate::secrets::TOKEN), "sha256~t")
+            .set(&secret_key("a", "s1", API_KEY), "sk-t")
             .unwrap();
         let st = AppState::new(dir.path(), probe.clone()).unwrap();
-        assert!(probe.store.set(st.connections.clone()).is_ok());
-        st.connections.upsert(sample_profile("a")).unwrap();
+        assert!(probe.store.set(st.installations.clone()).is_ok());
+        st.installations.upsert(sample_profile("a")).unwrap();
         probe.reads.lock().unwrap().clear();
-        let path = write_file(dir.path(), "https://api.changed:6443");
+        let path = write_file(dir.path(), "http://changed:8000");
 
         let applied = import_from_file(&st, &path, None, true, true).unwrap();
         assert_eq!(applied.warnings[0].key, "messages.import.addressChanged");
         let reads = probe.reads.lock().unwrap().clone();
-        assert!(!reads.is_empty(), "токен прочитан");
+        assert!(!reads.is_empty(), "ключ прочитан");
         assert!(reads.iter().all(|locked| !locked), "{reads:?}");
     }
 
@@ -354,9 +353,9 @@ mod tests {
         let dir = tempfile::tempdir().unwrap();
         let probe = Arc::new(LockProbe::default());
         let st = AppState::new(dir.path(), probe.clone()).unwrap();
-        st.connections.upsert(sample_profile("a")).unwrap();
+        st.installations.upsert(sample_profile("a")).unwrap();
         probe.reads.lock().unwrap().clear();
-        let path = write_file(dir.path(), "https://api.changed:6443");
+        let path = write_file(dir.path(), "http://changed:8000");
 
         let preview = import_from_file(&st, &path, None, false, false).unwrap();
         assert!(preview.updated.is_empty() && preview.warnings.is_empty());
@@ -372,9 +371,9 @@ mod tests {
         let dir = tempfile::tempdir().unwrap();
         let data = dir.path().join("data");
         let st = AppState::new(&data, Arc::new(MemoryStore::default())).unwrap();
-        st.connections.upsert(sample_profile("a")).unwrap();
-        let path = write_file(dir.path(), "https://api.changed:6443");
-        let conns = data.join("connections.json");
+        st.installations.upsert(sample_profile("a")).unwrap();
+        let path = write_file(dir.path(), "http://changed:8000");
+        let conns = data.join("installations.json");
         let before = std::fs::read(&conns).unwrap();
 
         // Каталог данных только для чтения: временный файл не создать, запись падает.
@@ -383,15 +382,15 @@ mod tests {
         std::fs::set_permissions(&data, std::fs::Permissions::from_mode(0o755)).unwrap();
         assert!(result.is_err());
         assert_eq!(std::fs::read(&conns).unwrap(), before);
-        assert_eq!(st.connections.list().len(), 1);
+        assert_eq!(st.installations.list().len(), 1);
         assert_eq!(
-            st.connections.get("a").unwrap().api_url,
-            "https://api.dev.example.com:6443"
+            st.installations.get("a").unwrap().servers[0].base_url,
+            "https://s1.dev.example.com:8000"
         );
     }
 
     #[test]
-    fn policy_forbidding_insecure_tls_skips_such_connections() {
+    fn policy_forbidding_insecure_tls_skips_such_installations() {
         let dir = tempfile::tempdir().unwrap();
         let mut policy = crate::policy::Policy::default();
         policy.status.allow_insecure_tls = false;
@@ -399,13 +398,13 @@ mod tests {
             .unwrap()
             .with_policy(policy);
         let mut insecure = serde_json::to_value(sample_profile("i")).unwrap();
-        insecure["ca"] = serde_json::json!({"mode": "insecure"});
-        let path = dir.path().join("in.glafira-kubernetes-connections.json");
+        insecure["servers"][0]["ca"] = serde_json::json!({"mode": "insecure"});
+        let path = dir.path().join("in.glafira-inference-installations.json");
         std::fs::write(
             &path,
             serde_json::to_vec(&serde_json::json!({
-                "format": "glafira-kubernetes-connections", "version": 1,
-                "folders": [], "connections": [insecure],
+                "format": "glafira-inference-installations", "version": 1,
+                "folders": [], "installations": [insecure],
             }))
             .unwrap(),
         )
@@ -416,7 +415,7 @@ mod tests {
             applied.skipped[0].reason.key,
             "errors.import.insecureForbidden"
         );
-        assert!(st.connections.list().is_empty());
+        assert!(st.installations.list().is_empty());
     }
 
     #[test]
@@ -428,4 +427,40 @@ mod tests {
         let err = import_from_file(&st, &path, None, true, false).unwrap_err();
         assert_eq!(err.key, "errors.import.notGlafiraFile");
     }
+
+    #[test]
+    fn keys_of_servers_removed_by_import_are_deleted() {
+        let dir = tempfile::tempdir().unwrap();
+        let secrets = Arc::new(MemoryStore::default());
+        for server in ["s1", "s2"] {
+            secrets
+                .set(&secret_key("a", server, API_KEY), "sk-t")
+                .unwrap();
+        }
+        let st = AppState::new(dir.path(), secrets.clone()).unwrap();
+        let mut two = sample_profile("a");
+        two.servers
+            .push(crate::storage::profile::sample_server("s2"));
+        st.installations.upsert(two).unwrap();
+        let path = write_file(dir.path(), "https://s1.dev.example.com:8000");
+
+        let preview = import_from_file(&st, &path, None, true, false).unwrap();
+        assert_eq!(preview.updated.len(), 1);
+        assert!(
+            secrets
+                .get(&secret_key("a", "s2", API_KEY))
+                .unwrap()
+                .is_some(),
+            "предпросмотр ничего не удаляет"
+        );
+        import_from_file(&st, &path, None, true, true).unwrap();
+        assert_eq!(st.installations.get("a").unwrap().servers.len(), 1);
+        assert_eq!(secrets.get(&secret_key("a", "s2", API_KEY)).unwrap(), None);
+        assert!(
+            secrets
+                .get(&secret_key("a", "s1", API_KEY))
+                .unwrap()
+                .is_some()
+        );
+    }
 }
````

- [ ] **Step 2: Тесты.**

<!-- block: file t04-src-tauri-src-storage-exchange-rs-tests step=4.2 path=src-tauri/src/storage/exchange.rs part=tests -->
````rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::MemoryStore;
    use crate::storage::installations::sample_profile;
    use crate::storage::profile::{TEST_CERT_PEM, pem_certificates, sample_server};

    fn store() -> (tempfile::TempDir, InstallationStore) {
        let d = tempfile::tempdir().unwrap();
        let s = InstallationStore::open(d.path()).unwrap();
        (d, s)
    }

    fn in_folder(id: &str, folder: Option<&str>) -> InstallationProfile {
        let mut p = sample_profile(id);
        p.folder_id = folder.map(String::from);
        p
    }

    /// ИФТ/{gpu-1}, ПСИ; инсталляции: a в ИФТ, b в gpu-1, c в ПСИ, d в корне.
    fn tree(s: &InstallationStore) -> (String, String, String) {
        let ift = s.create_folder("ИФТ", None).unwrap();
        let g1 = s.create_folder("gpu-1", Some(&ift.id)).unwrap();
        let psi = s.create_folder("ПСИ", None).unwrap();
        s.upsert(in_folder("a", Some(&ift.id))).unwrap();
        s.upsert(in_folder("b", Some(&g1.id))).unwrap();
        s.upsert(in_folder("c", Some(&psi.id))).unwrap();
        s.upsert(in_folder("d", None)).unwrap();
        (ift.id, g1.id, psi.id)
    }

    fn ids<T>(items: &[T], f: impl Fn(&T) -> &str) -> Vec<String> {
        let mut v: Vec<String> = items.iter().map(|i| f(i).to_string()).collect();
        v.sort();
        v
    }

    #[test]
    fn exports_whole_tree() {
        let (_d, s) = store();
        let (ift, g1, psi) = tree(&s);
        let out = build_export(
            &s,
            &MemoryStore::default(),
            None,
            "9.9.9",
            "2026-10-29T12:00:00Z",
        )
        .unwrap();
        assert_eq!(out.format, "glafira-inference-installations");
        assert_eq!(out.version, 1);
        assert_eq!(out.app_version, "9.9.9");
        assert_eq!(out.exported_at, "2026-10-29T12:00:00Z");
        let mut want = vec![ift, g1, psi];
        want.sort();
        assert_eq!(ids(&out.folders, |f| &f.id), want);
        assert_eq!(
            ids(&out.installations, |c| &c.profile.id),
            ["a", "b", "c", "d"]
        );

        let v = serde_json::to_value(&out).unwrap();
        assert_eq!(v["format"], "glafira-inference-installations");
        assert!(v["exportedAt"].is_string());
        assert!(v["installations"][0]["needsSecret"]["apiKeys"].is_array());
        assert!(v["installations"][0]["servers"][0]["baseUrl"].is_string());
    }

    #[test]
    fn exports_subtree_with_root_detached() {
        let (_d, s) = store();
        let (ift, g1, _psi) = tree(&s);
        let out = build_export(&s, &MemoryStore::default(), Some(&ift), "1", "now").unwrap();
        let mut want = vec![ift.clone(), g1.clone()];
        want.sort();
        assert_eq!(ids(&out.folders, |f| &f.id), want);
        let root = out.folders.iter().find(|f| f.id == ift).unwrap();
        assert_eq!(root.parent_id, None);
        let child = out.folders.iter().find(|f| f.id == g1).unwrap();
        assert_eq!(child.parent_id.as_deref(), Some(ift.as_str()));
        assert_eq!(ids(&out.installations, |c| &c.profile.id), ["a", "b"]);
    }

    #[test]
    fn api_keys_are_never_exported_but_flagged_per_server() {
        let (_d, s) = store();
        let mut two = sample_profile("two");
        two.servers.push(sample_server("s2"));
        s.upsert(two).unwrap();
        s.upsert(sample_profile("empty")).unwrap();
        let secrets = MemoryStore::default();
        secrets
            .set(&secret_key("two", "s2", API_KEY), "sk-s3cr3t")
            .unwrap();
        // Пустое значение и ключ удалённого сервера флаг не поднимают.
        secrets.set(&secret_key("two", "s1", API_KEY), "").unwrap();
        secrets
            .set(&secret_key("empty", "gone", API_KEY), "sk-old")
            .unwrap();
        let out = build_export(&s, &secrets, None, "1", "now").unwrap();
        let json = serde_json::to_string(&out).unwrap();
        assert!(
            !json.contains("s3cr3t") && !json.contains("sk-old"),
            "{json}"
        );
        let flags = |id: &str| {
            out.installations
                .iter()
                .find(|c| c.profile.id == id)
                .unwrap()
                .needs_secret
                .clone()
        };
        assert_eq!(flags("two").api_keys, ["s2"]);
        assert_eq!(flags("empty"), NeedsSecret::default());
    }

    #[test]
    fn unknown_local_fields_are_not_exported() {
        let (_d, s) = store();
        s.upsert(sample_profile("a")).unwrap();
        s.update(|data| {
            data.installations[0]
                .extra
                .insert("futureField".into(), serde_json::json!("local"));
            data.installations[0].servers[0]
                .extra
                .insert("futureServerField".into(), serde_json::json!("local"));
            Ok(())
        })
        .unwrap();
        let out = build_export(&s, &MemoryStore::default(), None, "1", "now").unwrap();
        assert!(out.installations[0].profile.extra.is_empty());
        let text = serde_json::to_string(&out).unwrap();
        assert!(!text.contains("futureField") && !text.contains("futureServerField"));
    }

    #[test]
    fn unknown_root_is_an_error() {
        let (_d, s) = store();
        tree(&s);
        let err = build_export(&s, &MemoryStore::default(), Some("nope"), "1", "now")
            .err()
            .unwrap();
        assert_eq!(err.key, "errors.folders.notFound");
    }

    // ---- импорт ----

    fn inst_json(id: &str, name: &str, folder: Option<&str>) -> serde_json::Value {
        let mut p = sample_profile(id);
        p.name = name.into();
        p.folder_id = folder.map(String::from);
        let mut v = serde_json::to_value(&p).unwrap();
        v["needsSecret"] = serde_json::json!({"apiKeys": []});
        v
    }

    fn folder_json(id: &str, name: &str, parent: Option<&str>, order: i64) -> serde_json::Value {
        serde_json::json!({"id": id, "name": name, "parentId": parent, "order": order})
    }

    fn file(folders: Vec<serde_json::Value>, insts: Vec<serde_json::Value>) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "format": "glafira-inference-installations",
            "version": 1,
            "exportedAt": "now",
            "appVersion": "1",
            "folders": folders,
            "installations": insts,
        }))
        .unwrap()
    }

    fn local_folder(id: &str, name: &str, parent: Option<&str>) -> InstallationFolder {
        InstallationFolder {
            id: id.into(),
            name: name.into(),
            parent_id: parent.map(String::from),
            order: 0,
            extra: Default::default(),
        }
    }

    fn opts(target: Option<&str>) -> ImportOptions<'_> {
        ImportOptions {
            target,
            update_existing: true,
            allow_insecure_tls: true,
        }
    }

    fn plan(data: &StoreData, bytes: &[u8], o: &ImportOptions) -> ImportPlan {
        let parsed = parse_exchange(bytes).unwrap();
        plan_with(data, &parsed, o, &MemoryStore::default()).unwrap()
    }

    /// План импорта, как в приложении: сохранённые секреты читаются до плана.
    fn plan_with(
        data: &StoreData,
        parsed: &ParsedExchange,
        o: &ImportOptions,
        secrets: &dyn SecretStore,
    ) -> AppResult<ImportPlan> {
        plan_import(data, parsed, o, &StoredSecrets::read(secrets, data, parsed))
    }

    fn inst<'a>(data: &'a StoreData, id: &str) -> &'a InstallationProfile {
        data.installations.iter().find(|c| c.id == id).unwrap()
    }

    fn folder<'a>(data: &'a StoreData, id: &str) -> &'a InstallationFolder {
        data.folders.iter().find(|f| f.id == id).unwrap()
    }

    fn preview_ids(items: &[PreviewItem]) -> Vec<&str> {
        items.iter().map(|i| i.id.as_str()).collect()
    }

    #[test]
    fn new_folders_and_installations_go_under_target_keeping_structure() {
        let data = StoreData {
            folders: vec![local_folder("t", "Target", None)],
            ..Default::default()
        };
        let bytes = file(
            vec![
                folder_json("g1", "gpu-1", Some("ift"), 0),
                folder_json("ift", "ИФТ", None, 0),
            ],
            vec![
                inst_json("a", "A", Some("ift")),
                inst_json("b", "B", Some("g1")),
                inst_json("c", "C", None),
            ],
        );
        let p = plan(&data, &bytes, &opts(Some("t")));
        let next = &p.next;
        assert_eq!(folder(next, "ift").parent_id.as_deref(), Some("t"));
        assert_eq!(folder(next, "g1").parent_id.as_deref(), Some("ift"));
        assert_eq!(inst(next, "a").folder_id.as_deref(), Some("ift"));
        assert_eq!(inst(next, "b").folder_id.as_deref(), Some("g1"));
        assert_eq!(inst(next, "c").folder_id.as_deref(), Some("t"));
        assert_eq!(preview_ids(&p.preview.added), ["ift", "g1", "a", "b", "c"]);
        let b = p.preview.added.iter().find(|i| i.id == "b").unwrap();
        assert_eq!(b.kind, TreeItemKind::Installation);
        assert_eq!(b.path, ["Target", "ИФТ", "gpu-1"]);
        assert!(p.preview.updated.is_empty() && p.preview.unchanged.is_empty());
        assert!(p.removed_servers.is_empty());
        check_tree(next).unwrap();
    }

    #[test]
    fn unknown_target_is_an_error() {
        let parsed = parse_exchange(&file(vec![], vec![])).unwrap();
        let err = plan_with(
            &StoreData::default(),
            &parsed,
            &opts(Some("nope")),
            &MemoryStore::default(),
        )
        .err()
        .unwrap();
        assert_eq!(err.key, "errors.folders.notFound");
    }

    #[test]
    fn matched_installation_is_updated_only_with_update_existing() {
        let mut local = sample_profile("a");
        local.folder_id = Some("f".into());
        local.order = 7;
        local.extra.insert("pinned".into(), serde_json::json!(true));
        local.servers[0]
            .extra
            .insert("gpuNode".into(), serde_json::json!("n1"));
        let data = StoreData {
            folders: vec![local_folder("f", "F", None)],
            installations: vec![local.clone()],
            ..Default::default()
        };
        let mut changed = inst_json("a", "inst-a", None);
        changed["servers"][0]["baseUrl"] = "http://other:8000".into();
        let bytes = file(vec![], vec![changed]);

        let p = plan(&data, &bytes, &opts(None));
        let a = inst(&p.next, "a");
        assert_eq!(a.servers[0].base_url, "http://other:8000");
        assert_eq!(a.folder_id.as_deref(), Some("f"));
        assert_eq!(a.order, 7);
        assert_eq!(a.extra.get("pinned"), Some(&serde_json::json!(true)));
        assert_eq!(
            a.servers[0].extra["gpuNode"], "n1",
            "поле сервера — локальное"
        );
        assert_eq!(preview_ids(&p.preview.updated), ["a"]);
        assert_eq!(p.preview.updated[0].path, ["F"]);

        let p = plan(
            &data,
            &bytes,
            &ImportOptions {
                update_existing: false,
                ..opts(None)
            },
        );
        assert_eq!(inst(&p.next, "a"), &local);
        assert_eq!(preview_ids(&p.preview.unchanged), ["a"]);

        // Те же настройки — «без изменений» и при update_existing.
        let same = file(vec![], vec![inst_json("a", "inst-a", None)]);
        let p = plan(&data, &same, &opts(None));
        assert_eq!(preview_ids(&p.preview.unchanged), ["a"]);
        assert_eq!(inst(&p.next, "a"), &local);
    }

    #[test]
    fn servers_missing_from_the_file_are_reported_for_key_removal() {
        let mut local = sample_profile("a");
        local.servers.push(sample_server("s2"));
        let data = StoreData {
            installations: vec![local],
            ..Default::default()
        };
        let mut incoming = inst_json("a", "inst-a", None);
        incoming["servers"][0]["name"] = "renamed".into();
        let p = plan(&data, &file(vec![], vec![incoming]), &opts(None));
        assert_eq!(inst(&p.next, "a").servers.len(), 1);
        assert_eq!(p.removed_servers, [("a".to_string(), "s2".to_string())]);
    }

    /// Локальное хранилище с инсталляцией `a` и план импорта файла, где `a` изменена `edit`.
    fn plan_update(
        local: InstallationProfile,
        secrets: &dyn SecretStore,
        edit: impl FnOnce(&mut serde_json::Value),
    ) -> ImportPreview {
        let data = StoreData {
            installations: vec![local],
            ..Default::default()
        };
        let mut changed = inst_json("a", "inst-a", None);
        edit(&mut changed);
        let parsed = parse_exchange(&file(vec![], vec![changed])).unwrap();
        let p = plan_with(&data, &parsed, &opts(None), secrets).unwrap();
        assert_eq!(preview_ids(&p.preview.updated), ["a"]);
        p.preview
    }

    fn with_key() -> MemoryStore {
        let secrets = MemoryStore::default();
        secrets
            .set(&secret_key("a", "s1", API_KEY), "sk-t")
            .unwrap();
        secrets
    }

    fn warning_keys(preview: &ImportPreview) -> Vec<&str> {
        preview.warnings.iter().map(|w| w.key.as_str()).collect()
    }

    #[test]
    fn address_change_with_a_stored_api_key_warns() {
        let p = plan_update(sample_profile("a"), &with_key(), |c| {
            c["servers"][0]["baseUrl"] = "http://other:8000".into();
        });
        assert_eq!(warning_keys(&p), ["messages.import.addressChanged"]);
        let w = &p.warnings[0];
        assert_eq!(w.params["name"], "inst-a");
        assert_eq!(w.params["server"], "vllm-s1");
        assert_eq!(w.params["from"], "https://s1.dev.example.com:8000");
        assert_eq!(w.params["to"], "http://other:8000");
    }

    /// Хранилище секретов, которое не читается (keychain заблокирован или отказал) и
    /// записывает, какие ключи у него спрашивали.
    #[derive(Default)]
    struct FailingStore {
        asked: std::sync::Mutex<Vec<String>>,
    }

    impl SecretStore for FailingStore {
        fn get(&self, key: &str) -> AppResult<Option<String>> {
            self.asked.lock().unwrap().push(key.to_string());
            Err(crate::error::AppError::internal_with("keychain is locked"))
        }
        fn set(&self, _: &str, _: &str) -> AppResult<()> {
            Err(crate::error::AppError::internal_with("keychain is locked"))
        }
        fn delete(&self, _: &str) -> AppResult<()> {
            Err(crate::error::AppError::internal_with("keychain is locked"))
        }
    }

    /// Сбой keychain считается «ключ есть»: лишнее предупреждение лучше пропущенного.
    #[test]
    fn keychain_error_counts_as_a_stored_key() {
        let secrets = FailingStore::default();
        let p = plan_update(sample_profile("a"), &secrets, |c| {
            c["servers"][0]["baseUrl"] = "http://other:8000".into();
        });
        assert_eq!(warning_keys(&p), ["messages.import.addressChanged"]);
        assert_eq!(
            *secrets.asked.lock().unwrap(),
            [secret_key("a", "s1", API_KEY)]
        );
    }

    /// Ключи читаются только у серверов, которые обновятся с новым направлением; то, чего
    /// не прочитали заранее, считается сохранённым.
    #[test]
    fn stored_keys_are_read_only_where_a_warning_is_possible() {
        let mut two = sample_profile("a");
        two.servers.push(sample_server("s2"));
        let data = StoreData {
            installations: vec![two, sample_profile("same")],
            ..Default::default()
        };
        let mut a = inst_json("a", "inst-a", None);
        a["servers"] = serde_json::json!([
            serde_json::to_value(sample_server("s1")).unwrap(),
            {"id": "s2", "name": "vllm-s2", "baseUrl": "http://moved:8000"},
            {"id": "s3", "name": "vllm-s3", "baseUrl": "http://new:8000"},
        ]);
        let mut same = inst_json("same", "inst-same", None);
        same["readOnly"] = true.into();
        let parsed = parse_exchange(&file(vec![], vec![a, same])).unwrap();
        let secrets = FailingStore::default();
        let stored = StoredSecrets::read(&secrets, &data, &parsed);
        assert_eq!(
            *secrets.asked.lock().unwrap(),
            [secret_key("a", "s2", API_KEY)]
        );
        assert!(stored.has("a", "s2"), "сбой keychain — «есть»");
        assert!(
            stored.has("a", "not-read"),
            "не прочитанное заранее — «есть»"
        );
        let p = plan_import(&data, &parsed, &opts(None), &stored).unwrap();
        assert_eq!(preview_ids(&p.preview.updated), ["a", "same"]);
        assert_eq!(warning_keys(&p.preview), ["messages.import.addressChanged"]);
        assert_eq!(p.preview.warnings[0].params["server"], "vllm-s2");
    }

    #[test]
    fn address_change_without_a_stored_key_does_not_warn() {
        let p = plan_update(sample_profile("a"), &MemoryStore::default(), |c| {
            c["servers"][0]["baseUrl"] = "http://other:8000".into();
        });
        assert!(p.warnings.is_empty());
        // Ключ другого сервера не в счёт.
        let secrets = MemoryStore::default();
        secrets
            .set(&secret_key("a", "s9", API_KEY), "sk-t")
            .unwrap();
        let p = plan_update(sample_profile("a"), &secrets, |c| {
            c["servers"][0]["baseUrl"] = "http://other:8000".into();
        });
        assert!(p.warnings.is_empty());
    }

    #[test]
    fn unchanged_address_does_not_warn() {
        let p = plan_update(sample_profile("a"), &with_key(), |c| {
            c["environment"] = "prod".into();
            c["readOnly"] = true.into();
        });
        assert!(p.warnings.is_empty());
        // Адрес с «/» на конце тот же адрес.
        let p = plan_update(sample_profile("a"), &with_key(), |c| {
            c["servers"][0]["baseUrl"] = "https://s1.dev.example.com:8000/".into();
            c["environment"] = "prod".into();
        });
        assert!(p.warnings.is_empty());
    }

    #[test]
    fn proxy_and_certificate_changes_warn_separately() {
        let p = plan_update(sample_profile("a"), &with_key(), |c| {
            c["servers"][0]["proxy"] =
                serde_json::json!({"mode": "url", "url": "http://proxy.other:3128"});
        });
        assert_eq!(warning_keys(&p), ["messages.import.proxyChanged"]);
        assert!(
            !p.warnings[0].text.contains("proxy.other"),
            "адрес прокси не печатается"
        );
        let p = plan_update(sample_profile("a"), &with_key(), |c| {
            c["servers"][0]["ca"] = serde_json::json!({"mode": "pem", "pem": TEST_CERT_PEM});
        });
        assert_eq!(warning_keys(&p), ["messages.import.tlsChanged"]);
        let p = plan_update(sample_profile("a"), &with_key(), |c| {
            c["servers"][0]["baseUrl"] = "http://other:8000".into();
            c["servers"][0]["proxy"] = serde_json::json!({"mode": "none"});
            c["servers"][0]["ca"] = serde_json::json!({"mode": "insecure"});
        });
        assert_eq!(
            warning_keys(&p),
            [
                "messages.import.addressChanged",
                "messages.import.proxyChanged",
                "messages.import.tlsChanged"
            ]
        );
    }

    #[test]
    fn added_installations_and_disabled_updates_do_not_warn() {
        let mut c = inst_json("a", "inst-a", None);
        c["servers"][0]["baseUrl"] = "http://other:8000".into();
        let bytes = file(vec![], vec![c.clone()]);
        let parsed = parse_exchange(&bytes).unwrap();
        let p = plan_with(&StoreData::default(), &parsed, &opts(None), &with_key()).unwrap();
        assert_eq!(preview_ids(&p.preview.added), ["a"]);
        assert!(p.preview.warnings.is_empty());
        let data = StoreData {
            installations: vec![sample_profile("a")],
            ..Default::default()
        };
        let off = ImportOptions {
            update_existing: false,
            ..opts(None)
        };
        let p = plan_with(&data, &parsed, &off, &with_key()).unwrap();
        assert_eq!(preview_ids(&p.preview.unchanged), ["a"]);
        assert!(p.preview.warnings.is_empty());
    }

    #[test]
    fn matched_folder_is_not_renamed_or_moved_and_receives_new_installations() {
        let data = StoreData {
            folders: vec![
                local_folder("p", "Parent", None),
                local_folder("f", "Local name", Some("p")),
            ],
            ..Default::default()
        };
        let bytes = file(
            vec![folder_json("f", "Renamed", None, 0)],
            vec![inst_json("n", "New", Some("f"))],
        );
        let p = plan(&data, &bytes, &opts(None));
        let f = folder(&p.next, "f");
        assert_eq!(f.name, "Local name");
        assert_eq!(f.parent_id.as_deref(), Some("p"));
        assert_eq!(inst(&p.next, "n").folder_id.as_deref(), Some("f"));
        assert_eq!(preview_ids(&p.preview.unchanged), ["f"]);
        assert_eq!(p.preview.added[0].path, ["Parent", "Local name"]);
    }

    #[test]
    fn name_conflicts_get_suffixes() {
        let mut x = sample_profile("x");
        x.name = "Qwen".into();
        let mut y = sample_profile("y");
        y.name = "qwen (2)".into();
        let data = StoreData {
            folders: vec![local_folder("l1", "ИФТ", None)],
            installations: vec![x, y],
            ..Default::default()
        };
        let bytes = file(
            vec![folder_json("n1", "ифт", None, 0)],
            vec![inst_json("n", "Qwen", None)],
        );
        let p = plan(&data, &bytes, &opts(None));
        assert_eq!(folder(&p.next, "n1").name, "ифт (2)");
        assert_eq!(inst(&p.next, "n").name, "Qwen (3)");
    }

    #[test]
    fn foreign_local_installations_are_untouched() {
        let mut mine = sample_profile("mine");
        mine.order = 3;
        let data = StoreData {
            installations: vec![mine.clone()],
            ..Default::default()
        };
        let p = plan(
            &data,
            &file(vec![], vec![inst_json("a", "A", None)]),
            &opts(None),
        );
        assert_eq!(inst(&p.next, "mine"), &mine);
        assert_eq!(inst(&p.next, "a").order, 4);
    }

    #[test]
    fn prod_read_only_comes_from_the_file() {
        let mut prod = inst_json("p", "P", None);
        prod["environment"] = "prod".into();
        let bytes = file(vec![], vec![prod]);
        let p = plan(&StoreData::default(), &bytes, &opts(None));
        assert!(!inst(&p.next, "p").read_only);
        assert!(p.preview.warnings.is_empty());
    }

    #[test]
    fn duplicate_installation_ids_are_skipped_with_reason() {
        let bytes = file(
            vec![],
            vec![
                inst_json("a", "First", None),
                inst_json("a", "Second", None),
            ],
        );
        let parsed = parse_exchange(&bytes).unwrap();
        assert_eq!(parsed.installations.len(), 1);
        assert_eq!(parsed.installations[0].0.name, "First");
        assert_eq!(parsed.skipped[0].name, "Second");
        assert_eq!(parsed.skipped[0].reason.key, "errors.import.duplicateId");
    }

    #[test]
    fn insecure_servers_are_skipped_when_policy_forbids() {
        let mut insecure = inst_json("i", "Insecure", None);
        insecure["servers"][0]["ca"] = serde_json::json!({"mode": "insecure"});
        let bytes = file(vec![], vec![insecure, inst_json("ok", "OK", None)]);
        let parsed = parse_exchange(&bytes).unwrap();
        let p = plan_with(
            &StoreData::default(),
            &parsed,
            &opts(None),
            &MemoryStore::default(),
        )
        .unwrap();
        assert_eq!(preview_ids(&p.preview.added), ["i", "ok"]);
        let forbid = ImportOptions {
            allow_insecure_tls: false,
            ..opts(None)
        };
        let p = plan_with(
            &StoreData::default(),
            &parsed,
            &forbid,
            &MemoryStore::default(),
        )
        .unwrap();
        assert_eq!(preview_ids(&p.preview.added), ["ok"]);
        assert_eq!(p.preview.skipped[0].name, "Insecure");
        assert_eq!(
            p.preview.skipped[0].reason.key,
            "errors.import.insecureForbidden"
        );
    }

    #[test]
    fn needs_secrets_counts_new_installations_only() {
        let mut a = inst_json("a", "A", None);
        a["needsSecret"]["apiKeys"] = serde_json::json!(["s1"]);
        let mut b = inst_json("b", "B", None);
        b["needsSecret"]["apiKeys"] = serde_json::json!(["s1"]);
        b["servers"][0]["baseUrl"] = "http://changed:8000".into();
        let data = StoreData {
            installations: vec![sample_profile("b")],
            ..Default::default()
        };
        let p = plan(&data, &file(vec![], vec![a, b]), &opts(None));
        assert_eq!(preview_ids(&p.preview.updated), ["b"]);
        assert_eq!(p.preview.needs_secrets, 1);
    }

    #[test]
    fn parse_exchange_tolerates_unknown_fields_and_skips_unsupported() {
        let mut ok = inst_json("ok", "OK", None);
        ok["futureField"] = serde_json::json!({"x": 1});
        ok["servers"][0]["futureServerField"] = 1.into();
        let mut future = inst_json("future", "Future", None);
        future["environment"] = "qa".into();
        let mut invalid = inst_json("bad", "Bad", None);
        invalid["servers"][0]["baseUrl"] = "ftp://h:21".into();
        let mut v: serde_json::Value =
            serde_json::from_slice(&file(vec![], vec![ok, future, invalid])).unwrap();
        v["somethingNew"] = serde_json::json!([1, 2]);
        let parsed = parse_exchange(&serde_json::to_vec(&v).unwrap()).unwrap();
        assert_eq!(parsed.installations.len(), 1);
        assert_eq!(parsed.installations[0].0.id, "ok");
        assert!(parsed.installations[0].0.extra.is_empty());
        assert!(parsed.installations[0].0.servers[0].extra.is_empty());
        assert_eq!(parsed.skipped.len(), 2);
        assert_eq!(parsed.skipped[0].name, "Future");
        assert_eq!(
            parsed.skipped[0].reason.key,
            "errors.import.unsupportedInstallation"
        );
        assert_eq!(parsed.skipped[1].name, "Bad");
        assert_eq!(
            parsed.skipped[1].reason.key,
            "errors.installations.badServerUrl"
        );

        v["version"] = 2.into();
        let err = parse_exchange(&serde_json::to_vec(&v).unwrap())
            .err()
            .unwrap();
        assert_eq!(err.key, "errors.import.incompatibleVersion");
        v["version"] = 1.into();
        v["format"] = "glafira-kubernetes-connections".into();
        let err = parse_exchange(&serde_json::to_vec(&v).unwrap())
            .err()
            .unwrap();
        assert_eq!(err.key, "errors.import.notGlafiraFile");
        let err = parse_exchange(b"{not json").err().unwrap();
        assert_eq!(err.key, "errors.import.invalidJson");
        let err = parse_exchange(b"[1, 2]").err().unwrap();
        assert_eq!(err.key, "errors.import.notGlafiraFile");
    }

    /// Импортируемый профиль приводится к каноническому виду до проверки, как при сохранении
    /// в форме.
    #[test]
    fn imported_profiles_are_normalized_before_validation() {
        let mut c = inst_json("n", "N", None);
        c["servers"][0]["baseUrl"] = " http://gpu-1:8000/ ".into();
        c["servers"][0]["metricsPath"] = "stats".into();
        let parsed = parse_exchange(&file(vec![], vec![c])).unwrap();
        assert!(parsed.skipped.is_empty(), "{:?}", parsed.skipped);
        let s = &parsed.installations[0].0.servers[0];
        assert_eq!(s.base_url, "http://gpu-1:8000");
        assert_eq!(s.metrics_path, "/stats");
    }

    #[test]
    fn export_then_import_into_empty_store_round_trips() {
        let (_d, s) = store();
        let (_ift, _g1, psi) = tree(&s);
        let mut x = sample_profile("x");
        x.folder_id = Some(psi.clone());
        s.upsert(x).unwrap();
        let secrets = MemoryStore::default();
        secrets
            .set(&secret_key("x", "s1", API_KEY), "sk-s3cr3t")
            .unwrap();
        let out = build_export(&s, &secrets, None, "1", "now").unwrap();
        let bytes = serde_json::to_vec(&out).unwrap();

        let parsed = parse_exchange(&bytes).unwrap();
        assert!(parsed.skipped.is_empty());
        let p = plan_with(
            &StoreData::default(),
            &parsed,
            &opts(None),
            &MemoryStore::default(),
        )
        .unwrap();
        assert_eq!(p.preview.needs_secrets, 1);
        let sort_f = |mut v: Vec<InstallationFolder>| {
            v.sort_by(|a, b| a.id.cmp(&b.id));
            v
        };
        let sort_c = |mut v: Vec<InstallationProfile>| {
            v.sort_by(|a, b| a.id.cmp(&b.id));
            v
        };
        assert_eq!(sort_f(p.next.folders.clone()), sort_f(s.list_folders()));
        assert_eq!(sort_c(p.next.installations.clone()), sort_c(s.list()));
    }

    #[test]
    fn private_keys_in_stored_ca_fields_never_reach_the_export() {
        let key = include_str!("testdata/test-key.pem");
        let mut p = sample_profile("k");
        p.servers[0].ca = CaConfig::Pem {
            pem: format!("{key}\n{TEST_CERT_PEM}"),
        };
        // Профиль, записанный в обход проверки (файл правили руками).
        let dir = tempfile::tempdir().unwrap();
        let stored = serde_json::json!({
            "schemaVersion": 1,
            "folders": [],
            "installations": [serde_json::to_value(&p).unwrap()],
        });
        std::fs::write(
            dir.path().join("installations.json"),
            serde_json::to_vec(&stored).unwrap(),
        )
        .unwrap();
        let s = InstallationStore::open(dir.path()).unwrap();
        assert_eq!(s.get("k").unwrap().servers[0].ca, p.servers[0].ca);

        let out = build_export(&s, &MemoryStore::default(), None, "1", "now").unwrap();
        let text = serde_json::to_string(&out).unwrap();
        assert!(!text.contains("PRIVATE KEY"), "{text}");
        let CaConfig::Pem { pem } = &out.installations[0].profile.servers[0].ca else {
            panic!("pem")
        };
        assert_eq!(
            pem_certificates(pem).unwrap(),
            pem_certificates(TEST_CERT_PEM).unwrap()
        );
        let parsed = parse_exchange(&serde_json::to_vec(&out).unwrap()).unwrap();
        assert!(parsed.skipped.is_empty());
    }
}
````

<!-- block: file t04-src-tauri-src-storage-profile-rs-tests step=4.2 path=src-tauri/src/storage/profile.rs part=tests -->
````rust
#[cfg(test)]
mod tests {
    use super::*;

    fn assert_invalid(p: &InstallationProfile, key: &str) -> AppError {
        let e = validate_profile(p).unwrap_err();
        assert_eq!((e.code, e.key.as_str()), (ErrorCode::InvalidInput, key));
        e
    }

    fn cert_with_key() -> String {
        format!("{TEST_CERT_PEM}\n{}", include_str!("testdata/test-key.pem"))
    }

    #[test]
    fn json_shape_matches_frontend_contract() {
        let v = serde_json::to_value(sample_profile("a")).unwrap();
        assert_eq!(
            v,
            serde_json::json!({
                "id": "a",
                "name": "inst-a",
                "color": "#4f8cff",
                "environment": "dev",
                "readOnly": false,
                "servers": [{
                    "id": "s1",
                    "name": "vllm-s1",
                    "baseUrl": "https://s1.dev.example.com:8000",
                    "kind": "auto",
                    "metricsPath": "/metrics",
                    "ca": { "mode": "system" },
                    "proxy": { "mode": "system" }
                }],
                "requestTimeoutMs": 30000,
                "connectTimeoutMs": 10000,
                "folderId": null,
                "order": 0
            })
        );
    }

    #[test]
    fn minimal_json_gets_defaults_and_keeps_unknown_fields() {
        let p: InstallationProfile = serde_json::from_value(serde_json::json!({
            "id": "a", "name": "n", "color": "#fff", "environment": "prod", "readOnly": true,
            "servers": [{ "id": "s", "name": "x", "baseUrl": "http://h:8000", "future": 1 }],
            "slo": { "ttftP95Ms": 500 }
        }))
        .unwrap();
        let s = &p.servers[0];
        assert_eq!(s.kind, ServerKind::Auto);
        assert_eq!(s.metrics_path, "/metrics");
        assert_eq!((&s.ca, &s.proxy), (&CaConfig::System, &ProxyConfig::System));
        assert_eq!(
            (p.request_timeout_ms, p.connect_timeout_ms),
            (30_000, 10_000)
        );
        assert_eq!(s.extra["future"], 1, "поле сервера из новой версии");
        assert_eq!(
            p.extra["slo"]["ttftP95Ms"], 500,
            "поле профиля из новой версии"
        );
        let back = serde_json::to_value(&p).unwrap();
        assert_eq!(back["slo"]["ttftP95Ms"], 500);
        assert_eq!(back["servers"][0]["future"], 1);
    }

    #[test]
    fn normalization_trims_names_urls_and_metrics_path() {
        let mut p = sample_profile("a");
        p.name = "  prod LLM ".into();
        p.servers[0].name = " gpu-1 ".into();
        p.servers[0].base_url = " http://gpu-1:8000/v/ ".into();
        p.servers[0].metrics_path = " stats ".into();
        normalize_profile(&mut p);
        assert_eq!(p.name, "prod LLM");
        assert_eq!(p.servers[0].name, "gpu-1");
        assert_eq!(p.servers[0].base_url, "http://gpu-1:8000/v");
        assert_eq!(p.servers[0].metrics_path, "/stats");
        p.servers[0].metrics_path = "   ".into();
        normalize_profile(&mut p);
        assert_eq!(p.servers[0].metrics_path, "/metrics");
    }

    #[test]
    fn server_ids() {
        for ok in ["s1", "a-b_C", &"x".repeat(64)] {
            assert!(is_valid_server_id(ok), "{ok}");
        }
        for bad in ["", "a/b", "a b", "сервер", &"x".repeat(65), "../x"] {
            assert!(!is_valid_server_id(bad), "{bad}");
        }
    }

    #[test]
    fn validation_accepts_sample_and_http_and_rejects_bad_fields() {
        assert!(validate_profile(&sample_profile("a")).is_ok());
        let mut p = sample_profile("a");
        p.servers[0].base_url = "http://10.0.0.5:8000".into();
        assert!(
            validate_profile(&p).is_ok(),
            "http — частый случай в контуре"
        );
        p.servers[0].base_url = "https://gw.corp/llm/qwen".into();
        assert!(validate_profile(&p).is_ok(), "путь за обратным прокси");

        type Change = Box<dyn Fn(&mut InstallationProfile)>;
        let cases: Vec<(&str, Change)> = vec![
            ("errors.installations.noId", Box::new(|p| p.id = " ".into())),
            (
                "errors.installations.emptyName",
                Box::new(|p| p.name = "".into()),
            ),
            (
                "errors.installations.noServers",
                Box::new(|p| p.servers.clear()),
            ),
            (
                "errors.installations.tooManyServers",
                Box::new(|p| {
                    p.servers = (0..=MAX_SERVERS)
                        .map(|i| sample_server(&format!("s{i}")))
                        .collect()
                }),
            ),
            (
                "errors.installations.badServerId",
                Box::new(|p| p.servers[0].id = "a/b".into()),
            ),
            (
                "errors.installations.emptyServerName",
                Box::new(|p| p.servers[0].name = " ".into()),
            ),
            (
                "errors.installations.badServerUrl",
                Box::new(|p| p.servers[0].base_url = "ftp://h:21".into()),
            ),
            (
                "errors.installations.badServerUrl",
                Box::new(|p| p.servers[0].base_url = "vllm:8000".into()),
            ),
            (
                "errors.installations.badServerUrl",
                Box::new(|p| p.servers[0].base_url = "http://h:8000/?x=1".into()),
            ),
            (
                "errors.installations.badServerUrl",
                Box::new(|p| p.servers[0].base_url = "http://h:8000/#f".into()),
            ),
            (
                "errors.installations.badMetricsPath",
                Box::new(|p| p.servers[0].metrics_path = "metrics".into()),
            ),
            (
                "errors.installations.badMetricsPath",
                Box::new(|p| p.servers[0].metrics_path = "/metrics?x=1".into()),
            ),
            (
                "errors.installations.badMetricsPath",
                Box::new(|p| p.servers[0].metrics_path = "/a b".into()),
            ),
            (
                "errors.installations.caPemInvalid",
                Box::new(|p| p.servers[0].ca = CaConfig::Pem { pem: "nope".into() }),
            ),
            (
                "errors.installations.badProxyUrl",
                Box::new(|p| {
                    p.servers[0].proxy = ProxyConfig::Url {
                        url: "proxy:3128".into(),
                    }
                }),
            ),
            (
                "errors.installations.duplicateServerId",
                Box::new(|p| {
                    let mut s = sample_server("s1");
                    s.name = "other".into();
                    p.servers.push(s)
                }),
            ),
            (
                "errors.installations.duplicateServerName",
                Box::new(|p| {
                    let mut s = sample_server("s2");
                    s.name = " VLLM-S1 ".into();
                    p.servers.push(s)
                }),
            ),
            (
                "errors.installations.timeoutTooSmall",
                Box::new(|p| p.connect_timeout_ms = 999),
            ),
        ];
        for (key, change) in cases {
            let mut p = sample_profile("a");
            change(&mut p);
            assert_invalid(&p, key);
        }
    }

    #[test]
    fn server_errors_name_the_server() {
        let mut p = sample_profile("a");
        p.servers[0].base_url = "ftp://h".into();
        let e = assert_invalid(&p, "errors.installations.badServerUrl");
        assert_eq!(e.params["server"], "vllm-s1");
        assert_eq!(e.params["url"], "ftp://h");
    }

    #[test]
    fn private_key_in_the_ca_field_is_rejected() {
        for tag in [
            "PRIVATE KEY",
            "RSA PRIVATE KEY",
            "EC PRIVATE KEY",
            "ENCRYPTED PRIVATE KEY",
        ] {
            let mut p = sample_profile("a");
            p.servers[0].ca = CaConfig::Pem {
                pem: format!("{TEST_CERT_PEM}\n-----BEGIN {tag}-----\nAAAA\n-----END {tag}-----\n"),
            };
            assert_invalid(&p, "errors.installations.pemContainsKey");
        }
        let mut p = sample_profile("a");
        p.servers[0].ca = CaConfig::Pem {
            pem: cert_with_key(),
        };
        assert_invalid(&p, "errors.installations.pemContainsKey");
        p.servers[0].ca = CaConfig::Pem {
            pem: format!("{TEST_CERT_PEM}\n{TEST_CERT_PEM}"),
        };
        assert!(validate_profile(&p).is_ok());
    }

    #[test]
    fn credentials_in_urls_are_rejected_without_echoing_them() {
        let mut p = sample_profile("a");
        p.servers[0].base_url = "https://admin:s3cr3t@vllm:8000".into();
        let e = assert_invalid(&p, "errors.installations.credentialsInUrl");
        assert_eq!(e.params["url"], "https://vllm:8000");
        let mut p = sample_profile("a");
        p.servers[0].proxy = ProxyConfig::Url {
            url: "http://u:s3cr3t@proxy:3128".into(),
        };
        let e = assert_invalid(&p, "errors.installations.credentialsInUrl");
        assert_eq!(e.params["url"], "http://proxy:3128");
        for (url, shown) in [
            ("ftp://u:s3cr3t@vllm:8000", "ftp://vllm:8000"),
            ("u:s3cr3t@vllm:8000", "vllm:8000"),
            ("http://u:s3cr3t@vl lm:8000", "http://vl lm:8000"),
        ] {
            let mut p = sample_profile("a");
            p.servers[0].base_url = url.into();
            let e = assert_invalid(&p, "errors.installations.badServerUrl");
            assert_eq!(e.params["url"], shown, "{url}");
            let all = serde_json::to_string(&e).unwrap();
            assert!(!all.contains("s3cr3t"), "{url}: {all}");
        }
    }

    #[test]
    fn strip_private_keys_keeps_only_the_certificates() {
        let key = include_str!("testdata/test-key.pem");
        let body = key.lines().nth(1).unwrap();
        let want = pem_certificates(TEST_CERT_PEM).unwrap();
        for text in [
            cert_with_key(),
            format!("{key}\n{TEST_CERT_PEM}"),
            format!("{TEST_CERT_PEM}\n-----BEGIN EC PRIVATE KEY-----\nAAAA\n"),
            format!("{TEST_CERT_PEM}\n-----BEGIN RSA PRIVATE KEY-----\nAAAA\n-----END"),
        ] {
            assert!(pem_has_private_key(&text), "{text}");
            let stripped = strip_private_keys(&text);
            assert!(
                !stripped.contains("PRIVATE KEY") && !stripped.contains(body),
                "{stripped}"
            );
            assert_eq!(pem_certificates(&stripped).unwrap(), want, "{stripped}");
        }
        let plain = format!("# CA\n{TEST_CERT_PEM}\n{TEST_CERT_PEM}");
        assert!(!pem_has_private_key(&plain));
        assert_eq!(strip_private_keys(&plain), plain);
        let nested = format!("-----BEGIN CERTIFICATE-----\nAAAA\n{key}");
        assert!(pem_has_private_key(&nested));
        assert!(!strip_private_keys(&nested).contains(body));
    }

    #[test]
    fn profile_json_never_contains_secrets() {
        let v = serde_json::to_string(&sample_profile("a")).unwrap();
        assert!(!v.contains("apiKey") && !v.contains("token"), "{v}");
    }
}
````

- [ ] **Step 3: Реализация.**

<!-- block: file t04-src-tauri-src-storage-exchange-rs-impl step=4.3 path=src-tauri/src/storage/exchange.rs part=impl -->
````rust
//! Файл обмена инсталляциями (`glafira-inference-installations`): экспорт без секретов, предпросмотр и
//! применение импорта.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::error::{AppResult, ErrorCode};
use crate::i18n::LocalizedText;
use crate::secrets::{API_KEY, SecretStore, secret_key};
use crate::storage::installations::{
    InstallationFolder, InstallationProfile, InstallationStore, StoreData, check_tree,
    folder_not_found, unique_name, validate_profile,
};
use crate::storage::profile::{CaConfig, ServerConfig, normalize_profile, strip_private_keys};
use crate::storage::tree::TreeItemKind;

pub const EXCHANGE_FORMAT: &str = "glafira-inference-installations";
pub const EXCHANGE_VERSION: u32 = 1;

/// Какие секреты были сохранены у инсталляции при экспорте: получатель вводит свои.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NeedsSecret {
    /// Серверы (id), у которых был сохранён ключ API.
    #[serde(default)]
    pub api_keys: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeFileOut<'a> {
    pub format: &'a str,
    pub version: u32,
    pub exported_at: String,
    pub app_version: &'a str,
    pub folders: Vec<InstallationFolder>,
    pub installations: Vec<ExportedInstallation>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ExportedInstallation {
    #[serde(flatten)]
    pub profile: InstallationProfile,
    #[serde(default)]
    pub needs_secret: NeedsSecret,
}

/// Итог экспорта для фронтенда.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExportSummary {
    pub installations: usize,
    pub folders: usize,
}

/// Сохранён ли непустой ключ API сервера `server` инсталляции `id`.
fn has_api_key(secrets: &dyn SecretStore, id: &str, server: &str) -> AppResult<bool> {
    Ok(secrets
        .get(&secret_key(id, server, API_KEY))?
        .is_some_and(|v| !v.is_empty()))
}

/// Серверы профиля, у которых сохранён ключ API: устаревшие записи keychain (сервер удалён)
/// не требуют ввода у получателя.
fn needs_secret(secrets: &dyn SecretStore, p: &InstallationProfile) -> AppResult<NeedsSecret> {
    let mut api_keys = Vec::new();
    for s in &p.servers {
        if has_api_key(secrets, &p.id, &s.id)? {
            api_keys.push(s.id.clone());
        }
    }
    Ok(NeedsSecret { api_keys })
}

/// Неизвестные поля профиля и серверов — локальные: в файл обмена и из него не переносятся.
fn clear_extra(p: &mut InstallationProfile) {
    p.extra.clear();
    for s in &mut p.servers {
        s.extra.clear();
    }
}

/// Приватный ключ из поля CA в файл обмена не попадает, сертификаты остаются (спец. §5.1:
/// секреты — только в keychain). Профили читаются с диска без повторной проверки (файл могли
/// править руками), поэтому ключ вырезается и здесь.
fn strip_pem_keys_for_export(p: &mut InstallationProfile) {
    for s in &mut p.servers {
        if let CaConfig::Pem { pem } = &mut s.ca {
            *pem = strip_private_keys(pem);
        }
    }
}

/// Собирает файл экспорта: всё дерево или поддерево `root` (сама папка становится корнем).
/// Секреты keychain не попадают в файл; неизвестные локальные поля (`extra`) не выносятся.
pub fn build_export(
    store: &InstallationStore,
    secrets: &dyn SecretStore,
    root: Option<&str>,
    app_version: &'static str,
    now: &str,
) -> AppResult<ExchangeFileOut<'static>> {
    let all_folders = store.list_folders();
    let included: Option<HashSet<String>> = match root {
        None => None,
        Some(root) => {
            if !all_folders.iter().any(|f| f.id == root) {
                return Err(folder_not_found(root));
            }
            let mut set = HashSet::from([root.to_string()]);
            // Дерево без циклов (инвариант хранилища), поэтому достаточно расширять до неподвижной точки.
            loop {
                let before = set.len();
                for f in &all_folders {
                    if f.parent_id.as_deref().is_some_and(|p| set.contains(p)) {
                        set.insert(f.id.clone());
                    }
                }
                if set.len() == before {
                    break;
                }
            }
            Some(set)
        }
    };
    let in_scope = |folder: Option<&str>| match &included {
        None => true,
        Some(set) => folder.is_some_and(|id| set.contains(id)),
    };

    let folders = all_folders
        .into_iter()
        .filter(|f| in_scope(Some(&f.id)))
        .map(|mut f| {
            if Some(f.id.as_str()) == root {
                f.parent_id = None;
            }
            f.extra.clear();
            f
        })
        .collect();

    let installations = store
        .list()
        .into_iter()
        .filter(|p| in_scope(p.folder_id.as_deref()))
        .map(|mut profile| {
            clear_extra(&mut profile);
            strip_pem_keys_for_export(&mut profile);
            let needs_secret = needs_secret(secrets, &profile)?;
            Ok(ExportedInstallation {
                profile,
                needs_secret,
            })
        })
        .collect::<AppResult<Vec<_>>>()?;

    Ok(ExchangeFileOut {
        format: EXCHANGE_FORMAT,
        version: EXCHANGE_VERSION,
        exported_at: now.to_string(),
        app_version,
        folders,
        installations,
    })
}

/// Инсталляция файла, которая не будет импортирована, и причина.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Skipped {
    pub name: String,
    pub reason: LocalizedText,
}

/// Прочитанный файл обмена: папки, инсталляции (с флагами секретов) и пропущенные инсталляции.
#[derive(Debug, Clone, Default)]
pub struct ParsedExchange {
    pub folders: Vec<InstallationFolder>,
    pub installations: Vec<(InstallationProfile, NeedsSecret)>,
    pub skipped: Vec<Skipped>,
}

fn not_glafira_file() -> crate::error::AppError {
    app_err!(
        ErrorCode::InvalidInput,
        "errors.import.notGlafiraFile",
        "This is not a Glafira installations file"
    )
}

fn invalid_json(error: impl std::fmt::Display) -> crate::error::AppError {
    app_err!(
        ErrorCode::InvalidInput,
        "errors.import.invalidJson",
        "The file is damaged or is not JSON: {{error}}",
        error = error
    )
}

/// Имя инсталляции для списка пропущенных: из файла, иначе id, иначе номер в списке.
fn display_name(v: &serde_json::Value, index: usize) -> String {
    ["name", "id"]
        .iter()
        .filter_map(|k| v.get(*k).and_then(|x| x.as_str()))
        .map(str::trim)
        .find(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_else(|| format!("#{}", index + 1))
}

/// Читает файл обмена. Ошибка — только для чужого или повреждённого файла и более новой версии;
/// неизвестные поля игнорируются, неподдерживаемая или невалидная инсталляция попадает в `skipped`.
pub fn parse_exchange(bytes: &[u8]) -> AppResult<ParsedExchange> {
    let root: serde_json::Value = serde_json::from_slice(bytes).map_err(invalid_json)?;
    if root.get("format").and_then(|f| f.as_str()) != Some(EXCHANGE_FORMAT) {
        return Err(not_glafira_file());
    }
    let version = root
        .get("version")
        .and_then(|v| v.as_u64())
        .ok_or_else(not_glafira_file)?;
    if version > u64::from(EXCHANGE_VERSION) {
        return Err(app_err!(
            ErrorCode::InvalidInput,
            "errors.import.incompatibleVersion",
            "The file was created by an incompatible version of Glafira — update the app"
        ));
    }
    let list = |key: &str| -> AppResult<Vec<serde_json::Value>> {
        match root.get(key) {
            None | Some(serde_json::Value::Null) => Ok(Vec::new()),
            Some(serde_json::Value::Array(items)) => Ok(items.clone()),
            Some(_) => Err(invalid_json(format!("“{key}” is not a list"))),
        }
    };

    let mut seen = HashSet::new();
    let mut folders = Vec::new();
    for v in list("folders")? {
        let mut f: InstallationFolder = serde_json::from_value(v).map_err(invalid_json)?;
        // Папка без id пропускается (её инсталляции попадут в цель); из повторов id берётся первая.
        if f.id.trim().is_empty() || !seen.insert(f.id.clone()) {
            continue;
        }
        f.extra.clear();
        let name = f.name.trim();
        f.name = if name.is_empty() { f.id.trim() } else { name }.to_string();
        folders.push(f);
    }

    let mut seen = HashSet::new();
    let mut installations = Vec::new();
    let mut skipped = Vec::new();
    for (i, v) in list("installations")?.into_iter().enumerate() {
        let name = display_name(&v, i);
        let mut c: ExportedInstallation = match serde_json::from_value(v) {
            Ok(c) => c,
            Err(e) => {
                skipped.push(Skipped {
                    name,
                    reason: ltext!(
                        "errors.import.unsupportedInstallation",
                        "Not supported by this version of Glafira (update the app): {{error}}",
                        error = e
                    ),
                });
                continue;
            }
        };
        // Неизвестные поля файла не переносятся в хранилище.
        clear_extra(&mut c.profile);
        // Как при сохранении в форме: имена и адреса без пробелов, адрес без «/» на конце.
        normalize_profile(&mut c.profile);
        if let Err(e) = validate_profile(&c.profile) {
            skipped.push(Skipped {
                name,
                reason: e.text(),
            });
            continue;
        }
        if !seen.insert(c.profile.id.clone()) {
            skipped.push(Skipped {
                name,
                reason: ltext!(
                    "errors.import.duplicateId",
                    "Duplicate installation id in the file"
                ),
            });
            continue;
        }
        installations.push((c.profile, c.needs_secret));
    }
    Ok(ParsedExchange {
        folders,
        installations,
        skipped,
    })
}

pub struct ImportOptions<'a> {
    /// Папка, под которую попадают корневые элементы файла; `None` — корень.
    pub target: Option<&'a str>,
    pub update_existing: bool,
    /// `false` — политика запрещает серверы без проверки сертификата (спец. §13).
    pub allow_insecure_tls: bool,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewItem {
    pub kind: TreeItemKind,
    pub id: String,
    pub name: String,
    /// Имена папок от корня до места элемента (сам элемент не входит).
    pub path: Vec<String>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub added: Vec<PreviewItem>,
    pub updated: Vec<PreviewItem>,
    pub unchanged: Vec<PreviewItem>,
    pub skipped: Vec<Skipped>,
    pub warnings: Vec<LocalizedText>,
    pub needs_secrets: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct ImportPlan {
    pub next: StoreData,
    pub preview: ImportPreview,
    /// Серверы обновлённых инсталляций, которых нет в файле: `(инсталляция, сервер)`. Их ключи
    /// API удаляются из keychain после записи.
    pub removed_servers: Vec<(String, String)>,
}

/// Имена папок от корня до `folder` включительно.
fn path_of(folders: &[InstallationFolder], folder: Option<&str>) -> Vec<String> {
    let mut path = Vec::new();
    let mut seen = HashSet::new();
    let mut cur = folder;
    while let Some(id) = cur {
        if !seen.insert(id) {
            break;
        }
        let Some(f) = folders.iter().find(|f| f.id == id) else {
            break;
        };
        path.push(f.name.clone());
        cur = f.parent_id.as_deref();
    }
    path.reverse();
    path
}

/// Папки файла в порядке «родитель раньше потомка» (внутри уровня — по `order`).
/// Папка с родителем не из файла — корневая в файле; цикл разрывается на первой оставшейся папке.
fn parents_first(folders: &[InstallationFolder]) -> Vec<(&InstallationFolder, bool)> {
    let ids: HashSet<&str> = folders.iter().map(|f| f.id.as_str()).collect();
    let mut rest: Vec<&InstallationFolder> = folders.iter().collect();
    rest.sort_by_key(|f| f.order);
    let mut done: HashSet<&str> = HashSet::new();
    let mut out = Vec::new();
    while !rest.is_empty() {
        let ready = |f: &&InstallationFolder| match f.parent_id.as_deref() {
            None => true,
            Some(p) => !ids.contains(p) || done.contains(p),
        };
        let (mut now, later): (Vec<_>, Vec<_>) = rest.into_iter().partition(ready);
        let mut forced_root = None;
        if now.is_empty() {
            // Цикл в файле: первая оставшаяся папка становится корневой.
            forced_root = Some(later[0].id.as_str());
            now.push(later[0]);
        }
        rest = later
            .into_iter()
            .filter(|f| Some(f.id.as_str()) != forced_root)
            .collect();
        for f in now {
            let root = Some(f.id.as_str()) == forced_root
                || f.parent_id.as_deref().is_none_or(|p| !ids.contains(p));
            out.push((f, root));
            done.insert(f.id.as_str());
        }
    }
    out
}

/// Настройки инсталляции без места в дереве и локальных неизвестных полей.
fn same_settings(local: &InstallationProfile, incoming: &InstallationProfile) -> bool {
    let strip = |p: &InstallationProfile| {
        let mut p = InstallationProfile {
            id: String::new(),
            folder_id: None,
            order: 0,
            ..p.clone()
        };
        clear_extra(&mut p);
        p
    };
    strip(local) == strip(incoming)
}

/// Меняется ли то, куда уходит ключ API сервера: адрес, прокси или проверка сертификата.
fn destination_changes(local: &ServerConfig, new: &ServerConfig) -> bool {
    local.base_url != new.base_url || local.proxy != new.proxy || local.ca != new.ca
}

/// Пары «сервер до импорта — сервер из файла» с тем же id, у которых меняется направление.
fn moved_servers<'a>(
    local: &'a InstallationProfile,
    new: &'a InstallationProfile,
) -> impl Iterator<Item = (&'a ServerConfig, &'a ServerConfig)> {
    new.servers.iter().filter_map(|n| {
        local
            .server(&n.id)
            .filter(|l| destination_changes(l, n))
            .map(|l| (l, n))
    })
}

/// Сохранены ли в keychain ключи API серверов, которые импорт обновит с новым направлением
/// (`destination_changes`). Читается до `InstallationStore::update`: запрос доступа к keychain
/// (на macOS — окно с паролем) не должен ждать под блокировкой записи хранилища, иначе ждали бы
/// и `list_installations` на главном потоке, и `installations.get`.
#[derive(Debug, Default)]
pub(crate) struct StoredSecrets {
    /// (id инсталляции, id сервера) → есть ли сохранённый ключ API.
    by_server: HashMap<(String, String), bool>,
}

impl StoredSecrets {
    /// Читает только нужное: серверы инсталляций, которые есть и в `data`, и в файле, у которых
    /// направление в файле другое. Сбой keychain считается «есть»: лишнее предупреждение лучше
    /// пропущенного.
    pub(crate) fn read(
        secrets: &dyn SecretStore,
        data: &StoreData,
        parsed: &ParsedExchange,
    ) -> Self {
        let mut by_server = HashMap::new();
        for (incoming, _) in &parsed.installations {
            let Some(local) = data.installations.iter().find(|l| l.id == incoming.id) else {
                continue;
            };
            for (_, server) in moved_servers(local, incoming) {
                let stored = has_api_key(secrets, &incoming.id, &server.id).unwrap_or(true);
                by_server.insert((incoming.id.clone(), server.id.clone()), stored);
            }
        }
        Self { by_server }
    }

    /// Сохранён ли ключ сервера `server` инсталляции `id`. Не прочитанное заранее (хранилище
    /// изменилось между чтением и записью) считается сохранённым.
    fn has(&self, id: &str, server: &str) -> bool {
        self.by_server
            .get(&(id.to_string(), server.to_string()))
            .copied()
            .unwrap_or(true)
    }
}

/// Предупреждения об обновлении инсталляции, у сервера которой меняется то, куда уходит
/// сохранённый ключ API: адрес, прокси или проверка сертификата.
fn destination_warnings(
    stored: &StoredSecrets,
    local: &InstallationProfile,
    new: &InstallationProfile,
) -> Vec<LocalizedText> {
    let mut out = Vec::new();
    for (old, server) in moved_servers(local, new) {
        if !stored.has(&new.id, &server.id) {
            continue;
        }
        let (name, srv) = (&new.name, &server.name);
        if old.base_url != server.base_url {
            out.push(ltext!(
                "messages.import.addressChanged",
                "“{{name}}”, server “{{server}}”: the address changes from {{from}} to {{to}}. The saved API key will be sent to the new address.",
                name = name,
                server = srv,
                from = old.base_url,
                to = server.base_url
            ));
        }
        if old.proxy != server.proxy {
            out.push(ltext!(
                "messages.import.proxyChanged",
                "“{{name}}”, server “{{server}}”: the proxy settings change. The saved API key will go through the new proxy.",
                name = name,
                server = srv
            ));
        }
        if old.ca != server.ca {
            out.push(ltext!(
                "messages.import.tlsChanged",
                "“{{name}}”, server “{{server}}”: the certificate verification settings change. The saved API key will be used with the new settings.",
                name = name,
                server = srv
            ));
        }
    }
    out
}

/// Строит новое состояние хранилища и предпросмотр импорта. Функция чистая: разрешение
/// политики приходит в `opts`, а сохранённые секреты — в `stored` (`StoredSecrets::read`, до
/// блокировки хранилища), для предупреждений об обновлении сервера с ключом API.
pub(crate) fn plan_import(
    data: &StoreData,
    parsed: &ParsedExchange,
    opts: &ImportOptions,
    stored: &StoredSecrets,
) -> AppResult<ImportPlan> {
    let target = opts.target.map(str::to_string);
    if let Some(t) = &target
        && !data.folders.iter().any(|f| &f.id == t)
    {
        return Err(folder_not_found(t));
    }
    let mut next = data.clone();
    let mut preview = ImportPreview {
        skipped: parsed.skipped.clone(),
        ..Default::default()
    };
    let mut removed_servers = Vec::new();
    // id папок файла совпадают с id в хранилище: существующие берутся как есть, новые создаются с тем же id.
    let file_folders: HashSet<&str> = parsed.folders.iter().map(|f| f.id.as_str()).collect();
    let local_folders: HashSet<&str> = data.folders.iter().map(|f| f.id.as_str()).collect();
    // (вид, id) в порядке появления, чтобы пути в предпросмотре считались по итоговому дереву.
    let mut added = Vec::new();
    let mut updated = Vec::new();
    let mut unchanged = Vec::new();

    for (f, is_root) in parents_first(&parsed.folders) {
        if local_folders.contains(f.id.as_str()) {
            unchanged.push((TreeItemKind::Folder, f.id.clone()));
            continue;
        }
        let parent = if is_root {
            target.clone()
        } else {
            f.parent_id.clone()
        };
        let siblings: Vec<&str> = next
            .folders
            .iter()
            .filter(|s| s.parent_id == parent)
            .map(|s| s.name.as_str())
            .collect();
        let name = unique_name(&siblings, &f.name);
        let order = next
            .folders
            .iter()
            .filter(|s| s.parent_id == parent)
            .map(|s| s.order + 1)
            .max()
            .unwrap_or(0);
        next.folders.push(InstallationFolder {
            id: f.id.clone(),
            name,
            parent_id: parent,
            order,
            extra: Default::default(),
        });
        added.push((TreeItemKind::Folder, f.id.clone()));
    }

    let mut installations: Vec<&(InstallationProfile, NeedsSecret)> =
        parsed.installations.iter().collect();
    installations.sort_by_key(|(p, _)| p.order);
    for (incoming, needs) in installations {
        if !opts.allow_insecure_tls && incoming.servers.iter().any(|s| s.ca == CaConfig::Insecure) {
            preview.skipped.push(Skipped {
                name: incoming.name.clone(),
                reason: ltext!(
                    "errors.import.insecureForbidden",
                    "Servers without TLS certificate verification are forbidden by your organization"
                ),
            });
            continue;
        }
        let mut profile = incoming.clone();
        if let Some(slot) = next.installations.iter_mut().find(|c| c.id == profile.id) {
            if same_settings(slot, incoming) || !opts.update_existing {
                unchanged.push((TreeItemKind::Installation, profile.id.clone()));
                continue;
            }
            // Место в дереве и неизвестные поля — локальные; ключи API получателя не трогаются.
            profile.folder_id = slot.folder_id.clone();
            profile.order = slot.order;
            profile.extra = std::mem::take(&mut slot.extra);
            for server in &mut profile.servers {
                if let Some(old) = slot.server(&server.id) {
                    server.extra = old.extra.clone();
                }
            }
            removed_servers.extend(
                slot.servers
                    .iter()
                    .filter(|old| profile.server(&old.id).is_none())
                    .map(|old| (profile.id.clone(), old.id.clone())),
            );
            // Адрес, прокси и CA берутся из файла, а ключи API остаются: файл из чужих рук
            // отправил бы их на другой сервер, и человек должен это увидеть.
            preview
                .warnings
                .extend(destination_warnings(stored, slot, &profile));
            updated.push((TreeItemKind::Installation, profile.id.clone()));
            *slot = profile;
            continue;
        }
        let folder = profile
            .folder_id
            .as_deref()
            .filter(|f| file_folders.contains(f))
            .map(str::to_string)
            .or_else(|| target.clone());
        let siblings: Vec<&str> = next
            .installations
            .iter()
            .filter(|c| c.folder_id == folder)
            .map(|c| c.name.as_str())
            .collect();
        let wanted = profile.name.trim();
        let taken = siblings
            .iter()
            .any(|s| s.trim().to_lowercase() == wanted.to_lowercase());
        let name = if taken {
            unique_name(&siblings, wanted)
        } else {
            profile.name.clone()
        };
        profile.order = next
            .installations
            .iter()
            .filter(|c| c.folder_id == folder)
            .map(|c| c.order + 1)
            .max()
            .unwrap_or(0);
        profile.folder_id = folder;
        profile.name = name;
        clear_extra(&mut profile);
        if !needs.api_keys.is_empty() {
            preview.needs_secrets += 1;
        }
        added.push((TreeItemKind::Installation, profile.id.clone()));
        next.installations.push(profile);
    }

    check_tree(&next)?;

    let folders_by_id: HashMap<&str, &InstallationFolder> =
        next.folders.iter().map(|f| (f.id.as_str(), f)).collect();
    let item = |(kind, id): (TreeItemKind, String)| -> PreviewItem {
        let (name, location) = match kind {
            TreeItemKind::Folder => {
                let f = folders_by_id[id.as_str()];
                (f.name.clone(), f.parent_id.as_deref())
            }
            TreeItemKind::Installation => {
                let c = next
                    .installations
                    .iter()
                    .find(|c| c.id == id)
                    .expect("planned installation exists");
                (c.name.clone(), c.folder_id.as_deref())
            }
        };
        PreviewItem {
            kind,
            path: path_of(&next.folders, location),
            id,
            name,
        }
    };
    preview.added = added.into_iter().map(item).collect();
    preview.updated = updated.into_iter().map(item).collect();
    preview.unchanged = unchanged.into_iter().map(item).collect();
    Ok(ImportPlan {
        next,
        preview,
        removed_servers,
    })
}
````

<!-- block: file t04-src-tauri-src-storage-profile-rs-impl step=4.3 path=src-tauri/src/storage/profile.rs part=impl -->
````rust
//! Профиль инсталляции инференса (спец. §5.1, §5.2): окружение, серверы и их настройки. Ключи API
//! серверов в профиле не хранятся, они лежат в keychain (`secrets::API_KEY`). Проверки PEM, адресов
//! и прокси — те же, что у профиля подключения Glafira for Kubernetes.

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult, ErrorCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Local,
    Dev,
    Test,
    Stage,
    Prod,
}

/// Тип сервера (спец. §4.3): `auto` — определить по ответам сервера при подключении.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServerKind {
    #[default]
    Auto,
    Vllm,
    Sglang,
    Openai,
}

/// Чему доверять при проверке сертификата сервера (только для `https://`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum CaConfig {
    /// Системное хранилище сертификатов ОС.
    System,
    /// Только перечисленные сертификаты (один или несколько PEM): системные корни не используются.
    Pem { pem: String },
    /// Без проверки сертификата (`insecure-skip-tls-verify`); может быть запрещено политикой.
    Insecure,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum ProxyConfig {
    /// Переменные окружения `HTTPS_PROXY` / `NO_PROXY`.
    System,
    None,
    Url {
        url: String,
    },
}

fn default_ca() -> CaConfig {
    CaConfig::System
}

fn default_proxy() -> ProxyConfig {
    ProxyConfig::System
}

fn default_metrics_path() -> String {
    "/metrics".into()
}

fn default_request_timeout() -> u64 {
    30_000
}

fn default_connect_timeout() -> u64 {
    10_000
}

/// Сколько серверов может быть в инсталляции: столько охватывает проверка (спец. §7.1).
pub const MAX_SERVERS: usize = 32;

/// Сервер инференса — компонент инсталляции (спец. §5.2). Реплики одной модели за балансировщиком
/// добавляются по отдельности.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    /// Идентификатор компонента: часть ключа keychain (`<инсталляция>/<сервер>/apiKey`).
    pub id: String,
    pub name: String,
    /// Базовый URL: `http://` или `https://`, хост и порт; допускается путь (обратный прокси).
    pub base_url: String,
    #[serde(default)]
    pub kind: ServerKind,
    #[serde(default = "default_metrics_path")]
    pub metrics_path: String,
    #[serde(default = "default_ca")]
    pub ca: CaConfig,
    #[serde(default = "default_proxy")]
    pub proxy: ProxyConfig,
    /// Поля из более новой версии Glafira: сохраняются при записи как есть.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallationProfile {
    pub id: String,
    pub name: String,
    pub color: String,
    pub environment: Environment,
    pub read_only: bool,
    #[serde(default)]
    pub servers: Vec<ServerConfig>,
    #[serde(default = "default_request_timeout")]
    pub request_timeout_ms: u64,
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_ms: u64,
    #[serde(default)]
    pub folder_id: Option<String>,
    #[serde(default)]
    pub order: i64,
    /// Поля из более новой версии Glafira: сохраняются при записи как есть.
    /// `InstallationStore::upsert` входящий `extra` не принимает: у существующего профиля
    /// остаётся сохранённый, новый получает пустой.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl InstallationProfile {
    pub fn server(&self, id: &str) -> Option<&ServerConfig> {
        self.servers.iter().find(|s| s.id == id)
    }
}

/// Приводит поля к каноническому виду до проверки: имена и адреса без пробелов по краям, адрес
/// без `/` на конце, путь метрик — с `/` в начале.
pub fn normalize_profile(p: &mut InstallationProfile) {
    p.name = p.name.trim().to_string();
    for s in &mut p.servers {
        s.name = s.name.trim().to_string();
        s.base_url = s.base_url.trim().trim_end_matches('/').to_string();
        let path = s.metrics_path.trim();
        s.metrics_path = if path.is_empty() {
            default_metrics_path()
        } else if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{path}")
        };
    }
}

/// Сертификаты из PEM-текста (DER). Ошибка — если текст не PEM или в нём нет ни одного сертификата.
pub fn pem_certificates(text: &str) -> Result<Vec<Vec<u8>>, String> {
    let blocks = pem::parse_many(text.as_bytes()).map_err(|e| e.to_string())?;
    let certs: Vec<Vec<u8>> = blocks
        .into_iter()
        .filter(|b| b.tag() == "CERTIFICATE")
        .map(|b| b.into_contents())
        .collect();
    if certs.is_empty() {
        return Err("no CERTIFICATE blocks".into());
    }
    Ok(certs)
}

/// Диапазоны PEM-текста, занятые блоками с приватным ключом: тег содержит `PRIVATE KEY`
/// (`PRIVATE KEY`, `RSA PRIVATE KEY`, `EC PRIVATE KEY`, `ENCRYPTED PRIVATE KEY`). Блок идёт от
/// `-----BEGIN` до конца `-----END …-----`, незавершённый — до конца текста. Разбор не зависит
/// от того, корректно ли тело блока: ключ находится и в повреждённом PEM.
fn private_key_ranges(text: &str) -> Vec<std::ops::Range<usize>> {
    const BEGIN: &str = "-----BEGIN ";
    const END: &str = "-----END ";
    const DASHES: &str = "-----";
    let mut ranges = Vec::new();
    let mut pos = 0;
    while let Some(found) = text[pos..].find(BEGIN) {
        let start = pos + found;
        let label_start = start + BEGIN.len();
        let Some(label_len) = text[label_start..].find(DASHES) else {
            break;
        };
        let body = label_start + label_len + DASHES.len();
        if !text[label_start..label_start + label_len].contains("PRIVATE KEY") {
            // Следующий блок ищем сразу за заголовком этого: так не пропускается ключ,
            // стоящий внутри незавершённого блока другого вида.
            pos = body;
            continue;
        }
        let end = text[body..]
            .find(END)
            .and_then(|e| {
                let tail = body + e + END.len();
                text[tail..].find(DASHES).map(|d| tail + d + DASHES.len())
            })
            .unwrap_or(text.len());
        ranges.push(start..end);
        pos = end;
    }
    ranges
}

/// В PEM-тексте есть блок с приватным ключом (секреты хранятся только в keychain, спец. §5.1).
pub fn pem_has_private_key(text: &str) -> bool {
    !private_key_ranges(text).is_empty()
}

/// PEM-текст без блоков с приватным ключом; остальное (сертификаты) остаётся как есть.
pub fn strip_private_keys(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;
    for range in private_key_ranges(text) {
        out.push_str(&text[pos..range.start]);
        pos = range.end;
    }
    out.push_str(&text[pos..]);
    out
}

/// Идентификатор сервера: латинские буквы, цифры, `-` и `_`, от 1 до 64 символов — он входит
/// в ключ keychain и в имена файлов.
pub fn is_valid_server_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn invalid(key_and_text: crate::i18n::LocalizedText) -> AppError {
    AppError::localized(ErrorCode::InvalidInput, key_and_text)
}

/// В адресе есть имя пользователя или пароль (`https://user:pass@host`).
fn has_userinfo(u: &url::Url) -> bool {
    !u.username().is_empty() || u.password().is_some()
}

/// Адрес без имени и пароля и без завершающего «/» — для текста ошибки.
pub(crate) fn without_userinfo(u: &url::Url) -> String {
    let mut clean = u.clone();
    // Ошибка возможна только для адреса без хоста; вызывающий код такие адреса сюда не передаёт.
    let _ = clean.set_username("");
    let _ = clean.set_password(None);
    clean.as_str().trim_end_matches('/').to_string()
}

/// Адрес для текста ошибки без имени и пароля: ни один адрес, который вводил человек или который
/// читается из профиля и окружения, не показывается как есть. Если адрес не разбирается как URL
/// с хостом (нет схемы, пробел в хосте, пустой хост), отбрасывается всё до последней «@».
pub(crate) fn redact_url(raw: &str) -> String {
    match url::Url::parse(raw) {
        Ok(u) if u.host().is_some() => without_userinfo(&u),
        _ => match raw.rsplit_once('@') {
            Some((head, tail)) => match head.split_once("://") {
                Some((scheme, _)) => format!("{scheme}://{tail}"),
                None => tail.to_string(),
            },
            None => raw.to_string(),
        },
    }
}

/// Ошибка «учётные данные в адресе»; в тексте адрес показан без имени и пароля.
fn credentials_in_url(u: &url::Url) -> AppError {
    invalid(ltext!(
        "errors.installations.credentialsInUrl",
        "Remove the user name and password from “{{url}}”: credentials do not belong in the address (a proxy with a password is set via the system HTTPS_PROXY)",
        url = without_userinfo(u)
    ))
}

/// Разобранный базовый URL сервера; ошибка называет сервер.
pub fn parse_base_url(s: &ServerConfig) -> AppResult<url::Url> {
    let bad = || {
        invalid(ltext!(
            "errors.installations.badServerUrl",
            "Invalid address “{{url}}” of server “{{server}}”: use http://host:port or https://host:port",
            url = redact_url(&s.base_url),
            server = s.name
        ))
    };
    let url = url::Url::parse(s.base_url.trim()).map_err(|_| bad())?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none_or(str::is_empty)
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(bad());
    }
    if has_userinfo(&url) {
        return Err(credentials_in_url(&url));
    }
    Ok(url)
}

fn validate_server(s: &ServerConfig) -> AppResult<()> {
    if !is_valid_server_id(&s.id) {
        return Err(invalid(ltext!(
            "errors.installations.badServerId",
            "Server “{{server}}” has an invalid ID",
            server = s.name
        )));
    }
    if s.name.trim().is_empty() {
        return Err(invalid(ltext!(
            "errors.installations.emptyServerName",
            "Server name cannot be empty"
        )));
    }
    parse_base_url(s)?;
    let path = &s.metrics_path;
    if !path.starts_with('/') || path.contains(['?', '#']) || path.chars().any(char::is_whitespace)
    {
        return Err(invalid(ltext!(
            "errors.installations.badMetricsPath",
            "Invalid metrics path “{{path}}” of server “{{server}}”: use a path such as /metrics",
            path = path.escape_debug(),
            server = s.name
        )));
    }
    if let CaConfig::Pem { pem } = &s.ca {
        if pem_has_private_key(pem) {
            return Err(invalid(ltext!(
                "errors.installations.pemContainsKey",
                "A certificate field contains a private key — remove it: Glafira needs only the CA certificate"
            )));
        }
        if pem_certificates(pem).is_err() {
            return Err(invalid(ltext!(
                "errors.installations.caPemInvalid",
                "The CA certificate of server “{{server}}” must be in PEM format (-----BEGIN CERTIFICATE-----)",
                server = s.name
            )));
        }
    }
    if let ProxyConfig::Url { url } = &s.proxy {
        let proxy = url::Url::parse(url.trim())
            .ok()
            .filter(|u| matches!(u.scheme(), "http" | "https") && u.host_str().is_some());
        let Some(proxy) = proxy else {
            return Err(invalid(ltext!(
                "errors.installations.badProxyUrl",
                "Invalid proxy address “{{url}}”: use http://host:port",
                url = redact_url(url)
            )));
        };
        if has_userinfo(&proxy) {
            return Err(credentials_in_url(&proxy));
        }
    }
    Ok(())
}

pub fn validate_profile(p: &InstallationProfile) -> AppResult<()> {
    if p.id.trim().is_empty() {
        return Err(invalid(ltext!(
            "errors.installations.noId",
            "The installation has no ID"
        )));
    }
    if p.name.trim().is_empty() {
        return Err(invalid(ltext!(
            "errors.installations.emptyName",
            "Installation name cannot be empty"
        )));
    }
    if p.servers.is_empty() {
        return Err(invalid(ltext!(
            "errors.installations.noServers",
            "Add at least one inference server"
        )));
    }
    if p.servers.len() > MAX_SERVERS {
        return Err(invalid(ltext!(
            "errors.installations.tooManyServers",
            "An installation can have at most {{max}} servers",
            max = MAX_SERVERS
        )));
    }
    let mut ids = std::collections::HashSet::new();
    let mut names = std::collections::HashSet::new();
    for s in &p.servers {
        validate_server(s)?;
        if !ids.insert(s.id.as_str()) {
            return Err(invalid(ltext!(
                "errors.installations.duplicateServerId",
                "Two servers have the same ID “{{id}}”",
                id = s.id
            )));
        }
        if !names.insert(s.name.trim().to_lowercase()) {
            return Err(invalid(ltext!(
                "errors.installations.duplicateServerName",
                "Two servers are named “{{server}}”: give them different names",
                server = s.name.trim()
            )));
        }
    }
    if p.request_timeout_ms < 1_000 || p.connect_timeout_ms < 1_000 {
        return Err(invalid(ltext!(
            "errors.installations.timeoutTooSmall",
            "Timeouts must be at least 1000 ms"
        )));
    }
    Ok(())
}

#[cfg(test)]
pub fn sample_server(id: &str) -> ServerConfig {
    ServerConfig {
        id: id.to_string(),
        name: format!("vllm-{id}"),
        base_url: format!("https://{id}.dev.example.com:8000"),
        kind: ServerKind::Auto,
        metrics_path: "/metrics".into(),
        ca: CaConfig::System,
        proxy: ProxyConfig::System,
        extra: Default::default(),
    }
}

#[cfg(test)]
pub fn sample_profile(id: &str) -> InstallationProfile {
    InstallationProfile {
        id: id.to_string(),
        name: format!("inst-{id}"),
        color: "#4f8cff".into(),
        environment: Environment::Dev,
        read_only: false,
        servers: vec![sample_server("s1")],
        request_timeout_ms: 30_000,
        connect_timeout_ms: 10_000,
        folder_id: None,
        order: 0,
        extra: Default::default(),
    }
}

/// Самоподписанный сертификат для тестов (CN=glafira-test).
#[cfg(test)]
pub const TEST_CERT_PEM: &str = include_str!("testdata/test-cert.pem");
````

<!-- block: patch t04-src step=4.3 -->
````diff
diff --git a/PORTED.md b/PORTED.md
index 8e86819..3fc41e7 100644
--- a/PORTED.md
+++ b/PORTED.md
@@ -51,3 +51,10 @@
 | `docs/admin/policy.json` | `docs/admin/policy.json` | `468e08a` | адаптирован: идентификаторы, без `allowExecAuth` |
 | `docs/admin/policy.mobileconfig` | `docs/admin/policy.mobileconfig` | `468e08a` | адаптирован: идентификаторы, без `allowExecAuth` |
 | `docs/admin/policy.reg` | `docs/admin/policy.reg` | `468e08a` | адаптирован: ветка реестра, без `allowExecAuth` |
+| `src-tauri/src/storage/testdata/test-cert.pem` | `src-tauri/src/storage/testdata/test-cert.pem` | `468e08a` | как есть |
+| `src-tauri/src/storage/testdata/test-key.pem` | `src-tauri/src/storage/testdata/test-key.pem` | `468e08a` | как есть |
+| `src-tauri/src/storage/profile.rs` | `src-tauri/src/storage/profile.rs` | `468e08a` | адаптирован: профиль инсталляции с серверами вместо профиля подключения; PEM, адреса и прокси — как есть |
+| `src-tauri/src/storage/installations.rs` | `src-tauri/src/storage/connections.rs` | `468e08a` | адаптирован: инсталляции вместо подключений, неизвестные поля серверов |
+| `src-tauri/src/storage/tree.rs` | `src-tauri/src/storage/tree.rs` | `468e08a` | адаптирован: инсталляции вместо подключений |
+| `src-tauri/src/storage/exchange.rs` | `src-tauri/src/storage/exchange.rs` | `468e08a` | адаптирован: ключи API серверов вместо токена и ключа сертификата, без exec-плагинов и доступа LLM, предупреждения о смене адреса сервера |
+| `src-tauri/src/commands/folders.rs` | `src-tauri/src/commands/folders.rs` | `468e08a` | адаптирован: удаление ключей серверов, которых нет в файле; без exec-плагинов и сессий |
diff --git a/src-tauri/Cargo.toml b/src-tauri/Cargo.toml
index 7bede8e..dbf20c8 100644
--- a/src-tauri/Cargo.toml
+++ b/src-tauri/Cargo.toml
@@ -27,6 +27,10 @@ tracing-subscriber = { version = "0.3", features = ["env-filter", "registry", "f
 keyring = { version = "3", features = ["apple-native", "windows-native", "sync-secret-service"] }
 rustls = { version = "0.23", default-features = false, features = ["ring", "std", "tls12"] }
 plist = "1"
+pem = "3"
+url = "2"
+uuid = { version = "1", features = ["v4"] }
+time = { version = "0.3", features = ["formatting", "parsing"] }
 sys-locale = "0.3"
 
 [target.'cfg(windows)'.dependencies]
diff --git a/src-tauri/src/commands/mod.rs b/src-tauri/src/commands/mod.rs
index 4aebfc4..3bf6b40 100644
--- a/src-tauri/src/commands/mod.rs
+++ b/src-tauri/src/commands/mod.rs
@@ -1,3 +1,4 @@
+pub mod folders;
 pub mod settings;
 
 use crate::error::{AppResult, ErrorCode};
diff --git a/src-tauri/src/lib.rs b/src-tauri/src/lib.rs
index b4ef572..c512187 100644
--- a/src-tauri/src/lib.rs
+++ b/src-tauri/src/lib.rs
@@ -40,6 +40,14 @@ pub fn run() {
             Ok(())
         })
         .invoke_handler(tauri::generate_handler![
+            commands::folders::list_folders,
+            commands::folders::create_folder,
+            commands::folders::rename_folder,
+            commands::folders::delete_folder,
+            commands::folders::move_items,
+            commands::folders::export_installations,
+            commands::folders::preview_import,
+            commands::folders::apply_import,
             commands::settings::get_language_settings,
             commands::settings::set_language,
             commands::settings::get_policy_status,
diff --git a/src-tauri/src/state.rs b/src-tauri/src/state.rs
index 599b28b..6486eea 100644
--- a/src-tauri/src/state.rs
+++ b/src-tauri/src/state.rs
@@ -3,10 +3,12 @@ use std::sync::Arc;
 
 use crate::error::AppResult;
 use crate::secrets::SecretStore;
+use crate::storage::installations::InstallationStore;
 
 /// Общее состояние приложения: хранилища и политика. Клонируется дёшево (всё за `Arc`).
 #[derive(Clone)]
 pub struct AppState {
+    pub installations: Arc<InstallationStore>,
     pub secrets: Arc<dyn SecretStore>,
     pub policy: Arc<crate::policy::Policy>,
     pub settings: Arc<crate::settings::SettingsStore>,
@@ -15,6 +17,7 @@ pub struct AppState {
 impl AppState {
     pub fn new(data_dir: &Path, secrets: Arc<dyn SecretStore>) -> AppResult<Self> {
         Ok(Self {
+            installations: Arc::new(InstallationStore::open(data_dir)?),
             secrets,
             policy: Arc::new(crate::policy::Policy::default()),
             settings: Arc::new(crate::settings::SettingsStore::open(data_dir)),
diff --git a/src-tauri/src/storage/mod.rs b/src-tauri/src/storage/mod.rs
index d521fbd..99c5e3f 100644
--- a/src-tauri/src/storage/mod.rs
+++ b/src-tauri/src/storage/mod.rs
@@ -1 +1,5 @@
+pub mod exchange;
 pub mod fs;
+pub mod installations;
+pub mod profile;
+pub mod tree;
````

- [ ] **Step 4: Проверки.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib`  
Expected: PASS — `test result: ok. 114 passed; 0 failed`

Run: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`  
Expected: PASS — без предупреждений

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml --check`  
Expected: PASS — без изменений

Run: `cargo deny --manifest-path src-tauri/Cargo.toml --config deny.toml check licenses advisories`  
Expected: PASS — `advisories ok, licenses ok`

Run: `cargo tree --manifest-path src-tauri/Cargo.toml -i aws-lc-rs`  
Expected: ошибка `did not match any packages` — `aws-lc-rs` в дереве нет (то же для `openssl-sys`)

- [ ] **Step 5: Commit.**

```bash
git add \
  PORTED.md \
  src-tauri/Cargo.lock \
  src-tauri/Cargo.toml \
  src-tauri/src/commands/folders.rs \
  src-tauri/src/commands/mod.rs \
  src-tauri/src/lib.rs \
  src-tauri/src/state.rs \
  src-tauri/src/storage/exchange.rs \
  src-tauri/src/storage/installations.rs \
  src-tauri/src/storage/mod.rs \
  src-tauri/src/storage/profile.rs \
  src-tauri/src/storage/testdata/test-cert.pem \
  src-tauri/src/storage/testdata/test-key.pem \
  src-tauri/src/storage/tree.rs
git commit -s -m "feat(storage): installation profile with servers, store, folders, export and import" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 5: HTTP-клиент сервера инференса из профиля и классификация ошибок

**Files:**
- Create (по Kubernetes-версии): `src-tauri/src/servers/errors.rs` ← `src-tauri/src/k8s/errors.rs`
- Create: `src-tauri/src/servers/client.rs`, `src-tauri/src/servers/mod.rs`, `src-tauri/src/servers/testdata/ca.pem`, `src-tauri/src/servers/testdata/server-key.pem`, `src-tauri/src/servers/testdata/server.pem`, `src-tauri/src/servers/testing.rs`
- Modify: `PORTED.md`, `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`
- Generated: `src-tauri/Cargo.lock` (cargo, pnpm)

**Interfaces:**
- Consumes: задача 4: `ServerConfig`, `parse_base_url`, `pem_certificates`, `redact_url`; задача 3: `PolicyStatus`.
- Produces: `servers::client::{ServerClient::new(server, Timeouts, api_key, policy), url, get, get_ok, get_json, HttpReply, Timeouts, ensure_allowed, parse_json, JSON_LIMIT, METRICS_LIMIT}`, `servers::errors::{chain_text, from_transport_text, from_reqwest, from_status, timeout, breaks_client}`, `#[cfg(test)] servers::testing::{TestServer::start, start_tls, Reply, Seen, TEST_CA_PEM}`.

reqwest 0.13 на rustls с ring: системные корни, только перечисленные CA (`tls_certs_only`) или режим без проверки, который запрещает политика. Ключ API — `Authorization: Bearer` с пометкой «чувствительный»; перенаправления не выполняются, чтобы ключ не ушёл на другой адрес. Тело ответа ограничено. Классификация сетевых ошибок перенесена из `k8s/errors.rs`, коды HTTP — свои. Тесты идут против локального HTTP- и HTTPS-сервера (`tokio-rustls`, тестовый CA и сертификат на `localhost` и `127.0.0.1` в `servers/testdata`): ключ, лимиты, перенаправление, таймаут, закрытый порт, доверенный и чужой CA.

- [ ] **Step 1: Тесты.**

<!-- block: file t05-src-tauri-src-servers-client-rs-tests step=5.1 path=src-tauri/src/servers/client.rs part=tests -->
````rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::servers::testing::{Reply, TEST_CA_PEM, TestServer};
    use crate::storage::profile::sample_server;

    const TIMEOUTS: Timeouts = Timeouts {
        connect_ms: 2_000,
        request_ms: 2_000,
    };

    fn server(url: &str) -> ServerConfig {
        let mut s = sample_server("s1");
        s.base_url = url.into();
        s.proxy = ProxyConfig::None;
        s
    }

    fn client(s: &ServerConfig, key: Option<&str>) -> ServerClient {
        ServerClient::new(s, TIMEOUTS, key.map(String::from), &PolicyStatus::default()).unwrap()
    }

    #[test]
    fn url_keeps_the_base_path() {
        let c = client(&server("https://gw.corp/llm/qwen"), None);
        assert_eq!(
            c.url("/v1/models").as_str(),
            "https://gw.corp/llm/qwen/v1/models"
        );
        let c = client(&server("http://10.0.0.5:8000"), None);
        assert_eq!(c.url("/metrics").as_str(), "http://10.0.0.5:8000/metrics");
        assert_eq!(c.url("health").as_str(), "http://10.0.0.5:8000/health");
    }

    #[tokio::test]
    async fn bearer_is_sent_only_with_a_key() {
        let srv = TestServer::start(vec![("/v1/models", Reply::json(r#"{"data":[]}"#))]).await;
        let with = client(&server(&srv.url()), Some(" sk-test "));
        let v: serde_json::Value = with.get_json("/v1/models").await.unwrap();
        assert_eq!(v["data"], serde_json::json!([]));
        client(&server(&srv.url()), Some(""))
            .get("/v1/models", JSON_LIMIT)
            .await
            .unwrap();
        let seen = srv.seen.lock().unwrap().clone();
        assert_eq!(seen[0].authorization.as_deref(), Some("Bearer sk-test"));
        assert_eq!(seen[1].authorization, None, "пустой ключ не отправляется");
        assert!(
            seen[0]
                .user_agent
                .as_deref()
                .is_some_and(|ua| ua.starts_with("glafira-inference/"))
        );
        assert!(!format!("{with:?}").contains("sk-test"), "Debug без ключа");
    }

    #[tokio::test]
    async fn statuses_and_limits() {
        let srv = TestServer::start(vec![
            ("/v1/models", Reply::status(401)),
            ("/metrics", Reply::text(&"x".repeat(100))),
            ("/bad", Reply::json("<html>")),
        ])
        .await;
        let c = client(&server(&srv.url()), None);
        let e = c.get_ok("/v1/models", JSON_LIMIT).await.unwrap_err();
        assert_eq!(
            (e.code, e.key.as_str()),
            (ErrorCode::AuthFailed, "errors.server.unauthorized")
        );
        let r = c.get("/v1/models", JSON_LIMIT).await.unwrap();
        assert_eq!(r.status, 401, "get отдаёт любой код");
        let r = c.get_ok("/metrics", 10).await.unwrap();
        assert_eq!((r.body.len(), r.truncated), (10, true));
        assert!(r.content_type.unwrap().starts_with("text/plain"));
        let r = c.get_ok("/metrics", 1000).await.unwrap();
        assert_eq!((r.body.len(), r.truncated), (100, false));
        let e = c.get_json::<serde_json::Value>("/bad").await.unwrap_err();
        assert_eq!(e.key, "errors.server.badJson");
        assert!(!serde_json::to_string(&e).unwrap().contains("html"));
        let e = c.get_ok("/nope", JSON_LIMIT).await.unwrap_err();
        assert_eq!(e.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn redirects_are_not_followed() {
        let srv = TestServer::start(vec![(
            "/v1/models",
            Reply::status(302).header("Location", "http://127.0.0.1:1/steal"),
        )])
        .await;
        let c = client(&server(&srv.url()), Some("sk-test"));
        let e = c.get_ok("/v1/models", JSON_LIMIT).await.unwrap_err();
        assert_eq!(e.key, "errors.server.redirect");
        assert_eq!(srv.paths(), ["/v1/models"]);
    }

    #[tokio::test]
    async fn slow_answer_is_a_timeout_and_closed_port_a_connection_error() {
        let srv = TestServer::start(vec![(
            "/health",
            Reply::status(200).delayed(Duration::from_secs(5)),
        )])
        .await;
        let quick = Timeouts {
            connect_ms: 2_000,
            request_ms: 1_000,
        };
        let c =
            ServerClient::new(&server(&srv.url()), quick, None, &PolicyStatus::default()).unwrap();
        let e = c.get("/health", JSON_LIMIT).await.unwrap_err();
        assert_eq!(e.code, ErrorCode::Timeout, "{:?}", e.details);
        // Порт 1 на localhost закрыт. С запасом: на Windows отказ приходит через ~2 с.
        let closed = Timeouts {
            connect_ms: 10_000,
            request_ms: 10_000,
        };
        let c = ServerClient::new(
            &server("http://127.0.0.1:1"),
            closed,
            None,
            &PolicyStatus::default(),
        )
        .unwrap();
        let e = c.get("/health", JSON_LIMIT).await.unwrap_err();
        assert_eq!(e.code, ErrorCode::ConnectionFailed, "{:?}", e.details);
    }

    #[tokio::test]
    async fn tls_trusts_only_the_given_ca_or_nothing_in_insecure_mode() {
        let srv = TestServer::start_tls(vec![("/health", Reply::status(200))]).await;
        let mut s = server(&srv.url());
        // Системные корни тестовый CA не знают.
        let e = client(&s, None)
            .get("/health", JSON_LIMIT)
            .await
            .unwrap_err();
        assert_eq!(e.code, ErrorCode::TlsError, "{:?}", e.details);
        s.ca = CaConfig::Pem {
            pem: TEST_CA_PEM.into(),
        };
        let r = client(&s, None).get("/health", JSON_LIMIT).await.unwrap();
        assert_eq!(r.status, 200);
        // Чужой CA вместо нужного — снова ошибка TLS.
        s.ca = CaConfig::Pem {
            pem: crate::storage::profile::TEST_CERT_PEM.into(),
        };
        let e = client(&s, None)
            .get("/health", JSON_LIMIT)
            .await
            .unwrap_err();
        assert_eq!(e.code, ErrorCode::TlsError);
        s.ca = CaConfig::Insecure;
        let r = client(&s, None).get("/health", JSON_LIMIT).await.unwrap();
        assert_eq!(r.status, 200);
    }

    #[test]
    fn policy_forbids_insecure_servers() {
        let mut s = server("https://vllm:8000");
        s.ca = CaConfig::Insecure;
        let policy = PolicyStatus {
            allow_insecure_tls: false,
            ..PolicyStatus::default()
        };
        let e = ServerClient::new(&s, TIMEOUTS, None, &policy).unwrap_err();
        assert_eq!(
            (e.code, e.key.as_str()),
            (
                ErrorCode::InvalidInput,
                "errors.installations.insecureForbidden"
            )
        );
        s.ca = CaConfig::System;
        assert!(ServerClient::new(&s, TIMEOUTS, None, &policy).is_ok());
    }

    #[test]
    fn bad_api_key_characters_are_an_input_error() {
        let c = client(&server("http://h:8000"), Some("bad\nkey"));
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let e = rt.block_on(c.get("/v1/models", JSON_LIMIT)).unwrap_err();
        assert_eq!(e.key, "errors.installations.badApiKey");
    }
}
````

<!-- block: file t05-src-tauri-src-servers-errors-rs-tests step=5.1 path=src-tauri/src/servers/errors.rs part=tests -->
````rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_texts() {
        for (text, code, key) in [
            (
                "error sending request: client error (Connect): invalid peer certificate: NotValidForName",
                ErrorCode::TlsError,
                "errors.server.hostnameMismatch",
            ),
            (
                "client error (Connect): invalid peer certificate: UnknownIssuer",
                ErrorCode::TlsError,
                "errors.server.caUntrusted",
            ),
            (
                "invalid peer certificate: Other(OtherError(\"“*.badssl.com” certificate is not trusted: -67843\"))",
                ErrorCode::TlsError,
                "errors.server.caUntrusted",
            ),
            (
                "invalid peer certificate: Other(OtherError(\"“*.badssl.com” certificate is expired: -67818\"))",
                ErrorCode::TlsError,
                "errors.server.certExpired",
            ),
            (
                "invalid peer certificate: BadSignature",
                ErrorCode::TlsError,
                "errors.server.tlsError",
            ),
            (
                "client error (Connect): dns error: failed to lookup address information",
                ErrorCode::ConnectionFailed,
                "errors.server.dns",
            ),
            (
                "client error (Connect): tcp connect error: Connection refused (os error 61)",
                ErrorCode::ConnectionFailed,
                "errors.server.connect",
            ),
            (
                "deadline has elapsed",
                ErrorCode::Timeout,
                "errors.server.timeout",
            ),
            (
                "something else",
                ErrorCode::Internal,
                "errors.server.transport",
            ),
        ] {
            let e = from_transport_text(text);
            assert_eq!((e.code, e.key.as_str()), (code, key), "{text}");
            assert_eq!(e.details.as_deref(), Some(text));
        }
        assert!(from_transport_text("tcp connect error").retryable);
    }

    #[test]
    fn statuses() {
        for (status, code, key) in [
            (401, ErrorCode::AuthFailed, "errors.server.unauthorized"),
            (403, ErrorCode::AuthFailed, "errors.server.unauthorized"),
            (404, ErrorCode::NotFound, "errors.server.notFound"),
            (302, ErrorCode::ConnectionFailed, "errors.server.redirect"),
            (
                503,
                ErrorCode::ConnectionFailed,
                "errors.server.serverError",
            ),
            (418, ErrorCode::Internal, "errors.server.unexpectedStatus"),
        ] {
            let e = from_status(status, "/v1/models");
            assert_eq!((e.code, e.key.as_str()), (code, key), "{status}");
        }
        assert_eq!(from_status(404, "/version").params["path"], "/version");
        assert!(breaks_client(ErrorCode::AuthFailed) && breaks_client(ErrorCode::TlsError));
        assert!(!breaks_client(ErrorCode::Timeout));
    }
}
````

- [ ] **Step 2: Реализация.**

<!-- block: file t05-src-tauri-src-servers-client-rs-impl step=5.2 path=src-tauri/src/servers/client.rs part=impl -->
````rust
//! HTTP-клиент сервера инференса из профиля (спец. §5.2): CA, прокси, таймауты и ключ API. Только
//! GET; перенаправления не выполняются — ключ API не уходит на другой адрес.

use std::time::Duration;

use serde::de::DeserializeOwned;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::policy::PolicyStatus;
use crate::servers::errors::{chain_text, from_reqwest, from_status};
use crate::storage::profile::{
    CaConfig, InstallationProfile, ProxyConfig, ServerConfig, parse_base_url, pem_certificates,
    redact_url,
};

/// Сколько читается из ответа JSON: модели, версия, сведения о сервере.
pub const JSON_LIMIT: usize = 1024 * 1024;
/// Сколько читается из `/metrics`: у vLLM с десятком моделей LoRA это сотни килобайт.
pub const METRICS_LIMIT: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeouts {
    pub connect_ms: u64,
    pub request_ms: u64,
}

impl From<&InstallationProfile> for Timeouts {
    fn from(p: &InstallationProfile) -> Self {
        Self {
            connect_ms: p.connect_timeout_ms,
            request_ms: p.request_timeout_ms,
        }
    }
}

/// Ответ сервера: код, тело (не больше лимита запроса) и тип содержимого.
#[derive(Debug, Clone, PartialEq)]
pub struct HttpReply {
    pub status: u16,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
    /// Тело длиннее лимита: прочитано только начало.
    pub truncated: bool,
}

impl HttpReply {
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

/// Сервер без проверки сертификата нельзя открыть, если политика это запрещает (спец. §13).
pub fn ensure_allowed(s: &ServerConfig, policy: &PolicyStatus) -> AppResult<()> {
    if s.ca == CaConfig::Insecure && !policy.allow_insecure_tls {
        return Err(app_err!(
            ErrorCode::InvalidInput,
            "errors.installations.insecureForbidden",
            "Servers without TLS certificate verification are forbidden by your organization"
        ));
    }
    Ok(())
}

pub struct ServerClient {
    http: reqwest::Client,
    base: url::Url,
    api_key: Option<String>,
}

/// Ключ API не печатается: структура может попасть в лог или сообщение об ошибке.
impl std::fmt::Debug for ServerClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerClient")
            .field("base", &self.base.as_str())
            .field("api_key", &self.api_key.as_ref().map(|_| "***"))
            .finish()
    }
}

impl ServerClient {
    /// Клиент сервера `s`. Пустой ключ API — то же, что его отсутствие.
    pub fn new(
        s: &ServerConfig,
        timeouts: Timeouts,
        api_key: Option<String>,
        policy: &PolicyStatus,
    ) -> AppResult<Self> {
        ensure_allowed(s, policy)?;
        let base = parse_base_url(s)?;
        crate::tls::ensure_crypto_provider();
        let mut b = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(timeouts.connect_ms))
            .timeout(Duration::from_millis(timeouts.request_ms))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("glafira-inference/", env!("CARGO_PKG_VERSION")));
        match &s.ca {
            CaConfig::System => {}
            CaConfig::Pem { pem } => {
                let bad_pem = || {
                    app_err!(
                        ErrorCode::InvalidInput,
                        "errors.installations.caPemInvalid",
                        "The CA certificate of server “{{server}}” must be in PEM format (-----BEGIN CERTIFICATE-----)",
                        server = s.name
                    )
                };
                let certs = pem_certificates(pem)
                    .map_err(|_| bad_pem())?
                    .iter()
                    .map(|der| reqwest::Certificate::from_der(der))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| bad_pem())?;
                b = b.tls_certs_only(certs);
            }
            CaConfig::Insecure => b = b.danger_accept_invalid_certs(true),
        }
        b = match &s.proxy {
            ProxyConfig::System => b,
            ProxyConfig::None => b.no_proxy(),
            ProxyConfig::Url { url } => b.proxy(reqwest::Proxy::all(url.trim()).map_err(|_| {
                app_err!(
                    ErrorCode::InvalidInput,
                    "errors.installations.badProxyUrl",
                    "Invalid proxy address “{{url}}”: use http://host:port",
                    url = redact_url(url)
                )
            })?),
        };
        let http = b
            .build()
            .map_err(|e| AppError::internal_with(format!("http client: {}", chain_text(&e))))?;
        Ok(Self {
            http,
            base,
            api_key: api_key.filter(|k| !k.trim().is_empty()),
        })
    }

    pub fn has_api_key(&self) -> bool {
        self.api_key.is_some()
    }

    /// Адрес запроса: путь базового адреса сервера + `path` (`https://gw/llm` + `/v1/models`).
    pub fn url(&self, path: &str) -> url::Url {
        let mut u = self.base.clone();
        let joined = format!(
            "{}/{}",
            u.path().trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        u.set_path(&joined);
        u
    }

    /// GET `path`: ответ с любым кодом, тело — не больше `limit` байт.
    pub async fn get(&self, path: &str, limit: usize) -> AppResult<HttpReply> {
        let mut req = self.http.get(self.url(path));
        if let Some(key) = &self.api_key {
            let mut value =
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", key.trim())).map_err(
                    |_| {
                        app_err!(
                            ErrorCode::InvalidInput,
                            "errors.installations.badApiKey",
                            "The API key contains characters that cannot be sent in a header"
                        )
                    },
                )?;
            value.set_sensitive(true);
            req = req.header(reqwest::header::AUTHORIZATION, value);
        }
        let mut resp = req.send().await.map_err(|e| from_reqwest(&e))?;
        let status = resp.status().as_u16();
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(String::from);
        let mut body = Vec::new();
        let mut truncated = false;
        while let Some(chunk) = resp.chunk().await.map_err(|e| from_reqwest(&e))? {
            let room = limit.saturating_sub(body.len());
            if chunk.len() > room {
                body.extend_from_slice(&chunk[..room]);
                truncated = true;
                break;
            }
            body.extend_from_slice(&chunk);
        }
        Ok(HttpReply {
            status,
            content_type,
            body,
            truncated,
        })
    }

    /// GET `path`, ответ только 2xx: иначе ошибка по коду (`from_status`).
    pub async fn get_ok(&self, path: &str, limit: usize) -> AppResult<HttpReply> {
        let reply = self.get(path, limit).await?;
        if !reply.ok() {
            return Err(from_status(reply.status, path));
        }
        Ok(reply)
    }

    /// GET `path` и разбор JSON. Ошибка разбора не цитирует тело ответа.
    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> AppResult<T> {
        let reply = self.get_ok(path, JSON_LIMIT).await?;
        parse_json(&reply.body, path)
    }
}

/// Разбор JSON ответа `path`; в тексте ошибки — только путь.
pub fn parse_json<T: DeserializeOwned>(body: &[u8], path: &str) -> AppResult<T> {
    serde_json::from_slice(body).map_err(|_| {
        app_err!(
            ErrorCode::Internal,
            "errors.server.badJson",
            "Unexpected answer to {{path}}: not the JSON this server type returns",
            path = path
        )
    })
}
````

<!-- block: file t05-src-tauri-src-servers-errors-rs-impl step=5.2 path=src-tauri/src/servers/errors.rs part=impl -->
````rust
//! Ошибки сервера инференса и сети → `AppError` (спец. §16). Классификация сетевых ошибок — по
//! тексту цепочки `source()`, как в Glafira for Kubernetes: так её можно применить к ошибкам любого
//! HTTP-клиента.

use crate::error::{AppError, ErrorCode};

/// Текст ошибки вместе со всеми причинами: «a: b: c».
pub fn chain_text(e: &dyn std::error::Error) -> String {
    let mut out = e.to_string();
    let mut cur = e.source();
    while let Some(s) = cur {
        let t = s.to_string();
        if !out.contains(&t) {
            out.push_str(": ");
            out.push_str(&t);
        }
        cur = s.source();
    }
    out
}

/// Сетевая ошибка по тексту цепочки причин (hyper, rustls, DNS).
pub fn from_transport_text(text: &str) -> AppError {
    let t = text.to_lowercase();
    let e = if t.contains("invalid peer certificate") || t.contains("certificate") {
        if t.contains("notvalidforname") || t.contains("not valid for name") {
            app_err!(
                ErrorCode::TlsError,
                "errors.server.hostnameMismatch",
                "The server certificate does not match its address — check the address"
            )
        } else if t.contains("expired") || t.contains("notvalidyet") {
            app_err!(
                ErrorCode::TlsError,
                "errors.server.certExpired",
                "The server certificate has expired or is not valid yet"
            )
        } else if t.contains("unknownissuer")
            || t.contains("not trusted")
            || t.contains("untrusted")
        {
            app_err!(
                ErrorCode::TlsError,
                "errors.server.caUntrusted",
                "The server certificate is not signed by a trusted CA — specify the CA of the server"
            )
        } else {
            app_err!(
                ErrorCode::TlsError,
                "errors.server.tlsError",
                "TLS connection error"
            )
        }
    } else if t.contains("tls") || t.contains("handshake") {
        app_err!(
            ErrorCode::TlsError,
            "errors.server.tlsError",
            "TLS connection error"
        )
    } else if t.contains("dns error") || t.contains("failed to lookup address") {
        app_err!(
            ErrorCode::ConnectionFailed,
            "errors.server.dns",
            "Cannot resolve the server host name"
        )
    } else if t.contains("timed out") || t.contains("elapsed") || t.contains("timeout") {
        timeout()
    } else if t.contains("connect") || t.contains("refused") || t.contains("reset") {
        app_err!(
            ErrorCode::ConnectionFailed,
            "errors.server.connect",
            "Cannot connect to the server"
        )
    } else {
        app_err!(
            ErrorCode::Internal,
            "errors.server.transport",
            "Network error: {{message}}",
            message = text
        )
    };
    e.with_details(text)
}

/// Ошибка, если операция не уложилась в таймаут запроса.
pub fn timeout() -> AppError {
    app_err!(
        ErrorCode::Timeout,
        "errors.server.timeout",
        "The server did not respond in time"
    )
}

/// Ошибка HTTP-клиента: таймаут или сетевая ошибка по тексту цепочки.
pub fn from_reqwest(e: &reqwest::Error) -> AppError {
    if e.is_timeout() {
        timeout().with_details(chain_text(e))
    } else {
        from_transport_text(&chain_text(e))
    }
}

/// Ответ сервера с кодом не 2xx на запрос `path`. Тело ответа в текст не попадает: сервер мог
/// вернуть в нём что угодно, а ошибка уходит в интерфейс.
pub fn from_status(status: u16, path: &str) -> AppError {
    match status {
        401 | 403 => app_err!(
            ErrorCode::AuthFailed,
            "errors.server.unauthorized",
            "The server rejected the request ({{status}}) — check the API key",
            status = status
        ),
        404 => app_err!(
            ErrorCode::NotFound,
            "errors.server.notFound",
            "The server has no {{path}} (404)",
            path = path
        ),
        300..=399 => app_err!(
            ErrorCode::ConnectionFailed,
            "errors.server.redirect",
            "The server redirects {{path}} to another address ({{status}}) — use the final address of the server",
            path = path,
            status = status
        ),
        500..=599 => app_err!(
            ErrorCode::ConnectionFailed,
            "errors.server.serverError",
            "The server failed to answer {{path}} ({{status}})",
            path = path,
            status = status
        ),
        _ => app_err!(
            ErrorCode::Internal,
            "errors.server.unexpectedStatus",
            "Unexpected answer to {{path}}: {{status}}",
            path = path,
            status = status
        ),
    }
}

/// С такой ошибкой клиент сервера дальше не работает: ключ API или TLS. Следующее чтение должно
/// подключиться заново.
pub fn breaks_client(code: ErrorCode) -> bool {
    matches!(code, ErrorCode::AuthFailed | ErrorCode::TlsError)
}
````

<!-- block: file t05-src-tauri-src-servers-mod-rs step=5.2 path=src-tauri/src/servers/mod.rs -->
````rust
//! Серверы инференса (спец. §4.3): HTTP-клиент из профиля и ошибки сервера.

pub mod client;
pub mod errors;
#[cfg(test)]
pub mod testing;
````

<!-- block: file t05-src-tauri-src-servers-testdata-ca-pem step=5.2 path=src-tauri/src/servers/testdata/ca.pem -->
````text
-----BEGIN CERTIFICATE-----
MIIBajCCARCgAwIBAgIUXQWYgRuxzR5s84W9paK9DciIQoMwCgYIKoZIzj0EAwIw
GjEYMBYGA1UEAwwPZ2xhZmlyYS10ZXN0LWNhMCAXDTI2MTAyOTEwMzIxNFoYDzIx
MjYxMDA1MTAzMjE0WjAaMRgwFgYDVQQDDA9nbGFmaXJhLXRlc3QtY2EwWTATBgcq
hkjOPQIBBggqhkjOPQMBBwNCAAQ7UCoRG+6aoMwkHSHmmpwDvdEBHliw64jwWt6e
mwcJZ9XigJ+FWglmJXW2YIeBLK/tw38XOZwJ07N3s+w7XYjwozIwMDAdBgNVHQ4E
FgQU7ZDoraJoWodXlxtHTGM1nTKMGUswDwYDVR0TAQH/BAUwAwEB/zAKBggqhkjO
PQQDAgNIADBFAiEA1ZR1hJD8K3SHpeHCp8D+LpMiN6NT6Ub+dUk6g0u8CawCIAVl
eboHTKEUJrsWR/iOUrdf72aWyhM2/nPpliNYAlf8
-----END CERTIFICATE-----
````

<!-- block: file t05-src-tauri-src-servers-testdata-server-key-pem step=5.2 path=src-tauri/src/servers/testdata/server-key.pem -->
````text
-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgO/lG1YucddhD694i
h2fdkd1VazErwT4WRGjHiV/Hm4ChRANCAAQDRI1Qc5aD8iCdIEpEGIffvw0Pu7oh
EkpphBF69SB1qZS7oKRcg7vY/KIxUtiRQShtOlXE8CLeCndzeT7c91cM
-----END PRIVATE KEY-----
````

<!-- block: file t05-src-tauri-src-servers-testdata-server-pem step=5.2 path=src-tauri/src/servers/testdata/server.pem -->
````text
-----BEGIN CERTIFICATE-----
MIIBfTCCASOgAwIBAgIUAVEtpfGY0jk4ZvezsU0nXtOawhAwCgYIKoZIzj0EAwIw
GjEYMBYGA1UEAwwPZ2xhZmlyYS10ZXN0LWNhMCAXDTI2MTAyOTEwMzIxNFoYDzIx
MjYxMDA1MTAzMjE0WjAUMRIwEAYDVQQDDAlsb2NhbGhvc3QwWTATBgcqhkjOPQIB
BggqhkjOPQMBBwNCAAQDRI1Qc5aD8iCdIEpEGIffvw0Pu7ohEkpphBF69SB1qZS7
oKRcg7vY/KIxUtiRQShtOlXE8CLeCndzeT7c91cMo0swSTAaBgNVHREEEzARggls
b2NhbGhvc3SHBH8AAAEwCQYDVR0TBAIwADALBgNVHQ8EBAMCB4AwEwYDVR0lBAww
CgYIKwYBBQUHAwEwCgYIKoZIzj0EAwIDSAAwRQIgY2gW852IYOE5XLh9P+ptFEVk
hS3+CNcU0hdfnievVRACIQClDe+6ioz6INdKKI0VZzT2CxZFjtRN2Jse2qAmDfty
uw==
-----END CERTIFICATE-----
````

<!-- block: file t05-src-tauri-src-servers-testing-rs step=5.2 path=src-tauri/src/servers/testing.rs -->
````rust
//! Сервер для тестов: HTTP/1.1 или HTTPS (тестовый CA, сертификат на `localhost` и `127.0.0.1`)
//! на локальном порту, ответы по пути запроса. Сеть за пределами машины тесты не трогают.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;

/// CA, которым подписан сертификат тестового HTTPS-сервера.
pub const TEST_CA_PEM: &str = include_str!("testdata/ca.pem");
const SERVER_CERT_PEM: &str = include_str!("testdata/server.pem");
const SERVER_KEY_PEM: &str = include_str!("testdata/server-key.pem");

#[derive(Clone, Debug)]
pub struct Reply {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
    pub delay: Option<Duration>,
    pub headers: Vec<(&'static str, String)>,
}

impl Reply {
    pub fn json(body: &str) -> Self {
        Self::with("application/json", 200, body)
    }

    pub fn text(body: &str) -> Self {
        Self::with("text/plain; version=0.0.4", 200, body)
    }

    pub fn status(status: u16) -> Self {
        Self::with("text/plain", status, "")
    }

    fn with(content_type: &'static str, status: u16, body: &str) -> Self {
        Self {
            status,
            content_type,
            body: body.as_bytes().to_vec(),
            delay: None,
            headers: Vec::new(),
        }
    }

    pub fn delayed(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }

    pub fn header(mut self, name: &'static str, value: &str) -> Self {
        self.headers.push((name, value.to_string()));
        self
    }
}

/// Запрос, который получил тестовый сервер.
#[derive(Clone, Debug, PartialEq)]
pub struct Seen {
    pub path: String,
    pub authorization: Option<String>,
    pub user_agent: Option<String>,
}

pub struct TestServer {
    pub addr: SocketAddr,
    pub tls: bool,
    pub seen: Arc<Mutex<Vec<Seen>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl TestServer {
    pub async fn start(routes: Vec<(&str, Reply)>) -> Self {
        Self::spawn(routes, false).await
    }

    pub async fn start_tls(routes: Vec<(&str, Reply)>) -> Self {
        Self::spawn(routes, true).await
    }

    /// Адрес сервера: `http://127.0.0.1:<порт>` или `https://127.0.0.1:<порт>`.
    pub fn url(&self) -> String {
        let scheme = if self.tls { "https" } else { "http" };
        format!("{scheme}://{}", self.addr)
    }

    pub fn paths(&self) -> Vec<String> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .map(|s| s.path.clone())
            .collect()
    }

    async fn spawn(routes: Vec<(&str, Reply)>, tls: bool) -> Self {
        crate::tls::ensure_crypto_provider();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let routes: Arc<HashMap<String, Reply>> = Arc::new(
            routes
                .into_iter()
                .map(|(p, r)| (p.to_string(), r))
                .collect(),
        );
        let seen = Arc::new(Mutex::new(Vec::new()));
        let acceptor = tls.then(acceptor);
        let task = {
            let seen = seen.clone();
            tokio::spawn(async move {
                loop {
                    let Ok((stream, _)) = listener.accept().await else {
                        return;
                    };
                    let (routes, seen, acceptor) = (routes.clone(), seen.clone(), acceptor.clone());
                    tokio::spawn(async move {
                        match acceptor {
                            Some(a) => {
                                if let Ok(s) = a.accept(stream).await {
                                    serve(s, &routes, &seen).await;
                                }
                            }
                            None => serve(stream, &routes, &seen).await,
                        }
                    });
                }
            })
        };
        Self {
            addr,
            tls,
            seen,
            task,
        }
    }
}

fn acceptor() -> tokio_rustls::TlsAcceptor {
    use rustls::pki_types::pem::PemObject;
    use rustls::pki_types::{CertificateDer, PrivateKeyDer};
    let certs = CertificateDer::pem_slice_iter(SERVER_CERT_PEM.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let key = PrivateKeyDer::from_pem_slice(SERVER_KEY_PEM.as_bytes()).unwrap();
    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .unwrap();
    tokio_rustls::TlsAcceptor::from(Arc::new(config))
}

async fn serve<S: AsyncRead + AsyncWrite + Unpin>(
    mut stream: S,
    routes: &HashMap<String, Reply>,
    seen: &Mutex<Vec<Seen>>,
) {
    let mut head = Vec::new();
    let mut buf = [0u8; 4096];
    while !head.windows(4).any(|w| w == b"\r\n\r\n") {
        match stream.read(&mut buf).await {
            Ok(0) | Err(_) => return,
            Ok(n) => head.extend_from_slice(&buf[..n]),
        }
        if head.len() > 64 * 1024 {
            return;
        }
    }
    let text = String::from_utf8_lossy(&head);
    let mut lines = text.lines();
    let path = lines
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("")
        .to_string();
    let header = |name: &str| {
        text.lines().skip(1).find_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.trim()
                .eq_ignore_ascii_case(name)
                .then(|| v.trim().to_string())
        })
    };
    seen.lock().unwrap().push(Seen {
        path: path.clone(),
        authorization: header("authorization"),
        user_agent: header("user-agent"),
    });
    let reply = routes
        .get(&path)
        .cloned()
        .unwrap_or_else(|| Reply::status(404));
    if let Some(d) = reply.delay {
        tokio::time::sleep(d).await;
    }
    let mut out = format!(
        "HTTP/1.1 {} X\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        reply.status,
        reply.content_type,
        reply.body.len()
    );
    for (k, v) in &reply.headers {
        out.push_str(&format!("{k}: {v}\r\n"));
    }
    out.push_str("\r\n");
    let _ = stream.write_all(out.as_bytes()).await;
    let _ = stream.write_all(&reply.body).await;
    let _ = stream.shutdown().await;
}
````

<!-- block: patch t05-src step=5.2 -->
````diff
diff --git a/PORTED.md b/PORTED.md
index 3fc41e7..bcdb543 100644
--- a/PORTED.md
+++ b/PORTED.md
@@ -58,3 +58,4 @@
 | `src-tauri/src/storage/tree.rs` | `src-tauri/src/storage/tree.rs` | `468e08a` | адаптирован: инсталляции вместо подключений |
 | `src-tauri/src/storage/exchange.rs` | `src-tauri/src/storage/exchange.rs` | `468e08a` | адаптирован: ключи API серверов вместо токена и ключа сертификата, без exec-плагинов и доступа LLM, предупреждения о смене адреса сервера |
 | `src-tauri/src/commands/folders.rs` | `src-tauri/src/commands/folders.rs` | `468e08a` | адаптирован: удаление ключей серверов, которых нет в файле; без exec-плагинов и сессий |
+| `src-tauri/src/servers/errors.rs` | `src-tauri/src/k8s/errors.rs` | `468e08a` | адаптирован: только сетевые ошибки и коды HTTP сервера инференса, ключи `errors.server.*` |
diff --git a/src-tauri/Cargo.toml b/src-tauri/Cargo.toml
index dbf20c8..741ee80 100644
--- a/src-tauri/Cargo.toml
+++ b/src-tauri/Cargo.toml
@@ -31,6 +31,8 @@ pem = "3"
 url = "2"
 uuid = { version = "1", features = ["v4"] }
 time = { version = "0.3", features = ["formatting", "parsing"] }
+tokio = { version = "1", features = ["rt", "macros", "time", "net", "io-util", "sync"] }
+reqwest = { version = "0.13", default-features = false, features = ["rustls-no-provider", "http2", "system-proxy"] }
 sys-locale = "0.3"
 
 [target.'cfg(windows)'.dependencies]
@@ -38,6 +40,8 @@ winreg = "0.55"
 
 [dev-dependencies]
 tempfile = "3"
+tokio = { version = "1", features = ["rt-multi-thread", "macros", "time", "net", "io-util", "test-util"] }
+tokio-rustls = { version = "0.26", default-features = false, features = ["ring", "tls12"] }
 
 [profile.release]
 codegen-units = 1
diff --git a/src-tauri/src/lib.rs b/src-tauri/src/lib.rs
index c512187..552b391 100644
--- a/src-tauri/src/lib.rs
+++ b/src-tauri/src/lib.rs
@@ -7,6 +7,7 @@ pub mod logging;
 pub mod policy;
 pub mod product;
 pub mod secrets;
+pub mod servers;
 pub mod settings;
 pub mod state;
 pub mod storage;
````

- [ ] **Step 3: Проверки.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib`  
Expected: PASS — `test result: ok. 124 passed; 0 failed`

Run: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`  
Expected: PASS — без предупреждений

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml --check`  
Expected: PASS — без изменений

Run: `cargo deny --manifest-path src-tauri/Cargo.toml --config deny.toml check licenses advisories`  
Expected: PASS — `advisories ok, licenses ok`

Run: `cargo tree --manifest-path src-tauri/Cargo.toml -i aws-lc-rs`  
Expected: ошибка `did not match any packages` — `aws-lc-rs` в дереве нет (то же для `openssl-sys`)

- [ ] **Step 4: Commit.**

```bash
git add \
  PORTED.md \
  src-tauri/Cargo.lock \
  src-tauri/Cargo.toml \
  src-tauri/src/lib.rs \
  src-tauri/src/servers/client.rs \
  src-tauri/src/servers/errors.rs \
  src-tauri/src/servers/mod.rs \
  src-tauri/src/servers/testdata/ca.pem \
  src-tauri/src/servers/testdata/server-key.pem \
  src-tauri/src/servers/testdata/server.pem \
  src-tauri/src/servers/testing.rs
git commit -s -m "feat(servers): HTTP client of an inference server from the profile, error classification, local test server" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 6: Определение vLLM, SGLang или OpenAI-совместимого сервера, версии и моделей по снимкам ответов

**Files:**
- Create: `src-tauri/src/servers/detect.rs`, `src-tauri/src/servers/probe.rs`, `src-tauri/src/servers/testdata/llamacpp/health.json`, `src-tauri/src/servers/testdata/llamacpp/models.json`, `src-tauri/src/servers/testdata/ollama/models.json`, `src-tauri/src/servers/testdata/sglang-0.4.6/model_info.json`, `src-tauri/src/servers/testdata/sglang-0.4.6/models.json`, `src-tauri/src/servers/testdata/sglang-0.4.6/server_info.json`, `src-tauri/src/servers/testdata/sglang-0.5.4/metrics.txt`, `src-tauri/src/servers/testdata/sglang-0.5.4/model_info.json`, `src-tauri/src/servers/testdata/sglang-0.5.4/models.json`, `src-tauri/src/servers/testdata/sglang-0.5.4/server_info.json`, `src-tauri/src/servers/testdata/vllm-0.11.0/metrics.txt`, `src-tauri/src/servers/testdata/vllm-0.11.0/models.json`, `src-tauri/src/servers/testdata/vllm-0.11.0/version.json`, `src-tauri/src/servers/testdata/vllm-0.7.3/metrics.txt`, `src-tauri/src/servers/testdata/vllm-0.7.3/models.json`, `src-tauri/src/servers/testdata/vllm-0.7.3/version.json`, `src-tauri/src/servers/testdata/vllm-0.8.5/metrics.txt`, `src-tauri/src/servers/testdata/vllm-0.8.5/models.json`, `src-tauri/src/servers/testdata/vllm-0.8.5/version.json`
- Modify: `src-tauri/src/servers/mod.rs`

**Interfaces:**
- Consumes: задача 5: `ServerClient`, `parse_json`; задача 4: `ServerKind`.
- Produces: `servers::detect::{Probe, Answer, DetectedKind (vllm | sglang | openai; label, min_version), Model { id, root, parent, max_model_len, owned_by }, ServerFacts { kind, detected, version, models, metrics, warnings }, detect, analyze, parse_models, version_numbers, version_at_least}`, `#[cfg(test)] detect::fixtures`, `servers::probe::{probe, facts, METRICS_PROBE_LIMIT}`; снимки ответов `servers/testdata/<сервер>-<версия>/`.

Чистые функции над снимком ответов (спец. §4.3). SGLang — по `/model_info` и `/server_info` (у старых версий — `/get_*`), префиксу `sglang:` в метриках или `owned_by: sglang`; vLLM — по префиксу `vllm:`, `owned_by: vllm` или JSON `/version`; иначе `openai`. Тип из профиля побеждает, но расхождение — предупреждение; версии ниже 0.8 (vLLM) и 0.4 (SGLang) работают как `openai` с предупреждением; без метрик — предупреждение, у SGLang — с подсказкой `--enable-metrics`. Из `/server_info` берётся только версия: в ответе SGLang бывает ключ API, и тест проверяет, что он не попадает в сведения. Снимки собраны по исходникам и документации серверов; сверка с настоящими серверами — пункты 11–12 смоука.

- [ ] **Step 1: Тесты.**

<!-- block: file t06-src-tauri-src-servers-detect-rs-tests step=6.1 path=src-tauri/src/servers/detect.rs part=tests -->
````rust
#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    fn keys(f: &ServerFacts) -> Vec<&str> {
        f.warnings.iter().map(|w| w.key.as_str()).collect()
    }

    #[test]
    fn snapshots_of_real_servers() {
        for (name, probe, kind, version, metrics) in [
            (
                "vllm 0.11",
                vllm_0_11(),
                DetectedKind::Vllm,
                Some("0.11.0"),
                true,
            ),
            (
                "vllm 0.8",
                vllm_0_8(),
                DetectedKind::Vllm,
                Some("0.8.5.post1"),
                true,
            ),
            (
                "sglang 0.5",
                sglang_0_5(),
                DetectedKind::Sglang,
                Some("0.5.4"),
                true,
            ),
            (
                "sglang 0.4",
                sglang_0_4(),
                DetectedKind::Sglang,
                Some("0.4.6.post1"),
                false,
            ),
            ("llama.cpp", llamacpp(), DetectedKind::Openai, None, false),
            ("ollama", ollama(), DetectedKind::Openai, None, false),
        ] {
            let f = analyze(ServerKind::Auto, &probe).unwrap();
            assert_eq!(
                (f.kind, f.detected, f.version.as_deref(), f.metrics),
                (kind, kind, version, metrics),
                "{name}"
            );
        }
    }

    #[test]
    fn models_with_lora_adapters_and_context_length() {
        let f = analyze(ServerKind::Auto, &vllm_0_8()).unwrap();
        assert_eq!(f.models.len(), 2);
        assert_eq!(f.models[0].max_model_len, Some(131072));
        assert_eq!(
            f.models[0].root.as_deref(),
            Some("/models/Meta-Llama-3.1-8B-Instruct")
        );
        assert_eq!(f.models[1].parent.as_deref(), Some("llama-3.1-8b"));
        assert_eq!(f.models[1].max_model_len, None);
        let f = analyze(ServerKind::Auto, &ollama()).unwrap();
        assert_eq!(
            f.models.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            ["llama3.1:8b", "qwen3:32b"]
        );
        let json = serde_json::to_value(analyze(ServerKind::Auto, &sglang_0_5()).unwrap()).unwrap();
        assert_eq!(json["kind"], "sglang");
        assert_eq!(json["models"][0]["maxModelLen"], 40960);
        assert!(json["models"][0].get("parent").is_none());
    }

    #[test]
    fn server_info_secrets_never_reach_the_facts() {
        let f = analyze(ServerKind::Auto, &sglang_0_5()).unwrap();
        let json = serde_json::to_string(&f).unwrap();
        assert!(!json.contains("sk-sglang-s3cr3t"), "{json}");
    }

    #[test]
    fn old_version_is_worked_with_as_openai_compatible() {
        let f = analyze(ServerKind::Auto, &vllm_0_7()).unwrap();
        assert_eq!(
            (f.detected, f.kind),
            (DetectedKind::Vllm, DetectedKind::Openai)
        );
        assert_eq!(keys(&f), ["messages.servers.unsupportedVersion"]);
        assert_eq!(f.warnings[0].params["min"], "0.8");
        assert!(!f.metrics, "метрики openai не читаются");
    }

    #[test]
    fn sglang_without_metrics_is_told_how_to_enable_them() {
        let f = analyze(ServerKind::Auto, &sglang_0_4()).unwrap();
        assert_eq!(keys(&f), ["messages.servers.noMetricsSglang"]);
        let mut p = vllm_0_11();
        p.metrics = None;
        let f = analyze(ServerKind::Auto, &p).unwrap();
        assert_eq!(keys(&f), ["messages.servers.noMetrics"]);
    }

    #[test]
    fn configured_kind_wins_with_a_warning_on_mismatch() {
        let f = analyze(ServerKind::Sglang, &vllm_0_11()).unwrap();
        assert_eq!(
            (f.kind, f.detected),
            (DetectedKind::Sglang, DetectedKind::Vllm)
        );
        assert_eq!(f.warnings[0].key, "messages.servers.kindMismatch");
        assert_eq!(f.warnings[0].params["detected"], "vLLM");
        let f = analyze(ServerKind::Vllm, &vllm_0_11()).unwrap();
        assert!(f.warnings.is_empty());
        let f = analyze(ServerKind::Openai, &vllm_0_11()).unwrap();
        assert_eq!((f.kind, f.version.as_deref()), (DetectedKind::Openai, None));
    }

    #[test]
    fn signals_each_point_to_a_kind() {
        // Обратный прокси вырезал `owned_by`, метрик нет: решает `/version`.
        let mut p = vllm_0_11();
        p.metrics = None;
        p.models = br#"{"data":[{"id":"m"}]}"#.to_vec();
        assert_eq!(
            detect(&p, &parse_models(&p.models).unwrap()),
            DetectedKind::Vllm
        );
        // HTML-заглушка прокси на любом пути — не признак SGLang.
        let mut p = llamacpp();
        p.model_info = Some(Answer {
            status: 200,
            body: b"<html>ok</html>".to_vec(),
        });
        p.version = p.model_info.clone();
        assert_eq!(
            detect(&p, &parse_models(&p.models).unwrap()),
            DetectedKind::Openai
        );
        // `/version` с JSON, но модели принадлежат llama.cpp — не vLLM.
        let mut p = llamacpp();
        p.version = Some(Answer {
            status: 200,
            body: br#"{"version":"b4500"}"#.to_vec(),
        });
        assert_eq!(
            detect(&p, &parse_models(&p.models).unwrap()),
            DetectedKind::Openai
        );
    }

    #[test]
    fn models_must_be_a_list() {
        let mut p = llamacpp();
        p.models = b"<html>".to_vec();
        let e = analyze(ServerKind::Auto, &p).unwrap_err();
        assert_eq!(e.key, "errors.server.badJson");
        p.models = br#"{"object":"list","data":[]}"#.to_vec();
        assert!(analyze(ServerKind::Auto, &p).unwrap().models.is_empty());
    }

    #[test]
    fn versions() {
        assert_eq!(version_numbers("0.10.1.1"), [0, 10, 1]);
        assert_eq!(version_numbers("0.6.3.post1"), [0, 6, 3]);
        assert_eq!(version_numbers("0.5.0rc1"), [0, 5, 0]);
        assert_eq!(version_numbers("v0.11.0+cu128"), [0, 11, 0]);
        assert!(version_numbers("main").is_empty());
        for (v, min, ok) in [
            ("0.11.0", "0.8", true),
            ("0.8.0", "0.8", true),
            ("0.7.3", "0.8", false),
            ("0.4.6.post1", "0.4", true),
            ("0.3.6", "0.4", false),
            ("1.0", "0.8", true),
            ("dev", "0.8", true),
        ] {
            assert_eq!(version_at_least(v, min), ok, "{v} >= {min}");
        }
    }
}
````

<!-- block: file t06-src-tauri-src-servers-probe-rs-tests step=6.1 path=src-tauri/src/servers/probe.rs part=tests -->
````rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use crate::policy::PolicyStatus;
    use crate::servers::client::Timeouts;
    use crate::servers::detect::DetectedKind;
    use crate::servers::testing::{Reply, TestServer};
    use crate::storage::profile::{ProxyConfig, ServerKind, sample_server};

    fn config(url: &str) -> ServerConfig {
        let mut s = sample_server("s1");
        s.base_url = url.into();
        s.proxy = ProxyConfig::None;
        s
    }

    fn client(s: &ServerConfig) -> ServerClient {
        let t = Timeouts {
            connect_ms: 2_000,
            request_ms: 2_000,
        };
        ServerClient::new(s, t, None, &PolicyStatus::default()).unwrap()
    }

    fn sglang_0_4_routes() -> Vec<(&'static str, Reply)> {
        vec![
            (
                "/v1/models",
                Reply::json(include_str!("testdata/sglang-0.4.6/models.json")),
            ),
            (
                "/get_model_info",
                Reply::json(include_str!("testdata/sglang-0.4.6/model_info.json")),
            ),
            (
                "/get_server_info",
                Reply::json(include_str!("testdata/sglang-0.4.6/server_info.json")),
            ),
        ]
    }

    #[tokio::test]
    async fn vllm_server_is_recognized_over_http() {
        let srv = TestServer::start(vec![
            (
                "/v1/models",
                Reply::json(include_str!("testdata/vllm-0.11.0/models.json")),
            ),
            (
                "/version",
                Reply::json(include_str!("testdata/vllm-0.11.0/version.json")),
            ),
            (
                "/metrics",
                Reply::text(include_str!("testdata/vllm-0.11.0/metrics.txt")),
            ),
        ])
        .await;
        let s = config(&srv.url());
        let f = facts(&client(&s), &s).await.unwrap();
        assert_eq!(
            (f.kind, f.version.as_deref(), f.metrics),
            (DetectedKind::Vllm, Some("0.11.0"), true)
        );
        assert_eq!(f.models[0].id, "qwen3-32b");
        let mut paths = srv.paths();
        paths.sort();
        assert_eq!(
            paths,
            [
                "/get_model_info",
                "/get_server_info",
                "/metrics",
                "/model_info",
                "/server_info",
                "/v1/models",
                "/version"
            ]
        );
    }

    #[tokio::test]
    async fn old_sglang_paths_are_used_when_new_ones_are_missing() {
        let srv = TestServer::start(sglang_0_4_routes()).await;
        let s = config(&srv.url());
        let f = facts(&client(&s), &s).await.unwrap();
        assert_eq!(
            (f.kind, f.version.as_deref()),
            (DetectedKind::Sglang, Some("0.4.6.post1"))
        );
        assert!(!f.metrics);
    }

    #[tokio::test]
    async fn metrics_path_comes_from_the_profile() {
        let mut routes = sglang_0_4_routes();
        routes.push((
            "/stats",
            Reply::text(include_str!("testdata/sglang-0.5.4/metrics.txt")),
        ));
        let srv = TestServer::start(routes).await;
        let mut s = config(&srv.url());
        s.metrics_path = "/stats".into();
        let f = facts(&client(&s), &s).await.unwrap();
        assert!(f.metrics);
        assert!(!srv.paths().contains(&"/metrics".to_string()));
    }

    #[tokio::test]
    async fn models_are_required() {
        let srv = TestServer::start(vec![("/v1/models", Reply::status(401))]).await;
        let s = config(&srv.url());
        let e = facts(&client(&s), &s).await.unwrap_err();
        assert_eq!(e.code, ErrorCode::AuthFailed);
        let srv = TestServer::start(vec![]).await;
        let s = config(&srv.url());
        let e = facts(&client(&s), &s).await.unwrap_err();
        assert_eq!(e.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn configured_kind_is_respected() {
        let srv = TestServer::start(sglang_0_4_routes()).await;
        let mut s = config(&srv.url());
        s.kind = ServerKind::Openai;
        let f = facts(&client(&s), &s).await.unwrap();
        assert_eq!(
            (f.kind, f.detected),
            (DetectedKind::Openai, DetectedKind::Sglang)
        );
        assert_eq!(f.warnings[0].key, "messages.servers.kindMismatch");
    }
}
````

- [ ] **Step 2: Реализация.**

<!-- block: file t06-src-tauri-src-servers-detect-rs-impl step=6.2 path=src-tauri/src/servers/detect.rs part=impl -->
````rust
//! Определение типа сервера по ответам (спец. §4.3) — чистые функции над снимком `Probe`.
//! Признаки: `/model_info` и `/server_info` SGLang (у старых версий — `/get_model_info` и
//! `/get_server_info`), `/version` vLLM, префиксы серий в `/metrics` (`vllm:`, `sglang:`) и поле
//! `owned_by` в `/v1/models`. Тип из профиля побеждает признаки, но расхождение — предупреждение.

use serde::{Deserialize, Serialize};

use crate::error::AppResult;
use crate::i18n::LocalizedText;
use crate::servers::client::parse_json;
use crate::storage::profile::ServerKind;

/// Тип сервера, с которым работает Glafira.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DetectedKind {
    Vllm,
    Sglang,
    Openai,
}

impl DetectedKind {
    pub fn label(self) -> &'static str {
        match self {
            DetectedKind::Vllm => "vLLM",
            DetectedKind::Sglang => "SGLang",
            DetectedKind::Openai => "OpenAI-compatible",
        }
    }

    /// Нижняя граница версии, с которой работают адаптеры (спец. §1); ниже — `openai`.
    pub fn min_version(self) -> Option<&'static str> {
        match self {
            DetectedKind::Vllm => Some("0.8"),
            DetectedKind::Sglang => Some("0.4"),
            DetectedKind::Openai => None,
        }
    }

    fn from_configured(k: ServerKind) -> Option<Self> {
        match k {
            ServerKind::Auto => None,
            ServerKind::Vllm => Some(DetectedKind::Vllm),
            ServerKind::Sglang => Some(DetectedKind::Sglang),
            ServerKind::Openai => Some(DetectedKind::Openai),
        }
    }
}

/// Ответ на один запрос снимка: код и тело (не больше лимита клиента).
#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    pub status: u16,
    pub body: Vec<u8>,
}

impl Answer {
    /// JSON-объект ответа 2xx; иначе `None` (404, HTML обратного прокси, обрыв).
    fn object(&self) -> Option<serde_json::Map<String, serde_json::Value>> {
        if !(200..300).contains(&self.status) {
            return None;
        }
        match serde_json::from_slice(&self.body).ok()? {
            serde_json::Value::Object(m) => Some(m),
            _ => None,
        }
    }
}

/// Снимок ответов сервера, по которому определяется тип. `None` — запрос не удался.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Probe {
    pub version: Option<Answer>,
    pub model_info: Option<Answer>,
    pub server_info: Option<Answer>,
    /// Начало `/metrics`, если он ответил 2xx.
    pub metrics: Option<String>,
    /// `/v1/models` — обязательный ответ: без него сервер не OpenAI-совместимый.
    pub models: Vec<u8>,
}

/// Модель из `/v1/models`. У адаптера LoRA `parent` — базовая модель.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_model_len: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owned_by: Option<String>,
}

#[derive(Deserialize)]
struct ModelList {
    data: Vec<RawModel>,
}

/// Модель как её отдают серверы: `max_model_len` у vLLM бывает `null`, у прочих его нет.
#[derive(Deserialize)]
struct RawModel {
    id: String,
    #[serde(default)]
    root: Option<String>,
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    max_model_len: Option<u64>,
    #[serde(default)]
    owned_by: Option<String>,
}

/// Модели из ответа `/v1/models`.
pub fn parse_models(body: &[u8]) -> AppResult<Vec<Model>> {
    let list: ModelList = parse_json(body, "/v1/models")?;
    Ok(list
        .data
        .into_iter()
        .map(|m| Model {
            id: m.id,
            root: m.root,
            parent: m.parent,
            max_model_len: m.max_model_len,
            owned_by: m.owned_by,
        })
        .collect())
}

/// Что известно о сервере после подключения.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerFacts {
    /// Тип, с которым работает Glafira: из профиля или по признакам; версия ниже границы — `openai`.
    pub kind: DetectedKind,
    /// Тип по признакам сервера.
    pub detected: DetectedKind,
    pub version: Option<String>,
    pub models: Vec<Model>,
    /// `/metrics` отвечает, и в нём есть серии этого типа.
    pub metrics: bool,
    pub warnings: Vec<LocalizedText>,
}

fn has_series(metrics: &Option<String>, prefix: &str) -> bool {
    metrics.as_deref().is_some_and(|m| {
        m.lines()
            .any(|l| l.starts_with(prefix) || l.starts_with(&format!("# TYPE {prefix}")))
    })
}

/// Тип по признакам, без учёта профиля.
pub fn detect(p: &Probe, models: &[Model]) -> DetectedKind {
    let model_info = p.model_info.as_ref().and_then(Answer::object);
    let server_info = p.server_info.as_ref().and_then(Answer::object);
    let owned = |who: &str| models.iter().any(|m| m.owned_by.as_deref() == Some(who));
    if model_info.is_some_and(|m| m.contains_key("model_path"))
        || server_info.is_some_and(|m| m.contains_key("model_path") && m.contains_key("version"))
        || has_series(&p.metrics, "sglang:")
        || owned("sglang")
    {
        return DetectedKind::Sglang;
    }
    let version_json = p
        .version
        .as_ref()
        .and_then(Answer::object)
        .is_some_and(|m| m.get("version").is_some_and(|v| v.is_string()));
    if has_series(&p.metrics, "vllm:") || owned("vllm") || (version_json && owned_none(models)) {
        return DetectedKind::Vllm;
    }
    DetectedKind::Openai
}

/// У моделей нет `owned_by` вовсе — обратный прокси мог его вырезать; `/version` тогда решает.
fn owned_none(models: &[Model]) -> bool {
    models.iter().all(|m| m.owned_by.is_none())
}

/// Версия сервера: vLLM — `/version`, SGLang — `version` в сведениях о сервере.
fn version_of(kind: DetectedKind, p: &Probe) -> Option<String> {
    let from = |a: &Option<Answer>| {
        a.as_ref()
            .and_then(Answer::object)
            .and_then(|m| m.get("version")?.as_str().map(String::from))
            .filter(|v| !v.trim().is_empty())
    };
    match kind {
        DetectedKind::Vllm => from(&p.version),
        DetectedKind::Sglang => from(&p.server_info).or_else(|| from(&p.version)),
        DetectedKind::Openai => None,
    }
}

/// Числа начала версии: `0.10.1.1`, `0.6.3.post1`, `0.5.0rc1`, `0.11.0+cu128` → `[0, 10, 1]`.
/// Берутся первые три числа; суффиксы сборок не учитываются.
pub fn version_numbers(v: &str) -> Vec<u64> {
    v.trim()
        .trim_start_matches('v')
        .split('.')
        .take(3)
        .map_while(|part| {
            let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
            digits.parse().ok()
        })
        .collect()
}

/// Версия `v` не ниже `min`. Неразборчивая версия считается поддерживаемой: Glafira не
/// отказывается от сервера из-за необычной строки.
pub fn version_at_least(v: &str, min: &str) -> bool {
    let (mut a, mut b) = (version_numbers(v), version_numbers(min));
    if a.is_empty() {
        return true;
    }
    let n = a.len().max(b.len());
    a.resize(n, 0);
    b.resize(n, 0);
    a >= b
}

/// Сведения о сервере по снимку и типу из профиля.
pub fn analyze(configured: ServerKind, p: &Probe) -> AppResult<ServerFacts> {
    let models = parse_models(&p.models)?;
    let detected = detect(p, &models);
    let mut warnings = Vec::new();
    let mut kind = detected;
    if let Some(chosen) = DetectedKind::from_configured(configured) {
        if chosen != detected {
            warnings.push(ltext!(
                "messages.servers.kindMismatch",
                "The server looks like {{detected}}, but {{configured}} is selected in the installation",
                detected = detected.label(),
                configured = chosen.label()
            ));
        }
        kind = chosen;
    }
    let version = version_of(kind, p);
    if let (Some(v), Some(min)) = (&version, kind.min_version())
        && !version_at_least(v, min)
    {
        warnings.push(ltext!(
            "messages.servers.unsupportedVersion",
            "{{kind}} {{version}} is older than {{min}}: Glafira works with it as with an OpenAI-compatible server",
            kind = kind.label(),
            version = v,
            min = min
        ));
        kind = DetectedKind::Openai;
    }
    let metrics = match kind {
        DetectedKind::Vllm => has_series(&p.metrics, "vllm:"),
        DetectedKind::Sglang => has_series(&p.metrics, "sglang:"),
        DetectedKind::Openai => false,
    };
    if !metrics && kind == DetectedKind::Sglang {
        warnings.push(ltext!(
            "messages.servers.noMetricsSglang",
            "SGLang does not expose metrics: start it with --enable-metrics"
        ));
    } else if !metrics && kind == DetectedKind::Vllm {
        warnings.push(ltext!(
            "messages.servers.noMetrics",
            "The server does not expose its metrics at the metrics path of the installation"
        ));
    }
    Ok(ServerFacts {
        kind,
        detected,
        version,
        models,
        metrics,
        warnings,
    })
}

/// Снимки ответов настоящих серверов для тестов (`servers/testdata/<сервер>-<версия>/`).
#[cfg(test)]
pub mod fixtures {
    use super::{Answer, Probe};

    fn ok(body: &str) -> Option<Answer> {
        Some(Answer {
            status: 200,
            body: body.as_bytes().to_vec(),
        })
    }

    fn missing() -> Option<Answer> {
        Some(Answer {
            status: 404,
            body: br#"{"detail":"Not Found"}"#.to_vec(),
        })
    }

    pub fn vllm_0_11() -> Probe {
        Probe {
            version: ok(include_str!("testdata/vllm-0.11.0/version.json")),
            model_info: missing(),
            server_info: missing(),
            metrics: Some(include_str!("testdata/vllm-0.11.0/metrics.txt").into()),
            models: include_bytes!("testdata/vllm-0.11.0/models.json").to_vec(),
        }
    }

    pub fn vllm_0_8() -> Probe {
        Probe {
            version: ok(include_str!("testdata/vllm-0.8.5/version.json")),
            model_info: missing(),
            server_info: missing(),
            metrics: Some(include_str!("testdata/vllm-0.8.5/metrics.txt").into()),
            models: include_bytes!("testdata/vllm-0.8.5/models.json").to_vec(),
        }
    }

    pub fn vllm_0_7() -> Probe {
        Probe {
            version: ok(include_str!("testdata/vllm-0.7.3/version.json")),
            model_info: missing(),
            server_info: missing(),
            metrics: Some(include_str!("testdata/vllm-0.7.3/metrics.txt").into()),
            models: include_bytes!("testdata/vllm-0.7.3/models.json").to_vec(),
        }
    }

    pub fn sglang_0_5() -> Probe {
        Probe {
            version: missing(),
            model_info: ok(include_str!("testdata/sglang-0.5.4/model_info.json")),
            server_info: ok(include_str!("testdata/sglang-0.5.4/server_info.json")),
            metrics: Some(include_str!("testdata/sglang-0.5.4/metrics.txt").into()),
            models: include_bytes!("testdata/sglang-0.5.4/models.json").to_vec(),
        }
    }

    /// SGLang 0.4 без `--enable-metrics`: `/metrics` нет.
    pub fn sglang_0_4() -> Probe {
        Probe {
            version: missing(),
            model_info: ok(include_str!("testdata/sglang-0.4.6/model_info.json")),
            server_info: ok(include_str!("testdata/sglang-0.4.6/server_info.json")),
            metrics: None,
            models: include_bytes!("testdata/sglang-0.4.6/models.json").to_vec(),
        }
    }

    pub fn llamacpp() -> Probe {
        Probe {
            version: missing(),
            model_info: missing(),
            server_info: missing(),
            metrics: None,
            models: include_bytes!("testdata/llamacpp/models.json").to_vec(),
        }
    }

    pub fn ollama() -> Probe {
        Probe {
            version: missing(),
            model_info: missing(),
            server_info: missing(),
            metrics: None,
            models: include_bytes!("testdata/ollama/models.json").to_vec(),
        }
    }
}
````

<!-- block: file t06-src-tauri-src-servers-probe-rs-impl step=6.2 path=src-tauri/src/servers/probe.rs part=impl -->
````rust
//! Снимок ответов сервера для определения типа (спец. §4.3). Запросы идут параллельно;
//! обязателен только `/v1/models`, ошибка остальных путей снимок не прерывает.

use crate::error::AppResult;
use crate::servers::client::{JSON_LIMIT, ServerClient};
use crate::servers::detect::{Answer, Probe, ServerFacts, analyze};
use crate::storage::profile::ServerConfig;

/// Сколько читается из `/metrics` для определения типа: признаки — в первых сериях.
pub const METRICS_PROBE_LIMIT: usize = 256 * 1024;

async fn optional(c: &ServerClient, path: &str) -> Option<Answer> {
    let r = c.get(path, JSON_LIMIT).await.ok()?;
    Some(Answer {
        status: r.status,
        body: r.body,
    })
}

/// Ответ первого пути, который есть у сервера: новые имена SGLang, затем устаревшие.
async fn first_found(c: &ServerClient, paths: [&str; 2]) -> Option<Answer> {
    match optional(c, paths[0]).await {
        Some(a) if a.status == 404 => optional(c, paths[1]).await.or(Some(a)),
        other => other,
    }
}

/// Снимок ответов сервера. Ошибка — только если `/v1/models` не ответил 2xx.
pub async fn probe(c: &ServerClient, metrics_path: &str) -> AppResult<Probe> {
    let (models, version, model_info, server_info, metrics) = tokio::join!(
        c.get_ok("/v1/models", JSON_LIMIT),
        optional(c, "/version"),
        first_found(c, ["/model_info", "/get_model_info"]),
        first_found(c, ["/server_info", "/get_server_info"]),
        c.get(metrics_path, METRICS_PROBE_LIMIT),
    );
    let metrics = metrics
        .ok()
        .filter(|r| r.ok())
        .map(|r| String::from_utf8_lossy(&r.body).into_owned());
    Ok(Probe {
        version,
        model_info,
        server_info,
        metrics,
        models: models?.body,
    })
}

/// Тип, версия и модели сервера.
pub async fn facts(c: &ServerClient, s: &ServerConfig) -> AppResult<ServerFacts> {
    analyze(s.kind, &probe(c, &s.metrics_path).await?)
}
````

<!-- block: file t06-src-tauri-src-servers-testdata-llamacpp-health-json step=6.2 path=src-tauri/src/servers/testdata/llamacpp/health.json -->
````json
{"status":"ok"}
````

<!-- block: file t06-src-tauri-src-servers-testdata-llamacpp-models-json step=6.2 path=src-tauri/src/servers/testdata/llamacpp/models.json -->
````json
{"object":"list","data":[{"id":"qwen2.5-7b-instruct-q4_k_m.gguf","object":"model","created":1761730000,"owned_by":"llamacpp","meta":{"vocab_type":2,"n_vocab":152064,"n_ctx_train":32768,"n_embd":3584,"n_params":7615616512,"size":4677120000}}]}
````

<!-- block: file t06-src-tauri-src-servers-testdata-ollama-models-json step=6.2 path=src-tauri/src/servers/testdata/ollama/models.json -->
````json
{"object":"list","data":[{"id":"llama3.1:8b","object":"model","created":1761700000,"owned_by":"library"},{"id":"qwen3:32b","object":"model","created":1761600000,"owned_by":"library"}]}
````

<!-- block: file t06-src-tauri-src-servers-testdata-sglang-0-4-6-model-info-json step=6.2 path=src-tauri/src/servers/testdata/sglang-0.4.6/model_info.json -->
````json
{"model_path":"/models/Llama-3.1-70B-Instruct","tokenizer_path":"/models/Llama-3.1-70B-Instruct","is_generation":true}
````

<!-- block: file t06-src-tauri-src-servers-testdata-sglang-0-4-6-models-json step=6.2 path=src-tauri/src/servers/testdata/sglang-0.4.6/models.json -->
````json
{"object":"list","data":[{"id":"/models/Llama-3.1-70B-Instruct","object":"model","created":1745000000,"owned_by":"sglang","root":"/models/Llama-3.1-70B-Instruct"}]}
````

<!-- block: file t06-src-tauri-src-servers-testdata-sglang-0-4-6-server-info-json step=6.2 path=src-tauri/src/servers/testdata/sglang-0.4.6/server_info.json -->
````json
{"model_path":"/models/Llama-3.1-70B-Instruct","tp_size":4,"api_key":null,"mem_fraction_static":0.85,"enable_metrics":false,"version":"0.4.6.post1","internal_states":[{"last_gen_throughput":0.0}]}
````

<!-- block: file t06-src-tauri-src-servers-testdata-sglang-0-5-4-metrics-txt step=6.2 path=src-tauri/src/servers/testdata/sglang-0.5.4/metrics.txt -->
````text
# HELP sglang:num_running_reqs The number of running requests.
# TYPE sglang:num_running_reqs gauge
sglang:num_running_reqs{model_name="qwen3-32b"} 2.0
# HELP sglang:num_queue_reqs The number of requests in the waiting queue.
# TYPE sglang:num_queue_reqs gauge
sglang:num_queue_reqs{model_name="qwen3-32b"} 0.0
# HELP sglang:token_usage The token usage.
# TYPE sglang:token_usage gauge
sglang:token_usage{model_name="qwen3-32b"} 0.12
````

<!-- block: file t06-src-tauri-src-servers-testdata-sglang-0-5-4-model-info-json step=6.2 path=src-tauri/src/servers/testdata/sglang-0.5.4/model_info.json -->
````json
{"model_path":"/models/Qwen3-32B","tokenizer_path":"/models/Qwen3-32B","is_generation":true,"preferred_sampling_params":null,"weight_version":"default","load_format":"auto","reasoning_parser":null,"tool_call_parser":null,"disaggregation_mode":"null","has_image_understanding":false,"has_audio_understanding":false,"model_type":"qwen3","architectures":["Qwen3ForCausalLM"]}
````

<!-- block: file t06-src-tauri-src-servers-testdata-sglang-0-5-4-models-json step=6.2 path=src-tauri/src/servers/testdata/sglang-0.5.4/models.json -->
````json
{"object":"list","data":[{"id":"qwen3-32b","object":"model","created":1761730000,"owned_by":"sglang","root":"/models/Qwen3-32B","max_model_len":40960}]}
````

<!-- block: file t06-src-tauri-src-servers-testdata-sglang-0-5-4-server-info-json step=6.2 path=src-tauri/src/servers/testdata/sglang-0.5.4/server_info.json -->
````json
{"model_path":"/models/Qwen3-32B","tokenizer_path":"/models/Qwen3-32B","served_model_name":"qwen3-32b","host":"0.0.0.0","port":30000,"api_key":"sk-sglang-s3cr3t","tp_size":2,"dp_size":1,"mem_fraction_static":0.88,"context_length":null,"enable_metrics":true,"log_requests":false,"version":"0.5.4","frontend":"python","startup_time":1761730000.5,"internal_states":[{"last_gen_throughput":512.3,"memory_usage":{"weight":30.6,"kvcache":31.2,"token_capacity":245760}}]}
````

<!-- block: file t06-src-tauri-src-servers-testdata-vllm-0-11-0-metrics-txt step=6.2 path=src-tauri/src/servers/testdata/vllm-0.11.0/metrics.txt -->
````text
# HELP python_gc_objects_collected_total Objects collected during gc
# TYPE python_gc_objects_collected_total counter
python_gc_objects_collected_total{generation="0"} 21458.0
# HELP vllm:num_requests_running Number of requests in model execution batches.
# TYPE vllm:num_requests_running gauge
vllm:num_requests_running{engine="0",model_name="qwen3-32b"} 3.0
# HELP vllm:num_requests_waiting Number of requests waiting to be processed.
# TYPE vllm:num_requests_waiting gauge
vllm:num_requests_waiting{engine="0",model_name="qwen3-32b"} 0.0
# HELP vllm:kv_cache_usage_perc KV-cache usage. 1 means 100 percent usage.
# TYPE vllm:kv_cache_usage_perc gauge
vllm:kv_cache_usage_perc{engine="0",model_name="qwen3-32b"} 0.31
# HELP vllm:num_preemptions_total Cumulative number of preemption from the engine.
# TYPE vllm:num_preemptions_total counter
vllm:num_preemptions_total{engine="0",model_name="qwen3-32b"} 0.0
# HELP vllm:cache_config_info Information of the LLMEngine CacheConfig
# TYPE vllm:cache_config_info gauge
vllm:cache_config_info{block_size="16",cache_dtype="auto",enable_prefix_caching="True",engine="0",gpu_memory_utilization="0.9",num_gpu_blocks="21904",num_cpu_blocks="None",swap_space="4"} 1.0
````

<!-- block: file t06-src-tauri-src-servers-testdata-vllm-0-11-0-models-json step=6.2 path=src-tauri/src/servers/testdata/vllm-0.11.0/models.json -->
````json
{"object":"list","data":[{"id":"qwen3-32b","object":"model","created":1761730000,"owned_by":"vllm","root":"/models/Qwen3-32B","parent":null,"max_model_len":32768,"permission":[{"id":"modelperm-5b1f2c","object":"model_permission","created":1761730000,"allow_create_engine":false,"allow_sampling":true,"allow_logprobs":true,"allow_search_indices":false,"allow_view":true,"allow_fine_tuning":false,"organization":"*","group":null,"is_blocking":false}]}]}
````

<!-- block: file t06-src-tauri-src-servers-testdata-vllm-0-11-0-version-json step=6.2 path=src-tauri/src/servers/testdata/vllm-0.11.0/version.json -->
````json
{"version":"0.11.0"}
````

<!-- block: file t06-src-tauri-src-servers-testdata-vllm-0-7-3-metrics-txt step=6.2 path=src-tauri/src/servers/testdata/vllm-0.7.3/metrics.txt -->
````text
# HELP vllm:num_requests_swapped Number of requests swapped to CPU.
# TYPE vllm:num_requests_swapped gauge
vllm:num_requests_swapped{model_name="mistral-7b"} 0.0
````

<!-- block: file t06-src-tauri-src-servers-testdata-vllm-0-7-3-models-json step=6.2 path=src-tauri/src/servers/testdata/vllm-0.7.3/models.json -->
````json
{"object":"list","data":[{"id":"mistral-7b","object":"model","created":1740000000,"owned_by":"vllm","root":"mistralai/Mistral-7B-Instruct-v0.3","parent":null,"max_model_len":32768,"permission":[]}]}
````

<!-- block: file t06-src-tauri-src-servers-testdata-vllm-0-7-3-version-json step=6.2 path=src-tauri/src/servers/testdata/vllm-0.7.3/version.json -->
````json
{"version":"0.7.3"}
````

<!-- block: file t06-src-tauri-src-servers-testdata-vllm-0-8-5-metrics-txt step=6.2 path=src-tauri/src/servers/testdata/vllm-0.8.5/metrics.txt -->
````text
# HELP vllm:num_requests_running Number of requests in model execution batches.
# TYPE vllm:num_requests_running gauge
vllm:num_requests_running{engine="0",model_name="llama-3.1-8b"} 0.0
# HELP vllm:gpu_cache_usage_perc GPU KV-cache usage. 1 means 100 percent usage.
# TYPE vllm:gpu_cache_usage_perc gauge
vllm:gpu_cache_usage_perc{engine="0",model_name="llama-3.1-8b"} 0.0
````

<!-- block: file t06-src-tauri-src-servers-testdata-vllm-0-8-5-models-json step=6.2 path=src-tauri/src/servers/testdata/vllm-0.8.5/models.json -->
````json
{"object":"list","data":[{"id":"llama-3.1-8b","object":"model","created":1747000000,"owned_by":"vllm","root":"/models/Meta-Llama-3.1-8B-Instruct","parent":null,"max_model_len":131072,"permission":[]},{"id":"sql-lora","object":"model","created":1747000000,"owned_by":"vllm","root":"/adapters/sql-lora","parent":"llama-3.1-8b","max_model_len":null,"permission":[]}]}
````

<!-- block: file t06-src-tauri-src-servers-testdata-vllm-0-8-5-version-json step=6.2 path=src-tauri/src/servers/testdata/vllm-0.8.5/version.json -->
````json
{"version":"0.8.5.post1"}
````

<!-- block: patch t06-src step=6.2 -->
````diff
diff --git a/src-tauri/src/servers/mod.rs b/src-tauri/src/servers/mod.rs
index 3e08717..ddf0214 100644
--- a/src-tauri/src/servers/mod.rs
+++ b/src-tauri/src/servers/mod.rs
@@ -1,6 +1,9 @@
-//! Серверы инференса (спец. §4.3): HTTP-клиент из профиля и ошибки сервера.
+//! Серверы инференса (спец. §4.3): HTTP-клиент из профиля, ошибки сервера, определение типа,
+//! версии и моделей.
 
 pub mod client;
+pub mod detect;
 pub mod errors;
+pub mod probe;
 #[cfg(test)]
 pub mod testing;
````

- [ ] **Step 3: Проверки.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib`  
Expected: PASS — `test result: ok. 138 passed; 0 failed`

Run: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`  
Expected: PASS — без предупреждений

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml --check`  
Expected: PASS — без изменений

- [ ] **Step 4: Commit.**

```bash
git add \
  src-tauri/src/servers/detect.rs \
  src-tauri/src/servers/mod.rs \
  src-tauri/src/servers/probe.rs \
  src-tauri/src/servers/testdata/llamacpp/health.json \
  src-tauri/src/servers/testdata/llamacpp/models.json \
  src-tauri/src/servers/testdata/ollama/models.json \
  src-tauri/src/servers/testdata/sglang-0.4.6/model_info.json \
  src-tauri/src/servers/testdata/sglang-0.4.6/models.json \
  src-tauri/src/servers/testdata/sglang-0.4.6/server_info.json \
  src-tauri/src/servers/testdata/sglang-0.5.4/metrics.txt \
  src-tauri/src/servers/testdata/sglang-0.5.4/model_info.json \
  src-tauri/src/servers/testdata/sglang-0.5.4/models.json \
  src-tauri/src/servers/testdata/sglang-0.5.4/server_info.json \
  src-tauri/src/servers/testdata/vllm-0.11.0/metrics.txt \
  src-tauri/src/servers/testdata/vllm-0.11.0/models.json \
  src-tauri/src/servers/testdata/vllm-0.11.0/version.json \
  src-tauri/src/servers/testdata/vllm-0.7.3/metrics.txt \
  src-tauri/src/servers/testdata/vllm-0.7.3/models.json \
  src-tauri/src/servers/testdata/vllm-0.7.3/version.json \
  src-tauri/src/servers/testdata/vllm-0.8.5/metrics.txt \
  src-tauri/src/servers/testdata/vllm-0.8.5/models.json \
  src-tauri/src/servers/testdata/vllm-0.8.5/version.json
git commit -s -m "feat(servers): detect vLLM, SGLang or an OpenAI-compatible server, its version and models from response snapshots" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 7: Подключение инсталляции: все серверы параллельно, пул клиентов, одно подключение за раз

**Files:**
- Create: `src-tauri/src/servers/pool.rs`
- Modify: `src-tauri/Cargo.toml`, `src-tauri/src/commands/folders.rs`, `src-tauri/src/servers/mod.rs`, `src-tauri/src/state.rs`
- Generated: `src-tauri/Cargo.lock` (cargo, pnpm)

**Interfaces:**
- Consumes: задачи 5–6; задача 2: `secrets`.
- Produces: `servers::pool::{ServerPool { get, insert, evict, connect_lock }, Connected { info, clients, client(id) }, InstallationInfo { installation_id, servers }, ServerState { server_id, facts?, error? }, read_api_keys, open}`; `AppState::{pool, connected(id, fresh), evict_installation(id)}`; импорт забывает подключения обновлённых инсталляций.

Подключение инсталляции — подключение каждого её сервера; ошибка сервера остаётся в его состоянии и не мешает остальным. Ключи API читаются из keychain вне async-потоков (`spawn_blocking`), сбой keychain — ошибка только этого сервера. Подключения одной инсталляции идут по очереди: кто ждал, получает готовый результат, сервер опрашивается один раз.

- [ ] **Step 1: Тесты.**

<!-- block: file t07-src-tauri-src-servers-pool-rs-tests step=7.1 path=src-tauri/src/servers/pool.rs part=tests -->
````rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use crate::secrets::MemoryStore;
    use crate::servers::detect::DetectedKind;
    use crate::servers::testing::{Reply, TestServer};
    use crate::storage::profile::{CaConfig, ProxyConfig, sample_profile, sample_server};

    pub(crate) fn vllm_routes() -> Vec<(&'static str, Reply)> {
        vec![
            (
                "/v1/models",
                Reply::json(include_str!("testdata/vllm-0.11.0/models.json")),
            ),
            (
                "/version",
                Reply::json(include_str!("testdata/vllm-0.11.0/version.json")),
            ),
            (
                "/metrics",
                Reply::text(include_str!("testdata/vllm-0.11.0/metrics.txt")),
            ),
        ]
    }

    fn server(id: &str, url: &str) -> ServerConfig {
        let mut s = sample_server(id);
        s.base_url = url.into();
        s.proxy = ProxyConfig::None;
        s
    }

    #[tokio::test]
    async fn every_server_gets_facts_or_its_own_error() {
        let ok = TestServer::start(vllm_routes()).await;
        let denied = TestServer::start(vec![("/v1/models", Reply::status(401))]).await;
        let mut p = sample_profile("a");
        p.servers = vec![server("s1", &ok.url()), server("s2", &denied.url()), {
            let mut s = server("s3", "https://vllm:8000");
            s.ca = CaConfig::Insecure;
            s
        }];
        let secrets = MemoryStore::default();
        secrets
            .set(&secret_key("a", "s1", API_KEY), "sk-one")
            .unwrap();
        let policy = PolicyStatus {
            allow_insecure_tls: false,
            ..PolicyStatus::default()
        };
        let c = open(&p, read_api_keys(&secrets, &p), &policy).await;
        let s = &c.info.servers;
        assert_eq!(
            s.iter().map(|x| x.server_id.as_str()).collect::<Vec<_>>(),
            ["s1", "s2", "s3"]
        );
        assert_eq!(s[0].facts.as_ref().unwrap().kind, DetectedKind::Vllm);
        assert_eq!(s[1].error.as_ref().unwrap().code, ErrorCode::AuthFailed);
        assert_eq!(
            s[2].error.as_ref().unwrap().key,
            "errors.installations.insecureForbidden"
        );
        assert!(c.client("s1").is_some() && c.client("s2").is_none());
        assert_eq!(
            ok.seen.lock().unwrap()[0].authorization.as_deref(),
            Some("Bearer sk-one")
        );
    }

    #[test]
    fn keychain_failure_is_an_error_of_that_server_only() {
        struct Broken;
        impl SecretStore for Broken {
            fn get(&self, key: &str) -> AppResult<Option<String>> {
                if key.ends_with("/s2/apiKey") {
                    Err(AppError::internal_with("locked"))
                } else {
                    Ok(None)
                }
            }
            fn set(&self, _: &str, _: &str) -> AppResult<()> {
                Ok(())
            }
            fn delete(&self, _: &str) -> AppResult<()> {
                Ok(())
            }
        }
        let mut p = sample_profile("a");
        p.servers.push(sample_server("s2"));
        let keys = read_api_keys(&Broken, &p);
        assert!(matches!(keys["s1"], Ok(None)));
        assert!(keys["s2"].is_err());
    }

    fn connected(id: &str) -> Arc<Connected> {
        Arc::new(Connected {
            info: InstallationInfo {
                installation_id: id.into(),
                servers: Vec::new(),
            },
            clients: HashMap::new(),
        })
    }

    #[test]
    fn insert_get_evict() {
        let pool = ServerPool::default();
        assert!(pool.get("a").is_none());
        pool.insert("a", connected("a"));
        assert_eq!(pool.get("a").unwrap().info.installation_id, "a");
        pool.evict("a");
        assert!(pool.get("a").is_none());
    }

    #[tokio::test]
    async fn connect_lock_serializes_one_installation_only() {
        let pool = Arc::new(ServerPool::default());
        let a = pool.connect_lock("a").await;
        let other = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            pool.connect_lock("b"),
        )
        .await;
        assert!(other.is_ok(), "другая инсталляция не ждёт");
        let same = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            pool.connect_lock("a"),
        )
        .await;
        assert!(same.is_err(), "та же инсталляция ждёт");
        drop(a);
        assert!(
            tokio::time::timeout(std::time::Duration::from_secs(1), pool.connect_lock("a"))
                .await
                .is_ok()
        );
    }

    #[test]
    fn info_serializes_facts_or_error() {
        let info = InstallationInfo {
            installation_id: "a".into(),
            servers: vec![ServerState {
                server_id: "s1".into(),
                facts: None,
                error: Some(crate::servers::errors::timeout()),
            }],
        };
        let v = serde_json::to_value(info).unwrap();
        assert_eq!(v["installationId"], "a");
        assert_eq!(v["servers"][0]["serverId"], "s1");
        assert_eq!(v["servers"][0]["error"]["code"], "TIMEOUT");
        assert!(v["servers"][0].get("facts").is_none());
    }
}
````

- [ ] **Step 2: Реализация.**

<!-- block: file t07-src-tauri-src-servers-pool-rs-impl step=7.2 path=src-tauri/src/servers/pool.rs part=impl -->
````rust
//! Подключённые инсталляции (спец. §4.1): клиенты серверов и сведения о них. Подключение
//! инсталляции — это подключение каждого её сервера; сервер с ошибкой не мешает остальным.
//! Подключения одной инсталляции идут по очереди: кто ждал, получает готовый результат.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::policy::PolicyStatus;
use crate::secrets::{API_KEY, SecretStore, secret_key};
use crate::servers::client::{ServerClient, Timeouts};
use crate::servers::detect::ServerFacts;
use crate::storage::profile::{InstallationProfile, ServerConfig};
use crate::sync::MutexExt;

/// Итог подключения сервера: сведения или ошибка.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerState {
    pub server_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facts: Option<ServerFacts>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AppError>,
}

/// Подключённая инсталляция для фронтенда: серверы в порядке профиля.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallationInfo {
    pub installation_id: String,
    pub servers: Vec<ServerState>,
}

/// Подключённая инсталляция: сведения и клиенты серверов, которые подключились.
pub struct Connected {
    pub info: InstallationInfo,
    pub clients: HashMap<String, Arc<ServerClient>>,
}

impl Connected {
    pub fn client(&self, server_id: &str) -> Option<Arc<ServerClient>> {
        self.clients.get(server_id).cloned()
    }
}

#[derive(Default)]
pub struct ServerPool {
    connected: Mutex<HashMap<String, Arc<Connected>>>,
    locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}

impl ServerPool {
    pub fn get(&self, id: &str) -> Option<Arc<Connected>> {
        self.connected.lock_safe().get(id).cloned()
    }

    pub fn insert(&self, id: &str, c: Arc<Connected>) {
        self.connected.lock_safe().insert(id.to_string(), c);
    }

    /// Забывает инсталляцию: следующее чтение подключится заново.
    pub fn evict(&self, id: &str) {
        self.connected.lock_safe().remove(id);
    }

    /// Очередь подключений инсталляции `id`.
    pub async fn connect_lock(&self, id: &str) -> tokio::sync::OwnedMutexGuard<()> {
        let lock = self
            .locks
            .lock_safe()
            .entry(id.to_string())
            .or_default()
            .clone();
        lock.lock_owned().await
    }
}

/// Ключи API серверов из keychain: блокирующий вызов, выполнять вне async-потоков. Сбой
/// keychain — ошибка только этого сервера.
pub fn read_api_keys(
    secrets: &dyn SecretStore,
    p: &InstallationProfile,
) -> HashMap<String, AppResult<Option<String>>> {
    p.servers
        .iter()
        .map(|s| {
            (
                s.id.clone(),
                secrets.get(&secret_key(&p.id, &s.id, API_KEY)),
            )
        })
        .collect()
}

async fn open_server(
    s: &ServerConfig,
    timeouts: Timeouts,
    key: AppResult<Option<String>>,
    policy: &PolicyStatus,
) -> AppResult<(ServerClient, ServerFacts)> {
    let client = ServerClient::new(s, timeouts, key?, policy)?;
    let facts = crate::servers::probe::facts(&client, s).await?;
    Ok((client, facts))
}

/// Подключает все серверы инсталляции параллельно. Ошибка сервера остаётся в его состоянии.
pub async fn open(
    p: &InstallationProfile,
    mut keys: HashMap<String, AppResult<Option<String>>>,
    policy: &PolicyStatus,
) -> Connected {
    let timeouts = Timeouts::from(p);
    let results = futures::future::join_all(p.servers.iter().map(|s| {
        let key = keys.remove(&s.id).unwrap_or(Ok(None));
        async move { (s.id.clone(), open_server(s, timeouts, key, policy).await) }
    }))
    .await;
    let mut clients = HashMap::new();
    let servers = results
        .into_iter()
        .map(|(server_id, r)| match r {
            Ok((client, facts)) => {
                clients.insert(server_id.clone(), Arc::new(client));
                ServerState {
                    server_id,
                    facts: Some(facts),
                    error: None,
                }
            }
            Err(e) => ServerState {
                server_id,
                facts: None,
                error: Some(e),
            },
        })
        .collect();
    Connected {
        info: InstallationInfo {
            installation_id: p.id.clone(),
            servers,
        },
        clients,
    }
}
````

<!-- block: patch t07-src step=7.2 -->
````diff
diff --git a/src-tauri/Cargo.toml b/src-tauri/Cargo.toml
index 741ee80..8bdec1f 100644
--- a/src-tauri/Cargo.toml
+++ b/src-tauri/Cargo.toml
@@ -32,6 +32,7 @@ url = "2"
 uuid = { version = "1", features = ["v4"] }
 time = { version = "0.3", features = ["formatting", "parsing"] }
 tokio = { version = "1", features = ["rt", "macros", "time", "net", "io-util", "sync"] }
+futures = "0.3"
 reqwest = { version = "0.13", default-features = false, features = ["rustls-no-provider", "http2", "system-proxy"] }
 sys-locale = "0.3"
 
diff --git a/src-tauri/src/commands/folders.rs b/src-tauri/src/commands/folders.rs
index 0d58772..6e4a2f3 100644
--- a/src-tauri/src/commands/folders.rs
+++ b/src-tauri/src/commands/folders.rs
@@ -181,6 +181,10 @@ fn import_from_file(
             tracing::warn!("api key of a removed server was not deleted: {e}");
         }
     }
+    // Обновлённые инсталляции подключатся заново с новым профилем — как при сохранении формы.
+    for item in &preview.updated {
+        st.evict_installation(&item.id);
+    }
     Ok(preview)
 }
 
@@ -255,15 +259,30 @@ mod tests {
         let conns = dir.path().join("installations.json");
         let before = std::fs::read(&conns).unwrap();
 
+        st.pool.insert(
+            "a",
+            Arc::new(crate::servers::pool::Connected {
+                info: crate::servers::pool::InstallationInfo {
+                    installation_id: "a".into(),
+                    servers: Vec::new(),
+                },
+                clients: Default::default(),
+            }),
+        );
         let preview = import_from_file(&st, &path, None, true, false).unwrap();
         assert_eq!(preview.updated.len(), 1);
         assert_eq!(preview.added.len(), 1);
+        assert!(st.pool.get("a").is_some(), "предпросмотр ничего не меняет");
         // prod-инсталляция импортируется как в файле: без принудительного read-only.
         assert!(preview.warnings.is_empty());
         assert_eq!(std::fs::read(&conns).unwrap(), before);
 
         let applied = import_from_file(&st, &path, None, true, true).unwrap();
         assert_eq!(applied, preview);
+        assert!(
+            st.pool.get("a").is_none(),
+            "обновлённая инсталляция подключится заново"
+        );
         assert_eq!(
             st.installations.get("a").unwrap().servers[0].base_url,
             "http://changed:8000"
diff --git a/src-tauri/src/servers/mod.rs b/src-tauri/src/servers/mod.rs
index ddf0214..3aa0e6d 100644
--- a/src-tauri/src/servers/mod.rs
+++ b/src-tauri/src/servers/mod.rs
@@ -1,9 +1,10 @@
 //! Серверы инференса (спец. §4.3): HTTP-клиент из профиля, ошибки сервера, определение типа,
-//! версии и моделей.
+//! версии и моделей, подключённые инсталляции.
 
 pub mod client;
 pub mod detect;
 pub mod errors;
+pub mod pool;
 pub mod probe;
 #[cfg(test)]
 pub mod testing;
diff --git a/src-tauri/src/state.rs b/src-tauri/src/state.rs
index 6486eea..c7185c8 100644
--- a/src-tauri/src/state.rs
+++ b/src-tauri/src/state.rs
@@ -1,17 +1,20 @@
 use std::path::Path;
 use std::sync::Arc;
 
-use crate::error::AppResult;
+use crate::error::{AppResult, ErrorCode};
 use crate::secrets::SecretStore;
+use crate::servers::pool::{Connected, ServerPool, open, read_api_keys};
 use crate::storage::installations::InstallationStore;
 
-/// Общее состояние приложения: хранилища и политика. Клонируется дёшево (всё за `Arc`).
+/// Общее состояние приложения: хранилища, политика и подключённые инсталляции.
+/// Клонируется дёшево (всё за `Arc`).
 #[derive(Clone)]
 pub struct AppState {
     pub installations: Arc<InstallationStore>,
     pub secrets: Arc<dyn SecretStore>,
     pub policy: Arc<crate::policy::Policy>,
     pub settings: Arc<crate::settings::SettingsStore>,
+    pub pool: Arc<ServerPool>,
 }
 
 impl AppState {
@@ -21,6 +24,7 @@ impl AppState {
             secrets,
             policy: Arc::new(crate::policy::Policy::default()),
             settings: Arc::new(crate::settings::SettingsStore::open(data_dir)),
+            pool: Arc::new(ServerPool::default()),
         })
     }
 
@@ -29,4 +33,106 @@ impl AppState {
         self.policy = Arc::new(policy);
         self
     }
+
+    /// Подключённая инсталляция: из пула или новое подключение (`fresh` — всегда новое).
+    /// Подключения одной инсталляции идут по очереди: кто ждал, получает готовый результат.
+    /// Ключи API читаются из keychain вне async-потоков.
+    pub async fn connected(&self, id: &str, fresh: bool) -> AppResult<Arc<Connected>> {
+        let _turn = self.pool.connect_lock(id).await;
+        if fresh {
+            self.pool.evict(id);
+        } else if let Some(c) = self.pool.get(id) {
+            return Ok(c);
+        }
+        let profile = self.installations.get(id)?;
+        let keys = {
+            let (secrets, p) = (self.secrets.clone(), profile.clone());
+            tokio::task::spawn_blocking(move || read_api_keys(secrets.as_ref(), &p))
+                .await
+                .map_err(|e| {
+                    app_err!(
+                        ErrorCode::Internal,
+                        "errors.app.taskPanicked",
+                        "Background task failed: {{error}}",
+                        error = e
+                    )
+                })?
+        };
+        let c = Arc::new(open(&profile, keys, &self.policy.status).await);
+        self.pool.insert(id, c.clone());
+        Ok(c)
+    }
+
+    /// Забывает подключение инсталляции: профиль или ключи изменились, удалена.
+    pub fn evict_installation(&self, id: &str) {
+        self.pool.evict(id);
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use crate::secrets::{API_KEY, MemoryStore, secret_key};
+    use crate::servers::testing::{Reply, TestServer};
+    use crate::storage::profile::{ProxyConfig, sample_profile};
+
+    async fn state_with(url: &str) -> (tempfile::TempDir, AppState, Arc<MemoryStore>) {
+        let dir = tempfile::tempdir().unwrap();
+        let secrets = Arc::new(MemoryStore::default());
+        let st = AppState::new(dir.path(), secrets.clone()).unwrap();
+        let mut p = sample_profile("a");
+        p.servers[0].base_url = url.into();
+        p.servers[0].proxy = ProxyConfig::None;
+        st.installations.upsert(p).unwrap();
+        (dir, st, secrets)
+    }
+
+    #[tokio::test]
+    async fn connection_is_reused_until_fresh_or_evicted() {
+        let srv = TestServer::start(vec![(
+            "/v1/models",
+            Reply::json(r#"{"data":[{"id":"m","owned_by":"llamacpp"}]}"#),
+        )])
+        .await;
+        let (_d, st, secrets) = state_with(&srv.url()).await;
+        secrets
+            .set(&secret_key("a", "s1", API_KEY), "sk-a")
+            .unwrap();
+        let c = st.connected("a", false).await.unwrap();
+        assert_eq!(c.info.servers[0].facts.as_ref().unwrap().models[0].id, "m");
+        let models = |srv: &TestServer| srv.paths().iter().filter(|p| *p == "/v1/models").count();
+        assert_eq!(models(&srv), 1);
+        st.connected("a", false).await.unwrap();
+        assert_eq!(models(&srv), 1, "из пула");
+        st.connected("a", true).await.unwrap();
+        assert_eq!(models(&srv), 2, "fresh — заново");
+        st.evict_installation("a");
+        st.connected("a", false).await.unwrap();
+        assert_eq!(models(&srv), 3, "после сброса — заново");
+        assert_eq!(
+            srv.seen.lock().unwrap()[0].authorization.as_deref(),
+            Some("Bearer sk-a")
+        );
+    }
+
+    #[tokio::test]
+    async fn unknown_installation_is_not_found() {
+        let dir = tempfile::tempdir().unwrap();
+        let st = AppState::new(dir.path(), Arc::new(MemoryStore::default())).unwrap();
+        let e = st.connected("nope", false).await.err().unwrap();
+        assert_eq!(e.code, ErrorCode::NotFound);
+    }
+
+    #[tokio::test]
+    async fn concurrent_connects_probe_once() {
+        let srv = TestServer::start(vec![(
+            "/v1/models",
+            Reply::json(r#"{"data":[]}"#).delayed(std::time::Duration::from_millis(200)),
+        )])
+        .await;
+        let (_d, st, _) = state_with(&srv.url()).await;
+        let (a, b) = tokio::join!(st.connected("a", false), st.connected("a", false));
+        assert!(Arc::ptr_eq(&a.unwrap(), &b.unwrap()));
+        assert_eq!(srv.paths().iter().filter(|p| *p == "/v1/models").count(), 1);
+    }
 }
````

- [ ] **Step 3: Проверки.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib`  
Expected: PASS — `test result: ok. 146 passed; 0 failed`

Run: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`  
Expected: PASS — без предупреждений

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml --check`  
Expected: PASS — без изменений

Run: `cargo deny --manifest-path src-tauri/Cargo.toml --config deny.toml check licenses advisories`  
Expected: PASS — `advisories ok, licenses ok`

Run: `cargo tree --manifest-path src-tauri/Cargo.toml -i aws-lc-rs`  
Expected: ошибка `did not match any packages` — `aws-lc-rs` в дереве нет (то же для `openssl-sys`)

- [ ] **Step 4: Commit.**

```bash
git add \
  src-tauri/Cargo.lock \
  src-tauri/Cargo.toml \
  src-tauri/src/commands/folders.rs \
  src-tauri/src/servers/mod.rs \
  src-tauri/src/servers/pool.rs \
  src-tauri/src/state.rs
git commit -s -m "feat(servers): connect an installation — every server in parallel, pool of clients, one connect at a time" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 8: Пошаговая проверка сервера: конфигурация, DNS/TCP, TLS, /health, ключ API, тип и версия, модели

**Files:**
- Create (по Kubernetes-версии): `src-tauri/src/servers/connectivity.rs` ← `src-tauri/src/k8s/connectivity.rs`
- Modify: `PORTED.md`, `src-tauri/src/servers/mod.rs`, `src-tauri/src/storage/profile.rs`

**Interfaces:**
- Consumes: задачи 5–6; задача 4: `validate_server` (становится pub).
- Produces: `servers::connectivity::{test_server(server, timeouts, api_key, policy) -> TestReport { ok, steps, warnings }, TestStep, tcp_target, tcp_probe}`.

Перенос `k8s/connectivity.rs` под сервер инференса (спец. §5.8). TCP — к прокси профиля, если он задан; при системном прокси шаг пропускается, соединение проверяют запросы. TLS — только для `https://`: любой HTTP-ответ доказывает рукопожатие. `/health`: 404 — пропуск (у Ollama его нет), 401/403 — проверит шаг «Вход». Вход без ключа и с неверным ключом различаются; сервер без `/v1/models` и без моделей — ошибка. Предупреждения успеха не отменяют.

- [ ] **Step 1: Тесты.**

<!-- block: file t08-src-tauri-src-servers-connectivity-rs-tests step=8.1 path=src-tauri/src/servers/connectivity.rs part=tests -->
````rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::servers::testing::{Reply, TEST_CA_PEM, TestServer};
    use crate::storage::profile::{CaConfig, sample_server};

    const TIMEOUTS: Timeouts = Timeouts {
        connect_ms: 2_000,
        request_ms: 2_000,
    };

    fn names(r: &TestReport) -> Vec<(&str, bool)> {
        r.steps.iter().map(|s| (s.name, s.ok)).collect()
    }

    fn server(url: &str) -> ServerConfig {
        let mut s = sample_server("s1");
        s.base_url = url.into();
        s.proxy = ProxyConfig::None;
        s
    }

    fn vllm(health: Reply) -> Vec<(&'static str, Reply)> {
        vec![
            ("/health", health),
            (
                "/v1/models",
                Reply::json(include_str!("testdata/vllm-0.11.0/models.json")),
            ),
            (
                "/version",
                Reply::json(include_str!("testdata/vllm-0.11.0/version.json")),
            ),
            (
                "/metrics",
                Reply::text(include_str!("testdata/vllm-0.11.0/metrics.txt")),
            ),
        ]
    }

    async fn run(s: &ServerConfig, key: Option<&str>) -> TestReport {
        test_server(s, TIMEOUTS, key.map(String::from), &PolicyStatus::default()).await
    }

    #[tokio::test]
    async fn all_steps_pass_on_vllm_over_http() {
        let srv = TestServer::start(vllm(Reply::status(200))).await;
        let r = run(&server(&srv.url()), Some("sk-1")).await;
        assert!(r.ok);
        assert_eq!(
            names(&r),
            [
                ("config", true),
                ("tcp", true),
                ("health", true),
                ("auth", true),
                ("kind", true),
                ("models", true)
            ]
        );
        assert_eq!(r.steps[1].message.params["address"], srv.addr.to_string());
        assert_eq!(
            r.steps[3].message.key,
            "messages.connectionTest.keyAccepted"
        );
        assert_eq!(r.steps[4].message.params["kind"], "vLLM");
        assert_eq!(r.steps[4].message.params["version"], "0.11.0");
        assert_eq!(r.steps[5].message.params["models"], "qwen3-32b (32768)");
        assert!(r.warnings.is_empty());
    }

    #[tokio::test]
    async fn https_adds_a_tls_step_and_stops_on_an_untrusted_certificate() {
        let srv = TestServer::start_tls(vllm(Reply::status(200))).await;
        let mut s = server(&srv.url());
        s.ca = CaConfig::Pem {
            pem: TEST_CA_PEM.into(),
        };
        let r = run(&s, None).await;
        assert!(r.ok);
        assert_eq!(r.steps[2].name, "tls");
        assert_eq!(
            r.steps[4].message.key,
            "messages.connectionTest.noKeyNeeded"
        );
        s.ca = CaConfig::System;
        let r = run(&s, None).await;
        assert_eq!(names(&r), [("config", true), ("tcp", true), ("tls", false)]);
        assert_eq!(r.steps[2].error.as_ref().unwrap().code, ErrorCode::TlsError);
    }

    #[tokio::test]
    async fn config_and_tcp_failures_stop_early() {
        let mut s = server("https://vllm:8000");
        s.ca = CaConfig::Insecure;
        let policy = PolicyStatus {
            allow_insecure_tls: false,
            ..PolicyStatus::default()
        };
        let r = test_server(&s, TIMEOUTS, None, &policy).await;
        assert_eq!(names(&r), [("config", false)]);
        assert_eq!(
            r.steps[0].message.key,
            "errors.installations.insecureForbidden"
        );
        let r = run(&server("ftp://h"), None).await;
        assert_eq!(r.steps[0].message.key, "errors.installations.badServerUrl");
        let closed = Timeouts {
            connect_ms: 10_000,
            request_ms: 10_000,
        };
        let r = test_server(
            &server("http://127.0.0.1:1"),
            closed,
            None,
            &PolicyStatus::default(),
        )
        .await;
        assert_eq!(names(&r), [("config", true), ("tcp", false)]);
    }

    #[tokio::test]
    async fn health_answers() {
        let srv = TestServer::start(vllm(Reply::status(503))).await;
        let r = run(&server(&srv.url()), None).await;
        assert_eq!(names(&r).last(), Some(&("health", false)));
        assert_eq!(r.steps[2].message.key, "errors.server.unhealthy");
        // Ollama: /health нет — шаг пропущен, проверка идёт дальше.
        let srv = TestServer::start(vec![(
            "/v1/models",
            Reply::json(include_str!("testdata/ollama/models.json")),
        )])
        .await;
        let r = run(&server(&srv.url()), None).await;
        assert!(r.ok);
        assert_eq!(
            r.steps[2].message.key,
            "messages.connectionTest.healthMissing"
        );
        assert_eq!(r.steps[4].message.params["kind"], "OpenAI-compatible");
    }

    #[tokio::test]
    async fn missing_and_rejected_keys_are_told_apart() {
        let srv = TestServer::start(vec![
            ("/health", Reply::status(401)),
            ("/v1/models", Reply::status(401)),
        ])
        .await;
        let r = run(&server(&srv.url()), None).await;
        assert_eq!(names(&r).last(), Some(&("auth", false)));
        assert_eq!(
            r.steps[2].message.key,
            "messages.connectionTest.healthNeedsKey"
        );
        assert_eq!(r.steps[3].message.key, "errors.server.keyRequired");
        let r = run(&server(&srv.url()), Some("sk-wrong")).await;
        assert_eq!(r.steps[3].message.key, "errors.server.unauthorized");
    }

    #[tokio::test]
    async fn not_an_openai_server_and_no_models() {
        let srv = TestServer::start(vec![("/health", Reply::status(200))]).await;
        let r = run(&server(&srv.url()), None).await;
        assert_eq!(
            r.steps.last().unwrap().message.key,
            "errors.server.notOpenAi"
        );
        let srv = TestServer::start(vec![(
            "/v1/models",
            Reply::json(r#"{"object":"list","data":[]}"#),
        )])
        .await;
        let r = run(&server(&srv.url()), None).await;
        assert_eq!(names(&r).last(), Some(&("models", false)));
        assert!(!r.ok);
    }

    #[tokio::test]
    async fn warnings_do_not_fail_the_test() {
        let srv = TestServer::start(vec![
            (
                "/v1/models",
                Reply::json(include_str!("testdata/vllm-0.7.3/models.json")),
            ),
            (
                "/version",
                Reply::json(include_str!("testdata/vllm-0.7.3/version.json")),
            ),
        ])
        .await;
        let r = run(&server(&srv.url()), None).await;
        assert!(r.ok);
        assert_eq!(r.warnings[0].key, "messages.servers.unsupportedVersion");
        assert_eq!(r.steps[4].message.params["version"], "0.7.3");
    }

    #[tokio::test]
    async fn tcp_goes_to_the_proxy_of_the_profile() {
        let proxy = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = proxy.local_addr().unwrap().port();
        let mut s = server("http://vllm.internal:8000");
        s.proxy = ProxyConfig::Url {
            url: format!("http://127.0.0.1:{port}"),
        };
        let url = parse_base_url(&s).unwrap();
        assert_eq!(tcp_target(&s, &url), Some(("127.0.0.1".into(), port, true)));
        let s = server("https://vllm.internal");
        let url = parse_base_url(&s).unwrap();
        assert_eq!(
            tcp_target(&s, &url),
            Some(("vllm.internal".into(), 443, false))
        );
    }

    #[tokio::test]
    async fn tcp_probe_reaches_listener_and_reports_refused_port() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let addr = tcp_probe("127.0.0.1", port, Duration::from_secs(2))
            .await
            .unwrap();
        assert_eq!(addr, format!("127.0.0.1:{port}"));
        // Порт 1 закрыт; срок с запасом: на Windows отказ приходит примерно через 2 с.
        let e = tcp_probe("127.0.0.1", 1, Duration::from_secs(10))
            .await
            .unwrap_err();
        assert_eq!(e.code, ErrorCode::ConnectionFailed);
    }
}
````

- [ ] **Step 2: Реализация.**

<!-- block: file t08-src-tauri-src-servers-connectivity-rs-impl step=8.2 path=src-tauri/src/servers/connectivity.rs part=impl -->
````rust
//! Пошаговая проверка подключения сервера (спец. §5.8): конфигурация → DNS/TCP → TLS → `/health` →
//! вход → тип и версия → модели. Останавливается на первом неуспешном шаге и называет его;
//! предупреждения (старая версия, другой тип в профиле, нет метрик) успеха не отменяют.

use std::time::Duration;

use serde::Serialize;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::i18n::LocalizedText;
use crate::policy::PolicyStatus;
use crate::servers::client::{JSON_LIMIT, ServerClient, Timeouts};
use crate::servers::detect::{ServerFacts, analyze};
use crate::servers::errors::timeout;
use crate::servers::probe::probe;
use crate::storage::profile::{ProxyConfig, ServerConfig, parse_base_url, validate_server};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestStep {
    pub name: &'static str,
    pub ok: bool,
    pub message: LocalizedText,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AppError>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestReport {
    pub ok: bool,
    pub steps: Vec<TestStep>,
    pub warnings: Vec<LocalizedText>,
}

fn passed(name: &'static str, message: LocalizedText) -> TestStep {
    TestStep {
        name,
        ok: true,
        message,
        error: None,
    }
}

fn failed_step(name: &'static str, e: AppError) -> TestStep {
    TestStep {
        name,
        ok: false,
        message: e.text(),
        error: Some(e),
    }
}

fn report(steps: Vec<TestStep>, warnings: Vec<LocalizedText>) -> TestReport {
    TestReport {
        ok: steps.iter().all(|s| s.ok),
        steps,
        warnings,
    }
}

/// Куда открыть TCP: прокси из профиля или сервер (порт по умолчанию — 80 или 443). `None` —
/// системный прокси из окружения: прямое соединение ничего бы не доказало.
pub fn tcp_target(s: &ServerConfig, url: &url::Url) -> Option<(String, u16, bool)> {
    let proxy = match &s.proxy {
        ProxyConfig::Url { url } => url::Url::parse(url.trim()).ok(),
        ProxyConfig::System if system_proxy_set(url.scheme()) => return None,
        _ => None,
    };
    let (target, via_proxy) = match &proxy {
        Some(p) => (p, true),
        None => (url, false),
    };
    let host = target
        .host_str()?
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_string();
    Some((host, target.port_or_known_default()?, via_proxy))
}

/// Задан ли системный прокси для схемы адреса (`HTTPS_PROXY` / `HTTP_PROXY`, `ALL_PROXY`).
fn system_proxy_set(scheme: &str) -> bool {
    let names: &[&str] = if scheme == "https" {
        &["HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"]
    } else {
        &["HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"]
    };
    names
        .iter()
        .any(|n| std::env::var(n).is_ok_and(|v| !v.trim().is_empty()))
}

/// DNS и TCP-соединение с `host:port` за отведённое время. Успех — адрес, к которому подключились.
pub async fn tcp_probe(host: &str, port: u16, limit: Duration) -> AppResult<String> {
    let addrs: Vec<_> =
        match tokio::time::timeout(limit, tokio::net::lookup_host((host, port))).await {
            Ok(Ok(a)) => a.collect(),
            Ok(Err(e)) => {
                return Err(app_err!(
                    ErrorCode::ConnectionFailed,
                    "errors.server.dns",
                    "Cannot resolve the server host name"
                )
                .with_details(e.to_string()));
            }
            Err(_) => return Err(timeout()),
        };
    let mut last = None;
    for addr in addrs {
        match tokio::time::timeout(limit, tokio::net::TcpStream::connect(addr)).await {
            Ok(Ok(_)) => return Ok(addr.to_string()),
            Ok(Err(e)) => last = Some(e.to_string()),
            Err(_) => last = Some("timed out".into()),
        }
    }
    Err(app_err!(
        ErrorCode::ConnectionFailed,
        "errors.server.connect",
        "Cannot connect to the server"
    )
    .with_details(last.unwrap_or_else(|| format!("{host}: no addresses"))))
}

/// Сколько моделей шаг «Модели» называет по имени; остальные — числом.
const SHOWN_MODELS: usize = 5;

fn models_message(f: &ServerFacts) -> LocalizedText {
    let mut list = f
        .models
        .iter()
        .take(SHOWN_MODELS)
        .map(|m| match m.max_model_len {
            Some(len) => format!("{} ({len})", m.id),
            None => m.id.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ");
    if f.models.len() > SHOWN_MODELS {
        list.push_str(&format!(", +{}", f.models.len() - SHOWN_MODELS));
    }
    ltext!(
        "messages.connectionTest.models",
        "Models: {{models}}",
        models = list
    )
}

fn kind_message(f: &ServerFacts) -> LocalizedText {
    match &f.version {
        Some(v) => ltext!(
            "messages.connectionTest.kindVersion",
            "{{kind}} {{version}}",
            kind = f.detected.label(),
            version = v
        ),
        None => ltext!(
            "messages.connectionTest.kind",
            "{{kind}}",
            kind = f.detected.label()
        ),
    }
}

/// Проверка одного сервера. `api_key` — из формы или keychain; пустой — без ключа.
pub async fn test_server(
    s: &ServerConfig,
    timeouts: Timeouts,
    api_key: Option<String>,
    policy: &PolicyStatus,
) -> TestReport {
    let mut steps = Vec::new();
    let has_key = api_key.as_deref().is_some_and(|k| !k.trim().is_empty());
    let prepared = validate_server(s)
        .and_then(|()| parse_base_url(s))
        .and_then(|url| Ok((url, ServerClient::new(s, timeouts, api_key, policy)?)));
    let (url, client) = match prepared {
        Ok(p) => p,
        Err(e) => return report(vec![failed_step("config", e)], Vec::new()),
    };
    steps.push(passed(
        "config",
        ltext!("messages.connectionTest.configOk", "Configuration is valid"),
    ));

    let limit = Duration::from_millis(timeouts.connect_ms);
    match tcp_target(s, &url) {
        None => steps.push(passed(
            "tcp",
            ltext!(
                "messages.connectionTest.tcpSystemProxy",
                "The system proxy is used: the connection is checked by the requests below"
            ),
        )),
        Some((host, port, via_proxy)) => match tcp_probe(&host, port, limit).await {
            Ok(address) if via_proxy => steps.push(passed(
                "tcp",
                ltext!(
                    "messages.connectionTest.tcpProxyOk",
                    "TCP connection to the proxy {{address}} established",
                    address = address
                ),
            )),
            Ok(address) => steps.push(passed(
                "tcp",
                ltext!(
                    "messages.connectionTest.tcpOk",
                    "TCP connection to {{address}} established",
                    address = address
                ),
            )),
            Err(e) => {
                steps.push(failed_step("tcp", e));
                return report(steps, Vec::new());
            }
        },
    }

    // `/health`: любой ответ сервера доказывает, что TLS-рукопожатие прошло.
    let https = url.scheme() == "https";
    let health = match client.get("/health", JSON_LIMIT).await {
        Ok(r) => r,
        Err(e) => {
            let step = if https && e.code == ErrorCode::TlsError {
                "tls"
            } else {
                "health"
            };
            steps.push(failed_step(step, e));
            return report(steps, Vec::new());
        }
    };
    if https {
        steps.push(passed(
            "tls",
            ltext!(
                "messages.connectionTest.tlsOk",
                "TLS connection established, the certificate is trusted"
            ),
        ));
    }
    match health.status {
        200..=299 => steps.push(passed(
            "health",
            ltext!("messages.connectionTest.healthOk", "The server is healthy"),
        )),
        404 => steps.push(passed(
            "health",
            ltext!(
                "messages.connectionTest.healthMissing",
                "The server has no /health — skipped"
            ),
        )),
        401 | 403 => steps.push(passed(
            "health",
            ltext!(
                "messages.connectionTest.healthNeedsKey",
                "/health requires an API key — checked by the next step"
            ),
        )),
        status => {
            steps.push(failed_step(
                "health",
                app_err!(
                    ErrorCode::ConnectionFailed,
                    "errors.server.unhealthy",
                    "The server is not healthy (/health answered {{status}}): it may still be loading the model",
                    status = status
                ),
            ));
            return report(steps, Vec::new());
        }
    }

    let snapshot = match probe(&client, &s.metrics_path).await {
        Ok(p) => p,
        Err(e) if e.code == ErrorCode::AuthFailed && !has_key => {
            steps.push(failed_step(
                "auth",
                app_err!(
                    ErrorCode::AuthFailed,
                    "errors.server.keyRequired",
                    "The server requires an API key — enter it in the server settings"
                )
                .with_details(e.message),
            ));
            return report(steps, Vec::new());
        }
        Err(e) if e.code == ErrorCode::AuthFailed => {
            steps.push(failed_step("auth", e));
            return report(steps, Vec::new());
        }
        Err(e) if e.code == ErrorCode::NotFound => {
            steps.push(failed_step(
                "auth",
                app_err!(
                    ErrorCode::NotFound,
                    "errors.server.notOpenAi",
                    "The server has no /v1/models — is this the address of an OpenAI-compatible server?"
                ),
            ));
            return report(steps, Vec::new());
        }
        Err(e) => {
            steps.push(failed_step("auth", e));
            return report(steps, Vec::new());
        }
    };
    steps.push(passed(
        "auth",
        if has_key {
            ltext!(
                "messages.connectionTest.keyAccepted",
                "The API key is accepted"
            )
        } else {
            ltext!(
                "messages.connectionTest.noKeyNeeded",
                "The server answers without an API key"
            )
        },
    ));

    let facts = match analyze(s.kind, &snapshot) {
        Ok(f) => f,
        Err(e) => {
            steps.push(failed_step("kind", e));
            return report(steps, Vec::new());
        }
    };
    steps.push(passed("kind", kind_message(&facts)));
    if facts.models.is_empty() {
        steps.push(failed_step(
            "models",
            app_err!(
                ErrorCode::NotFound,
                "errors.server.noModels",
                "The server serves no models"
            ),
        ));
    } else {
        steps.push(passed("models", models_message(&facts)));
    }
    report(steps, facts.warnings)
}
````

<!-- block: patch t08-src step=8.2 -->
````diff
diff --git a/PORTED.md b/PORTED.md
index bcdb543..07bacc9 100644
--- a/PORTED.md
+++ b/PORTED.md
@@ -59,3 +59,4 @@
 | `src-tauri/src/storage/exchange.rs` | `src-tauri/src/storage/exchange.rs` | `468e08a` | адаптирован: ключи API серверов вместо токена и ключа сертификата, без exec-плагинов и доступа LLM, предупреждения о смене адреса сервера |
 | `src-tauri/src/commands/folders.rs` | `src-tauri/src/commands/folders.rs` | `468e08a` | адаптирован: удаление ключей серверов, которых нет в файле; без exec-плагинов и сессий |
 | `src-tauri/src/servers/errors.rs` | `src-tauri/src/k8s/errors.rs` | `468e08a` | адаптирован: только сетевые ошибки и коды HTTP сервера инференса, ключи `errors.server.*` |
+| `src-tauri/src/servers/connectivity.rs` | `src-tauri/src/k8s/connectivity.rs` | `468e08a` | адаптирован: шаги сервера инференса (здоровье, вход ключом API, тип и версия, модели), предупреждения, системный прокси |
diff --git a/src-tauri/src/servers/mod.rs b/src-tauri/src/servers/mod.rs
index 3aa0e6d..d3f6102 100644
--- a/src-tauri/src/servers/mod.rs
+++ b/src-tauri/src/servers/mod.rs
@@ -1,7 +1,8 @@
 //! Серверы инференса (спец. §4.3): HTTP-клиент из профиля, ошибки сервера, определение типа,
-//! версии и моделей, подключённые инсталляции.
+//! версии и моделей, подключённые инсталляции, пошаговая проверка подключения.
 
 pub mod client;
+pub mod connectivity;
 pub mod detect;
 pub mod errors;
 pub mod pool;
diff --git a/src-tauri/src/storage/profile.rs b/src-tauri/src/storage/profile.rs
index 09cbde3..f865da2 100644
--- a/src-tauri/src/storage/profile.rs
+++ b/src-tauri/src/storage/profile.rs
@@ -288,7 +288,8 @@ pub fn parse_base_url(s: &ServerConfig) -> AppResult<url::Url> {
     Ok(url)
 }
 
-fn validate_server(s: &ServerConfig) -> AppResult<()> {
+/// Проверка одного сервера: и при сохранении профиля, и при проверке подключения из формы.
+pub fn validate_server(s: &ServerConfig) -> AppResult<()> {
     if !is_valid_server_id(&s.id) {
         return Err(invalid(ltext!(
             "errors.installations.badServerId",
````

- [ ] **Step 3: Проверки.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib`  
Expected: PASS — `test result: ok. 155 passed; 0 failed`

Run: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`  
Expected: PASS — без предупреждений

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml --check`  
Expected: PASS — без изменений

- [ ] **Step 4: Commit.**

```bash
git add \
  PORTED.md \
  src-tauri/src/servers/connectivity.rs \
  src-tauri/src/servers/mod.rs \
  src-tauri/src/storage/profile.rs
git commit -s -m "feat(servers): step-by-step server check — configuration, DNS/TCP, TLS, /health, API key, kind and version, models" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 9: Команды Tauri: инсталляции, проверка сервера, подключение, «О программе»

**Files:**
- Create (по Kubernetes-версии): `src-tauri/src/commands/app.rs` ← `src-tauri/src/commands/app.rs`, `src-tauri/src/commands/installations.rs` ← `src-tauri/src/commands/connections.rs`
- Modify: `PORTED.md`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/state.rs`

**Interfaces:**
- Consumes: задачи 4, 7, 8.
- Produces: команды `list_installations`, `save_installation(input { profile, secrets { apiKeys: { id сервера: значение } } })`, `delete_installation`, `test_server_connection(input, serverId)`, `connect_installation(installationId, fresh)`, `disconnect_installation`, `get_app_info`; `AppState::delete_installation_full`.

Сохранение согласовано с keychain: ключи — после записи профиля, сначала `Set`, потом `Delete`; отказ keychain откатывает профиль. Ключи серверов, убранных из профиля, удаляются. Проверка сервера берёт ключ из формы, а если его там нет — из keychain. Сохранение и удаление забывают подключение. «О программе» — без обновлений: они придут в I2 вместе с релизным процессом.

- [ ] **Step 1: Тесты.**

<!-- block: file t09-src-tauri-src-commands-installations-rs-tests step=9.1 path=src-tauri/src/commands/installations.rs part=tests -->
````rust
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::error::{AppError, ErrorCode};
    use crate::secrets::{MemoryStore, SecretStore};
    use crate::storage::profile::{CaConfig, sample_profile, sample_server};

    fn state(dir: &std::path::Path) -> AppState {
        AppState::new(dir, Arc::new(MemoryStore::default())).unwrap()
    }

    fn input(profile: InstallationProfile, keys: &[(&str, &str)]) -> InstallationInput {
        InstallationInput {
            profile,
            secrets: SecretsInput {
                api_keys: keys
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            },
        }
    }

    fn key(st: &AppState, id: &str, server: &str) -> Option<String> {
        st.secrets.get(&secret_key(id, server, API_KEY)).unwrap()
    }

    #[test]
    fn secret_ops_matrix() {
        let mut prev = sample_profile("a");
        prev.servers.push(sample_server("gone"));
        let mut saved = sample_profile("a");
        saved.servers.push(sample_server("s2"));
        saved.servers.push(sample_server("s3"));
        let s = SecretsInput {
            api_keys: [("s2", " sk-2 "), ("s3", " ")]
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        };
        assert_eq!(
            secret_ops(&saved, Some(&prev), &s),
            [
                ("s1".to_string(), SecretOp::Keep),
                ("s2".to_string(), SecretOp::Set("sk-2".into())),
                ("s3".to_string(), SecretOp::Delete),
                ("gone".to_string(), SecretOp::Delete),
            ]
        );
        assert_eq!(format!("{s:?}"), r#"{"s2": "***", "s3": "***"}"#);
    }

    #[test]
    fn new_installation_gets_an_id_and_keys() {
        let dir = tempfile::tempdir().unwrap();
        let st = state(dir.path());
        let mut p = sample_profile("");
        p.id.clear();
        let saved = save_installation_blocking(&st, input(p, &[("s1", "sk-1")])).unwrap();
        assert!(!saved.id.is_empty());
        assert_eq!(key(&st, &saved.id, "s1").as_deref(), Some("sk-1"));
        let raw = std::fs::read_to_string(dir.path().join("installations.json")).unwrap();
        assert!(!raw.contains("sk-1"));
    }

    #[test]
    fn keys_are_kept_replaced_and_removed_with_their_servers() {
        let dir = tempfile::tempdir().unwrap();
        let st = state(dir.path());
        let mut two = sample_profile("a");
        two.servers.push(sample_server("s2"));
        save_installation_blocking(&st, input(two, &[("s1", "sk-1"), ("s2", "sk-2")])).unwrap();
        // Без ключей в форме — сохранённые остаются; s2 удалён из профиля — его ключ тоже.
        save_installation_blocking(&st, input(sample_profile("a"), &[])).unwrap();
        assert_eq!(key(&st, "a", "s1").as_deref(), Some("sk-1"));
        assert_eq!(key(&st, "a", "s2"), None);
        save_installation_blocking(&st, input(sample_profile("a"), &[("s1", "")])).unwrap();
        assert_eq!(key(&st, "a", "s1"), None);
    }

    /// Keychain, который отказывает в записи.
    struct ReadOnlyKeychain(MemoryStore);

    impl SecretStore for ReadOnlyKeychain {
        fn get(&self, key: &str) -> AppResult<Option<String>> {
            self.0.get(key)
        }
        fn set(&self, _: &str, _: &str) -> AppResult<()> {
            Err(AppError::internal_with("denied"))
        }
        fn delete(&self, _: &str) -> AppResult<()> {
            Err(AppError::internal_with("denied"))
        }
    }

    #[test]
    fn keychain_failure_rolls_the_profile_back() {
        let dir = tempfile::tempdir().unwrap();
        let st = AppState::new(
            dir.path(),
            Arc::new(ReadOnlyKeychain(MemoryStore::default())),
        )
        .unwrap();
        let e = save_installation_blocking(&st, input(sample_profile("a"), &[("s1", "sk")]));
        assert!(e.is_err());
        assert!(st.installations.get("a").is_err(), "новый профиль удалён");
        st.installations.upsert(sample_profile("b")).unwrap();
        let mut renamed = sample_profile("b");
        renamed.name = "renamed".into();
        assert!(save_installation_blocking(&st, input(renamed, &[("s1", "sk")])).is_err());
        assert_eq!(st.installations.get("b").unwrap().name, "inst-b");
    }

    #[test]
    fn policy_forbids_saving_insecure_servers() {
        let dir = tempfile::tempdir().unwrap();
        let mut policy = crate::policy::Policy::default();
        policy.status.allow_insecure_tls = false;
        let st = state(dir.path()).with_policy(policy);
        let mut p = sample_profile("a");
        p.servers[0].ca = CaConfig::Insecure;
        let e = save_installation_blocking(&st, input(p, &[])).unwrap_err();
        assert_eq!(
            (e.code, e.key.as_str()),
            (
                ErrorCode::InvalidInput,
                "errors.installations.insecureForbidden"
            )
        );
        assert!(st.installations.list().is_empty());
    }

    #[test]
    fn saving_and_deleting_forget_the_connection() {
        let dir = tempfile::tempdir().unwrap();
        let st = state(dir.path());
        let fake = || {
            Arc::new(crate::servers::pool::Connected {
                info: InstallationInfo {
                    installation_id: "a".into(),
                    servers: Vec::new(),
                },
                clients: Default::default(),
            })
        };
        save_installation_blocking(&st, input(sample_profile("a"), &[("s1", "sk")])).unwrap();
        st.pool.insert("a", fake());
        save_installation_blocking(&st, input(sample_profile("a"), &[])).unwrap();
        assert!(st.pool.get("a").is_none());
        st.pool.insert("a", fake());
        st.delete_installation_full("a").unwrap();
        assert!(st.pool.get("a").is_none());
        assert!(st.installations.get("a").is_err());
        assert_eq!(key(&st, "a", "s1"), None);
    }
}
````

- [ ] **Step 2: Реализация.**

<!-- block: file t09-src-tauri-src-commands-app-rs step=9.2 path=src-tauri/src/commands/app.rs -->
````rust
use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::error::AppResult;

/// Версия и каталоги для «О программе».
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub commit: String,
    pub os: String,
    pub arch: String,
    pub data_dir: String,
    pub log_dir: String,
}

fn dir(app: &AppHandle, which: &str) -> String {
    let p = match which {
        "data" => app.path().app_data_dir(),
        _ => app.path().app_log_dir(),
    };
    p.map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "—".into())
}

#[tauri::command]
pub fn get_app_info(app: AppHandle) -> AppResult<AppInfo> {
    Ok(AppInfo {
        name: crate::product::PRODUCT_NAME.into(),
        version: env!("CARGO_PKG_VERSION").into(),
        commit: env!("GIT_COMMIT").into(),
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        data_dir: dir(&app, "data"),
        log_dir: dir(&app, "log"),
    })
}
````

<!-- block: file t09-src-tauri-src-commands-installations-rs-impl step=9.2 path=src-tauri/src/commands/installations.rs part=impl -->
````rust
use std::collections::BTreeMap;

use serde::Deserialize;
use tauri::State;

use super::blocking;
use crate::error::AppResult;
use crate::secrets::{API_KEY, secret_key};
use crate::servers::client::{Timeouts, ensure_allowed};
use crate::servers::connectivity::{TestReport, test_server};
use crate::servers::pool::InstallationInfo;
use crate::state::AppState;
use crate::storage::profile::InstallationProfile;

/// Ключи API из формы: id сервера → значение. Сервера нет в списке — оставить сохранённый,
/// `""` — удалить, иное — сохранить. `Debug` печатает вместо значений `***`.
#[derive(Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretsInput {
    #[serde(default)]
    pub api_keys: BTreeMap<String, String>,
}

/// Ключи не печатаются: структура может попасть в лог или сообщение об ошибке.
impl std::fmt::Debug for SecretsInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(self.api_keys.keys().map(|k| (k, "***")))
            .finish()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallationInput {
    pub profile: InstallationProfile,
    #[serde(default)]
    pub secrets: SecretsInput,
}

/// Что сделать с ключом при сохранении.
#[derive(Debug, PartialEq, Eq)]
enum SecretOp {
    Keep,
    Set(String),
    Delete,
}

/// Ключи серверов профиля — по данным формы; ключи серверов, которых больше нет, удаляются.
fn secret_ops(
    saved: &InstallationProfile,
    previous: Option<&InstallationProfile>,
    s: &SecretsInput,
) -> Vec<(String, SecretOp)> {
    let mut ops: Vec<(String, SecretOp)> = saved
        .servers
        .iter()
        .map(|server| {
            let op = match s.api_keys.get(&server.id).map(|v| v.trim()) {
                None => SecretOp::Keep,
                Some("") => SecretOp::Delete,
                Some(v) => SecretOp::Set(v.to_string()),
            };
            (server.id.clone(), op)
        })
        .collect();
    for old in previous.map(|p| p.servers.as_slice()).unwrap_or_default() {
        if saved.server(&old.id).is_none() {
            ops.push((old.id.clone(), SecretOp::Delete));
        }
    }
    ops
}

#[tauri::command]
pub fn list_installations(state: State<'_, AppState>) -> AppResult<Vec<InstallationProfile>> {
    Ok(state.installations.list())
}

#[tauri::command]
pub async fn save_installation(
    state: State<'_, AppState>,
    input: InstallationInput,
) -> AppResult<InstallationProfile> {
    let st = state.inner().clone();
    blocking(move || save_installation_blocking(&st, input)).await
}

/// Сохраняет профиль и ключи согласованно: если keychain не принял изменение, профиль
/// возвращается к прежнему состоянию (или удаляется, если он новый).
pub fn save_installation_blocking(
    st: &AppState,
    input: InstallationInput,
) -> AppResult<InstallationProfile> {
    let InstallationInput {
        mut profile,
        secrets,
    } = input;
    for s in &profile.servers {
        ensure_allowed(s, &st.policy.status)?;
    }
    if profile.id.is_empty() {
        profile.id = uuid::Uuid::new_v4().to_string();
    }
    let previous = st.installations.get(&profile.id).ok();
    // upsert проверяет профиль — ключи трогаем только после успешного сохранения.
    let saved = st.installations.upsert(profile)?;
    // Сначала `Set`, потом `Delete`: если keychain не примет новый ключ, удалённые ещё на
    // месте, и откат профиля не оставляет серверы без ключей.
    let mut ops = secret_ops(&saved, previous.as_ref(), &secrets);
    ops.sort_by_key(|(_, op)| *op == SecretOp::Delete);
    let result = ops.into_iter().try_for_each(|(server, op)| {
        let key = secret_key(&saved.id, &server, API_KEY);
        match op {
            SecretOp::Keep => Ok(()),
            SecretOp::Set(v) => st.secrets.set(&key, &v),
            SecretOp::Delete => st.secrets.delete(&key),
        }
    });
    if result.is_err() {
        let rollback = match previous {
            Some(p) => st.installations.upsert(p).map(|_| ()),
            None => st.installations.delete(&saved.id),
        };
        if let Err(r) = rollback {
            tracing::error!("failed to roll back profile {}: {r}", saved.id);
        }
    }
    // Клиенты работают со старыми адресами и ключами: следующий вызов подключится заново.
    st.evict_installation(&saved.id);
    result.map(|()| saved)
}

#[tauri::command]
pub async fn delete_installation(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let st = state.inner().clone();
    blocking(move || st.delete_installation_full(&id)).await
}

/// Проверяет сервер и несохранённого профиля: ключ из формы накладывается поверх keychain
/// (с той же семантикой, что и при сохранении).
#[tauri::command]
pub async fn test_server_connection(
    state: State<'_, AppState>,
    input: InstallationInput,
    server_id: String,
) -> AppResult<TestReport> {
    let st = state.inner().clone();
    let profile = input.profile.clone();
    let server = profile.server(&server_id).cloned().ok_or_else(|| {
        app_err!(
            crate::error::ErrorCode::NotFound,
            "errors.installations.serverNotFound",
            "Server {{id}} is not in the installation",
            id = server_id
        )
    })?;
    let typed = input.secrets.api_keys.get(&server_id).cloned();
    let key = match typed {
        Some(v) => Some(v),
        None => {
            let key = secret_key(&profile.id, &server_id, API_KEY);
            blocking(move || st.secrets.get(&key)).await?
        }
    };
    Ok(test_server(&server, Timeouts::from(&profile), key, &state.policy.status).await)
}

/// Подключается (или берёт подключение из пула) и возвращает сведения о серверах.
/// `fresh` — подключиться заново («Обновить», после смены ключа).
#[tauri::command]
pub async fn connect_installation(
    state: State<'_, AppState>,
    installation_id: String,
    fresh: bool,
) -> AppResult<InstallationInfo> {
    let c = state.connected(&installation_id, fresh).await?;
    Ok(c.info.clone())
}

#[tauri::command]
pub fn disconnect_installation(
    state: State<'_, AppState>,
    installation_id: String,
) -> AppResult<()> {
    state.evict_installation(&installation_id);
    Ok(())
}
````

<!-- block: patch t09-src step=9.2 -->
````diff
diff --git a/PORTED.md b/PORTED.md
index 07bacc9..4d3b6df 100644
--- a/PORTED.md
+++ b/PORTED.md
@@ -60,3 +60,5 @@
 | `src-tauri/src/commands/folders.rs` | `src-tauri/src/commands/folders.rs` | `468e08a` | адаптирован: удаление ключей серверов, которых нет в файле; без exec-плагинов и сессий |
 | `src-tauri/src/servers/errors.rs` | `src-tauri/src/k8s/errors.rs` | `468e08a` | адаптирован: только сетевые ошибки и коды HTTP сервера инференса, ключи `errors.server.*` |
 | `src-tauri/src/servers/connectivity.rs` | `src-tauri/src/k8s/connectivity.rs` | `468e08a` | адаптирован: шаги сервера инференса (здоровье, вход ключом API, тип и версия, модели), предупреждения, системный прокси |
+| `src-tauri/src/commands/app.rs` | `src-tauri/src/commands/app.rs` | `468e08a` | адаптирован: только сведения «О программе», обновления — в I2 |
+| `src-tauri/src/commands/installations.rs` | `src-tauri/src/commands/connections.rs` | `468e08a` | адаптирован: ключи API серверов вместо токена и ключа сертификата, проверка сервера, подключение инсталляции |
diff --git a/src-tauri/src/commands/mod.rs b/src-tauri/src/commands/mod.rs
index 3bf6b40..e709519 100644
--- a/src-tauri/src/commands/mod.rs
+++ b/src-tauri/src/commands/mod.rs
@@ -1,4 +1,6 @@
+pub mod app;
 pub mod folders;
+pub mod installations;
 pub mod settings;
 
 use crate::error::{AppResult, ErrorCode};
diff --git a/src-tauri/src/lib.rs b/src-tauri/src/lib.rs
index 552b391..870a16c 100644
--- a/src-tauri/src/lib.rs
+++ b/src-tauri/src/lib.rs
@@ -41,6 +41,13 @@ pub fn run() {
             Ok(())
         })
         .invoke_handler(tauri::generate_handler![
+            commands::app::get_app_info,
+            commands::installations::list_installations,
+            commands::installations::save_installation,
+            commands::installations::delete_installation,
+            commands::installations::test_server_connection,
+            commands::installations::connect_installation,
+            commands::installations::disconnect_installation,
             commands::folders::list_folders,
             commands::folders::create_folder,
             commands::folders::rename_folder,
diff --git a/src-tauri/src/state.rs b/src-tauri/src/state.rs
index c7185c8..dd09592 100644
--- a/src-tauri/src/state.rs
+++ b/src-tauri/src/state.rs
@@ -67,6 +67,20 @@ impl AppState {
     pub fn evict_installation(&self, id: &str) {
         self.pool.evict(id);
     }
+
+    /// Удаляет инсталляцию: профиль, ключи API её серверов и подключение.
+    pub fn delete_installation_full(&self, id: &str) -> AppResult<()> {
+        let profile = self.installations.get(id)?;
+        self.installations.delete(id)?;
+        for server in &profile.servers {
+            for field in crate::secrets::SERVER_SECRET_FIELDS {
+                self.secrets
+                    .delete(&crate::secrets::secret_key(id, &server.id, field))?;
+            }
+        }
+        self.evict_installation(id);
+        Ok(())
+    }
 }
 
 #[cfg(test)]
````

- [ ] **Step 3: Проверки.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib`  
Expected: PASS — `test result: ok. 161 passed; 0 failed`

Run: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`  
Expected: PASS — без предупреждений

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml --check`  
Expected: PASS — без изменений

- [ ] **Step 4: Commit.**

```bash
git add \
  PORTED.md \
  src-tauri/src/commands/app.rs \
  src-tauri/src/commands/installations.rs \
  src-tauri/src/commands/mod.rs \
  src-tauri/src/lib.rs \
  src-tauri/src/state.rs
git commit -s -m "feat(commands): Tauri commands for installations, server check, connection and app info" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 10: Основа фронтенда: i18n, словари en и ru, типы IPC и API-клиент, общие компоненты

**Files:**
- Port: `scripts/changelog.mjs`, `scripts/changelog.test.mjs`, `scripts/i18n-check.mjs` (с правками), `scripts/i18n-check.test.mjs`, `scripts/merge-locales.mjs`, `scripts/merge-locales.test.mjs`, `src/components/common/EnvBadge.tsx`, `src/components/common/ErrorBanner.test.tsx` (с правками), `src/components/common/ErrorBanner.tsx`, `src/components/common/Hint.test.tsx`, `src/components/common/Hint.tsx`, `src/i18n/documentLang.test.ts`, `src/i18n/index.ts`, `src/i18n/languages.ts` (с правками), `src/i18n/locales.test.ts`, `src/i18n/text.test.ts`, `src/i18n/text.ts`, `src/lib/activate.test.tsx`, `src/lib/activate.ts`, `src/lib/env.ts`, `src/lib/focusTrap.ts`, `src/lib/format.test.ts`, `src/lib/format.ts`, `src/lib/formatAgo.ts`, `src/lib/links.ts` (с правками), `src/lib/plainInputs.test.ts`, `src/lib/plainInputs.ts`, `src/lib/useDebouncedValue.test.ts`, `src/lib/useDebouncedValue.ts`, `src/lib/useLanguageSync.ts`, `src/test/render.tsx`
- Create: `CHANGELOG.md`, `docs/changelog/ru.md`, `docs/guide/en/.gitkeep`, `docs/guide/ru/.gitkeep`, `docs/i18n/same-as-en.json`, `src/api/client.test.ts`, `src/api/client.ts`, `src/api/types.ts`, `src/locales/en.json`, `src/locales/ru.json`
- Modify: `PORTED.md`, `package.json`, `src/App.test.tsx`, `src/App.tsx`, `src/main.tsx`, `src/test/setup.ts`

**Interfaces:**
- Consumes: команды задач 3, 4, 9.
- Produces: `src/api/types.ts` (`ErrorCode`, `LocalizedText`, `AppError`, `Environment`, `ServerKind`, `CaConfig`, `ProxyConfig`, `ServerConfig`, `InstallationProfile`, `SecretsInput { apiKeys }`, `InstallationFolder`, `ExportSummary`, `PreviewItem`, `ImportPreview`, `TreeItemRef`, `TestStepName`, `TestStep`, `TestReport`, `DetectedKind`, `Model`, `ServerFacts`, `ServerState`, `InstallationInfo`, `PolicyStatus`, `AppInfo`, `LanguageSettings`), `src/api/client.ts` — `api.{getAppInfo, getLanguageSettings, setLanguage, getPolicyStatus, listInstallations, saveInstallation(profile, secrets), deleteInstallation, testServer(profile, serverId, secrets), connectInstallation(id, fresh), disconnectInstallation, listFolders, createFolder, renameFolder, deleteFolder, moveItems, exportInstallations, previewImport, applyImport}`, `toAppError`; `pnpm i18n:check`; `node scripts/merge-locales.mjs < фрагмент.json`.

Переносятся i18n (`index.ts`, `text.ts`), `lib/*`, тестовые помощники, `ErrorBanner`, `Hint`, `EnvBadge`, проверка локализации и `merge-locales`. Глав руководства пока нет (`GUIDE_CHAPTERS = []`), списки изменений — с одним заголовком. Словари создаются фрагментом: все 88 ключей Rust из задач 2–9 (английский — ровно шаблоны Rust) и общие строки.

- [ ] **Step 1: Перенос из Kubernetes-версии.**

Файлы копируются с коммита `468e08a` репозитория `~/projects/glafira-openshift` (`$KUBERNETES_REPO`) и правятся патчами — против перенесённого файла. Строки о них в `PORTED.md` добавляет шаг реализации.

<!-- block: shell t10-port step=10.1 -->
````bash
scripts/port.sh scripts/i18n-check.mjs scripts/i18n-check.mjs
scripts/port.sh scripts/i18n-check.test.mjs scripts/i18n-check.test.mjs
scripts/port.sh scripts/changelog.mjs scripts/changelog.mjs
scripts/port.sh scripts/changelog.test.mjs scripts/changelog.test.mjs
scripts/port.sh scripts/merge-locales.mjs scripts/merge-locales.mjs
scripts/port.sh scripts/merge-locales.test.mjs scripts/merge-locales.test.mjs
scripts/port.sh src/i18n/index.ts src/i18n/index.ts
scripts/port.sh src/i18n/text.ts src/i18n/text.ts
scripts/port.sh src/i18n/languages.ts src/i18n/languages.ts
scripts/port.sh src/i18n/documentLang.test.ts src/i18n/documentLang.test.ts
scripts/port.sh src/i18n/locales.test.ts src/i18n/locales.test.ts
scripts/port.sh src/i18n/text.test.ts src/i18n/text.test.ts
scripts/port.sh src/lib/activate.ts src/lib/activate.ts
scripts/port.sh src/lib/activate.test.tsx src/lib/activate.test.tsx
scripts/port.sh src/lib/env.ts src/lib/env.ts
scripts/port.sh src/lib/focusTrap.ts src/lib/focusTrap.ts
scripts/port.sh src/lib/format.ts src/lib/format.ts
scripts/port.sh src/lib/format.test.ts src/lib/format.test.ts
scripts/port.sh src/lib/formatAgo.ts src/lib/formatAgo.ts
scripts/port.sh src/lib/links.ts src/lib/links.ts
scripts/port.sh src/lib/plainInputs.ts src/lib/plainInputs.ts
scripts/port.sh src/lib/plainInputs.test.ts src/lib/plainInputs.test.ts
scripts/port.sh src/lib/useDebouncedValue.ts src/lib/useDebouncedValue.ts
scripts/port.sh src/lib/useDebouncedValue.test.ts src/lib/useDebouncedValue.test.ts
scripts/port.sh src/lib/useLanguageSync.ts src/lib/useLanguageSync.ts
scripts/port.sh src/components/common/EnvBadge.tsx src/components/common/EnvBadge.tsx
scripts/port.sh src/components/common/ErrorBanner.tsx src/components/common/ErrorBanner.tsx
scripts/port.sh src/components/common/ErrorBanner.test.tsx src/components/common/ErrorBanner.test.tsx
scripts/port.sh src/components/common/Hint.tsx src/components/common/Hint.tsx
scripts/port.sh src/components/common/Hint.test.tsx src/components/common/Hint.test.tsx
scripts/port.sh src/test/render.tsx src/test/render.tsx
````

<!-- block: patch t10-port-scripts-i18n-check-mjs step=10.1 -->
````diff
diff --git a/scripts/i18n-check.mjs b/scripts/i18n-check.mjs
--- a/scripts/i18n-check.mjs
+++ b/scripts/i18n-check.mjs
@@ -562,7 +562,7 @@ export function findRustCyrillic(files) {
 }
 
 /** Главы руководства: тот же список, что `GUIDE_CHAPTERS` в `src/guide/chapters.ts`. */
-export const GUIDE_CHAPTERS = ["getting-started", "connections", "projects-workloads", "pods-logs", "health-check", "llm-mcp", "updates"];
+export const GUIDE_CHAPTERS = [];
 
 const KEBAB = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
 
````

<!-- block: patch t10-port-src-i18n-languages-ts step=10.1 -->
````diff
diff --git a/src/i18n/languages.ts b/src/i18n/languages.ts
--- a/src/i18n/languages.ts
+++ b/src/i18n/languages.ts
@@ -1,4 +1,4 @@
-/** Языки, для которых есть словарь `src/locales/<код>.json`. Остальные 10 языков добавляет этап O6. */
+/** Языки, для которых есть словарь `src/locales/<код>.json`. Остальные 10 языков семейства добавляет этап I7. */
 export type LanguageCode = "en" | "ru";
 
 export const LANGUAGES: { code: LanguageCode; nativeName: string }[] = [
````

<!-- block: patch t10-port-src-lib-links-ts step=10.1 -->
````diff
diff --git a/src/lib/links.ts b/src/lib/links.ts
--- a/src/lib/links.ts
+++ b/src/lib/links.ts
@@ -1,9 +1,9 @@
 import { openUrl } from "@tauri-apps/plugin-opener";
 
 /** Страница поддержки проекта — один адрес на все входы: низ левой панели, «О программе», меню Help. */
-export const SUPPORT_URL = "https://github.com/vvnovg/glafira-kubernetes-releases/blob/main/SUPPORT.md";
+export const SUPPORT_URL = "https://github.com/vvnovg/glafira-inference-releases/blob/main/SUPPORT.md";
 /** Текст лицензии Apache 2.0 в репозитории исходного кода. */
-export const LICENSE_URL = "https://github.com/vvnovg/glafira-kubernetes/blob/main/LICENSE";
+export const LICENSE_URL = "https://github.com/vvnovg/glafira-inference/blob/main/LICENSE";
 
 /** Открывает страницу поддержки в системном браузере. */
 export function openSupport(): void {
````

<!-- block: patch t10-port-src-components-common-errorbanner-test-tsx step=10.1 -->
````diff
diff --git a/src/components/common/ErrorBanner.test.tsx b/src/components/common/ErrorBanner.test.tsx
--- a/src/components/common/ErrorBanner.test.tsx
+++ b/src/components/common/ErrorBanner.test.tsx
@@ -7,15 +7,15 @@ describe("ErrorBanner", () => {
     render(
       <ErrorBanner
         error={{
-          code: "FORBIDDEN",
-          key: "errors.k8s.forbiddenIn",
-          params: { verb: "list", resource: "pods", namespace: "team-a" },
-          message: "Not allowed: list pods in project “team-a”",
+          code: "AUTH_FAILED",
+          key: "errors.server.unauthorized",
+          params: { status: "401" },
+          message: "The server rejected the request (401) — check the API key",
           retryable: false,
         }}
       />,
     );
-    expect(screen.getByText("Нет прав: list pods в проекте «team-a»")).toBeInTheDocument();
+    expect(screen.getByText("Сервер отклонил запрос (401) — проверьте ключ API")).toBeInTheDocument();
   });
 
   it("errors.generic показывает исходный текст как есть", () => {
````

- [ ] **Step 2: Тесты.**

<!-- block: file t10-src-api-client-test-ts step=10.2 path=src/api/client.test.ts -->
````ts
import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

import { api, toAppError } from "./client";
import type { AppError, InstallationProfile } from "./types";

beforeEach(() => {
  invoke.mockReset();
});

describe("api client", () => {
  it("saveInstallation оборачивает профиль и ключи в input, ключи по умолчанию пустые", async () => {
    invoke.mockResolvedValue({});
    const profile = { id: "a" } as InstallationProfile;
    await api.saveInstallation(profile, { apiKeys: { s1: "sk" } });
    expect(invoke).toHaveBeenCalledWith("save_installation", {
      input: { profile, secrets: { apiKeys: { s1: "sk" } } },
    });
    await api.saveInstallation(profile);
    expect(invoke).toHaveBeenLastCalledWith("save_installation", { input: { profile, secrets: {} } });
  });

  it("testServer передаёт профиль формы, ключи и id сервера", async () => {
    invoke.mockResolvedValue({ ok: true, steps: [], warnings: [] });
    const profile = { id: "", name: "x" } as InstallationProfile;
    await api.testServer(profile, "s1", { apiKeys: { s1: "sk" } });
    expect(invoke).toHaveBeenCalledWith("test_server_connection", {
      input: { profile, secrets: { apiKeys: { s1: "sk" } } },
      serverId: "s1",
    });
  });

  it("connectInstallation передаёт аргументы в camelCase, fresh по умолчанию false", async () => {
    invoke.mockResolvedValue({});
    await api.connectInstallation("i1");
    expect(invoke).toHaveBeenCalledWith("connect_installation", { installationId: "i1", fresh: false });
    await api.connectInstallation("i1", true);
    expect(invoke).toHaveBeenLastCalledWith("connect_installation", { installationId: "i1", fresh: true });
  });

  it("ошибка из Rust приходит как есть, прочие — как INTERNAL", async () => {
    const e: AppError = { code: "TIMEOUT", key: "errors.server.timeout", message: "late", retryable: true };
    invoke.mockRejectedValue(e);
    await expect(api.listInstallations()).rejects.toEqual(e);
    invoke.mockRejectedValue(new Error("boom"));
    await expect(api.listInstallations()).rejects.toEqual({ code: "INTERNAL", message: "boom", retryable: false });
    expect(toAppError("x")).toEqual({ code: "INTERNAL", message: "x", retryable: false });
  });
});
````

<!-- block: patch t10-tests step=10.2 -->
````diff
diff --git a/src/App.test.tsx b/src/App.test.tsx
index 70a970e..899042c 100644
--- a/src/App.test.tsx
+++ b/src/App.test.tsx
@@ -1,4 +1,11 @@
 import { render, screen } from "@testing-library/react";
+import { vi } from "vitest";
+
+vi.mock("./api/client", () => ({
+  api: { getLanguageSettings: vi.fn(() => new Promise(() => {})) },
+  toAppError: (e: unknown) => e,
+}));
+
 import App from "./App";
 
 test("показывает имя продукта", () => {
diff --git a/src/test/setup.ts b/src/test/setup.ts
index f149f27..1483b57 100644
--- a/src/test/setup.ts
+++ b/src/test/setup.ts
@@ -1 +1,4 @@
 import "@testing-library/jest-dom/vitest";
+import { i18n } from "../i18n";
+
+await i18n.changeLanguage("ru");
````

- [ ] **Step 3: Реализация.**

<!-- block: file t10-changelog-md step=10.3 path=CHANGELOG.md -->
````markdown
# Changelog

Versions in the other interface languages are in [docs/changelog](docs/changelog/). The section of a version is shown in About as the update notes, in the interface language.
````

<!-- block: file t10-docs-changelog-ru-md step=10.3 path=docs/changelog/ru.md -->
````markdown
# Список изменений

Русская версия [CHANGELOG.md](../../CHANGELOG.md). Раздел версии показывается в «О программе» как заметки к обновлению.
````

<!-- block: shell t10-docs-guide-en-gitkeep step=10.3 -->
````bash
mkdir -p docs/guide/en
: > docs/guide/en/.gitkeep
````

<!-- block: shell t10-docs-guide-ru-gitkeep step=10.3 -->
````bash
mkdir -p docs/guide/ru
: > docs/guide/ru/.gitkeep
````

<!-- block: file t10-docs-i18n-same-as-en-json step=10.3 path=docs/i18n/same-as-en.json -->
````json
{
  "ru": [
    "errors.generic",
    "messages.connectionTest.kind",
    "messages.connectionTest.kindVersion"
  ]
}
````

<!-- block: file t10-src-api-client-ts step=10.3 path=src/api/client.ts -->
````ts
import { invoke } from "@tauri-apps/api/core";
import type {
  AppError,
  AppInfo,
  ExportSummary,
  ImportPreview,
  InstallationFolder,
  InstallationInfo,
  InstallationProfile,
  LanguageSettings,
  PolicyStatus,
  SecretsInput,
  TestReport,
  TreeItemRef,
} from "./types";

export function isAppError(e: unknown): e is AppError {
  return typeof e === "object" && e !== null && "code" in e && "message" in e;
}

export function toAppError(e: unknown): AppError {
  // Ошибка из Rust приходит целиком: key/params/details сохраняются как есть.
  if (isAppError(e)) return e;
  if (e instanceof Error) return { code: "INTERNAL", message: e.message, retryable: false };
  return { code: "INTERNAL", message: String(e), retryable: false };
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw toAppError(e);
  }
}

export const api = {
  getAppInfo: () => call<AppInfo>("get_app_info"),
  getLanguageSettings: () => call<LanguageSettings>("get_language_settings"),
  setLanguage: (lang: string | null) => call<LanguageSettings>("set_language", { lang }),
  getPolicyStatus: () => call<PolicyStatus>("get_policy_status"),

  listInstallations: () => call<InstallationProfile[]>("list_installations"),
  saveInstallation: (profile: InstallationProfile, secrets: SecretsInput = {}) =>
    call<InstallationProfile>("save_installation", { input: { profile, secrets } }),
  deleteInstallation: (id: string) => call<void>("delete_installation", { id }),
  /** Проверка сервера `serverId` несохранённого профиля: ключ из формы — поверх keychain. */
  testServer: (profile: InstallationProfile, serverId: string, secrets: SecretsInput = {}) =>
    call<TestReport>("test_server_connection", { input: { profile, secrets }, serverId }),
  /** `fresh` — подключиться заново, а не взять подключение из пула. */
  connectInstallation: (installationId: string, fresh = false) =>
    call<InstallationInfo>("connect_installation", { installationId, fresh }),
  disconnectInstallation: (installationId: string) => call<void>("disconnect_installation", { installationId }),

  listFolders: () => call<InstallationFolder[]>("list_folders"),
  createFolder: (name: string, parentId: string | null) =>
    call<InstallationFolder>("create_folder", { name, parentId }),
  renameFolder: (id: string, name: string) => call<InstallationFolder>("rename_folder", { id, name }),
  deleteFolder: (id: string) => call<void>("delete_folder", { id }),
  moveItems: (items: TreeItemRef[], parentId: string | null, before: TreeItemRef | null) =>
    call<void>("move_items", { items, parentId, before }),
  exportInstallations: (path: string, folderId: string | null) =>
    call<ExportSummary>("export_installations", { path, folderId }),
  previewImport: (path: string, targetFolderId: string | null, updateExisting: boolean) =>
    call<ImportPreview>("preview_import", { path, targetFolderId, updateExisting }),
  applyImport: (path: string, targetFolderId: string | null, updateExisting: boolean) =>
    call<ImportPreview>("apply_import", { path, targetFolderId, updateExisting }),
};
````

<!-- block: file t10-src-api-types-ts step=10.3 path=src/api/types.ts -->
````ts
// Типы IPC-контракта с Rust (спец. §4.2). Поля — camelCase, как сериализует serde.

export type ErrorCode =
  | "CONNECTION_FAILED"
  | "TLS_ERROR"
  | "AUTH_FAILED"
  | "SESSION_EXPIRED"
  | "FORBIDDEN"
  | "NOT_FOUND"
  | "CONFLICT"
  | "TIMEOUT"
  | "INVALID_INPUT"
  | "READ_ONLY"
  | "CONFIRMATION_REQUIRED"
  | "INTERNAL";

/** Текст из Rust: ключ словаря, параметры и английский текст (запасной вариант). */
export interface LocalizedText {
  key: string;
  params: Record<string, string>;
  text: string;
}

export interface AppError {
  code: ErrorCode;
  /** Ключ словаря; у технического текста без перевода — `errors.generic` с `params.message`. */
  key?: string;
  params?: Record<string, string>;
  /** Английский текст (запасной вариант перевода). */
  message: string;
  details?: string;
  retryable: boolean;
}

export type Environment = "local" | "dev" | "test" | "stage" | "prod";

/** Тип сервера в профиле: `auto` — определить по ответам сервера. */
export type ServerKind = "auto" | "vllm" | "sglang" | "openai";

export type CaConfig = { mode: "system" } | { mode: "pem"; pem: string } | { mode: "insecure" };

export type ProxyConfig = { mode: "system" } | { mode: "none" } | { mode: "url"; url: string };

/** Сервер инференса — компонент инсталляции. Ключ API хранится в keychain, в профиле его нет. */
export interface ServerConfig {
  id: string;
  name: string;
  baseUrl: string;
  kind: ServerKind;
  metricsPath: string;
  ca: CaConfig;
  proxy: ProxyConfig;
}

export interface InstallationProfile {
  id: string;
  name: string;
  color: string;
  environment: Environment;
  readOnly: boolean;
  servers: ServerConfig[];
  requestTimeoutMs: number;
  connectTimeoutMs: number;
  folderId: string | null;
  order: number;
}

/** Ключи API из формы: id сервера → значение; сервера нет — оставить сохранённый, "" — удалить. */
export interface SecretsInput {
  apiKeys?: Record<string, string>;
}

export interface InstallationFolder {
  id: string;
  name: string;
  parentId: string | null;
  order: number;
}

/** Итог экспорта инсталляций: сколько записано в файл. */
export interface ExportSummary {
  installations: number;
  folders: number;
}

/** Элемент предпросмотра импорта; `path` — имена папок от корня до места элемента. */
export interface PreviewItem {
  kind: "folder" | "installation";
  id: string;
  name: string;
  path: string[];
}

/** Инсталляция файла, которая не будет импортирована. */
export interface SkippedImport {
  name: string;
  reason: LocalizedText;
}

/** Предпросмотр (и итог) импорта инсталляций. */
export interface ImportPreview {
  added: PreviewItem[];
  updated: PreviewItem[];
  unchanged: PreviewItem[];
  skipped: SkippedImport[];
  warnings: LocalizedText[];
  /** Сколько новых инсталляций ждут ключи API. */
  needsSecrets: number;
}

/** Ссылка на элемент дерева инсталляций для перемещения. */
export interface TreeItemRef {
  kind: "folder" | "installation";
  id: string;
}

export type TestStepName = "config" | "tcp" | "tls" | "health" | "auth" | "kind" | "models";

export interface TestStep {
  name: TestStepName;
  ok: boolean;
  message: LocalizedText;
  error?: AppError;
}

export interface TestReport {
  ok: boolean;
  steps: TestStep[];
  /** Предупреждения (старая версия, другой тип в профиле, нет метрик) успеха не отменяют. */
  warnings: LocalizedText[];
}

/** Тип сервера, с которым работает Glafira. */
export type DetectedKind = "vllm" | "sglang" | "openai";

/** Модель из `/v1/models`; у адаптера LoRA `parent` — базовая модель. */
export interface Model {
  id: string;
  root?: string;
  parent?: string;
  maxModelLen?: number;
  ownedBy?: string;
}

export interface ServerFacts {
  kind: DetectedKind;
  detected: DetectedKind;
  version: string | null;
  models: Model[];
  /** `/metrics` отвечает, и в нём есть серии этого типа. */
  metrics: boolean;
  warnings: LocalizedText[];
}

/** Итог подключения сервера: сведения или ошибка. */
export interface ServerState {
  serverId: string;
  facts?: ServerFacts;
  error?: AppError;
}

export interface InstallationInfo {
  installationId: string;
  servers: ServerState[];
}

export interface PolicyStatus {
  managed: boolean;
  source: string | null;
  updatesEnabled: boolean;
  updateUrl: string | null;
  language: string | null;
  mcpEnabled: boolean;
  /** false — серверы без проверки TLS запрещены администратором. */
  allowInsecureTls: boolean;
  errors: LocalizedText[];
}

export interface AppInfo {
  name: string;
  version: string;
  commit: string;
  os: string;
  arch: string;
  dataDir: string;
  logDir: string;
}

export type LanguageReason = "policy" | "user" | "system" | "default";

export interface LanguageSettings {
  system: string | null;
  systemMatch: string | null;
  chosen: string | null;
  policy: string | null;
  effective: string;
  reason: LanguageReason;
  languages: string[];
}
````

<!-- block: file t10-src-locales-en-json step=10.3 path=src/locales/en.json -->
````json
{
  "errors": {
    "installations": {
      "serverNotFound": "Server {{id}} is not in the installation",
      "insecureForbidden": "Servers without TLS certificate verification are forbidden by your organization",
      "caPemInvalid": "The CA certificate of server “{{server}}” must be in PEM format (-----BEGIN CERTIFICATE-----)",
      "badProxyUrl": "Invalid proxy address “{{url}}”: use http://host:port",
      "badApiKey": "The API key contains characters that cannot be sent in a header",
      "notFound": "Installation {{id}} not found",
      "credentialsInUrl": "Remove the user name and password from “{{url}}”: credentials do not belong in the address (a proxy with a password is set via the system HTTPS_PROXY)",
      "badServerUrl": "Invalid address “{{url}}” of server “{{server}}”: use http://host:port or https://host:port",
      "badServerId": "Server “{{server}}” has an invalid ID",
      "emptyServerName": "Server name cannot be empty",
      "badMetricsPath": "Invalid metrics path “{{path}}” of server “{{server}}”: use a path such as /metrics",
      "pemContainsKey": "A certificate field contains a private key — remove it: Glafira needs only the CA certificate",
      "noId": "The installation has no ID",
      "emptyName": "Installation name cannot be empty",
      "noServers": "Add at least one inference server",
      "tooManyServers": "An installation can have at most {{max}} servers",
      "duplicateServerId": "Two servers have the same ID “{{id}}”",
      "duplicateServerName": "Two servers are named “{{server}}”: give them different names",
      "timeoutTooSmall": "Timeouts must be at least 1000 ms"
    },
    "app": {
      "taskPanicked": "Background task failed: {{error}}"
    },
    "settings": {
      "languageByPolicy": "The language is set by your organization",
      "unknownLanguage": "Unknown language “{{code}}”"
    },
    "internal": "Internal error",
    "io": "I/O error: {{error}}",
    "json": "JSON error: {{error}}",
    "generic": "{{message}}",
    "policy": {
      "invalidBool": "{{key}}: invalid boolean {{value}}",
      "invalidNumber": "{{key}}: invalid number {{value}}",
      "invalidString": "{{key}}: invalid string “{{value}}”",
      "invalidType": "{{key}}: invalid value of type {{kind}}",
      "unreadable": "Policy cannot be read: {{reason}}",
      "httpsRequired": "{{key}}: an https:// address is required — updates are not downloaded over http",
      "badLanguage": "{{key}}: unknown language “{{value}}”"
    },
    "secrets": {
      "keychain": "Cannot access the system secret store",
      "readOnly": "OverlayStore is read-only"
    },
    "server": {
      "badJson": "Unexpected answer to {{path}}: not the JSON this server type returns",
      "dns": "Cannot resolve the server host name",
      "connect": "Cannot connect to the server",
      "unhealthy": "The server is not healthy (/health answered {{status}}): it may still be loading the model",
      "keyRequired": "The server requires an API key — enter it in the server settings",
      "notOpenAi": "The server has no /v1/models — is this the address of an OpenAI-compatible server?",
      "noModels": "The server serves no models",
      "hostnameMismatch": "The server certificate does not match its address — check the address",
      "certExpired": "The server certificate has expired or is not valid yet",
      "caUntrusted": "The server certificate is not signed by a trusted CA — specify the CA of the server",
      "tlsError": "TLS connection error",
      "transport": "Network error: {{message}}",
      "timeout": "The server did not respond in time",
      "unauthorized": "The server rejected the request ({{status}}) — check the API key",
      "notFound": "The server has no {{path}} (404)",
      "redirect": "The server redirects {{path}} to another address ({{status}}) — use the final address of the server",
      "serverError": "The server failed to answer {{path}} ({{status}})",
      "unexpectedStatus": "Unexpected answer to {{path}}: {{status}}"
    },
    "import": {
      "notGlafiraFile": "This is not a Glafira installations file",
      "invalidJson": "The file is damaged or is not JSON: {{error}}",
      "incompatibleVersion": "The file was created by an incompatible version of Glafira — update the app",
      "unsupportedInstallation": "Not supported by this version of Glafira (update the app): {{error}}",
      "duplicateId": "Duplicate installation id in the file",
      "insecureForbidden": "Servers without TLS certificate verification are forbidden by your organization"
    },
    "storage": {
      "writeFailed": "Failed to write {{path}}: {{error}}",
      "newerSchema": "{{file}} was created by a newer version of the app (schemaVersion {{version}})",
      "quarantineFailed": "File {{path}} is corrupted ({{parseError}}) and a backup copy could not be saved: {{renameError}}"
    },
    "folders": {
      "notFound": "Folder {{id}} not found",
      "nameEmpty": "Enter a folder name",
      "nameTooLong": "The folder name is longer than 100 characters",
      "cycle": "A folder cannot be moved into itself or its subfolder",
      "nameTaken": "A folder named “{{name}}” already exists here",
      "notEmpty": "The folder is not empty: move or delete its installations and folders first"
    }
  },
  "messages": {
    "connectionTest": {
      "models": "Models: {{models}}",
      "kindVersion": "{{kind}} {{version}}",
      "kind": "{{kind}}",
      "configOk": "Configuration is valid",
      "tcpSystemProxy": "The system proxy is used: the connection is checked by the requests below",
      "tcpProxyOk": "TCP connection to the proxy {{address}} established",
      "tcpOk": "TCP connection to {{address}} established",
      "tlsOk": "TLS connection established, the certificate is trusted",
      "healthOk": "The server is healthy",
      "healthMissing": "The server has no /health — skipped",
      "healthNeedsKey": "/health requires an API key — checked by the next step",
      "keyAccepted": "The API key is accepted",
      "noKeyNeeded": "The server answers without an API key"
    },
    "servers": {
      "kindMismatch": "The server looks like {{detected}}, but {{configured}} is selected in the installation",
      "unsupportedVersion": "{{kind}} {{version}} is older than {{min}}: Glafira works with it as with an OpenAI-compatible server",
      "noMetricsSglang": "SGLang does not expose metrics: start it with --enable-metrics",
      "noMetrics": "The server does not expose its metrics at the metrics path of the installation"
    },
    "import": {
      "addressChanged": "“{{name}}”, server “{{server}}”: the address changes from {{from}} to {{to}}. The saved API key will be sent to the new address.",
      "proxyChanged": "“{{name}}”, server “{{server}}”: the proxy settings change. The saved API key will go through the new proxy.",
      "tlsChanged": "“{{name}}”, server “{{server}}”: the certificate verification settings change. The saved API key will be used with the new settings."
    }
  },
  "common": {
    "retry": "Retry",
    "cancel": "Cancel",
    "save": "Save",
    "delete": "Delete",
    "close": "Close",
    "loading": "Loading…",
    "details": "Details",
    "readOnly": "Read-only"
  },
  "hints": {
    "label": "Hint: {{name}}"
  }
}
````

<!-- block: file t10-src-locales-ru-json step=10.3 path=src/locales/ru.json -->
````json
{
  "errors": {
    "app": {
      "taskPanicked": "Фоновая задача завершилась аварийно: {{error}}"
    },
    "folders": {
      "cycle": "Папку нельзя переместить в неё саму или во вложенную папку",
      "nameEmpty": "Укажите имя папки",
      "nameTaken": "Папка «{{name}}» здесь уже есть",
      "nameTooLong": "Имя папки длиннее 100 символов",
      "notEmpty": "Папка не пуста: сначала перенесите или удалите её инсталляции и папки",
      "notFound": "Папка {{id}} не найдена"
    },
    "generic": "{{message}}",
    "import": {
      "duplicateId": "Повторяющийся id инсталляции в файле",
      "incompatibleVersion": "Файл создан в несовместимой версии Glafira — обновите приложение",
      "insecureForbidden": "Серверы без проверки TLS-сертификата запрещены вашей организацией",
      "invalidJson": "Файл повреждён или это не JSON: {{error}}",
      "notGlafiraFile": "Это не файл инсталляций Glafira",
      "unsupportedInstallation": "Не поддерживается этой версией Glafira (обновите приложение): {{error}}"
    },
    "installations": {
      "badApiKey": "В ключе API есть символы, которые нельзя передать в заголовке",
      "badMetricsPath": "Неверный путь метрик «{{path}}» сервера «{{server}}»: укажите путь вида /metrics",
      "badProxyUrl": "Неверный адрес прокси «{{url}}»: используйте http://хост:порт",
      "badServerId": "У сервера «{{server}}» недопустимый идентификатор",
      "badServerUrl": "Неверный адрес «{{url}}» сервера «{{server}}»: используйте http://хост:порт или https://хост:порт",
      "caPemInvalid": "CA-сертификат сервера «{{server}}» должен быть в формате PEM (-----BEGIN CERTIFICATE-----)",
      "credentialsInUrl": "Уберите имя пользователя и пароль из «{{url}}»: учётным данным не место в адресе (прокси с паролем задаётся системной HTTPS_PROXY)",
      "duplicateServerId": "У двух серверов одинаковый идентификатор «{{id}}»",
      "duplicateServerName": "Два сервера называются «{{server}}»: дайте им разные имена",
      "emptyName": "Имя инсталляции не может быть пустым",
      "emptyServerName": "Имя сервера не может быть пустым",
      "insecureForbidden": "Серверы без проверки TLS-сертификата запрещены вашей организацией",
      "noId": "У инсталляции нет идентификатора",
      "noServers": "Добавьте хотя бы один сервер инференса",
      "notFound": "Инсталляция {{id}} не найдена",
      "pemContainsKey": "В поле сертификата есть приватный ключ — уберите его: Glafira нужен только CA-сертификат",
      "serverNotFound": "Сервера {{id}} нет в инсталляции",
      "timeoutTooSmall": "Таймауты должны быть не меньше 1000 мс",
      "tooManyServers": "В инсталляции может быть не больше {{max}} серверов"
    },
    "internal": "Внутренняя ошибка",
    "io": "Ошибка ввода-вывода: {{error}}",
    "json": "Ошибка JSON: {{error}}",
    "policy": {
      "badLanguage": "{{key}}: неизвестный язык «{{value}}»",
      "httpsRequired": "{{key}}: нужен адрес https:// — обновления по http не загружаются",
      "invalidBool": "{{key}}: недопустимое логическое {{value}}",
      "invalidNumber": "{{key}}: недопустимое число {{value}}",
      "invalidString": "{{key}}: недопустимая строка «{{value}}»",
      "invalidType": "{{key}}: недопустимое значение типа {{kind}}",
      "unreadable": "Политика не читается: {{reason}}"
    },
    "secrets": {
      "keychain": "Ошибка доступа к системному хранилищу секретов",
      "readOnly": "OverlayStore только для чтения"
    },
    "server": {
      "badJson": "Неожиданный ответ на {{path}}: это не тот JSON, который отдаёт сервер этого типа",
      "caUntrusted": "Сертификат сервера подписан недоверенным CA — укажите CA сервера",
      "certExpired": "Сертификат сервера истёк или ещё не действует",
      "connect": "Не удалось подключиться к серверу",
      "dns": "Не удалось разрешить имя хоста сервера",
      "hostnameMismatch": "Сертификат сервера не подходит к его адресу — проверьте адрес",
      "keyRequired": "Сервер требует ключ API — укажите его в настройках сервера",
      "noModels": "На сервере нет ни одной модели",
      "notFound": "У сервера нет {{path}} (404)",
      "notOpenAi": "У сервера нет /v1/models — это точно адрес OpenAI-совместимого сервера?",
      "redirect": "Сервер перенаправляет {{path}} на другой адрес ({{status}}) — укажите конечный адрес сервера",
      "serverError": "Сервер не смог ответить на {{path}} ({{status}})",
      "timeout": "Сервер не ответил вовремя",
      "tlsError": "Ошибка TLS-соединения",
      "transport": "Сетевая ошибка: {{message}}",
      "unauthorized": "Сервер отклонил запрос ({{status}}) — проверьте ключ API",
      "unexpectedStatus": "Неожиданный ответ на {{path}}: {{status}}",
      "unhealthy": "Сервер нездоров (/health ответил {{status}}): возможно, он ещё загружает модель"
    },
    "settings": {
      "languageByPolicy": "Язык задан вашей организацией",
      "unknownLanguage": "Неизвестный язык «{{code}}»"
    },
    "storage": {
      "newerSchema": "{{file}} создан более новой версией приложения (schemaVersion {{version}})",
      "quarantineFailed": "Файл {{path}} повреждён ({{parseError}}), не удалось сохранить копию: {{renameError}}",
      "writeFailed": "Не удалось записать {{path}}: {{error}}"
    }
  },
  "messages": {
    "connectionTest": {
      "configOk": "Конфигурация верна",
      "healthMissing": "У сервера нет /health — шаг пропущен",
      "healthNeedsKey": "/health требует ключ API — его проверит следующий шаг",
      "healthOk": "Сервер здоров",
      "keyAccepted": "Ключ API принят",
      "kind": "{{kind}}",
      "kindVersion": "{{kind}} {{version}}",
      "models": "Модели: {{models}}",
      "noKeyNeeded": "Сервер отвечает без ключа API",
      "tcpOk": "TCP-соединение с {{address}} установлено",
      "tcpProxyOk": "TCP-соединение с прокси {{address}} установлено",
      "tcpSystemProxy": "Используется системный прокси: соединение проверят запросы ниже",
      "tlsOk": "TLS-соединение установлено, сертификату можно доверять"
    },
    "import": {
      "addressChanged": "«{{name}}», сервер «{{server}}»: адрес меняется с {{from}} на {{to}}. Сохранённый ключ API уйдёт на новый адрес.",
      "proxyChanged": "«{{name}}», сервер «{{server}}»: меняются настройки прокси. Сохранённый ключ API пойдёт через новый прокси.",
      "tlsChanged": "«{{name}}», сервер «{{server}}»: меняется проверка сертификата. Сохранённый ключ API будет использоваться с новыми настройками."
    },
    "servers": {
      "kindMismatch": "Сервер похож на {{detected}}, а в инсталляции выбран {{configured}}",
      "noMetrics": "Сервер не отдаёт метрики по пути метрик инсталляции",
      "noMetricsSglang": "SGLang не отдаёт метрики: запустите его с --enable-metrics",
      "unsupportedVersion": "{{kind}} {{version}} старше {{min}}: Glafira работает с ним как с OpenAI-совместимым сервером"
    }
  },
  "common": {
    "retry": "Повторить",
    "cancel": "Отмена",
    "save": "Сохранить",
    "delete": "Удалить",
    "close": "Закрыть",
    "loading": "Загрузка…",
    "details": "Подробности",
    "readOnly": "Только чтение"
  },
  "hints": {
    "label": "Подсказка: {{name}}"
  }
}
````

<!-- block: patch t10-src step=10.3 -->
````diff
diff --git a/PORTED.md b/PORTED.md
index 4d3b6df..cf47234 100644
--- a/PORTED.md
+++ b/PORTED.md
@@ -62,3 +62,35 @@
 | `src-tauri/src/servers/connectivity.rs` | `src-tauri/src/k8s/connectivity.rs` | `468e08a` | адаптирован: шаги сервера инференса (здоровье, вход ключом API, тип и версия, модели), предупреждения, системный прокси |
 | `src-tauri/src/commands/app.rs` | `src-tauri/src/commands/app.rs` | `468e08a` | адаптирован: только сведения «О программе», обновления — в I2 |
 | `src-tauri/src/commands/installations.rs` | `src-tauri/src/commands/connections.rs` | `468e08a` | адаптирован: ключи API серверов вместо токена и ключа сертификата, проверка сервера, подключение инсталляции |
+| `scripts/i18n-check.mjs` | `scripts/i18n-check.mjs` | `468e08a` | адаптирован: глав руководства пока нет |
+| `scripts/i18n-check.test.mjs` | `scripts/i18n-check.test.mjs` | `468e08a` | как есть |
+| `scripts/changelog.mjs` | `scripts/changelog.mjs` | `468e08a` | как есть |
+| `scripts/changelog.test.mjs` | `scripts/changelog.test.mjs` | `468e08a` | как есть |
+| `scripts/merge-locales.mjs` | `scripts/merge-locales.mjs` | `468e08a` | как есть |
+| `scripts/merge-locales.test.mjs` | `scripts/merge-locales.test.mjs` | `468e08a` | как есть |
+| `src/i18n/index.ts` | `src/i18n/index.ts` | `468e08a` | как есть |
+| `src/i18n/text.ts` | `src/i18n/text.ts` | `468e08a` | как есть |
+| `src/i18n/languages.ts` | `src/i18n/languages.ts` | `468e08a` | адаптирован: ссылка на этап I7 |
+| `src/i18n/documentLang.test.ts` | `src/i18n/documentLang.test.ts` | `468e08a` | как есть |
+| `src/i18n/locales.test.ts` | `src/i18n/locales.test.ts` | `468e08a` | как есть |
+| `src/i18n/text.test.ts` | `src/i18n/text.test.ts` | `468e08a` | как есть |
+| `src/lib/activate.ts` | `src/lib/activate.ts` | `468e08a` | как есть |
+| `src/lib/activate.test.tsx` | `src/lib/activate.test.tsx` | `468e08a` | как есть |
+| `src/lib/env.ts` | `src/lib/env.ts` | `468e08a` | как есть |
+| `src/lib/focusTrap.ts` | `src/lib/focusTrap.ts` | `468e08a` | как есть |
+| `src/lib/format.ts` | `src/lib/format.ts` | `468e08a` | как есть |
+| `src/lib/format.test.ts` | `src/lib/format.test.ts` | `468e08a` | как есть |
+| `src/lib/formatAgo.ts` | `src/lib/formatAgo.ts` | `468e08a` | как есть |
+| `src/lib/links.ts` | `src/lib/links.ts` | `468e08a` | адаптирован: адреса репозиториев |
+| `src/lib/plainInputs.ts` | `src/lib/plainInputs.ts` | `468e08a` | как есть |
+| `src/lib/plainInputs.test.ts` | `src/lib/plainInputs.test.ts` | `468e08a` | как есть |
+| `src/lib/useDebouncedValue.ts` | `src/lib/useDebouncedValue.ts` | `468e08a` | как есть |
+| `src/lib/useDebouncedValue.test.ts` | `src/lib/useDebouncedValue.test.ts` | `468e08a` | как есть |
+| `src/lib/useLanguageSync.ts` | `src/lib/useLanguageSync.ts` | `468e08a` | как есть |
+| `src/components/common/EnvBadge.tsx` | `src/components/common/EnvBadge.tsx` | `468e08a` | как есть |
+| `src/components/common/ErrorBanner.tsx` | `src/components/common/ErrorBanner.tsx` | `468e08a` | как есть |
+| `src/components/common/ErrorBanner.test.tsx` | `src/components/common/ErrorBanner.test.tsx` | `468e08a` | адаптирован: ошибка сервера инференса |
+| `src/components/common/Hint.tsx` | `src/components/common/Hint.tsx` | `468e08a` | как есть |
+| `src/components/common/Hint.test.tsx` | `src/components/common/Hint.test.tsx` | `468e08a` | как есть |
+| `src/test/render.tsx` | `src/test/render.tsx` | `468e08a` | как есть |
+| `src/test/setup.ts` | `src/test/setup.ts` | `468e08a` | как есть |
diff --git a/package.json b/package.json
index a228c27..032eff9 100644
--- a/package.json
+++ b/package.json
@@ -11,7 +11,8 @@
     "preview": "vite preview",
     "tauri": "tauri",
     "test": "vitest run",
-    "test:scripts": "node --test \"scripts/*.test.mjs\""
+    "test:scripts": "node --test \"scripts/*.test.mjs\"",
+    "i18n:check": "node scripts/i18n-check.mjs --rust-cyrillic src-tauri/src"
   },
   "dependencies": {
     "@floating-ui/react": "^0.27.20",
diff --git a/src/App.tsx b/src/App.tsx
index 30d33c1..a5b7169 100644
--- a/src/App.tsx
+++ b/src/App.tsx
@@ -1,3 +1,24 @@
+import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
+import { useLanguageSync } from "./lib/useLanguageSync";
+
+function LanguageSync() {
+  useLanguageSync();
+  return null;
+}
+
+// Без автоповтора: backend сам ждёт до таймаута и быстро отказывает при недоступном сервере,
+// а повтор удваивал бы ожидание. Повторить можно кнопкой в баннере ошибки.
+// Без перезапроса при возвращении сети: вкладки остаются смонтированными, и событие online
+// заново подключало бы каждую открытую инсталляцию без действия пользователя.
+const queryClient = new QueryClient({
+  defaultOptions: { queries: { retry: false, refetchOnWindowFocus: false, refetchOnReconnect: false } },
+});
+
 export default function App() {
-  return <main className="page">Glafira for LLM Inference</main>;
+  return (
+    <QueryClientProvider client={queryClient}>
+      <LanguageSync />
+      <main className="page">Glafira for LLM Inference</main>
+    </QueryClientProvider>
+  );
 }
diff --git a/src/main.tsx b/src/main.tsx
index 8993bd8..b250a45 100644
--- a/src/main.tsx
+++ b/src/main.tsx
@@ -1,8 +1,12 @@
 import "./polyfills";
+import "./i18n";
 import React from "react";
 import ReactDOM from "react-dom/client";
 import App from "./App";
 import "./styles.css";
+import { installPlainInputs } from "./lib/plainInputs";
+
+installPlainInputs();
 
 ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
   <React.StrictMode>
````

<!-- block: shell t10-install step=10.3 -->
````bash
pnpm install
````

- [ ] **Step 4: Проверки.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib`  
Expected: PASS — `test result: ok. 161 passed; 0 failed`

Run: `pnpm test`  
Expected: PASS — `Test Files 12 passed`, `Tests 41 passed`

Run: `pnpm exec tsc --noEmit`  
Expected: PASS — без ошибок

Run: `pnpm i18n:check`  
Expected: PASS — `✓ словари en, ru согласованы, вызовов из Rust: 95`

Run: `pnpm test:scripts`  
Expected: PASS — `# pass 54`, `# fail 0`

Run: `pnpm build`  
Expected: PASS — `✓ dist/assets: чанки совместимы с WebKit macOS 11+`

- [ ] **Step 5: Commit.**

```bash
git add \
  CHANGELOG.md \
  PORTED.md \
  docs/changelog/ru.md \
  docs/guide/en/.gitkeep \
  docs/guide/ru/.gitkeep \
  docs/i18n/same-as-en.json \
  package.json \
  scripts/changelog.mjs \
  scripts/changelog.test.mjs \
  scripts/i18n-check.mjs \
  scripts/i18n-check.test.mjs \
  scripts/merge-locales.mjs \
  scripts/merge-locales.test.mjs \
  src/App.test.tsx \
  src/App.tsx \
  src/api/client.test.ts \
  src/api/client.ts \
  src/api/types.ts \
  src/components/common/EnvBadge.tsx \
  src/components/common/ErrorBanner.test.tsx \
  src/components/common/ErrorBanner.tsx \
  src/components/common/Hint.test.tsx \
  src/components/common/Hint.tsx \
  src/i18n/documentLang.test.ts \
  src/i18n/index.ts \
  src/i18n/languages.ts \
  src/i18n/locales.test.ts \
  src/i18n/text.test.ts \
  src/i18n/text.ts \
  src/lib/activate.test.tsx \
  src/lib/activate.ts \
  src/lib/env.ts \
  src/lib/focusTrap.ts \
  src/lib/format.test.ts \
  src/lib/format.ts \
  src/lib/formatAgo.ts \
  src/lib/links.ts \
  src/lib/plainInputs.test.ts \
  src/lib/plainInputs.ts \
  src/lib/useDebouncedValue.test.ts \
  src/lib/useDebouncedValue.ts \
  src/lib/useLanguageSync.ts \
  src/locales/en.json \
  src/locales/ru.json \
  src/main.tsx \
  src/test/render.tsx \
  src/test/setup.ts
git commit -s -m "feat(ui): frontend foundation — i18n with en and ru dictionaries, IPC types and API client, shared components" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 11: Форма инсталляции: серверы с типом, ключом API, CA и прокси, пошаговая проверка каждого сервера

**Files:**
- Port: `src/components/installations/tree/folderTree.test.ts` (с правками), `src/components/installations/tree/folderTree.ts` (с правками)
- Create (по Kubernetes-версии): `src/components/installations/InstallationFormTab.tsx` ← `src/components/connections/ConnectionFormTab.tsx`, `src/state/workspace.test.ts` ← `src/state/workspace.test.ts`, `src/state/workspace.ts` ← `src/state/workspace.ts`
- Create: `src/app.css`, `src/components/installations/InstallationForm.test.tsx`, `src/components/installations/InstallationForm.tsx`, `src/components/installations/profile.test.ts`, `src/components/installations/profile.ts`
- Modify: `PORTED.md`, `docs/i18n/same-as-en.json`, `src/api/types.ts`, `src/locales/en.json`, `src/locales/ru.json`, `src/main.tsx`

**Interfaces:**
- Consumes: задача 10: `api`, типы, `ErrorBanner`, `Hint`.
- Produces: `src/state/workspace.ts` — `TabSpec` (`installation`, `installation-form`, `about`, `help`, `settings`), `tabId`, `tabInstallation`, `useWorkspace { open, close, closeForInstallation, activate }`; `src/components/installations/profile.ts` — `newInstallation`, `newServer`, `newServerId`, `withCaMode`, `withProxyMode`, `isPlainHttp`; `InstallationForm`, `InstallationFormTab`; `tree/folderTree.ts` (`buildTree`, `filterTree`, `folderPaths`, `canMoveFolder`); `src/app.css`.

Поля инсталляции и блоки серверов: имя, адрес, тип, ключ API, путь метрик, CA, прокси. У каждого сервера своя проверка; отчёт с шагами и предупреждениями — под сервером. Ключ: пустое поле оставляет сохранённый, флажок «Удалить сохранённый ключ» отправляет `""`. По `http://` с ключом (введённым или сохранённым) — предупреждение. Новая prod-инсталляция — только для чтения. Удаление — подтверждением внутри формы: `window.confirm` в Tauri 2 не показывается. Id нового сервера — из `crypto.getRandomValues`: `randomUUID` нет в WebKit macOS 11.

- [ ] **Step 1: Перенос из Kubernetes-версии.**

Файлы копируются с коммита `468e08a` репозитория `~/projects/glafira-openshift` (`$KUBERNETES_REPO`) и правятся патчами — против перенесённого файла. Строки о них в `PORTED.md` добавляет шаг реализации.

<!-- block: shell t11-port step=11.1 -->
````bash
scripts/port.sh src/components/connections/tree/folderTree.ts src/components/installations/tree/folderTree.ts
scripts/port.sh src/components/connections/tree/folderTree.test.ts src/components/installations/tree/folderTree.test.ts
````

<!-- block: patch t11-port-src-components-installations-tree-foldertree-ts step=11.1 -->
````diff
diff --git a/src/components/installations/tree/folderTree.ts b/src/components/installations/tree/folderTree.ts
--- a/src/components/installations/tree/folderTree.ts
+++ b/src/components/installations/tree/folderTree.ts
@@ -1,8 +1,8 @@
-import type { ConnectionFolder, ConnectionProfile } from "../../../api/types";
+import type { InstallationFolder, InstallationProfile } from "../../../api/types";
 
 export type TreeNode =
-  | { kind: "folder"; folder: ConnectionFolder; children: TreeNode[] }
-  | { kind: "connection"; connection: ConnectionProfile };
+  | { kind: "folder"; folder: InstallationFolder; children: TreeNode[] }
+  | { kind: "installation"; installation: InstallationProfile };
 
 const byName = (a: string, b: string) => a.localeCompare(b, undefined, { sensitivity: "base", numeric: true });
 
@@ -16,19 +16,19 @@ function byOrderThenName(a: { order?: number; name: string }, b: { order?: numbe
 
 /**
  * Дерево из плоских списков: папки первыми, внутри уровня — по order, затем по имени.
- * Ссылка на несуществующую папку (у папки или подключения) означает корень.
+ * Ссылка на несуществующую папку (у папки или инсталляции) означает корень.
  */
-export function buildTree(folders: ConnectionFolder[], connections: ConnectionProfile[]): TreeNode[] {
+export function buildTree(folders: InstallationFolder[], installations: InstallationProfile[]): TreeNode[] {
   const known = new Set(folders.map((f) => f.id));
   const parentOf = (id: string | null | undefined) => (id && known.has(id) ? id : null);
 
-  const foldersByParent = new Map<string | null, ConnectionFolder[]>();
+  const foldersByParent = new Map<string | null, InstallationFolder[]>();
   for (const f of folders) {
     const key = parentOf(f.parentId);
     foldersByParent.set(key, [...(foldersByParent.get(key) ?? []), f]);
   }
-  const connsByFolder = new Map<string | null, ConnectionProfile[]>();
-  for (const c of connections) {
+  const connsByFolder = new Map<string | null, InstallationProfile[]>();
+  for (const c of installations) {
     const key = parentOf(c.folderId);
     connsByFolder.set(key, [...(connsByFolder.get(key) ?? []), c]);
   }
@@ -44,7 +44,7 @@ export function buildTree(folders: ConnectionFolder[], connections: ConnectionPr
       visited.add(folder.id);
       nodes.push({ kind: "folder", folder, children: level(folder.id) });
     }
-    for (const connection of conns) nodes.push({ kind: "connection", connection });
+    for (const installation of conns) nodes.push({ kind: "installation", installation });
     return nodes;
   };
   return level(null);
@@ -52,14 +52,14 @@ export function buildTree(folders: ConnectionFolder[], connections: ConnectionPr
 
 /**
  * Фильтр по имени без учёта регистра: совпавшая папка остаётся целиком,
- * у совпавшего подключения остаются предки. Пустой запрос возвращает исходный массив.
+ * у совпавшей инсталляции остаются предки. Пустой запрос возвращает исходный массив.
  */
 export function filterTree(nodes: TreeNode[], query: string): TreeNode[] {
   const q = query.trim().toLocaleLowerCase();
   if (!q) return nodes;
   const walk = (list: TreeNode[]): TreeNode[] =>
     list.flatMap((node): TreeNode[] => {
-      if (node.kind === "connection") return node.connection.name.toLocaleLowerCase().includes(q) ? [node] : [];
+      if (node.kind === "installation") return node.installation.name.toLocaleLowerCase().includes(q) ? [node] : [];
       if (node.folder.name.toLocaleLowerCase().includes(q)) return [node];
       const children = walk(node.children);
       return children.length > 0 ? [{ ...node, children }] : [];
@@ -68,12 +68,12 @@ export function filterTree(nodes: TreeNode[], query: string): TreeNode[] {
 }
 
 /** Пути папок от корня (для выпадающих списков), отсортированы по пути. */
-export function folderPaths(folders: ConnectionFolder[]): { id: string; path: string[] }[] {
+export function folderPaths(folders: InstallationFolder[]): { id: string; path: string[] }[] {
   const byId = new Map(folders.map((f) => [f.id, f]));
-  const pathOf = (folder: ConnectionFolder): string[] => {
+  const pathOf = (folder: InstallationFolder): string[] => {
     const path: string[] = [];
     const seen = new Set<string>();
-    let cur: ConnectionFolder | undefined = folder;
+    let cur: InstallationFolder | undefined = folder;
     while (cur && !seen.has(cur.id)) {
       seen.add(cur.id);
       path.unshift(cur.name);
@@ -92,7 +92,7 @@ export function folderPaths(folders: ConnectionFolder[]): { id: string; path: st
 }
 
 /** Можно ли перенести папку под targetParentId (null — корень): не в себя, не в потомка, не в несуществующую. */
-export function canMoveFolder(folders: ConnectionFolder[], folderId: string, targetParentId: string | null): boolean {
+export function canMoveFolder(folders: InstallationFolder[], folderId: string, targetParentId: string | null): boolean {
   if (targetParentId === null) return true;
   const byId = new Map(folders.map((f) => [f.id, f]));
   if (!byId.has(targetParentId)) return false;
````

<!-- block: patch t11-port-src-components-installations-tree-foldertree-test-ts step=11.1 -->
````diff
diff --git a/src/components/installations/tree/folderTree.test.ts b/src/components/installations/tree/folderTree.test.ts
--- a/src/components/installations/tree/folderTree.test.ts
+++ b/src/components/installations/tree/folderTree.test.ts
@@ -1,20 +1,20 @@
 import { describe, expect, it } from "vitest";
-import type { ConnectionFolder, ConnectionProfile } from "../../../api/types";
-import { newProfile } from "../profile";
+import type { InstallationFolder, InstallationProfile } from "../../../api/types";
+import { newInstallation } from "../profile";
 import { buildTree, canMoveFolder, filterTree, folderPaths, type TreeNode } from "./folderTree";
 
-const folder = (id: string, name: string, parentId: string | null = null, order = 0): ConnectionFolder => ({ id, name, parentId, order });
-const conn = (id: string, name: string, folderId: string | null = null, order = 0): ConnectionProfile => ({
-  ...newProfile(),
+const folder = (id: string, name: string, parentId: string | null = null, order = 0): InstallationFolder => ({ id, name, parentId, order });
+const conn = (id: string, name: string, folderId: string | null = null, order = 0): InstallationProfile => ({
+  ...newInstallation(),
   id,
   name,
   folderId,
   order,
 });
 
-/** Компактная запись дерева для сравнения: папка — "имя/[дети]", подключение — имя. */
+/** Компактная запись дерева для сравнения: папка — "имя/[дети]", инсталляция — имя. */
 function shape(nodes: TreeNode[]): unknown[] {
-  return nodes.map((n) => (n.kind === "folder" ? { [n.folder.name]: shape(n.children) } : n.connection.name));
+  return nodes.map((n) => (n.kind === "folder" ? { [n.folder.name]: shape(n.children) } : n.installation.name));
 }
 
 describe("buildTree", () => {
@@ -34,7 +34,7 @@ describe("buildTree", () => {
     expect(shape(tree)).toEqual([{ A: [{ B: [{ C: ["x"] }] }, "y"] }, "z"]);
   });
 
-  it("подключение с несуществующей папкой и без order — в корне", () => {
+  it("инсталляция с несуществующей папкой и без order — в корне", () => {
     const orphan = { ...conn("o", "orphan", "missing"), order: undefined };
     const tree = buildTree([folder("a", "A", "missing")], [orphan, conn("n", "named")]);
     expect(shape(tree)).toEqual([{ A: [] }, "named", "orphan"]);
@@ -51,7 +51,7 @@ describe("filterTree", () => {
     expect(filterTree(tree, "  ")).toBe(tree);
   });
 
-  it("совпало подключение — остаются его предки", () => {
+  it("совпала инсталляция — остаются её предки", () => {
     expect(shape(filterTree(tree, "IFT"))).toEqual([{ "Проект А": [{ ИФТ: ["kafka-ift"] }] }]);
   });
 
````

- [ ] **Step 2: Тесты.**

<!-- block: file t11-src-components-installations-installationform-test-tsx step=11.2 path=src/components/installations/InstallationForm.test.tsx -->
````tsx
import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { renderWithClient } from "../../test/render";

vi.mock("../../api/client", async (importOriginal) => {
  const orig = await importOriginal<typeof import("../../api/client")>();
  return {
    ...orig,
    api: {
      testServer: vi.fn(),
      saveInstallation: vi.fn(),
      deleteInstallation: vi.fn(),
      listInstallations: vi.fn().mockResolvedValue([]),
      listFolders: vi.fn().mockResolvedValue([]),
      moveItems: vi.fn(),
      getPolicyStatus: vi.fn(),
    },
  };
});

import { api } from "../../api/client";
import type { InstallationProfile, PolicyStatus } from "../../api/types";
import { useWorkspace } from "../../state/workspace";
import { InstallationForm } from "./InstallationForm";
import { newInstallation } from "./profile";

const POLICY: PolicyStatus = {
  managed: false,
  source: null,
  updatesEnabled: true,
  updateUrl: null,
  language: null,
  mcpEnabled: true,
  allowInsecureTls: true,
  errors: [],
};

const msg = (key: string, text: string, params: Record<string, string> = {}) => ({ key, params, text });

function saved(): InstallationProfile {
  return {
    ...newInstallation(),
    id: "i1",
    name: "prod-llm",
    servers: [
      { id: "s1", name: "gpu-1", baseUrl: "https://gpu-1:8000", kind: "auto", metricsPath: "/metrics", ca: { mode: "system" }, proxy: { mode: "system" } },
      { id: "s2", name: "gpu-2", baseUrl: "http://gpu-2:8000", kind: "vllm", metricsPath: "/metrics", ca: { mode: "system" }, proxy: { mode: "system" } },
    ],
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.listFolders).mockResolvedValue([]);
  vi.mocked(api.getPolicyStatus).mockResolvedValue(POLICY);
  useWorkspace.setState({ tabs: [], activeId: null });
});

describe("InstallationForm", () => {
  it("сохраняет новую инсталляцию с ключом сервера и открывает её вкладку", async () => {
    vi.mocked(api.saveInstallation).mockImplementation(async (p) => ({ ...p, id: "new-id" }));
    const user = userEvent.setup();
    useWorkspace.getState().open({ kind: "installation-form", installationId: null });
    renderWithClient(<InstallationForm initial={newInstallation()} tabId="installation-form:new" />);

    await user.type(screen.getByLabelText("Имя"), "dev-llm");
    await user.type(screen.getByLabelText("Имя сервера"), "vllm-1");
    await user.type(screen.getByLabelText("Адрес"), "https://vllm.dev:8000");
    await user.selectOptions(screen.getByLabelText("Тип сервера"), "vllm");
    await user.type(screen.getByLabelText("Ключ API"), "sk-dev");
    await user.click(screen.getByRole("button", { name: "Сохранить" }));

    await waitFor(() => expect(api.saveInstallation).toHaveBeenCalled());
    const [profile, secrets] = vi.mocked(api.saveInstallation).mock.calls[0];
    expect(profile).toMatchObject({ id: "", name: "dev-llm", folderId: null });
    expect(profile.servers[0]).toMatchObject({ name: "vllm-1", baseUrl: "https://vllm.dev:8000", kind: "vllm" });
    expect(secrets).toEqual({ apiKeys: { [profile.servers[0].id]: "sk-dev" } });
    await waitFor(() => expect(useWorkspace.getState().activeId).toBe("installation:new-id"));
    expect(useWorkspace.getState().tabs.map((t) => t.id)).toEqual(["installation:new-id"]);
  });

  it("новая prod-инсталляция — только для чтения", async () => {
    const user = userEvent.setup();
    renderWithClient(<InstallationForm initial={newInstallation()} tabId="t" />);
    await user.selectOptions(screen.getByLabelText("Окружение"), "prod");
    expect(screen.getByLabelText("Только чтение")).toBeChecked();
  });

  it("проверяет сервер формы и показывает шаги и предупреждения", async () => {
    vi.mocked(api.testServer).mockResolvedValue({
      ok: true,
      steps: [
        { name: "config", ok: true, message: msg("messages.connectionTest.configOk", "Configuration is valid") },
        { name: "tcp", ok: true, message: msg("messages.connectionTest.tcpOk", "TCP", { address: "10.0.0.5:8000" }) },
        { name: "kind", ok: true, message: msg("messages.connectionTest.kindVersion", "vLLM 0.7.3", { kind: "vLLM", version: "0.7.3" }) },
      ],
      warnings: [msg("messages.servers.unsupportedVersion", "old", { kind: "vLLM", version: "0.7.3", min: "0.8" })],
    });
    const user = userEvent.setup();
    renderWithClient(<InstallationForm initial={saved()} tabId="t" />);
    const second = screen.getByRole("group", { name: "Сервер 2" });
    await user.type(within(second).getByLabelText("Ключ API"), "sk-2");
    await user.click(within(second).getByRole("button", { name: "Проверить сервер" }));

    await waitFor(() => expect(api.testServer).toHaveBeenCalled());
    const [profile, serverId, secrets] = vi.mocked(api.testServer).mock.calls[0];
    expect(serverId).toBe("s2");
    expect(profile.servers).toHaveLength(2);
    expect(secrets).toEqual({ apiKeys: { s2: "sk-2" } });
    const report = await within(second).findByRole("list", { name: "Результат проверки" });
    expect(within(report).getByText("TCP-соединение с 10.0.0.5:8000 установлено")).toBeInTheDocument();
    expect(within(report).getByText(/vLLM 0\.7\.3 старше 0\.8/)).toBeInTheDocument();
    expect(within(screen.getByRole("group", { name: "Сервер 1" })).queryByRole("list")).toBeNull();
  });

  it("сохранённый ключ: пустое поле оставляет его, флажок удаляет", async () => {
    vi.mocked(api.saveInstallation).mockImplementation(async (p) => p);
    const user = userEvent.setup();
    renderWithClient(<InstallationForm initial={saved()} tabId="t" />);
    const first = screen.getByRole("group", { name: "Сервер 1" });
    expect(within(first).getByLabelText("Ключ API")).toHaveAttribute("placeholder", "оставьте пустым, чтобы не менять сохранённый");
    await user.click(within(first).getByLabelText("Удалить сохранённый ключ"));
    expect(within(first).getByLabelText("Ключ API")).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Сохранить" }));
    await waitFor(() => expect(api.saveInstallation).toHaveBeenCalled());
    expect(vi.mocked(api.saveInstallation).mock.calls[0][1]).toEqual({ apiKeys: { s1: "" } });
  });

  it("ключ по http — предупреждение", async () => {
    const user = userEvent.setup();
    renderWithClient(<InstallationForm initial={newInstallation()} tabId="t" />);
    await user.type(screen.getByLabelText("Адрес"), "http://10.0.0.5:8000");
    expect(screen.queryByText(/ключ API уйдёт открытым текстом/)).toBeNull();
    await user.type(screen.getByLabelText("Ключ API"), "sk");
    expect(screen.getByText("Адрес — http://: ключ API уйдёт открытым текстом")).toBeInTheDocument();
    // У сохранённого сервера с http ключ может уже быть: предупреждение и без ввода.
    renderWithClient(<InstallationForm initial={saved()} tabId="t2" />);
    expect(within(screen.getAllByRole("group", { name: "Сервер 2" })[0]).getByRole("alert")).toBeInTheDocument();
  });

  it("серверы добавляются и убираются; последний убрать нельзя", async () => {
    vi.mocked(api.saveInstallation).mockImplementation(async (p) => p);
    const user = userEvent.setup();
    renderWithClient(<InstallationForm initial={newInstallation()} tabId="t" />);
    expect(screen.queryByRole("button", { name: "Убрать сервер" })).toBeNull();
    await user.click(screen.getByRole("button", { name: "Добавить сервер" }));
    expect(screen.getAllByRole("group", { name: /Сервер \d/ })).toHaveLength(2);
    await user.click(screen.getAllByRole("button", { name: "Убрать сервер" })[0]);
    expect(screen.getAllByRole("group", { name: /Сервер \d/ })).toHaveLength(1);
  });

  it("CA: введённый PEM не теряется при переключении режима; без проверки — запрет политикой", async () => {
    vi.mocked(api.getPolicyStatus).mockResolvedValue({ ...POLICY, allowInsecureTls: false });
    const user = userEvent.setup();
    renderWithClient(<InstallationForm initial={newInstallation()} tabId="t" />);
    await user.selectOptions(screen.getByLabelText("Проверка сертификата"), "pem");
    await user.type(screen.getByLabelText("CA-сертификаты (PEM)"), "PEM-TEXT");
    await user.selectOptions(screen.getByLabelText("Проверка сертификата"), "system");
    await user.selectOptions(screen.getByLabelText("Проверка сертификата"), "pem");
    expect(screen.getByLabelText("CA-сертификаты (PEM)")).toHaveValue("PEM-TEXT");
    await waitFor(() => expect(screen.getByRole("option", { name: "Без проверки (небезопасно)" })).toBeDisabled());
  });

  it("ошибка сохранения показывается, вкладка остаётся", async () => {
    vi.mocked(api.saveInstallation).mockRejectedValue({
      code: "INVALID_INPUT",
      key: "errors.installations.noServers",
      message: "Add at least one inference server",
      retryable: false,
    });
    const user = userEvent.setup();
    useWorkspace.getState().open({ kind: "installation-form", installationId: null });
    renderWithClient(<InstallationForm initial={newInstallation()} tabId="installation-form:new" />);
    await user.click(screen.getByRole("button", { name: "Сохранить" }));
    expect(await screen.findByText("Добавьте хотя бы один сервер инференса")).toBeInTheDocument();
    expect(useWorkspace.getState().activeId).toBe("installation-form:new");
  });

  it("удаление — после подтверждения в форме; вкладки инсталляции закрываются", async () => {
    vi.mocked(api.deleteInstallation).mockResolvedValue(undefined);
    const user = userEvent.setup();
    const ws = useWorkspace.getState();
    ws.open({ kind: "installation", installationId: "i1" });
    ws.open({ kind: "installation-form", installationId: "i1" });
    renderWithClient(<InstallationForm initial={saved()} tabId="installation-form:i1" />);
    await user.click(screen.getByRole("button", { name: "Удалить" }));
    expect(api.deleteInstallation).not.toHaveBeenCalled();
    const confirm = screen.getByRole("group", { name: "Подтверждение удаления" });
    expect(within(confirm).getByText("Удалить «prod-llm» и ключи API её серверов?")).toBeInTheDocument();
    await user.click(within(confirm).getByRole("button", { name: "Удалить" }));
    await waitFor(() => expect(api.deleteInstallation).toHaveBeenCalledWith("i1"));
    await waitFor(() => expect(useWorkspace.getState().tabs).toEqual([]));
  });
});
````

<!-- block: file t11-src-components-installations-profile-test-ts step=11.2 path=src/components/installations/profile.test.ts -->
````ts
import { describe, expect, it } from "vitest";
import { isPlainHttp, newInstallation, newServer, newServerId, withCaMode, withProxyMode } from "./profile";

describe("профиль инсталляции", () => {
  it("новая инсталляция — с одним пустым сервером, тип определяется автоматически", () => {
    const p = newInstallation();
    expect(p.id).toBe("");
    expect(p.servers).toHaveLength(1);
    expect(p.servers[0]).toMatchObject({ kind: "auto", metricsPath: "/metrics", ca: { mode: "system" }, proxy: { mode: "system" } });
  });

  it("id сервера — короткий, латиница и цифры, у каждого свой", () => {
    const a = newServerId();
    expect(a).toMatch(/^s-[0-9a-f]{12}$/);
    expect(newServer().id).not.toBe(newServer().id);
  });

  it("CA и прокси: введённое не теряется при переключении режима", () => {
    expect(withCaMode({ mode: "pem", pem: "X" }, "pem")).toEqual({ mode: "pem", pem: "X" });
    expect(withCaMode({ mode: "system" }, "pem", "LAST")).toEqual({ mode: "pem", pem: "LAST" });
    expect(withCaMode({ mode: "pem", pem: "X" }, "insecure")).toEqual({ mode: "insecure" });
    expect(withProxyMode({ mode: "none" }, "url", "http://p:3128")).toEqual({ mode: "url", url: "http://p:3128" });
    expect(withProxyMode({ mode: "url", url: "u" }, "system")).toEqual({ mode: "system" });
  });

  it("http — без TLS", () => {
    expect(isPlainHttp("http://10.0.0.5:8000")).toBe(true);
    expect(isPlainHttp(" HTTP://h")).toBe(true);
    expect(isPlainHttp("https://h")).toBe(false);
    expect(isPlainHttp("")).toBe(false);
  });
});
````

<!-- block: file t11-src-state-workspace-test-ts step=11.2 path=src/state/workspace.test.ts -->
````ts
import { beforeEach, describe, expect, it } from "vitest";
import { tabId, tabInstallation, useWorkspace } from "./workspace";

beforeEach(() => {
  useWorkspace.setState({ tabs: [], activeId: null });
});

describe("workspace", () => {
  it("повторное открытие активирует существующую вкладку", () => {
    const { open } = useWorkspace.getState();
    open({ kind: "installation", installationId: "a" });
    open({ kind: "settings" });
    open({ kind: "installation", installationId: "a" });
    const s = useWorkspace.getState();
    expect(s.tabs).toHaveLength(2);
    expect(s.activeId).toBe("installation:a");
  });

  it("закрытие активной вкладки активирует соседнюю", () => {
    const { open, close, activate } = useWorkspace.getState();
    open({ kind: "installation", installationId: "a" });
    open({ kind: "installation", installationId: "b" });
    open({ kind: "installation", installationId: "c" });
    activate("installation:b");
    close("installation:b");
    expect(useWorkspace.getState().activeId).toBe("installation:c");
    close("installation:c");
    expect(useWorkspace.getState().activeId).toBe("installation:a");
    close("installation:a");
    expect(useWorkspace.getState().activeId).toBeNull();
  });

  it("id вкладок и инсталляция вкладки", () => {
    expect(tabId({ kind: "installation-form", installationId: null })).toBe("installation-form:new");
    expect(tabId({ kind: "installation-form", installationId: "a" })).toBe("installation-form:a");
    expect(tabId({ kind: "installation", installationId: "a" })).toBe("installation:a");
    expect(tabId({ kind: "help", chapter: "installations" })).toBe("help");
    expect(tabId({ kind: "about" })).toBe("about");
    expect(tabInstallation({ kind: "installation-form", installationId: "a" })).toBe("a");
    expect(tabInstallation({ kind: "settings" })).toBeNull();
  });

  it("справка: повторное открытие меняет главу, а не добавляет вкладку", () => {
    const { open } = useWorkspace.getState();
    open({ kind: "help", chapter: "getting-started" });
    open({ kind: "help", chapter: "installations", anchor: "servers" });
    const s = useWorkspace.getState();
    expect(s.tabs).toHaveLength(1);
    expect(s.tabs[0]).toMatchObject({ kind: "help", chapter: "installations", anchor: "servers" });
  });

  it("closeForInstallation закрывает все вкладки инсталляции", () => {
    const { open, closeForInstallation } = useWorkspace.getState();
    open({ kind: "installation", installationId: "a" });
    open({ kind: "installation-form", installationId: "a" });
    open({ kind: "installation", installationId: "b" });
    open({ kind: "settings" });
    useWorkspace.getState().activate("installation-form:a");
    closeForInstallation("a");
    const s = useWorkspace.getState();
    expect(s.tabs.map((t) => t.id)).toEqual(["installation:b", "settings"]);
    expect(s.activeId).toBe("installation:b");
  });
});
````

- [ ] **Step 3: Увидеть падение.**

Run: `pnpm vitest run src/components/installations/InstallationForm.test.tsx src/components/installations/profile.test.ts src/state/workspace.test.ts`  
Expected: FAIL — нет модулей `./workspace`, `./profile`, `./InstallationForm`.

- [ ] **Step 4: Реализация.**

<!-- block: file t11-src-app-css step=11.4 path=src/app.css -->
````css
/* Экраны инсталляций; общие стили — в styles.css (перенесён из семейства). */
/* Сервер в форме инсталляции: свой блок с проверкой под полями. */
fieldset.server { border: 1px solid var(--border); border-radius: 6px; padding: 8px 12px; display: flex; flex-direction: column; gap: 8px; margin: 0; min-width: 0; }
fieldset.server legend { padding: 0 4px; font-weight: 600; }
.page h3 { margin: 4px 0 0; font-size: 14px; }
/* Подсказка в пустом поле — курсивом и бледнее: её не путают ни с введённым значением, ни с подписью. */
input::placeholder, textarea::placeholder { font-style: italic; color: var(--muted); opacity: 0.7; }
````

<!-- block: file t11-src-components-installations-installationform-tsx step=11.4 path=src/components/installations/InstallationForm.tsx -->
````tsx
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api, toAppError } from "../../api/client";
import type { AppError, Environment, InstallationProfile, SecretsInput, ServerConfig, ServerKind, TestReport } from "../../api/types";
import { tr } from "../../i18n/text";
import { useWorkspace } from "../../state/workspace";
import { ErrorBanner } from "../common/ErrorBanner";
import { Hint } from "../common/Hint";
import { type CaMode, isPlainHttp, newServer, type ProxyMode, withCaMode, withProxyMode } from "./profile";
import { folderPaths } from "./tree/folderTree";

const ENVIRONMENTS: Environment[] = ["local", "dev", "test", "stage", "prod"];
const KINDS: ServerKind[] = ["auto", "vllm", "sglang", "openai"];
const CA_MODES: CaMode[] = ["system", "pem", "insecure"];
const PROXY_MODES: ProxyMode[] = ["system", "none", "url"];

/** Ключ API сервера в форме: введённое значение и «удалить сохранённый». */
interface KeyInput {
  value: string;
  clear: boolean;
}

export function InstallationForm({ initial, tabId }: { initial: InstallationProfile; tabId: string }) {
  const { t } = useTranslation();
  const [profile, setProfile] = useState(initial);
  // Ключи из формы: пустое поле — оставить сохранённый (у нового сервера его нет).
  const [keys, setKeys] = useState<Record<string, KeyInput>>({});
  // Последние введённые PEM и адрес прокси по серверам: переключение режима туда и обратно их не теряет.
  const [lastPem, setLastPem] = useState<Record<string, string>>({});
  const [lastProxy, setLastProxy] = useState<Record<string, string>>({});
  const [reports, setReports] = useState<Record<string, TestReport>>({});
  const [testing, setTesting] = useState<string | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [busy, setBusy] = useState(false);
  // Подтверждение удаления внутри формы: нативный window.confirm в Tauri 2 не показывается и возвращает false.
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const queryClient = useQueryClient();
  const foldersQ = useQuery({ queryKey: ["folders"], queryFn: api.listFolders });
  const policyQ = useQuery({ queryKey: ["policy"], queryFn: api.getPolicyStatus });
  const insecureForbidden = policyQ.data?.allowInsecureTls === false;
  const paths = useMemo(() => folderPaths(foldersQ.data ?? []), [foldersQ.data]);
  // Ссылка на несуществующую папку означает корень (так её показывает и дерево).
  const knownFolder = (id: string | null | undefined): string | null =>
    !id || (foldersQ.isSuccess && !paths.some((p) => p.id === id)) ? null : id;
  // Папка, с которой форма открылась: перенос — только если пользователь сменил её в списке.
  const [loadedFolderId] = useState(initial.folderId);
  const [folderTouched, setFolderTouched] = useState(false);
  const folderId = knownFolder(folderTouched ? profile.folderId : initial.folderId);
  const open = useWorkspace((s) => s.open);
  const close = useWorkspace((s) => s.close);
  const closeForInstallation = useWorkspace((s) => s.closeForInstallation);
  const isNew = initial.id === "";
  const savedServers = new Set(initial.servers.map((s) => s.id));

  const update = (patch: Partial<InstallationProfile>) => setProfile((p) => ({ ...p, ...patch }));
  const updateServer = (id: string, patch: Partial<ServerConfig>) =>
    setProfile((p) => ({ ...p, servers: p.servers.map((s) => (s.id === id ? { ...s, ...patch } : s)) }));
  const setKey = (id: string, patch: Partial<KeyInput>) =>
    setKeys((k) => ({ ...k, [id]: { value: k[id]?.value ?? "", clear: k[id]?.clear ?? false, ...patch } }));
  const current = (): InstallationProfile => ({ ...profile, folderId });
  const secrets = (): SecretsInput => {
    const apiKeys: Record<string, string> = {};
    for (const s of profile.servers) {
      const k = keys[s.id];
      if (k?.clear) apiKeys[s.id] = "";
      else if (k?.value.trim()) apiKeys[s.id] = k.value;
    }
    return { apiKeys };
  };

  async function run(fn: () => Promise<void>) {
    setBusy(true);
    setError(null);
    try {
      await fn();
    } catch (e) {
      setError(toAppError(e));
    } finally {
      setBusy(false);
    }
  }

  const onTest = (serverId: string) =>
    run(async () => {
      setTesting(serverId);
      setReports((r) => {
        const rest = { ...r };
        delete rest[serverId];
        return rest;
      });
      try {
        const report = await api.testServer(current(), serverId, secrets());
        setReports((r) => ({ ...r, [serverId]: report }));
      } finally {
        setTesting(null);
      }
    });

  const onSave = () =>
    run(async () => {
      const saved = await api.saveInstallation(current(), secrets());
      // save_installation не меняет место существующей инсталляции в дереве — перенос отдельной командой.
      try {
        if (!isNew && folderTouched && folderId !== knownFolder(loadedFolderId)) {
          await api.moveItems([{ kind: "installation", id: saved.id }], folderId, null);
        }
      } finally {
        // Настройки уже сохранены, даже если перенос не удался: список нужно обновить в любом случае.
        await queryClient.invalidateQueries({ queryKey: ["installations"] });
      }
      // Rust забыл подключение инсталляции: вкладка подключится заново с новым профилем.
      queryClient.removeQueries({ predicate: (q) => q.queryKey[1] === saved.id });
      close(tabId);
      open({ kind: "installation", installationId: saved.id });
    });

  const onDelete = () =>
    run(async () => {
      setConfirmingDelete(false);
      await api.deleteInstallation(profile.id);
      await queryClient.invalidateQueries({ queryKey: ["installations"] });
      queryClient.removeQueries({ predicate: (q) => q.queryKey[1] === profile.id });
      closeForInstallation(profile.id);
      close(tabId);
    });

  const addServer = () => update({ servers: [...profile.servers, newServer()] });
  const removeServer = (id: string) => {
    update({ servers: profile.servers.filter((s) => s.id !== id) });
    setReports((r) => {
      const rest = { ...r };
      delete rest[id];
      return rest;
    });
  };

  return (
    <div className="page">
      <div className="toolbar">
        <h2>{isNew ? t("installations.form.titleNew") : t("installations.form.titleEdit", { name: initial.name })}</h2>
      </div>
      <div className="form">
        <label htmlFor="if-name">{t("installations.form.name")}</label>
        <input id="if-name" value={profile.name} onChange={(e) => update({ name: e.target.value })} />

        <label htmlFor="if-folder">{t("installations.form.folder")}</label>
        <select
          id="if-folder"
          value={folderId ?? ""}
          onChange={(e) => {
            setFolderTouched(true);
            update({ folderId: e.target.value || null });
          }}
        >
          <option value="">{t("installations.form.folderRoot")}</option>
          {paths.map((p) => (
            <option key={p.id} value={p.id}>
              {p.path.join(" / ")}
            </option>
          ))}
        </select>

        <span className="field-label">
          <label htmlFor="if-env">{t("installations.form.environment")}</label>
          <Hint k="hints.installations.environment" label={t("installations.form.environment")} />
        </span>
        <select
          id="if-env"
          value={profile.environment}
          onChange={(e) => {
            const environment = e.target.value as Environment;
            // Новая prod-инсталляция по умолчанию только для чтения; снять флажок можно сразу.
            update({ environment, readOnly: environment === "prod" && isNew ? true : profile.readOnly });
          }}
        >
          {ENVIRONMENTS.map((env) => (
            <option key={env} value={env}>
              {env}
            </option>
          ))}
        </select>

        <span className="field-label">
          <label htmlFor="if-ro">{t("common.readOnly")}</label>
          <Hint k="hints.installations.readOnly" label={t("common.readOnly")} />
        </span>
        <span>
          <input id="if-ro" type="checkbox" checked={profile.readOnly} onChange={(e) => update({ readOnly: e.target.checked })} />
        </span>

        <label htmlFor="if-color">{t("installations.form.color")}</label>
        <input id="if-color" type="color" value={profile.color} onChange={(e) => update({ color: e.target.value })} />

        <label htmlFor="if-timeout">{t("installations.form.requestTimeout")}</label>
        <input
          id="if-timeout"
          type="number"
          min={1000}
          step={1000}
          value={profile.requestTimeoutMs}
          onChange={(e) => update({ requestTimeoutMs: Number(e.target.value) })}
        />

        <label htmlFor="if-ctimeout">{t("installations.form.connectTimeout")}</label>
        <input
          id="if-ctimeout"
          type="number"
          min={1000}
          step={1000}
          value={profile.connectTimeoutMs}
          onChange={(e) => update({ connectTimeoutMs: Number(e.target.value) })}
        />
      </div>

      <h3>{t("installations.form.servers")}</h3>
      {profile.servers.map((s, i) => (
        <ServerFields
          key={s.id}
          server={s}
          index={i}
          saved={!isNew && savedServers.has(s.id)}
          keyInput={keys[s.id] ?? { value: "", clear: false }}
          insecureForbidden={insecureForbidden}
          busy={busy}
          testing={testing === s.id}
          report={reports[s.id]}
          canRemove={profile.servers.length > 1}
          onChange={(patch) => updateServer(s.id, patch)}
          onKey={(patch) => setKey(s.id, patch)}
          onCaMode={(mode) => {
            if (s.ca.mode === "pem") {
              const pem = s.ca.pem;
              setLastPem((m) => ({ ...m, [s.id]: pem }));
            }
            updateServer(s.id, { ca: withCaMode(s.ca, mode, lastPem[s.id]) });
          }}
          onProxyMode={(mode) => {
            if (s.proxy.mode === "url") {
              const url = s.proxy.url;
              setLastProxy((m) => ({ ...m, [s.id]: url }));
            }
            updateServer(s.id, { proxy: withProxyMode(s.proxy, mode, lastProxy[s.id]) });
          }}
          onTest={() => onTest(s.id)}
          onRemove={() => removeServer(s.id)}
        />
      ))}
      <div className="toolbar">
        <button type="button" disabled={busy} onClick={addServer}>
          {t("installations.form.addServer")}
        </button>
      </div>

      <div className="toolbar">
        <button type="button" className="primary" disabled={busy} onClick={onSave}>
          {t("common.save")}
        </button>
        {!isNew &&
          (confirmingDelete ? (
            <span role="group" aria-label={t("installations.form.deleteGroup")}>
              <strong>{t("installations.form.deleteConfirm", { name: profile.name })}</strong>{" "}
              <button type="button" className="danger" disabled={busy} onClick={onDelete}>
                {t("installations.form.deleteYes")}
              </button>{" "}
              <button type="button" disabled={busy} onClick={() => setConfirmingDelete(false)}>
                {t("common.cancel")}
              </button>
            </span>
          ) : (
            <button type="button" className="danger" disabled={busy} onClick={() => setConfirmingDelete(true)}>
              {t("common.delete")}
            </button>
          ))}
        {busy && <span className="muted">{t("installations.form.busy")}</span>}
      </div>

      {error && <ErrorBanner error={error} />}
    </div>
  );
}

function ServerFields({
  server: s,
  index,
  saved,
  keyInput,
  insecureForbidden,
  busy,
  testing,
  report,
  canRemove,
  onChange,
  onKey,
  onCaMode,
  onProxyMode,
  onTest,
  onRemove,
}: {
  server: ServerConfig;
  index: number;
  /** Сервер уже сохранён: у него может быть ключ в keychain. */
  saved: boolean;
  keyInput: KeyInput;
  insecureForbidden: boolean;
  busy: boolean;
  testing: boolean;
  report?: TestReport;
  canRemove: boolean;
  onChange: (patch: Partial<ServerConfig>) => void;
  onKey: (patch: Partial<KeyInput>) => void;
  onCaMode: (mode: CaMode) => void;
  onProxyMode: (mode: ProxyMode) => void;
  onTest: () => void;
  onRemove: () => void;
}) {
  const { t } = useTranslation();
  const id = (field: string) => `if-s${index}-${field}`;
  const keyGoesPlain = isPlainHttp(s.baseUrl) && !keyInput.clear && (keyInput.value.trim() !== "" || saved);
  return (
    <fieldset className="server" aria-label={t("installations.form.server", { n: index + 1 })}>
      <legend>{s.name.trim() || t("installations.form.server", { n: index + 1 })}</legend>
      <div className="form">
        <label htmlFor={id("name")}>{t("installations.form.serverName")}</label>
        <input id={id("name")} placeholder="vllm-1" value={s.name} onChange={(e) => onChange({ name: e.target.value })} />

        <span className="field-label">
          <label htmlFor={id("url")}>{t("installations.form.baseUrl")}</label>
          <Hint k="hints.installations.baseUrl" label={t("installations.form.baseUrl")} />
        </span>
        <input id={id("url")} placeholder="http://vllm.internal:8000" value={s.baseUrl} onChange={(e) => onChange({ baseUrl: e.target.value })} />

        <span className="field-label">
          <label htmlFor={id("kind")}>{t("installations.form.kind")}</label>
          <Hint k="hints.installations.kind" label={t("installations.form.kind")} />
        </span>
        <select id={id("kind")} value={s.kind} onChange={(e) => onChange({ kind: e.target.value as ServerKind })}>
          {KINDS.map((k) => (
            <option key={k} value={k}>
              {t(`installations.form.kinds.${k}`)}
            </option>
          ))}
        </select>

        <span className="field-label">
          <label htmlFor={id("key")}>{t("installations.form.apiKey")}</label>
          <Hint k="hints.installations.apiKey" label={t("installations.form.apiKey")} />
        </span>
        <input
          id={id("key")}
          type="password"
          autoComplete="off"
          placeholder={saved ? t("installations.form.apiKeyKeep") : ""}
          disabled={keyInput.clear}
          value={keyInput.value}
          onChange={(e) => onKey({ value: e.target.value })}
        />
        {saved && (
          <label className="inline full">
            <input type="checkbox" checked={keyInput.clear} onChange={(e) => onKey({ clear: e.target.checked })} />{" "}
            {t("installations.form.apiKeyClear")}
          </label>
        )}
        {keyGoesPlain && (
          <p className="warn full" role="alert">
            {t("installations.form.apiKeyOverHttp")}
          </p>
        )}

        <span className="field-label">
          <label htmlFor={id("metrics")}>{t("installations.form.metricsPath")}</label>
          <Hint k="hints.installations.metricsPath" label={t("installations.form.metricsPath")} />
        </span>
        <input id={id("metrics")} value={s.metricsPath} onChange={(e) => onChange({ metricsPath: e.target.value })} />

        <span className="field-label">
          <label htmlFor={id("ca")}>{t("installations.form.ca")}</label>
          <Hint k="hints.installations.ca" label={t("installations.form.ca")} />
        </span>
        <select id={id("ca")} value={s.ca.mode} onChange={(e) => onCaMode(e.target.value as CaMode)}>
          {CA_MODES.map((m) => (
            <option key={m} value={m} disabled={m === "insecure" && insecureForbidden && s.ca.mode !== "insecure"}>
              {t(`installations.form.caMode.${m}`)}
            </option>
          ))}
        </select>
        {s.ca.mode === "pem" && (
          <textarea
            className="full"
            aria-label={t("installations.form.caPem")}
            rows={4}
            placeholder="-----BEGIN CERTIFICATE-----"
            value={s.ca.pem}
            onChange={(e) => onChange({ ca: { mode: "pem", pem: e.target.value } })}
          />
        )}
        {s.ca.mode === "insecure" && (
          <p className="warn full" role="alert">
            {insecureForbidden ? t("installations.form.insecureForbidden") : t("installations.form.insecureWarning")}
          </p>
        )}

        <span className="field-label">
          <label htmlFor={id("proxy")}>{t("installations.form.proxy")}</label>
          <Hint k="hints.installations.proxy" label={t("installations.form.proxy")} />
        </span>
        <select id={id("proxy")} value={s.proxy.mode} onChange={(e) => onProxyMode(e.target.value as ProxyMode)}>
          {PROXY_MODES.map((m) => (
            <option key={m} value={m}>
              {t(`installations.form.proxyMode.${m}`)}
            </option>
          ))}
        </select>
        {s.proxy.mode === "url" && (
          <input
            className="full"
            aria-label={t("installations.form.proxyUrl")}
            placeholder="http://proxy.example.com:3128"
            value={s.proxy.url}
            onChange={(e) => onChange({ proxy: { mode: "url", url: e.target.value } })}
          />
        )}
      </div>
      <div className="toolbar">
        <button type="button" disabled={busy} onClick={onTest}>
          {t("installations.form.test")}
        </button>
        <Hint k="hints.installations.test" label={t("installations.form.test")} />
        {canRemove && (
          <button type="button" disabled={busy} onClick={onRemove}>
            {t("installations.form.removeServer")}
          </button>
        )}
        {testing && <span className="muted">{t("installations.form.testing")}</span>}
      </div>
      {report && (
        <ul className="steps" aria-label={t("installations.form.report")}>
          {report.steps.map((step) => (
            <li key={step.name}>
              {step.ok ? "✅" : "❌"} <strong>{t(`installations.form.step.${step.name}`)}:</strong> <span>{tr(step.message)}</span>
              {step.error?.details && <pre className="muted">{step.error.details}</pre>}
            </li>
          ))}
          {report.warnings.map((w) => (
            <li key={w.key} className="warn">
              ⚠ {tr(w)}
            </li>
          ))}
        </ul>
      )}
    </fieldset>
  );
}
````

<!-- block: file t11-src-components-installations-installationformtab-tsx step=11.4 path=src/components/installations/InstallationFormTab.tsx -->
````tsx
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { api } from "../../api/client";
import { ErrorBanner } from "../common/ErrorBanner";
import { InstallationForm } from "./InstallationForm";
import { newInstallation } from "./profile";

export function InstallationFormTab({
  installationId,
  folderId = null,
  tabId,
}: {
  installationId: string | null;
  /** Папка новой инсталляции; новая папка пересоздаёт форму (вкладка «новая инсталляция» одна). */
  folderId?: string | null;
  tabId: string;
}) {
  const { t } = useTranslation();
  const q = useQuery({ queryKey: ["installations"], queryFn: api.listInstallations });
  if (installationId === null)
    return <InstallationForm key={`new:${folderId ?? ""}`} initial={{ ...newInstallation(), folderId }} tabId={tabId} />;
  if (q.isPending) return <p className="empty">{t("common.loading")}</p>;
  if (q.error) return <ErrorBanner error={q.error} onRetry={() => q.refetch()} />;
  const profile = q.data.find((c) => c.id === installationId);
  if (!profile) return <p className="empty">{t("installations.form.deleted")}</p>;
  return <InstallationForm key={profile.id} initial={profile} tabId={tabId} />;
}
````

<!-- block: file t11-src-components-installations-profile-ts step=11.4 path=src/components/installations/profile.ts -->
````ts
import type { CaConfig, InstallationProfile, ProxyConfig, ServerConfig } from "../../api/types";

export type CaMode = CaConfig["mode"];
export type ProxyMode = ProxyConfig["mode"];

/**
 * Идентификатор нового сервера: часть ключа keychain, поэтому только латиница, цифры и «-».
 * `crypto.getRandomValues`, а не `randomUUID`: тот появился только в Safari 15.4, а нужен WebKit macOS 11.
 */
export function newServerId(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(6));
  return `s-${Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("")}`;
}

export function newServer(): ServerConfig {
  return {
    id: newServerId(),
    name: "",
    baseUrl: "",
    kind: "auto",
    metricsPath: "/metrics",
    ca: { mode: "system" },
    proxy: { mode: "system" },
  };
}

export function newInstallation(): InstallationProfile {
  return {
    id: "",
    name: "",
    color: "#3b82f6",
    environment: "dev",
    readOnly: false,
    servers: [newServer()],
    requestTimeoutMs: 30000,
    connectTimeoutMs: 10000,
    folderId: null,
    order: 0,
  };
}

/** Режим CA; введённый PEM сохраняется при переключении туда и обратно. */
export function withCaMode(ca: CaConfig, mode: CaMode, lastPem = ""): CaConfig {
  if (mode === "pem") return { mode, pem: ca.mode === "pem" ? ca.pem : lastPem };
  return { mode };
}

/** Режим прокси; введённый адрес сохраняется при переключении туда и обратно. */
export function withProxyMode(proxy: ProxyConfig, mode: ProxyMode, lastUrl = ""): ProxyConfig {
  if (mode === "url") return { mode, url: proxy.mode === "url" ? proxy.url : lastUrl };
  return { mode };
}

/** Адрес без TLS: ключ API уходит по нему открытым текстом. */
export function isPlainHttp(baseUrl: string): boolean {
  return /^\s*http:\/\//i.test(baseUrl);
}
````

<!-- block: file t11-src-state-workspace-ts step=11.4 path=src/state/workspace.ts -->
````ts
import { create } from "zustand";

export type TabSpec =
  /** Инсталляция: серверы, их тип, версия, модели и состояние. */
  | { kind: "installation"; installationId: string }
  /** folderId — папка новой инсталляции («Новая инсталляция здесь»). */
  | { kind: "installation-form"; installationId: string | null; folderId?: string | null }
  | { kind: "about" }
  /** Справка: глава и раздел руководства (F1, ссылки). */
  | { kind: "help"; chapter?: string; anchor?: string }
  | { kind: "settings" };

export type Tab = TabSpec & { id: string };

export function tabId(spec: TabSpec): string {
  switch (spec.kind) {
    case "installation-form":
      return `installation-form:${spec.installationId ?? "new"}`;
    case "installation":
      return `installation:${spec.installationId}`;
    default:
      return spec.kind;
  }
}

/** Инсталляция вкладки или `null`, если вкладка не про инсталляцию. */
export function tabInstallation(tab: TabSpec): string | null {
  return "installationId" in tab ? tab.installationId : null;
}

interface WorkspaceState {
  tabs: Tab[];
  activeId: string | null;
  open: (spec: TabSpec) => void;
  close: (id: string) => void;
  /** Закрывает все вкладки инсталляции (например, после её удаления). */
  closeForInstallation: (installationId: string) => void;
  activate: (id: string) => void;
}

export const useWorkspace = create<WorkspaceState>((set) => ({
  tabs: [],
  activeId: null,
  open: (spec) =>
    set((s) => {
      const id = tabId(spec);
      const tab = { ...spec, id } as Tab;
      // Уже открытая вкладка получает новый spec (например, главу справки).
      if (s.tabs.some((t) => t.id === id)) return { tabs: s.tabs.map((t) => (t.id === id ? tab : t)), activeId: id };
      return { tabs: [...s.tabs, tab], activeId: id };
    }),
  close: (id) =>
    set((s) => {
      const idx = s.tabs.findIndex((t) => t.id === id);
      if (idx < 0) return s;
      const tabs = s.tabs.filter((t) => t.id !== id);
      const activeId = s.activeId !== id ? s.activeId : (tabs[idx] ?? tabs[idx - 1])?.id ?? null;
      return { tabs, activeId };
    }),
  closeForInstallation: (installationId) =>
    set((s) => {
      const firstIdx = s.tabs.findIndex((t) => tabInstallation(t) === installationId);
      if (firstIdx < 0) return s;
      const tabs = s.tabs.filter((t) => tabInstallation(t) !== installationId);
      const activeId = tabs.some((t) => t.id === s.activeId)
        ? s.activeId
        : (tabs[firstIdx] ?? tabs[tabs.length - 1])?.id ?? null;
      return { tabs, activeId };
    }),
  activate: (id) => set({ activeId: id }),
}));
````

<!-- block: patch t11-src step=11.4 -->
````diff
diff --git a/PORTED.md b/PORTED.md
index cf47234..7c85b14 100644
--- a/PORTED.md
+++ b/PORTED.md
@@ -94,3 +94,8 @@
 | `src/components/common/Hint.test.tsx` | `src/components/common/Hint.test.tsx` | `468e08a` | как есть |
 | `src/test/render.tsx` | `src/test/render.tsx` | `468e08a` | как есть |
 | `src/test/setup.ts` | `src/test/setup.ts` | `468e08a` | как есть |
+| `src/components/installations/tree/folderTree.ts` | `src/components/connections/tree/folderTree.ts` | `468e08a` | адаптирован: инсталляции вместо подключений |
+| `src/components/installations/tree/folderTree.test.ts` | `src/components/connections/tree/folderTree.test.ts` | `468e08a` | адаптирован: инсталляции вместо подключений |
+| `src/state/workspace.ts` | `src/state/workspace.ts` | `468e08a` | адаптирован: вкладки инсталляций вместо вкладок кластера |
+| `src/state/workspace.test.ts` | `src/state/workspace.test.ts` | `468e08a` | адаптирован: вкладки инсталляций |
+| `src/components/installations/InstallationFormTab.tsx` | `src/components/connections/ConnectionFormTab.tsx` | `468e08a` | адаптирован: инсталляция вместо подключения |
diff --git a/docs/i18n/same-as-en.json b/docs/i18n/same-as-en.json
index aa2ac29..6bdc309 100644
--- a/docs/i18n/same-as-en.json
+++ b/docs/i18n/same-as-en.json
@@ -1,6 +1,10 @@
 {
   "ru": [
     "errors.generic",
+    "installations.form.kinds.sglang",
+    "installations.form.kinds.vllm",
+    "installations.form.step.tcp",
+    "installations.form.step.tls",
     "messages.connectionTest.kind",
     "messages.connectionTest.kindVersion"
   ]
diff --git a/src/api/types.ts b/src/api/types.ts
index 4b23436..fea32c7 100644
--- a/src/api/types.ts
+++ b/src/api/types.ts
@@ -62,7 +62,8 @@ export interface InstallationProfile {
   requestTimeoutMs: number;
   connectTimeoutMs: number;
   folderId: string | null;
-  order: number;
+  /** Место в папке; у записей без него — в конце уровня. */
+  order?: number;
 }
 
 /** Ключи API из формы: id сервера → значение; сервера нет — оставить сохранённый, "" — удалить. */
diff --git a/src/main.tsx b/src/main.tsx
index b250a45..67c6f14 100644
--- a/src/main.tsx
+++ b/src/main.tsx
@@ -4,6 +4,7 @@ import React from "react";
 import ReactDOM from "react-dom/client";
 import App from "./App";
 import "./styles.css";
+import "./app.css";
 import { installPlainInputs } from "./lib/plainInputs";
 
 installPlainInputs();
````

<!-- block: locales t11-locales step=11.4 -->
````json
{
  "en": {
    "installations": {
      "form": {
        "titleNew": "New installation",
        "titleEdit": "Installation “{{name}}”",
        "name": "Name",
        "folder": "Folder",
        "folderRoot": "— root —",
        "environment": "Environment",
        "color": "Color",
        "requestTimeout": "Request timeout, ms",
        "connectTimeout": "Connect timeout, ms",
        "servers": "Inference servers",
        "server": "Server {{n}}",
        "serverName": "Server name",
        "baseUrl": "Address",
        "kind": "Server type",
        "kinds": {
          "auto": "Detect automatically",
          "vllm": "vLLM",
          "sglang": "SGLang",
          "openai": "OpenAI-compatible"
        },
        "apiKey": "API key",
        "apiKeyKeep": "leave empty to keep the saved key",
        "apiKeyClear": "Delete the saved key",
        "apiKeyOverHttp": "The address is http:// — the API key will be sent unencrypted",
        "metricsPath": "Metrics path",
        "ca": "Certificate verification",
        "caMode": {
          "system": "System store",
          "pem": "Listed certificates (PEM)",
          "insecure": "No verification (insecure)"
        },
        "caPem": "CA certificates (PEM)",
        "insecureWarning": "The server certificate is not verified: anyone on the way can read the API key and the traffic",
        "insecureForbidden": "Servers without certificate verification are forbidden by your organization",
        "proxy": "Proxy",
        "proxyMode": {
          "system": "System (HTTPS_PROXY)",
          "none": "No proxy",
          "url": "Proxy address"
        },
        "proxyUrl": "Proxy address",
        "test": "Check the server",
        "testing": "Checking…",
        "report": "Check result",
        "removeServer": "Remove the server",
        "addServer": "Add a server",
        "busy": "Please wait…",
        "deleted": "This installation has been deleted",
        "deleteGroup": "Delete confirmation",
        "deleteConfirm": "Delete “{{name}}” and the API keys of its servers?",
        "deleteYes": "Delete",
        "step": {
          "config": "Configuration",
          "tcp": "DNS/TCP",
          "tls": "TLS",
          "health": "Health",
          "auth": "Sign-in",
          "kind": "Type and version",
          "models": "Models"
        }
      }
    },
    "hints": {
      "installations": {
        "environment": "Color and protection: a new <code>prod</code> installation is created read-only.",
        "readOnly": "Glafira will not run anything that loads or changes the servers of this installation.",
        "baseUrl": "The OpenAI-compatible API without /v1: <code>http://vllm.internal:8000</code>. A path is allowed behind a reverse proxy.",
        "kind": "Glafira recognizes vLLM and SGLang by their answers. Choose by hand only if detection is wrong; other servers work as OpenAI-compatible.",
        "apiKey": "The <code>--api-key</code> of the server. Kept in the system secret store, not in a file; sent as <code>Authorization: Bearer</code>.",
        "metricsPath": "Where the server exposes Prometheus metrics: <code>/metrics</code>; SGLang needs <code>--enable-metrics</code>.",
        "ca": "System store — the CAs this computer trusts. Listed certificates — only them, the system ones are not used.",
        "proxy": "System — the <code>HTTPS_PROXY</code> and <code>NO_PROXY</code> variables. Or an explicit address such as <code>http://proxy:3128</code>.",
        "test": "Checks this server step by step with the values of the form, without saving: address, TLS, /health, API key, type, version, models."
      }
    }
  },
  "ru": {
    "installations": {
      "form": {
        "titleNew": "Новая инсталляция",
        "titleEdit": "Инсталляция «{{name}}»",
        "name": "Имя",
        "folder": "Папка",
        "folderRoot": "— корень —",
        "environment": "Окружение",
        "color": "Цвет",
        "requestTimeout": "Таймаут запроса, мс",
        "connectTimeout": "Таймаут подключения, мс",
        "servers": "Серверы инференса",
        "server": "Сервер {{n}}",
        "serverName": "Имя сервера",
        "baseUrl": "Адрес",
        "kind": "Тип сервера",
        "kinds": {
          "auto": "Определить автоматически",
          "vllm": "vLLM",
          "sglang": "SGLang",
          "openai": "OpenAI-совместимый"
        },
        "apiKey": "Ключ API",
        "apiKeyKeep": "оставьте пустым, чтобы не менять сохранённый",
        "apiKeyClear": "Удалить сохранённый ключ",
        "apiKeyOverHttp": "Адрес — http://: ключ API уйдёт открытым текстом",
        "metricsPath": "Путь метрик",
        "ca": "Проверка сертификата",
        "caMode": {
          "system": "Системное хранилище",
          "pem": "Перечисленные сертификаты (PEM)",
          "insecure": "Без проверки (небезопасно)"
        },
        "caPem": "CA-сертификаты (PEM)",
        "insecureWarning": "Сертификат сервера не проверяется: ключ API и трафик может прочитать любой на пути",
        "insecureForbidden": "Серверы без проверки сертификата запрещены вашей организацией",
        "proxy": "Прокси",
        "proxyMode": {
          "system": "Системный (HTTPS_PROXY)",
          "none": "Без прокси",
          "url": "Адрес прокси"
        },
        "proxyUrl": "Адрес прокси",
        "test": "Проверить сервер",
        "testing": "Проверка…",
        "report": "Результат проверки",
        "removeServer": "Убрать сервер",
        "addServer": "Добавить сервер",
        "busy": "Подождите…",
        "deleted": "Инсталляция удалена",
        "deleteGroup": "Подтверждение удаления",
        "deleteConfirm": "Удалить «{{name}}» и ключи API её серверов?",
        "deleteYes": "Удалить",
        "step": {
          "config": "Конфигурация",
          "tcp": "DNS/TCP",
          "tls": "TLS",
          "health": "Здоровье",
          "auth": "Вход",
          "kind": "Тип и версия",
          "models": "Модели"
        }
      }
    },
    "hints": {
      "installations": {
        "environment": "Цвет и защита: новая инсталляция <code>prod</code> создаётся только для чтения.",
        "readOnly": "Glafira не запустит ничего, что нагружает или меняет серверы этой инсталляции.",
        "baseUrl": "OpenAI-совместимый API без /v1: <code>http://vllm.internal:8000</code>. Путь допускается, если сервер за обратным прокси.",
        "kind": "vLLM и SGLang Glafira узнаёт по ответам. Выбирайте вручную, только если тип определился неверно; прочие серверы — OpenAI-совместимые.",
        "apiKey": "<code>--api-key</code> сервера. Хранится в системном хранилище секретов, а не в файле; уходит заголовком <code>Authorization: Bearer</code>.",
        "metricsPath": "Где сервер отдаёт метрики Prometheus: <code>/metrics</code>; SGLang нужен <code>--enable-metrics</code>.",
        "ca": "Системное хранилище — CA, которым доверяет компьютер. Перечисленные сертификаты — только они, системные не используются.",
        "proxy": "Системный — переменные <code>HTTPS_PROXY</code> и <code>NO_PROXY</code>. Или явный адрес вида <code>http://proxy:3128</code>.",
        "test": "Проверяет этот сервер по шагам со значениями формы, без сохранения: адрес, TLS, /health, ключ API, тип, версию, модели."
      }
    }
  }
}
````

- [ ] **Step 5: Проверки.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib`  
Expected: PASS — `test result: ok. 161 passed; 0 failed`

Run: `pnpm test`  
Expected: PASS — `Test Files 16 passed`, `Tests 70 passed`

Run: `pnpm exec tsc --noEmit`  
Expected: PASS — без ошибок

Run: `pnpm i18n:check`  
Expected: PASS — `✓ словари en, ru согласованы, вызовов из Rust: 95`

Run: `pnpm build`  
Expected: PASS — `✓ dist/assets: чанки совместимы с WebKit macOS 11+`

- [ ] **Step 6: Commit.**

```bash
git add \
  PORTED.md \
  docs/i18n/same-as-en.json \
  src/api/types.ts \
  src/app.css \
  src/components/installations/InstallationForm.test.tsx \
  src/components/installations/InstallationForm.tsx \
  src/components/installations/InstallationFormTab.tsx \
  src/components/installations/profile.test.ts \
  src/components/installations/profile.ts \
  src/components/installations/tree/folderTree.test.ts \
  src/components/installations/tree/folderTree.ts \
  src/locales/en.json \
  src/locales/ru.json \
  src/main.tsx \
  src/state/workspace.test.ts \
  src/state/workspace.ts
git commit -s -m "feat(ui): installation form — servers with type, API key, CA and proxy, a step-by-step check of each server" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 12: Вкладка инсталляции: тип, версия, модели, метрики и ошибки серверов, подключение заново

**Files:**
- Create: `src/components/installations/InstallationPanel.test.tsx`, `src/components/installations/InstallationPanel.tsx`
- Modify: `src/app.css`, `src/locales/en.json`, `src/locales/ru.json`

**Interfaces:**
- Consumes: задачи 10–11: `api.connectInstallation`, `useWorkspace`.
- Produces: `InstallationPanel`, `installationQueryKey(id) = ["installation", id]`, `modelLabel(model)`.

При открытии вкладка подключается ко всем серверам. Карточка сервера: адрес, тип и версия (и «по признакам — …», если тип выбран вручную или версия старая), модели с длиной контекста и базовой моделью адаптера LoRA, отдаёт ли метрики, ошибка сервера, предупреждения. «Подключиться заново» — свежее подключение (`fresh`).

- [ ] **Step 1: Тесты.**

<!-- block: file t12-src-components-installations-installationpanel-test-tsx step=12.1 path=src/components/installations/InstallationPanel.test.tsx -->
````tsx
import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { renderWithClient } from "../../test/render";

vi.mock("../../api/client", async (importOriginal) => {
  const orig = await importOriginal<typeof import("../../api/client")>();
  return { ...orig, api: { listInstallations: vi.fn(), connectInstallation: vi.fn() } };
});

import { api } from "../../api/client";
import type { InstallationInfo, InstallationProfile } from "../../api/types";
import { useWorkspace } from "../../state/workspace";
import { InstallationPanel, modelLabel } from "./InstallationPanel";
import { newInstallation } from "./profile";

const msg = (key: string, text: string, params: Record<string, string> = {}) => ({ key, params, text });

const PROFILE: InstallationProfile = {
  ...newInstallation(),
  id: "i1",
  name: "prod-llm",
  environment: "prod",
  readOnly: true,
  servers: [
    { id: "s1", name: "gpu-1", baseUrl: "http://gpu-1:8000", kind: "auto", metricsPath: "/metrics", ca: { mode: "system" }, proxy: { mode: "system" } },
    { id: "s2", name: "gpu-2", baseUrl: "http://gpu-2:8000", kind: "auto", metricsPath: "/metrics", ca: { mode: "system" }, proxy: { mode: "system" } },
  ],
};

const INFO: InstallationInfo = {
  installationId: "i1",
  servers: [
    {
      serverId: "s1",
      facts: {
        kind: "vllm",
        detected: "vllm",
        version: "0.11.0",
        models: [
          { id: "llama-3.1-8b", maxModelLen: 131072 },
          { id: "sql-lora", parent: "llama-3.1-8b" },
        ],
        metrics: true,
        warnings: [],
      },
    },
    {
      serverId: "s2",
      error: { code: "AUTH_FAILED", key: "errors.server.unauthorized", params: { status: "401" }, message: "rejected", retryable: false },
    },
  ],
};

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.listInstallations).mockResolvedValue([PROFILE]);
  vi.mocked(api.connectInstallation).mockResolvedValue(INFO);
  useWorkspace.setState({ tabs: [], activeId: null });
});

describe("InstallationPanel", () => {
  it("показывает серверы: тип, версию, модели и метрики или ошибку сервера", async () => {
    renderWithClient(<InstallationPanel installationId="i1" />);
    const first = await screen.findByRole("region", { name: "gpu-1" });
    expect(within(first).getByText("vLLM 0.11.0")).toBeInTheDocument();
    expect(within(first).getByText("llama-3.1-8b · 131072")).toBeInTheDocument();
    expect(within(first).getByText("sql-lora → llama-3.1-8b")).toBeInTheDocument();
    expect(within(first).getByText("отдаёт")).toBeInTheDocument();
    const second = screen.getByRole("region", { name: "gpu-2" });
    expect(within(second).getByText("Сервер отклонил запрос (401) — проверьте ключ API")).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("Подключено серверов: 1 из 2");
    expect(api.connectInstallation).toHaveBeenCalledWith("i1", false);
  });

  it("предупреждения и тип, выбранный вручную", async () => {
    vi.mocked(api.connectInstallation).mockResolvedValue({
      installationId: "i1",
      servers: [
        {
          serverId: "s1",
          facts: {
            kind: "openai",
            detected: "vllm",
            version: null,
            models: [],
            metrics: false,
            warnings: [msg("messages.servers.unsupportedVersion", "old", { kind: "vLLM", version: "0.7.3", min: "0.8" })],
          },
        },
      ],
    });
    renderWithClient(<InstallationPanel installationId="i1" />);
    const first = await screen.findByRole("region", { name: "gpu-1" });
    expect(within(first).getByText("OpenAI")).toBeInTheDocument();
    expect(within(first).getByText("по признакам — vLLM")).toBeInTheDocument();
    expect(within(first).getByText("моделей нет")).toBeInTheDocument();
    expect(within(first).getByText(/vLLM 0\.7\.3 старше 0\.8/)).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "gpu-2" })).getByText("Не подключался")).toBeInTheDocument();
  });

  it("«Подключиться заново» — свежее подключение; «Изменить» открывает форму", async () => {
    const user = userEvent.setup();
    renderWithClient(<InstallationPanel installationId="i1" />);
    await screen.findByRole("region", { name: "gpu-1" });
    await user.click(screen.getByRole("button", { name: "Подключиться заново" }));
    await waitFor(() => expect(api.connectInstallation).toHaveBeenLastCalledWith("i1", true));
    await user.click(screen.getByRole("button", { name: "Изменить" }));
    expect(useWorkspace.getState().activeId).toBe("installation-form:i1");
  });

  it("ошибка подключения целиком — баннер с повтором", async () => {
    vi.mocked(api.connectInstallation).mockRejectedValueOnce({ code: "NOT_FOUND", key: "errors.installations.notFound", params: { id: "i1" }, message: "nf", retryable: false });
    const user = userEvent.setup();
    renderWithClient(<InstallationPanel installationId="i1" />);
    expect(await screen.findByText("Инсталляция i1 не найдена")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Повторить" }));
    expect(await screen.findByRole("region", { name: "gpu-1" })).toBeInTheDocument();
  });

  it("удалённая инсталляция", async () => {
    vi.mocked(api.listInstallations).mockResolvedValue([]);
    renderWithClient(<InstallationPanel installationId="i1" />);
    expect(await screen.findByText("Инсталляция удалена")).toBeInTheDocument();
  });

  it("подпись модели", () => {
    expect(modelLabel({ id: "m" })).toBe("m");
    expect(modelLabel({ id: "m", maxModelLen: 4096 })).toBe("m · 4096");
    expect(modelLabel({ id: "a", parent: "b", maxModelLen: 8 })).toBe("a → b · 8");
  });
});
````

- [ ] **Step 2: Увидеть падение.**

Run: `pnpm vitest run src/components/installations/InstallationPanel.test.tsx`  
Expected: FAIL — нет модуля `./InstallationPanel`.

- [ ] **Step 3: Реализация.**

<!-- block: file t12-src-components-installations-installationpanel-tsx step=12.3 path=src/components/installations/InstallationPanel.tsx -->
````tsx
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { api, toAppError } from "../../api/client";
import type { Model, ServerConfig, ServerFacts, ServerState } from "../../api/types";
import { tr } from "../../i18n/text";
import { useWorkspace } from "../../state/workspace";
import { EnvBadge } from "../common/EnvBadge";
import { ErrorBanner } from "../common/ErrorBanner";
import { Hint } from "../common/Hint";

const KIND_LABELS: Record<ServerFacts["kind"], string> = { vllm: "vLLM", sglang: "SGLang", openai: "OpenAI" };

/** Ключ запроса сведений о подключённой инсталляции. */
export const installationQueryKey = (id: string) => ["installation", id] as const;

/** Вкладка инсталляции: подключается при открытии и показывает серверы — тип, версию, модели, метрики, ошибки. */
export function InstallationPanel({ installationId }: { installationId: string }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const open = useWorkspace((s) => s.open);
  const installations = useQuery({ queryKey: ["installations"], queryFn: api.listInstallations });
  const q = useQuery({ queryKey: installationQueryKey(installationId), queryFn: () => api.connectInstallation(installationId, false) });
  const profile = installations.data?.find((c) => c.id === installationId);

  if (!profile) return <p className="empty">{installations.isPending ? t("common.loading") : t("installations.form.deleted")}</p>;

  // Подключение заново: свежие сведения о серверах (после перезапуска сервера, смены модели).
  const reconnect = () =>
    void queryClient
      .fetchQuery({ queryKey: installationQueryKey(installationId), queryFn: () => api.connectInstallation(installationId, true), staleTime: 0 })
      .catch(() => undefined);

  const states = new Map((q.data?.servers ?? []).map((s) => [s.serverId, s]));
  const connected = q.data?.servers.filter((s) => s.facts).length ?? 0;

  return (
    <div className="page">
      <div className="toolbar">
        <h2>{profile.name}</h2>
        <EnvBadge environment={profile.environment} readOnly={profile.readOnly} />
        {q.data && (
          <span className="muted" role="status">
            {t("installations.panel.summary", { connected, total: profile.servers.length })}
          </span>
        )}
        <button type="button" disabled={q.isFetching} onClick={reconnect}>
          {t("installations.panel.reconnect")}
        </button>
        <button type="button" onClick={() => open({ kind: "installation-form", installationId })}>
          {t("installations.panel.edit")}
        </button>
      </div>
      {q.isPending && <p className="muted">{t("installations.panel.connecting")}</p>}
      {q.error && <ErrorBanner error={toAppError(q.error)} onRetry={() => void q.refetch()} />}
      {q.data && (
        <div className="cards">
          {profile.servers.map((s) => (
            <ServerCard key={s.id} server={s} state={states.get(s.id)} />
          ))}
        </div>
      )}
    </div>
  );
}

function ServerCard({ server, state }: { server: ServerConfig; state?: ServerState }) {
  const { t } = useTranslation();
  const facts = state?.facts;
  return (
    <section className="card" aria-label={server.name}>
      <h4>
        {facts ? "✅" : "❌"} {server.name}
      </h4>
      <dl className="facts">
        <dt>{t("installations.panel.address")}</dt>
        <dd>{server.baseUrl}</dd>
        {facts && <ServerFactsRows facts={facts} />}
      </dl>
      {state?.error && <ErrorBanner error={state.error} />}
      {!state && <p className="muted">{t("installations.panel.notConnected")}</p>}
      {facts && facts.warnings.length > 0 && (
        <ul className="steps">
          {facts.warnings.map((w) => (
            <li key={w.key} className="warn">
              ⚠ {tr(w)}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function ServerFactsRows({ facts }: { facts: ServerFacts }) {
  const { t } = useTranslation();
  const kind = KIND_LABELS[facts.kind];
  return (
    <>
      <dt>
        <span className="field-label">
          {t("installations.panel.kind")}
          <Hint k="hints.installations.detectedKind" label={t("installations.panel.kind")} />
        </span>
      </dt>
      <dd>
        {facts.version ? `${kind} ${facts.version}` : kind}
        {facts.kind !== facts.detected && <span className="muted"> {t("installations.panel.detectedAs", { kind: KIND_LABELS[facts.detected] })}</span>}
      </dd>
      <dt>{t("installations.panel.models")}</dt>
      <dd>
        {facts.models.length === 0 ? (
          <span className="warn">{t("installations.panel.noModels")}</span>
        ) : (
          <ul className="kv">
            {facts.models.map((m) => (
              <li key={m.id}>
                <code>{modelLabel(m)}</code>
              </li>
            ))}
          </ul>
        )}
      </dd>
      <dt>{t("installations.panel.metrics")}</dt>
      <dd>{facts.metrics ? t("installations.panel.metricsYes") : t("installations.panel.metricsNo")}</dd>
    </>
  );
}

/** Модель с контекстом и базовой моделью адаптера LoRA: `qwen3-32b · 32768`, `sql-lora → llama-3.1-8b`. */
export function modelLabel(m: Model): string {
  const parts = [m.parent ? `${m.id} → ${m.parent}` : m.id];
  if (m.maxModelLen) parts.push(String(m.maxModelLen));
  return parts.join(" · ");
}

````

<!-- block: patch t12-src step=12.3 -->
````diff
diff --git a/src/app.css b/src/app.css
index 3096404..f931f88 100644
--- a/src/app.css
+++ b/src/app.css
@@ -5,3 +5,12 @@ fieldset.server legend { padding: 0 4px; font-weight: 600; }
 .page h3 { margin: 4px 0 0; font-size: 14px; }
 /* Подсказка в пустом поле — курсивом и бледнее: её не путают ни с введённым значением, ни с подписью. */
 input::placeholder, textarea::placeholder { font-style: italic; color: var(--muted); opacity: 0.7; }
+/* Карточки серверов на вкладке инсталляции. */
+.cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 8px; }
+.card { border: 1px solid var(--border); border-radius: 6px; padding: 8px; display: flex; flex-direction: column; gap: 4px; min-width: 0; }
+.card h4 { margin: 0; font-size: 13px; overflow-wrap: anywhere; }
+.facts { display: grid; grid-template-columns: max-content minmax(0, 1fr); gap: 2px 12px; margin: 0; }
+.facts dt { color: var(--muted); }
+.facts dd { margin: 0; overflow-wrap: anywhere; }
+.kv { list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; gap: 4px; }
+.kv code { background: var(--bg-alt); border-radius: 3px; padding: 0 4px; overflow-wrap: anywhere; }
````

<!-- block: locales t12-locales step=12.3 -->
````json
{
  "en": {
    "installations": {
      "panel": {
        "summary": "Servers connected: {{connected}} of {{total}}",
        "reconnect": "Reconnect",
        "edit": "Edit",
        "connecting": "Connecting to the servers…",
        "address": "Address",
        "kind": "Type",
        "detectedAs": "detected as {{kind}}",
        "models": "Models",
        "noModels": "no models",
        "metrics": "Metrics",
        "metricsYes": "exposed",
        "metricsNo": "not exposed",
        "notConnected": "Not connected"
      }
    },
    "hints": {
      "installations": {
        "detectedKind": "The type Glafira works with. If it differs from what the server looks like, the type is chosen in the installation or the server version is too old."
      }
    }
  },
  "ru": {
    "installations": {
      "panel": {
        "summary": "Подключено серверов: {{connected}} из {{total}}",
        "reconnect": "Подключиться заново",
        "edit": "Изменить",
        "connecting": "Подключение к серверам…",
        "address": "Адрес",
        "kind": "Тип",
        "detectedAs": "по признакам — {{kind}}",
        "models": "Модели",
        "noModels": "моделей нет",
        "metrics": "Метрики",
        "metricsYes": "отдаёт",
        "metricsNo": "не отдаёт",
        "notConnected": "Не подключался"
      }
    },
    "hints": {
      "installations": {
        "detectedKind": "Тип, с которым работает Glafira. Если он не совпадает с признаками сервера, тип выбран в инсталляции или версия сервера слишком старая."
      }
    }
  }
}
````

- [ ] **Step 4: Проверки.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib`  
Expected: PASS — `test result: ok. 161 passed; 0 failed`

Run: `pnpm test`  
Expected: PASS — `Test Files 17 passed`, `Tests 76 passed`

Run: `pnpm exec tsc --noEmit`  
Expected: PASS — без ошибок

Run: `pnpm i18n:check`  
Expected: PASS — `✓ словари en, ru согласованы, вызовов из Rust: 95`

Run: `pnpm build`  
Expected: PASS — `✓ dist/assets: чанки совместимы с WebKit macOS 11+`

- [ ] **Step 5: Commit.**

```bash
git add \
  src/app.css \
  src/components/installations/InstallationPanel.test.tsx \
  src/components/installations/InstallationPanel.tsx \
  src/locales/en.json \
  src/locales/ru.json
git commit -s -m "feat(ui): installation tab — servers with type, version, models, metrics and per-server errors, reconnect" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 13: Настройки языка, встроенная справка с главами «Начало работы» и «Инсталляции», «О программе»

**Files:**
- Port: `scripts/guide-publish.d.mts`, `scripts/guide-publish.mjs` (с правками), `scripts/guide-publish.test.mjs`, `src/components/help/HelpPanel.test.tsx` (с правками), `src/components/help/HelpPanel.tsx`, `src/components/settings/SettingsPanel.tsx` (с правками), `src/guide/chapters.test.ts` (с правками), `src/guide/chapters.ts` (с правками), `src/guide/parse.test.ts` (с правками), `src/guide/parse.ts`, `src/state/helpContext.ts`
- Create (по Kubernetes-версии): `src/components/about/AboutPanel.test.tsx` ← `src/components/about/AboutPanel.test.tsx`, `src/components/about/AboutPanel.tsx` ← `src/components/about/AboutPanel.tsx`, `src/components/help/helpChapterFor.test.ts` ← `src/components/help/helpChapterFor.test.ts`, `src/components/help/helpChapterFor.ts` ← `src/components/help/helpChapterFor.ts`, `src/components/settings/SettingsPanel.test.tsx` ← `src/components/settings/SettingsPanel.test.tsx`
- Create: `docs/guide/en/getting-started.md`, `docs/guide/en/installations.md`, `docs/guide/ru/getting-started.md`, `docs/guide/ru/installations.md`
- Modify: `PORTED.md`, `package.json`, `scripts/i18n-check.mjs`, `src/locales/en.json`, `src/locales/ru.json`
- Delete: `docs/guide/en/.gitkeep`, `docs/guide/ru/.gitkeep`

**Interfaces:**
- Consumes: задачи 10–12: `api`, `useWorkspace`, `tabId`.
- Produces: `GUIDE_CHAPTERS = ["getting-started", "installations"]` (`src/guide/chapters.ts` и `scripts/i18n-check.mjs`), `loadGuide`, `parseChapter`, `searchGuide`, `HelpPanel`, `helpChapterFor(tab)`, `useHelpChapter`, `SettingsPanel`, `AboutPanel`; главы `docs/guide/{en,ru}/{getting-started,installations}.md`; `pnpm guide:publish`.

Справка, разбор глав и публикация руководства переносятся; глав две. Настройки — без раздела MCP, «О программе» — без обновлений: и то и другое приходит в I2. Метки `[[ui:…]]` глав ссылаются на ключи словаря, поэтому строки боковой панели, которые нужны главам, добавляются уже здесь.

- [ ] **Step 1: Перенос из Kubernetes-версии.**

Файлы копируются с коммита `468e08a` репозитория `~/projects/glafira-openshift` (`$KUBERNETES_REPO`) и правятся патчами — против перенесённого файла. Строки о них в `PORTED.md` добавляет шаг реализации.

<!-- block: shell t13-port step=13.1 -->
````bash
scripts/port.sh scripts/guide-publish.mjs scripts/guide-publish.mjs
scripts/port.sh scripts/guide-publish.test.mjs scripts/guide-publish.test.mjs
scripts/port.sh scripts/guide-publish.d.mts scripts/guide-publish.d.mts
scripts/port.sh src/guide/chapters.ts src/guide/chapters.ts
scripts/port.sh src/guide/chapters.test.ts src/guide/chapters.test.ts
scripts/port.sh src/guide/parse.ts src/guide/parse.ts
scripts/port.sh src/guide/parse.test.ts src/guide/parse.test.ts
scripts/port.sh src/components/help/HelpPanel.tsx src/components/help/HelpPanel.tsx
scripts/port.sh src/components/help/HelpPanel.test.tsx src/components/help/HelpPanel.test.tsx
scripts/port.sh src/state/helpContext.ts src/state/helpContext.ts
scripts/port.sh src/components/settings/SettingsPanel.tsx src/components/settings/SettingsPanel.tsx
````

<!-- block: patch t13-port-scripts-guide-publish-mjs step=13.1 -->
````diff
diff --git a/scripts/guide-publish.mjs b/scripts/guide-publish.mjs
--- a/scripts/guide-publish.mjs
+++ b/scripts/guide-publish.mjs
@@ -79,7 +79,7 @@ export function publishGuide(root, outDir, chapters = GUIDE_CHAPTERS) {
     toc.push({ code, nativeName, items });
   }
   const readme = [
-    "# Glafira for Kubernetes — User guide",
+    "# Glafira for LLM Inference — User guide",
     "",
     toc.map(({ code, nativeName }) => `[${nativeName}](${code}/getting-started.md)`).join(" · "),
     "",
````

<!-- block: patch t13-port-src-guide-chapters-ts step=13.1 -->
````diff
diff --git a/src/guide/chapters.ts b/src/guide/chapters.ts
--- a/src/guide/chapters.ts
+++ b/src/guide/chapters.ts
@@ -1,5 +1,5 @@
 /** Главы руководства (спец. §16); каждый этап добавляет свои. Тот же список — в `scripts/i18n-check.mjs`. */
-export const GUIDE_CHAPTERS = ["getting-started", "connections", "projects-workloads", "pods-logs", "health-check", "llm-mcp", "updates"] as const;
+export const GUIDE_CHAPTERS = ["getting-started", "installations"] as const;
 
 export type ChapterId = (typeof GUIDE_CHAPTERS)[number];
 
````

<!-- block: patch t13-port-src-guide-chapters-test-ts step=13.1 -->
````diff
diff --git a/src/guide/chapters.test.ts b/src/guide/chapters.test.ts
--- a/src/guide/chapters.test.ts
+++ b/src/guide/chapters.test.ts
@@ -5,7 +5,7 @@ describe("loadGuide", () => {
   it("loads every chapter of a language from docs/guide", async () => {
     const ru = await loadGuide("ru");
     expect(Object.keys(ru).sort()).toEqual([...GUIDE_CHAPTERS].sort());
-    expect(ru.connections).toMatch(/^# Подключения/);
+    expect(ru.installations).toMatch(/^# Инсталляции/);
   });
 
   it("falls back to English for a language without chapters", async () => {
````

<!-- block: patch t13-port-src-guide-parse-test-ts step=13.1 -->
````diff
diff --git a/src/guide/parse.test.ts b/src/guide/parse.test.ts
--- a/src/guide/parse.test.ts
+++ b/src/guide/parse.test.ts
@@ -29,8 +29,8 @@ const TEXT = `# Подключения
 
 describe("parseChapter", () => {
   it("splits sections with anchors and keeps the intro as id null", () => {
-    const ch = parseChapter("connections", TEXT, label);
-    expect(ch.id).toBe("connections");
+    const ch = parseChapter("installations", TEXT, label);
+    expect(ch.id).toBe("installations");
     expect(ch.title).toBe("Подключения");
     expect(ch.sections.map((s) => s.id)).toEqual([null, "plain", "mtls"]);
     expect(ch.sections[0].title).toBe("Подключения");
@@ -41,7 +41,7 @@ describe("parseChapter", () => {
   });
 
   it("replaces ui labels with bold dictionary values", () => {
-    const ch = parseChapter("connections", TEXT, label);
+    const ch = parseChapter("installations", TEXT, label);
     expect(ch.sections[0].markdown).toContain("**Настройки**");
     expect(ch.sections[2].markdown).toContain("**О программе**");
     expect(ch.sections[0].markdown).not.toContain("[[ui:");
@@ -100,12 +100,12 @@ describe("parseChapter", () => {
 });
 
 describe("searchGuide", () => {
-  const chapters = [parseChapter("connections", TEXT, label)];
+  const chapters = [parseChapter("installations", TEXT, label)];
 
   it("finds a section case-insensitively with a snippet", () => {
     const res = searchGuide(chapters, "СЕРТИФИКАТ");
     expect(res).toHaveLength(1);
-    expect(res[0]).toMatchObject({ chapter: "connections", section: "mtls", title: "Взаимный TLS" });
+    expect(res[0]).toMatchObject({ chapter: "installations", section: "mtls", title: "Взаимный TLS" });
     expect(res[0].snippet.toLowerCase()).toContain("сертификат");
   });
 
````

<!-- block: patch t13-port-src-components-help-helppanel-test-tsx step=13.1 -->
````diff
diff --git a/src/components/help/HelpPanel.test.tsx b/src/components/help/HelpPanel.test.tsx
--- a/src/components/help/HelpPanel.test.tsx
+++ b/src/components/help/HelpPanel.test.tsx
@@ -14,8 +14,8 @@ vi.mock("../../guide/chapters", async (importOriginal) => {
     ...actual,
     loadGuide: vi.fn().mockResolvedValue({
       "getting-started":
-        "# Начало работы\n\nОткройте [[ui:sidebar.settings]].\n\n## Первое подключение {#first-connection}\n\nСм. [mTLS](connections.md#mtls), [сайт](https://example.com) и [файл](../admin.md), [плохая](javascript:alert(1)).\n\nЗначение: [[ui:test.tricky]].\n\n![картинка](https://example.com/a.png)\n\n## Без якоря\n\nТекст без якоря.\n",
-      connections: "# Подключения\n\n## Простое {#plain}\n\nАдрес брокера.\n\n## mTLS {#mtls}\n\nНужен сертификат клиента.\n",
+        "# Начало работы\n\nОткройте [[ui:sidebar.settings]].\n\n## Первое подключение {#first-connection}\n\nСм. [mTLS](installations.md#mtls), [сайт](https://example.com) и [файл](../admin.md), [плохая](javascript:alert(1)).\n\nЗначение: [[ui:test.tricky]].\n\n![картинка](https://example.com/a.png)\n\n## Без якоря\n\nТекст без якоря.\n",
+      installations: "# Инсталляции\n\n## Простое {#plain}\n\nАдрес брокера.\n\n## mTLS {#mtls}\n\nНужен сертификат клиента.\n",
     }),
   };
 });
@@ -34,22 +34,22 @@ describe("HelpPanel", () => {
     expect(await screen.findByRole("heading", { level: 1, name: "Начало работы" })).toBeInTheDocument();
     const toc = screen.getByRole("navigation", { name: "Содержание" });
     expect(toc).toHaveTextContent("Начало работы");
-    expect(toc).toHaveTextContent("Подключения");
+    expect(toc).toHaveTextContent("Инсталляции");
     expect(toc).toHaveTextContent("Первое подключение");
     expect(screen.getByRole("heading", { level: 2, name: "Первое подключение" })).toHaveAttribute("id", "first-connection");
     expect(screen.getByText("Настройки").tagName).toBe("STRONG");
   });
 
   it("opens the chapter from the spec", async () => {
-    renderWithClient(<HelpPanel spec={{ chapter: "connections", anchor: "mtls" }} />);
-    expect(await screen.findByRole("heading", { level: 1, name: "Подключения" })).toBeInTheDocument();
+    renderWithClient(<HelpPanel spec={{ chapter: "installations", anchor: "mtls" }} />);
+    expect(await screen.findByRole("heading", { level: 1, name: "Инсталляции" })).toBeInTheDocument();
     await waitFor(() => expect(Element.prototype.scrollIntoView).toHaveBeenCalled());
   });
 
   it("follows an internal link to the chapter and anchor", async () => {
     const { container } = renderWithClient(<HelpPanel spec={EMPTY} />);
     fireEvent.click(await screen.findByRole("link", { name: "mTLS" }));
-    expect(await screen.findByRole("heading", { level: 1, name: "Подключения" })).toBeInTheDocument();
+    expect(await screen.findByRole("heading", { level: 1, name: "Инсталляции" })).toBeInTheDocument();
     const target = container.querySelector('[id="mtls"]');
     expect(target).not.toBeNull();
     await waitFor(() => expect(Element.prototype.scrollIntoView).toHaveBeenCalled());
@@ -89,9 +89,9 @@ describe("HelpPanel", () => {
     await screen.findByRole("heading", { level: 1, name: "Начало работы" });
     const input = screen.getByRole("searchbox", { name: "Поиск по руководству" });
     fireEvent.change(input, { target: { value: "СЕРТИФИКАТ" } });
-    const hit = screen.getByRole("button", { name: /Подключения › mTLS/ });
+    const hit = screen.getByRole("button", { name: /Инсталляции › mTLS/ });
     fireEvent.click(hit);
-    expect(await screen.findByRole("heading", { level: 1, name: "Подключения" })).toBeInTheDocument();
+    expect(await screen.findByRole("heading", { level: 1, name: "Инсталляции" })).toBeInTheDocument();
     fireEvent.change(input, { target: { value: "zzzz" } });
     expect(screen.getByText("Ничего не найдено")).toBeInTheDocument();
   });
````

<!-- block: patch t13-port-src-components-settings-settingspanel-tsx step=13.1 -->
````diff
diff --git a/src/components/settings/SettingsPanel.tsx b/src/components/settings/SettingsPanel.tsx
--- a/src/components/settings/SettingsPanel.tsx
+++ b/src/components/settings/SettingsPanel.tsx
@@ -1,5 +1,4 @@
 import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
-import { useState } from "react";
 import { useTranslation } from "react-i18next";
 import { api } from "../../api/client";
 import { LANGUAGES } from "../../i18n/languages";
@@ -7,7 +6,6 @@ import { i18n, setLanguage } from "../../i18n";
 import { useHelpChapter } from "../../state/helpContext";
 import { tabId } from "../../state/workspace";
 import { ErrorBanner } from "../common/ErrorBanner";
-import { McpSection } from "./McpSection";
 
 export function SettingsPanel() {
   const { t } = useTranslation();
@@ -21,10 +19,8 @@ export function SettingsPanel() {
     },
   });
 
-  // F1: в разделе MCP — глава «LLM-ассистенты», в остальных — «Начало работы» (язык описан там).
-  // Раздел выбирает и фокус, и щелчок: WebKit на macOS не даёт фокус флажку и кнопке по щелчку.
-  const [mcpFocused, setMcpFocused] = useState(false);
-  useHelpChapter(tabId({ kind: "settings" }), mcpFocused ? "llm-mcp" : "getting-started");
+  // F1: «Начало работы» (язык описан там). Раздел LLM-ассистентов появится вместе с MCP-шлюзом (этап I2).
+  useHelpChapter(tabId({ kind: "settings" }), "getting-started");
 
   if (lang.error) return <ErrorBanner error={lang.error} onRetry={() => lang.refetch()} />;
   const s = lang.data;
@@ -41,7 +37,7 @@ export function SettingsPanel() {
       <div className="toolbar">
         <h2>{t("settings.title")}</h2>
       </div>
-      <section onFocusCapture={() => setMcpFocused(false)} onPointerDownCapture={() => setMcpFocused(false)}>
+      <section>
         <h3>{t("settings.language.section")}</h3>
         <div className="field">
           <label htmlFor="settings-language">{t("settings.language.label")}</label>
@@ -65,9 +61,6 @@ export function SettingsPanel() {
         {byPolicy && <p className="hint">{t("settings.language.policy")}</p>}
         {change.error && <ErrorBanner error={change.error} />}
       </section>
-      <div onFocusCapture={() => setMcpFocused(true)} onPointerDownCapture={() => setMcpFocused(true)}>
-        <McpSection />
-      </div>
     </div>
   );
 }
````

- [ ] **Step 2: Тесты.**

<!-- block: file t13-src-components-about-aboutpanel-test-tsx step=13.2 path=src/components/about/AboutPanel.test.tsx -->
````tsx
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { renderWithClient } from "../../test/render";

vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
vi.mock("../../api/client", async (importOriginal) => {
  const orig = await importOriginal<typeof import("../../api/client")>();
  return { ...orig, api: { getAppInfo: vi.fn(), getPolicyStatus: vi.fn() } };
});

import { openUrl } from "@tauri-apps/plugin-opener";
import { api } from "../../api/client";
import { LICENSE_URL, SUPPORT_URL } from "../../lib/links";
import { AboutPanel } from "./AboutPanel";

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.getAppInfo).mockResolvedValue({
    name: "Glafira for LLM Inference", version: "0.1.0", commit: "abc1234", os: "macos", arch: "aarch64",
    dataDir: "/data", logDir: "/logs",
  });
  vi.mocked(api.getPolicyStatus).mockResolvedValue({
    managed: true, source: "/etc/x", updatesEnabled: true, updateUrl: null, language: null, mcpEnabled: true,
    allowInsecureTls: false,
    errors: [{ key: "errors.policy.invalidBool", params: { key: "mcpEnabled", value: "2" }, text: "x" }],
  });
});

describe("AboutPanel", () => {
  it("версия, сборка, политика, ссылки и товарные знаки", async () => {
    const user = userEvent.setup();
    renderWithClient(<AboutPanel />);
    expect(await screen.findByText("Glafira for LLM Inference")).toBeInTheDocument();
    expect(screen.getByText("0.1.0")).toBeInTheDocument();
    expect(screen.getByText("abc1234")).toBeInTheDocument();
    expect(await screen.findByText("задана")).toBeInTheDocument();
    expect(screen.getByText("Политика организации: mcpEnabled: недопустимое логическое 2")).toBeInTheDocument();
    expect(screen.getByText(/vLLM, SGLang, LiteLLM, NVIDIA/)).toBeInTheDocument();
    expect(screen.getByText("/data")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Apache 2.0" }));
    expect(openUrl).toHaveBeenCalledWith(LICENSE_URL);
    await user.click(screen.getByRole("button", { name: "Поддержать проект" }));
    expect(openUrl).toHaveBeenCalledWith(SUPPORT_URL);
  });

  it("ошибка — баннер с повтором", async () => {
    vi.mocked(api.getAppInfo).mockRejectedValueOnce({ code: "INTERNAL", key: "errors.internal", message: "x", retryable: false });
    const user = userEvent.setup();
    renderWithClient(<AboutPanel />);
    await user.click(await screen.findByRole("button", { name: "Повторить" }));
    expect(await screen.findByText("Glafira for LLM Inference")).toBeInTheDocument();
  });
});
````

<!-- block: file t13-src-components-help-helpchapterfor-test-ts step=13.2 path=src/components/help/helpChapterFor.test.ts -->
````ts
import { describe, expect, it } from "vitest";
import type { TabSpec } from "../../state/workspace";
import { helpChapterFor } from "./helpChapterFor";

const cases: [TabSpec | null, string][] = [
  [{ kind: "installation-form", installationId: null }, "installations"],
  [{ kind: "installation", installationId: "i" }, "installations"],
  [{ kind: "about" }, "getting-started"],
  [{ kind: "settings" }, "getting-started"],
  [{ kind: "help", chapter: "installations" }, "getting-started"],
  [null, "getting-started"],
];

describe("helpChapterFor", () => {
  it.each(cases)("%j → %s", (tab, chapter) => {
    expect(helpChapterFor(tab)).toBe(chapter);
  });
});
````

<!-- block: file t13-src-components-settings-settingspanel-test-tsx step=13.2 path=src/components/settings/SettingsPanel.test.tsx -->
````tsx
import { act, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { LanguageSettings } from "../../api/types";
import { i18n } from "../../i18n";
import { renderWithClient } from "../../test/render";
import { useHelpContext } from "../../state/helpContext";

const base: LanguageSettings = {
  system: "ru-RU", systemMatch: "ru", chosen: null, policy: null, effective: "ru", reason: "system",
  languages: ["en", "ru"],
};

vi.mock("../../api/client", async (importOriginal) => {
  const orig = await importOriginal<typeof import("../../api/client")>();
  return { ...orig, api: { getLanguageSettings: vi.fn(), setLanguage: vi.fn() } };
});

import { api } from "../../api/client";
import { SettingsPanel } from "./SettingsPanel";

beforeEach(() => {
  vi.clearAllMocks();
});

// Выбор «en» переключает глобальный язык; сброс — в act, пока компоненты смонтированы.
afterEach(() => act(() => i18n.changeLanguage("ru")));

describe("SettingsPanel", () => {
  it("показывает язык системы и меняет язык", async () => {
    vi.mocked(api.getLanguageSettings).mockResolvedValue(base);
    vi.mocked(api.setLanguage).mockResolvedValue({ ...base, chosen: "en", effective: "en", reason: "user" });
    const user = userEvent.setup();
    renderWithClient(<SettingsPanel />);
    const select = await screen.findByLabelText("Язык интерфейса");
    expect(screen.getByRole("option", { name: "Как в системе (Русский)" })).toBeInTheDocument();
    expect(screen.getAllByRole("option").map((o) => o.textContent)).toEqual(expect.arrayContaining(["English", "Русский"]));
    await user.selectOptions(select, "en");
    expect(api.setLanguage).toHaveBeenCalledWith("en");
    expect(await screen.findByLabelText("Interface language")).toBeInTheDocument();
  });

  it("язык из политики нельзя сменить", async () => {
    vi.mocked(api.getLanguageSettings).mockResolvedValue({ ...base, policy: "ru", reason: "policy" });
    renderWithClient(<SettingsPanel />);
    expect(await screen.findByLabelText("Язык интерфейса")).toBeDisabled();
    expect(screen.getByText("Язык задан вашей организацией")).toBeInTheDocument();
  });

  it("F1 в настройках — глава «Начало работы»", async () => {
    vi.mocked(api.getLanguageSettings).mockResolvedValue(base);
    renderWithClient(<SettingsPanel />);
    await screen.findByLabelText("Язык интерфейса");
    expect(useHelpContext.getState().byTab.settings).toBe("getting-started");
  });
});
````

- [ ] **Step 3: Реализация.**

<!-- block: file t13-docs-guide-en-getting-started-md step=13.3 path=docs/guide/en/getting-started.md -->
````markdown
# Getting started

Glafira for LLM Inference is a desktop app for connecting to corporate open-source LLM inference, monitoring,
diagnosing and auditing it, with vLLM and SGLang in focus. Glafira changes nothing on the servers: it reads their
answers with your API keys. Glafira never reads or shows the texts of your users' requests and answers.

## Installation {#install}

Installers for macOS (`.dmg`), Windows (`.msi`) and Linux (`.deb`, `.rpm`, `.AppImage`) are published at
https://github.com/vvnovg/glafira-inference-releases/releases. Your organization can configure the app with a policy:
then some settings are set by the administrator.

## First installation {#first-installation}

An installation is the inference of one system: the vLLM, SGLang or other OpenAI-compatible servers that serve its
models.

1. Click [[ui:sidebar.newInstallation]] in the left panel.
2. Name the installation and choose its [[ui:installations.form.environment]].
3. For each server enter the [[ui:installations.form.baseUrl]] — the address of the OpenAI-compatible API without
   `/v1`, such as `http://vllm.internal:8000` — and the [[ui:installations.form.apiKey]] if the server runs with
   `--api-key`. More in [Servers](installations.md#servers).
4. Click [[ui:installations.form.test]]: Glafira checks the address, TLS, `/health`, the key, the type and version of
   the server and its models step by step and shows the step that fails. Then click [[ui:common.save]].
5. The installation tab opens: which servers are connected, their type, version, models and whether they expose
   metrics.

## Interface language {#language}

Open [[ui:sidebar.settings]] and choose the [[ui:settings.language.label]]. By default it follows the system. If the
language is set by your organization, it cannot be changed.

## Help {#help}

F1 on any screen opens its chapter. The guide works offline; search is in the field above the contents.
````

<!-- block: file t13-docs-guide-en-installations-md step=13.3 path=docs/guide/en/installations.md -->
````markdown
# Installations

An installation groups the inference servers of one system. Its profile is kept on your computer, the API keys in the
system secret store (macOS Keychain, Windows Credential Manager, Secret Service on Linux).

## Servers {#servers}

Add each server separately, replicas of one model behind a load balancer too: this shows which one answers
differently. The address is the OpenAI-compatible API without `/v1`: `http://host:port` or `https://host:port`. A path
is allowed for a server behind a reverse proxy (`https://gw.corp/llm/qwen`). A user name and password in the address
are not allowed.

The [[ui:installations.form.metricsPath]] is where the server exposes Prometheus metrics, usually `/metrics`. SGLang
exposes them only when started with `--enable-metrics`.

## Server type {#kind}

By default the type is detected from the answers: vLLM by `/version` and `vllm:` metrics, SGLang by `/model_info` and
`/server_info`. Other servers with `/v1/models` (Ollama, llama.cpp, Triton, TGI) work as OpenAI-compatible. vLLM older
than 0.8 and SGLang older than 0.4 work as OpenAI-compatible too, and Glafira warns about it.

Choose the type by hand only if detection is wrong: Glafira works with the chosen type and shows that the server
looks different.

## API key {#api-key}

The key is the same value as the `--api-key` of the server; Glafira sends it as `Authorization: Bearer`. The key is
kept in the system secret store; it is neither in the profile file nor in an export. An empty field of a saved
server keeps the previous key, the [[ui:installations.form.apiKeyClear]] checkbox deletes it.

Over an `http://` address the key travels unencrypted, and the form warns about it. Glafira does not follow redirects
to another address, so the key never goes where you did not send it.

## Server check {#check}

[[ui:installations.form.test]] checks the server with the values of the form, before saving: configuration, DNS and
TCP, TLS (for `https://`), `/health`, sign-in with the key, type and version, models. The check stops at the first
failed step and names it. Warnings (an old version, another type in the installation, no metrics) do not fail it.

## Certificates and proxy {#tls}

[[ui:installations.form.ca]]: the system store is the CAs this computer trusts; listed certificates are only them,
without the system ones. Mode without certificate verification is insecure, and your organization can forbid it.

[[ui:installations.form.proxy]]: system takes the `HTTPS_PROXY` and `NO_PROXY` variables; you can also enter a proxy
address or turn the proxy off.

## Installation tab {#installation-tab}

When opened, the tab connects to all servers at once. For each one it shows whether it is connected, its type and
version, the models with their context length (the base model for LoRA adapters) and whether it exposes metrics. An
error of one server does not affect the others. [[ui:installations.panel.reconnect]] reads everything again, for
example after a server restart.

## Environments and read-only {#environments}

The environment sets the color of the stripe above the tabs and the badge. A new `prod` installation is created
read-only: Glafira will not run anything that loads or changes its servers.

## Folders, export and import {#folders}

Drag installations into folders. [[ui:sidebar.exportAll]] and the export of a folder write a file without API keys:
the recipient enters their own. Import shows what will be added and what updated. If an update changes the address,
the proxy or the certificate verification of a server with a saved key, the preview warns that the key will go to the
new address.
````

<!-- block: file t13-docs-guide-ru-getting-started-md step=13.3 path=docs/guide/ru/getting-started.md -->
````markdown
# Начало работы

Glafira for LLM Inference — настольное приложение для подключения к корпоративному инференсу open-source LLM, его
мониторинга, диагностики и аудита; в фокусе — vLLM и SGLang. Glafira ничего не меняет в серверах: она читает их ответы
с вашими ключами API. Тексты запросов и ответов ваших пользователей Glafira не читает и не показывает.

## Установка {#install}

Установщики для macOS (`.dmg`), Windows (`.msi`) и Linux (`.deb`, `.rpm`, `.AppImage`) публикуются на
https://github.com/vvnovg/glafira-inference-releases/releases. Организация может настроить приложение политикой:
тогда часть настроек задаёт администратор.

## Первая инсталляция {#first-installation}

Инсталляция — это инференс одной системы: серверы vLLM, SGLang или другие OpenAI-совместимые серверы, которые
обслуживают её модели.

1. Нажмите [[ui:sidebar.newInstallation]] в левой панели.
2. Дайте инсталляции имя и выберите [[ui:installations.form.environment]].
3. Для каждого сервера укажите [[ui:installations.form.baseUrl]] — адрес OpenAI-совместимого API без `/v1`, например
   `http://vllm.internal:8000`, и [[ui:installations.form.apiKey]], если сервер запущен с `--api-key`. Подробнее —
   [Серверы](installations.md#servers).
4. Нажмите [[ui:installations.form.test]]: Glafira по шагам проверит адрес, TLS, `/health`, ключ, тип и версию
   сервера и его модели и покажет, на каком шаге ошибка. Затем нажмите [[ui:common.save]].
5. Откроется вкладка инсталляции: какие серверы подключились, их тип, версия, модели и отдают ли они метрики.

## Язык интерфейса {#language}

Откройте [[ui:sidebar.settings]] и выберите [[ui:settings.language.label]]. По умолчанию — как в системе. Если язык
задан вашей организацией, выбор недоступен.

## Справка {#help}

Клавиша F1 на любом экране открывает главу о нём. Руководство работает без интернета; поиск — в поле над содержанием.
````

<!-- block: file t13-docs-guide-ru-installations-md step=13.3 path=docs/guide/ru/installations.md -->
````markdown
# Инсталляции

Инсталляция объединяет серверы инференса одной системы. Её профиль хранится на вашем компьютере, ключи API — в
системном хранилище секретов (Связка ключей macOS, диспетчер учётных данных Windows, Secret Service в Linux).

## Серверы {#servers}

Добавьте каждый сервер отдельно — и реплики одной модели за балансировщиком тоже: так видно, какая из них отвечает
не так. Адрес — OpenAI-совместимый API без `/v1`: `http://хост:порт` или `https://хост:порт`. Путь допускается, если
сервер стоит за обратным прокси (`https://gw.corp/llm/qwen`). Имя пользователя и пароль в адресе запрещены.

[[ui:installations.form.metricsPath]] — где сервер отдаёт метрики Prometheus, обычно `/metrics`. SGLang отдаёт их
только после запуска с `--enable-metrics`.

## Тип сервера {#kind}

По умолчанию тип определяется по ответам: vLLM — по `/version` и метрикам `vllm:`, SGLang — по `/model_info` и
`/server_info`. Остальные серверы с `/v1/models` (Ollama, llama.cpp, Triton, TGI) работают как OpenAI-совместимые.
vLLM старше 0.8 и SGLang старше 0.4 тоже работают как OpenAI-совместимые — Glafira предупредит об этом.

Выберите тип вручную, только если он определился неверно: Glafira будет работать с выбранным типом и покажет, что
признаки сервера с ним не совпадают.

## Ключ API {#api-key}

Ключ — то же значение, что `--api-key` сервера; Glafira отправляет его заголовком `Authorization: Bearer`. Ключ
хранится в системном хранилище секретов, в файле профиля и в экспорте его нет. Пустое поле у сохранённого сервера
оставляет прежний ключ, флажок [[ui:installations.form.apiKeyClear]] удаляет его.

По адресу `http://` ключ уходит открытым текстом — форма предупредит об этом. Перенаправления на другой адрес Glafira
не выполняет, поэтому ключ не уходит туда, куда вы его не направляли.

## Проверка сервера {#check}

[[ui:installations.form.test]] проверяет сервер со значениями формы, ещё до сохранения: конфигурация, DNS и TCP,
TLS (для `https://`), `/health`, вход с ключом, тип и версия, модели. Проверка останавливается на первом неуспешном
шаге и называет его. Предупреждения — старая версия, другой тип в инсталляции, нет метрик — успеха не отменяют.

## Сертификаты и прокси {#tls}

[[ui:installations.form.ca]]: системное хранилище — CA, которым доверяет компьютер; перечисленные сертификаты — только
они, системные не используются. Режим без проверки сертификата небезопасен, и организация может его запретить.

[[ui:installations.form.proxy]]: системный берёт переменные `HTTPS_PROXY` и `NO_PROXY`; можно указать адрес прокси
явно или отключить прокси.

## Вкладка инсталляции {#installation-tab}

При открытии вкладка подключается ко всем серверам сразу. Для каждого видно: подключился ли он, тип и версию, модели
с длиной контекста (у адаптеров LoRA — базовую модель) и отдаёт ли сервер метрики. Ошибка одного сервера не мешает
остальным. [[ui:installations.panel.reconnect]] читает сведения заново — например, после перезапуска сервера.

## Окружения и только чтение {#environments}

Окружение задаёт цвет полосы над вкладками и бейдж. Новая инсталляция `prod` создаётся только для чтения: Glafira
не запустит ничего, что нагружает или меняет её серверы.

## Папки, экспорт и импорт {#folders}

Инсталляции раскладываются по папкам перетаскиванием. [[ui:sidebar.exportAll]] и экспорт папки пишут файл без ключей
API: у получателя инсталляция ждёт своих ключей. Импорт показывает, что добавится и что обновится. Если обновление
меняет адрес, прокси или проверку сертификата сервера с сохранённым ключом, предпросмотр предупредит: ключ уйдёт на
новый адрес.
````

<!-- block: file t13-src-components-about-aboutpanel-tsx step=13.3 path=src/components/about/AboutPanel.tsx -->
````tsx
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { api } from "../../api/client";
import { tr } from "../../i18n/text";
import { openLicense, openSupport } from "../../lib/links";
import { ErrorBanner } from "../common/ErrorBanner";

export function AboutPanel() {
  const { t } = useTranslation();
  const info = useQuery({ queryKey: ["app-info"], queryFn: api.getAppInfo });
  const policy = useQuery({ queryKey: ["policy"], queryFn: api.getPolicyStatus });

  if (info.error) return <ErrorBanner error={info.error} onRetry={() => info.refetch()} />;
  const i = info.data;
  if (!i) return <p className="empty">{t("common.loading")}</p>;
  return (
    <div className="page">
      <div className="toolbar">
        <h2>{i.name}</h2>
      </div>
      <p className="about-summary">{t("about.summary")}</p>
      <dl className="stats">
        <div><dt>{t("about.version")}</dt><dd>{i.version}</dd></div>
        <div><dt>{t("about.build")}</dt><dd>{i.commit}</dd></div>
        <div><dt>{t("about.system")}</dt><dd>{i.os} / {i.arch}</dd></div>
        <div>
          <dt>{t("about.license")}</dt>
          <dd>
            <button type="button" className="link" onClick={openLicense}>
              Apache 2.0
            </button>
          </dd>
        </div>
        <div><dt>{t("about.policy")}</dt><dd>{policy.data ? (policy.data.managed ? t("about.policySet") : t("about.policyNone")) : "…"}</dd></div>
      </dl>
      {policy.data?.errors.map((e, n) => (
        <p key={n} className="warn">
          {t("about.policyError", { error: tr(e) })}
        </p>
      ))}
      <p>
        <button type="button" className="link" onClick={openSupport}>
          {t("support.title")}
        </button>{" "}
        <span className="muted">{t("support.hint")}</span>
      </p>
      <p className="muted">{t("about.dataDir")} <code>{i.dataDir}</code></p>
      <p className="muted">{t("about.logDir")} <code>{i.logDir}</code></p>
      <p className="muted">{t("about.trademarks")}</p>
    </div>
  );
}
````

<!-- block: file t13-src-components-help-helpchapterfor-ts step=13.3 path=src/components/help/helpChapterFor.ts -->
````ts
import type { ChapterId } from "../../guide/chapters";
import type { TabSpec } from "../../state/workspace";

/** Глава руководства для экрана (F1). */
export function helpChapterFor(tab: TabSpec | null): ChapterId {
  switch (tab?.kind) {
    case "installation-form":
    case "installation":
      return "installations";
    default:
      return "getting-started";
  }
}
````

<!-- block: patch t13-src step=13.3 -->
````diff
diff --git a/PORTED.md b/PORTED.md
index 7c85b14..67fba0e 100644
--- a/PORTED.md
+++ b/PORTED.md
@@ -99,3 +99,19 @@
 | `src/state/workspace.ts` | `src/state/workspace.ts` | `468e08a` | адаптирован: вкладки инсталляций вместо вкладок кластера |
 | `src/state/workspace.test.ts` | `src/state/workspace.test.ts` | `468e08a` | адаптирован: вкладки инсталляций |
 | `src/components/installations/InstallationFormTab.tsx` | `src/components/connections/ConnectionFormTab.tsx` | `468e08a` | адаптирован: инсталляция вместо подключения |
+| `scripts/guide-publish.mjs` | `scripts/guide-publish.mjs` | `468e08a` | адаптирован: имя продукта |
+| `scripts/guide-publish.test.mjs` | `scripts/guide-publish.test.mjs` | `468e08a` | как есть |
+| `scripts/guide-publish.d.mts` | `scripts/guide-publish.d.mts` | `468e08a` | как есть |
+| `src/guide/chapters.ts` | `src/guide/chapters.ts` | `468e08a` | адаптирован: главы «Начало работы» и «Инсталляции» |
+| `src/guide/chapters.test.ts` | `src/guide/chapters.test.ts` | `468e08a` | адаптирован: глава «Инсталляции» |
+| `src/guide/parse.ts` | `src/guide/parse.ts` | `468e08a` | как есть |
+| `src/guide/parse.test.ts` | `src/guide/parse.test.ts` | `468e08a` | адаптирован: глава «Инсталляции» |
+| `src/components/help/HelpPanel.tsx` | `src/components/help/HelpPanel.tsx` | `468e08a` | как есть |
+| `src/components/help/HelpPanel.test.tsx` | `src/components/help/HelpPanel.test.tsx` | `468e08a` | адаптирован: глава «Инсталляции» |
+| `src/components/help/helpChapterFor.ts` | `src/components/help/helpChapterFor.ts` | `468e08a` | адаптирован: вкладки инсталляций |
+| `src/components/help/helpChapterFor.test.ts` | `src/components/help/helpChapterFor.test.ts` | `468e08a` | адаптирован: вкладки инсталляций |
+| `src/state/helpContext.ts` | `src/state/helpContext.ts` | `468e08a` | как есть |
+| `src/components/settings/SettingsPanel.tsx` | `src/components/settings/SettingsPanel.tsx` | `468e08a` | адаптирован: без раздела MCP (этап I2) |
+| `src/components/settings/SettingsPanel.test.tsx` | `src/components/settings/SettingsPanel.test.tsx` | `468e08a` | адаптирован: без раздела MCP |
+| `src/components/about/AboutPanel.tsx` | `src/components/about/AboutPanel.tsx` | `468e08a` | адаптирован: без обновлений (этап I2) |
+| `src/components/about/AboutPanel.test.tsx` | `src/components/about/AboutPanel.test.tsx` | `468e08a` | адаптирован: без обновлений |
diff --git a/package.json b/package.json
index 032eff9..e32ba80 100644
--- a/package.json
+++ b/package.json
@@ -12,7 +12,8 @@
     "tauri": "tauri",
     "test": "vitest run",
     "test:scripts": "node --test \"scripts/*.test.mjs\"",
-    "i18n:check": "node scripts/i18n-check.mjs --rust-cyrillic src-tauri/src"
+    "i18n:check": "node scripts/i18n-check.mjs --rust-cyrillic src-tauri/src",
+    "guide:publish": "node scripts/guide-publish.mjs dist/guide"
   },
   "dependencies": {
     "@floating-ui/react": "^0.27.20",
diff --git a/scripts/i18n-check.mjs b/scripts/i18n-check.mjs
index a0ad3d9..f63ad59 100644
--- a/scripts/i18n-check.mjs
+++ b/scripts/i18n-check.mjs
@@ -562,7 +562,7 @@ export function findRustCyrillic(files) {
 }
 
 /** Главы руководства: тот же список, что `GUIDE_CHAPTERS` в `src/guide/chapters.ts`. */
-export const GUIDE_CHAPTERS = [];
+export const GUIDE_CHAPTERS = ["getting-started", "installations"];
 
 const KEBAB = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
 
````

<!-- block: locales t13-locales step=13.3 -->
````json
{
  "en": {
    "settings": {
      "title": "Settings",
      "language": {
        "section": "Language",
        "label": "Interface language",
        "system": "System default ({{name}})",
        "systemUnknown": "System default",
        "policy": "The language is set by your organization"
      }
    },
    "about": {
      "summary": "Glafira is a desktop app for connecting to corporate LLM inference, monitoring, diagnosing and auditing it, with vLLM and SGLang in focus: installations of several servers, API keys in the system secret store, step-by-step server checks, folders, export and import.",
      "version": "Version",
      "build": "Build",
      "system": "System",
      "license": "License",
      "policy": "Organization policy",
      "policySet": "set",
      "policyNone": "none",
      "policyError": "Organization policy: {{error}}",
      "dataDir": "Data:",
      "logDir": "Logs:",
      "trademarks": "vLLM, SGLang, LiteLLM, NVIDIA and other product names are trademarks of their respective owners. This project is not affiliated with them."
    },
    "support": {
      "title": "Support the project",
      "hint": "Glafira is free and open source; donations keep it going."
    },
    "help": {
      "search": "Search the guide",
      "noResults": "Nothing found",
      "contents": "Contents",
      "loadFailed": "Could not load the guide"
    },
    "sidebar": {
      "settings": "Settings",
      "help": "Help",
      "about": "About",
      "newInstallation": "New installation",
      "newFolder": "New folder",
      "exportAll": "Export all…",
      "import": "Import…"
    }
  },
  "ru": {
    "settings": {
      "title": "Настройки",
      "language": {
        "section": "Язык",
        "label": "Язык интерфейса",
        "system": "Как в системе ({{name}})",
        "systemUnknown": "Как в системе",
        "policy": "Язык задан вашей организацией"
      }
    },
    "about": {
      "summary": "Glafira — настольное приложение для подключения к корпоративному инференсу LLM, его мониторинга, диагностики и аудита, в фокусе — vLLM и SGLang: инсталляции из нескольких серверов, ключи API в системном хранилище секретов, пошаговая проверка серверов, папки, экспорт и импорт.",
      "version": "Версия",
      "build": "Сборка",
      "system": "Система",
      "license": "Лицензия",
      "policy": "Политика организации",
      "policySet": "задана",
      "policyNone": "нет",
      "policyError": "Политика организации: {{error}}",
      "dataDir": "Данные:",
      "logDir": "Логи:",
      "trademarks": "vLLM, SGLang, LiteLLM, NVIDIA и другие названия продуктов — товарные знаки их правообладателей. Проект с ними не связан."
    },
    "support": {
      "title": "Поддержать проект",
      "hint": "Glafira бесплатна, её исходный код открыт; пожертвования помогают её развивать."
    },
    "help": {
      "search": "Поиск по руководству",
      "noResults": "Ничего не найдено",
      "contents": "Содержание",
      "loadFailed": "Не удалось загрузить руководство"
    },
    "sidebar": {
      "settings": "Настройки",
      "help": "Справка",
      "about": "О программе",
      "newInstallation": "Новая инсталляция",
      "newFolder": "Новая папка",
      "exportAll": "Экспортировать всё…",
      "import": "Импортировать…"
    }
  }
}
````

<!-- block: shell t13-delete step=13.3 -->
````bash
rm docs/guide/en/.gitkeep
rm docs/guide/ru/.gitkeep
````

<!-- block: shell t13-install step=13.3 -->
````bash
pnpm install
````

- [ ] **Step 4: Проверки.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib`  
Expected: PASS — `test result: ok. 161 passed; 0 failed`

Run: `pnpm test`  
Expected: PASS — `Test Files 23 passed`, `Tests 109 passed`

Run: `pnpm exec tsc --noEmit`  
Expected: PASS — без ошибок

Run: `pnpm i18n:check`  
Expected: PASS — `✓ словари en, ru согласованы, вызовов из Rust: 95`

Run: `pnpm test:scripts`  
Expected: PASS — `# pass 63`, `# fail 0`

Run: `pnpm build`  
Expected: PASS — `✓ dist/assets: чанки совместимы с WebKit macOS 11+`

- [ ] **Step 5: Commit.**

```bash
git add \
  PORTED.md \
  docs/guide/en/.gitkeep \
  docs/guide/en/getting-started.md \
  docs/guide/en/installations.md \
  docs/guide/ru/.gitkeep \
  docs/guide/ru/getting-started.md \
  docs/guide/ru/installations.md \
  package.json \
  scripts/guide-publish.d.mts \
  scripts/guide-publish.mjs \
  scripts/guide-publish.test.mjs \
  scripts/i18n-check.mjs \
  src/components/about/AboutPanel.test.tsx \
  src/components/about/AboutPanel.tsx \
  src/components/help/HelpPanel.test.tsx \
  src/components/help/HelpPanel.tsx \
  src/components/help/helpChapterFor.test.ts \
  src/components/help/helpChapterFor.ts \
  src/components/settings/SettingsPanel.test.tsx \
  src/components/settings/SettingsPanel.tsx \
  src/guide/chapters.test.ts \
  src/guide/chapters.ts \
  src/guide/parse.test.ts \
  src/guide/parse.ts \
  src/locales/en.json \
  src/locales/ru.json \
  src/state/helpContext.ts
git commit -s -m "feat(ui): language settings, built-in guide with the getting-started and installations chapters, About" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 14: Дерево инсталляций с папками, перетаскиванием, экспортом и импортом; раскладка, боковая панель, меню Help

**Files:**
- Port: `src/components/Layout.test.tsx` (с правками), `src/components/Sidebar.test.tsx` (с правками), `src/components/Sidebar.tsx` (с правками), `src/components/TabContent.help.test.tsx`, `src/components/installations/tree/FolderMenu.tsx`, `src/components/installations/tree/ImportDialog.test.tsx` (с правками), `src/components/installations/tree/ImportDialog.tsx` (с правками), `src/components/installations/tree/InstallationTree.test.tsx` (с правками), `src/components/installations/tree/InstallationTree.tsx` (с правками), `src/guide/noLookbehind.test.ts`, `src/lib/tabActive.test.tsx`, `src/lib/tabActive.ts`, `src/lib/useAppMenu.test.ts`, `src/lib/useAppMenu.ts`, `src/lib/useHelpShortcut.ts`
- Create (по Kubernetes-версии): `src/components/Layout.tsx` ← `src/components/Layout.tsx`, `src/components/TabContent.tsx` ← `src/components/TabContent.tsx`, `src/components/installations/tree/InstallationNode.tsx` ← `src/components/connections/tree/ConnectionNode.tsx`
- Modify: `PORTED.md`, `src/App.test.tsx`, `src/App.tsx`, `src/app.css`, `src/locales/en.json`, `src/locales/ru.json`

**Interfaces:**
- Consumes: задачи 10–13.
- Produces: `InstallationTree`, `InstallationNode`, `FolderMenu`, `ImportDialog`, `Layout`, `Sidebar`, `TabContent`, `useAppMenu`, `useHelpShortcut`, `TabActiveContext`/`useTabActive`; `App` с `Layout` и меню Help.

Дерево Kubernetes-версии переносится с переименованием и без импорта kubeconfig. Узел инсталляции — «Обзор» и серверы с состоянием из подключения вкладки (само дерево не подключается). Импорт без перезапуска сессий: сессий в I1a нет, а подключения обновлённых инсталляций сбрасываются `resetQueries`. Раскладка, боковая панель и вкладки — без MCP, проверки и обновлений.

- [ ] **Step 1: Перенос из Kubernetes-версии.**

Файлы копируются с коммита `468e08a` репозитория `~/projects/glafira-openshift` (`$KUBERNETES_REPO`) и правятся патчами — против перенесённого файла. Строки о них в `PORTED.md` добавляет шаг реализации.

<!-- block: shell t14-port step=14.1 -->
````bash
scripts/port.sh src/components/connections/tree/ConnectionTree.tsx src/components/installations/tree/InstallationTree.tsx
scripts/port.sh src/components/connections/tree/ConnectionTree.test.tsx src/components/installations/tree/InstallationTree.test.tsx
scripts/port.sh src/components/connections/tree/FolderMenu.tsx src/components/installations/tree/FolderMenu.tsx
scripts/port.sh src/components/connections/tree/ImportDialog.tsx src/components/installations/tree/ImportDialog.tsx
scripts/port.sh src/components/connections/tree/ImportDialog.test.tsx src/components/installations/tree/ImportDialog.test.tsx
scripts/port.sh src/components/Layout.test.tsx src/components/Layout.test.tsx
scripts/port.sh src/components/Sidebar.tsx src/components/Sidebar.tsx
scripts/port.sh src/components/Sidebar.test.tsx src/components/Sidebar.test.tsx
scripts/port.sh src/components/TabContent.help.test.tsx src/components/TabContent.help.test.tsx
scripts/port.sh src/lib/useAppMenu.ts src/lib/useAppMenu.ts
scripts/port.sh src/lib/useAppMenu.test.ts src/lib/useAppMenu.test.ts
scripts/port.sh src/lib/useHelpShortcut.ts src/lib/useHelpShortcut.ts
scripts/port.sh src/lib/tabActive.ts src/lib/tabActive.ts
scripts/port.sh src/lib/tabActive.test.tsx src/lib/tabActive.test.tsx
scripts/port.sh src/guide/noLookbehind.test.ts src/guide/noLookbehind.test.ts
````

<!-- block: patch t14-port-src-components-installations-tree-installationtree-tsx step=14.1 -->
````diff
diff --git a/src/components/installations/tree/InstallationTree.tsx b/src/components/installations/tree/InstallationTree.tsx
--- a/src/components/installations/tree/InstallationTree.tsx
+++ b/src/components/installations/tree/InstallationTree.tsx
@@ -3,14 +3,13 @@ import { open as openDialog, save } from "@tauri-apps/plugin-dialog";
 import { type DragEvent, type KeyboardEvent, useEffect, useMemo, useRef, useState } from "react";
 import { Trans, useTranslation } from "react-i18next";
 import { api, toAppError } from "../../../api/client";
-import type { AppError, ConnectionFolder, ConnectionProfile, ImportPreview, TreeItemRef } from "../../../api/types";
+import type { AppError, ImportPreview, InstallationFolder, TreeItemRef } from "../../../api/types";
 import { tr } from "../../../i18n/text";
 import { useWorkspace } from "../../../state/workspace";
 import { ErrorBanner } from "../../common/ErrorBanner";
-import { ConnectionNode } from "./ConnectionNode";
+import { InstallationNode } from "./InstallationNode";
 import { FolderMenu, type FolderMenuItem } from "./FolderMenu";
 import { ImportDialog } from "./ImportDialog";
-import { KubeconfigImportDialog } from "./KubeconfigImportDialog";
 import { buildTree, canMoveFolder, filterTree, type TreeNode } from "./folderTree";
 
 const COLLAPSED_KEY = "glafira.tree.collapsed";
@@ -20,14 +19,14 @@ const DRAG_TYPE = "application/x-glafira-tree-item";
 const BEFORE_ZONE = 0.3;
 /** Фильтр диалогов сохранения и открытия файла экспорта. */
 const EXCHANGE_FILTERS = [{ name: "Glafira", extensions: ["json"] }];
-const EXCHANGE_SUFFIX = ".glafira-kubernetes-connections.json";
+const EXCHANGE_SUFFIX = ".glafira-inference-installations.json";
 
 /** Имя файла экспорта по умолчанию: имя папки (без недопустимых в именах файлов символов) или дата. */
-function exportFileName(folder: ConnectionFolder | null): string {
+function exportFileName(folder: InstallationFolder | null): string {
   if (folder) return `${folder.name.replace(/[\\/:*?"<>|]/g, "_")}${EXCHANGE_SUFFIX}`;
   const d = new Date();
   const pad = (n: number) => String(n).padStart(2, "0");
-  return `glafira-kubernetes-connections-${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}${EXCHANGE_SUFFIX}`;
+  return `glafira-inference-installations-${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}${EXCHANGE_SUFFIX}`;
 }
 
 function readCollapsed(): Set<string> {
@@ -99,7 +98,7 @@ function FolderNameInput({ initial, onSubmit, onCancel }: { initial: string; onS
 }
 
 /** Подтверждение удаления папки внутри страницы (window.confirm в Tauri не работает). */
-function DeleteFolderConfirm({ folder, onConfirm, onCancel }: { folder: ConnectionFolder; onConfirm: () => Promise<unknown>; onCancel: () => void }) {
+function DeleteFolderConfirm({ folder, onConfirm, onCancel }: { folder: InstallationFolder; onConfirm: () => Promise<unknown>; onCancel: () => void }) {
   const { t } = useTranslation();
   const [error, setError] = useState<AppError | null>(null);
   const [busy, setBusy] = useState(false);
@@ -138,15 +137,15 @@ function DeleteFolderConfirm({ folder, onConfirm, onCancel }: { folder: Connecti
 }
 
 type Editing = { mode: "create"; parentId: string | null } | { mode: "rename"; folderId: string };
-/** Куда бросают: внутрь папки, перед подключением или в корень. */
+/** Куда бросают: внутрь папки, перед инсталляцией или в корень. */
 type DropTarget = { key: string; parentId: string | null; before: TreeItemRef | null };
 
-/** Дерево подключений в левой панели: заголовок с действиями, фильтр, папки и подключения. */
-export function ConnectionTree() {
+/** Дерево инсталляций в левой панели: заголовок с действиями, фильтр, папки и инсталляции. */
+export function InstallationTree() {
   const { t } = useTranslation();
   const queryClient = useQueryClient();
   const open = useWorkspace((s) => s.open);
-  const connectionsQ = useQuery({ queryKey: ["connections"], queryFn: api.listConnections });
+  const installationsQ = useQuery({ queryKey: ["installations"], queryFn: api.listInstallations });
   const foldersQ = useQuery({ queryKey: ["folders"], queryFn: api.listFolders });
   const [query, setQuery] = useState("");
   const [collapsed, setCollapsed] = useState(readCollapsed);
@@ -161,20 +160,18 @@ export function ConnectionTree() {
   const [exchangeError, setExchangeError] = useState<unknown>(null);
   /** Открытый диалог импорта: выбранный файл и начальная цель. */
   const [importing, setImporting] = useState<{ path: string; targetId: string | null } | null>(null);
-  /** Открытый диалог импорта из kubeconfig: папка новых подключений. */
-  const [kubeImport, setKubeImport] = useState<{ targetId: string | null } | null>(null);
   const dragged = useRef<TreeItemRef | null>(null);
 
-  const connections = connectionsQ.data;
+  const installations = installationsQ.data;
   const folders = foldersQ.data;
-  const tree = useMemo(() => buildTree(folders ?? [], connections ?? []), [folders, connections]);
+  const tree = useMemo(() => buildTree(folders ?? [], installations ?? []), [folders, installations]);
   const visible = useMemo(() => filterTree(tree, query), [tree, query]);
   const filtering = query.trim() !== "";
 
   const refresh = () =>
     Promise.all([
       queryClient.invalidateQueries({ queryKey: ["folders"] }),
-      queryClient.invalidateQueries({ queryKey: ["connections"] }),
+      queryClient.invalidateQueries({ queryKey: ["installations"] }),
     ]);
 
   // Запись в localStorage — эффектом, а не внутри updater: updater должен быть чистым.
@@ -219,14 +216,14 @@ export function ConnectionTree() {
     await refresh();
   };
 
-  const exportTree = async (folder: ConnectionFolder | null) => {
+  const exportTree = async (folder: InstallationFolder | null) => {
     setNotice(null);
     setExchangeError(null);
     try {
       const path = await save({ defaultPath: exportFileName(folder), filters: EXCHANGE_FILTERS });
       if (!path) return;
-      const summary = await api.exportConnections(path, folder ? folder.id : null);
-      setNotice(t("exchange.exported", { connections: summary.connections, folders: summary.folders }));
+      const summary = await api.exportInstallations(path, folder ? folder.id : null);
+      setNotice(t("exchange.exported", { installations: summary.installations, folders: summary.folders }));
     } catch (err) {
       setExchangeError(err);
     }
@@ -250,27 +247,15 @@ export function ConnectionTree() {
     setNotice(parts.join(". "));
   };
 
-  const startKubeImport = (targetId: string | null) => {
-    setNotice(null);
-    setExchangeError(null);
-    setKubeImport({ targetId });
-  };
-
-  const onKubeImported = (saved: ConnectionProfile[]) => {
-    setKubeImport(null);
-    setNotice(t("kubeconfig.imported", { count: saved.length }));
-  };
-
   const headerMenuItems: FolderMenuItem[] = [
     { label: t("sidebar.newFolder"), onSelect: () => startCreate(null) },
     { label: t("sidebar.exportAll"), onSelect: () => void exportTree(null) },
     { label: t("sidebar.import"), onSelect: () => void startImport(null) },
-    { label: t("sidebar.importKubeconfig"), onSelect: () => startKubeImport(null) },
   ];
 
-  const folderMenuItems = (folder: ConnectionFolder): FolderMenuItem[] => [
+  const folderMenuItems = (folder: InstallationFolder): FolderMenuItem[] => [
     { label: t("sidebar.newFolder"), onSelect: () => startCreate(folder.id) },
-    { label: t("sidebar.folder.newConnection"), onSelect: () => open({ kind: "connection-form", connectionId: null, folderId: folder.id }) },
+    { label: t("sidebar.folder.newInstallation"), onSelect: () => open({ kind: "installation-form", installationId: null, folderId: folder.id }) },
     {
       label: t("sidebar.folder.rename"),
       onSelect: () => {
@@ -287,7 +272,6 @@ export function ConnectionTree() {
     },
     { label: t("sidebar.folder.export"), onSelect: () => void exportTree(folder) },
     { label: t("sidebar.folder.importHere"), onSelect: () => void startImport(folder.id) },
-    { label: t("sidebar.folder.importKubeconfigHere"), onSelect: () => startKubeImport(folder.id) },
   ];
 
   // --- Перетаскивание ---
@@ -348,13 +332,13 @@ export function ConnectionTree() {
     },
   });
 
-  /** Конец папок уровня parentId: туда попадает папка, брошенная на подключение этого уровня. */
+  /** Конец папок уровня parentId: туда попадает папка, брошенная на инсталляция этого уровня. */
   const foldersEndKey = (parentId: string | null) => `end:${parentId ?? ""}`;
 
   const rootTarget = dropTarget(() => ({ key: "root", parentId: null, before: null }), ["root"]);
 
   /** Строка папки: верхние ~30% — «перед папкой» (только для папок), остальное — «внутрь». */
-  const folderRowTarget = (folder: ConnectionFolder, parentId: string | null) =>
+  const folderRowTarget = (folder: InstallationFolder, parentId: string | null) =>
     dropTarget(
       (item, e) => {
         const rect = e.currentTarget.getBoundingClientRect();
@@ -367,13 +351,13 @@ export function ConnectionTree() {
       [`fb:${folder.id}`, `f:${folder.id}`],
     );
 
-  /** Строка подключения: подключение встаёт перед ним, папка — в конец папок того же уровня. */
-  const connectionRowTarget = (id: string, parentId: string | null) =>
+  /** Строка инсталляции: инсталляция встаёт перед ним, папка — в конец папок того же уровня. */
+  const installationRowTarget = (id: string, parentId: string | null) =>
     dropTarget(
       (item) =>
         item.kind === "folder"
           ? { key: foldersEndKey(parentId), parentId, before: null }
-          : { key: `c:${id}`, parentId, before: { kind: "connection", id } },
+          : { key: `c:${id}`, parentId, before: { kind: "installation", id } },
       [`c:${id}`, foldersEndKey(parentId)],
     );
 
@@ -387,7 +371,7 @@ export function ConnectionTree() {
   /** Последняя папка уровня (подсветка «в конец папок»); null — папок на уровне нет. */
   const lastFolderId = (nodes: TreeNode[]) => nodes.reduce<string | null>((last, n) => (n.kind === "folder" ? n.folder.id : last), null);
 
-  const folderRowClass = (folder: ConnectionFolder, parentId: string | null, isLast: boolean, children: TreeNode[]) => {
+  const folderRowClass = (folder: InstallationFolder, parentId: string | null, isLast: boolean, children: TreeNode[]) => {
     const classes = ["tree-folder"];
     if (dropKey === `fb:${folder.id}`) classes.push("drop-before");
     // «Внутрь» — и когда папку бросают в конец папок этого уровня, а своих вложенных папок нет.
@@ -399,15 +383,15 @@ export function ConnectionTree() {
   const renderNodes = (nodes: TreeNode[], parentId: string | null) => {
     const last = lastFolderId(nodes);
     return nodes.map((node) => {
-      if (node.kind === "connection") {
-        const c = node.connection;
+      if (node.kind === "installation") {
+        const c = node.installation;
         return (
-          <ConnectionNode
+          <InstallationNode
             key={c.id}
-            connection={c}
+            installation={c}
             rowProps={{
-              ...dragSource({ kind: "connection", id: c.id }),
-              ...connectionRowTarget(c.id, parentId),
+              ...dragSource({ kind: "installation", id: c.id }),
+              ...installationRowTarget(c.id, parentId),
               className: dropKey === `c:${c.id}` ? "drop-before" : undefined,
             }}
           />
@@ -473,17 +457,17 @@ export function ConnectionTree() {
     });
   };
 
-  const error = connectionsQ.error ?? foldersQ.error;
+  const error = installationsQ.error ?? foldersQ.error;
   const rootHighlight = dropKey === "root" || (dropKey === foldersEndKey(null) && lastFolderId(visible) === null);
-  // Пока не загружены и подключения, и папки, подсказку о пустом списке не показываем.
-  const empty = connectionsQ.isSuccess && foldersQ.isSuccess && (connections ?? []).length === 0 && (folders ?? []).length === 0;
+  // Пока не загружены и инсталляции, и папки, подсказку о пустом списке не показываем.
+  const empty = installationsQ.isSuccess && foldersQ.isSuccess && (installations ?? []).length === 0 && (folders ?? []).length === 0;
 
   return (
     <>
       <div className={rootHighlight ? "sidebar-header drop-into" : "sidebar-header"} {...rootTarget}>
-        <span>{t("sidebar.connections")}</span>
+        <span>{t("sidebar.installations")}</span>
         <span className="sidebar-actions">
-          <button type="button" title={t("sidebar.newConnection")} onClick={() => open({ kind: "connection-form", connectionId: null })}>
+          <button type="button" title={t("sidebar.newInstallation")} onClick={() => open({ kind: "installation-form", installationId: null })}>
             +
           </button>
           <button type="button" title={t("sidebar.newFolder")} aria-label={t("sidebar.newFolder")} onClick={() => startCreate(null)}>
@@ -538,9 +522,6 @@ export function ConnectionTree() {
       {importing && (
         <ImportDialog path={importing.path} initialTargetId={importing.targetId} onClose={() => setImporting(null)} onImported={onImported} />
       )}
-      {kubeImport && (
-        <KubeconfigImportDialog initialTargetId={kubeImport.targetId} onClose={() => setKubeImport(null)} onImported={onKubeImported} />
-      )}
     </>
   );
 }
````

<!-- block: patch t14-port-src-components-installations-tree-installationtree-test-tsx step=14.1 -->
````diff
diff --git a/src/components/installations/tree/InstallationTree.test.tsx b/src/components/installations/tree/InstallationTree.test.tsx
--- a/src/components/installations/tree/InstallationTree.test.tsx
+++ b/src/components/installations/tree/InstallationTree.test.tsx
@@ -2,27 +2,24 @@ import { act, createEvent, fireEvent, screen, waitFor, within } from "@testing-l
 import userEvent from "@testing-library/user-event";
 import { StrictMode } from "react";
 import { beforeEach, describe, expect, it, vi } from "vitest";
-import type { ConnectionFolder, ConnectionProfile } from "../../../api/types";
+import type { InstallationFolder, InstallationProfile } from "../../../api/types";
 import { renderWithClient } from "../../../test/render";
-import { newProfile } from "../profile";
+import { newInstallation } from "../profile";
 
 vi.mock("../../../api/client", async (importOriginal) => {
   const orig = await importOriginal<typeof import("../../../api/client")>();
   return {
     ...orig,
     api: {
-      getHealth: vi.fn().mockResolvedValue({ lastCheck: null, ignored: [], annotations: [] }),
       listFolders: vi.fn(),
-      listConnections: vi.fn(),
+      listInstallations: vi.fn(),
       createFolder: vi.fn(),
       renameFolder: vi.fn(),
       deleteFolder: vi.fn(),
       moveItems: vi.fn(),
-      exportConnections: vi.fn(),
+      exportInstallations: vi.fn(),
       previewImport: vi.fn(),
       applyImport: vi.fn(),
-      previewKubeconfigImport: vi.fn(),
-      applyKubeconfigImport: vi.fn(),
     },
   };
 });
@@ -31,22 +28,22 @@ vi.mock("@tauri-apps/plugin-dialog", () => ({ save: vi.fn(), open: vi.fn() }));
 import { open as openDialog, save } from "@tauri-apps/plugin-dialog";
 import { api } from "../../../api/client";
 import { useWorkspace } from "../../../state/workspace";
-import { ConnectionTree } from "./ConnectionTree";
+import { InstallationTree } from "./InstallationTree";
 
-const FOLDERS: ConnectionFolder[] = [
+const FOLDERS: InstallationFolder[] = [
   { id: "a", name: "Проект А", parentId: null, order: 0 },
   { id: "b", name: "ИФТ", parentId: "a", order: 0 },
 ];
-const CONNS: ConnectionProfile[] = [
-  { ...newProfile(), id: "x", name: "ocp-ift", folderId: "b", order: 0 },
-  { ...newProfile(), id: "y", name: "local", folderId: null, order: 0 },
+const CONNS: InstallationProfile[] = [
+  { ...newInstallation(), id: "x", name: "ocp-ift", folderId: "b", order: 0 },
+  { ...newInstallation(), id: "y", name: "local", folderId: null, order: 0 },
 ];
 
 beforeEach(() => {
   vi.clearAllMocks();
   localStorage.clear();
   vi.mocked(api.listFolders).mockResolvedValue(FOLDERS);
-  vi.mocked(api.listConnections).mockResolvedValue(CONNS);
+  vi.mocked(api.listInstallations).mockResolvedValue(CONNS);
   useWorkspace.setState({ tabs: [], activeId: null });
 });
 
@@ -79,9 +76,9 @@ async function openFolderMenu(name: string) {
   return { user, menu: screen.getByRole("menu") };
 }
 
-describe("ConnectionTree", () => {
-  it("показывает папки с подключениями и пункты подключения", async () => {
-    renderWithClient(<ConnectionTree />);
+describe("InstallationTree", () => {
+  it("показывает папки с инсталляциями и пункты инсталляции", async () => {
+    renderWithClient(<InstallationTree />);
     expect(await screen.findByText("ocp-ift")).toBeInTheDocument();
     expect(screen.getByRole("button", { name: "Развернуть или свернуть Проект А" })).toHaveAttribute("aria-expanded", "true");
     expect(screen.getByRole("button", { name: "Развернуть или свернуть ИФТ" })).toBeInTheDocument();
@@ -89,18 +86,18 @@ describe("ConnectionTree", () => {
     expect(screen.getAllByRole("button", { name: "Обзор" })).toHaveLength(2);
   });
 
-  it("«Обзор» открывает вкладку подключения", async () => {
-    vi.mocked(api.listConnections).mockResolvedValue([CONNS[1]]);
+  it("«Обзор» открывает вкладку инсталляции", async () => {
+    vi.mocked(api.listInstallations).mockResolvedValue([CONNS[1]]);
     const user = userEvent.setup();
-    renderWithClient(<ConnectionTree />);
+    renderWithClient(<InstallationTree />);
     await screen.findByText("local");
     await user.click(screen.getByRole("button", { name: "Обзор" }));
-    expect(useWorkspace.getState().activeId).toBe("cluster:y");
+    expect(useWorkspace.getState().activeId).toBe("installation:y");
   });
 
   it("сворачивание папки сохраняется между перерисовками", async () => {
     const user = userEvent.setup();
-    const first = renderWithClient(<ConnectionTree />);
+    const first = renderWithClient(<InstallationTree />);
     await screen.findByText("ocp-ift");
     await user.click(screen.getByRole("button", { name: "Развернуть или свернуть Проект А" }));
     expect(screen.queryByText("ocp-ift")).not.toBeInTheDocument();
@@ -108,7 +105,7 @@ describe("ConnectionTree", () => {
     expect(JSON.parse(localStorage.getItem("glafira.tree.collapsed") ?? "[]")).toEqual(["a"]);
     first.unmount();
 
-    renderWithClient(<ConnectionTree />);
+    renderWithClient(<InstallationTree />);
     expect(await screen.findByText("local")).toBeInTheDocument();
     expect(screen.queryByText("ocp-ift")).not.toBeInTheDocument();
   });
@@ -117,7 +114,7 @@ describe("ConnectionTree", () => {
     const user = userEvent.setup();
     renderWithClient(
       <StrictMode>
-        <ConnectionTree />
+        <InstallationTree />
       </StrictMode>,
     );
     await screen.findByText("ocp-ift");
@@ -128,21 +125,21 @@ describe("ConnectionTree", () => {
   });
 
   it("подсказка о пустом списке не мелькает, пока папки загружаются", async () => {
-    let resolveFolders: (f: ConnectionFolder[]) => void = () => {};
+    let resolveFolders: (f: InstallationFolder[]) => void = () => {};
     vi.mocked(api.listFolders).mockReturnValue(new Promise((r) => (resolveFolders = r)));
-    vi.mocked(api.listConnections).mockResolvedValue([]);
-    renderWithClient(<ConnectionTree />);
-    await waitFor(() => expect(api.listConnections).toHaveBeenCalled());
+    vi.mocked(api.listInstallations).mockResolvedValue([]);
+    renderWithClient(<InstallationTree />);
+    await waitFor(() => expect(api.listInstallations).toHaveBeenCalled());
     await act(async () => {
       await new Promise((r) => setTimeout(r, 20));
     });
-    expect(screen.queryByText(/Добавьте первое подключение/)).not.toBeInTheDocument();
+    expect(screen.queryByText(/Добавьте первую инсталляцию/)).not.toBeInTheDocument();
     resolveFolders([]);
-    expect(await screen.findByText(/Добавьте первое подключение/)).toBeInTheDocument();
+    expect(await screen.findByText(/Добавьте первую инсталляцию/)).toBeInTheDocument();
   });
 
   it("строка папки с открытым меню не перетаскивается", async () => {
-    renderWithClient(<ConnectionTree />);
+    renderWithClient(<InstallationTree />);
     await screen.findByText("ocp-ift");
     expect(folderRow("Проект А")).toHaveAttribute("draggable", "true");
     await openFolderMenu("Проект А");
@@ -152,18 +149,18 @@ describe("ConnectionTree", () => {
 
   it("испорченное значение в localStorage не ломает дерево", async () => {
     localStorage.setItem("glafira.tree.collapsed", "{not json");
-    renderWithClient(<ConnectionTree />);
+    renderWithClient(<InstallationTree />);
     expect(await screen.findByText("ocp-ift")).toBeInTheDocument();
   });
 
   it("фильтр скрывает несовпавшие, разворачивает папки и сообщает, если ничего нет", async () => {
     localStorage.setItem("glafira.tree.collapsed", JSON.stringify(["a"]));
     const user = userEvent.setup();
-    renderWithClient(<ConnectionTree />);
+    renderWithClient(<InstallationTree />);
     await screen.findByText("local");
     expect(screen.queryByText("ocp-ift")).not.toBeInTheDocument();
 
-    const filter = screen.getByRole("searchbox", { name: "Фильтр подключений" });
+    const filter = screen.getByRole("searchbox", { name: "Фильтр инсталляций" });
     await user.type(filter, "ift");
     expect(screen.getByText("ocp-ift")).toBeInTheDocument();
     expect(screen.queryByText("local")).not.toBeInTheDocument();
@@ -179,9 +176,9 @@ describe("ConnectionTree", () => {
     expect(screen.queryByText("ocp-ift")).not.toBeInTheDocument();
   });
 
-  it("подключения видны, даже если папки не загрузились", async () => {
+  it("инсталляции видны, даже если папки не загрузились", async () => {
     vi.mocked(api.listFolders).mockRejectedValue(new Error("boom"));
-    renderWithClient(<ConnectionTree />);
+    renderWithClient(<InstallationTree />);
     expect(await screen.findByText("ocp-ift")).toBeInTheDocument();
     expect(screen.getByText("local")).toBeInTheDocument();
   });
@@ -189,7 +186,7 @@ describe("ConnectionTree", () => {
   it("«+ папка» создаёт папку в корне по Enter", async () => {
     vi.mocked(api.createFolder).mockResolvedValue({ id: "n", name: "ИФТ", parentId: null, order: 1 });
     const user = userEvent.setup();
-    renderWithClient(<ConnectionTree />);
+    renderWithClient(<InstallationTree />);
     await screen.findByText("local");
     await user.click(screen.getByRole("button", { name: "Новая папка" }));
     const input = screen.getByRole("textbox", { name: "Имя папки" });
@@ -202,7 +199,7 @@ describe("ConnectionTree", () => {
 
   it("Esc отменяет ввод имени", async () => {
     const user = userEvent.setup();
-    renderWithClient(<ConnectionTree />);
+    renderWithClient(<InstallationTree />);
     await screen.findByText("local");
     await user.click(screen.getByRole("button", { name: "Новая папка" }));
     await user.type(screen.getByRole("textbox", { name: "Имя папки" }), "x{Escape}");
@@ -218,7 +215,7 @@ describe("ConnectionTree", () => {
       message: "Folder “ИФТ” already exists here",
       retryable: false,
     });
-    renderWithClient(<ConnectionTree />);
+    renderWithClient(<InstallationTree />);
     await screen.findByText("ocp-ift");
     const { user } = await openFolderMenu("Проект А");
     await user.click(screen.getByRole("menuitem", { name: "Новая папка" }));
@@ -231,18 +228,17 @@ describe("ConnectionTree", () => {
 
   it("меню папки открывается правым кликом и закрывается по Esc", async () => {
     const user = userEvent.setup();
-    renderWithClient(<ConnectionTree />);
+    renderWithClient(<InstallationTree />);
     await screen.findByText("ocp-ift");
     fireEvent.contextMenu(folderRow("ИФТ"));
     const menu = screen.getByRole("menu");
     expect(within(menu).getAllByRole("menuitem").map((m) => m.textContent)).toEqual([
       "Новая папка",
-      "Новое подключение здесь",
+      "Новая инсталляция здесь",
       "Переименовать",
       "Удалить",
       "Экспортировать папку…",
       "Импортировать сюда…",
-      "Импорт из kubeconfig сюда…",
     ]);
     await user.keyboard("{Escape}");
     expect(screen.queryByRole("menu")).not.toBeInTheDocument();
@@ -250,7 +246,7 @@ describe("ConnectionTree", () => {
 
   it("меню закрывается кликом вне его", async () => {
     const user = userEvent.setup();
-    renderWithClient(<ConnectionTree />);
+    renderWithClient(<InstallationTree />);
     await screen.findByText("ocp-ift");
     await openFolderMenu("ИФТ");
     await user.click(screen.getByText("local"));
@@ -259,7 +255,7 @@ describe("ConnectionTree", () => {
 
   it("переименование через меню", async () => {
     vi.mocked(api.renameFolder).mockResolvedValue({ id: "a", name: "Проект Б", parentId: null, order: 0 });
-    renderWithClient(<ConnectionTree />);
+    renderWithClient(<InstallationTree />);
     await screen.findByText("ocp-ift");
     const { user } = await openFolderMenu("Проект А");
     await user.click(screen.getByRole("menuitem", { name: "Переименовать" }));
@@ -277,7 +273,7 @@ describe("ConnectionTree", () => {
       message: "Folder is not empty",
       retryable: false,
     });
-    renderWithClient(<ConnectionTree />);
+    renderWithClient(<InstallationTree />);
     await screen.findByText("ocp-ift");
     const { user } = await openFolderMenu("Проект А");
     await user.click(screen.getByRole("menuitem", { name: "Удалить" }));
@@ -293,7 +289,7 @@ describe("ConnectionTree", () => {
 
   it("удаление пустой папки", async () => {
     vi.mocked(api.deleteFolder).mockResolvedValue(undefined);
-    renderWithClient(<ConnectionTree />);
+    renderWithClient(<InstallationTree />);
     await screen.findByText("ocp-ift");
     const { user } = await openFolderMenu("ИФТ");
     await user.click(screen.getByRole("menuitem", { name: "Удалить" }));
@@ -302,20 +298,20 @@ describe("ConnectionTree", () => {
     await vi.waitFor(() => expect(screen.queryByRole("group")).not.toBeInTheDocument());
   });
 
-  it("«Новое подключение здесь» открывает форму с папкой", async () => {
-    renderWithClient(<ConnectionTree />);
+  it("«Новая инсталляция здесь» открывает форму с папкой", async () => {
+    renderWithClient(<InstallationTree />);
     await screen.findByText("ocp-ift");
     const { user } = await openFolderMenu("ИФТ");
-    await user.click(screen.getByRole("menuitem", { name: "Новое подключение здесь" }));
+    await user.click(screen.getByRole("menuitem", { name: "Новая инсталляция здесь" }));
     expect(useWorkspace.getState().tabs).toEqual([
-      expect.objectContaining({ kind: "connection-form", connectionId: null, folderId: "b" }),
+      expect.objectContaining({ kind: "installation-form", installationId: null, folderId: "b" }),
     ]);
     expect(screen.queryByRole("menu")).not.toBeInTheDocument();
   });
 
-  it("перетаскивание подключения на папку переносит его внутрь", async () => {
+  it("перетаскивание инсталляции на папку переносит его внутрь", async () => {
     vi.mocked(api.moveItems).mockResolvedValue(undefined);
-    renderWithClient(<ConnectionTree />);
+    renderWithClient(<InstallationTree />);
     await screen.findByText("ocp-ift");
     const dataTransfer = stubDataTransfer();
     fireEvent.dragStart(connRow("local"), { dataTransfer });
@@ -323,22 +319,22 @@ describe("ConnectionTree", () => {
     expect(notPrevented).toBe(false);
     expect(folderRow("ИФТ")).toHaveClass("drop-into");
     fireEvent.drop(folderRow("ИФТ"), { dataTransfer });
-    expect(api.moveItems).toHaveBeenCalledWith([{ kind: "connection", id: "y" }], "b", null);
+    expect(api.moveItems).toHaveBeenCalledWith([{ kind: "installation", id: "y" }], "b", null);
   });
 
-  it("перетаскивание на подключение ставит перед ним в его папку", async () => {
+  it("перетаскивание на инсталляция ставит перед ним в его папку", async () => {
     vi.mocked(api.moveItems).mockResolvedValue(undefined);
-    renderWithClient(<ConnectionTree />);
+    renderWithClient(<InstallationTree />);
     await screen.findByText("ocp-ift");
     const dataTransfer = stubDataTransfer();
     fireEvent.dragStart(connRow("local"), { dataTransfer });
     fireEvent.dragOver(connRow("ocp-ift"), { dataTransfer });
     fireEvent.drop(connRow("ocp-ift"), { dataTransfer });
-    expect(api.moveItems).toHaveBeenCalledWith([{ kind: "connection", id: "y" }], "b", { kind: "connection", id: "x" });
+    expect(api.moveItems).toHaveBeenCalledWith([{ kind: "installation", id: "y" }], "b", { kind: "installation", id: "x" });
   });
 
   it("папку нельзя бросить в её потомка", async () => {
-    renderWithClient(<ConnectionTree />);
+    renderWithClient(<InstallationTree />);
     await screen.findByText("ocp-ift");
     const dataTransfer = stubDataTransfer();
     fireEvent.dragStart(folderRow("Проект А"), { dataTransfer });
@@ -351,10 +347,10 @@ describe("ConnectionTree", () => {
 
   it("перетаскивание на заголовок панели переносит в корень", async () => {
     vi.mocked(api.moveItems).mockResolvedValue(undefined);
-    renderWithClient(<ConnectionTree />);
+    renderWithClient(<InstallationTree />);
     await screen.findByText("ocp-ift");
     const dataTransfer = stubDataTransfer();
-    const header = screen.getByText("Подключения").closest(".sidebar-header") as HTMLElement;
+    const header = screen.getByText("Инсталляции").closest(".sidebar-header") as HTMLElement;
     fireEvent.dragStart(folderRow("ИФТ"), { dataTransfer });
     expect(fireEvent.dragOver(header, { dataTransfer })).toBe(false);
     fireEvent.drop(header, { dataTransfer });
@@ -362,7 +358,7 @@ describe("ConnectionTree", () => {
   });
 
   describe("порядок папок и цели перетаскивания", () => {
-    const SIBLINGS: ConnectionFolder[] = [
+    const SIBLINGS: InstallationFolder[] = [
       { id: "A", name: "Альфа", parentId: null, order: 0 },
       { id: "B", name: "Бета", parentId: null, order: 1 },
       { id: "C", name: "Вложенная", parentId: "A", order: 0 },
@@ -379,7 +375,7 @@ describe("ConnectionTree", () => {
     });
 
     it("папка, брошенная на верх строки соседней папки, встаёт перед ней", async () => {
-      renderWithClient(<ConnectionTree />);
+      renderWithClient(<InstallationTree />);
       await screen.findByText("local");
       const target = folderRow("Альфа");
       stubRect(target);
@@ -393,7 +389,7 @@ describe("ConnectionTree", () => {
     });
 
     it("ниже верхней зоны строки папки — внутрь", async () => {
-      renderWithClient(<ConnectionTree />);
+      renderWithClient(<InstallationTree />);
       await screen.findByText("local");
       const target = folderRow("Альфа");
       stubRect(target);
@@ -405,8 +401,8 @@ describe("ConnectionTree", () => {
       expect(api.moveItems).toHaveBeenCalledWith([{ kind: "folder", id: "B" }], "A", null);
     });
 
-    it("подключение на верх строки папки кладётся внутрь (зона «перед» — только для папок)", async () => {
-      renderWithClient(<ConnectionTree />);
+    it("инсталляция на верх строки папки кладётся внутрь (зона «перед» — только для папок)", async () => {
+      renderWithClient(<InstallationTree />);
       await screen.findByText("local");
       const target = folderRow("Альфа");
       stubRect(target);
@@ -415,11 +411,11 @@ describe("ConnectionTree", () => {
       dragAt("dragOver", target, dataTransfer, 101);
       expect(target).toHaveClass("drop-into");
       dragAt("drop", target, dataTransfer, 101);
-      expect(api.moveItems).toHaveBeenCalledWith([{ kind: "connection", id: "y" }], "A", null);
+      expect(api.moveItems).toHaveBeenCalledWith([{ kind: "installation", id: "y" }], "A", null);
     });
 
-    it("папка на подключение — в конец папок этого уровня, подсвечивается последняя папка", async () => {
-      renderWithClient(<ConnectionTree />);
+    it("папка на инсталляция — в конец папок этого уровня, подсвечивается последняя папка", async () => {
+      renderWithClient(<InstallationTree />);
       await screen.findByText("local");
       const dataTransfer = stubDataTransfer();
       fireEvent.dragStart(folderRow("Вложенная"), { dataTransfer });
@@ -431,7 +427,7 @@ describe("ConnectionTree", () => {
     });
 
     it("папку нельзя поставить перед ней самой", async () => {
-      renderWithClient(<ConnectionTree />);
+      renderWithClient(<InstallationTree />);
       await screen.findByText("local");
       const target = folderRow("Бета");
       stubRect(target);
@@ -446,7 +442,7 @@ describe("ConnectionTree", () => {
   describe("доступность меню", () => {
     it("триггер: aria-haspopup и aria-expanded; Esc возвращает фокус и не уходит в document", async () => {
       const user = userEvent.setup();
-      renderWithClient(<ConnectionTree />);
+      renderWithClient(<InstallationTree />);
       await screen.findByText("ocp-ift");
       const trigger = within(folderRow("ИФТ")).getByRole("button", { name: "Ещё действия" });
       expect(trigger).toHaveAttribute("aria-haspopup", "menu");
@@ -469,9 +465,9 @@ describe("ConnectionTree", () => {
 
     it("Tab закрывает меню", async () => {
       const user = userEvent.setup();
-      renderWithClient(<ConnectionTree />);
+      renderWithClient(<InstallationTree />);
       await screen.findByText("ocp-ift");
-      const header = screen.getByText("Подключения").closest(".sidebar-header") as HTMLElement;
+      const header = screen.getByText("Инсталляции").closest(".sidebar-header") as HTMLElement;
       await user.click(within(header).getByRole("button", { name: "Ещё действия" }));
       expect(screen.getByRole("menu")).toBeInTheDocument();
       await user.tab();
@@ -479,7 +475,7 @@ describe("ConnectionTree", () => {
     });
 
     it("перетаскивание не начинается из меню", async () => {
-      renderWithClient(<ConnectionTree />);
+      renderWithClient(<InstallationTree />);
       await screen.findByText("ocp-ift");
       await openFolderMenu("ИФТ");
       const dataTransfer = stubDataTransfer();
@@ -493,7 +489,7 @@ describe("ConnectionTree", () => {
 
   it("пустое поле имени отменяется при потере фокуса, непустое остаётся", async () => {
     const user = userEvent.setup();
-    renderWithClient(<ConnectionTree />);
+    renderWithClient(<InstallationTree />);
     await screen.findByText("local");
     await user.click(screen.getByRole("button", { name: "Новая папка" }));
     fireEvent.blur(screen.getByRole("textbox", { name: "Имя папки" }));
@@ -510,52 +506,52 @@ describe("ConnectionTree", () => {
     async function openHeaderMenu() {
       const user = userEvent.setup();
       await screen.findByText("local");
-      const header = screen.getByText("Подключения").closest(".sidebar-header") as HTMLElement;
+      const header = screen.getByText("Инсталляции").closest(".sidebar-header") as HTMLElement;
       await user.click(within(header).getByRole("button", { name: "Ещё действия" }));
       return { user, menu: screen.getByRole("menu") };
     }
 
     it("«Экспортировать папку…» сохраняет папку и сообщает итог", async () => {
       vi.mocked(save).mockResolvedValue("/tmp/a.json");
-      vi.mocked(api.exportConnections).mockResolvedValue({ connections: 1, folders: 2 });
-      renderWithClient(<ConnectionTree />);
+      vi.mocked(api.exportInstallations).mockResolvedValue({ installations: 1, folders: 2 });
+      renderWithClient(<InstallationTree />);
       await screen.findByText("ocp-ift");
       const { user, menu } = await openFolderMenu("Проект А");
       await user.click(within(menu).getByRole("menuitem", { name: "Экспортировать папку…" }));
       expect(save).toHaveBeenCalledWith({
-        defaultPath: "Проект А.glafira-kubernetes-connections.json",
+        defaultPath: "Проект А.glafira-inference-installations.json",
         filters: [{ name: "Glafira", extensions: ["json"] }],
       });
-      expect(api.exportConnections).toHaveBeenCalledWith("/tmp/a.json", "a");
-      expect(await screen.findByRole("status")).toHaveTextContent("Сохранено: подключений 1, папок 2");
+      expect(api.exportInstallations).toHaveBeenCalledWith("/tmp/a.json", "a");
+      expect(await screen.findByRole("status")).toHaveTextContent("Сохранено: инсталляций 1, папок 2");
     });
 
     it("«Экспортировать всё…» передаёт null и имя файла с датой", async () => {
       vi.mocked(save).mockResolvedValue("/tmp/all.json");
-      vi.mocked(api.exportConnections).mockResolvedValue({ connections: 2, folders: 2 });
-      renderWithClient(<ConnectionTree />);
+      vi.mocked(api.exportInstallations).mockResolvedValue({ installations: 2, folders: 2 });
+      renderWithClient(<InstallationTree />);
       const { user, menu } = await openHeaderMenu();
       await user.click(within(menu).getByRole("menuitem", { name: "Экспортировать всё…" }));
       const arg = vi.mocked(save).mock.calls[0][0];
-      expect(arg?.defaultPath).toMatch(/^glafira-kubernetes-connections-\d{4}-\d{2}-\d{2}\.glafira-kubernetes-connections\.json$/);
-      expect(api.exportConnections).toHaveBeenCalledWith("/tmp/all.json", null);
+      expect(arg?.defaultPath).toMatch(/^glafira-inference-installations-\d{4}-\d{2}-\d{2}\.glafira-inference-installations\.json$/);
+      expect(api.exportInstallations).toHaveBeenCalledWith("/tmp/all.json", null);
     });
 
     it("отмена диалога сохранения ничего не вызывает", async () => {
       vi.mocked(save).mockResolvedValue(null);
-      renderWithClient(<ConnectionTree />);
+      renderWithClient(<InstallationTree />);
       await screen.findByText("ocp-ift");
       const { user, menu } = await openFolderMenu("ИФТ");
       await user.click(within(menu).getByRole("menuitem", { name: "Экспортировать папку…" }));
       await waitFor(() => expect(save).toHaveBeenCalled());
-      expect(api.exportConnections).not.toHaveBeenCalled();
+      expect(api.exportInstallations).not.toHaveBeenCalled();
       expect(screen.queryByRole("status")).not.toBeInTheDocument();
     });
 
     it("ошибка экспорта показывается баннером", async () => {
       vi.mocked(save).mockResolvedValue("/tmp/a.json");
-      vi.mocked(api.exportConnections).mockRejectedValue({ code: "INTERNAL", message: "disk full", retryable: false });
-      renderWithClient(<ConnectionTree />);
+      vi.mocked(api.exportInstallations).mockRejectedValue({ code: "INTERNAL", message: "disk full", retryable: false });
+      renderWithClient(<InstallationTree />);
       const { user, menu } = await openHeaderMenu();
       await user.click(within(menu).getByRole("menuitem", { name: "Экспортировать всё…" }));
       expect(await screen.findByRole("alert")).toHaveTextContent("disk full");
@@ -566,52 +562,32 @@ describe("ConnectionTree", () => {
       vi.mocked(api.previewImport).mockResolvedValue(EMPTY_PREVIEW);
       vi.mocked(api.applyImport).mockResolvedValue({
         ...EMPTY_PREVIEW,
-        added: [{ kind: "connection", id: "n", name: "n", path: [] }],
+        added: [{ kind: "installation", id: "n", name: "n", path: [] }],
         needsSecrets: 1,
       });
-      renderWithClient(<ConnectionTree />);
+      renderWithClient(<InstallationTree />);
       await screen.findByText("ocp-ift");
       const { user, menu } = await openFolderMenu("ИФТ");
       await user.click(within(menu).getByRole("menuitem", { name: "Импортировать сюда…" }));
       expect(openDialog).toHaveBeenCalledWith({ multiple: false, directory: false, filters: [{ name: "Glafira", extensions: ["json"] }] });
-      const dialog = await screen.findByRole("dialog", { name: "Импорт подключений" });
+      const dialog = await screen.findByRole("dialog", { name: "Импорт инсталляций" });
       await waitFor(() => expect(api.previewImport).toHaveBeenCalledWith("/tmp/in.json", "b", true));
       await user.click(within(dialog).getByRole("button", { name: "Импортировать" }));
       expect(api.applyImport).toHaveBeenCalledWith("/tmp/in.json", "b", true);
       await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
       const status = screen.getByRole("status");
       expect(status).toHaveTextContent("Импортировано: добавлено 1, обновлено 0");
-      expect(status).toHaveTextContent("Введите токен или ключ для 1 подключения");
+      expect(status).toHaveTextContent("Введите ключи API для 1 инсталляции");
     });
 
     it("«Импортировать…» в заголовке: отмена выбора файла не открывает диалог", async () => {
       vi.mocked(openDialog).mockResolvedValue(null);
-      renderWithClient(<ConnectionTree />);
+      renderWithClient(<InstallationTree />);
       const { user, menu } = await openHeaderMenu();
       await user.click(within(menu).getByRole("menuitem", { name: "Импортировать…" }));
       await waitFor(() => expect(openDialog).toHaveBeenCalled());
       expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
       expect(api.previewImport).not.toHaveBeenCalled();
     });
-
-    it("«Импорт из kubeconfig сюда…» открывает диалог с этой папкой и сообщает итог", async () => {
-      vi.mocked(api.previewKubeconfigImport).mockResolvedValue({
-        path: "/home/u/.kube/config",
-        contexts: [
-          { name: "dev", cluster: "c", server: "https://api.dev:6443", user: "u", namespace: null, auth: "token",
-            insecure: false, current: true, exec: null, unsupported: null, existingId: null },
-        ],
-      });
-      vi.mocked(api.applyKubeconfigImport).mockResolvedValue([{ ...newProfile(), id: "k", name: "dev" }]);
-      renderWithClient(<ConnectionTree />);
-      await screen.findByText("ocp-ift");
-      const { user, menu } = await openFolderMenu("ИФТ");
-      await user.click(within(menu).getByRole("menuitem", { name: "Импорт из kubeconfig сюда…" }));
-      const dialog = await screen.findByRole("dialog", { name: "Импорт из kubeconfig" });
-      await user.click(await within(dialog).findByRole("button", { name: "Импортировать 1 контекст" }));
-      expect(api.applyKubeconfigImport).toHaveBeenCalledWith("/home/u/.kube/config", ["dev"], "b", []);
-      await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
-      expect(screen.getByRole("status")).toHaveTextContent("Импортировано 1 подключение");
-    });
   });
 });
````

<!-- block: patch t14-port-src-components-installations-tree-importdialog-tsx step=14.1 -->
````diff
diff --git a/src/components/installations/tree/ImportDialog.tsx b/src/components/installations/tree/ImportDialog.tsx
--- a/src/components/installations/tree/ImportDialog.tsx
+++ b/src/components/installations/tree/ImportDialog.tsx
@@ -4,7 +4,6 @@ import { useTranslation } from "react-i18next";
 import { api } from "../../../api/client";
 import type { ImportPreview, PreviewItem } from "../../../api/types";
 import { tr } from "../../../i18n/text";
-import { restartSessions } from "../../../lib/afterSignIn";
 import { trapTab } from "../../../lib/focusTrap";
 import { ErrorBanner } from "../../common/ErrorBanner";
 import { Hint } from "../../common/Hint";
@@ -38,7 +37,7 @@ function Group({ title, items }: { title: string; items: string[] }) {
 }
 
 /**
- * Модальный диалог импорта подключений внутри страницы: цель, флажок «Обновлять существующие»,
+ * Модальный диалог импорта инсталляций внутри страницы: цель, флажок «Обновлять существующие»,
  * предпросмотр (повторяется при каждой смене цели или флажка) и применение.
  */
 export function ImportDialog({ path, initialTargetId, onClose, onImported }: Props) {
@@ -77,16 +76,13 @@ export function ImportDialog({ path, initialTargetId, onClose, onImported }: Pro
       const result = await api.applyImport(path, targetId, updateExisting);
       await Promise.all([
         queryClient.invalidateQueries({ queryKey: ["folders"] }),
-        queryClient.invalidateQueries({ queryKey: ["connections"] }),
+        queryClient.invalidateQueries({ queryKey: ["installations"] }),
       ]);
-      // Бэкенд сбросил клиентов обновлённых подключений, а открытая вкладка показывает прежние адрес
-      // и вход: сброс запросов перезапрашивает данные у смонтированных подписчиков (удаление кэша
-      // им ничего бы не сообщило). Ответ не ждём — диалог не зависит от связи с кластером.
-      const updated = new Set(result.updated.filter((i) => i.kind === "connection").map((i) => i.id));
+      // Бэкенд забыл подключения обновлённых инсталляций, а открытая вкладка показывает прежние адреса:
+      // сброс запросов перезапрашивает данные у смонтированных подписчиков (удаление кэша им ничего бы
+      // не сообщило). Ответ не ждём — диалог не зависит от связи с серверами.
+      const updated = new Set(result.updated.filter((i) => i.kind === "installation").map((i) => i.id));
       void queryClient.resetQueries({ predicate: (q) => typeof q.queryKey[1] === "string" && updated.has(q.queryKey[1]) });
-      // Сессии watch и логов обновлённых подключений остановлены (`stop_for`) без события: открытые списки перезапускаются.
-      // Добавленные и не изменившиеся подключения Rust не трогает.
-      restartSessions(updated);
       onImported(result);
     } catch (err) {
       setApplyError(err);
@@ -114,7 +110,7 @@ export function ImportDialog({ path, initialTargetId, onClose, onImported }: Pro
             <Hint k="hints.import.target" label={t("exchange.import.target")} />
           </span>
           <select id={`${titleId}-target`} value={targetId ?? ""} disabled={busy} onChange={(e) => setTargetId(e.target.value || null)}>
-            <option value="">{t("connections.form.folderRoot")}</option>
+            <option value="">{t("installations.form.folderRoot")}</option>
             {paths.map((p) => (
               <option key={p.id} value={p.id}>
                 {p.path.join(" / ")}
````

<!-- block: patch t14-port-src-components-installations-tree-importdialog-test-tsx step=14.1 -->
````diff
diff --git a/src/components/installations/tree/ImportDialog.test.tsx b/src/components/installations/tree/ImportDialog.test.tsx
--- a/src/components/installations/tree/ImportDialog.test.tsx
+++ b/src/components/installations/tree/ImportDialog.test.tsx
@@ -2,8 +2,7 @@ import { QueryClientProvider, useQuery } from "@tanstack/react-query";
 import { screen, waitFor, within } from "@testing-library/react";
 import userEvent from "@testing-library/user-event";
 import { beforeEach, describe, expect, it, vi } from "vitest";
-import type { ConnectionFolder, ImportPreview } from "../../../api/types";
-import { useConnectionEpoch } from "../../../state/connectionEpoch";
+import type { InstallationFolder, ImportPreview } from "../../../api/types";
 import { renderWithClient } from "../../../test/render";
 
 vi.mock("../../../api/client", async (importOriginal) => {
@@ -11,7 +10,7 @@ vi.mock("../../../api/client", async (importOriginal) => {
   return {
     ...orig,
     api: {
-      connect: vi.fn(),
+      connectInstallation: vi.fn(),
       listFolders: vi.fn(),
       previewImport: vi.fn(),
       applyImport: vi.fn(),
@@ -22,7 +21,7 @@ vi.mock("../../../api/client", async (importOriginal) => {
 import { api } from "../../../api/client";
 import { ImportDialog } from "./ImportDialog";
 
-const FOLDERS: ConnectionFolder[] = [
+const FOLDERS: InstallationFolder[] = [
   { id: "a", name: "Проект А", parentId: null, order: 0 },
   { id: "b", name: "ИФТ", parentId: "a", order: 0 },
 ];
@@ -30,31 +29,30 @@ const FOLDERS: ConnectionFolder[] = [
 const PREVIEW: ImportPreview = {
   added: [
     { kind: "folder", id: "f1", name: "Новая", path: [] },
-    { kind: "connection", id: "c1", name: "ocp-new", path: ["Новая"] },
+    { kind: "installation", id: "c1", name: "ocp-new", path: ["Новая"] },
   ],
-  updated: [{ kind: "connection", id: "c2", name: "ocp-upd", path: ["Проект А"] }],
+  updated: [{ kind: "installation", id: "c2", name: "ocp-upd", path: ["Проект А"] }],
   unchanged: [],
   skipped: [
     {
       name: "broken",
-      reason: { key: "errors.import.unsupportedConnection", params: { error: "oops" }, text: "Unsupported: oops" },
+      reason: { key: "errors.import.unsupportedInstallation", params: { error: "oops" }, text: "Unsupported: oops" },
     },
   ],
   warnings: [{ key: "errors.generic", params: { message: "Предупреждение из файла" }, text: "Warning from the file" }],
   needsSecrets: 3,
 };
 
-const PATH = "/tmp/in.glafira-kubernetes-connections.json";
+const PATH = "/tmp/in.glafira-inference-installations.json";
 
-/** Открытая вкладка подключения «c2» (оно обновляется импортом): её данные подписаны на `["connection", id]`. */
+/** Открытая вкладка инсталляции «c2» (она обновляется импортом): её данные подписаны на `["installation", id]`. */
 function OpenTab() {
-  const q = useQuery({ queryKey: ["connection", "c2"], queryFn: () => api.connect("c2", false) });
+  const q = useQuery({ queryKey: ["installation", "c2"], queryFn: () => api.connectInstallation("c2", false) });
   return <output aria-label="Вкладка">{q.data === undefined ? "…" : String(q.data)}</output>;
 }
 
 beforeEach(() => {
   vi.clearAllMocks();
-  useConnectionEpoch.setState({ epochs: {} });
   vi.mocked(api.listFolders).mockResolvedValue(FOLDERS);
   vi.mocked(api.previewImport).mockResolvedValue(PREVIEW);
   vi.mocked(api.applyImport).mockResolvedValue(PREVIEW);
@@ -70,7 +68,7 @@ function renderDialog(targetId: string | null = null) {
 describe("ImportDialog", () => {
   it("показывает сводку и группы из предпросмотра", async () => {
     renderDialog();
-    const dialog = screen.getByRole("dialog", { name: "Импорт подключений" });
+    const dialog = screen.getByRole("dialog", { name: "Импорт инсталляций" });
     expect(await within(dialog).findByText("Добавится: 2 · Обновится: 1 · Без изменений: 0 · Пропущено: 1")).toBeInTheDocument();
     expect(api.previewImport).toHaveBeenCalledWith(PATH, null, true);
     expect(within(dialog).getByText("Новые (2)")).toBeInTheDocument();
@@ -90,7 +88,7 @@ describe("ImportDialog", () => {
     expect(within(select).getByRole("option", { name: "Проект А / ИФТ" })).toBeInTheDocument();
     await user.selectOptions(select, "b");
     await waitFor(() => expect(api.previewImport).toHaveBeenLastCalledWith(PATH, "b", true));
-    await user.click(screen.getByRole("checkbox", { name: "Обновлять существующие подключения" }));
+    await user.click(screen.getByRole("checkbox", { name: "Обновлять существующие инсталляции" }));
     await waitFor(() => expect(api.previewImport).toHaveBeenLastCalledWith(PATH, "b", false));
   });
 
@@ -103,7 +101,7 @@ describe("ImportDialog", () => {
   it("показывает предупреждения и строку о секретах", async () => {
     renderDialog();
     expect(await screen.findByText("Предупреждение из файла")).toBeInTheDocument();
-    expect(screen.getByText("После импорта введите токены или ключи для 3 подключений")).toBeInTheDocument();
+    expect(screen.getByText("После импорта введите ключи API для 3 инсталляций")).toBeInTheDocument();
   });
 
   it("«Импортировать» применяет импорт с теми же аргументами и закрывает диалог", async () => {
@@ -111,22 +109,22 @@ describe("ImportDialog", () => {
     const { onImported, client } = renderDialog();
     const invalidate = vi.spyOn(client, "invalidateQueries");
     await screen.findByText(/^Добавится:/);
-    await user.click(screen.getByRole("checkbox", { name: "Обновлять существующие подключения" }));
+    await user.click(screen.getByRole("checkbox", { name: "Обновлять существующие инсталляции" }));
     await waitFor(() => expect(api.previewImport).toHaveBeenLastCalledWith(PATH, null, false));
     await user.click(screen.getByRole("button", { name: "Импортировать" }));
     expect(api.applyImport).toHaveBeenCalledWith(PATH, null, false);
     await waitFor(() => expect(onImported).toHaveBeenCalledWith(PREVIEW));
-    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["connections"] });
+    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["installations"] });
     expect(invalidate).toHaveBeenCalledWith({ queryKey: ["folders"] });
   });
 
-  it("предупреждает о смене адреса подключения с сохранёнными учётными данными", async () => {
+  it("предупреждает о смене адреса сервера с сохранённым ключом API", async () => {
     vi.mocked(api.previewImport).mockResolvedValue({
       ...PREVIEW,
       warnings: [
         {
           key: "messages.import.addressChanged",
-          params: { name: "prod", from: "https://api.old:6443", to: "https://api.new:6443" },
+          params: { name: "prod", server: "gpu-1", from: "http://gpu-1:8000", to: "http://gpu-9:8000" },
           text: "English fallback",
         },
       ],
@@ -134,34 +132,34 @@ describe("ImportDialog", () => {
     renderDialog();
     expect(
       await screen.findByText(
-        "«prod»: адрес API меняется с https://api.old:6443 на https://api.new:6443. Сохранённые учётные данные (токен, ключ или подтверждённый exec-плагин) будут отправлены на новый адрес.",
+        "«prod», сервер «gpu-1»: адрес меняется с http://gpu-1:8000 на http://gpu-9:8000. Сохранённый ключ API уйдёт на новый адрес.",
       ),
     ).toBeInTheDocument();
   });
 
-  it("после импорта сбрасывает кэш обновлённых подключений", async () => {
+  it("после импорта сбрасывает кэш обновлённых инсталляций", async () => {
     let cached: unknown[] = [];
     const onImported = vi.fn(() => {
-      cached = [rendered.client.getQueryData(["connection", "c2"]), rendered.client.getQueryData(["namespaces", "c2"])];
+      cached = [rendered.client.getQueryData(["installation", "c2"]), rendered.client.getQueryData(["namespaces", "c2"])];
     });
     const user = userEvent.setup();
     const rendered = renderWithClient(<ImportDialog path={PATH} initialTargetId={null} onClose={vi.fn()} onImported={onImported} />);
-    rendered.client.setQueryData(["connection", "c2"], { stale: true });
+    rendered.client.setQueryData(["installation", "c2"], { stale: true });
     rendered.client.setQueryData(["namespaces", "c2"], ["a"]);
-    // Добавленное и посторонние подключения не затрагиваются.
-    rendered.client.setQueryData(["connection", "c1"], { keep: true });
-    rendered.client.setQueryData(["connection", "other"], { keep: true });
+    // Добавленное и посторонние инсталляции не затрагиваются.
+    rendered.client.setQueryData(["installation", "c1"], { keep: true });
+    rendered.client.setQueryData(["installation", "other"], { keep: true });
     await screen.findByText(/^Добавится:/);
     await user.click(screen.getByRole("button", { name: "Импортировать" }));
     await waitFor(() => expect(onImported).toHaveBeenCalled());
-    // К моменту onImported открытая вкладка подключения уже не видит прежний адрес и вход.
+    // К моменту onImported открытая вкладка инсталляции уже не видит прежний адрес и вход.
     expect(cached).toEqual([undefined, undefined]);
-    expect(rendered.client.getQueryData(["connection", "c1"])).toEqual({ keep: true });
-    expect(rendered.client.getQueryData(["connection", "other"])).toEqual({ keep: true });
+    expect(rendered.client.getQueryData(["installation", "c1"])).toEqual({ keep: true });
+    expect(rendered.client.getQueryData(["installation", "other"])).toEqual({ keep: true });
   });
 
-  it("после импорта открытая вкладка обновлённого подключения перезапрашивает данные", async () => {
-    vi.mocked(api.connect).mockResolvedValueOnce("сеанс истёк" as never).mockResolvedValue("подключено" as never);
+  it("после импорта открытая вкладка обновлённой инсталляции перезапрашивает данные", async () => {
+    vi.mocked(api.connectInstallation).mockResolvedValueOnce("сеанс истёк" as never).mockResolvedValue("подключено" as never);
     const user = userEvent.setup();
     renderWithClient(
       <>
@@ -172,49 +170,21 @@ describe("ImportDialog", () => {
     await waitFor(() => expect(screen.getByLabelText("Вкладка")).toHaveTextContent("сеанс истёк"));
     await screen.findByText(/^Добавится:/);
     await user.click(screen.getByRole("button", { name: "Импортировать" }));
-    // Подписчик ключа `["connection", "c2"]` получает свежие данные без повторного открытия вкладки.
+    // Подписчик ключа `["installation", "c2"]` получает свежие данные без повторного открытия вкладки.
     await waitFor(() => expect(screen.getByLabelText("Вкладка")).toHaveTextContent("подключено"));
-    expect(api.connect).toHaveBeenCalledTimes(2);
+    expect(api.connectInstallation).toHaveBeenCalledTimes(2);
   });
 
-  it("после импорта перезапускает watch и логи только обновлённых подключений", async () => {
-    // Rust остановил их сессии без события; добавленные, не изменившиеся и папки сессий не имеют.
-    vi.mocked(api.applyImport).mockResolvedValue({
-      ...PREVIEW,
-      updated: [
-        { kind: "connection", id: "c2", name: "ocp-upd", path: ["Проект А"] },
-        { kind: "folder", id: "f2", name: "Старая", path: [] },
-      ],
-      unchanged: [{ kind: "connection", id: "c3", name: "ocp-same", path: [] }],
-    });
-    const user = userEvent.setup();
-    const { onImported } = renderDialog();
-    await screen.findByText(/^Добавится:/);
-    await user.click(screen.getByRole("button", { name: "Импортировать" }));
-    await waitFor(() => expect(onImported).toHaveBeenCalled());
-    expect(useConnectionEpoch.getState().epochs).toEqual({ c2: 1 });
-  });
-
-  it("сбой применения не перезапускает сессии", async () => {
-    vi.mocked(api.applyImport).mockRejectedValue({ code: "INTERNAL", message: "Disk is full", retryable: false });
-    const user = userEvent.setup();
-    renderDialog();
-    await screen.findByText(/^Добавится:/);
-    await user.click(screen.getByRole("button", { name: "Импортировать" }));
-    expect(await screen.findByText("Disk is full")).toBeInTheDocument();
-    expect(useConnectionEpoch.getState().epochs).toEqual({});
-  });
-
-  it("сбой применения не сбрасывает кэш подключений", async () => {
+  it("сбой применения не сбрасывает кэш инсталляций", async () => {
     vi.mocked(api.applyImport).mockRejectedValue({ code: "INTERNAL", message: "Disk is full", retryable: false });
     const user = userEvent.setup();
     const { client } = renderWithClient(<ImportDialog path={PATH} initialTargetId={null} onClose={vi.fn()} onImported={vi.fn()} />);
-    client.setQueryData(["connection", "c2"], { keep: true });
+    client.setQueryData(["installation", "c2"], { keep: true });
     await screen.findByText(/^Добавится:/);
     await user.click(screen.getByRole("button", { name: "Импортировать" }));
     expect(await screen.findByText("Disk is full")).toBeInTheDocument();
     // Импорт записывается одной записью: после сбоя ничего не изменилось.
-    expect(client.getQueryData(["connection", "c2"])).toEqual({ keep: true });
+    expect(client.getQueryData(["installation", "c2"])).toEqual({ keep: true });
   });
 
   it("«Отмена» закрывает диалог без импорта", async () => {
````

<!-- block: patch t14-port-src-components-layout-test-tsx step=14.1 -->
````diff
diff --git a/src/components/Layout.test.tsx b/src/components/Layout.test.tsx
--- a/src/components/Layout.test.tsx
+++ b/src/components/Layout.test.tsx
@@ -1,6 +1,6 @@
 import { act, fireEvent, screen, within } from "@testing-library/react";
 import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
-import type { ConnectionProfile } from "../api/types";
+import type { InstallationProfile } from "../api/types";
 import { useHelpContext } from "../state/helpContext";
 import { useWorkspace } from "../state/workspace";
 import { renderWithClient } from "../test/render";
@@ -8,22 +8,19 @@ import { Layout } from "./Layout";
 
 vi.mock("../api/client", async (importOriginal) => {
   const actual = await importOriginal<typeof import("../api/client")>();
-  const conn = {
+  const inst: InstallationProfile = {
     id: "c1",
     name: "prod-1",
     color: "#888",
     environment: "prod",
     readOnly: true,
-    apiUrl: "https://api.prod:6443",
-    auth: { method: "browser" },
-    ca: { mode: "system" },
-    proxy: { mode: "system" },
-    favoriteNamespaces: [],
-    manualNamespaces: [],
+    servers: [],
     requestTimeoutMs: 5000,
     connectTimeoutMs: 5000,
-  } as unknown as ConnectionProfile;
-  return { ...actual, api: { ...actual.api, listConnections: vi.fn().mockResolvedValue([conn]), listFolders: vi.fn().mockResolvedValue([]) } };
+    folderId: null,
+    order: 0,
+  };
+  return { ...actual, api: { ...actual.api, listInstallations: vi.fn().mockResolvedValue([inst]), listFolders: vi.fn().mockResolvedValue([]) } };
 });
 
 // Вместо экрана — метка, показана ли его вкладка (контекст `Layout` для опросов и watch).
@@ -39,10 +36,10 @@ describe("Layout tabs", () => {
     useHelpContext.setState({ byTab: {} });
     useWorkspace.setState({
       tabs: [
-        { kind: "cluster", connectionId: "c1", id: "cluster:c1" },
-        { kind: "connection-form", connectionId: "c1", id: "connection-form:c1" },
+        { kind: "installation", installationId: "c1", id: "installation:c1" },
+        { kind: "installation-form", installationId: "c1", id: "installation-form:c1" },
       ],
-      activeId: "cluster:c1",
+      activeId: "installation:c1",
     });
   });
 
@@ -67,16 +64,16 @@ describe("Layout tabs", () => {
   it("tells each screen whether its tab is shown: hidden tabs stay mounted but must stop polling", () => {
     renderWithClient(<Layout />);
     const shown = (id: string) => screen.getByTestId(id).getAttribute("data-tab-active");
-    expect([shown("cluster:c1"), shown("connection-form:c1")]).toEqual(["true", "false"]);
-    expect(screen.getByTestId("connection-form:c1").closest("section")).toHaveAttribute("hidden");
+    expect([shown("installation:c1"), shown("installation-form:c1")]).toEqual(["true", "false"]);
+    expect(screen.getByTestId("installation-form:c1").closest("section")).toHaveAttribute("hidden");
     fireEvent.click(within(screen.getByRole("tablist")).getAllByRole("tab")[1]);
-    expect([shown("cluster:c1"), shown("connection-form:c1")]).toEqual(["false", "true"]);
+    expect([shown("installation:c1"), shown("installation-form:c1")]).toEqual(["false", "true"]);
   });
 
   it("F1 opens the guide chapter of the active screen", () => {
     useWorkspace.setState({
-      tabs: [{ kind: "connection-form", connectionId: "c1", id: "connection-form:c1" }],
-      activeId: "connection-form:c1",
+      tabs: [{ kind: "installation-form", installationId: "c1", id: "installation-form:c1" }],
+      activeId: "installation-form:c1",
     });
     renderWithClient(<Layout />);
     const ev = new KeyboardEvent("keydown", { key: "F1", bubbles: true, cancelable: true });
@@ -86,7 +83,7 @@ describe("Layout tabs", () => {
     expect(ev.defaultPrevented).toBe(true);
     const { tabs, activeId } = useWorkspace.getState();
     expect(activeId).toBe("help");
-    expect(tabs.find((t) => t.id === "help")).toMatchObject({ kind: "help", chapter: "connections" });
+    expect(tabs.find((t) => t.id === "help")).toMatchObject({ kind: "help", chapter: "installations" });
     expect(within(screen.getByRole("tablist")).getByRole("tab", { name: "Справка" })).toBeInTheDocument();
   });
 
@@ -95,10 +92,10 @@ describe("Layout tabs", () => {
       tabs: [{ kind: "settings", id: "settings" }],
       activeId: "settings",
     });
-    useHelpContext.setState({ byTab: { settings: "connections" } });
+    useHelpContext.setState({ byTab: { settings: "installations" } });
     renderWithClient(<Layout />);
     fireEvent.keyDown(window, { key: "F1" });
-    expect(useWorkspace.getState().tabs.find((t) => t.id === "help")).toMatchObject({ chapter: "connections" });
+    expect(useWorkspace.getState().tabs.find((t) => t.id === "help")).toMatchObject({ chapter: "installations" });
   });
 
   it("opens the guide from the sidebar", () => {
````

<!-- block: patch t14-port-src-components-sidebar-tsx step=14.1 -->
````diff
diff --git a/src/components/Sidebar.tsx b/src/components/Sidebar.tsx
--- a/src/components/Sidebar.tsx
+++ b/src/components/Sidebar.tsx
@@ -1,8 +1,7 @@
 import { useTranslation } from "react-i18next";
 import { useWorkspace } from "../state/workspace";
 import { openSupport } from "../lib/links";
-import { ConnectionTree } from "./connections/tree/ConnectionTree";
-import { McpIndicator } from "./mcp/McpIndicator";
+import { InstallationTree } from "./installations/tree/InstallationTree";
 
 export function Sidebar() {
   const { t } = useTranslation();
@@ -10,9 +9,8 @@ export function Sidebar() {
 
   return (
     <aside className="sidebar">
-      <ConnectionTree />
+      <InstallationTree />
       <div className="sidebar-footer">
-        <McpIndicator />
         <button type="button" className="link" onClick={() => open({ kind: "settings" })}>
           {t("sidebar.settings")}
         </button>
````

<!-- block: patch t14-port-src-components-sidebar-test-tsx step=14.1 -->
````diff
diff --git a/src/components/Sidebar.test.tsx b/src/components/Sidebar.test.tsx
--- a/src/components/Sidebar.test.tsx
+++ b/src/components/Sidebar.test.tsx
@@ -1,9 +1,8 @@
 import { screen } from "@testing-library/react";
 import userEvent from "@testing-library/user-event";
 import { describe, expect, it, vi } from "vitest";
-import { mcpStatus } from "../test/mcp";
 import { renderWithClient } from "../test/render";
-import { newProfile } from "./connections/profile";
+import { newInstallation } from "./installations/profile";
 
 vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
 vi.mock("../api/client", async (importOriginal) => {
@@ -11,10 +10,8 @@ vi.mock("../api/client", async (importOriginal) => {
   return {
     ...orig,
     api: {
-      getHealth: vi.fn().mockResolvedValue({ lastCheck: null, ignored: [], annotations: [] }),
-      listConnections: vi.fn().mockResolvedValue([{ ...newProfile(), id: "c", name: "local" }]),
+      listInstallations: vi.fn().mockResolvedValue([{ ...newInstallation(), id: "c", name: "local" }]),
       listFolders: vi.fn().mockResolvedValue([]),
-      getMcpStatus: vi.fn().mockResolvedValue(mcpStatus()),
     },
   };
 });
@@ -28,8 +25,7 @@ describe("Sidebar", () => {
     const user = userEvent.setup();
     const { container } = renderWithClient(<Sidebar />);
     expect(await screen.findByRole("button", { name: "Обзор" })).toBeEnabled();
-    // McpIndicator без сессий MCP ничего не рисует; на случай сессий исключаем его кнопку.
-    const footer = [...container.querySelectorAll<HTMLButtonElement>(".sidebar-footer button:not(.mcp-indicator)")].map((b) => b.textContent);
+    const footer = [...container.querySelectorAll<HTMLButtonElement>(".sidebar-footer button")].map((b) => b.textContent);
     expect(footer).toEqual(["Настройки", "Поддержать проект", "Справка", "О программе"]);
     const links = screen.getAllByRole("button").map((b) => b.textContent);
     expect(links.some((l) => l?.startsWith("Лицензия"))).toBe(false);
````

- [ ] **Step 2: Тесты.**

<!-- block: patch t14-tests step=14.2 -->
````diff
diff --git a/src/App.test.tsx b/src/App.test.tsx
index 899042c..c0ffcff 100644
--- a/src/App.test.tsx
+++ b/src/App.test.tsx
@@ -1,14 +1,22 @@
 import { render, screen } from "@testing-library/react";
 import { vi } from "vitest";
 
-vi.mock("./api/client", () => ({
-  api: { getLanguageSettings: vi.fn(() => new Promise(() => {})) },
-  toAppError: (e: unknown) => e,
-}));
+vi.mock("./api/client", async (importOriginal) => {
+  const orig = await importOriginal<typeof import("./api/client")>();
+  return {
+    ...orig,
+    api: {
+      getLanguageSettings: vi.fn(() => new Promise(() => {})),
+      listInstallations: vi.fn().mockResolvedValue([]),
+      listFolders: vi.fn().mockResolvedValue([]),
+    },
+  };
+});
 
 import App from "./App";
 
-test("показывает имя продукта", () => {
+test("первый запуск: пустое дерево с подсказкой и пустая рабочая область", async () => {
   render(<App />);
-  expect(screen.getByText("Glafira for LLM Inference")).toBeInTheDocument();
+  expect(await screen.findByText(/Добавьте первую инсталляцию/)).toBeInTheDocument();
+  expect(screen.getByText("Выберите инсталляцию слева или добавьте новую.")).toBeInTheDocument();
 });
````

- [ ] **Step 3: Увидеть падение.**

Run: `pnpm vitest run src/App.test.tsx`  
Expected: FAIL — нет модулей `./InstallationNode` и `./Sidebar` в новом виде (тесты ждут дерева инсталляций).

- [ ] **Step 4: Реализация.**

<!-- block: file t14-src-components-layout-tsx step=14.4 path=src/components/Layout.tsx -->
````tsx
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { api } from "../api/client";
import type { InstallationProfile } from "../api/types";
import { ENV_COLORS } from "../lib/env";
import { type Tab, tabInstallation, useWorkspace } from "../state/workspace";
import { EnvBadge } from "./common/EnvBadge";
import { Sidebar } from "./Sidebar";
import { i18n } from "../i18n";
import { TabContent } from "./TabContent";
import { TabActiveContext } from "../lib/tabActive";
import { useHelpShortcut } from "../lib/useHelpShortcut";

function tabTitle(tab: Tab, byId: Map<string, InstallationProfile>): string {
  const instId = tabInstallation(tab);
  const name = instId ? byId.get(instId)?.name ?? "?" : "";
  switch (tab.kind) {
    case "settings":
      return i18n.t("tabs.settings");
    case "about":
      return i18n.t("tabs.about");
    case "help":
      return i18n.t("tabs.help");
    case "installation-form":
      return tab.installationId ? i18n.t("tabs.installationForm", { name }) : i18n.t("tabs.newInstallation");
    case "installation":
      return name;
  }
}

export function Layout() {
  const { t } = useTranslation(); // заодно перерисовывает заголовки вкладок при смене языка
  const { tabs, activeId, activate, close } = useWorkspace();
  useHelpShortcut();
  const { data: installations = [] } = useQuery({ queryKey: ["installations"], queryFn: api.listInstallations });
  const byId = new Map(installations.map((c) => [c.id, c]));
  const active = tabs.find((x) => x.id === activeId);
  const activeInstId = active ? tabInstallation(active) : null;
  const activeInst = activeInstId ? byId.get(activeInstId) : undefined;

  return (
    <div className="layout">
      <Sidebar />
      <main className="workspace">
        <div className="env-stripe" style={{ background: activeInst ? ENV_COLORS[activeInst.environment] : "transparent" }} />
        <nav className="tabbar" role="tablist">
          {tabs.map((tab) => {
            const tInstId = tabInstallation(tab);
            const inst = tInstId ? byId.get(tInstId) : undefined;
            const isActive = tab.id === activeId;
            return (
              <div key={tab.id} className={isActive ? "tab active" : "tab"}>
                {inst && <EnvBadge environment={inst.environment} readOnly={inst.readOnly} />}
                <button type="button" role="tab" aria-selected={isActive} onClick={() => activate(tab.id)}>
                  {tabTitle(tab, byId)}
                </button>
                <button
                  type="button"
                  className="icon"
                  aria-label={t("layout.closeTab")}
                  onClick={(e) => {
                    e.stopPropagation();
                    close(tab.id);
                  }}
                >
                  ×
                </button>
              </div>
            );
          })}
        </nav>
        {tabs.length === 0 && <p className="empty">{t("layout.empty")}</p>}
        {/* Неактивные вкладки скрыты, но не размонтированы — подключения и состояние форм сохраняются. */}
        {/* Экраны узнают о скрытии из контекста: опрос серверов (с этапа I1b) останавливается. */}
        {tabs.map((tab) => (
          <section key={tab.id} className="tab-content" hidden={tab.id !== activeId}>
            <TabActiveContext.Provider value={tab.id === activeId}>
              <TabContent tab={tab} />
            </TabActiveContext.Provider>
          </section>
        ))}
      </main>
    </div>
  );
}
````

<!-- block: file t14-src-components-tabcontent-tsx step=14.4 path=src/components/TabContent.tsx -->
````tsx
import { Component, lazy, type ReactNode, Suspense } from "react";
import { useTranslation } from "react-i18next";
import type { Tab } from "../state/workspace";
import { AboutPanel } from "./about/AboutPanel";
import { InstallationFormTab } from "./installations/InstallationFormTab";
import { InstallationPanel } from "./installations/InstallationPanel";
import { SettingsPanel } from "./settings/SettingsPanel";

// Справка (react-markdown и главы) грузится отдельным чанком при первом открытии.
const HelpPanel = lazy(() => import("./help/HelpPanel").then((m) => ({ default: m.HelpPanel })));

/** Сбой чанка или рендера справки показывает «Не удалось загрузить руководство», не роняя окно. */
class HelpBoundary extends Component<{ fallback: ReactNode; children: ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  render() {
    return this.state.failed ? this.props.fallback : this.props.children;
  }
}

function LazyHelp({ tab }: { tab: Extract<Tab, { kind: "help" }> }) {
  const { t } = useTranslation();
  return (
    <HelpBoundary fallback={<p className="empty">{t("help.loadFailed")}</p>}>
      <Suspense fallback={<p className="empty">{t("common.loading")}</p>}>
        <HelpPanel spec={tab} />
      </Suspense>
    </HelpBoundary>
  );
}

export function TabContent({ tab }: { tab: Tab }) {
  switch (tab.kind) {
    case "about":
      return <AboutPanel />;
    case "help":
      return <LazyHelp tab={tab} />;
    case "settings":
      return <SettingsPanel />;
    case "installation-form":
      return <InstallationFormTab installationId={tab.installationId} folderId={tab.folderId ?? null} tabId={tab.id} />;
    case "installation":
      return <InstallationPanel installationId={tab.installationId} />;
  }
}
````

<!-- block: file t14-src-components-installations-tree-installationnode-tsx step=14.4 path=src/components/installations/tree/InstallationNode.tsx -->
````tsx
import { useQuery } from "@tanstack/react-query";
import type { HTMLAttributes } from "react";
import { useTranslation } from "react-i18next";
import { api } from "../../../api/client";
import type { InstallationProfile } from "../../../api/types";
import { useWorkspace } from "../../../state/workspace";
import { EnvBadge } from "../../common/EnvBadge";
import { installationQueryKey } from "../InstallationPanel";

/**
 * Узел инсталляции в дереве: точка цвета, имя, окружение, «Обзор» и серверы. Состояние серверов — из
 * подключения вкладки инсталляции; само дерево не подключается. rowProps — атрибуты строки (перетаскивание).
 */
export function InstallationNode({ installation: c, rowProps }: { installation: InstallationProfile; rowProps?: HTMLAttributes<HTMLDivElement> }) {
  const { t } = useTranslation();
  const open = useWorkspace((s) => s.open);
  const info = useQuery({ queryKey: installationQueryKey(c.id), queryFn: () => api.connectInstallation(c.id, false), enabled: false });
  const states = new Map((info.data?.servers ?? []).map((s) => [s.serverId, s]));
  const openTab = () => open({ kind: "installation", installationId: c.id });

  return (
    <li>
      <div {...rowProps} className={rowProps?.className ? `tree-conn ${rowProps.className}` : "tree-conn"}>
        <span className="dot" style={{ background: c.color }} />
        <span className="tree-name">{c.name}</span>
        <EnvBadge environment={c.environment} readOnly={c.readOnly} />
        <button type="button" className="icon" title={t("sidebar.editInstallation")} onClick={() => open({ kind: "installation-form", installationId: c.id })}>
          ✎
        </button>
      </div>
      <ul>
        <li>
          <button type="button" className="link" onClick={openTab}>
            {t("sidebar.overview")}
          </button>
        </li>
        {c.servers.map((s) => {
          const state = states.get(s.id);
          const mark = !state ? "○" : state.facts ? "●" : "✕";
          const status = !state ? t("sidebar.serverUnknown") : state.facts ? t("sidebar.serverOk") : t("sidebar.serverError");
          return (
            <li key={s.id}>
              <button type="button" className="link" title={s.baseUrl} onClick={openTab}>
                <span className={state ? (state.facts ? "srv-ok" : "srv-error") : "muted"} role="img" aria-label={status}>
                  {mark}
                </span>{" "}
                {s.name}
              </button>
            </li>
          );
        })}
      </ul>
    </li>
  );
}
````

<!-- block: patch t14-src step=14.4 -->
````diff
diff --git a/PORTED.md b/PORTED.md
index 67fba0e..b80cabe 100644
--- a/PORTED.md
+++ b/PORTED.md
@@ -115,3 +115,21 @@
 | `src/components/settings/SettingsPanel.test.tsx` | `src/components/settings/SettingsPanel.test.tsx` | `468e08a` | адаптирован: без раздела MCP |
 | `src/components/about/AboutPanel.tsx` | `src/components/about/AboutPanel.tsx` | `468e08a` | адаптирован: без обновлений (этап I2) |
 | `src/components/about/AboutPanel.test.tsx` | `src/components/about/AboutPanel.test.tsx` | `468e08a` | адаптирован: без обновлений |
+| `src/components/installations/tree/InstallationTree.tsx` | `src/components/connections/tree/ConnectionTree.tsx` | `468e08a` | адаптирован: инсталляции вместо подключений, без импорта kubeconfig |
+| `src/components/installations/tree/InstallationTree.test.tsx` | `src/components/connections/tree/ConnectionTree.test.tsx` | `468e08a` | адаптирован: инсталляции, без импорта kubeconfig |
+| `src/components/installations/tree/FolderMenu.tsx` | `src/components/connections/tree/FolderMenu.tsx` | `468e08a` | как есть |
+| `src/components/installations/tree/ImportDialog.tsx` | `src/components/connections/tree/ImportDialog.tsx` | `468e08a` | адаптирован: инсталляции, без перезапуска сессий |
+| `src/components/installations/tree/ImportDialog.test.tsx` | `src/components/connections/tree/ImportDialog.test.tsx` | `468e08a` | адаптирован: ключи API серверов, без сессий |
+| `src/components/installations/tree/InstallationNode.tsx` | `src/components/connections/tree/ConnectionNode.tsx` | `468e08a` | адаптирован: серверы инсталляции вместо проектов кластера |
+| `src/components/Layout.tsx` | `src/components/Layout.tsx` | `468e08a` | адаптирован: вкладки инсталляций, без MCP, проверки и обновлений |
+| `src/components/Layout.test.tsx` | `src/components/Layout.test.tsx` | `468e08a` | адаптирован: вкладки инсталляций |
+| `src/components/Sidebar.tsx` | `src/components/Sidebar.tsx` | `468e08a` | адаптирован: дерево инсталляций, без индикатора MCP |
+| `src/components/Sidebar.test.tsx` | `src/components/Sidebar.test.tsx` | `468e08a` | адаптирован: без MCP |
+| `src/components/TabContent.tsx` | `src/components/TabContent.tsx` | `468e08a` | адаптирован: вкладки инсталляций |
+| `src/components/TabContent.help.test.tsx` | `src/components/TabContent.help.test.tsx` | `468e08a` | как есть |
+| `src/lib/useAppMenu.ts` | `src/lib/useAppMenu.ts` | `468e08a` | как есть |
+| `src/lib/useAppMenu.test.ts` | `src/lib/useAppMenu.test.ts` | `468e08a` | как есть |
+| `src/lib/useHelpShortcut.ts` | `src/lib/useHelpShortcut.ts` | `468e08a` | как есть |
+| `src/lib/tabActive.ts` | `src/lib/tabActive.ts` | `468e08a` | как есть |
+| `src/lib/tabActive.test.tsx` | `src/lib/tabActive.test.tsx` | `468e08a` | как есть |
+| `src/guide/noLookbehind.test.ts` | `src/guide/noLookbehind.test.ts` | `468e08a` | как есть |
diff --git a/src/App.tsx b/src/App.tsx
index a5b7169..1d760f2 100644
--- a/src/App.tsx
+++ b/src/App.tsx
@@ -1,4 +1,6 @@
 import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
+import { Layout } from "./components/Layout";
+import { useAppMenu } from "./lib/useAppMenu";
 import { useLanguageSync } from "./lib/useLanguageSync";
 
 function LanguageSync() {
@@ -6,6 +8,12 @@ function LanguageSync() {
   return null;
 }
 
+/** Меню Help на macOS (на других системах ничего не делает). */
+function AppMenu() {
+  useAppMenu();
+  return null;
+}
+
 // Без автоповтора: backend сам ждёт до таймаута и быстро отказывает при недоступном сервере,
 // а повтор удваивал бы ожидание. Повторить можно кнопкой в баннере ошибки.
 // Без перезапроса при возвращении сети: вкладки остаются смонтированными, и событие online
@@ -18,7 +26,8 @@ export default function App() {
   return (
     <QueryClientProvider client={queryClient}>
       <LanguageSync />
-      <main className="page">Glafira for LLM Inference</main>
+      <AppMenu />
+      <Layout />
     </QueryClientProvider>
   );
 }
diff --git a/src/app.css b/src/app.css
index f931f88..5852acc 100644
--- a/src/app.css
+++ b/src/app.css
@@ -14,3 +14,6 @@ input::placeholder, textarea::placeholder { font-style: italic; color: var(--mut
 .facts dd { margin: 0; overflow-wrap: anywhere; }
 .kv { list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; gap: 4px; }
 .kv code { background: var(--bg-alt); border-radius: 3px; padding: 0 4px; overflow-wrap: anywhere; }
+/* Состояние сервера в дереве инсталляций. */
+.srv-ok { color: #16a34a; }
+.srv-error { color: var(--danger); }
````

<!-- block: locales t14-locales step=14.4 -->
````json
{
  "en": {
    "sidebar": {
      "installations": "Installations",
      "editInstallation": "Edit installation",
      "overview": "Overview",
      "emptyHint": "Add your first installation or import a file in the <code>⋯</code> menu.",
      "filter": "Filter installations",
      "folderToggle": "Expand or collapse {{name}}",
      "noMatches": "Nothing found",
      "menu": "More actions",
      "serverOk": "connected",
      "serverError": "error",
      "serverUnknown": "not connected yet",
      "folder": {
        "newInstallation": "New installation here",
        "rename": "Rename",
        "delete": "Delete",
        "deleteConfirm": "Delete folder “{{name}}”?",
        "namePlaceholder": "Folder name",
        "export": "Export folder…",
        "importHere": "Import here…"
      }
    },
    "tabs": {
      "settings": "Settings",
      "about": "About",
      "help": "Help",
      "installationForm": "Installation: {{name}}",
      "newInstallation": "New installation"
    },
    "layout": {
      "closeTab": "Close tab",
      "empty": "Select an installation on the left or add a new one."
    },
    "menu": {
      "help": "Help",
      "guide": "Glafira Guide",
      "about": "About Glafira"
    },
    "exchange": {
      "exported": "Saved: installations {{installations}}, folders {{folders}}",
      "imported": "Imported: added {{added}}, updated {{updated}}",
      "secretsNeeded_one": "Enter API keys for {{count}} installation",
      "secretsNeeded_other": "Enter API keys for {{count}} installations",
      "import": {
        "title": "Import installations",
        "target": "Import into",
        "updateExisting": "Update existing installations",
        "summary": "Added: {{added}} · Updated: {{updated}} · Unchanged: {{unchanged}} · Skipped: {{skipped}}",
        "groupAdded": "New",
        "groupUpdated": "Will be updated",
        "groupUnchanged": "Unchanged",
        "groupSkipped": "Skipped",
        "secrets_one": "Enter API keys for {{count}} installation after import",
        "secrets_other": "Enter API keys for {{count}} installations after import",
        "apply": "Import"
      }
    },
    "hints": {
      "import": {
        "target": "Where new folders and installations go. Existing ones stay where they are.",
        "updateExisting": "Installations that already exist get the settings from the file; your API keys and their place in the tree are kept."
      }
    }
  },
  "ru": {
    "sidebar": {
      "installations": "Инсталляции",
      "editInstallation": "Изменить инсталляцию",
      "overview": "Обзор",
      "emptyHint": "Добавьте первую инсталляцию или импортируйте файл из меню <code>⋯</code>.",
      "filter": "Фильтр инсталляций",
      "folderToggle": "Развернуть или свернуть {{name}}",
      "noMatches": "Ничего не найдено",
      "menu": "Ещё действия",
      "serverOk": "подключён",
      "serverError": "ошибка",
      "serverUnknown": "ещё не подключался",
      "folder": {
        "newInstallation": "Новая инсталляция здесь",
        "rename": "Переименовать",
        "delete": "Удалить",
        "deleteConfirm": "Удалить папку «{{name}}»?",
        "namePlaceholder": "Имя папки",
        "export": "Экспортировать папку…",
        "importHere": "Импортировать сюда…"
      }
    },
    "tabs": {
      "settings": "Настройки",
      "about": "О программе",
      "help": "Справка",
      "installationForm": "Инсталляция: {{name}}",
      "newInstallation": "Новая инсталляция"
    },
    "layout": {
      "closeTab": "Закрыть вкладку",
      "empty": "Выберите инсталляцию слева или добавьте новую."
    },
    "menu": {
      "help": "Справка",
      "guide": "Руководство Glafira",
      "about": "О программе"
    },
    "exchange": {
      "exported": "Сохранено: инсталляций {{installations}}, папок {{folders}}",
      "imported": "Импортировано: добавлено {{added}}, обновлено {{updated}}",
      "secretsNeeded_one": "Введите ключи API для {{count}} инсталляции",
      "secretsNeeded_few": "Введите ключи API для {{count}} инсталляций",
      "secretsNeeded_many": "Введите ключи API для {{count}} инсталляций",
      "secretsNeeded_other": "Введите ключи API для {{count}} инсталляции",
      "import": {
        "title": "Импорт инсталляций",
        "target": "Куда импортировать",
        "updateExisting": "Обновлять существующие инсталляции",
        "summary": "Добавится: {{added}} · Обновится: {{updated}} · Без изменений: {{unchanged}} · Пропущено: {{skipped}}",
        "groupAdded": "Новые",
        "groupUpdated": "Обновятся",
        "groupUnchanged": "Без изменений",
        "groupSkipped": "Пропущены",
        "secrets_one": "После импорта введите ключи API для {{count}} инсталляции",
        "secrets_few": "После импорта введите ключи API для {{count}} инсталляций",
        "secrets_many": "После импорта введите ключи API для {{count}} инсталляций",
        "secrets_other": "После импорта введите ключи API для {{count}} инсталляции",
        "apply": "Импортировать"
      }
    },
    "hints": {
      "import": {
        "target": "Куда попадут новые папки и инсталляции. Существующие останутся на своих местах.",
        "updateExisting": "Уже существующие инсталляции получат настройки из файла; ваши ключи API и их место в дереве сохранятся."
      }
    }
  }
}
````

- [ ] **Step 5: Проверки.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib`  
Expected: PASS — `test result: ok. 161 passed; 0 failed`

Run: `pnpm test`  
Expected: PASS — `Test Files 31 passed`, `Tests 182 passed`

Run: `pnpm exec tsc --noEmit`  
Expected: PASS — без ошибок

Run: `pnpm i18n:check`  
Expected: PASS — `✓ словари en, ru согласованы, вызовов из Rust: 95`

Run: `pnpm build`  
Expected: PASS — `✓ dist/assets: чанки совместимы с WebKit macOS 11+`

- [ ] **Step 6: Commit.**

```bash
git add \
  PORTED.md \
  src/App.test.tsx \
  src/App.tsx \
  src/app.css \
  src/components/Layout.test.tsx \
  src/components/Layout.tsx \
  src/components/Sidebar.test.tsx \
  src/components/Sidebar.tsx \
  src/components/TabContent.help.test.tsx \
  src/components/TabContent.tsx \
  src/components/installations/tree/FolderMenu.tsx \
  src/components/installations/tree/ImportDialog.test.tsx \
  src/components/installations/tree/ImportDialog.tsx \
  src/components/installations/tree/InstallationNode.tsx \
  src/components/installations/tree/InstallationTree.test.tsx \
  src/components/installations/tree/InstallationTree.tsx \
  src/guide/noLookbehind.test.ts \
  src/lib/tabActive.test.tsx \
  src/lib/tabActive.ts \
  src/lib/useAppMenu.test.ts \
  src/lib/useAppMenu.ts \
  src/lib/useHelpShortcut.ts \
  src/locales/en.json \
  src/locales/ru.json
git commit -s -m "feat(ui): installation tree with folders, drag and drop, export and import, layout, sidebar and the Help menu" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 15: Мок-сервер vLLM и SGLang для разработки и смоука без GPU, CI

**Files:**
- Port: `.github/workflows/ci.yml` (с правками)
- Create (по Kubernetes-версии): `.github/FUNDING.yml` ← `.github/FUNDING.yml`
- Create: `scripts/mock-server.mjs`, `scripts/mock-server.test.mjs`
- Modify: `PORTED.md`, `package.json`

**Interfaces:**
- Consumes: задача 6: снимки `src-tauri/src/servers/testdata`.
- Produces: `scripts/mock-server.mjs` (`pnpm mock-server --kind … --port … --api-key … --health …`), `KINDS`, `parseArgs`, `handler`; `.github/workflows/ci.yml`, `.github/FUNDING.yml`.

Мок отвечает снимками ответов тех же версий, что проверяют тесты Rust: vLLM 0.11, 0.8 и 0.7, SGLang 0.5 и 0.4, llama.cpp, Ollama. `/health` — без ключа, остальное — с ключом, если он задан. CI — как у Kubernetes-версии, но без стенда kind и крейтов MCP.

- [ ] **Step 1: Перенос из Kubernetes-версии.**

Файлы копируются с коммита `468e08a` репозитория `~/projects/glafira-openshift` (`$KUBERNETES_REPO`) и правятся патчами — против перенесённого файла. Строки о них в `PORTED.md` добавляет шаг реализации.

<!-- block: shell t15-port step=15.1 -->
````bash
scripts/port.sh .github/workflows/ci.yml .github/workflows/ci.yml
````

<!-- block: patch t15-port-github-workflows-ci-yml step=15.1 -->
````diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -33,8 +33,6 @@ jobs:
         with:
           workspaces: |
             src-tauri -> target
-            crates/glafira-mcp-core -> target
-            crates/glafira-mcp-kubernetes -> target
       - name: Системные зависимости Tauri
         run: |
           sudo apt-get update
@@ -46,36 +44,14 @@ jobs:
       - run: pnpm build
       - run: cargo fmt --manifest-path src-tauri/Cargo.toml --check
       - run: cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
-      - name: MCP-шлюз (крейты glafira-mcp-core и glafira-mcp-kubernetes)
-        run: |
-          for c in glafira-mcp-core glafira-mcp-kubernetes; do
-            cargo fmt --manifest-path crates/$c/Cargo.toml --check
-            cargo clippy --manifest-path crates/$c/Cargo.toml --all-targets -- -D warnings
-            cargo test --manifest-path crates/$c/Cargo.toml
-          done
       - name: Лицензии и уязвимости зависимостей (cargo deny, deny.toml)
         run: |
           cargo deny --version 2>/dev/null | grep -q ' 0\.20\.2$' || cargo install cargo-deny --locked --version 0.20.2 --force
           cargo deny --manifest-path src-tauri/Cargo.toml --config deny.toml check licenses advisories
-          cargo deny --manifest-path crates/glafira-mcp-core/Cargo.toml --config deny.toml check licenses advisories
-          cargo deny --manifest-path crates/glafira-mcp-kubernetes/Cargo.toml --config deny.toml check licenses advisories
-      - name: Кластер kind
-        uses: helm/kind-action@1544676c07bb3570fcb70166beaa07c4528fd4aa # v1.15.1
-        with:
-          version: v0.33.0
-          cluster_name: glafira
-      - name: Стенд — учётные записи и объекты проекта team-a
-        run: dev/kind/setup.sh
-      - name: Тесты Rust (unit + интеграционные)
+      # Серверы инференса — тестовые HTTP- и HTTPS-серверы на локальных портах и снимки ответов vLLM и SGLang.
+      - name: Тесты Rust
         timeout-minutes: 30
-        run: cargo test --manifest-path src-tauri/Cargo.toml --lib --tests
-        env:
-          GO_IT_KUBECONFIG: ${{ github.workspace }}/dev/kind/out/kubeconfig
-      - name: Стенд при падении — поды и события team-a
-        if: failure()
-        run: |
-          kubectl --context kind-glafira -n team-a get pods -o wide || true
-          kubectl --context kind-glafira -n team-a get events --sort-by=.lastTimestamp || true
+        run: cargo test --manifest-path src-tauri/Cargo.toml --lib
 
   platforms:
     if: github.event_name == 'workflow_dispatch' || startsWith(github.ref, 'refs/tags/')
@@ -100,17 +76,9 @@ jobs:
         with:
           workspaces: |
             src-tauri -> target
-            crates/glafira-mcp-core -> target
-            crates/glafira-mcp-kubernetes -> target
       - run: pnpm install --frozen-lockfile
       - run: pnpm test
+      - run: pnpm test:scripts
       - run: pnpm build
       - name: Unit-тесты Rust
         run: cargo test --manifest-path src-tauri/Cargo.toml --lib
-      # Шим MCP — режим GUI-бинаря: на Windows проверяется, что stdio GUI-процесса работает через pipe.
-      - name: MCP-шлюз (stdio GUI-процесса, named pipe / Unix socket)
-        shell: bash
-        run: |
-          cargo clippy --manifest-path crates/glafira-mcp-core/Cargo.toml --all-targets -- -D warnings
-          cargo test --manifest-path crates/glafira-mcp-core/Cargo.toml
-          cargo test --manifest-path crates/glafira-mcp-kubernetes/Cargo.toml
````

- [ ] **Step 2: Тесты.**

<!-- block: file t15-scripts-mock-server-test-mjs step=15.2 path=scripts/mock-server.test.mjs -->
````js
import assert from "node:assert/strict";
import { createServer } from "node:http";
import { test } from "node:test";
import { handler, KINDS, parseArgs } from "./mock-server.mjs";

async function serve(opts) {
  const server = createServer(handler({ health: 200, apiKey: null, ...opts }));
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const base = `http://127.0.0.1:${server.address().port}`;
  return { base, close: () => new Promise((resolve) => server.close(resolve)) };
}

test("параметры: значения по умолчанию, свои и ошибки", () => {
  assert.deepEqual(parseArgs([]), { kind: "vllm", port: 8000, host: "127.0.0.1", apiKey: null, health: 200 });
  assert.deepEqual(parseArgs(["--kind", "sglang", "--port", "30000", "--api-key", "sk", "--health", "503"]), {
    kind: "sglang",
    port: 30000,
    host: "127.0.0.1",
    apiKey: "sk",
    health: 503,
  });
  assert.throws(() => parseArgs(["--kind", "tgi"]), /неизвестный вид/);
  assert.throws(() => parseArgs(["--verbose", "1"]), /неизвестный параметр/);
});

test("каждый вид отдаёт свои снимки", async () => {
  for (const [kind, { routes }] of Object.entries(KINDS)) {
    const s = await serve({ kind });
    try {
      for (const path of Object.keys(routes)) {
        const r = await fetch(`${s.base}${path}`);
        assert.equal(r.status, 200, `${kind} ${path}`);
        assert.ok((await r.text()).length > 0, `${kind} ${path}`);
      }
      assert.equal((await fetch(`${s.base}/nope`)).status, 404);
    } finally {
      await s.close();
    }
  }
});

test("vLLM: версия и метрики; SGLang 0.4 — только старые пути", async () => {
  let s = await serve({ kind: "vllm" });
  try {
    assert.deepEqual(await (await fetch(`${s.base}/version`)).json(), { version: "0.11.0" });
    assert.match(await (await fetch(`${s.base}/metrics`)).text(), /^vllm:num_requests_running/m);
  } finally {
    await s.close();
  }
  s = await serve({ kind: "sglang-0.4" });
  try {
    assert.equal((await fetch(`${s.base}/model_info`)).status, 404);
    assert.equal((await fetch(`${s.base}/get_model_info`)).status, 200);
    assert.equal((await fetch(`${s.base}/metrics`)).status, 404);
  } finally {
    await s.close();
  }
});

test("ключ API: /health без ключа, остальное — только с ним; здоровье задаётся", async () => {
  const s = await serve({ kind: "vllm", apiKey: "sk-dev", health: 503 });
  try {
    assert.equal((await fetch(`${s.base}/health`)).status, 503);
    assert.equal((await fetch(`${s.base}/v1/models`)).status, 401);
    const ok = await fetch(`${s.base}/v1/models`, { headers: { authorization: "Bearer sk-dev" } });
    assert.equal(ok.status, 200);
  } finally {
    await s.close();
  }
  const o = await serve({ kind: "ollama" });
  try {
    assert.equal((await fetch(`${o.base}/health`)).status, 404, "у Ollama нет /health");
  } finally {
    await o.close();
  }
});
````

- [ ] **Step 3: Реализация.**

<!-- block: file t15-github-funding-yml step=15.3 path=.github/FUNDING.yml -->
````yaml
github: vvnovg
custom: ["https://github.com/vvnovg/glafira-inference-releases/blob/main/SUPPORT.md"]
````

<!-- block: file t15-scripts-mock-server-mjs step=15.3 path=scripts/mock-server.mjs mode=755 -->
````js
#!/usr/bin/env node
// Мок-сервер инференса для разработки и смоука без GPU: отвечает как vLLM, SGLang или OpenAI-совместимый
// сервер снимками ответов из src-tauri/src/servers/testdata (те же снимки проверяют тесты Rust).
// Только модули node:. Использование:
//   node scripts/mock-server.mjs --kind vllm|vllm-0.8|vllm-old|sglang|sglang-0.4|llamacpp|ollama
//        [--port 8000] [--host 127.0.0.1] [--api-key sk-dev] [--health 200|503]
import { readFileSync } from "node:fs";
import { createServer } from "node:http";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const DATA = join(fileURLToPath(new URL("..", import.meta.url)), "src-tauri/src/servers/testdata");

/** Пути сервера каждого вида → файл снимка (каталог testdata/<вид>). */
export const KINDS = {
  vllm: { dir: "vllm-0.11.0", routes: { "/version": "version.json", "/v1/models": "models.json", "/metrics": "metrics.txt" } },
  "vllm-0.8": { dir: "vllm-0.8.5", routes: { "/version": "version.json", "/v1/models": "models.json", "/metrics": "metrics.txt" } },
  "vllm-old": { dir: "vllm-0.7.3", routes: { "/version": "version.json", "/v1/models": "models.json", "/metrics": "metrics.txt" } },
  sglang: {
    dir: "sglang-0.5.4",
    routes: {
      "/model_info": "model_info.json",
      "/get_model_info": "model_info.json",
      "/server_info": "server_info.json",
      "/get_server_info": "server_info.json",
      "/v1/models": "models.json",
      "/metrics": "metrics.txt",
    },
  },
  // SGLang 0.4 без --enable-metrics: только устаревшие пути сведений, /metrics нет.
  "sglang-0.4": {
    dir: "sglang-0.4.6",
    routes: { "/get_model_info": "model_info.json", "/get_server_info": "server_info.json", "/v1/models": "models.json" },
  },
  llamacpp: { dir: "llamacpp", routes: { "/v1/models": "models.json" } },
  // Ollama: /health нет.
  ollama: { dir: "ollama", routes: { "/v1/models": "models.json" }, noHealth: true },
};

export function parseArgs(argv) {
  const opts = { kind: "vllm", port: 8000, host: "127.0.0.1", apiKey: null, health: 200 };
  for (let i = 0; i < argv.length; i++) {
    const [name, value] = [argv[i], argv[i + 1]];
    if (name === "--kind") opts.kind = value;
    else if (name === "--port") opts.port = Number(value);
    else if (name === "--host") opts.host = value;
    else if (name === "--api-key") opts.apiKey = value;
    else if (name === "--health") opts.health = Number(value);
    else throw new Error(`неизвестный параметр ${name}`);
    i++;
  }
  if (!KINDS[opts.kind]) throw new Error(`неизвестный вид ${opts.kind}: ${Object.keys(KINDS).join(", ")}`);
  return opts;
}

/** Обработчик запросов: /health без ключа (как у vLLM), остальное — с ключом, если он задан. */
export function handler(opts) {
  const kind = KINDS[opts.kind];
  return (req, res) => {
    const path = new URL(req.url ?? "/", "http://x").pathname;
    const reply = (status, type, body) => {
      res.writeHead(status, { "content-type": type });
      res.end(body);
    };
    if (path === "/health" && !kind.noHealth) return reply(opts.health, "text/plain", "");
    if (opts.apiKey && req.headers.authorization !== `Bearer ${opts.apiKey}`)
      return reply(401, "application/json", JSON.stringify({ error: "Unauthorized" }));
    const file = kind.routes[path];
    if (!file) return reply(404, "application/json", JSON.stringify({ detail: "Not Found" }));
    const body = readFileSync(join(DATA, kind.dir, file));
    reply(200, file.endsWith(".txt") ? "text/plain; version=0.0.4" : "application/json", body);
  };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const opts = parseArgs(process.argv.slice(2));
  createServer(handler(opts)).listen(opts.port, opts.host, () => {
    const key = opts.apiKey ? `, ключ API ${opts.apiKey}` : "";
    console.log(`✓ мок ${opts.kind}: http://${opts.host}:${opts.port}${key}`);
  });
}
````

<!-- block: patch t15-src step=15.3 -->
````diff
diff --git a/PORTED.md b/PORTED.md
index b80cabe..6ad9868 100644
--- a/PORTED.md
+++ b/PORTED.md
@@ -133,3 +133,5 @@
 | `src/lib/tabActive.ts` | `src/lib/tabActive.ts` | `468e08a` | как есть |
 | `src/lib/tabActive.test.tsx` | `src/lib/tabActive.test.tsx` | `468e08a` | как есть |
 | `src/guide/noLookbehind.test.ts` | `src/guide/noLookbehind.test.ts` | `468e08a` | как есть |
+| `.github/workflows/ci.yml` | `.github/workflows/ci.yml` | `468e08a` | адаптирован: без стенда kind и крейтов MCP |
+| `.github/FUNDING.yml` | `.github/FUNDING.yml` | `468e08a` | адаптирован: адрес страницы поддержки |
diff --git a/package.json b/package.json
index e32ba80..6a6b14a 100644
--- a/package.json
+++ b/package.json
@@ -13,7 +13,8 @@
     "test": "vitest run",
     "test:scripts": "node --test \"scripts/*.test.mjs\"",
     "i18n:check": "node scripts/i18n-check.mjs --rust-cyrillic src-tauri/src",
-    "guide:publish": "node scripts/guide-publish.mjs dist/guide"
+    "guide:publish": "node scripts/guide-publish.mjs dist/guide",
+    "mock-server": "node scripts/mock-server.mjs"
   },
   "dependencies": {
     "@floating-ui/react": "^0.27.20",
````

<!-- block: shell t15-install step=15.3 -->
````bash
pnpm install
````

- [ ] **Step 4: Проверки.**

Run: `pnpm test`  
Expected: PASS — `Test Files 31 passed`, `Tests 182 passed`

Run: `pnpm exec tsc --noEmit`  
Expected: PASS — без ошибок

Run: `pnpm i18n:check`  
Expected: PASS — `✓ словари en, ru согласованы, вызовов из Rust: 95`

Run: `pnpm test:scripts`  
Expected: PASS — `# pass 67`, `# fail 0`

Run: `pnpm build`  
Expected: PASS — `✓ dist/assets: чанки совместимы с WebKit macOS 11+`

- [ ] **Step 5: Commit.**

```bash
git add \
  .github/FUNDING.yml \
  .github/workflows/ci.yml \
  PORTED.md \
  package.json \
  scripts/mock-server.mjs \
  scripts/mock-server.test.mjs
git commit -s -m "chore: mock vLLM and SGLang server for development and smoke tests without a GPU, CI" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 16: Документация: README, правила локализации, глоссарий, смоук, уточнения спеки

**Files:**
- Port: `docs/i18n/README.md` (с правками)
- Create: `README.md`, `docs/i18n/glossary.md`, `docs/smoke/i1a.md`
- Modify: `PORTED.md`, `docs/superpowers/specs/2026-10-29-glafira-inference-design.md`

**Interfaces:**
- Consumes: задачи 1–15.
- Produces: `README.md`, `docs/i18n/README.md`, `docs/i18n/glossary.md`, `docs/smoke/i1a.md`; уточнения спеки (§5.2, §5.8, §5.9, §16).

README описывает запуск, тесты, мок-сервер, устройство инсталляций и серверов, хранение и политику. Смоук — 14 пунктов на мок-сервере и, если есть, на стенде с vLLM и SGLang. В спеку вносятся уточнения по итогам прототипа (список — в шапке плана).

- [ ] **Step 1: Перенос из Kubernetes-версии.**

Файлы копируются с коммита `468e08a` репозитория `~/projects/glafira-openshift` (`$KUBERNETES_REPO`) и правятся патчами — против перенесённого файла. Строки о них в `PORTED.md` добавляет шаг реализации.

<!-- block: shell t16-port step=16.1 -->
````bash
scripts/port.sh docs/i18n/README.md docs/i18n/README.md
````

<!-- block: patch t16-port-docs-i18n-readme-md step=16.1 -->
````diff
diff --git a/docs/i18n/README.md b/docs/i18n/README.md
--- a/docs/i18n/README.md
+++ b/docs/i18n/README.md
@@ -1,11 +1,11 @@
 # Локализация (i18n)
 
 Интерфейс и тексты Rust локализуются через словари `src/locales/<код>.json`: `en` — эталон ключей и параметров,
-`ru` — эталон смысла. До этапа O6 словарей два; остальные 10 языков Kafka-версии добавляются по правилам ниже.
+`ru` — эталон смысла. До этапа I7 словарей два; остальные 10 языков семейства добавляются по правилам ниже.
 Термины — [glossary.md](glossary.md).
 
 ## Соглашение о ключах
-- Вложенные объекты, ключи в camelCase: `<область>.<экран/компонент>.<смысл>`, например `connections.form.apiUrl`.
+- Вложенные объекты, ключи в camelCase: `<область>.<экран/компонент>.<смысл>`, например `installations.form.baseUrl`.
 - Общие тексты — в `common.*`; ошибки из Rust — в `errors.<модуль>.<смысл>`.
 - Параметры — `{{name}}`, одинаковые во всех словарях; фраза целиком — один ключ.
 - Видимые подписи обязательны: `aria-label`, `title`, `placeholder` тоже идут через словарь.
@@ -31,14 +31,14 @@
   - Вызовы в модуле `#[cfg(test)] mod tests` в конце файла проверка не смотрит.
 - В IPC уходит `{ key, params, text }`: `text` — английский запасной вариант (его же пишут в лог).
 - На фронтенде `tr(x)` (`src/i18n/text.ts`) переводит по ключу, а если ключа нет — показывает `text`.
-- `LocalizedText::raw(..)` — только технический текст (сообщение API-сервера), который нельзя перевести.
+- `LocalizedText::raw(..)` — только технический текст (сообщение сервера), который нельзя перевести.
 - Кириллица в строковых литералах Rust вне тестов запрещена: тексты — только через словари.
 
 ## Проверки
 `pnpm i18n:check` = `node scripts/i18n-check.mjs --rust-cyrillic src-tauri/src`: одинаковые ключи, параметры и
 теги во всех словарях; плюральные формы; строки, совпадающие с en, — только из `docs/i18n/same-as-en.json`;
 ключи `ltext!`/`app_err!` и их английский текст; ключи фронтенда; подсказки; главы руководства; кириллица в коде.
-С этапа O6 — флаг `--require-all-languages`.
+С этапа I7 — флаг `--require-all-languages`.
 
 ## Подсказки
 `<Hint k="hints.<экран>.<поле>" label={t("…подпись…")} />` (`src/components/common/Hint.tsx`) — рядом с подписью
````

- [ ] **Step 2: Реализация.**

<!-- block: file t16-readme-md step=16.2 path=README.md -->
````markdown
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
````

<!-- block: file t16-docs-i18n-glossary-md step=16.2 path=docs/i18n/glossary.md -->
````markdown
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
````

<!-- block: file t16-docs-smoke-i1a-md step=16.2 path=docs/smoke/i1a.md -->
````markdown
# Смоук I1a (каркас и подключения)

Сборка: `pnpm tauri dev`. Серверы: мок-сервер (`pnpm mock-server …`, README) — для всех пунктов; настоящий стенд с
vLLM и SGLang на GPU — для пунктов 11–12, если он есть. Каждый мок — в своём терминале, на своём порту.

1. Первый запуск. В левой панели «Инсталляции» и подсказка «Добавьте первую инсталляцию…»; в рабочей области
   «Выберите инсталляцию слева или добавьте новую.». Внизу панели — «Настройки», «Поддержать проект», «Справка»,
   «О программе».
2. vLLM. `pnpm mock-server --kind vllm --port 8000`. «+» → имя `dev-llm`, сервер `vllm-1`, адрес
   `http://127.0.0.1:8000`. «Проверить сервер»: шаги «Конфигурация», «DNS/TCP», «Здоровье», «Вход» («Сервер отвечает
   без ключа API»), «Тип и версия» — «vLLM 0.11.0», «Модели» — «qwen3-32b (32768)», все ✅. «Сохранить» → вкладка
   инсталляции: карточка `vllm-1` ✅, «vLLM 0.11.0», модель `qwen3-32b · 32768`, метрики «отдаёт»; «Подключено
   серверов: 1 из 1». В дереве у сервера — ●.
3. Ключ API. Перезапустить мок с `--api-key sk-dev`. «Подключиться заново» → карточка ❌ «Сервер отклонил запрос
   (401) — проверьте ключ API». В форме у сервера поле «Ключ API» с подсказкой «оставьте пустым, чтобы не менять
   сохранённый» и предупреждение о `http://` после ввода ключа; ввести `sk-dev`, «Проверить сервер»: «Вход» ✅
   «Ключ API принят». «Сохранить» → карточка ✅. В `installations.json` ключа нет; в Связке ключей macOS (Keychain
   Access) — запись сервиса `glafira-inference`. Без ключа «Проверить сервер» → ❌ «Вход» «Сервер требует ключ API…».
4. SGLang и второй сервер. `pnpm mock-server --kind sglang --port 30000`. В форме «Добавить сервер»: `sglang-1`,
   `http://127.0.0.1:30000`. Проверка — «SGLang 0.5.4», модель с длиной контекста 40960. Сохранить: на вкладке две
   карточки, обе ✅. «Убрать сервер» у второго и «Сохранить» — карточка исчезает.
5. Старые версии и предупреждения. `--kind vllm-old --port 8001`: проверка ✅ с ⚠ «vLLM 0.7.3 старше 0.8: Glafira
   работает с ним как с OpenAI-совместимым сервером»; на вкладке тип «OpenAI», «по признакам — vLLM».
   `--kind sglang-0.4 --port 30001`: «SGLang 0.4.6.post1» и ⚠ «SGLang не отдаёт метрики: запустите его с
   --enable-metrics». Тип «SGLang», выбранный вручную у сервера vLLM, → ⚠ «Сервер похож на vLLM, а в инсталляции выбран
   SGLang».
6. Другие серверы. `--kind ollama --port 11434`: шаг «Здоровье» ✅ «У сервера нет /health — шаг пропущен», тип
   «OpenAI-compatible», модели `llama3.1:8b`, `qwen3:32b`. `--kind llamacpp`: тип «OpenAI-compatible».
7. Ошибки. Мок с `--health 503` → ❌ «Здоровье» «Сервер нездоров (/health ответил 503)…». Закрытый порт → ❌ «DNS/TCP».
   Адрес `ftp://h` → ❌ «Конфигурация». Адрес `https://admin:secret@h:8000` → «Уберите имя пользователя и пароль…»,
   пароля в тексте нет. Сервер с адресом `https://127.0.0.1:8000` к моку по http → ❌ «TLS» или «Здоровье» (ошибка
   TLS-соединения).
8. Окружения. Новая инсталляция с окружением `prod` — флажок «Только чтение» поставлен; во вкладке и дереве — бейдж
   `PROD` и замок, полоса над вкладками красная.
9. Папки, экспорт и импорт. Меню «⋯» → «Новая папка», перетащить инсталляцию в папку; «Экспортировать папку…» →
   файл `*.glafira-inference-installations.json` без ключа `sk-dev` (`needsSecret.apiKeys` с id сервера). Изменить в
   файле адрес сервера, «Импортировать сюда…» с «Обновлять существующие инсталляции» → предупреждение «…сервер
   «vllm-1»: адрес меняется с … на …. Сохранённый ключ API уйдёт на новый адрес.»; после импорта вкладка подключается
   заново. Удаление инсталляции — после подтверждения в форме; вкладки закрываются, запись в Связке ключей удалена.
10. Политика. `docs/admin/policy.json` как политика (macOS — `defaults write` или профиль `docs/admin/policy.mobileconfig`):
    «Без проверки (небезопасно)» в форме недоступна, «Язык интерфейса» не меняется, в «О программе» — «Политика
    организации: задана».
11. Настоящий vLLM (стенд). Сервер vLLM 0.8+ на GPU: тип и версия совпадают с `vllm --version`, модели — с
    `/v1/models`, метрики «отдаёт». Сверить снимки ответов `src-tauri/src/servers/testdata/vllm-*` с настоящими
    (`/version`, `/v1/models`, начало `/metrics`): расхождения — в спеку этапа I1b.
12. Настоящий SGLang (стенд). SGLang 0.4+ с `--enable-metrics`: тип, версия и модели; сверить `/model_info`,
    `/server_info` (без ключа API в снимке!) и начало `/metrics` со снимками `sglang-*`.
13. Справка и язык. F1 на форме и вкладке инсталляции — глава «Инсталляции», в настройках — «Начало работы»; поиск по
    руководству находит «ключ API». «Настройки» → «Язык интерфейса» — English: интерфейс и справка на английском.
14. Меню macOS «Справка»: «Руководство Glafira», «Поддержать проект», «О программе».
````

<!-- block: patch t16-src step=16.2 -->
````diff
diff --git a/PORTED.md b/PORTED.md
index 6ad9868..96d7a2b 100644
--- a/PORTED.md
+++ b/PORTED.md
@@ -135,3 +135,4 @@
 | `src/guide/noLookbehind.test.ts` | `src/guide/noLookbehind.test.ts` | `468e08a` | как есть |
 | `.github/workflows/ci.yml` | `.github/workflows/ci.yml` | `468e08a` | адаптирован: без стенда kind и крейтов MCP |
 | `.github/FUNDING.yml` | `.github/FUNDING.yml` | `468e08a` | адаптирован: адрес страницы поддержки |
+| `docs/i18n/README.md` | `docs/i18n/README.md` | `468e08a` | адаптирован: этап I7, пример ключа |
diff --git a/docs/superpowers/specs/2026-10-29-glafira-inference-design.md b/docs/superpowers/specs/2026-10-29-glafira-inference-design.md
index d903518..b9f8225 100644
--- a/docs/superpowers/specs/2026-10-29-glafira-inference-design.md
+++ b/docs/superpowers/specs/2026-10-29-glafira-inference-design.md
@@ -237,10 +237,12 @@ crates/glafira-mcp-inference — инструменты, типы ответов
 
 ### 5.2 Серверы
 
-- имя, **базовый URL** (`https://vllm.internal:8000`; допускается путь, если сервер стоит за обратным прокси);
-  `user:pass@` в URL отклоняется (`errors.connections.credentialsInUrl`);
+- имя, **базовый URL**: `http://` или `https://` (внутренние серверы часто без TLS; по `http://` ключ API уходит
+  открытым текстом — форма предупреждает), допускается путь, если сервер стоит за обратным прокси
+  (`https://gw.corp/llm/qwen`); `user:pass@` в URL отклоняется (`errors.installations.credentialsInUrl`);
 - **тип:** `auto` (по умолчанию, §4.3), `vllm`, `sglang`, `openai`;
-- **ключ API** (keychain), отправляется как `Authorization: Bearer` во все запросы к серверу;
+- **ключ API** (keychain), отправляется как `Authorization: Bearer` во все запросы к серверу; перенаправления HTTP
+  Glafira не выполняет (3xx — ошибка `errors.server.redirect`): ключ не уходит на другой адрес;
 - путь метрик (по умолчанию `/metrics`), CA (системные корни, файл или PEM), `insecure-skip-tls-verify` — с постоянным
   предупреждением, запрещается политикой `allowInsecureTls = false`; прокси — системный, свой или без прокси;
 - серверы одной модели за балансировщиком добавляются по отдельности (реплики); адрес балансировщика — тоже сервер,
@@ -328,8 +330,10 @@ gpus? [{ hostRef, index }] }]`. Автоопределение (`detect_bindings
 ### 5.8 Проверка подключения
 
 По шагам для каждого компонента, как в семействе (✅/❌/— с причиной):
-- **сервер:** конфигурация → DNS/TCP → TLS → `/health` → вход (`/v1/models` с ключом; 401/403 — ❌ на шаге «Вход») →
-  тип и версия → модели;
+- **сервер:** конфигурация → DNS/TCP (к прокси профиля, если он задан; при системном прокси из окружения шаг
+  пропускается — соединение проверяют запросы) → TLS (только `https://`; любой HTTP-ответ доказывает рукопожатие) →
+  `/health` (404 — шаг пропущен: у Ollama его нет; 401/403 — проверит шаг «Вход») → вход (`/v1/models` с ключом;
+  401/403 — ❌ на шаге «Вход», без ключа — «Сервер требует ключ API») → тип и версия → модели (пустой список — ❌);
 - **история метрик:** DNS/TCP → TLS → вход → запрос `up` → найдены серии серверов;
 - **DCGM:** DNS/TCP → `/metrics` → есть `DCGM_FI_DEV_GPU_UTIL`;
 - **Kubernetes:** шаги Kubernetes-версии → права на чтение подов в указанных namespace → найден инференс;
@@ -338,9 +342,10 @@ gpus? [{ hostRef, index }] }]`. Автоопределение (`detect_bindings
 
 ### 5.9 Папки, экспорт и импорт
 
-Как в семействе: папки, перетаскивание, экспорт `*.glafira-inference-installations.json` без секретов, импорт с
-предпросмотром; при обновлении существующей инсталляции импорт предупреждает, если меняются адреса компонентов с
-сохранёнными секретами (секреты уйдут на новый адрес).
+Как в семействе: папки, перетаскивание, экспорт `*.glafira-inference-installations.json` без секретов (у инсталляции —
+`needsSecret.apiKeys`, id серверов с сохранённым ключом), импорт с предпросмотром; при обновлении существующей
+инсталляции импорт предупреждает, если у сервера с сохранённым ключом меняются адрес, прокси или проверка сертификата
+(ключ уйдёт на новый адрес). Ключи серверов, которых нет в файле, при обновлении удаляются из keychain.
 
 ---
 
@@ -860,13 +865,12 @@ keychain: сервис glafira-inference, ключи <installation-id>/<componen
 
 `AppError { code, key, params, message, details?, retryable }` — как в семействе. Коды семейства (`CONNECTION_FAILED`,
 `TLS_ERROR`, `AUTH_FAILED`, `SESSION_EXPIRED`, `FORBIDDEN`, `NOT_FOUND`, `CONFLICT`, `TIMEOUT`, `INVALID_INPUT`,
-`READ_ONLY`, `CONFIRMATION_REQUIRED`, `INTERNAL`) плюс новые. Отказ по политике — `FORBIDDEN` с ключом вида
-`errors.<область>.<действие>Forbidden` («запрещено вашей организацией»), как `errors.connections.execForbidden`
-в Kubernetes-версии. Новые коды:
+`READ_ONLY`, `CONFIRMATION_REQUIRED`, `INTERNAL`) плюс новые. Отказ по политике — `INVALID_INPUT` с ключом вида
+`errors.<область>.<действие>Forbidden` («запрещено вашей организацией»), как `errors.connections.insecureForbidden`
+в Kubernetes-версии (уточнено в I1a: семейство отказывает по политике кодом `INVALID_INPUT`). Новые коды:
 
 | `code` | Причина | Поведение UI |
 |---|---|---|
-| `SERVER_UNSUPPORTED` | версия vLLM или SGLang ниже нижней границы | работа как `openai-compatible`, предупреждение в обзоре |
 | `HISTORY_UNAVAILABLE` | у сервера нет источника истории или сопоставления серий | окна истории неактивны с подсказкой |
 | `SSH_HOST_KEY_UNKNOWN` | ключ хоста не подтверждён | диалог с отпечатком |
 | `SSH_HOST_KEY_CHANGED` | ключ хоста не совпал с подтверждённым | баннер с объяснением, «Изменить хост» |
@@ -877,6 +881,10 @@ keychain: сервис glafira-inference, ключи <installation-id>/<componen
 **Частичные результаты:** недоступные части экрана помечаются «—» с подсказкой причины, остальное показывается;
 аргумент без источника — `unknown`, а не пусто.
 
+Версия vLLM или SGLang ниже нижней границы — не ошибка: сервер работает как `openai-compatible`, а в его сведениях и
+в проверке подключения — предупреждение `messages.servers.unsupportedVersion` (уточнено в I1a вместо кода
+`SERVER_UNSUPPORTED`).
+
 ---
 
 ## 17. Локализация и документация
````

- [ ] **Step 3: Проверки.**

Run: `pnpm i18n:check`  
Expected: PASS — `✓ словари en, ru согласованы, вызовов из Rust: 95`

- [ ] **Step 4: Commit.**

```bash
git add \
  PORTED.md \
  README.md \
  docs/i18n/README.md \
  docs/i18n/glossary.md \
  docs/smoke/i1a.md \
  docs/superpowers/specs/2026-10-29-glafira-inference-design.md
git commit -s -m "docs: README, localization rules, glossary, I1a smoke checklist and spec clarifications from the prototype" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

## Покрытие спеки

| Требование | Задачи |
|---|---|
| §3 перенос из семейства, `PORTED.md` | 1–4, 10, 13, 14 |
| §5.1 профиль инсталляции: окружение, только чтение, серверы, таймауты; секреты в keychain; локальные поля | 4, 9, 11 |
| §5.2 серверы: адрес, тип, ключ API, путь метрик, CA, прокси | 4, 5, 11 |
| §4.3 определение типа, версии и моделей, нижние границы версий | 6 |
| §4.1 пул подключений, таймауты, блокирующие вызовы вне async-потоков | 7, 9 |
| §5.8 проверка подключения сервера | 8, 11 |
| §5.9 папки, экспорт и импорт | 4, 14 |
| §6.1–6.2 дерево и обзор инсталляции (в объёме I1a: серверы, тип, версия, модели, метрики) | 12, 14 |
| §12 интерфейс: раскладка, бейдж окружения, подсказки, справка, «О программе», первый запуск | 11–14 |
| §13 политика `allowInsecureTls`, `language`; секреты; CSP | 3, 5, 9 |
| §14 хранение: `installations.json`, `settings.json`, keychain | 2–4 |
| §16 ошибки сервера, частичные результаты | 5, 8, 12 |
| §17 словари en и ru, руководство (главы «Начало работы» и «Инсталляции») | 10–14 |
| §18 тесты без GPU: снимки ответов, локальные серверы, мок-сервер | 5–8, 15 |
| §19 CI | 15 |
| Смоук, README, уточнения спеки | 16 |
