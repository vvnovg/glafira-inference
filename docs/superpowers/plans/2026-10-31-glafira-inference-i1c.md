# I1c «История метрик» — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** История метрик сервера из Prometheus или VictoriaMetrics: источник в инсталляции (секрет — в keychain), его проверка, найденные цели и автоподбор селектора сервера, шаблоны PromQL по карте метрик адаптера, окна 1 ч / 24 ч / 7 д / 30 д на вкладке «Метрики», окно «Сейчас» с последних 15 минут истории, мок источника для отказов, глава справки «История метрик» и смоук с настоящим Prometheus.

**Architecture:** Подход A спеки I1c: PromQL собирает Rust. Чистые функции `history/` — селекторы и автоподбор (`targets.rs`), шаблоны над картой метрик адаптера (`promql.rs`), окна (`window.rs`), сборка точек `ServerMetrics` из матриц (`series.rs`); клиент API источника (`client.rs`) поверх механики `ServerClient`; проверка источника (`check.rs`) и окно сервера (`read.rs`, не больше 4 запросов одновременно); команды `check_history_source` и `read_server_history`, открытый источник — в пуле подключений до `evict`. Фронтенд — `useServerHistory` (окно по выбору, ответ живёт минуту), предзаполнение буфера «Сейчас» в `useServerMetrics`, окна в `ServerMetricsView`, раздел формы `HistoryFields`.

**Tech Stack:** Rust 2024 (tauri 2, reqwest, tokio, futures, serde, serde_json, url — новых крейтов нет), TypeScript (React 19, TanStack Query 5, i18next, uPlot 1.6.32 — новых зависимостей нет), Vitest + Testing Library, pnpm 11, Node 22 (мок-сервер без зависимостей), Prometheus 3.15.1 (`brew install prometheus`) — только для смоука и снятых ответов.

**Спецификация:** `docs/superpowers/specs/2026-10-31-glafira-inference-i1c-design.md` (спека этапа, разделы 1–9) поверх спеки I1b и общей `docs/superpowers/specs/2026-10-29-glafira-inference-design.md` (§5.3, §5.8, §6.3, §6.6, §17, §18). I1d «GPU» — отдельный план.

Решения, которые прототип уточнил (задача 12 вносит их в спеку I1c):
- без проверки сертификата — режим `CaConfig::Insecure`, как у серверов, а не отдельный флаг; политика запрещает его и при сохранении, и при импорте;
- секрет источника — одна запись keychain `<инсталляция>/history/secret` (токен или пароль по виду входа); при смене вида входа без нового секрета он удаляется — токен не становится паролем;
- проверка и автоподбор — одна команда `check_history_source` (шаги, найденные цели, подсказки), команды `discover_history_targets` нет;
- ошибки сети (DNS, TCP, TLS, таймаут соединения) — общие `errors.server.*`; ответы источника — `errors.history.*`;
- точки окна — от начала до конца включительно (61 / 61 / 961 / 841 / 961), интервал `rate` — не меньше минуты: «за минуту» у окна `now` — настоящая минута;
- у точек истории `count` перцентилей — 0 (число наблюдений не запрашивается), `preemptions1m` — скорость × 60, `engines` — 0;
- свой селектор разбирает только Rust (при сохранении и проверке) — второго разбора в TypeScript нет;
- мок источника: токен — `--api-key`, `--slow N`, `--huge`; после «перезапуска» мок отдаёт счётчики снимка, а не нули, поэтому `rate` показывает всплеск — смоук это оговаривает;
- в тестах Rust тестовый HTTP-сервер ищет ответ сначала по пути с параметрами, затем без них.

**Проверено до написания плана.** Весь код плана собран и прогнан в отдельной ветке репозитория (по коммиту на задачу, база `4944a12` — `main` @ `1a3cac7` со спекой I1c); повторное применение всех блоков плана в чистом дереве `4944a12` даёт то же дерево после каждой задачи побайтно, проверки после каждой задачи — зелёные:
- Rust — 283 unit-теста (`cargo test --lib`; было 243), clippy с `-D warnings`, `cargo fmt --check`; новых крейтов нет;
- фронтенд — 249 тестов в 35 файлах (было 233), `tsc --noEmit`, `pnpm build` с проверкой совместимости бандла с WebKit macOS 11, `pnpm i18n:check` (en и ru, 139 вызовов из Rust, 5 глав руководства на двух языках);
- скрипты — 110 тестов (было 107), в том числе мок `prometheus`;
- настоящий Prometheus 3.15.1, который опрашивал моки `vllm`, `vllm --engines 2`, `vllm --reset-after 20` и `sglang`: все 20 шаблонов на каждой из четырёх целей отработали; его ответы — `testdata/prometheus-3.15.1`, на них проходят тесты клиента, сборки точек и чтения окна;
- сквозная проверка Rust против живого Prometheus: проверка источника — 8 шагов ✅, автоподбор нашёл все цели (`localhost:8000` → `127.0.0.1:8000`), окна `now`, `1h`, `24h` читаются за 4–18 мс, у двух движков скорости вдвое выше.
Не проверено: окно приложения (`pnpm tauri dev`) и смоук `docs/smoke/i1c.md`; VictoriaMetrics; настоящие vLLM и SGLang на GPU.

## Global Constraints

- PromQL собирает только Rust по карте метрик адаптера; произвольных запросов к источнику интерфейс не делает (спека I1c §1, §4).
- Кандидаты имён серий — через `or` в порядке карты; метрика без значения во всём окне — «нет данных» (`missing`), не ноль; шаг без значения — разрыв (§4).
- Шаг окон 1 ч…30 д — не меньше 1 мин, не больше 1000 точек на ряд; `now` — 15 мин с шагом 15 с (§4).
- Не больше 4 запросов к источнику одновременно; сбой одного — `partial`, всех — ошибка (§4).
- История запрашивается только пока вкладка открыта; ответы не пишутся на диск и не кешируются в Glafira дольше минуты (общая спека §6.6, спека I1c §4–5).
- Токен и пароль источника — только в keychain; в профиле, экспорте, логах и ошибках их нет (§2).
- Сбой источника не мешает подключению серверов (§3).
- Все тексты интерфейса — в словарях en и ru; Rust — через `ltext!`/`app_err!` с ключом; глава руководства — на двух языках.
- Сообщения коммитов — по-английски, `git commit -s`, строка `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` перед `Signed-off-by`.

## Структура файлов

| Файл | Ответственность |
|---|---|
| `src-tauri/src/storage/profile.rs` | `HistorySource`, `HistoryAuth`, `history_selector`, проверка источника |
| `src-tauri/src/history/targets.rs` | селектор: разбор, сборка; автоподбор цели |
| `src-tauri/src/history/promql.rs` | шаблоны PromQL по карте метрик |
| `src-tauri/src/history/window.rs` | окна, шаги, интервал `rate` |
| `src-tauri/src/history/client.rs` | клиент API источника, разбор ответов, ошибки |
| `src-tauri/src/history/series.rs` | точки `ServerMetrics` из матриц |
| `src-tauri/src/history/check.rs`, `read.rs` | проверка источника, окно сервера |
| `src-tauri/src/commands/history.rs` | команды `check_history_source`, `read_server_history` |
| `src-tauri/src/history/testdata/prometheus-3.15.1/` | ответы настоящего Prometheus |
| `scripts/mock-server.mjs`, `scripts/prometheus/smoke.yml` | мок `prometheus`, конфиг Prometheus для смоука |
| `src/components/servers/useServerMetrics.ts` | `useServerHistory`, предзаполнение «Сейчас» |
| `src/components/servers/ServerMetricsView.tsx`, `MetricChart.tsx`, `chart.ts` | окна истории, граница |
| `src/components/installations/HistoryFields.tsx` | раздел формы «История метрик» |
| `docs/guide/{en,ru}/history.md`, `docs/smoke/i1c.md` | глава и смоук |

## Блоки кода

Код переносится как есть. Перед каждым блоком стоит HTML-комментарий `<!-- block: <вид> <id> step=<задача>.<шаг> [path=…] [part=…] [mode=…] [eol=none] [replace=1] -->`: `shell` — команды bash из корня репозитория; `file` — новый файл по `path` (у Rust-файлов с тестами две части: `part=tests` — модуль тестов, `part=impl` — реализация, итоговый файл = реализация, пустая строка, модуль тестов); `patch` — патч `git diff` (применять `git apply --recount`); `locales` — фрагмент словарей для `scripts/merge-locales.mjs` (`replace=1` — с `--replace`: значение меняется). Ограждения — четыре и больше обратных кавычек: внутри встречаются тройные. Блоки не перепечатывайте: их применяет скрипт ниже, по шагам, в порядке появления в плане. Патчи рассчитаны на дерево `4944a12` с применёнными предыдущими задачами плана.

В шаге 1 каждой задачи, кроме тестов, применяется подключение модулей (`mod.rs`, `lib.rs`): без него новые тесты Rust не компилировались бы вовсе, и падения в шаге 2 не было бы. У задачи 1 тесты — в изменённых модулях, поэтому шагов 1–2 у неё нет.

Скрипт извлекается из этого плана один раз, до задачи 1 (план лежит в `docs/superpowers/plans/` репозитория):

```bash
awk '/^<!-- block: helper i1c-blocks.py -->$/{f=1;next} f&&/^````/{n++; if(n==2) exit; next} f' \
  docs/superpowers/plans/2026-10-31-glafira-inference-i1c.md > /tmp/i1c-blocks.py
python3 /tmp/i1c-blocks.py list | head
```

<!-- block: helper i1c-blocks.py -->
````
#!/usr/bin/env python3
"""Блоки плана I1c: извлечение и применение по шагам (как у I1a; у locales — флаг replace=1).

Использование (из корня репозитория):
  python3 /tmp/i1c-blocks.py list                 — все блоки: id, вид, шаг, путь
  python3 /tmp/i1c-blocks.py get <id>             — содержимое блока в stdout
  python3 /tmp/i1c-blocks.py step <задача>.<шаг>  — применить блоки шага по порядку
План — $PLAN или docs/superpowers/plans/2026-10-31-glafira-inference-i1c.md.

Виды блоков: shell — команды bash из корня репозитория; file — файл по path (part=tests пишет модуль тестов,
part=impl ставит реализацию перед ним через пустую строку; mode — права; eol=none — файл без перевода строки в конце); patch — `git apply --recount`;
locales — строки словарей для `scripts/merge-locales.mjs` (replace=1 — заменить значения, `--replace`).
"""
import os
import re
import subprocess
import sys

PLAN = os.environ.get("PLAN", "docs/superpowers/plans/2026-10-31-glafira-inference-i1c.md")
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
    if b.get("eol") == "none":
        body = body[:-1]
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
        flags = ["--replace"] if b.get("replace") == "1" else []
        subprocess.run(["node", "scripts/merge-locales.mjs", *flags], input=b["body"].encode(), check=True)
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

Шаг задачи применяется так: `python3 /tmp/i1c-blocks.py step 3.1`, затем команды проверки из плана.

---

### Задача 1: Источник истории в профиле инсталляции

Профиль инсталляции получает необязательный источник истории `history` (адрес, вход, CA, прокси; спека I1c §2),
сервер — необязательный `historySelector`. Токен или пароль источника — одной записью keychain
`<инсталляция>/history/secret`: сохранение формы задаёт и удаляет его по тем же правилам, что ключи API, и не
переносит токен в пароль при смене вида входа; удаление инсталляции удаляет и его. Экспорт ставит
`needsSecret.history`, импорт предупреждает о смене адреса источника с сохранённым секретом и удаляет секрет, если
источника в файле нет. Источник без проверки сертификата запрещает политика `allowInsecureTls = false`.

**Files:**
- Modify: `src-tauri/src/commands/folders.rs`
- Modify: `src-tauri/src/commands/installations.rs`
- Modify: `src-tauri/src/secrets.rs`
- Modify: `src-tauri/src/servers/client.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/storage/exchange.rs`
- Modify: `src-tauri/src/storage/profile.rs`
- Modify: `src/locales/en.json`
- Modify: `src/locales/ru.json`

**Interfaces:**
- Produces: `storage::profile::{HistoryAuth, HistorySource, HISTORY_NAME, parse_history_url, validate_history}`,
  поле `InstallationProfile.history: Option<HistorySource>` и `ServerConfig.history_selector: Option<String>`
  (JSON `history`, `historySelector`, без поля, если `None`); `secrets::{HISTORY, HISTORY_SECRET, history_secret_key}`;
  `servers::client::ensure_ca_allowed(&CaConfig, &PolicyStatus)`; `SecretsInput.history_secret: Option<String>` (JSON
  `historySecret`); `NeedsSecret.history: bool`; `ImportPlan.removed_history: Vec<String>`;
  `profile::sample_history()` для тестов.

- [ ] **Шаг 1.3: Реализация.** Профиль, нормализация и проверка источника, секрет в keychain при сохранении и удалении, экспорт и импорт,
тексты ошибок и предупреждений. Тесты — в изменённых модулях (`profile.rs`, `commands/installations.rs`,
`exchange.rs`, `secrets.rs`), поэтому отдельного шага с падением у задачи нет.

<!-- block: patch t01-src-tauri-src-commands-folders-rs step=1.3 -->
````
diff --git a/src-tauri/src/commands/folders.rs b/src-tauri/src/commands/folders.rs
index b13275a..7bee482 100644
--- a/src-tauri/src/commands/folders.rs
+++ b/src-tauri/src/commands/folders.rs
@@ -165,10 +165,10 @@ fn import_from_file(
     if !apply {
         return Ok(plan_import(&snapshot, &parsed, &opts, &stored)?.preview);
     }
-    let (preview, removed) = st.installations.update(|data| {
+    let (preview, removed, removed_history) = st.installations.update(|data| {
         let plan = plan_import(data, &parsed, &opts, &stored)?;
         *data = plan.next;
-        Ok((plan.preview, plan.removed_servers))
+        Ok((plan.preview, plan.removed_servers, plan.removed_history))
     })?;
     // Секреты серверов, которых больше нет в профиле, не нужны: удаляются все, как при
     // сохранении и удалении инсталляции. Сбой keychain импорт не отменяет: профиль уже
@@ -180,6 +180,12 @@ fn import_from_file(
             );
         }
     }
+    for installation in removed_history {
+        let key = crate::secrets::history_secret_key(&installation);
+        if let Err(e) = st.secrets.delete(&key) {
+            tracing::warn!("history secret of installation {installation} was not deleted: {e}");
+        }
+    }
     // Обновлённые инсталляции подключатся заново с новым профилем — как при сохранении формы.
     for item in &preview.updated {
         st.evict_installation(&item.id);
````

<!-- block: patch t01-src-tauri-src-commands-installations-rs step=1.3 -->
````
diff --git a/src-tauri/src/commands/installations.rs b/src-tauri/src/commands/installations.rs
index fed29cc..96f12b4 100644
--- a/src-tauri/src/commands/installations.rs
+++ b/src-tauri/src/commands/installations.rs
@@ -5,27 +5,35 @@ use tauri::State;
 
 use super::blocking;
 use crate::error::AppResult;
-use crate::secrets::{API_KEY, secret_key};
-use crate::servers::client::{Timeouts, ensure_allowed};
+use crate::secrets::{API_KEY, history_secret_key, secret_key};
+use crate::servers::client::{Timeouts, ensure_allowed, ensure_ca_allowed};
 use crate::servers::connectivity::{TestReport, test_server};
 use crate::servers::pool::InstallationInfo;
 use crate::state::AppState;
-use crate::storage::profile::InstallationProfile;
+use crate::storage::profile::{HistoryAuth, InstallationProfile};
 
-/// Ключи API из формы: id сервера → значение. Сервера нет в списке — оставить сохранённый,
-/// `""` — удалить, иное — сохранить. `Debug` печатает вместо значений `***`.
+/// Секреты из формы. Ключи API: id сервера → значение; сервера нет в списке — оставить
+/// сохранённый, `""` — удалить, иное — сохранить. Секрет источника истории (токен или пароль) —
+/// по тем же правилам: `None` — оставить, `""` — удалить. `Debug` печатает вместо значений `***`.
 #[derive(Default, Clone, Deserialize)]
 #[serde(rename_all = "camelCase")]
 pub struct SecretsInput {
     #[serde(default)]
     pub api_keys: BTreeMap<String, String>,
+    #[serde(default)]
+    pub history_secret: Option<String>,
 }
 
 /// Ключи не печатаются: структура может попасть в лог или сообщение об ошибке.
 impl std::fmt::Debug for SecretsInput {
     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
         f.debug_map()
-            .entries(self.api_keys.keys().map(|k| (k, "***")))
+            .entries(self.api_keys.keys().map(|k| (k.as_str(), "***")))
+            .entries(
+                self.history_secret
+                    .as_ref()
+                    .map(|_| ("historySecret", "***")),
+            )
             .finish()
     }
 }
@@ -80,6 +88,34 @@ fn secret_ops(
         .collect()
 }
 
+/// Секрет источника истории — по данным формы. Без источника или без входа секрет не нужен.
+/// Сохранённый остаётся, только если вид входа не изменился: токен не становится паролем.
+fn history_op(
+    saved: &InstallationProfile,
+    previous: Option<&InstallationProfile>,
+    s: &SecretsInput,
+) -> SecretOp {
+    let mode = |p: &InstallationProfile| {
+        p.history
+            .as_ref()
+            .map(|h| std::mem::discriminant(&h.auth))
+            .filter(|_| {
+                p.history
+                    .as_ref()
+                    .is_some_and(|h| h.auth != HistoryAuth::None)
+            })
+    };
+    let Some(now) = mode(saved) else {
+        return SecretOp::Delete;
+    };
+    match s.history_secret.as_deref().map(str::trim) {
+        Some("") => SecretOp::Delete,
+        Some(v) => SecretOp::Set(v.to_string()),
+        None if previous.and_then(mode) == Some(now) => SecretOp::Keep,
+        None => SecretOp::Delete,
+    }
+}
+
 /// Серверы прежнего профиля, которых нет в сохранённом: их ключи больше не нужны.
 fn removed_servers(
     saved: &InstallationProfile,
@@ -94,20 +130,16 @@ fn removed_servers(
         .collect()
 }
 
-/// Применяет операции над ключами серверов профиля. Перед каждым `Set` и `Delete` читается
-/// прежнее значение: если любая операция откажет, уже изменённые ключи возвращаются к нему
-/// (`Some` — `set`, `None` — `delete`), и keychain остаётся таким, каким был. Сбой самого
-/// восстановления пишется в журнал (без значения), вызвавший получает исходную ошибку.
-fn apply_secret_ops(
-    st: &AppState,
-    installation_id: &str,
-    ops: Vec<(String, SecretOp)>,
-) -> AppResult<()> {
+/// Применяет операции над записями keychain (полный ключ записи → операция). Перед каждым
+/// `Set` и `Delete` читается прежнее значение: если любая операция откажет, уже изменённые
+/// записи возвращаются к нему (`Some` — `set`, `None` — `delete`), и keychain остаётся таким,
+/// каким был. Сбой самого восстановления пишется в журнал (без значения), вызвавший получает
+/// исходную ошибку.
+fn apply_secret_ops(st: &AppState, ops: Vec<(String, SecretOp)>) -> AppResult<()> {
     // Изменённые ключи вместе с прежним значением (`None` — ключа не было).
     let mut done: Vec<(String, Option<String>)> = Vec::new();
     // `Keep` ключ не трогает — keychain для него не читается.
-    for (server, op) in ops.into_iter().filter(|(_, op)| *op != SecretOp::Keep) {
-        let key = secret_key(installation_id, &server, API_KEY);
+    for (key, op) in ops.into_iter().filter(|(_, op)| *op != SecretOp::Keep) {
         let step = st.secrets.get(&key).and_then(|before| {
             match &op {
                 SecretOp::Set(v) => st.secrets.set(&key, v),
@@ -168,6 +200,9 @@ pub fn save_installation_blocking(
     for s in &profile.servers {
         ensure_allowed(s, &st.policy.status)?;
     }
+    if let Some(h) = &profile.history {
+        ensure_ca_allowed(&h.ca, &st.policy.status)?;
+    }
     if profile.id.is_empty() {
         profile.id = uuid::Uuid::new_v4().to_string();
     }
@@ -176,8 +211,15 @@ pub fn save_installation_blocking(
     let saved = st.installations.upsert(profile)?;
     // Ключи меняются по одному, прежние значения запоминаются: если keychain откажет посреди
     // изменений, `apply_secret_ops` вернёт уже изменённые ключи, а профиль откатится ниже.
-    let ops = secret_ops(&saved, previous.as_ref(), &secrets);
-    let result = apply_secret_ops(st, &saved.id, ops);
+    let mut ops: Vec<(String, SecretOp)> = secret_ops(&saved, previous.as_ref(), &secrets)
+        .into_iter()
+        .map(|(server, op)| (secret_key(&saved.id, &server, API_KEY), op))
+        .collect();
+    ops.push((
+        history_secret_key(&saved.id),
+        history_op(&saved, previous.as_ref(), &secrets),
+    ));
+    let result = apply_secret_ops(st, ops);
     if result.is_err() {
         let rollback = match previous {
             Some(p) => st.installations.upsert(p).map(|_| ()),
@@ -264,7 +306,9 @@ mod tests {
     use super::*;
     use crate::error::{AppError, ErrorCode};
     use crate::secrets::{MemoryStore, SecretStore};
-    use crate::storage::profile::{CaConfig, sample_profile, sample_server};
+    use crate::storage::profile::{
+        CaConfig, HistoryAuth, HistorySource, sample_history, sample_profile, sample_server,
+    };
 
     fn state(dir: &std::path::Path) -> AppState {
         AppState::new(dir, Arc::new(MemoryStore::default())).unwrap()
@@ -278,6 +322,7 @@ mod tests {
                     .iter()
                     .map(|(k, v)| (k.to_string(), v.to_string()))
                     .collect(),
+                history_secret: None,
             },
         }
     }
@@ -299,6 +344,7 @@ mod tests {
                 .iter()
                 .map(|(k, v)| (k.to_string(), v.to_string()))
                 .collect(),
+            history_secret: None,
         };
         // Убранный сервер (`gone`) в операции не входит: его ключ чистится после фиксации.
         // Новый сервер без ключа в форме (`s4`) получает `Delete`, а не `Keep`: осиротевший
@@ -320,6 +366,89 @@ mod tests {
         assert_eq!(format!("{s:?}"), r#"{"s2": "***", "s3": "***"}"#);
     }
 
+    fn with_history(auth: HistoryAuth) -> InstallationProfile {
+        let mut p = sample_profile("a");
+        p.history = Some(HistorySource {
+            auth,
+            ..sample_history()
+        });
+        p
+    }
+
+    #[test]
+    fn history_secret_ops() {
+        let bearer = with_history(HistoryAuth::Bearer);
+        let basic = with_history(HistoryAuth::Basic {
+            username: "u".into(),
+        });
+        let none = with_history(HistoryAuth::None);
+        let typed = |v: Option<&str>| SecretsInput {
+            history_secret: v.map(String::from),
+            ..SecretsInput::default()
+        };
+        assert_eq!(
+            history_op(&bearer, Some(&bearer), &typed(None)),
+            SecretOp::Keep
+        );
+        assert_eq!(
+            history_op(&bearer, None, &typed(Some(" t "))),
+            SecretOp::Set("t".into())
+        );
+        assert_eq!(
+            history_op(&bearer, Some(&bearer), &typed(Some(""))),
+            SecretOp::Delete
+        );
+        // Токен не становится паролем: смена вида входа без нового секрета его удаляет.
+        assert_eq!(
+            history_op(&basic, Some(&bearer), &typed(None)),
+            SecretOp::Delete
+        );
+        // Без источника или без входа секрет не нужен, даже если его прислали.
+        assert_eq!(
+            history_op(&none, Some(&bearer), &typed(Some("t"))),
+            SecretOp::Delete
+        );
+        assert_eq!(
+            history_op(&sample_profile("a"), Some(&bearer), &typed(None)),
+            SecretOp::Delete
+        );
+        let s = typed(Some("t"));
+        assert_eq!(format!("{s:?}"), r#"{"historySecret": "***"}"#);
+    }
+
+    #[test]
+    fn history_secret_is_saved_and_deleted_with_the_installation() {
+        let dir = tempfile::tempdir().unwrap();
+        let st = state(dir.path());
+        let mut i = input(with_history(HistoryAuth::Bearer), &[]);
+        i.secrets.history_secret = Some("prom-t0ken".into());
+        let saved = save_installation_blocking(&st, i).unwrap();
+        let read = || st.secrets.get(&history_secret_key(&saved.id)).unwrap();
+        assert_eq!(read().as_deref(), Some("prom-t0ken"));
+        let profile = std::fs::read_to_string(dir.path().join("installations.json")).unwrap();
+        assert!(!profile.contains("prom-t0ken"), "{profile}");
+        // Сохранение без секрета в форме его не трогает.
+        save_installation_blocking(&st, input(saved.clone(), &[])).unwrap();
+        assert_eq!(read().as_deref(), Some("prom-t0ken"));
+        st.delete_installation_full(&saved.id).unwrap();
+        assert_eq!(read(), None);
+    }
+
+    #[test]
+    fn policy_forbids_an_insecure_history_source() {
+        let dir = tempfile::tempdir().unwrap();
+        let mut policy = crate::policy::Policy::default();
+        policy.status.allow_insecure_tls = false;
+        let st = state(dir.path()).with_policy(policy);
+        let mut p = sample_profile("a");
+        p.history = Some(HistorySource {
+            ca: CaConfig::Insecure,
+            ..sample_history()
+        });
+        let e = save_installation_blocking(&st, input(p, &[])).unwrap_err();
+        assert_eq!(e.key, "errors.installations.insecureForbidden");
+    }
+
     #[test]
     fn secret_op_debug_hides_the_key() {
         let debug = format!("{:?}", SecretOp::Set("sk-secret".into()));
````

<!-- block: patch t01-src-tauri-src-secrets-rs step=1.3 -->
````
diff --git a/src-tauri/src/secrets.rs b/src-tauri/src/secrets.rs
index 27afd43..280f658 100644
--- a/src-tauri/src/secrets.rs
+++ b/src-tauri/src/secrets.rs
@@ -21,6 +21,17 @@ pub fn secret_key(installation_id: &str, component_id: &str, field: &str) -> Str
     format!("{installation_id}/{component_id}/{field}")
 }
 
+/// Компонент «источник истории» в ключе keychain (спека I1c §2). Поле `secret` у серверов не
+/// бывает, поэтому сервер с id `history` с источником не путается.
+pub const HISTORY: &str = "history";
+/// Секрет источника истории: токен (`bearer`) или пароль (`basic`) — по виду входа в профиле.
+pub const HISTORY_SECRET: &str = "secret";
+
+/// Ключ keychain секрета источника истории инсталляции.
+pub fn history_secret_key(installation_id: &str) -> String {
+    secret_key(installation_id, HISTORY, HISTORY_SECRET)
+}
+
 /// OS keychain: macOS Keychain, Windows Credential Manager, Secret Service.
 /// `service` — имя сервиса записей; прежние имена нужны только для миграции.
 pub struct KeyringStore {
@@ -143,6 +154,7 @@ mod tests {
     #[test]
     fn key_format() {
         assert_eq!(secret_key("i1", "s1", API_KEY), "i1/s1/apiKey");
+        assert_eq!(history_secret_key("i1"), "i1/history/secret");
     }
 
     #[test]
````

<!-- block: patch t01-src-tauri-src-servers-client-rs step=1.3 -->
````
diff --git a/src-tauri/src/servers/client.rs b/src-tauri/src/servers/client.rs
index d95cdb8..9f2c72f 100644
--- a/src-tauri/src/servers/client.rs
+++ b/src-tauri/src/servers/client.rs
@@ -73,7 +73,12 @@ impl HttpReply {
 
 /// Сервер без проверки сертификата нельзя открыть, если политика это запрещает (спец. §13).
 pub fn ensure_allowed(s: &ServerConfig, policy: &PolicyStatus) -> AppResult<()> {
-    if s.ca == CaConfig::Insecure && !policy.allow_insecure_tls {
+    ensure_ca_allowed(&s.ca, policy)
+}
+
+/// То же для любого адреса с настройкой CA: сервера или источника истории.
+pub fn ensure_ca_allowed(ca: &CaConfig, policy: &PolicyStatus) -> AppResult<()> {
+    if *ca == CaConfig::Insecure && !policy.allow_insecure_tls {
         return Err(app_err!(
             ErrorCode::InvalidInput,
             "errors.installations.insecureForbidden",
````

<!-- block: patch t01-src-tauri-src-state-rs step=1.3 -->
````
diff --git a/src-tauri/src/state.rs b/src-tauri/src/state.rs
index f38beee..7d37ec6 100644
--- a/src-tauri/src/state.rs
+++ b/src-tauri/src/state.rs
@@ -75,7 +75,8 @@ impl AppState {
         first_err.map_or(Ok(()), Err)
     }
 
-    /// Удаляет инсталляцию: профиль, ключи API её серверов и подключение. Подключение
+    /// Удаляет инсталляцию: профиль, ключи API её серверов, секрет источника истории и
+    /// подключение. Подключение
     /// забывается сразу после профиля, а ключи удаляются все, даже если keychain отказал
     /// на части из них. Профиля уже нет, повторить удаление нельзя, поэтому сбой keychain
     /// только пишется в журнал (без значений), а удаление считается состоявшимся: как при
@@ -92,6 +93,9 @@ impl AppState {
                 );
             }
         }
+        if let Err(e) = self.secrets.delete(&crate::secrets::history_secret_key(id)) {
+            tracing::warn!("history secret of the deleted installation {id} was not deleted: {e}");
+        }
         Ok(())
     }
 }
````

<!-- block: patch t01-src-tauri-src-storage-exchange-rs step=1.3 -->
````
diff --git a/src-tauri/src/storage/exchange.rs b/src-tauri/src/storage/exchange.rs
index eefdc00..a12b34e 100644
--- a/src-tauri/src/storage/exchange.rs
+++ b/src-tauri/src/storage/exchange.rs
@@ -7,12 +7,14 @@ use serde::{Deserialize, Serialize};
 
 use crate::error::{AppResult, ErrorCode};
 use crate::i18n::LocalizedText;
-use crate::secrets::{API_KEY, SecretStore, secret_key};
+use crate::secrets::{API_KEY, SecretStore, history_secret_key, secret_key};
 use crate::storage::installations::{
     InstallationFolder, InstallationProfile, InstallationStore, StoreData, check_tree,
     folder_not_found, unique_name, validate_profile,
 };
-use crate::storage::profile::{CaConfig, ServerConfig, normalize_profile, strip_private_keys};
+use crate::storage::profile::{
+    CaConfig, HistoryAuth, HistorySource, ServerConfig, normalize_profile, strip_private_keys,
+};
 use crate::storage::tree::TreeItemKind;
 
 pub const EXCHANGE_FORMAT: &str = "glafira-inference-installations";
@@ -25,6 +27,9 @@ pub struct NeedsSecret {
     /// Серверы (id), у которых был сохранён ключ API.
     #[serde(default)]
     pub api_keys: Vec<String>,
+    /// Был сохранён секрет источника истории (токен или пароль, спека I1c §2).
+    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
+    pub history: bool,
 }
 
 #[derive(Serialize)]
@@ -62,8 +67,21 @@ fn has_api_key(secrets: &dyn SecretStore, id: &str, server: &str) -> AppResult<b
         .is_some_and(|v| !v.is_empty()))
 }
 
-/// Серверы профиля, у которых сохранён ключ API: устаревшие записи keychain (сервер удалён)
-/// не требуют ввода у получателя.
+/// Сохранён ли непустой секрет источника истории инсталляции `id`.
+fn has_history_secret(secrets: &dyn SecretStore, id: &str) -> AppResult<bool> {
+    Ok(secrets
+        .get(&history_secret_key(id))?
+        .is_some_and(|v| !v.is_empty()))
+}
+
+/// Источник истории с входом по токену или паролю: только ему нужен секрет.
+fn history_with_secret(p: &InstallationProfile) -> Option<&HistorySource> {
+    p.history.as_ref().filter(|h| h.auth != HistoryAuth::None)
+}
+
+/// Серверы профиля, у которых сохранён ключ API, и сохранён ли секрет источника истории:
+/// устаревшие записи keychain (сервер удалён, вход в источник выключен) не требуют ввода у
+/// получателя.
 fn needs_secret(secrets: &dyn SecretStore, p: &InstallationProfile) -> AppResult<NeedsSecret> {
     let mut api_keys = Vec::new();
     for s in &p.servers {
@@ -71,23 +89,29 @@ fn needs_secret(secrets: &dyn SecretStore, p: &InstallationProfile) -> AppResult
             api_keys.push(s.id.clone());
         }
     }
-    Ok(NeedsSecret { api_keys })
+    let history = history_with_secret(p).is_some() && has_history_secret(secrets, &p.id)?;
+    Ok(NeedsSecret { api_keys, history })
 }
 
-/// Неизвестные поля профиля и серверов — локальные: в файл обмена и из него не переносятся.
+/// Неизвестные поля профиля, серверов и источника истории — локальные: в файл обмена и из него
+/// не переносятся.
 fn clear_extra(p: &mut InstallationProfile) {
     p.extra.clear();
     for s in &mut p.servers {
         s.extra.clear();
     }
+    if let Some(h) = &mut p.history {
+        h.extra.clear();
+    }
 }
 
 /// Приватный ключ из поля CA в файл обмена не попадает, сертификаты остаются (спец. §5.1:
 /// секреты — только в keychain). Профили читаются с диска без повторной проверки (файл могли
 /// править руками), поэтому ключ вырезается и здесь.
 fn strip_pem_keys_for_export(p: &mut InstallationProfile) {
-    for s in &mut p.servers {
-        if let CaConfig::Pem { pem } = &mut s.ca {
+    let history = p.history.as_mut().map(|h| &mut h.ca);
+    for ca in p.servers.iter_mut().map(|s| &mut s.ca).chain(history) {
+        if let CaConfig::Pem { pem } = ca {
             *pem = strip_private_keys(pem);
         }
     }
@@ -336,6 +360,9 @@ pub(crate) struct ImportPlan {
     /// Серверы обновлённых инсталляций, которых нет в файле: `(инсталляция, сервер)`. Их ключи
     /// API удаляются из keychain после записи.
     pub removed_servers: Vec<(String, String)>,
+    /// Обновлённые инсталляции, у которых источник истории убран или сменил вид входа: их
+    /// секрет источника удаляется из keychain после записи.
+    pub removed_history: Vec<String>,
 }
 
 /// Имена папок от корня до `folder` включительно.
@@ -411,6 +438,24 @@ fn destination_changes(local: &ServerConfig, new: &ServerConfig) -> bool {
     local.base_url != new.base_url || local.proxy != new.proxy || local.ca != new.ca
 }
 
+/// Меняется ли то, куда уходит секрет источника истории; `None`, если источника нет в одном из
+/// профилей.
+fn history_moves(local: &InstallationProfile, new: &InstallationProfile) -> bool {
+    match (history_with_secret(local), history_with_secret(new)) {
+        (Some(l), Some(n)) => l.url != n.url || l.proxy != n.proxy || l.ca != n.ca,
+        _ => false,
+    }
+}
+
+/// Остаётся ли сохранённый секрет источника истории после обновления: только при том же виде
+/// входа (токен не становится паролем), как при сохранении формы.
+fn history_secret_survives(local: &InstallationProfile, new: &InstallationProfile) -> bool {
+    match (history_with_secret(local), history_with_secret(new)) {
+        (Some(l), Some(n)) => std::mem::discriminant(&l.auth) == std::mem::discriminant(&n.auth),
+        _ => false,
+    }
+}
+
 /// Пары «сервер до импорта — сервер из файла» с тем же id, у которых меняется направление.
 fn moved_servers<'a>(
     local: &'a InstallationProfile,
@@ -432,6 +477,8 @@ fn moved_servers<'a>(
 pub(crate) struct StoredSecrets {
     /// (id инсталляции, id сервера) → есть ли сохранённый ключ API.
     by_server: HashMap<(String, String), bool>,
+    /// id инсталляции → есть ли сохранённый секрет источника истории.
+    by_history: HashMap<String, bool>,
 }
 
 impl StoredSecrets {
@@ -444,6 +491,7 @@ impl StoredSecrets {
         parsed: &ParsedExchange,
     ) -> Self {
         let mut by_server = HashMap::new();
+        let mut by_history = HashMap::new();
         for (incoming, _) in &parsed.installations {
             let Some(local) = data.installations.iter().find(|l| l.id == incoming.id) else {
                 continue;
@@ -452,8 +500,15 @@ impl StoredSecrets {
                 let stored = has_api_key(secrets, &incoming.id, &server.id).unwrap_or(true);
                 by_server.insert((incoming.id.clone(), server.id.clone()), stored);
             }
+            if history_moves(local, incoming) {
+                let stored = has_history_secret(secrets, &incoming.id).unwrap_or(true);
+                by_history.insert(incoming.id.clone(), stored);
+            }
+        }
+        Self {
+            by_server,
+            by_history,
         }
-        Self { by_server }
     }
 
     /// Сохранён ли ключ сервера `server` инсталляции `id`. Не прочитанное заранее (хранилище
@@ -464,6 +519,11 @@ impl StoredSecrets {
             .copied()
             .unwrap_or(true)
     }
+
+    /// Сохранён ли секрет источника истории инсталляции `id` (не прочитанное — сохранён).
+    fn has_history(&self, id: &str) -> bool {
+        self.by_history.get(id).copied().unwrap_or(true)
+    }
 }
 
 /// Предупреждения об обновлении инсталляции, у сервера которой меняется то, куда уходит
@@ -506,6 +566,16 @@ fn destination_warnings(
             ));
         }
     }
+    if history_moves(local, new)
+        && history_secret_survives(local, new)
+        && stored.has_history(&new.id)
+    {
+        out.push(ltext!(
+            "messages.import.historyChanged",
+            "“{{name}}”: the address, proxy or certificate verification of the metrics history source changes. The saved token or password will be sent with the new settings.",
+            name = new.name
+        ));
+    }
     out
 }
 
@@ -530,6 +600,7 @@ pub(crate) fn plan_import(
         ..Default::default()
     };
     let mut removed_servers = Vec::new();
+    let mut removed_history = Vec::new();
     // id папок файла совпадают с id в хранилище: существующие берутся как есть, новые создаются с тем же id.
     let file_folders: HashSet<&str> = parsed.folders.iter().map(|f| f.id.as_str()).collect();
     let local_folders: HashSet<&str> = data.folders.iter().map(|f| f.id.as_str()).collect();
@@ -576,7 +647,13 @@ pub(crate) fn plan_import(
         parsed.installations.iter().collect();
     installations.sort_by_key(|(p, _)| p.order);
     for (incoming, needs) in installations {
-        if !opts.allow_insecure_tls && incoming.servers.iter().any(|s| s.ca == CaConfig::Insecure) {
+        let insecure_history = incoming
+            .history
+            .as_ref()
+            .is_some_and(|h| h.ca == CaConfig::Insecure);
+        if !opts.allow_insecure_tls
+            && (insecure_history || incoming.servers.iter().any(|s| s.ca == CaConfig::Insecure))
+        {
             preview.skipped.push(Skipped {
                 name: incoming.name.clone(),
                 reason: ltext!(
@@ -601,6 +678,21 @@ pub(crate) fn plan_import(
                     server.extra = old.extra.clone();
                 }
             }
+            if let (Some(h), Some(old)) = (&mut profile.history, &slot.history) {
+                h.extra = old.extra.clone();
+            }
+            // Секрет источника истории, который больше не нужен, удаляется: источника нет в
+            // файле или у него другой вид входа.
+            if history_with_secret(slot).is_some() && !history_secret_survives(slot, &profile) {
+                removed_history.push(profile.id.clone());
+                if profile.history.is_none() {
+                    preview.warnings.push(ltext!(
+                        "messages.import.historyRemoved",
+                        "“{{name}}”: the metrics history source is not in the file and will be removed together with its saved token or password",
+                        name = profile.name
+                    ));
+                }
+            }
             // Сервера, которого нет в файле, не станет, а с ним и его ключа API. Человек должен
             // увидеть это в предпросмотре: «Обновлять существующие» включено по умолчанию.
             for old in slot
@@ -656,7 +748,7 @@ pub(crate) fn plan_import(
         profile.folder_id = folder;
         profile.name = name;
         clear_extra(&mut profile);
-        if !needs.api_keys.is_empty() {
+        if !needs.api_keys.is_empty() || needs.history {
             preview.needs_secrets += 1;
         }
         added.push((TreeItemKind::Installation, profile.id.clone()));
@@ -696,6 +788,7 @@ pub(crate) fn plan_import(
         next,
         preview,
         removed_servers,
+        removed_history,
     })
 }
 
@@ -1052,6 +1145,99 @@ mod tests {
         p.preview
     }
 
+    fn with_history(auth: HistoryAuth) -> InstallationProfile {
+        let mut p = sample_profile("a");
+        p.history = Some(HistorySource {
+            auth,
+            ..crate::storage::profile::sample_history()
+        });
+        p
+    }
+
+    fn history_json(auth: serde_json::Value) -> serde_json::Value {
+        serde_json::json!({ "url": "http://prometheus.dev.example.com:9090", "auth": auth })
+    }
+
+    #[test]
+    fn history_secret_is_flagged_but_never_exported() {
+        let (_d, s) = store();
+        s.upsert(with_history(HistoryAuth::Bearer)).unwrap();
+        let mut off = with_history(HistoryAuth::None);
+        off.id = "off".into();
+        s.upsert(off).unwrap();
+        let secrets = MemoryStore::default();
+        secrets.set(&history_secret_key("a"), "prom-t0ken").unwrap();
+        // Секрет без входа в источник — устаревшая запись: флага нет.
+        secrets.set(&history_secret_key("off"), "old").unwrap();
+        let out = build_export(&s, &secrets, None, "1", "now").unwrap();
+        let json = serde_json::to_string(&out).unwrap();
+        assert!(!json.contains("prom-t0ken"), "{json}");
+        let flags = |id: &str| {
+            out.installations
+                .iter()
+                .find(|c| c.profile.id == id)
+                .unwrap()
+                .needs_secret
+                .clone()
+        };
+        assert!(flags("a").history);
+        assert!(!flags("off").history);
+        assert!(!json.contains(r#""history":false"#), "{json}");
+    }
+
+    #[test]
+    fn history_source_change_with_a_saved_secret_warns() {
+        let secrets = MemoryStore::default();
+        secrets.set(&history_secret_key("a"), "t").unwrap();
+        let p = plan_update(with_history(HistoryAuth::Bearer), &secrets, |c| {
+            c["history"] = history_json(serde_json::json!({ "mode": "bearer" }));
+            c["history"]["url"] = "http://other:9090".into();
+        });
+        assert_eq!(warning_keys(&p), ["messages.import.historyChanged"]);
+        // Без сохранённого секрета предупреждать не о чем.
+        let p = plan_update(
+            with_history(HistoryAuth::Bearer),
+            &MemoryStore::default(),
+            |c| {
+                c["history"] = history_json(serde_json::json!({ "mode": "bearer" }));
+                c["history"]["url"] = "http://other:9090".into();
+            },
+        );
+        assert!(warning_keys(&p).is_empty());
+    }
+
+    #[test]
+    fn removed_history_source_drops_its_secret() {
+        let data = StoreData {
+            installations: vec![with_history(HistoryAuth::Bearer)],
+            ..Default::default()
+        };
+        let parsed = parse_exchange(&file(vec![], vec![inst_json("a", "inst-a", None)])).unwrap();
+        let p = plan_with(&data, &parsed, &opts(None), &MemoryStore::default()).unwrap();
+        assert_eq!(p.removed_history, ["a"]);
+        assert_eq!(warning_keys(&p.preview), ["messages.import.historyRemoved"]);
+        // Другой вид входа — секрет тоже не нужен, но источник остаётся: без предупреждения.
+        let mut basic = inst_json("a", "inst-a", None);
+        basic["history"] = history_json(serde_json::json!({ "mode": "basic", "username": "u" }));
+        let parsed = parse_exchange(&file(vec![], vec![basic])).unwrap();
+        let p = plan_with(&data, &parsed, &opts(None), &MemoryStore::default()).unwrap();
+        assert_eq!(p.removed_history, ["a"]);
+        assert!(warning_keys(&p.preview).is_empty());
+    }
+
+    #[test]
+    fn insecure_history_source_is_skipped_when_forbidden() {
+        let mut v = inst_json("i", "Insecure history", None);
+        v["history"] = history_json(serde_json::json!({ "mode": "none" }));
+        v["history"]["ca"] = serde_json::json!({ "mode": "insecure" });
+        let o = ImportOptions {
+            allow_insecure_tls: false,
+            ..opts(None)
+        };
+        let p = plan(&StoreData::default(), &file(vec![], vec![v]), &o);
+        assert_eq!(p.preview.skipped[0].name, "Insecure history");
+    }
+
     fn with_key() -> MemoryStore {
         let secrets = MemoryStore::default();
         secrets
````

<!-- block: patch t01-src-tauri-src-storage-profile-rs step=1.3 -->
````
diff --git a/src-tauri/src/storage/profile.rs b/src-tauri/src/storage/profile.rs
index 6ba982e..2db4629 100644
--- a/src-tauri/src/storage/profile.rs
+++ b/src-tauri/src/storage/profile.rs
@@ -1,6 +1,7 @@
-//! Профиль инсталляции инференса (спец. §5.1, §5.2): окружение, серверы и их настройки. Ключи API
-//! серверов в профиле не хранятся, они лежат в keychain (`secrets::API_KEY`). Проверки PEM, адресов
-//! и прокси — те же, что у профиля подключения Glafira for Kubernetes.
+//! Профиль инсталляции инференса (спец. §5.1, §5.2, §5.3): окружение, серверы, источник истории
+//! метрик и их настройки. Ключи API серверов и секрет источника истории в профиле не хранятся, они
+//! лежат в keychain (`secrets::API_KEY`, `secrets::HISTORY_SECRET`). Проверки PEM, адресов и
+//! прокси — те же, что у профиля подключения Glafira for Kubernetes.
 
 use serde::{Deserialize, Serialize};
 
@@ -92,6 +93,41 @@ pub struct ServerConfig {
     pub ca: CaConfig,
     #[serde(default = "default_proxy")]
     pub proxy: ProxyConfig,
+    /// Селектор серий сервера в источнике истории (`{job="vllm", instance="10.0.0.5:8000"}`, спека
+    /// I1c §3). `None` — история сервера недоступна.
+    #[serde(default, skip_serializing_if = "Option::is_none")]
+    pub history_selector: Option<String>,
+    /// Поля из более новой версии Glafira: сохраняются при записи как есть.
+    #[serde(flatten)]
+    pub extra: serde_json::Map<String, serde_json::Value>,
+}
+
+/// Вход в источник истории (спека I1c §2). Токен и пароль — только в keychain
+/// (`secrets::HISTORY_SECRET`), в профиле — только вид входа и имя пользователя.
+#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
+#[serde(tag = "mode", rename_all = "camelCase")]
+pub enum HistoryAuth {
+    #[default]
+    None,
+    Bearer,
+    Basic {
+        username: String,
+    },
+}
+
+/// Источник истории метрик — Prometheus или VictoriaMetrics (спека I1c §2); не больше одного на
+/// инсталляцию. Тип источника определяет проверка, в профиле его нет.
+#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
+#[serde(rename_all = "camelCase")]
+pub struct HistorySource {
+    /// Адрес API: `http(s)://host:port`, допускается путь (`https://gw/prometheus`).
+    pub url: String,
+    #[serde(default)]
+    pub auth: HistoryAuth,
+    #[serde(default = "default_ca")]
+    pub ca: CaConfig,
+    #[serde(default = "default_proxy")]
+    pub proxy: ProxyConfig,
     /// Поля из более новой версии Glafira: сохраняются при записи как есть.
     #[serde(flatten)]
     pub extra: serde_json::Map<String, serde_json::Value>,
@@ -107,6 +143,9 @@ pub struct InstallationProfile {
     pub read_only: bool,
     #[serde(default)]
     pub servers: Vec<ServerConfig>,
+    /// Источник истории метрик (спека I1c §2); `None` — окна истории недоступны.
+    #[serde(default, skip_serializing_if = "Option::is_none")]
+    pub history: Option<HistorySource>,
     #[serde(default = "default_request_timeout")]
     pub request_timeout_ms: u64,
     #[serde(default = "default_connect_timeout")]
@@ -129,12 +168,23 @@ impl InstallationProfile {
 }
 
 /// Приводит поля к каноническому виду до проверки: имена и адреса без пробелов по краям, адрес
-/// без `/` на конце, путь метрик — с `/` в начале.
+/// без `/` на конце, путь метрик — с `/` в начале, пустой селектор истории — его отсутствие.
 pub fn normalize_profile(p: &mut InstallationProfile) {
     p.name = p.name.trim().to_string();
+    if let Some(h) = &mut p.history {
+        h.url = h.url.trim().trim_end_matches('/').to_string();
+        if let HistoryAuth::Basic { username } = &mut h.auth {
+            *username = username.trim().to_string();
+        }
+    }
     for s in &mut p.servers {
         s.name = s.name.trim().to_string();
         s.base_url = s.base_url.trim().trim_end_matches('/').to_string();
+        s.history_selector = s
+            .history_selector
+            .take()
+            .map(|v| v.trim().to_string())
+            .filter(|v| !v.is_empty());
         let path = s.metrics_path.trim();
         s.metrics_path = if path.is_empty() {
             default_metrics_path()
@@ -315,7 +365,13 @@ pub fn validate_server(s: &ServerConfig) -> AppResult<()> {
             server = s.name
         )));
     }
-    if let CaConfig::Pem { pem } = &s.ca {
+    validate_ca(&s.ca, &s.name)?;
+    validate_proxy(&s.proxy)
+}
+
+/// CA из PEM: только сертификаты, без приватного ключа. `owner` — имя в тексте ошибки.
+fn validate_ca(ca: &CaConfig, owner: &str) -> AppResult<()> {
+    if let CaConfig::Pem { pem } = ca {
         if pem_has_private_key(pem) {
             return Err(invalid(ltext!(
                 "errors.installations.pemContainsKey",
@@ -326,11 +382,16 @@ pub fn validate_server(s: &ServerConfig) -> AppResult<()> {
             return Err(invalid(ltext!(
                 "errors.installations.caPemInvalid",
                 "The CA certificate of server “{{server}}” must be in PEM format (-----BEGIN CERTIFICATE-----)",
-                server = s.name
+                server = owner
             )));
         }
     }
-    if let ProxyConfig::Url { url } = &s.proxy {
+    Ok(())
+}
+
+/// Свой прокси — `http(s)://host:port` без имени и пароля.
+fn validate_proxy(proxy: &ProxyConfig) -> AppResult<()> {
+    if let ProxyConfig::Url { url } = proxy {
         let proxy = url::Url::parse(url.trim())
             .ok()
             .filter(|u| matches!(u.scheme(), "http" | "https") && u.host_str().is_some());
@@ -348,6 +409,47 @@ pub fn validate_server(s: &ServerConfig) -> AppResult<()> {
     Ok(())
 }
 
+/// Имя источника истории в текстах ошибок CA (`errors.installations.caPemInvalid`).
+pub const HISTORY_NAME: &str = "Prometheus / VictoriaMetrics";
+
+/// Разобранный адрес источника истории: `http(s)://`, хост, без имени и пароля, запроса и якоря.
+pub fn parse_history_url(h: &HistorySource) -> AppResult<url::Url> {
+    let bad = || {
+        invalid(ltext!(
+            "errors.history.badUrl",
+            "Invalid address “{{url}}” of the metrics history source: use http://host:9090 or https://host",
+            url = redact_url(&h.url)
+        ))
+    };
+    let url = url::Url::parse(h.url.trim()).map_err(|_| bad())?;
+    if !matches!(url.scheme(), "http" | "https")
+        || url.host_str().is_none_or(str::is_empty)
+        || url.query().is_some()
+        || url.fragment().is_some()
+    {
+        return Err(bad());
+    }
+    if has_userinfo(&url) {
+        return Err(credentials_in_url(&url));
+    }
+    Ok(url)
+}
+
+/// Проверка источника истории: и при сохранении профиля, и при проверке из формы.
+pub fn validate_history(h: &HistorySource) -> AppResult<()> {
+    parse_history_url(h)?;
+    if let HistoryAuth::Basic { username } = &h.auth
+        && (username.trim().is_empty() || username.contains(':'))
+    {
+        return Err(invalid(ltext!(
+            "errors.history.badUsername",
+            "Enter the user name of the metrics history source (without “:”)"
+        )));
+    }
+    validate_ca(&h.ca, HISTORY_NAME)?;
+    validate_proxy(&h.proxy)
+}
+
 pub fn validate_profile(p: &InstallationProfile) -> AppResult<()> {
     if p.id.trim().is_empty() {
         return Err(invalid(ltext!(
@@ -393,6 +495,9 @@ pub fn validate_profile(p: &InstallationProfile) -> AppResult<()> {
             )));
         }
     }
+    if let Some(h) = &p.history {
+        validate_history(h)?;
+    }
     if p.request_timeout_ms < 1_000 || p.connect_timeout_ms < 1_000 {
         return Err(invalid(ltext!(
             "errors.installations.timeoutTooSmall",
@@ -412,6 +517,18 @@ pub fn sample_server(id: &str) -> ServerConfig {
         metrics_path: "/metrics".into(),
         ca: CaConfig::System,
         proxy: ProxyConfig::System,
+        history_selector: None,
+        extra: Default::default(),
+    }
+}
+
+#[cfg(test)]
+pub fn sample_history() -> HistorySource {
+    HistorySource {
+        url: "http://prometheus.dev.example.com:9090".into(),
+        auth: HistoryAuth::None,
+        ca: CaConfig::System,
+        proxy: ProxyConfig::System,
         extra: Default::default(),
     }
 }
@@ -425,6 +542,7 @@ pub fn sample_profile(id: &str) -> InstallationProfile {
         environment: Environment::Dev,
         read_only: false,
         servers: vec![sample_server("s1")],
+        history: None,
         request_timeout_ms: 30_000,
         connect_timeout_ms: 10_000,
         folder_id: None,
@@ -731,4 +849,110 @@ mod tests {
         let v = serde_json::to_string(&sample_profile("a")).unwrap();
         assert!(!v.contains("apiKey") && !v.contains("token"), "{v}");
     }
+
+    #[test]
+    fn history_json_shape_and_defaults() {
+        let mut p = sample_profile("a");
+        p.history = Some(HistorySource {
+            auth: HistoryAuth::Basic {
+                username: "reader".into(),
+            },
+            ..sample_history()
+        });
+        p.servers[0].history_selector = Some(r#"{job="vllm", instance="10.0.0.5:8000"}"#.into());
+        let v = serde_json::to_value(&p).unwrap();
+        assert_eq!(
+            v["history"],
+            serde_json::json!({
+                "url": "http://prometheus.dev.example.com:9090",
+                "auth": { "mode": "basic", "username": "reader" },
+                "ca": { "mode": "system" },
+                "proxy": { "mode": "system" }
+            })
+        );
+        assert_eq!(
+            v["servers"][0]["historySelector"],
+            r#"{job="vllm", instance="10.0.0.5:8000"}"#
+        );
+        let minimal: HistorySource =
+            serde_json::from_value(serde_json::json!({ "url": "http://h:9090" })).unwrap();
+        assert_eq!(minimal.auth, HistoryAuth::None);
+        assert_eq!(
+            (&minimal.ca, &minimal.proxy),
+            (&CaConfig::System, &ProxyConfig::System)
+        );
+        let s = serde_json::to_string(&p).unwrap();
+        assert!(!s.contains("password") && !s.contains("token"), "{s}");
+    }
+
+    #[test]
+    fn history_is_normalized_and_validated() {
+        let mut p = sample_profile("a");
+        p.history = Some(HistorySource {
+            url: " http://prom:9090/ ".into(),
+            auth: HistoryAuth::Basic {
+                username: " reader ".into(),
+            },
+            ..sample_history()
+        });
+        p.servers[0].history_selector = Some("   ".into());
+        normalize_profile(&mut p);
+        let h = p.history.as_ref().unwrap();
+        assert_eq!(h.url, "http://prom:9090");
+        assert_eq!(
+            h.auth,
+            HistoryAuth::Basic {
+                username: "reader".into()
+            }
+        );
+        assert_eq!(p.servers[0].history_selector, None);
+        assert!(validate_profile(&p).is_ok());
+
+        let with = |h: HistorySource| {
+            let mut p = sample_profile("a");
+            p.history = Some(h);
+            p
+        };
+        for url in [
+            "prom:9090",
+            "ftp://prom",
+            "http://prom:9090/?x=1",
+            "http://",
+        ] {
+            let h = HistorySource {
+                url: url.into(),
+                ..sample_history()
+            };
+            assert_invalid(&with(h), "errors.history.badUrl");
+        }
+        let h = HistorySource {
+            url: "http://u:p@prom:9090".into(),
+            ..sample_history()
+        };
+        let e = assert_invalid(&with(h), "errors.installations.credentialsInUrl");
+        assert!(!e.message.contains("u:p"), "{}", e.message);
+        for username in ["", " ", "a:b"] {
+            let h = HistorySource {
+                auth: HistoryAuth::Basic {
+                    username: username.into(),
+                },
+                ..sample_history()
+            };
+            assert_invalid(&with(h), "errors.history.badUsername");
+        }
+        let h = HistorySource {
+            ca: CaConfig::Pem {
+                pem: cert_with_key(),
+            },
+            ..sample_history()
+        };
+        assert_invalid(&with(h), "errors.installations.pemContainsKey");
+        let h = HistorySource {
+            proxy: ProxyConfig::Url {
+                url: "socks://x".into(),
+            },
+            ..sample_history()
+        };
+        assert_invalid(&with(h), "errors.installations.badProxyUrl");
+    }
 }
````

<!-- block: locales t01-locales-t01 step=1.3 -->
````
{
  "en": {
    "errors": { "history": {
      "badUrl": "Invalid address “{{url}}” of the metrics history source: use http://host:9090 or https://host",
      "badUsername": "Enter the user name of the metrics history source (without “:”)"
    } },
    "messages": { "import": {
      "historyChanged": "“{{name}}”: the address, proxy or certificate verification of the metrics history source changes. The saved token or password will be sent with the new settings.",
      "historyRemoved": "“{{name}}”: the metrics history source is not in the file and will be removed together with its saved token or password"
    } }
  },
  "ru": {
    "errors": { "history": {
      "badUrl": "Неверный адрес «{{url}}» источника истории метрик: используйте http://хост:9090 или https://хост",
      "badUsername": "Укажите имя пользователя источника истории метрик (без «:»)"
    } },
    "messages": { "import": {
      "historyChanged": "«{{name}}»: у источника истории метрик меняется адрес, прокси или проверка сертификата. Сохранённый токен или пароль уйдёт с новыми настройками.",
      "historyRemoved": "«{{name}}»: источника истории метрик нет в файле — он будет удалён вместе с сохранённым токеном или паролем"
    } }
  }
}
````

- [ ] **Шаг 1.4: Проверки.**

Run: `cd src-tauri && cargo test --lib -q`
Expected: `test result: ok. 252 passed`
Run: `cd src-tauri && cargo clippy --lib --tests -q -- -D warnings && cargo fmt --check`
Expected: без вывода
Run: `pnpm i18n:check`
Expected: `✓ словари en, ru согласованы, вызовов из Rust: 113`

- [ ] **Шаг 1.5: Коммит.**

<!-- block: shell t01-commit step=1.5 -->
````
git add src-tauri/src/commands/folders.rs \
  src-tauri/src/commands/installations.rs \
  src-tauri/src/secrets.rs \
  src-tauri/src/servers/client.rs \
  src-tauri/src/state.rs \
  src-tauri/src/storage/exchange.rs \
  src-tauri/src/storage/profile.rs \
  src/locales/en.json \
  src/locales/ru.json
git commit -s -q -F - <<'EOF'
feat(history): metrics history source in the installation profile

An installation gets an optional Prometheus/VictoriaMetrics source (address,
auth, CA, proxy) and each server an optional series selector. The token or
password lives only in the keychain; export flags it, import warns when the
source moves and drops the secret when the source or its auth mode goes.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
````

---

### Задача 2: Селекторы серий и автоподбор целей

Модуль `history` и в нём `targets.rs` (спека I1c §3): строгий разбор селектора (`{` условия `}`, операции `=`,
`!=`, `=~`, `!~`, значения в двойных кавычках, без `__name__`, ошибка — с номером символа), сборка условий для
шаблонов и автоподбор цели по `host:port` адреса сервера (`localhost` = `127.0.0.1` = `::1`, порт по умолчанию — из
схемы). Профиль проверяет селектор сервера тем же разбором.

**Files:**
- Create: `src-tauri/src/history/mod.rs`
- Create: `src-tauri/src/history/targets.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/storage/profile.rs`
- Modify: `src/locales/en.json`
- Modify: `src/locales/ru.json`

**Interfaces:**
- Produces: `history::targets::{Target { job, instance, kind }, TargetKind, Matcher, MatchOp, parse_selector,
  render, escape, instance_matches, matching_targets}`; `Target::selector()` → `{job="…", instance="…"}`;
  `render(&[Matcher])` — условия без скобок через запятую.

- [ ] **Шаг 2.1: Тесты.** Тесты `history/targets.rs`; модуль `history` подключается в `lib.rs`.

<!-- block: file t02-src-tauri-src-history-mod-rs step=2.1 path=src-tauri/src/history/mod.rs -->
````
//! История метрик из Prometheus или VictoriaMetrics (спека I1c): цели и селекторы серий.

pub mod targets;
````

<!-- block: file t02-src-tauri-src-history-targets-rs-tests step=2.1 path=src-tauri/src/history/targets.rs part=tests -->
````
#[cfg(test)]
mod tests {
    use super::*;

    fn m(label: &str, op: MatchOp, value: &str) -> Matcher {
        Matcher {
            label: label.into(),
            op,
            value: value.into(),
        }
    }

    #[test]
    fn selector_is_parsed_strictly() {
        assert_eq!(
            parse_selector(r#" { job="vllm", instance = "10.0.0.5:8000" , } "#).unwrap(),
            [
                m("job", MatchOp::Eq, "vllm"),
                m("instance", MatchOp::Eq, "10.0.0.5:8000")
            ]
        );
        assert_eq!(
            parse_selector(r#"{pod=~"vllm-.*",namespace!="dev",zone!~"b|c"}"#).unwrap(),
            [
                m("pod", MatchOp::Re, "vllm-.*"),
                m("namespace", MatchOp::Ne, "dev"),
                m("zone", MatchOp::Nre, "b|c"),
            ]
        );
        assert_eq!(
            parse_selector(r#"{a="q\"x\\y\nz"}"#).unwrap(),
            [m("a", MatchOp::Eq, "q\"x\\y\nz")]
        );
        for (bad, pos) in [
            ("", 1),
            ("job=\"x\"", 1),
            ("{}", 2),
            ("{job}", 5),
            ("{job=x}", 6),
            ("{job='x'}", 6),
            ("{job=\"x\"", 9),
            ("{job=\"x\"} extra", 11),
            ("{1job=\"x\"}", 2),
            ("{job=\"x\\t\"}", 8),
            ("{job=\"x\" instance=\"y\"}", 10),
        ] {
            let e = parse_selector(bad).unwrap_err();
            assert_eq!(e.key, "errors.history.selector", "{bad}");
            assert_eq!(e.params["pos"], pos.to_string(), "{bad}");
        }
        let e = parse_selector(r#"{__name__="up"}"#).unwrap_err();
        assert_eq!(e.key, "errors.history.selectorName");
    }

    #[test]
    fn selector_round_trips_with_escaping() {
        let parsed = parse_selector(r#"{job="v\"llm", instance=~"a\\b"}"#).unwrap();
        assert_eq!(render(&parsed), r#"job="v\"llm",instance=~"a\\b""#);
        let t = Target {
            job: "vllm".into(),
            instance: "10.0.0.5:8000".into(),
            kind: TargetKind::Vllm,
        };
        assert_eq!(t.selector(), r#"{job="vllm", instance="10.0.0.5:8000"}"#);
        assert_eq!(parse_selector(&t.selector()).unwrap().len(), 2);
    }

    fn target(job: &str, instance: &str) -> Target {
        Target {
            job: job.into(),
            instance: instance.into(),
            kind: TargetKind::Vllm,
        }
    }

    fn url(s: &str) -> url::Url {
        url::Url::parse(s).unwrap()
    }

    #[test]
    fn instance_matches_host_and_port() {
        let base = url("http://localhost:8000/v1");
        for ok in [
            "127.0.0.1:8000",
            "localhost:8000",
            "[::1]:8000",
            "LOCALHOST:8000",
        ] {
            assert!(instance_matches(ok, &base), "{ok}");
        }
        for no in ["127.0.0.1:8001", "10.0.0.5:8000", "localhost", "x:y"] {
            assert!(!instance_matches(no, &base), "{no}");
        }
        // Порт по умолчанию — из схемы адреса сервера.
        assert!(instance_matches("vllm.ai", &url("https://vllm.ai")));
        assert!(instance_matches("vllm.ai:443", &url("https://vllm.ai")));
        assert!(!instance_matches("vllm.ai:80", &url("https://vllm.ai")));
        assert!(instance_matches(
            "[fd00::5]:8000",
            &url("http://[fd00::5]:8000")
        ));
    }

    #[test]
    fn matching_targets_are_ordered_by_job() {
        let targets = [
            target("vllm", "10.0.0.5:8000"),
            target("inference", "10.0.0.5:8000"),
            target("vllm", "10.0.0.6:8000"),
        ];
        let found = matching_targets(&url("http://10.0.0.5:8000"), &targets);
        assert_eq!(
            found.iter().map(|t| t.job.as_str()).collect::<Vec<_>>(),
            ["inference", "vllm"]
        );
        assert!(matching_targets(&url("http://svc:8000"), &targets).is_empty());
    }
}
````

<!-- block: patch t02-src-tauri-src-lib-rs step=2.1 -->
````
diff --git a/src-tauri/src/lib.rs b/src-tauri/src/lib.rs
index 695fe1a..d22004a 100644
--- a/src-tauri/src/lib.rs
+++ b/src-tauri/src/lib.rs
@@ -3,6 +3,7 @@ pub mod i18n;
 pub mod commands;
 mod dock_name;
 pub mod error;
+pub mod history;
 pub mod logging;
 pub mod metrics;
 pub mod policy;
````

- [ ] **Шаг 2.2: Увидеть падение.**

Run: `cd src-tauri && cargo test --lib -q history`
Expected: FAIL — не компилируется: тестам не хватает типов и функций, которые появятся в шаге 3

- [ ] **Шаг 2.3: Реализация.** Разбор, сборка, автоподбор, проверка селектора в профиле и тексты ошибок.

<!-- block: file t02-src-tauri-src-history-targets-rs step=2.3 path=src-tauri/src/history/targets.rs part=impl -->
````
//! Цели инференса в источнике истории и сопоставление их с серверами (спека I1c §3): разбор и
//! сборка селектора серий, автоподбор цели по `host:port` адреса сервера.

use serde::Serialize;

use crate::error::{AppError, AppResult, ErrorCode};

/// Тип цели — по префиксу её серий (`vllm:`, `sglang:`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetKind {
    Vllm,
    Sglang,
}

/// Цель инференса, найденная в источнике: пара `job`, `instance`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub job: String,
    pub instance: String,
    pub kind: TargetKind,
}

impl Target {
    /// Селектор серий этой цели: `{job="…", instance="…"}`.
    pub fn selector(&self) -> String {
        format!(
            "{{job=\"{}\", instance=\"{}\"}}",
            escape(&self.job),
            escape(&self.instance)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchOp {
    Eq,
    Ne,
    Re,
    Nre,
}

impl MatchOp {
    fn as_str(self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Ne => "!=",
            Self::Re => "=~",
            Self::Nre => "!~",
        }
    }
}

/// Условие на метку: `job="vllm"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Matcher {
    pub label: String,
    pub op: MatchOp,
    pub value: String,
}

/// Значение метки в двойных кавычках PromQL: `\` и `"` экранируются, перевод строки — `\n`.
pub fn escape(v: &str) -> String {
    v.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Условия селектора без фигурных скобок, для подстановки в шаблон: `job="vllm",instance="h:8000"`.
pub fn render(matchers: &[Matcher]) -> String {
    matchers
        .iter()
        .map(|m| format!("{}{}\"{}\"", m.label, m.op.as_str(), escape(&m.value)))
        .collect::<Vec<_>>()
        .join(",")
}

fn bad_selector(pos: usize) -> AppError {
    app_err!(
        ErrorCode::InvalidInput,
        "errors.history.selector",
        "The series selector is invalid at character {{pos}}: use {job=\"vllm\", instance=\"host:port\"}",
        pos = pos + 1
    )
}

/// Строгий разбор селектора (спека I1c §3): `{` условия через запятую `}`, операции `=`, `!=`,
/// `=~`, `!~`, значения в двойных кавычках (`\"`, `\\`, `\n`), хотя бы одно условие, без
/// `__name__` — имя серии подставляет шаблон. Ошибка называет позицию (с 1) в символах.
pub fn parse_selector(text: &str) -> AppResult<Vec<Matcher>> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let skip_ws = |i: &mut usize| {
        while *i < chars.len() && chars[*i].is_whitespace() {
            *i += 1;
        }
    };
    skip_ws(&mut i);
    if chars.get(i) != Some(&'{') {
        return Err(bad_selector(i));
    }
    i += 1;
    let mut out = Vec::new();
    loop {
        skip_ws(&mut i);
        if chars.get(i) == Some(&'}') && !out.is_empty() {
            i += 1;
            break;
        }
        let start = i;
        while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
            i += 1;
        }
        let label: String = chars[start..i].iter().collect();
        if label.is_empty() || label.starts_with(|c: char| c.is_ascii_digit()) {
            return Err(bad_selector(start));
        }
        if label == "__name__" {
            return Err(app_err!(
                ErrorCode::InvalidInput,
                "errors.history.selectorName",
                "The series selector must not set __name__: Glafira adds the series names itself"
            ));
        }
        skip_ws(&mut i);
        let op = match (chars.get(i), chars.get(i + 1)) {
            (Some('='), Some('~')) => MatchOp::Re,
            (Some('!'), Some('=')) => MatchOp::Ne,
            (Some('!'), Some('~')) => MatchOp::Nre,
            (Some('='), _) => MatchOp::Eq,
            _ => return Err(bad_selector(i)),
        };
        i += op.as_str().len();
        skip_ws(&mut i);
        if chars.get(i) != Some(&'"') {
            return Err(bad_selector(i));
        }
        i += 1;
        let mut value = String::new();
        loop {
            match chars.get(i) {
                None => return Err(bad_selector(i)),
                Some('"') => break,
                Some('\\') => {
                    match chars.get(i + 1) {
                        Some('"') => value.push('"'),
                        Some('\\') => value.push('\\'),
                        Some('n') => value.push('\n'),
                        _ => return Err(bad_selector(i)),
                    }
                    i += 2;
                }
                Some(&c) => {
                    value.push(c);
                    i += 1;
                }
            }
        }
        i += 1;
        out.push(Matcher { label, op, value });
        skip_ws(&mut i);
        match chars.get(i) {
            Some(',') => i += 1,
            Some('}') => {
                i += 1;
                break;
            }
            _ => return Err(bad_selector(i)),
        }
    }
    skip_ws(&mut i);
    if i != chars.len() {
        return Err(bad_selector(i));
    }
    Ok(out)
}

/// Хост и порт для сравнения: имя без регистра, `localhost`, `127.0.0.1` и `::1` — один адрес.
fn endpoint(host: &str, port: u16) -> (String, u16) {
    let host = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_lowercase();
    let host = match host.as_str() {
        "localhost" | "127.0.0.1" | "::1" => "loopback".to_string(),
        _ => host,
    };
    (host, port)
}

/// `instance` цели как хост и порт; без порта — `default_port` (порт по умолчанию схемы).
fn instance_endpoint(instance: &str, default_port: u16) -> Option<(String, u16)> {
    let (host, port) = if let Some(rest) = instance.strip_prefix('[') {
        let (host, tail) = rest.split_once(']')?;
        match tail.strip_prefix(':') {
            Some(p) => (host, p.parse().ok()?),
            None if tail.is_empty() => (host, default_port),
            None => return None,
        }
    } else {
        match instance.rsplit_once(':') {
            Some((h, p)) if !h.contains(':') => (h, p.parse().ok()?),
            Some(_) => (instance, default_port),
            None => (instance, default_port),
        }
    };
    Some(endpoint(host, port))
}

/// Совпадает ли `instance` цели с хостом и портом адреса сервера (схема и путь не учитываются).
pub fn instance_matches(instance: &str, base: &url::Url) -> bool {
    let (Some(host), Some(port)) = (base.host_str(), base.port_or_known_default()) else {
        return false;
    };
    let scheme_port = if base.scheme() == "https" { 443 } else { 80 };
    instance_endpoint(instance, scheme_port).is_some_and(|e| e == endpoint(host, port))
}

/// Автоподбор (спека I1c §3): цели, у которых `instance` — хост и порт адреса сервера, по
/// порядку `job`, затем `instance`; первая — подобранная.
pub fn matching_targets<'a>(base: &url::Url, targets: &'a [Target]) -> Vec<&'a Target> {
    let mut found: Vec<&Target> = targets
        .iter()
        .filter(|t| instance_matches(&t.instance, base))
        .collect();
    found.sort();
    found
}
````

<!-- block: patch t02-src-tauri-src-storage-profile-rs step=2.3 -->
````
diff --git a/src-tauri/src/storage/profile.rs b/src-tauri/src/storage/profile.rs
index 2db4629..593f3fd 100644
--- a/src-tauri/src/storage/profile.rs
+++ b/src-tauri/src/storage/profile.rs
@@ -365,6 +365,9 @@ pub fn validate_server(s: &ServerConfig) -> AppResult<()> {
             server = s.name
         )));
     }
+    if let Some(selector) = &s.history_selector {
+        crate::history::targets::parse_selector(selector)?;
+    }
     validate_ca(&s.ca, &s.name)?;
     validate_proxy(&s.proxy)
 }
@@ -907,6 +910,11 @@ mod tests {
         );
         assert_eq!(p.servers[0].history_selector, None);
         assert!(validate_profile(&p).is_ok());
+        p.servers[0].history_selector = Some(r#"{job="vllm"}"#.into());
+        assert!(validate_profile(&p).is_ok());
+        p.servers[0].history_selector = Some("job=vllm".into());
+        assert_invalid(&p, "errors.history.selector");
+        p.servers[0].history_selector = None;
 
         let with = |h: HistorySource| {
             let mut p = sample_profile("a");
````

<!-- block: locales t02-locales-t02 step=2.3 -->
````
{
  "en": { "errors": { "history": {
    "selector": "The series selector is invalid at character {{pos}}: use {job=\"vllm\", instance=\"host:port\"}",
    "selectorName": "The series selector must not set __name__: Glafira adds the series names itself"
  } } },
  "ru": { "errors": { "history": {
    "selector": "Ошибка в селекторе серий на символе {{pos}}: используйте {job=\"vllm\", instance=\"хост:порт\"}",
    "selectorName": "В селекторе серий не задают __name__: имена серий Glafira подставляет сама"
  } } }
}
````

- [ ] **Шаг 2.4: Проверки.**

Run: `cd src-tauri && cargo test --lib -q`
Expected: `test result: ok. 256 passed`
Run: `cd src-tauri && cargo clippy --lib --tests -q -- -D warnings && cargo fmt --check`
Expected: без вывода
Run: `pnpm i18n:check`
Expected: `✓ словари en, ru согласованы, вызовов из Rust: 115`

- [ ] **Шаг 2.5: Коммит.**

<!-- block: shell t02-commit step=2.5 -->
````
git add src-tauri/src/history/mod.rs \
  src-tauri/src/history/targets.rs \
  src-tauri/src/lib.rs \
  src-tauri/src/storage/profile.rs \
  src/locales/en.json \
  src/locales/ru.json
git commit -s -q -F - <<'EOF'
feat(history): series selectors and target auto-match

A strict PromQL selector parser (matchers, quoted values, no __name__) with
the error position, selector rendering for templates, and auto-match of
inference targets to servers by host and port (localhost equals 127.0.0.1
and ::1, scheme default ports).

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
````

---

### Задача 3: Шаблоны PromQL и окна истории

Чистые функции над картой метрик адаптера (спека I1c §4): по запросу на ряд — суммы датчиков, максимум и среднее
KV, скорости счётчиков, завершения по причинам, доля попаданий в prefix-cache, `histogram_quantile` (p50, p95, p99 у
TTFT, ITL, E2E и p95 у очереди) — 20 запросов у vLLM и SGLang; кандидаты имён — через `or`, фильтр карты — условием
на метку. Окна `now`, `1h`, `24h`, `7d`, `30d`: шаг, границы (конец округлён до шага) и интервал `rate`.

**Files:**
- Create: `src-tauri/src/history/promql.rs`
- Create: `src-tauri/src/history/window.rs`
- Modify: `src-tauri/src/history/mod.rs`

**Interfaces:**
- Consumes: `servers::adapters::{MetricSpec, Canonical, MetricKind}`, `history::targets::escape`.
- Produces: `history::promql::{queries, Query { key, expr, group_by }, QueryKey}`; `QueryKey::chart()` — id графика
  вкладки «Метрики», `QueryKey::canonical()`; `history::window::{Window, Range { start, end, step, rate }, range,
  duration}`; JSON `Window` — `now`, `1h`, `24h`, `7d`, `30d`.

- [ ] **Шаг 3.1: Тесты.** Тесты `history/promql.rs` (точный текст шаблонов) и `history/window.rs`; модули подключаются в `history/mod.rs`.

<!-- block: patch t03-src-tauri-src-history-mod-rs step=3.1 -->
````
diff --git a/src-tauri/src/history/mod.rs b/src-tauri/src/history/mod.rs
index c22e4da..d64e6d9 100644
--- a/src-tauri/src/history/mod.rs
+++ b/src-tauri/src/history/mod.rs
@@ -1,3 +1,6 @@
-//! История метрик из Prometheus или VictoriaMetrics (спека I1c): цели и селекторы серий.
+//! История метрик из Prometheus или VictoriaMetrics (спека I1c): цели и селекторы серий, шаблоны
+//! PromQL и окна.
 
+pub mod promql;
 pub mod targets;
+pub mod window;
````

<!-- block: file t03-src-tauri-src-history-promql-rs-tests step=3.1 path=src-tauri/src/history/promql.rs part=tests -->
````
#[cfg(test)]
mod tests {
    use super::*;
    use crate::servers::adapters::adapter;
    use crate::servers::detect::DetectedKind;

    const SEL: &str = r#"job="vllm",instance="127.0.0.1:8000""#;

    fn exprs(kind: DetectedKind) -> Vec<(QueryKey, String)> {
        queries(adapter(kind).metric_map(), SEL, "1m")
            .into_iter()
            .map(|q| (q.key, q.expr))
            .collect()
    }

    fn expr(kind: DetectedKind, key: QueryKey) -> String {
        exprs(kind)
            .into_iter()
            .find(|(k, _)| *k == key)
            .map(|(_, e)| e)
            .unwrap_or_else(|| panic!("{key:?}"))
    }

    #[test]
    fn vllm_templates() {
        let v = DetectedKind::Vllm;
        assert_eq!(exprs(v).len(), 20);
        assert_eq!(
            expr(v, QueryKey::RequestsRunning),
            r#"sum(vllm:num_requests_running{job="vllm",instance="127.0.0.1:8000"})"#
        );
        assert_eq!(
            expr(v, QueryKey::KvUsage),
            r#"max(vllm:kv_cache_usage_perc{job="vllm",instance="127.0.0.1:8000"}) or max(vllm:gpu_cache_usage_perc{job="vllm",instance="127.0.0.1:8000"})"#
        );
        assert_eq!(
            expr(v, QueryKey::KvUsageAvg),
            r#"avg(vllm:kv_cache_usage_perc{job="vllm",instance="127.0.0.1:8000"}) or avg(vllm:gpu_cache_usage_perc{job="vllm",instance="127.0.0.1:8000"})"#
        );
        assert_eq!(
            expr(v, QueryKey::PromptTokens),
            r#"sum(rate(vllm:prompt_tokens_total{job="vllm",instance="127.0.0.1:8000"}[1m]))"#
        );
        assert_eq!(
            expr(v, QueryKey::Aborted),
            r#"sum(rate(vllm:request_success_total{job="vllm",instance="127.0.0.1:8000",finished_reason="abort"}[1m]))"#
        );
        assert_eq!(
            expr(v, QueryKey::FinishedByReason),
            r#"sum by (finished_reason) (rate(vllm:request_success_total{job="vllm",instance="127.0.0.1:8000"}[1m]))"#
        );
        assert_eq!(
            expr(v, QueryKey::PrefixCacheHitRatio),
            r#"(sum(rate(vllm:prefix_cache_hits_total{job="vllm",instance="127.0.0.1:8000"}[1m])) or sum(rate(vllm:gpu_prefix_cache_hits_total{job="vllm",instance="127.0.0.1:8000"}[1m]))) / (sum(rate(vllm:prefix_cache_queries_total{job="vllm",instance="127.0.0.1:8000"}[1m])) or sum(rate(vllm:gpu_prefix_cache_queries_total{job="vllm",instance="127.0.0.1:8000"}[1m])))"#
        );
        assert_eq!(
            expr(
                v,
                QueryKey::Quantile {
                    canonical: Canonical::Itl,
                    q: 0.95
                }
            ),
            r#"histogram_quantile(0.95, sum by (le) (rate(vllm:inter_token_latency_seconds_bucket{job="vllm",instance="127.0.0.1:8000"}[1m])) or sum by (le) (rate(vllm:time_per_output_token_seconds_bucket{job="vllm",instance="127.0.0.1:8000"}[1m])))"#
        );
        // Очередь — только p95, длины запросов и ответов история не запрашивает.
        let queue: Vec<_> = exprs(v)
            .into_iter()
            .filter(|(k, _)| k.chart() == "queue")
            .collect();
        assert_eq!(queue.len(), 1);
        assert!(
            !exprs(v)
                .iter()
                .any(|(_, e)| e.contains("request_prompt_tokens"))
        );
    }

    #[test]
    fn sglang_templates() {
        let s = DetectedKind::Sglang;
        assert_eq!(exprs(s).len(), 20);
        assert_eq!(
            expr(s, QueryKey::Aborted),
            r#"sum(rate(sglang:num_aborted_requests_total{job="vllm",instance="127.0.0.1:8000"}[1m])) or sum(rate(sglang:num_aborted_requests{job="vllm",instance="127.0.0.1:8000"}[1m]))"#
        );
        assert_eq!(
            expr(s, QueryKey::PrefixCacheHitRatio),
            r#"(sum(rate(sglang:cached_tokens_total{job="vllm",instance="127.0.0.1:8000"}[1m]))) / (sum(rate(sglang:prompt_tokens_total{job="vllm",instance="127.0.0.1:8000"}[1m])))"#
        );
        let finished = queries(adapter(s).metric_map(), SEL, "1m")
            .into_iter()
            .find(|q| q.key == QueryKey::FinishedByReason)
            .unwrap();
        assert_eq!(finished.group_by, Some("finished_reason"));
    }

    #[test]
    fn openai_has_no_queries_and_keys_name_charts() {
        assert!(exprs(DetectedKind::Openai).is_empty());
        let charts: std::collections::BTreeSet<_> = exprs(DetectedKind::Vllm)
            .into_iter()
            .map(|(k, _)| k.chart())
            .collect();
        assert_eq!(
            charts.into_iter().collect::<Vec<_>>(),
            [
                "e2e",
                "finished",
                "itl",
                "kv",
                "preemptions",
                "prefixCache",
                "queue",
                "requests",
                "tokens",
                "ttft"
            ]
        );
        assert_eq!(
            QueryKey::Quantile {
                canonical: Canonical::Ttft,
                q: 0.5
            }
            .canonical(),
            Canonical::Ttft
        );
    }

    #[test]
    fn filter_values_are_escaped() {
        let spec = [MetricSpec::new(
            Canonical::RequestAbortedTotal,
            MetricKind::Counter,
            &["x_total"],
        )
        .only("reason", "a\"b")];
        let q = queries(&spec, "job=\"j\"", "5m");
        assert_eq!(
            q[0].expr,
            r#"sum(rate(x_total{job="j",reason="a\"b"}[5m]))"#
        );
    }
}
````

<!-- block: file t03-src-tauri-src-history-window-rs-tests step=3.1 path=src-tauri/src/history/window.rs part=tests -->
````
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_keep_points_per_series_within_limits() {
        for (w, points) in [
            (Window::Now, 60),
            (Window::Hour, 60),
            (Window::Day, 960),
            (Window::Week, 840),
            (Window::Month, 960),
        ] {
            assert_eq!(w.length_sec() / w.step_sec(), points, "{w:?}");
            assert!(points <= 1000);
            if w != Window::Now {
                assert!(w.step_sec() >= 60, "{w:?}");
            }
        }
    }

    #[test]
    fn range_is_aligned_to_the_step() {
        let r = range(Window::Day, 1_000_000_055, Some(15));
        assert_eq!(r.end, 999_999_990);
        assert_eq!(r.end - r.start, 86_400);
        assert_eq!((r.step, r.rate), (90, 90));
        // Четыре интервала опроса больше шага; без интервала — не меньше минуты.
        assert_eq!(range(Window::Hour, 3_600, Some(30)).rate, 120);
        assert_eq!(range(Window::Now, 3_600, None).rate, 60);
        assert_eq!(range(Window::Now, 3_600, Some(5)).rate, 60);
        assert_eq!(range(Window::Now, 100, None).start, 0);
    }

    #[test]
    fn window_names_and_durations() {
        let names: Vec<String> = [
            Window::Now,
            Window::Hour,
            Window::Day,
            Window::Week,
            Window::Month,
        ]
        .iter()
        .map(|w| serde_json::to_string(w).unwrap())
        .collect();
        assert_eq!(
            names,
            [r#""now""#, r#""1h""#, r#""24h""#, r#""7d""#, r#""30d""#]
        );
        assert_eq!(
            serde_json::from_str::<Window>(r#""7d""#).unwrap(),
            Window::Week
        );
        assert_eq!(duration(60), "1m");
        assert_eq!(duration(90), "90s");
        assert_eq!(duration(2_700), "45m");
    }
}
````

- [ ] **Шаг 3.2: Увидеть падение.**

Run: `cd src-tauri && cargo test --lib -q history`
Expected: FAIL — не компилируется: тестам не хватает типов и функций, которые появятся в шаге 3

- [ ] **Шаг 3.3: Реализация.** Шаблоны и окна.

<!-- block: file t03-src-tauri-src-history-promql-rs step=3.3 path=src-tauri/src/history/promql.rs part=impl -->
````
//! Шаблоны PromQL истории (спека I1c §4): чистые функции над картой метрик адаптера. Кандидаты
//! имён серий объединяются через `or` — тот же порядок «первый найденный», что у карты в I1b;
//! селектор сервера и фильтр карты становятся условиями на метки.

use crate::servers::adapters::{Canonical, MetricKind, MetricSpec};

/// Ряд истории, который даёт один запрос.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QueryKey {
    RequestsRunning,
    RequestsWaiting,
    KvUsage,
    KvUsageAvg,
    PromptTokens,
    GenerationTokens,
    Preemptions,
    Aborted,
    FinishedByReason,
    PrefixCacheHitRatio,
    Quantile { canonical: Canonical, q: f64 },
}

impl QueryKey {
    /// График вкладки «Метрики», которому принадлежит ряд (для `partial`).
    pub fn chart(self) -> &'static str {
        match self {
            Self::RequestsRunning | Self::RequestsWaiting => "requests",
            Self::KvUsage | Self::KvUsageAvg => "kv",
            Self::PromptTokens | Self::GenerationTokens => "tokens",
            Self::Preemptions => "preemptions",
            Self::Aborted | Self::FinishedByReason => "finished",
            Self::PrefixCacheHitRatio => "prefixCache",
            Self::Quantile { canonical, .. } => match canonical {
                Canonical::Ttft => "ttft",
                Canonical::Itl => "itl",
                Canonical::E2e => "e2e",
                _ => "queue",
            },
        }
    }

    /// Каноническая метрика ряда: если ряд пуст во всём окне, она попадает в `missing`.
    pub fn canonical(self) -> Canonical {
        match self {
            Self::RequestsRunning => Canonical::RequestsRunning,
            Self::RequestsWaiting => Canonical::RequestsWaiting,
            Self::KvUsage | Self::KvUsageAvg => Canonical::KvUsage,
            Self::PromptTokens => Canonical::PromptTokensTotal,
            Self::GenerationTokens => Canonical::GenerationTokensTotal,
            Self::Preemptions => Canonical::PreemptionsTotal,
            Self::Aborted => Canonical::RequestAbortedTotal,
            Self::FinishedByReason => Canonical::RequestSuccessTotal,
            Self::PrefixCacheHitRatio => Canonical::PrefixCacheHitsTotal,
            Self::Quantile { canonical, .. } => canonical,
        }
    }
}

/// Запрос истории: ряд и выражение PromQL.
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub key: QueryKey,
    pub expr: String,
    /// Метка, по которой ряд разбит (`finished_reason`); у остальных — `None`.
    pub group_by: Option<&'static str>,
}

/// Квантили перцентилей истории: p50, p95, p99 у TTFT, ITL, E2E; p95 у очереди.
const LATENCY: [(Canonical, &[f64]); 4] = [
    (Canonical::Ttft, &[0.5, 0.95, 0.99]),
    (Canonical::Itl, &[0.5, 0.95, 0.99]),
    (Canonical::E2e, &[0.5, 0.95, 0.99]),
    (Canonical::QueueTime, &[0.95]),
];

/// Серия `name` с условиями селектора `selector` (без скобок) и фильтром карты.
fn series(name: &str, selector: &str, spec: &MetricSpec) -> String {
    match spec.filter {
        Some((label, value)) => format!(
            "{name}{{{selector},{label}=\"{}\"}}",
            super::targets::escape(value)
        ),
        None => format!("{name}{{{selector}}}"),
    }
}

/// `f(кандидат 1) or f(кандидат 2) …`.
fn any_of(spec: &MetricSpec, f: impl Fn(&str) -> String) -> String {
    spec.candidates
        .iter()
        .map(|c| f(c))
        .collect::<Vec<_>>()
        .join(" or ")
}

/// Запросы истории сервера по карте метрик адаптера. `selector` — условия селектора сервера без
/// фигурных скобок (`targets::render`), `range` — интервал `rate` (`window::duration`). Метрики,
/// которых нет в карте (тип «OpenAI-совместимый»), запросов не дают.
pub fn queries(map: &[MetricSpec], selector: &str, range: &str) -> Vec<Query> {
    let spec = |c: Canonical| map.iter().find(|s| s.canonical == c);
    let mut out = Vec::new();
    let mut push = |key: QueryKey, expr: String| {
        out.push(Query {
            key,
            expr,
            group_by: None,
        })
    };
    let gauge =
        |s: &MetricSpec, agg: &str| any_of(s, |n| format!("{agg}({})", series(n, selector, s)));
    let rate = |s: &MetricSpec| {
        any_of(s, |n| {
            format!("sum(rate({}[{range}]))", series(n, selector, s))
        })
    };
    if let Some(s) = spec(Canonical::RequestsRunning) {
        push(QueryKey::RequestsRunning, gauge(s, "sum"));
    }
    if let Some(s) = spec(Canonical::RequestsWaiting) {
        push(QueryKey::RequestsWaiting, gauge(s, "sum"));
    }
    if let Some(s) = spec(Canonical::KvUsage) {
        push(QueryKey::KvUsage, gauge(s, "max"));
        push(QueryKey::KvUsageAvg, gauge(s, "avg"));
    }
    for (c, key) in [
        (Canonical::PromptTokensTotal, QueryKey::PromptTokens),
        (Canonical::GenerationTokensTotal, QueryKey::GenerationTokens),
        (Canonical::PreemptionsTotal, QueryKey::Preemptions),
        (Canonical::RequestAbortedTotal, QueryKey::Aborted),
    ] {
        if let Some(s) = spec(c) {
            push(key, rate(s));
        }
    }
    if let (Some(h), Some(q)) = (
        spec(Canonical::PrefixCacheHitsTotal),
        spec(Canonical::PrefixCacheQueriesTotal),
    ) {
        push(
            QueryKey::PrefixCacheHitRatio,
            format!("({}) / ({})", rate(h), rate(q)),
        );
    }
    for (c, quantiles) in LATENCY {
        let Some(s) = spec(c).filter(|s| s.kind == MetricKind::Histogram) else {
            continue;
        };
        let buckets = any_of(s, |n| {
            format!(
                "sum by (le) (rate({}[{range}]))",
                series(&format!("{n}_bucket"), selector, s)
            )
        });
        for &q in quantiles {
            push(
                QueryKey::Quantile { canonical: c, q },
                format!("histogram_quantile({q}, {buckets})"),
            );
        }
    }
    if let Some(s) = spec(Canonical::RequestSuccessTotal) {
        let label = s.group_by.unwrap_or("finished_reason");
        out.push(Query {
            key: QueryKey::FinishedByReason,
            expr: any_of(s, |n| {
                format!(
                    "sum by ({label}) (rate({}[{range}]))",
                    series(n, selector, s)
                )
            }),
            group_by: Some(label),
        });
    }
    out
}
````

<!-- block: file t03-src-tauri-src-history-window-rs step=3.3 path=src-tauri/src/history/window.rs part=impl -->
````
//! Окна истории (спека I1c §4): длина, шаг, интервал `rate` и границы запроса. Шаг — не меньше
//! 1 мин у окон 1 ч…30 д и не больше 1000 точек на ряд; окно `now` предзаполняет «Сейчас».

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Window {
    #[serde(rename = "now")]
    Now,
    #[serde(rename = "1h")]
    Hour,
    #[serde(rename = "24h")]
    Day,
    #[serde(rename = "7d")]
    Week,
    #[serde(rename = "30d")]
    Month,
}

impl Window {
    /// Длина окна, с.
    pub fn length_sec(self) -> u64 {
        match self {
            Self::Now => 15 * 60,
            Self::Hour => 3_600,
            Self::Day => 86_400,
            Self::Week => 7 * 86_400,
            Self::Month => 30 * 86_400,
        }
    }

    /// Шаг точек, с.
    pub fn step_sec(self) -> u64 {
        match self {
            Self::Now => 15,
            Self::Hour => 60,
            Self::Day => 90,
            Self::Week => 12 * 60,
            Self::Month => 45 * 60,
        }
    }
}

/// Границы запроса `query_range` и интервал `rate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    /// Начало и конец, с Unix; конец — `now`, округлённый вниз до шага.
    pub start: u64,
    pub end: u64,
    pub step: u64,
    /// Интервал `rate`, с: не меньше шага, минуты и четырёх интервалов опроса цели.
    pub rate: u64,
}

/// Окно, которое кончается сейчас. `scrape_sec` — интервал опроса цели источником (`None` —
/// неизвестен). Конец округляется до шага: повторные запросы окна попадают в кеш источника.
pub fn range(window: Window, now_sec: u64, scrape_sec: Option<u64>) -> Range {
    let step = window.step_sec();
    let end = now_sec - now_sec % step;
    Range {
        start: end.saturating_sub(window.length_sec()),
        end,
        step,
        rate: step.max(60).max(4 * scrape_sec.unwrap_or(0)),
    }
}

/// Длительность PromQL: `90s`, `1m`, `45m`.
pub fn duration(sec: u64) -> String {
    if sec.is_multiple_of(60) {
        format!("{}m", sec / 60)
    } else {
        format!("{sec}s")
    }
}
````

- [ ] **Шаг 3.4: Проверки.**

Run: `cd src-tauri && cargo test --lib -q`
Expected: `test result: ok. 263 passed`
Run: `cd src-tauri && cargo clippy --lib --tests -q -- -D warnings && cargo fmt --check`
Expected: без вывода

- [ ] **Шаг 3.5: Коммит.**

<!-- block: shell t03-commit step=3.5 -->
````
git add src-tauri/src/history/mod.rs \
  src-tauri/src/history/promql.rs \
  src-tauri/src/history/window.rs
git commit -s -q -F - <<'EOF'
feat(history): PromQL templates and history windows

Templates are pure functions over the adapter metric map: sums, KV max and
average, rates, finished requests by reason, prefix-cache ratio and
histogram_quantile, with series-name candidates joined by `or`. Windows
now/1h/24h/7d/30d define the step, the aligned range and the rate interval.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
````

---

### Задача 4: Клиент источника и ответы настоящего Prometheus

Клиент источника поверх механики `ServerClient` (спека I1c §4): CA, прокси, таймауты, лимит 8 МБ, без
перенаправлений; вход по токену или по имени и паролю (`with_basic_auth`), запросы с параметрами (`get_query`).
Разбор `vector` и `matrix` (`NaN`, `±Inf` — нет значения), ошибки API — ключами `errors.history.*`, интервал опроса
цели из `/api/v1/targets`. Тесты разбирают ответы настоящего Prometheus 3.15.1, который опрашивал мок-серверы
(`testdata/prometheus-3.15.1`, README рассказывает, как сняты). Тестовый HTTP-сервер ищет ответ и по пути без
параметров запроса.

**Files:**
- Create: `src-tauri/src/history/client.rs`
- Create: `src-tauri/src/history/testdata/prometheus-3.15.1/README.md`
- Create: `src-tauri/src/history/testdata/prometheus-3.15.1/bad-data.json`
- Create: `src-tauri/src/history/testdata/prometheus-3.15.1/buildinfo.json`
- Create: `src-tauri/src/history/testdata/prometheus-3.15.1/discover-sglang.json`
- Create: `src-tauri/src/history/testdata/prometheus-3.15.1/discover.json`
- Create: `src-tauri/src/history/testdata/prometheus-3.15.1/empty.json`
- Create: `src-tauri/src/history/testdata/prometheus-3.15.1/range-sglang-30000.json`
- Create: `src-tauri/src/history/testdata/prometheus-3.15.1/range-vllm-8000.json`
- Create: `src-tauri/src/history/testdata/prometheus-3.15.1/range-vllm-8003.json`
- Create: `src-tauri/src/history/testdata/prometheus-3.15.1/targets.json`
- Create: `src-tauri/src/history/testdata/prometheus-3.15.1/up.json`
- Modify: `src-tauri/src/history/mod.rs`
- Modify: `src-tauri/src/servers/client.rs`
- Modify: `src-tauri/src/servers/testing.rs`
- Modify: `src/locales/en.json`
- Modify: `src/locales/ru.json`

**Interfaces:**
- Produces: `history::client::{HistoryClient, BuildInfo, SourceKind, Sample, RangeSeries, parse_api, parse_vector,
  parse_matrix, parse_duration, scrape_interval_of, as_server, ERROR_TEXT_LIMIT}`; `HistoryClient::new(&HistorySource,
  Timeouts, Option<String>, &PolicyStatus)`, методы `buildinfo`, `query`, `query_range(&str, &Range)`,
  `scrape_interval(job, instance)`; `ServerClient::with_basic_auth`, `ServerClient::get_query`; тестовые
  `history::client::tests::{snapshot, source}`.

- [ ] **Шаг 4.1: Тесты.** Тесты `history/client.rs` на снятых ответах и против тестового HTTP-сервера; модуль подключается.

<!-- block: file t04-src-tauri-src-history-client-rs-tests step=4.1 path=src-tauri/src/history/client.rs part=tests -->
````
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::servers::testing::{Reply, TestServer};
    use crate::storage::profile::{ProxyConfig, sample_history};

    pub(crate) fn snapshot(name: &str) -> String {
        std::fs::read_to_string(format!(
            "{}/src/history/testdata/prometheus-3.15.1/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    fn reply(status: u16, body: &str) -> HttpReply {
        HttpReply {
            status,
            content_type: Some("application/json".into()),
            body: body.as_bytes().to_vec(),
            truncated: false,
        }
    }

    const TIMEOUTS: Timeouts = Timeouts {
        connect_ms: 2_000,
        request_ms: 2_000,
    };

    pub(crate) fn source(url: &str, auth: HistoryAuth) -> HistorySource {
        HistorySource {
            url: url.into(),
            auth,
            proxy: ProxyConfig::None,
            ..sample_history()
        }
    }

    #[test]
    fn real_answers_are_parsed() {
        let up = parse_api(&reply(200, &snapshot("up.json")), "/api/v1/query").unwrap();
        let up = parse_vector(&up).unwrap();
        assert_eq!(up.len(), 4);
        assert!(up.iter().all(|s| s.value == Some(1.0)));
        let discover = parse_api(&reply(200, &snapshot("discover.json")), "/api/v1/query").unwrap();
        let mut found: Vec<_> = parse_vector(&discover)
            .unwrap()
            .into_iter()
            .map(|s| format!("{} {}", s.labels["job"], s.labels["instance"]))
            .collect();
        found.sort();
        assert_eq!(
            found,
            [
                "sglang 127.0.0.1:30000",
                "vllm 127.0.0.1:8000",
                "vllm 127.0.0.1:8001",
                "vllm 127.0.0.1:8003"
            ]
        );
        let empty = parse_api(&reply(200, &snapshot("empty.json")), "/api/v1/query").unwrap();
        assert!(parse_vector(&empty).unwrap().is_empty());
        let targets = parse_api(&reply(200, &snapshot("targets.json")), "/api/v1/targets").unwrap();
        assert_eq!(
            scrape_interval_of(&targets, "vllm", "127.0.0.1:8001"),
            Some(5)
        );
        assert_eq!(scrape_interval_of(&targets, "vllm", "127.0.0.1:9"), None);
    }

    #[test]
    fn real_range_answer_is_parsed_with_gaps() {
        let doc: Value = serde_json::from_str(&snapshot("range-vllm-8000.json")).unwrap();
        let first = &doc["queries"][0];
        let reply = reply(200, &first["response"].to_string());
        let series = parse_matrix(&parse_api(&reply, "/api/v1/query_range").unwrap()).unwrap();
        assert_eq!(series.len(), 1);
        let points = &series[0].points;
        let start = doc["start"].as_u64().unwrap();
        // Prometheus запущен позже начала окна: точек меньше 61, первая — не в начале окна.
        assert!(points.len() < 61 && points.len() > 40, "{}", points.len());
        assert!(points[0].0 > start);
        assert!(points.windows(2).all(|w| w[1].0 - w[0].0 == 15));
    }

    #[test]
    fn values_nan_and_infinity_are_missing() {
        let data = serde_json::json!({
            "resultType": "matrix",
            "result": [{ "metric": {}, "values": [[10, "1.5"], [25, "NaN"], [40, "+Inf"], [55, "-Inf"]] }]
        });
        let s = parse_matrix(&data).unwrap();
        assert_eq!(
            s[0].points,
            [(10, Some(1.5)), (25, None), (40, None), (55, None)]
        );
        assert!(s[0].labels.is_empty());
        let e =
            parse_matrix(&serde_json::json!({ "resultType": "vector", "result": [] })).unwrap_err();
        assert_eq!(e.key, "errors.history.unexpected");
    }

    #[test]
    fn errors_by_status_and_body() {
        let key = |r: HttpReply| parse_api(&r, "/api/v1/query").unwrap_err().key;
        assert_eq!(
            key(reply(400, &snapshot("bad-data.json"))),
            "errors.history.badData"
        );
        let e = parse_api(&reply(400, &snapshot("bad-data.json")), "/q").unwrap_err();
        assert!(e.params["message"].contains("unclosed left parenthesis"));
        assert_eq!(key(reply(401, "")), "errors.history.unauthorized");
        assert_eq!(key(reply(403, "{}")), "errors.history.forbidden");
        assert_eq!(
            key(reply(
                503,
                r#"{"status":"error","errorType":"timeout","error":"query timed out"}"#
            )),
            "errors.history.timeout"
        );
        assert_eq!(
            key(reply(
                400,
                r#"{"status":"error","errorType":"bad_data","error":"exceeded maximum resolution of 11,000 points per timeseries"}"#
            )),
            "errors.history.tooManyPoints"
        );
        assert_eq!(
            key(reply(
                422,
                r#"{"status":"error","errorType":"execution","error":"many-to-many matching not allowed"}"#
            )),
            "errors.history.failed"
        );
        assert_eq!(key(reply(502, "<html>")), "errors.history.status");
        assert_eq!(key(reply(200, "<html>")), "errors.history.unexpected");
        let mut big = reply(200, "{");
        big.truncated = true;
        assert_eq!(key(big), "errors.history.tooLarge");
        let long = format!(
            r#"{{"status":"error","errorType":"bad_data","error":"{}"}}"#,
            "x".repeat(600)
        );
        let e = parse_api(&reply(400, &long), "/q").unwrap_err();
        assert_eq!(e.params["message"].chars().count(), ERROR_TEXT_LIMIT + 1);
    }

    #[test]
    fn durations() {
        assert_eq!(parse_duration("5s"), Some(5));
        assert_eq!(parse_duration("1m30s"), Some(90));
        assert_eq!(parse_duration("1h"), Some(3_600));
        for bad in ["", "5", "1.5s", "500ms", "x"] {
            assert_eq!(parse_duration(bad), None, "{bad}");
        }
    }

    #[tokio::test]
    async fn requests_carry_the_query_and_the_secret() {
        let server = TestServer::start(vec![
            (
                "/api/v1/status/buildinfo",
                Reply::json(&snapshot("buildinfo.json")),
            ),
            ("/api/v1/query", Reply::json(&snapshot("up.json"))),
            ("/api/v1/targets", Reply::json(&snapshot("targets.json"))),
        ])
        .await;
        let policy = PolicyStatus::default();
        let c = HistoryClient::new(
            &source(&server.url(), HistoryAuth::Bearer),
            TIMEOUTS,
            Some("prom-t0ken".into()),
            &policy,
        )
        .unwrap();
        assert!(!format!("{c:?}").contains("prom-t0ken"));
        let info = c.buildinfo().await.unwrap().unwrap();
        assert_eq!(
            (info.kind, info.version.as_deref()),
            (SourceKind::Prometheus, Some("3.15.1"))
        );
        assert_eq!(c.query("up").await.unwrap().len(), 4);
        assert_eq!(
            c.scrape_interval("sglang", "127.0.0.1:30000").await,
            Some(5)
        );
        let seen = server.seen.lock().unwrap().clone();
        assert_eq!(seen[1].path, "/api/v1/query?query=up");
        assert!(
            seen.iter()
                .all(|s| s.authorization.as_deref() == Some("Bearer prom-t0ken"))
        );

        let basic = HistoryClient::new(
            &source(
                &server.url(),
                HistoryAuth::Basic {
                    username: "reader".into(),
                },
            ),
            TIMEOUTS,
            Some("pa55".into()),
            &policy,
        )
        .unwrap();
        basic.query("up").await.unwrap();
        let last = server.seen.lock().unwrap().last().cloned().unwrap();
        // reader:pa55 в base64.
        assert_eq!(
            last.authorization.as_deref(),
            Some("Basic cmVhZGVyOnBhNTU=")
        );
    }

    #[tokio::test]
    async fn missing_buildinfo_is_not_an_error() {
        let server = TestServer::start(vec![]).await;
        let c = HistoryClient::new(
            &source(&server.url(), HistoryAuth::None),
            TIMEOUTS,
            None,
            &PolicyStatus::default(),
        )
        .unwrap();
        assert_eq!(c.buildinfo().await.unwrap(), None);
        assert_eq!(c.scrape_interval("vllm", "x:1").await, None);
    }
}
````

<!-- block: patch t04-src-tauri-src-history-mod-rs step=4.1 -->
````
diff --git a/src-tauri/src/history/mod.rs b/src-tauri/src/history/mod.rs
index d64e6d9..78a6e88 100644
--- a/src-tauri/src/history/mod.rs
+++ b/src-tauri/src/history/mod.rs
@@ -1,6 +1,7 @@
-//! История метрик из Prometheus или VictoriaMetrics (спека I1c): цели и селекторы серий, шаблоны
-//! PromQL и окна.
+//! История метрик из Prometheus или VictoriaMetrics (спека I1c): клиент источника, цели и
+//! селекторы серий, шаблоны PromQL и окна.
 
+pub mod client;
 pub mod promql;
 pub mod targets;
 pub mod window;
````

- [ ] **Шаг 4.2: Увидеть падение.**

Run: `cd src-tauri && cargo test --lib -q history`
Expected: FAIL — не компилируется: тестам не хватает типов и функций, которые появятся в шаге 3

- [ ] **Шаг 4.3: Реализация.** Клиент, снятые ответы Prometheus, поиск ответа тестового сервера без параметров, тексты ошибок.

<!-- block: file t04-src-tauri-src-history-client-rs step=4.3 path=src-tauri/src/history/client.rs part=impl -->
````
//! Клиент источника истории (спека I1c §4): HTTP API Prometheus и VictoriaMetrics поверх механики
//! `ServerClient` — CA, прокси, таймауты инсталляции, лимит ответа, без перенаправлений. Токен или
//! пароль источника не попадает ни в логи, ни в ошибки.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::history::window::Range;
use crate::policy::PolicyStatus;
use crate::servers::client::{HttpReply, METRICS_LIMIT, ServerClient, Timeouts};
use crate::storage::profile::{
    HISTORY_NAME, HistoryAuth, HistorySource, ServerConfig, ServerKind, parse_history_url,
};

/// Сколько символов текста ошибки источника показывается (спека I1c §3).
pub const ERROR_TEXT_LIMIT: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Prometheus,
    Victoriametrics,
}

/// Сведения `buildinfo`: тип источника и версия.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildInfo {
    pub kind: SourceKind,
    pub version: Option<String>,
}

/// Значение мгновенного запроса: метки и значение (`NaN`, `±Inf` — `None`).
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub labels: BTreeMap<String, String>,
    pub value: Option<f64>,
}

/// Ряд `query_range`: метки и точки (секунды Unix, значение).
#[derive(Debug, Clone, PartialEq)]
pub struct RangeSeries {
    pub labels: BTreeMap<String, String>,
    pub points: Vec<(u64, Option<f64>)>,
}

pub struct HistoryClient {
    http: ServerClient,
}

/// Секрет не печатается: клиент может попасть в лог.
impl std::fmt::Debug for HistoryClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HistoryClient")
            .field("http", &self.http)
            .finish()
    }
}

fn history_err(key_and_text: crate::i18n::LocalizedText, code: ErrorCode) -> AppError {
    AppError::localized(code, key_and_text)
}

fn unexpected(path: &str) -> AppError {
    history_err(
        ltext!(
            "errors.history.unexpected",
            "Unexpected answer of the metrics history source to {{path}}: not the Prometheus HTTP API",
            path = path
        ),
        ErrorCode::Internal,
    )
}

/// Текст ошибки источника, не длиннее `ERROR_TEXT_LIMIT` символов.
fn clip(text: &str) -> String {
    match text.char_indices().nth(ERROR_TEXT_LIMIT) {
        Some((i, _)) => format!("{}…", &text[..i]),
        None => text.to_string(),
    }
}

/// Поле `data` ответа API или ошибка по коду и телу (спека I1c §3).
pub fn parse_api(reply: &HttpReply, path: &str) -> AppResult<Value> {
    if reply.truncated {
        return Err(history_err(
            ltext!(
                "errors.history.tooLarge",
                "The answer of the metrics history source is larger than 8 MB: choose a shorter window"
            ),
            ErrorCode::Internal,
        ));
    }
    match reply.status {
        401 => {
            return Err(history_err(
                ltext!(
                    "errors.history.unauthorized",
                    "The metrics history source rejected the sign-in (401): check the token or the user name and password"
                ),
                ErrorCode::AuthFailed,
            ));
        }
        403 => {
            return Err(history_err(
                ltext!(
                    "errors.history.forbidden",
                    "The metrics history source denied access (403): the account may not read metrics"
                ),
                ErrorCode::AuthFailed,
            ));
        }
        _ => {}
    }
    let body: Option<Value> = serde_json::from_slice(&reply.body).ok();
    let field = |k: &str| body.as_ref().and_then(|b| b.get(k)).and_then(Value::as_str);
    match field("status") {
        Some("success") if reply.ok() => body
            .and_then(|mut b| b.get_mut("data").map(Value::take))
            .ok_or_else(|| unexpected(path)),
        Some("error") => {
            let text = clip(field("error").unwrap_or_default());
            Err(match field("errorType") {
                Some("timeout") => history_err(
                    ltext!(
                        "errors.history.timeout",
                        "The metrics history source did not finish the query in time: choose a shorter window"
                    ),
                    ErrorCode::Timeout,
                ),
                Some("bad_data") if text.contains("maximum resolution") => history_err(
                    ltext!(
                        "errors.history.tooManyPoints",
                        "The metrics history source refused the query: too many points per series"
                    ),
                    ErrorCode::InvalidInput,
                ),
                Some("bad_data") => history_err(
                    ltext!(
                        "errors.history.badData",
                        "The metrics history source rejected the query: {{message}}",
                        message = text
                    ),
                    ErrorCode::InvalidInput,
                ),
                _ => history_err(
                    ltext!(
                        "errors.history.failed",
                        "The metrics history source failed the query: {{message}}",
                        message = text
                    ),
                    ErrorCode::ConnectionFailed,
                ),
            })
        }
        _ if !reply.ok() => Err(history_err(
            ltext!(
                "errors.history.status",
                "The metrics history source answered {{path}} with {{status}}",
                path = path,
                status = reply.status
            ),
            ErrorCode::ConnectionFailed,
        )),
        _ => Err(unexpected(path)),
    }
}

/// Значение точки: строка числа; `NaN` и `±Inf` — отсутствие значения.
fn value(v: &Value) -> Option<f64> {
    v.as_str()?.parse::<f64>().ok().filter(|x| x.is_finite())
}

fn labels(v: &Value) -> BTreeMap<String, String> {
    v.get("metric")
        .and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

fn result<'a>(data: &'a Value, kind: &str, path: &str) -> AppResult<&'a Vec<Value>> {
    if data.get("resultType").and_then(Value::as_str) != Some(kind) {
        return Err(unexpected(path));
    }
    data.get("result")
        .and_then(Value::as_array)
        .ok_or_else(|| unexpected(path))
}

/// `data` мгновенного запроса (`resultType: vector`).
pub fn parse_vector(data: &Value) -> AppResult<Vec<Sample>> {
    let path = "/api/v1/query";
    result(data, "vector", path)?
        .iter()
        .map(|s| {
            let v = s.get("value").and_then(Value::as_array);
            Ok(Sample {
                labels: labels(s),
                value: v.and_then(|v| v.get(1)).and_then(value),
            })
        })
        .collect()
}

/// `data` запроса за окно (`resultType: matrix`).
pub fn parse_matrix(data: &Value) -> AppResult<Vec<RangeSeries>> {
    let path = "/api/v1/query_range";
    result(data, "matrix", path)?
        .iter()
        .map(|s| {
            let points = s
                .get("values")
                .and_then(Value::as_array)
                .ok_or_else(|| unexpected(path))?
                .iter()
                .map(|p| {
                    let t = p
                        .get(0)
                        .and_then(Value::as_f64)
                        .ok_or_else(|| unexpected(path))?;
                    Ok((t.round() as u64, p.get(1).and_then(value)))
                })
                .collect::<AppResult<Vec<_>>>()?;
            Ok(RangeSeries {
                labels: labels(s),
                points,
            })
        })
        .collect()
}

/// Длительность Go/Prometheus (`5s`, `1m`, `1m30s`, `1h`) в секундах; дробные и `ms` — `None`.
pub fn parse_duration(s: &str) -> Option<u64> {
    let mut total = 0u64;
    let mut num = String::new();
    let mut any = false;
    for c in s.chars() {
        if c.is_ascii_digit() {
            num.push(c);
            continue;
        }
        let n: u64 = num.parse().ok()?;
        num.clear();
        total += n * match c {
            's' => 1,
            'm' => 60,
            'h' => 3_600,
            'd' => 86_400,
            _ => return None,
        };
        any = true;
    }
    (any && num.is_empty()).then_some(total)
}

/// `scrapeInterval` цели `job`, `instance` из `data` ответа `/api/v1/targets`.
pub fn scrape_interval_of(data: &Value, job: &str, instance: &str) -> Option<u64> {
    data.get("activeTargets")?
        .as_array()?
        .iter()
        .find(|t| {
            let l = t.get("labels");
            let get = |k: &str| l.and_then(|l| l.get(k)).and_then(Value::as_str);
            get("job") == Some(job) && get("instance") == Some(instance)
        })?
        .get("scrapeInterval")?
        .as_str()
        .and_then(parse_duration)
}

impl HistoryClient {
    /// Клиент источника `h`; `secret` — токен или пароль из формы или keychain.
    pub fn new(
        h: &HistorySource,
        timeouts: Timeouts,
        secret: Option<String>,
        policy: &PolicyStatus,
    ) -> AppResult<Self> {
        parse_history_url(h)?;
        let server = ServerConfig {
            id: crate::secrets::HISTORY.into(),
            name: HISTORY_NAME.into(),
            base_url: h.url.clone(),
            kind: ServerKind::Auto,
            metrics_path: "/metrics".into(),
            ca: h.ca.clone(),
            proxy: h.proxy.clone(),
            history_selector: None,
            extra: Default::default(),
        };
        let secret = secret.filter(|s| !s.is_empty());
        let http = match (&h.auth, secret) {
            (HistoryAuth::Bearer, token) => ServerClient::new(&server, timeouts, token, policy)?,
            (HistoryAuth::Basic { username }, Some(password)) => {
                ServerClient::new(&server, timeouts, None, policy)?
                    .with_basic_auth(username, password)
            }
            _ => ServerClient::new(&server, timeouts, None, policy)?,
        };
        Ok(Self { http })
    }

    async fn api(&self, path: &str, query: &[(&str, &str)]) -> AppResult<Value> {
        let reply = self.http.get_query(path, query, METRICS_LIMIT).await?;
        parse_api(&reply, path)
    }

    /// Тип и версия источника; `None` — у источника нет `buildinfo` (404).
    pub async fn buildinfo(&self) -> AppResult<Option<BuildInfo>> {
        let path = "/api/v1/status/buildinfo";
        let reply = self.http.get_query(path, &[], METRICS_LIMIT).await?;
        if reply.status == 404 {
            return Ok(None);
        }
        let data = parse_api(&reply, path)?;
        let version = data
            .get("version")
            .and_then(Value::as_str)
            .map(String::from);
        let vm = version
            .as_deref()
            .is_some_and(|v| v.to_lowercase().contains("victoria"));
        Ok(Some(BuildInfo {
            kind: if vm {
                SourceKind::Victoriametrics
            } else {
                SourceKind::Prometheus
            },
            version,
        }))
    }

    /// Мгновенный запрос.
    pub async fn query(&self, expr: &str) -> AppResult<Vec<Sample>> {
        parse_vector(&self.api("/api/v1/query", &[("query", expr)]).await?)
    }

    /// Запрос за окно `range`.
    pub async fn query_range(&self, expr: &str, range: &Range) -> AppResult<Vec<RangeSeries>> {
        let (start, end, step) = (
            range.start.to_string(),
            range.end.to_string(),
            range.step.to_string(),
        );
        let data = self
            .api(
                "/api/v1/query_range",
                &[
                    ("query", expr),
                    ("start", &start),
                    ("end", &end),
                    ("step", &step),
                ],
            )
            .await?;
        parse_matrix(&data)
    }

    /// Интервал опроса цели; без `/api/v1/targets` или без цели — `None`.
    pub async fn scrape_interval(&self, job: &str, instance: &str) -> Option<u64> {
        let data = self
            .api("/api/v1/targets", &[("state", "active")])
            .await
            .ok()?;
        scrape_interval_of(&data, job, instance)
    }
}
````

<!-- block: file t04-src-tauri-src-history-testdata-prometheus-3-15-1-readme-md step=4.3 path=src-tauri/src/history/testdata/prometheus-3.15.1/README.md -->
````
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
````

<!-- block: file t04-src-tauri-src-history-testdata-prometheus-3-15-1-bad-data-json step=4.3 path=src-tauri/src/history/testdata/prometheus-3.15.1/bad-data.json -->
````
{"status":"error","errorType":"bad_data","error":"invalid parameter \"query\": 1:39: parse error: unclosed left parenthesis"}
````

<!-- block: file t04-src-tauri-src-history-testdata-prometheus-3-15-1-buildinfo-json step=4.3 path=src-tauri/src/history/testdata/prometheus-3.15.1/buildinfo.json -->
````
{"status":"success","data":{"version":"3.15.1","revision":"non-git","branch":"non-git","buildUser":"reproducible@reproducible","buildDate":"20261021-13:24:03","goVersion":"go1.27.3"}}
````

<!-- block: file t04-src-tauri-src-history-testdata-prometheus-3-15-1-discover-sglang-json step=4.3 path=src-tauri/src/history/testdata/prometheus-3.15.1/discover-sglang.json -->
````
{"status":"success","data":{"resultType":"vector","result":[{"metric":{"instance":"127.0.0.1:30000","job":"sglang"},"value":[1793468540.558,"175"]}]}}
````

<!-- block: file t04-src-tauri-src-history-testdata-prometheus-3-15-1-discover-json step=4.3 path=src-tauri/src/history/testdata/prometheus-3.15.1/discover.json -->
````
{"status":"success","data":{"resultType":"vector","result":[{"metric":{"instance":"127.0.0.1:8001","job":"vllm"},"value":[1793468540.553,"774"]},{"metric":{"instance":"127.0.0.1:8003","job":"vllm"},"value":[1793468540.553,"389"]},{"metric":{"instance":"127.0.0.1:8000","job":"vllm"},"value":[1793468540.553,"389"]},{"metric":{"instance":"127.0.0.1:30000","job":"sglang"},"value":[1793468540.553,"175"]}]}}
````

<!-- block: file t04-src-tauri-src-history-testdata-prometheus-3-15-1-empty-json step=4.3 path=src-tauri/src/history/testdata/prometheus-3.15.1/empty.json -->
````
{"status":"success","data":{"resultType":"vector","result":[]}}
````

<!-- block: file t04-src-tauri-src-history-testdata-prometheus-3-15-1-range-sglang-30000-json step=4.3 path=src-tauri/src/history/testdata/prometheus-3.15.1/range-sglang-30000.json -->
````
{"start":1793467635,"end":1793468535,"step":15,"rate":"1m","queries":[{"expr":"sum(sglang:num_running_reqs{job=\"sglang\",instance=\"127.0.0.1:30000\"})","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"4"],[1793467815,"4"],[1793467830,"4"],[1793467845,"4"],[1793467860,"4"],[1793467875,"4"],[1793467890,"4"],[1793467905,"4"],[1793467920,"4"],[1793467935,"4"],[1793467950,"4"],[1793467965,"4"],[1793467980,"4"],[1793467995,"4"],[1793468010,"4"],[1793468025,"4"],[1793468040,"4"],[1793468055,"4"],[1793468070,"4"],[1793468085,"4"],[1793468100,"4"],[1793468115,"4"],[1793468130,"4"],[1793468145,"4"],[1793468160,"4"],[1793468175,"4"],[1793468190,"4"],[1793468205,"4"],[1793468220,"4"],[1793468235,"4"],[1793468250,"4"],[1793468265,"4"],[1793468280,"4"],[1793468295,"4"],[1793468310,"4"],[1793468325,"4"],[1793468340,"4"],[1793468355,"4"],[1793468370,"4"],[1793468385,"4"],[1793468400,"4"],[1793468415,"4"],[1793468430,"4"],[1793468445,"4"],[1793468460,"4"],[1793468475,"4"],[1793468490,"4"],[1793468505,"4"],[1793468520,"4"],[1793468535,"4"]]}]}}},{"expr":"sum(sglang:num_queue_reqs{job=\"sglang\",instance=\"127.0.0.1:30000\"})","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"2"],[1793467815,"2"],[1793467830,"2"],[1793467845,"2"],[1793467860,"2"],[1793467875,"2"],[1793467890,"2"],[1793467905,"2"],[1793467920,"2"],[1793467935,"2"],[1793467950,"2"],[1793467965,"2"],[1793467980,"2"],[1793467995,"2"],[1793468010,"2"],[1793468025,"2"],[1793468040,"2"],[1793468055,"2"],[1793468070,"2"],[1793468085,"2"],[1793468100,"2"],[1793468115,"2"],[1793468130,"2"],[1793468145,"2"],[1793468160,"2"],[1793468175,"2"],[1793468190,"2"],[1793468205,"2"],[1793468220,"2"],[1793468235,"2"],[1793468250,"2"],[1793468265,"2"],[1793468280,"2"],[1793468295,"2"],[1793468310,"2"],[1793468325,"2"],[1793468340,"2"],[1793468355,"2"],[1793468370,"2"],[1793468385,"2"],[1793468400,"2"],[1793468415,"2"],[1793468430,"2"],[1793468445,"2"],[1793468460,"2"],[1793468475,"2"],[1793468490,"2"],[1793468505,"2"],[1793468520,"2"],[1793468535,"2"]]}]}}},{"expr":"max(sglang:token_usage{job=\"sglang\",instance=\"127.0.0.1:30000\"})","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0.3963"],[1793467815,"0.7832"],[1793467830,"0.6339"],[1793467845,"0.3143"],[1793467860,"0.7574"],[1793467875,"0.688"],[1793467890,"0.2301"],[1793467905,"0.7205"],[1793467920,"0.7324"],[1793467935,"0.2548"],[1793467950,"0.6732"],[1793467965,"0.7661"],[1793467980,"0.3386"],[1793467995,"0.6164"],[1793468010,"0.7885"],[1793468025,"0.4196"],[1793468040,"0.5512"],[1793468055,"0.7991"],[1793468070,"0.4962"],[1793468085,"0.4791"],[1793468100,"0.7977"],[1793468115,"0.5668"],[1793468130,"0.4013"],[1793468145,"0.7844"],[1793468160,"0.6302"],[1793468175,"0.3195"],[1793468190,"0.7594"],[1793468205,"0.6849"],[1793468220,"0.2354"],[1793468235,"0.7231"],[1793468250,"0.7299"],[1793468265,"0.2495"],[1793468280,"0.6764"],[1793468295,"0.7643"],[1793468310,"0.3334"],[1793468325,"0.6202"],[1793468340,"0.7874"],[1793468355,"0.4146"],[1793468370,"0.5555"],[1793468385,"0.7988"],[1793468400,"0.4915"],[1793468415,"0.4838"],[1793468430,"0.7982"],[1793468445,"0.5626"],[1793468460,"0.4063"],[1793468475,"0.7856"],[1793468490,"0.6264"],[1793468505,"0.3247"],[1793468520,"0.7613"],[1793468535,"0.6817"]]}]}}},{"expr":"avg(sglang:token_usage{job=\"sglang\",instance=\"127.0.0.1:30000\"})","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0.3963"],[1793467815,"0.7832"],[1793467830,"0.6339"],[1793467845,"0.3143"],[1793467860,"0.7574"],[1793467875,"0.688"],[1793467890,"0.2301"],[1793467905,"0.7205"],[1793467920,"0.7324"],[1793467935,"0.2548"],[1793467950,"0.6732"],[1793467965,"0.7661"],[1793467980,"0.3386"],[1793467995,"0.6164"],[1793468010,"0.7885"],[1793468025,"0.4196"],[1793468040,"0.5512"],[1793468055,"0.7991"],[1793468070,"0.4962"],[1793468085,"0.4791"],[1793468100,"0.7977"],[1793468115,"0.5668"],[1793468130,"0.4013"],[1793468145,"0.7844"],[1793468160,"0.6302"],[1793468175,"0.3195"],[1793468190,"0.7594"],[1793468205,"0.6849"],[1793468220,"0.2354"],[1793468235,"0.7231"],[1793468250,"0.7299"],[1793468265,"0.2495"],[1793468280,"0.6764"],[1793468295,"0.7643"],[1793468310,"0.3334"],[1793468325,"0.6202"],[1793468340,"0.7874"],[1793468355,"0.4146"],[1793468370,"0.5555"],[1793468385,"0.7988"],[1793468400,"0.4915"],[1793468415,"0.4838"],[1793468430,"0.7982"],[1793468445,"0.5626"],[1793468460,"0.4063"],[1793468475,"0.7856"],[1793468490,"0.6264"],[1793468505,"0.3247"],[1793468520,"0.7613"],[1793468535,"0.6817"]]}]}}},{"expr":"sum(rate(sglang:prompt_tokens_total{job=\"sglang\",instance=\"127.0.0.1:30000\"}[1m]))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"423.16"],[1793467815,"1023.16"],[1793467830,"1623.16"],[1793467845,"2223.16"],[1793467860,"2399.912730446166"],[1793467875,"2400"],[1793467890,"2399.912730446166"],[1793467905,"2399.912730446166"],[1793467920,"2399.8690980491974"],[1793467935,"2399.912730446166"],[1793467950,"2400"],[1793467965,"2399.9563644297377"],[1793467980,"2400"],[1793467995,"2399.9563644297377"],[1793468010,"2399.9563644297377"],[1793468025,"2399.9563644297377"],[1793468040,"2400"],[1793468055,"2399.9563644297377"],[1793468070,"2399.9563644297377"],[1793468085,"2399.9563644297377"],[1793468100,"2399.9563644297377"],[1793468115,"2399.912730446166"],[1793468130,"2399.9563644297377"],[1793468145,"2399.912730446166"],[1793468160,"2399.912730446166"],[1793468175,"2399.8690980491974"],[1793468190,"2399.9563644297377"],[1793468205,"2399.912730446166"],[1793468220,"2399.912730446166"],[1793468235,"2400"],[1793468250,"2400"],[1793468265,"2400"],[1793468280,"2399.9563644297377"],[1793468295,"2399.9563644297377"],[1793468310,"2399.9563644297377"],[1793468325,"2399.9563644297377"],[1793468340,"2400"],[1793468355,"2399.912730446166"],[1793468370,"2399.9563644297377"],[1793468385,"2399.9563644297377"],[1793468400,"2399.9563644297377"],[1793468415,"2399.912730446166"],[1793468430,"2399.912730446166"],[1793468445,"2399.9563644297377"],[1793468460,"2399.9563644297377"],[1793468475,"2400"],[1793468490,"2399.9563644297377"],[1793468505,"2399.9563644297377"],[1793468520,"2400.1309162317943"],[1793468535,"2400.218201654696"]]}]}}},{"expr":"sum(rate(sglang:generation_tokens_total{job=\"sglang\",instance=\"127.0.0.1:30000\"}[1m]))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"158.685"],[1793467815,"383.685"],[1793467830,"608.6850000000001"],[1793467845,"833.685"],[1793467860,"899.9672739173121"],[1793467875,"900"],[1793467890,"899.9672739173121"],[1793467905,"899.9672739173121"],[1793467920,"899.9509117684491"],[1793467935,"899.9672739173121"],[1793467950,"900"],[1793467965,"899.9836366611516"],[1793467980,"900"],[1793467995,"899.9836366611516"],[1793468010,"899.9836366611516"],[1793468025,"899.9836366611516"],[1793468040,"900"],[1793468055,"899.9836366611516"],[1793468070,"899.9836366611516"],[1793468085,"899.9836366611516"],[1793468100,"899.9836366611516"],[1793468115,"899.9672739173121"],[1793468130,"899.9836366611516"],[1793468145,"899.9672739173121"],[1793468160,"899.9672739173121"],[1793468175,"899.9509117684491"],[1793468190,"899.9836366611516"],[1793468205,"899.9672739173121"],[1793468220,"899.9672739173121"],[1793468235,"900"],[1793468250,"900"],[1793468265,"900"],[1793468280,"899.9836366611516"],[1793468295,"899.9836366611516"],[1793468310,"899.9836366611516"],[1793468325,"899.9836366611516"],[1793468340,"900"],[1793468355,"899.9672739173121"],[1793468370,"899.9836366611516"],[1793468385,"899.9836366611516"],[1793468400,"899.9836366611516"],[1793468415,"899.9672739173121"],[1793468430,"899.9672739173121"],[1793468445,"899.9836366611516"],[1793468460,"899.9836366611516"],[1793468475,"900"],[1793468490,"899.9836366611516"],[1793468505,"899.9836366611516"],[1793468520,"900.0490935869229"],[1793468535,"900.0818256205109"]]}]}}},{"expr":"sum(rate(sglang:num_retracted_requests_total{job=\"sglang\",instance=\"127.0.0.1:30000\"}[1m]))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0.010578999999999995"],[1793467815,"0.025579"],[1793467830,"0.040579"],[1793467845,"0.055578999999999996"],[1793467860,"0.05999781826115415"],[1793467875,"0.05999999999999999"],[1793467890,"0.05999781826115413"],[1793467905,"0.05999781826115414"],[1793467920,"0.059996727451229935"],[1793467935,"0.059997818261154155"],[1793467950,"0.06000000000000001"],[1793467965,"0.059998909110743424"],[1793467980,"0.05999999999999998"],[1793467995,"0.05999890911074345"],[1793468010,"0.05999890911074345"],[1793468025,"0.05999890911074345"],[1793468040,"0.05999999999999998"],[1793468055,"0.05999890911074345"],[1793468070,"0.05999890911074345"],[1793468085,"0.05999890911074339"],[1793468100,"0.05999890911074345"],[1793468115,"0.059997818261154155"],[1793468130,"0.05999890911074345"],[1793468145,"0.059997818261154155"],[1793468160,"0.05999781826115409"],[1793468175,"0.059996727451229956"],[1793468190,"0.05999890911074345"],[1793468205,"0.059997818261154155"],[1793468220,"0.059997818261154155"],[1793468235,"0.05999999999999994"],[1793468250,"0.06000000000000001"],[1793468265,"0.06000000000000001"],[1793468280,"0.05999890911074345"],[1793468295,"0.05999890911074345"],[1793468310,"0.05999890911074339"],[1793468325,"0.05999890911074339"],[1793468340,"0.060000000000000074"],[1793468355,"0.059997818261154225"],[1793468370,"0.05999890911074339"],[1793468385,"0.05999890911074339"],[1793468400,"0.05999890911074339"],[1793468415,"0.059997818261154225"],[1793468430,"0.059997818261154225"],[1793468445,"0.05999890911074339"],[1793468460,"0.05999890911074339"],[1793468475,"0.05999999999999994"],[1793468490,"0.05999890911074352"],[1793468505,"0.05999890911074352"],[1793468520,"0.06000327290579481"],[1793468535,"0.06000545504136734"]]}]}}},{"expr":"sum(rate(sglang:num_aborted_requests_total{job=\"sglang\",instance=\"127.0.0.1:30000\"}[1m])) or sum(rate(sglang:num_aborted_requests{job=\"sglang\",instance=\"127.0.0.1:30000\"}[1m]))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0.02468433333333334"],[1793467815,"0.059684333333333346"],[1793467830,"0.09468433333333334"],[1793467845,"0.12968433333333332"],[1793467860,"0.13999490927602634"],[1793467875,"0.13999999999999999"],[1793467890,"0.13999490927602634"],[1793467905,"0.1399949092760263"],[1793467920,"0.13999236405286986"],[1793467935,"0.1399949092760264"],[1793467950,"0.13999999999999999"],[1793467965,"0.13999745459173468"],[1793467980,"0.13999999999999999"],[1793467995,"0.13999745459173468"],[1793468010,"0.13999745459173474"],[1793468025,"0.13999745459173474"],[1793468040,"0.1399999999999999"],[1793468055,"0.13999745459173463"],[1793468070,"0.13999745459173474"],[1793468085,"0.13999745459173474"],[1793468100,"0.13999745459173474"],[1793468115,"0.13999490927602626"],[1793468130,"0.13999745459173463"],[1793468145,"0.1399949092760264"],[1793468160,"0.1399949092760264"],[1793468175,"0.1399923640528699"],[1793468190,"0.13999745459173463"],[1793468205,"0.1399949092760264"],[1793468220,"0.1399949092760264"],[1793468235,"0.1399999999999999"],[1793468250,"0.14000000000000004"],[1793468265,"0.14000000000000004"],[1793468280,"0.13999745459173474"],[1793468295,"0.13999745459173474"],[1793468310,"0.1399974545917345"],[1793468325,"0.13999745459173474"],[1793468340,"0.14000000000000004"],[1793468355,"0.1399949092760264"],[1793468370,"0.13999745459173474"],[1793468385,"0.1399974545917345"],[1793468400,"0.13999745459173474"],[1793468415,"0.1399949092760264"],[1793468430,"0.1399949092760264"],[1793468445,"0.13999745459173474"],[1793468460,"0.1399974545917345"],[1793468475,"0.14000000000000004"],[1793468490,"0.13999745459173474"],[1793468505,"0.13999745459173474"],[1793468520,"0.14000763678018807"],[1793468535,"0.14001272842985704"]]}]}}},{"expr":"(sum(rate(sglang:cached_tokens_total{job=\"sglang\",instance=\"127.0.0.1:30000\"}[1m]))) / (sum(rate(sglang:prompt_tokens_total{job=\"sglang\",instance=\"127.0.0.1:30000\"}[1m])))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0.4"],[1793467815,"0.4"],[1793467830,"0.39999999999999997"],[1793467845,"0.39999999999999997"],[1793467860,"0.39999999999999997"],[1793467875,"0.4"],[1793467890,"0.39999999999999997"],[1793467905,"0.39999999999999997"],[1793467920,"0.4"],[1793467935,"0.39999999999999997"],[1793467950,"0.4"],[1793467965,"0.39999999999999997"],[1793467980,"0.4"],[1793467995,"0.39999999999999997"],[1793468010,"0.39999999999999997"],[1793468025,"0.39999999999999997"],[1793468040,"0.4"],[1793468055,"0.39999999999999997"],[1793468070,"0.39999999999999997"],[1793468085,"0.39999999999999997"],[1793468100,"0.39999999999999997"],[1793468115,"0.39999999999999997"],[1793468130,"0.39999999999999997"],[1793468145,"0.39999999999999997"],[1793468160,"0.39999999999999997"],[1793468175,"0.4"],[1793468190,"0.39999999999999997"],[1793468205,"0.39999999999999997"],[1793468220,"0.39999999999999997"],[1793468235,"0.4"],[1793468250,"0.4"],[1793468265,"0.4"],[1793468280,"0.39999999999999997"],[1793468295,"0.39999999999999997"],[1793468310,"0.39999999999999997"],[1793468325,"0.39999999999999997"],[1793468340,"0.4"],[1793468355,"0.39999999999999997"],[1793468370,"0.39999999999999997"],[1793468385,"0.39999999999999997"],[1793468400,"0.39999999999999997"],[1793468415,"0.39999999999999997"],[1793468430,"0.39999999999999997"],[1793468445,"0.39999999999999997"],[1793468460,"0.39999999999999997"],[1793468475,"0.4"],[1793468490,"0.39999999999999997"],[1793468505,"0.39999999999999997"],[1793468520,"0.4"],[1793468535,"0.39999999999999997"]]}]}}},{"expr":"histogram_quantile(0.5, sum by (le) (rate(sglang:time_to_first_token_seconds_bucket{job=\"sglang\",instance=\"127.0.0.1:30000\"}[1m])))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0.07500000000000001"],[1793467815,"0.075"],[1793467830,"0.07500000000000001"],[1793467845,"0.07500000000000001"],[1793467860,"0.075"],[1793467875,"0.07500000000000001"],[1793467890,"0.075"],[1793467905,"0.075"],[1793467920,"0.07500000000000001"],[1793467935,"0.075"],[1793467950,"0.07500000000000001"],[1793467965,"0.07500000000000001"],[1793467980,"0.07500000000000001"],[1793467995,"0.07500000000000001"],[1793468010,"0.07500000000000001"],[1793468025,"0.07500000000000001"],[1793468040,"0.07500000000000001"],[1793468055,"0.07500000000000001"],[1793468070,"0.07500000000000001"],[1793468085,"0.07500000000000001"],[1793468100,"0.07500000000000001"],[1793468115,"0.075"],[1793468130,"0.07500000000000001"],[1793468145,"0.075"],[1793468160,"0.075"],[1793468175,"0.07500000000000001"],[1793468190,"0.07500000000000001"],[1793468205,"0.075"],[1793468220,"0.075"],[1793468235,"0.07500000000000001"],[1793468250,"0.07500000000000001"],[1793468265,"0.07500000000000001"],[1793468280,"0.07500000000000001"],[1793468295,"0.07500000000000001"],[1793468310,"0.07500000000000001"],[1793468325,"0.07500000000000001"],[1793468340,"0.07500000000000001"],[1793468355,"0.075"],[1793468370,"0.07500000000000001"],[1793468385,"0.07500000000000001"],[1793468400,"0.07500000000000001"],[1793468415,"0.075"],[1793468430,"0.075"],[1793468445,"0.07500000000000001"],[1793468460,"0.07500000000000001"],[1793468475,"0.07500000000000001"],[1793468490,"0.07500000000000001"],[1793468505,"0.07500000000000001"],[1793468520,"0.07500000000000001"],[1793468535,"0.07500000000000001"]]}]}}},{"expr":"histogram_quantile(0.95, sum by (le) (rate(sglang:time_to_first_token_seconds_bucket{job=\"sglang\",instance=\"127.0.0.1:30000\"}[1m])))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0.3375"],[1793467815,"0.3374999999999998"],[1793467830,"0.33749999999999974"],[1793467845,"0.3375"],[1793467860,"0.33749999999999997"],[1793467875,"0.33750000000000013"],[1793467890,"0.33749999999999997"],[1793467905,"0.33749999999999997"],[1793467920,"0.3375000000000001"],[1793467935,"0.33749999999999997"],[1793467950,"0.33750000000000013"],[1793467965,"0.3375"],[1793467980,"0.33750000000000013"],[1793467995,"0.3375"],[1793468010,"0.3375"],[1793468025,"0.3375"],[1793468040,"0.33750000000000013"],[1793468055,"0.3375"],[1793468070,"0.3375"],[1793468085,"0.3375"],[1793468100,"0.3375"],[1793468115,"0.33749999999999997"],[1793468130,"0.3375"],[1793468145,"0.33749999999999997"],[1793468160,"0.33749999999999997"],[1793468175,"0.3375000000000001"],[1793468190,"0.3375"],[1793468205,"0.33749999999999997"],[1793468220,"0.33749999999999997"],[1793468235,"0.33750000000000013"],[1793468250,"0.33750000000000013"],[1793468265,"0.33750000000000013"],[1793468280,"0.3375"],[1793468295,"0.3375"],[1793468310,"0.3375"],[1793468325,"0.3375"],[1793468340,"0.33750000000000013"],[1793468355,"0.33749999999999997"],[1793468370,"0.3375"],[1793468385,"0.3375"],[1793468400,"0.3375"],[1793468415,"0.33749999999999997"],[1793468430,"0.33749999999999997"],[1793468445,"0.3375"],[1793468460,"0.3375"],[1793468475,"0.33750000000000013"],[1793468490,"0.3375"],[1793468505,"0.3375"],[1793468520,"0.3375000000000002"],[1793468535,"0.33750000000000013"]]}]}}},{"expr":"histogram_quantile(0.99, sum by (le) (rate(sglang:time_to_first_token_seconds_bucket{job=\"sglang\",instance=\"127.0.0.1:30000\"}[1m])))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0.5100000000000001"],[1793467815,"0.5099999999999999"],[1793467830,"0.5099999999999995"],[1793467845,"0.5099999999999999"],[1793467860,"0.51"],[1793467875,"0.5100000000000005"],[1793467890,"0.51"],[1793467905,"0.51"],[1793467920,"0.5100000000000003"],[1793467935,"0.51"],[1793467950,"0.5100000000000005"],[1793467965,"0.51"],[1793467980,"0.5100000000000005"],[1793467995,"0.51"],[1793468010,"0.51"],[1793468025,"0.51"],[1793468040,"0.5100000000000005"],[1793468055,"0.51"],[1793468070,"0.51"],[1793468085,"0.51"],[1793468100,"0.51"],[1793468115,"0.51"],[1793468130,"0.51"],[1793468145,"0.51"],[1793468160,"0.51"],[1793468175,"0.5100000000000003"],[1793468190,"0.51"],[1793468205,"0.51"],[1793468220,"0.51"],[1793468235,"0.5100000000000005"],[1793468250,"0.5100000000000005"],[1793468265,"0.5100000000000005"],[1793468280,"0.51"],[1793468295,"0.51"],[1793468310,"0.51"],[1793468325,"0.51"],[1793468340,"0.5100000000000005"],[1793468355,"0.51"],[1793468370,"0.51"],[1793468385,"0.51"],[1793468400,"0.51"],[1793468415,"0.51"],[1793468430,"0.51"],[1793468445,"0.51"],[1793468460,"0.51"],[1793468475,"0.5100000000000005"],[1793468490,"0.51"],[1793468505,"0.51"],[1793468520,"0.5100000000000005"],[1793468535,"0.5100000000000005"]]}]}}},{"expr":"histogram_quantile(0.5, sum by (le) (rate(sglang:inter_token_latency_seconds_bucket{job=\"sglang\",instance=\"127.0.0.1:30000\"}[1m])))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0.0125"],[1793467815,"0.012499999999999999"],[1793467830,"0.0125"],[1793467845,"0.0125"],[1793467860,"0.0125"],[1793467875,"0.0125"],[1793467890,"0.0125"],[1793467905,"0.0125"],[1793467920,"0.0125"],[1793467935,"0.0125"],[1793467950,"0.0125"],[1793467965,"0.0125"],[1793467980,"0.0125"],[1793467995,"0.0125"],[1793468010,"0.0125"],[1793468025,"0.0125"],[1793468040,"0.0125"],[1793468055,"0.0125"],[1793468070,"0.0125"],[1793468085,"0.0125"],[1793468100,"0.0125"],[1793468115,"0.0125"],[1793468130,"0.0125"],[1793468145,"0.0125"],[1793468160,"0.0125"],[1793468175,"0.0125"],[1793468190,"0.0125"],[1793468205,"0.0125"],[1793468220,"0.0125"],[1793468235,"0.0125"],[1793468250,"0.0125"],[1793468265,"0.0125"],[1793468280,"0.0125"],[1793468295,"0.0125"],[1793468310,"0.0125"],[1793468325,"0.0125"],[1793468340,"0.0125"],[1793468355,"0.0125"],[1793468370,"0.0125"],[1793468385,"0.0125"],[1793468400,"0.0125"],[1793468415,"0.0125"],[1793468430,"0.0125"],[1793468445,"0.0125"],[1793468460,"0.0125"],[1793468475,"0.0125"],[1793468490,"0.0125"],[1793468505,"0.0125"],[1793468520,"0.012499999999999999"],[1793468535,"0.0125"]]}]}}},{"expr":"histogram_quantile(0.95, sum by (le) (rate(sglang:inter_token_latency_seconds_bucket{job=\"sglang\",instance=\"127.0.0.1:30000\"}[1m])))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0.019305555555555555"],[1793467815,"0.019305555555555555"],[1793467830,"0.019305555555555555"],[1793467845,"0.019305555555555555"],[1793467860,"0.019305555555555555"],[1793467875,"0.019305555555555555"],[1793467890,"0.019305555555555555"],[1793467905,"0.019305555555555555"],[1793467920,"0.019305555555555555"],[1793467935,"0.019305555555555555"],[1793467950,"0.019305555555555555"],[1793467965,"0.019305555555555555"],[1793467980,"0.019305555555555555"],[1793467995,"0.019305555555555555"],[1793468010,"0.019305555555555555"],[1793468025,"0.019305555555555555"],[1793468040,"0.019305555555555555"],[1793468055,"0.019305555555555555"],[1793468070,"0.019305555555555555"],[1793468085,"0.019305555555555555"],[1793468100,"0.019305555555555555"],[1793468115,"0.019305555555555555"],[1793468130,"0.019305555555555555"],[1793468145,"0.019305555555555555"],[1793468160,"0.019305555555555555"],[1793468175,"0.019305555555555555"],[1793468190,"0.019305555555555555"],[1793468205,"0.019305555555555555"],[1793468220,"0.019305555555555555"],[1793468235,"0.019305555555555555"],[1793468250,"0.019305555555555555"],[1793468265,"0.019305555555555555"],[1793468280,"0.019305555555555555"],[1793468295,"0.019305555555555555"],[1793468310,"0.019305555555555555"],[1793468325,"0.019305555555555555"],[1793468340,"0.019305555555555555"],[1793468355,"0.019305555555555555"],[1793468370,"0.019305555555555555"],[1793468385,"0.019305555555555555"],[1793468400,"0.019305555555555555"],[1793468415,"0.019305555555555555"],[1793468430,"0.019305555555555555"],[1793468445,"0.019305555555555555"],[1793468460,"0.019305555555555555"],[1793468475,"0.019305555555555555"],[1793468490,"0.019305555555555555"],[1793468505,"0.019305555555555555"],[1793468520,"0.019305555555555555"],[1793468535,"0.019305555555555555"]]}]}}},{"expr":"histogram_quantile(0.99, sum by (le) (rate(sglang:inter_token_latency_seconds_bucket{job=\"sglang\",instance=\"127.0.0.1:30000\"}[1m])))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0.022749999999999996"],[1793467815,"0.022750000000000003"],[1793467830,"0.022750000000000003"],[1793467845,"0.022750000000000003"],[1793467860,"0.02275"],[1793467875,"0.02275"],[1793467890,"0.02275"],[1793467905,"0.02275"],[1793467920,"0.02275"],[1793467935,"0.02275"],[1793467950,"0.02275"],[1793467965,"0.022749999999999996"],[1793467980,"0.02275"],[1793467995,"0.022749999999999996"],[1793468010,"0.022749999999999996"],[1793468025,"0.022749999999999996"],[1793468040,"0.02275"],[1793468055,"0.022749999999999996"],[1793468070,"0.022749999999999996"],[1793468085,"0.022749999999999996"],[1793468100,"0.022749999999999996"],[1793468115,"0.02275"],[1793468130,"0.022749999999999996"],[1793468145,"0.02275"],[1793468160,"0.02275"],[1793468175,"0.02275"],[1793468190,"0.022749999999999996"],[1793468205,"0.02275"],[1793468220,"0.02275"],[1793468235,"0.02275"],[1793468250,"0.02275"],[1793468265,"0.02275"],[1793468280,"0.022749999999999996"],[1793468295,"0.022749999999999996"],[1793468310,"0.022749999999999996"],[1793468325,"0.022749999999999996"],[1793468340,"0.02275"],[1793468355,"0.02275"],[1793468370,"0.022749999999999996"],[1793468385,"0.022749999999999996"],[1793468400,"0.022749999999999996"],[1793468415,"0.02275"],[1793468430,"0.02275"],[1793468445,"0.022749999999999996"],[1793468460,"0.022749999999999996"],[1793468475,"0.02275"],[1793468490,"0.022749999999999996"],[1793468505,"0.022749999999999996"],[1793468520,"0.022750000000000003"],[1793468535,"0.02275"]]}]}}},{"expr":"histogram_quantile(0.5, sum by (le) (rate(sglang:e2e_request_latency_seconds_bucket{job=\"sglang\",instance=\"127.0.0.1:30000\"}[1m])))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"2.75"],[1793467815,"2.75"],[1793467830,"2.75"],[1793467845,"2.75"],[1793467860,"2.75"],[1793467875,"2.75"],[1793467890,"2.75"],[1793467905,"2.75"],[1793467920,"2.75"],[1793467935,"2.75"],[1793467950,"2.75"],[1793467965,"2.75"],[1793467980,"2.75"],[1793467995,"2.75"],[1793468010,"2.75"],[1793468025,"2.75"],[1793468040,"2.75"],[1793468055,"2.75"],[1793468070,"2.75"],[1793468085,"2.75"],[1793468100,"2.75"],[1793468115,"2.75"],[1793468130,"2.75"],[1793468145,"2.75"],[1793468160,"2.75"],[1793468175,"2.75"],[1793468190,"2.75"],[1793468205,"2.75"],[1793468220,"2.75"],[1793468235,"2.75"],[1793468250,"2.75"],[1793468265,"2.75"],[1793468280,"2.75"],[1793468295,"2.75"],[1793468310,"2.75"],[1793468325,"2.75"],[1793468340,"2.75"],[1793468355,"2.75"],[1793468370,"2.75"],[1793468385,"2.75"],[1793468400,"2.75"],[1793468415,"2.75"],[1793468430,"2.75"],[1793468445,"2.75"],[1793468460,"2.75"],[1793468475,"2.75"],[1793468490,"2.75"],[1793468505,"2.75"],[1793468520,"2.75"],[1793468535,"2.75"]]}]}}},{"expr":"histogram_quantile(0.95, sum by (le) (rate(sglang:e2e_request_latency_seconds_bucket{job=\"sglang\",instance=\"127.0.0.1:30000\"}[1m])))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"5.55"],[1793467815,"5.549999999999999"],[1793467830,"5.55"],[1793467845,"5.55"],[1793467860,"5.55"],[1793467875,"5.55"],[1793467890,"5.55"],[1793467905,"5.55"],[1793467920,"5.55"],[1793467935,"5.55"],[1793467950,"5.55"],[1793467965,"5.55"],[1793467980,"5.55"],[1793467995,"5.55"],[1793468010,"5.55"],[1793468025,"5.55"],[1793468040,"5.55"],[1793468055,"5.55"],[1793468070,"5.55"],[1793468085,"5.55"],[1793468100,"5.55"],[1793468115,"5.55"],[1793468130,"5.55"],[1793468145,"5.55"],[1793468160,"5.55"],[1793468175,"5.55"],[1793468190,"5.55"],[1793468205,"5.55"],[1793468220,"5.55"],[1793468235,"5.55"],[1793468250,"5.55"],[1793468265,"5.55"],[1793468280,"5.55"],[1793468295,"5.55"],[1793468310,"5.55"],[1793468325,"5.55"],[1793468340,"5.55"],[1793468355,"5.55"],[1793468370,"5.55"],[1793468385,"5.55"],[1793468400,"5.55"],[1793468415,"5.55"],[1793468430,"5.55"],[1793468445,"5.55"],[1793468460,"5.55"],[1793468475,"5.55"],[1793468490,"5.55"],[1793468505,"5.55"],[1793468520,"5.55"],[1793468535,"5.55"]]}]}}},{"expr":"histogram_quantile(0.99, sum by (le) (rate(sglang:e2e_request_latency_seconds_bucket{job=\"sglang\",instance=\"127.0.0.1:30000\"}[1m])))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"5.91"],[1793467815,"5.91"],[1793467830,"5.91"],[1793467845,"5.91"],[1793467860,"5.91"],[1793467875,"5.91"],[1793467890,"5.91"],[1793467905,"5.91"],[1793467920,"5.91"],[1793467935,"5.91"],[1793467950,"5.91"],[1793467965,"5.91"],[1793467980,"5.91"],[1793467995,"5.91"],[1793468010,"5.91"],[1793468025,"5.91"],[1793468040,"5.91"],[1793468055,"5.91"],[1793468070,"5.91"],[1793468085,"5.91"],[1793468100,"5.91"],[1793468115,"5.91"],[1793468130,"5.91"],[1793468145,"5.91"],[1793468160,"5.91"],[1793468175,"5.91"],[1793468190,"5.91"],[1793468205,"5.91"],[1793468220,"5.91"],[1793468235,"5.91"],[1793468250,"5.91"],[1793468265,"5.91"],[1793468280,"5.91"],[1793468295,"5.91"],[1793468310,"5.91"],[1793468325,"5.91"],[1793468340,"5.91"],[1793468355,"5.91"],[1793468370,"5.91"],[1793468385,"5.91"],[1793468400,"5.91"],[1793468415,"5.91"],[1793468430,"5.91"],[1793468445,"5.91"],[1793468460,"5.91"],[1793468475,"5.91"],[1793468490,"5.91"],[1793468505,"5.91"],[1793468520,"5.91"],[1793468535,"5.91"]]}]}}},{"expr":"histogram_quantile(0.95, sum by (le) (rate(sglang:queue_time_seconds_bucket{job=\"sglang\",instance=\"127.0.0.1:30000\"}[1m])))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0.10999999999999993"],[1793467815,"0.10999999999999993"],[1793467830,"0.10999999999999993"],[1793467845,"0.1100000000000001"],[1793467860,"0.10999999999999985"],[1793467875,"0.10999999999999993"],[1793467890,"0.10999999999999985"],[1793467905,"0.10999999999999985"],[1793467920,"0.11000000000000001"],[1793467935,"0.10999999999999985"],[1793467950,"0.10999999999999993"],[1793467965,"0.11000000000000006"],[1793467980,"0.10999999999999993"],[1793467995,"0.11000000000000006"],[1793468010,"0.11000000000000006"],[1793468025,"0.11000000000000006"],[1793468040,"0.10999999999999993"],[1793468055,"0.11000000000000006"],[1793468070,"0.11000000000000006"],[1793468085,"0.11000000000000006"],[1793468100,"0.11000000000000006"],[1793468115,"0.10999999999999985"],[1793468130,"0.11000000000000006"],[1793468145,"0.10999999999999985"],[1793468160,"0.10999999999999985"],[1793468175,"0.11000000000000001"],[1793468190,"0.11000000000000006"],[1793468205,"0.10999999999999985"],[1793468220,"0.10999999999999985"],[1793468235,"0.10999999999999993"],[1793468250,"0.10999999999999993"],[1793468265,"0.10999999999999993"],[1793468280,"0.11000000000000006"],[1793468295,"0.11000000000000006"],[1793468310,"0.11000000000000006"],[1793468325,"0.11000000000000006"],[1793468340,"0.10999999999999993"],[1793468355,"0.10999999999999985"],[1793468370,"0.11000000000000006"],[1793468385,"0.11000000000000006"],[1793468400,"0.11000000000000006"],[1793468415,"0.10999999999999985"],[1793468430,"0.10999999999999985"],[1793468445,"0.11000000000000006"],[1793468460,"0.11000000000000006"],[1793468475,"0.10999999999999993"],[1793468490,"0.11000000000000006"],[1793468505,"0.11000000000000006"],[1793468520,"0.11"],[1793468535,"0.11000000000000013"]]}]}}},{"expr":"sum by (finished_reason) (rate(sglang:num_requests_total{job=\"sglang\",instance=\"127.0.0.1:30000\"}[1m]))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"3.1737"],[1793467815,"7.673699999999999"],[1793467830,"12.1737"],[1793467845,"16.6737"],[1793467860,"17.999345478346243"],[1793467875,"18"],[1793467890,"17.999345478346243"],[1793467905,"17.999345478346243"],[1793467920,"17.999018235368982"],[1793467935,"17.999345478346243"],[1793467950,"18"],[1793467965,"17.999672733223033"],[1793467980,"18"],[1793467995,"17.999672733223033"],[1793468010,"17.999672733223033"],[1793468025,"17.999672733223033"],[1793468040,"18"],[1793468055,"17.999672733223033"],[1793468070,"17.999672733223033"],[1793468085,"17.999672733223033"],[1793468100,"17.999672733223033"],[1793468115,"17.999345478346243"],[1793468130,"17.999672733223033"],[1793468145,"17.999345478346243"],[1793468160,"17.999345478346243"],[1793468175,"17.999018235368982"],[1793468190,"17.999672733223033"],[1793468205,"17.999345478346243"],[1793468220,"17.999345478346243"],[1793468235,"18"],[1793468250,"18"],[1793468265,"18"],[1793468280,"17.999672733223033"],[1793468295,"17.999672733223033"],[1793468310,"17.999672733223033"],[1793468325,"17.999672733223033"],[1793468340,"18"],[1793468355,"17.999345478346243"],[1793468370,"17.999672733223033"],[1793468385,"17.999672733223033"],[1793468400,"17.999672733223033"],[1793468415,"17.999345478346243"],[1793468430,"17.999345478346243"],[1793468445,"17.999672733223033"],[1793468460,"17.999672733223033"],[1793468475,"18"],[1793468490,"17.999672733223033"],[1793468505,"17.999672733223033"],[1793468520,"18.00098187173846"],[1793468535,"18.00163651241022"]]}]}}}]}
````

<!-- block: file t04-src-tauri-src-history-testdata-prometheus-3-15-1-range-vllm-8000-json step=4.3 path=src-tauri/src/history/testdata/prometheus-3.15.1/range-vllm-8000.json -->
````
{"start":1793467635,"end":1793468535,"step":15,"rate":"1m","queries":[{"expr":"sum(vllm:num_requests_running{job=\"vllm\",instance=\"127.0.0.1:8000\"})","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0"],[1793467815,"0"],[1793467830,"0"],[1793467845,"0"],[1793467860,"0"],[1793467875,"0"],[1793467890,"0"],[1793467905,"0"],[1793467920,"0"],[1793467935,"0"],[1793467950,"0"],[1793467965,"0"],[1793467980,"0"],[1793467995,"0"],[1793468010,"0"],[1793468025,"0"],[1793468040,"0"],[1793468055,"0"],[1793468070,"0"],[1793468085,"0"],[1793468100,"0"],[1793468115,"0"],[1793468130,"0"],[1793468145,"0"],[1793468160,"0"],[1793468175,"0"],[1793468190,"0"],[1793468205,"0"],[1793468220,"0"],[1793468235,"0"],[1793468250,"0"],[1793468265,"0"],[1793468280,"0"],[1793468295,"0"],[1793468310,"0"],[1793468325,"0"],[1793468340,"0"],[1793468355,"0"],[1793468370,"0"],[1793468385,"0"],[1793468400,"0"],[1793468415,"0"],[1793468430,"0"],[1793468445,"0"],[1793468460,"0"],[1793468475,"0"],[1793468490,"0"],[1793468505,"0"],[1793468520,"0"],[1793468535,"0"]]}]}}},{"expr":"sum(vllm:num_requests_waiting{job=\"vllm\",instance=\"127.0.0.1:8000\"})","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0"],[1793467815,"0"],[1793467830,"0"],[1793467845,"0"],[1793467860,"0"],[1793467875,"0"],[1793467890,"0"],[1793467905,"0"],[1793467920,"0"],[1793467935,"0"],[1793467950,"0"],[1793467965,"0"],[1793467980,"0"],[1793467995,"0"],[1793468010,"0"],[1793468025,"0"],[1793468040,"0"],[1793468055,"0"],[1793468070,"0"],[1793468085,"0"],[1793468100,"0"],[1793468115,"0"],[1793468130,"0"],[1793468145,"0"],[1793468160,"0"],[1793468175,"0"],[1793468190,"0"],[1793468205,"0"],[1793468220,"0"],[1793468235,"0"],[1793468250,"0"],[1793468265,"0"],[1793468280,"0"],[1793468295,"0"],[1793468310,"0"],[1793468325,"0"],[1793468340,"0"],[1793468355,"0"],[1793468370,"0"],[1793468385,"0"],[1793468400,"0"],[1793468415,"0"],[1793468430,"0"],[1793468445,"0"],[1793468460,"0"],[1793468475,"0"],[1793468490,"0"],[1793468505,"0"],[1793468520,"0"],[1793468535,"0"]]}]}}},{"expr":"max(vllm:kv_cache_usage_perc{job=\"vllm\",instance=\"127.0.0.1:8000\"}) or max(vllm:gpu_cache_usage_perc{job=\"vllm\",instance=\"127.0.0.1:8000\"})","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0.3963"],[1793467815,"0.7832"],[1793467830,"0.6339"],[1793467845,"0.3143"],[1793467860,"0.7574"],[1793467875,"0.688"],[1793467890,"0.2301"],[1793467905,"0.7205"],[1793467920,"0.7324"],[1793467935,"0.2548"],[1793467950,"0.6732"],[1793467965,"0.7661"],[1793467980,"0.3386"],[1793467995,"0.6164"],[1793468010,"0.7885"],[1793468025,"0.4196"],[1793468040,"0.5512"],[1793468055,"0.7991"],[1793468070,"0.4962"],[1793468085,"0.4791"],[1793468100,"0.7977"],[1793468115,"0.5668"],[1793468130,"0.4013"],[1793468145,"0.7844"],[1793468160,"0.6302"],[1793468175,"0.3195"],[1793468190,"0.7594"],[1793468205,"0.6849"],[1793468220,"0.2354"],[1793468235,"0.7231"],[1793468250,"0.7299"],[1793468265,"0.2495"],[1793468280,"0.6764"],[1793468295,"0.7643"],[1793468310,"0.3334"],[1793468325,"0.6202"],[1793468340,"0.7874"],[1793468355,"0.4146"],[1793468370,"0.5555"],[1793468385,"0.7988"],[1793468400,"0.4915"],[1793468415,"0.4838"],[1793468430,"0.7982"],[1793468445,"0.5626"],[1793468460,"0.4063"],[1793468475,"0.7856"],[1793468490,"0.6264"],[1793468505,"0.3247"],[1793468520,"0.7613"],[1793468535,"0.6817"]]}]}}},{"expr":"avg(vllm:kv_cache_usage_perc{job=\"vllm\",instance=\"127.0.0.1:8000\"}) or avg(vllm:gpu_cache_usage_perc{job=\"vllm\",instance=\"127.0.0.1:8000\"})","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0.3963"],[1793467815,"0.7832"],[1793467830,"0.6339"],[1793467845,"0.3143"],[1793467860,"0.7574"],[1793467875,"0.688"],[1793467890,"0.2301"],[1793467905,"0.7205"],[1793467920,"0.7324"],[1793467935,"0.2548"],[1793467950,"0.6732"],[1793467965,"0.7661"],[1793467980,"0.3386"],[1793467995,"0.6164"],[1793468010,"0.7885"],[1793468025,"0.4196"],[1793468040,"0.5512"],[1793468055,"0.7991"],[1793468070,"0.4962"],[1793468085,"0.4791"],[1793468100,"0.7977"],[1793468115,"0.5668"],[1793468130,"0.4013"],[1793468145,"0.7844"],[1793468160,"0.6302"],[1793468175,"0.3195"],[1793468190,"0.7594"],[1793468205,"0.6849"],[1793468220,"0.2354"],[1793468235,"0.7231"],[1793468250,"0.7299"],[1793468265,"0.2495"],[1793468280,"0.6764"],[1793468295,"0.7643"],[1793468310,"0.3334"],[1793468325,"0.6202"],[1793468340,"0.7874"],[1793468355,"0.4146"],[1793468370,"0.5555"],[1793468385,"0.7988"],[1793468400,"0.4915"],[1793468415,"0.4838"],[1793468430,"0.7982"],[1793468445,"0.5626"],[1793468460,"0.4063"],[1793468475,"0.7856"],[1793468490,"0.6264"],[1793468505,"0.3247"],[1793468520,"0.7613"],[1793468535,"0.6817"]]}]}}},{"expr":"sum(rate(vllm:prompt_tokens_total{job=\"vllm\",instance=\"127.0.0.1:8000\"}[1m]))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"3.7056129999999974"],[1793467815,"8.910613000000001"],[1793467830,"14.115613000000002"],[1793467845,"19.320612999999998"],[1793467860,"20.82"],[1793467875,"20.819999999999997"],[1793467890,"20.82"],[1793467905,"20.819242936620487"],[1793467920,"20.81962146142797"],[1793467935,"20.818864425576795"],[1793467950,"20.818864425576795"],[1793467965,"20.819621461427964"],[1793467980,"20.819999999999997"],[1793467995,"20.81962146142797"],[1793468010,"20.81962146142798"],[1793468025,"20.81962146142798"],[1793468040,"20.81999999999999"],[1793468055,"20.819621461427964"],[1793468070,"20.81962146142798"],[1793468085,"20.820000000000004"],[1793468100,"20.81962146142798"],[1793468115,"20.81999999999999"],[1793468130,"20.81999999999999"],[1793468145,"20.820000000000004"],[1793468160,"20.819621461427964"],[1793468175,"20.81962146142798"],[1793468190,"20.819621461427964"],[1793468205,"20.81962146142798"],[1793468220,"20.81962146142798"],[1793468235,"20.82151429194848"],[1793468250,"20.820000000000004"],[1793468265,"20.81962146142798"],[1793468280,"20.820000000000004"],[1793468295,"20.81962146142798"],[1793468310,"20.819999999999972"],[1793468325,"20.820000000000004"],[1793468340,"20.820000000000004"],[1793468355,"20.820000000000004"],[1793468370,"20.81962146142798"],[1793468385,"20.819999999999972"],[1793468400,"20.820000000000004"],[1793468415,"20.819242936620494"],[1793468430,"20.81962146142798"],[1793468445,"20.820000000000004"],[1793468460,"20.819999999999972"],[1793468475,"20.820000000000004"],[1793468490,"20.820000000000004"],[1793468505,"20.81962146142798"],[1793468520,"20.821135698310822"],[1793468535,"20.822271520529487"]]}]}}},{"expr":"sum(rate(vllm:generation_tokens_total{job=\"vllm\",instance=\"127.0.0.1:8000\"}[1m]))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"1.797631666666667"],[1793467815,"4.322631666666667"],[1793467830,"6.847631666666667"],[1793467845,"9.372631666666665"],[1793467860,"10.1"],[1793467875,"10.1"],[1793467890,"10.1"],[1793467905,"10.099632740627614"],[1793467920,"10.099816366975146"],[1793467935,"10.09944912095704"],[1793467950,"10.09944912095704"],[1793467965,"10.099816366975146"],[1793467980,"10.1"],[1793467995,"10.099816366975146"],[1793468010,"10.099816366975146"],[1793468025,"10.099816366975146"],[1793468040,"10.1"],[1793468055,"10.099816366975146"],[1793468070,"10.099816366975146"],[1793468085,"10.1"],[1793468100,"10.099816366975146"],[1793468115,"10.1"],[1793468130,"10.1"],[1793468145,"10.1"],[1793468160,"10.099816366975146"],[1793468175,"10.099816366975146"],[1793468190,"10.099816366975146"],[1793468205,"10.099816366975146"],[1793468220,"10.099816366975146"],[1793468235,"10.100734598879919"],[1793468250,"10.1"],[1793468265,"10.099816366975146"],[1793468280,"10.1"],[1793468295,"10.099816366975146"],[1793468310,"10.1"],[1793468325,"10.1"],[1793468340,"10.1"],[1793468355,"10.1"],[1793468370,"10.099816366975146"],[1793468385,"10.1"],[1793468400,"10.1"],[1793468415,"10.099632740627614"],[1793468430,"10.099816366975146"],[1793468445,"10.1"],[1793468460,"10.1"],[1793468475,"10.1"],[1793468490,"10.1"],[1793468505,"10.099816366975146"],[1793468520,"10.100550939142135"],[1793468535,"10.10110193839328"]]}]}}},{"expr":"sum(rate(vllm:num_preemptions_total{job=\"vllm\",instance=\"127.0.0.1:8000\"}[1m]))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0"],[1793467815,"0"],[1793467830,"0"],[1793467845,"0"],[1793467860,"0"],[1793467875,"0"],[1793467890,"0"],[1793467905,"0"],[1793467920,"0"],[1793467935,"0"],[1793467950,"0"],[1793467965,"0"],[1793467980,"0"],[1793467995,"0"],[1793468010,"0"],[1793468025,"0"],[1793468040,"0"],[1793468055,"0"],[1793468070,"0"],[1793468085,"0"],[1793468100,"0"],[1793468115,"0"],[1793468130,"0"],[1793468145,"0"],[1793468160,"0"],[1793468175,"0"],[1793468190,"0"],[1793468205,"0"],[1793468220,"0"],[1793468235,"0"],[1793468250,"0"],[1793468265,"0"],[1793468280,"0"],[1793468295,"0"],[1793468310,"0"],[1793468325,"0"],[1793468340,"0"],[1793468355,"0"],[1793468370,"0"],[1793468385,"0"],[1793468400,"0"],[1793468415,"0"],[1793468430,"0"],[1793468445,"0"],[1793468460,"0"],[1793468475,"0"],[1793468490,"0"],[1793468505,"0"],[1793468520,"0"],[1793468535,"0"]]}]}}},{"expr":"sum(rate(vllm:request_success_total{job=\"vllm\",instance=\"127.0.0.1:8000\",finished_reason=\"abort\"}[1m]))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0"],[1793467815,"0"],[1793467830,"0"],[1793467845,"0"],[1793467860,"0"],[1793467875,"0"],[1793467890,"0"],[1793467905,"0"],[1793467920,"0"],[1793467935,"0"],[1793467950,"0"],[1793467965,"0"],[1793467980,"0"],[1793467995,"0"],[1793468010,"0"],[1793468025,"0"],[1793468040,"0"],[1793468055,"0"],[1793468070,"0"],[1793468085,"0"],[1793468100,"0"],[1793468115,"0"],[1793468130,"0"],[1793468145,"0"],[1793468160,"0"],[1793468175,"0"],[1793468190,"0"],[1793468205,"0"],[1793468220,"0"],[1793468235,"0"],[1793468250,"0"],[1793468265,"0"],[1793468280,"0"],[1793468295,"0"],[1793468310,"0"],[1793468325,"0"],[1793468340,"0"],[1793468355,"0"],[1793468370,"0"],[1793468385,"0"],[1793468400,"0"],[1793468415,"0"],[1793468430,"0"],[1793468445,"0"],[1793468460,"0"],[1793468475,"0"],[1793468490,"0"],[1793468505,"0"],[1793468520,"0"],[1793468535,"0"]]}]}}},{"expr":"(sum(rate(vllm:prefix_cache_hits_total{job=\"vllm\",instance=\"127.0.0.1:8000\"}[1m])) or sum(rate(vllm:gpu_prefix_cache_hits_total{job=\"vllm\",instance=\"127.0.0.1:8000\"}[1m]))) / (sum(rate(vllm:prefix_cache_queries_total{job=\"vllm\",instance=\"127.0.0.1:8000\"}[1m])) or sum(rate(vllm:gpu_prefix_cache_queries_total{job=\"vllm\",instance=\"127.0.0.1:8000\"}[1m])))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0.4457252641690684"],[1793467815,"0.44572526416906816"],[1793467830,"0.4457252641690681"],[1793467845,"0.44572526416906816"],[1793467860,"0.44572526416906827"],[1793467875,"0.4457252641690683"],[1793467890,"0.4457252641690681"],[1793467905,"0.4457252641690682"],[1793467920,"0.4457252641690683"],[1793467935,"0.4457252641690682"],[1793467950,"0.4457252641690682"],[1793467965,"0.4457252641690683"],[1793467980,"0.4457252641690683"],[1793467995,"0.4457252641690683"],[1793468010,"0.44572526416906794"],[1793468025,"0.4457252641690681"],[1793468040,"0.4457252641690685"],[1793468055,"0.4457252641690685"],[1793468070,"0.4457252641690681"],[1793468085,"0.44572526416906777"],[1793468100,"0.4457252641690681"],[1793468115,"0.4457252641690685"],[1793468130,"0.4457252641690685"],[1793468145,"0.44572526416906816"],[1793468160,"0.4457252641690681"],[1793468175,"0.4457252641690681"],[1793468190,"0.4457252641690689"],[1793468205,"0.4457252641690681"],[1793468220,"0.4457252641690681"],[1793468235,"0.44572526416906844"],[1793468250,"0.44572526416906777"],[1793468265,"0.4457252641690685"],[1793468280,"0.4457252641690686"],[1793468295,"0.44572526416906777"],[1793468310,"0.44572526416906844"],[1793468325,"0.44572526416906777"],[1793468340,"0.4457252641690686"],[1793468355,"0.4457252641690686"],[1793468370,"0.44572526416906777"],[1793468385,"0.44572526416906844"],[1793468400,"0.44572526416906777"],[1793468415,"0.4457252641690685"],[1793468430,"0.4457252641690685"],[1793468445,"0.44572526416906777"],[1793468460,"0.44572526416906844"],[1793468475,"0.44572526416906777"],[1793468490,"0.4457252641690686"],[1793468505,"0.4457252641690685"],[1793468520,"0.4457252641690677"],[1793468535,"0.44572526416906844"]]}]}}},{"expr":"histogram_quantile(0.5, sum by (le) (rate(vllm:time_to_first_token_seconds_bucket{job=\"vllm\",instance=\"127.0.0.1:8000\"}[1m])))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0.06500000000000002"],[1793467815,"0.065"],[1793467830,"0.065"],[1793467845,"0.065"],[1793467860,"0.065"],[1793467875,"0.065"],[1793467890,"0.065"],[1793467905,"0.065"],[1793467920,"0.065"],[1793467935,"0.06499999999999999"],[1793467950,"0.06499999999999999"],[1793467965,"0.065"],[1793467980,"0.065"],[1793467995,"0.06499999999999999"],[1793468010,"0.06499999999999999"],[1793468025,"0.06499999999999999"],[1793468040,"0.065"],[1793468055,"0.06499999999999999"],[1793468070,"0.06499999999999999"],[1793468085,"0.06500000000000002"],[1793468100,"0.06499999999999999"],[1793468115,"0.06499999999999999"],[1793468130,"0.06499999999999999"],[1793468145,"0.06499999999999999"],[1793468160,"0.06500000000000002"],[1793468175,"0.06499999999999999"],[1793468190,"0.06499999999999999"],[1793468205,"0.06499999999999999"],[1793468220,"0.06499999999999999"],[1793468235,"0.06500000000000002"],[1793468250,"0.06499999999999999"],[1793468265,"0.06499999999999999"],[1793468280,"0.06499999999999999"],[1793468295,"0.06499999999999999"],[1793468310,"0.06500000000000002"],[1793468325,"0.06500000000000002"],[1793468340,"0.06499999999999996"],[1793468355,"0.06499999999999996"],[1793468370,"0.06500000000000002"],[1793468385,"0.06500000000000002"],[1793468400,"0.06500000000000002"],[1793468415,"0.06499999999999996"],[1793468430,"0.06499999999999996"],[1793468445,"0.06500000000000002"],[1793468460,"0.06500000000000002"],[1793468475,"0.06500000000000002"],[1793468490,"0.06499999999999996"],[1793468505,"0.06499999999999996"],[1793468520,"0.06500000000000002"],[1793468535,"0.06500000000000002"]]}]}}},{"expr":"histogram_quantile(0.95, sum by (le) (rate(vllm:time_to_first_token_seconds_bucket{job=\"vllm\",instance=\"127.0.0.1:8000\"}[1m])))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0.23500000000000043"],[1793467815,"0.23499999999999993"],[1793467830,"0.23500000000000001"],[1793467845,"0.2350000000000001"],[1793467860,"0.23499999999999982"],[1793467875,"0.23499999999999985"],[1793467890,"0.23500000000000015"],[1793467905,"0.23499999999999996"],[1793467920,"0.23499999999999982"],[1793467935,"0.2349999999999997"],[1793467950,"0.2349999999999997"],[1793467965,"0.23500000000000013"],[1793467980,"0.23499999999999985"],[1793467995,"0.2349999999999998"],[1793468010,"0.2350000000000001"],[1793468025,"0.2349999999999998"],[1793468040,"0.23499999999999985"],[1793468055,"0.2349999999999998"],[1793468070,"0.2349999999999998"],[1793468085,"0.23500000000000065"],[1793468100,"0.2349999999999998"],[1793468115,"0.23499999999999982"],[1793468130,"0.23499999999999982"],[1793468145,"0.23499999999999982"],[1793468160,"0.2350000000000005"],[1793468175,"0.2349999999999998"],[1793468190,"0.23499999999999904"],[1793468205,"0.2349999999999998"],[1793468220,"0.2349999999999998"],[1793468235,"0.23500000000000063"],[1793468250,"0.23500000000000057"],[1793468265,"0.23499999999999904"],[1793468280,"0.23499999999999907"],[1793468295,"0.2350000000000004"],[1793468310,"0.23500000000000065"],[1793468325,"0.23500000000000065"],[1793468340,"0.234999999999999"],[1793468355,"0.234999999999999"],[1793468370,"0.2350000000000005"],[1793468385,"0.23500000000000065"],[1793468400,"0.23500000000000065"],[1793468415,"0.23499999999999913"],[1793468430,"0.23499999999999896"],[1793468445,"0.23500000000000065"],[1793468460,"0.23500000000000065"],[1793468475,"0.23500000000000065"],[1793468490,"0.234999999999999"],[1793468505,"0.23499999999999896"],[1793468520,"0.2350000000000006"],[1793468535,"0.23500000000000057"]]}]}}},{"expr":"histogram_quantile(0.99, sum by (le) (rate(vllm:time_to_first_token_seconds_bucket{job=\"vllm\",instance=\"127.0.0.1:8000\"}[1m])))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"2.050000000000007"],[1793467815,"2.0499999999999994"],[1793467830,"2.0500000000000007"],[1793467845,"2.0500000000000016"],[1793467860,"2.0499999999999976"],[1793467875,"2.0499999999999976"],[1793467890,"2.0500000000000025"],[1793467905,"2.0499999999999994"],[1793467920,"2.049999999999997"],[1793467935,"2.0499999999999954"],[1793467950,"2.0499999999999954"],[1793467965,"2.0500000000000025"],[1793467980,"2.0499999999999976"],[1793467995,"2.049999999999997"],[1793468010,"2.0500000000000025"],[1793468025,"2.049999999999997"],[1793468040,"2.0499999999999976"],[1793468055,"2.049999999999997"],[1793468070,"2.049999999999997"],[1793468085,"2.05000000000001"],[1793468100,"2.049999999999997"],[1793468115,"2.0499999999999976"],[1793468130,"2.0499999999999976"],[1793468145,"2.0499999999999976"],[1793468160,"2.0500000000000074"],[1793468175,"2.049999999999997"],[1793468190,"2.0499999999999847"],[1793468205,"2.049999999999997"],[1793468220,"2.049999999999997"],[1793468235,"2.0500000000000096"],[1793468250,"2.05000000000001"],[1793468265,"2.0499999999999847"],[1793468280,"2.0499999999999847"],[1793468295,"2.0500000000000074"],[1793468310,"2.05000000000001"],[1793468325,"2.05000000000001"],[1793468340,"2.0499999999999847"],[1793468355,"2.0499999999999847"],[1793468370,"2.0500000000000074"],[1793468385,"2.05000000000001"],[1793468400,"2.05000000000001"],[1793468415,"2.049999999999987"],[1793468430,"2.0499999999999847"],[1793468445,"2.05000000000001"],[1793468460,"2.05000000000001"],[1793468475,"2.05000000000001"],[1793468490,"2.0499999999999847"],[1793468505,"2.0499999999999847"],[1793468520,"2.0500000000000087"],[1793468535,"2.0500000000000087"]]}]}}},{"expr":"histogram_quantile(0.5, sum by (le) (rate(vllm:inter_token_latency_seconds_bucket{job=\"vllm\",instance=\"127.0.0.1:8000\"}[1m])) or sum by (le) (rate(vllm:time_per_output_token_seconds_bucket{job=\"vllm\",instance=\"127.0.0.1:8000\"}[1m])))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0.01159883720930233"],[1793467815,"0.011598837209302324"],[1793467830,"0.011598837209302326"],[1793467845,"0.011598837209302324"],[1793467860,"0.011598837209302324"],[1793467875,"0.011598837209302326"],[1793467890,"0.011598837209302324"],[1793467905,"0.011598837209302326"],[1793467920,"0.011598837209302326"],[1793467935,"0.011598837209302326"],[1793467950,"0.011598837209302328"],[1793467965,"0.011598837209302328"],[1793467980,"0.011598837209302326"],[1793467995,"0.011598837209302326"],[1793468010,"0.011598837209302324"],[1793468025,"0.011598837209302324"],[1793468040,"0.011598837209302328"],[1793468055,"0.011598837209302328"],[1793468070,"0.011598837209302324"],[1793468085,"0.011598837209302324"],[1793468100,"0.011598837209302324"],[1793468115,"0.011598837209302328"],[1793468130,"0.011598837209302328"],[1793468145,"0.011598837209302324"],[1793468160,"0.011598837209302324"],[1793468175,"0.011598837209302324"],[1793468190,"0.011598837209302328"],[1793468205,"0.011598837209302324"],[1793468220,"0.011598837209302324"],[1793468235,"0.011598837209302326"],[1793468250,"0.011598837209302324"],[1793468265,"0.011598837209302324"],[1793468280,"0.011598837209302324"],[1793468295,"0.011598837209302324"],[1793468310,"0.01159883720930233"],[1793468325,"0.011598837209302324"],[1793468340,"0.011598837209302324"],[1793468355,"0.011598837209302324"],[1793468370,"0.011598837209302324"],[1793468385,"0.01159883720930233"],[1793468400,"0.011598837209302324"],[1793468415,"0.011598837209302324"],[1793468430,"0.011598837209302324"],[1793468445,"0.011598837209302324"],[1793468460,"0.01159883720930233"],[1793468475,"0.011598837209302324"],[1793468490,"0.011598837209302324"],[1793468505,"0.011598837209302324"],[1793468520,"0.011598837209302323"],[1793468535,"0.011598837209302331"]]}]}}},{"expr":"histogram_quantile(0.95, sum by (le) (rate(vllm:inter_token_latency_seconds_bucket{job=\"vllm\",instance=\"127.0.0.1:8000\"}[1m])) or sum by (le) (rate(vllm:time_per_output_token_seconds_bucket{job=\"vllm\",instance=\"127.0.0.1:8000\"}[1m])))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0.024026162790697702"],[1793467815,"0.024026162790697667"],[1793467830,"0.024026162790697674"],[1793467845,"0.02402616279069767"],[1793467860,"0.02402616279069767"],[1793467875,"0.024026162790697674"],[1793467890,"0.02402616279069767"],[1793467905,"0.024026162790697674"],[1793467920,"0.024026162790697674"],[1793467935,"0.024026162790697667"],[1793467950,"0.024026162790697678"],[1793467965,"0.024026162790697688"],[1793467980,"0.024026162790697674"],[1793467995,"0.024026162790697674"],[1793468010,"0.024026162790697664"],[1793468025,"0.024026162790697664"],[1793468040,"0.02402616279069769"],[1793468055,"0.024026162790697688"],[1793468070,"0.024026162790697664"],[1793468085,"0.024026162790697667"],[1793468100,"0.024026162790697664"],[1793468115,"0.02402616279069769"],[1793468130,"0.02402616279069769"],[1793468145,"0.024026162790697667"],[1793468160,"0.024026162790697664"],[1793468175,"0.024026162790697664"],[1793468190,"0.024026162790697688"],[1793468205,"0.024026162790697664"],[1793468220,"0.024026162790697664"],[1793468235,"0.024026162790697688"],[1793468250,"0.024026162790697667"],[1793468265,"0.024026162790697664"],[1793468280,"0.024026162790697667"],[1793468295,"0.024026162790697664"],[1793468310,"0.02402616279069771"],[1793468325,"0.024026162790697667"],[1793468340,"0.024026162790697667"],[1793468355,"0.024026162790697667"],[1793468370,"0.024026162790697664"],[1793468385,"0.02402616279069771"],[1793468400,"0.024026162790697667"],[1793468415,"0.024026162790697667"],[1793468430,"0.024026162790697664"],[1793468445,"0.024026162790697667"],[1793468460,"0.02402616279069771"],[1793468475,"0.024026162790697667"],[1793468490,"0.024026162790697667"],[1793468505,"0.024026162790697664"],[1793468520,"0.024026162790697667"],[1793468535,"0.02402616279069771"]]}]}}},{"expr":"histogram_quantile(0.99, sum by (le) (rate(vllm:inter_token_latency_seconds_bucket{job=\"vllm\",instance=\"127.0.0.1:8000\"}[1m])) or sum by (le) (rate(vllm:time_per_output_token_seconds_bucket{job=\"vllm\",instance=\"127.0.0.1:8000\"}[1m])))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0.033035714285715445"],[1793467815,"0.03303571428571392"],[1793467830,"0.03303571428571424"],[1793467845,"0.03303571428571415"],[1793467860,"0.03303571428571413"],[1793467875,"0.03303571428571434"],[1793467890,"0.03303571428571413"],[1793467905,"0.033035714285714314"],[1793467920,"0.03303571428571421"],[1793467935,"0.03303571428571396"],[1793467950,"0.03303571428571439"],[1793467965,"0.033035714285714855"],[1793467980,"0.03303571428571434"],[1793467995,"0.03303571428571421"],[1793468010,"0.03303571428571378"],[1793468025,"0.03303571428571378"],[1793468040,"0.03303571428571499"],[1793468055,"0.033035714285714855"],[1793468070,"0.03303571428571378"],[1793468085,"0.03303571428571391"],[1793468100,"0.03303571428571378"],[1793468115,"0.03303571428571499"],[1793468130,"0.03303571428571499"],[1793468145,"0.03303571428571391"],[1793468160,"0.03303571428571378"],[1793468175,"0.03303571428571378"],[1793468190,"0.033035714285714855"],[1793468205,"0.03303571428571378"],[1793468220,"0.03303571428571378"],[1793468235,"0.03303571428571485"],[1793468250,"0.03303571428571391"],[1793468265,"0.03303571428571378"],[1793468280,"0.03303571428571391"],[1793468295,"0.03303571428571378"],[1793468310,"0.03303571428571585"],[1793468325,"0.03303571428571391"],[1793468340,"0.03303571428571391"],[1793468355,"0.03303571428571391"],[1793468370,"0.03303571428571378"],[1793468385,"0.03303571428571585"],[1793468400,"0.03303571428571391"],[1793468415,"0.033035714285713884"],[1793468430,"0.03303571428571378"],[1793468445,"0.03303571428571391"],[1793468460,"0.03303571428571585"],[1793468475,"0.03303571428571391"],[1793468490,"0.03303571428571391"],[1793468505,"0.03303571428571378"],[1793468520,"0.033035714285713884"],[1793468535,"0.03303571428571587"]]}]}}},{"expr":"histogram_quantile(0.5, sum by (le) (rate(vllm:e2e_request_latency_seconds_bucket{job=\"vllm\",instance=\"127.0.0.1:8000\"}[1m])))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0.2647058823529413"],[1793467815,"0.2647058823529412"],[1793467830,"0.2647058823529412"],[1793467845,"0.26470588235294124"],[1793467860,"0.2647058823529411"],[1793467875,"0.2647058823529411"],[1793467890,"0.26470588235294124"],[1793467905,"0.26470588235294124"],[1793467920,"0.2647058823529411"],[1793467935,"0.26470588235294124"],[1793467950,"0.26470588235294107"],[1793467965,"0.2647058823529412"],[1793467980,"0.2647058823529411"],[1793467995,"0.2647058823529411"],[1793468010,"0.26470588235294135"],[1793468025,"0.2647058823529411"],[1793468040,"0.2647058823529411"],[1793468055,"0.2647058823529411"],[1793468070,"0.2647058823529411"],[1793468085,"0.26470588235294135"],[1793468100,"0.2647058823529411"],[1793468115,"0.2647058823529411"],[1793468130,"0.2647058823529411"],[1793468145,"0.2647058823529411"],[1793468160,"0.26470588235294135"],[1793468175,"0.26470588235294135"],[1793468190,"0.2647058823529409"],[1793468205,"0.2647058823529409"],[1793468220,"0.26470588235294135"],[1793468235,"0.26470588235294135"],[1793468250,"0.26470588235294135"],[1793468265,"0.2647058823529409"],[1793468280,"0.2647058823529409"],[1793468295,"0.26470588235294135"],[1793468310,"0.26470588235294135"],[1793468325,"0.26470588235294135"],[1793468340,"0.2647058823529409"],[1793468355,"0.2647058823529409"],[1793468370,"0.26470588235294135"],[1793468385,"0.26470588235294135"],[1793468400,"0.26470588235294135"],[1793468415,"0.2647058823529409"],[1793468430,"0.2647058823529409"],[1793468445,"0.26470588235294135"],[1793468460,"0.26470588235294135"],[1793468475,"0.26470588235294135"],[1793468490,"0.2647058823529409"],[1793468505,"0.26470588235294135"],[1793468520,"0.2647058823529413"],[1793468535,"0.2647058823529409"]]}]}}},{"expr":"histogram_quantile(0.95, sum by (le) (rate(vllm:e2e_request_latency_seconds_bucket{job=\"vllm\",instance=\"127.0.0.1:8000\"}[1m])))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0.725000000000002"],[1793467815,"0.725"],[1793467830,"0.7250000000000001"],[1793467845,"0.7250000000000004"],[1793467860,"0.7249999999999992"],[1793467875,"0.7249999999999992"],[1793467890,"0.7250000000000006"],[1793467905,"0.7249999999999998"],[1793467920,"0.7249999999999991"],[1793467935,"0.7249999999999991"],[1793467950,"0.7249999999999986"],[1793467965,"0.7250000000000003"],[1793467980,"0.7249999999999992"],[1793467995,"0.7249999999999991"],[1793468010,"0.7250000000000008"],[1793468025,"0.7249999999999995"],[1793468040,"0.7249999999999988"],[1793468055,"0.7249999999999986"],[1793468070,"0.7249999999999995"],[1793468085,"0.7250000000000028"],[1793468100,"0.7249999999999995"],[1793468115,"0.7249999999999988"],[1793468130,"0.7249999999999988"],[1793468145,"0.7249999999999996"],[1793468160,"0.725000000000002"],[1793468175,"0.7249999999999995"],[1793468190,"0.7249999999999955"],[1793468205,"0.7249999999999986"],[1793468220,"0.7249999999999995"],[1793468235,"0.7250000000000018"],[1793468250,"0.7250000000000028"],[1793468265,"0.7249999999999955"],[1793468280,"0.7249999999999965"],[1793468295,"0.725000000000002"],[1793468310,"0.7250000000000009"],[1793468325,"0.7250000000000028"],[1793468340,"0.7249999999999965"],[1793468355,"0.7249999999999965"],[1793468370,"0.725000000000002"],[1793468385,"0.7250000000000009"],[1793468400,"0.7250000000000028"],[1793468415,"0.7249999999999971"],[1793468430,"0.7249999999999964"],[1793468445,"0.7250000000000028"],[1793468460,"0.7250000000000009"],[1793468475,"0.7250000000000028"],[1793468490,"0.7249999999999965"],[1793468505,"0.7249999999999964"],[1793468520,"0.7250000000000024"],[1793468535,"0.7250000000000005"]]}]}}},{"expr":"histogram_quantile(0.99, sum by (le) (rate(vllm:e2e_request_latency_seconds_bucket{job=\"vllm\",instance=\"127.0.0.1:8000\"}[1m])))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"1.3500000000000023"],[1793467815,"1.3499999999999996"],[1793467830,"1.35"],[1793467845,"1.3500000000000005"],[1793467860,"1.3499999999999992"],[1793467875,"1.3499999999999992"],[1793467890,"1.3500000000000008"],[1793467905,"1.3499999999999999"],[1793467920,"1.3499999999999992"],[1793467935,"1.3499999999999985"],[1793467950,"1.3499999999999985"],[1793467965,"1.3500000000000008"],[1793467980,"1.3499999999999992"],[1793467995,"1.3499999999999992"],[1793468010,"1.3500000000000008"],[1793468025,"1.3499999999999992"],[1793468040,"1.3499999999999992"],[1793468055,"1.3499999999999992"],[1793468070,"1.3499999999999992"],[1793468085,"1.3500000000000032"],[1793468100,"1.3499999999999992"],[1793468115,"1.3499999999999992"],[1793468130,"1.3499999999999992"],[1793468145,"1.3499999999999992"],[1793468160,"1.3500000000000023"],[1793468175,"1.3499999999999992"],[1793468190,"1.349999999999995"],[1793468205,"1.3499999999999992"],[1793468220,"1.3499999999999992"],[1793468235,"1.3500000000000032"],[1793468250,"1.3500000000000032"],[1793468265,"1.349999999999995"],[1793468280,"1.349999999999995"],[1793468295,"1.3500000000000023"],[1793468310,"1.3500000000000032"],[1793468325,"1.3500000000000032"],[1793468340,"1.349999999999995"],[1793468355,"1.349999999999995"],[1793468370,"1.3500000000000023"],[1793468385,"1.3500000000000032"],[1793468400,"1.3500000000000032"],[1793468415,"1.3499999999999956"],[1793468430,"1.349999999999995"],[1793468445,"1.3500000000000032"],[1793468460,"1.3500000000000032"],[1793468475,"1.3500000000000032"],[1793468490,"1.349999999999995"],[1793468505,"1.349999999999995"],[1793468520,"1.350000000000003"],[1793468535,"1.350000000000003"]]}]}}},{"expr":"histogram_quantile(0.95, sum by (le) (rate(vllm:request_queue_time_seconds_bucket{job=\"vllm\",instance=\"127.0.0.1:8000\"}[1m])))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0.285"],[1793467815,"0.285"],[1793467830,"0.285"],[1793467845,"0.285"],[1793467860,"0.285"],[1793467875,"0.285"],[1793467890,"0.285"],[1793467905,"0.285"],[1793467920,"0.2849999999999999"],[1793467935,"0.2849999999999999"],[1793467950,"0.2849999999999999"],[1793467965,"0.2849999999999999"],[1793467980,"0.285"],[1793467995,"0.2849999999999999"],[1793468010,"0.2849999999999999"],[1793468025,"0.2849999999999999"],[1793468040,"0.285"],[1793468055,"0.2849999999999999"],[1793468070,"0.2849999999999999"],[1793468085,"0.285"],[1793468100,"0.2849999999999999"],[1793468115,"0.285"],[1793468130,"0.285"],[1793468145,"0.285"],[1793468160,"0.2849999999999999"],[1793468175,"0.2849999999999999"],[1793468190,"0.2849999999999999"],[1793468205,"0.2849999999999999"],[1793468220,"0.2849999999999999"],[1793468235,"0.285"],[1793468250,"0.285"],[1793468265,"0.2849999999999999"],[1793468280,"0.285"],[1793468295,"0.2849999999999999"],[1793468310,"0.285"],[1793468325,"0.285"],[1793468340,"0.285"],[1793468355,"0.285"],[1793468370,"0.2849999999999999"],[1793468385,"0.285"],[1793468400,"0.285"],[1793468415,"0.285"],[1793468430,"0.2849999999999999"],[1793468445,"0.285"],[1793468460,"0.285"],[1793468475,"0.285"],[1793468490,"0.285"],[1793468505,"0.2849999999999999"],[1793468520,"0.285"],[1793468535,"0.285"]]}]}}},{"expr":"sum by (finished_reason) (rate(vllm:request_success_total{job=\"vllm\",instance=\"127.0.0.1:8000\"}[1m]))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{"finished_reason":"abort"},"values":[[1793467800,"0"],[1793467815,"0"],[1793467830,"0"],[1793467845,"0"],[1793467860,"0"],[1793467875,"0"],[1793467890,"0"],[1793467905,"0"],[1793467920,"0"],[1793467935,"0"],[1793467950,"0"],[1793467965,"0"],[1793467980,"0"],[1793467995,"0"],[1793468010,"0"],[1793468025,"0"],[1793468040,"0"],[1793468055,"0"],[1793468070,"0"],[1793468085,"0"],[1793468100,"0"],[1793468115,"0"],[1793468130,"0"],[1793468145,"0"],[1793468160,"0"],[1793468175,"0"],[1793468190,"0"],[1793468205,"0"],[1793468220,"0"],[1793468235,"0"],[1793468250,"0"],[1793468265,"0"],[1793468280,"0"],[1793468295,"0"],[1793468310,"0"],[1793468325,"0"],[1793468340,"0"],[1793468355,"0"],[1793468370,"0"],[1793468385,"0"],[1793468400,"0"],[1793468415,"0"],[1793468430,"0"],[1793468445,"0"],[1793468460,"0"],[1793468475,"0"],[1793468490,"0"],[1793468505,"0"],[1793468520,"0"],[1793468535,"0"]]},{"metric":{"finished_reason":"error"},"values":[[1793467800,"0"],[1793467815,"0"],[1793467830,"0"],[1793467845,"0"],[1793467860,"0"],[1793467875,"0"],[1793467890,"0"],[1793467905,"0"],[1793467920,"0"],[1793467935,"0"],[1793467950,"0"],[1793467965,"0"],[1793467980,"0"],[1793467995,"0"],[1793468010,"0"],[1793468025,"0"],[1793468040,"0"],[1793468055,"0"],[1793468070,"0"],[1793468085,"0"],[1793468100,"0"],[1793468115,"0"],[1793468130,"0"],[1793468145,"0"],[1793468160,"0"],[1793468175,"0"],[1793468190,"0"],[1793468205,"0"],[1793468220,"0"],[1793468235,"0"],[1793468250,"0"],[1793468265,"0"],[1793468280,"0"],[1793468295,"0"],[1793468310,"0"],[1793468325,"0"],[1793468340,"0"],[1793468355,"0"],[1793468370,"0"],[1793468385,"0"],[1793468400,"0"],[1793468415,"0"],[1793468430,"0"],[1793468445,"0"],[1793468460,"0"],[1793468475,"0"],[1793468490,"0"],[1793468505,"0"],[1793468520,"0"],[1793468535,"0"]]},{"metric":{"finished_reason":"length"},"values":[[1793467800,"0.017798333333333336"],[1793467815,"0.042798333333333334"],[1793467830,"0.06779833333333334"],[1793467845,"0.09279833333333333"],[1793467860,"0.09999999999999999"],[1793467875,"0.09999999999999999"],[1793467890,"0.09999999999999999"],[1793467905,"0.09999636376859024"],[1793467920,"0.09999818185123907"],[1793467935,"0.0999945457520499"],[1793467950,"0.0999945457520499"],[1793467965,"0.09999818185123907"],[1793467980,"0.09999999999999999"],[1793467995,"0.09999818185123907"],[1793468010,"0.09999818185123907"],[1793468025,"0.09999818185123907"],[1793468040,"0.09999999999999999"],[1793468055,"0.09999818185123907"],[1793468070,"0.09999818185123907"],[1793468085,"0.09999999999999999"],[1793468100,"0.09999818185123907"],[1793468115,"0.09999999999999999"],[1793468130,"0.09999999999999999"],[1793468145,"0.09999999999999999"],[1793468160,"0.09999818185123907"],[1793468175,"0.09999818185123907"],[1793468190,"0.09999818185123907"],[1793468205,"0.09999818185123907"],[1793468220,"0.09999818185123907"],[1793468235,"0.10000727325623682"],[1793468250,"0.09999999999999999"],[1793468265,"0.09999818185123907"],[1793468280,"0.09999999999999999"],[1793468295,"0.09999818185123907"],[1793468310,"0.09999999999999999"],[1793468325,"0.09999999999999999"],[1793468340,"0.09999999999999999"],[1793468355,"0.09999999999999999"],[1793468370,"0.09999818185123907"],[1793468385,"0.09999999999999999"],[1793468400,"0.09999999999999999"],[1793468415,"0.09999636376859024"],[1793468430,"0.09999818185123907"],[1793468445,"0.09999999999999999"],[1793468460,"0.09999999999999999"],[1793468475,"0.09999999999999999"],[1793468490,"0.09999999999999999"],[1793468505,"0.09999818185123907"],[1793468520,"0.10000545484299143"],[1793468535,"0.10001091028112158"]]},{"metric":{"finished_reason":"repetition"},"values":[[1793467800,"0"],[1793467815,"0"],[1793467830,"0"],[1793467845,"0"],[1793467860,"0"],[1793467875,"0"],[1793467890,"0"],[1793467905,"0"],[1793467920,"0"],[1793467935,"0"],[1793467950,"0"],[1793467965,"0"],[1793467980,"0"],[1793467995,"0"],[1793468010,"0"],[1793468025,"0"],[1793468040,"0"],[1793468055,"0"],[1793468070,"0"],[1793468085,"0"],[1793468100,"0"],[1793468115,"0"],[1793468130,"0"],[1793468145,"0"],[1793468160,"0"],[1793468175,"0"],[1793468190,"0"],[1793468205,"0"],[1793468220,"0"],[1793468235,"0"],[1793468250,"0"],[1793468265,"0"],[1793468280,"0"],[1793468295,"0"],[1793468310,"0"],[1793468325,"0"],[1793468340,"0"],[1793468355,"0"],[1793468370,"0"],[1793468385,"0"],[1793468400,"0"],[1793468415,"0"],[1793468430,"0"],[1793468445,"0"],[1793468460,"0"],[1793468475,"0"],[1793468490,"0"],[1793468505,"0"],[1793468520,"0"],[1793468535,"0"]]},{"metric":{"finished_reason":"stop"},"values":[[1793467800,"0.08899166666666668"],[1793467815,"0.21399166666666666"],[1793467830,"0.3389916666666667"],[1793467845,"0.46399166666666664"],[1793467860,"0.5"],[1793467875,"0.5"],[1793467890,"0.5"],[1793467905,"0.4999818188429512"],[1793467920,"0.4999909092561953"],[1793467935,"0.4999727287602495"],[1793467950,"0.4999727287602495"],[1793467965,"0.4999909092561953"],[1793467980,"0.5"],[1793467995,"0.4999909092561953"],[1793468010,"0.4999909092561953"],[1793468025,"0.4999909092561953"],[1793468040,"0.5"],[1793468055,"0.4999909092561953"],[1793468070,"0.4999909092561953"],[1793468085,"0.5"],[1793468100,"0.4999909092561953"],[1793468115,"0.5"],[1793468130,"0.5"],[1793468145,"0.5"],[1793468160,"0.4999909092561953"],[1793468175,"0.4999909092561953"],[1793468190,"0.4999909092561953"],[1793468205,"0.4999909092561953"],[1793468220,"0.4999909092561953"],[1793468235,"0.5000363662811841"],[1793468250,"0.5"],[1793468265,"0.4999909092561953"],[1793468280,"0.5"],[1793468295,"0.4999909092561953"],[1793468310,"0.5"],[1793468325,"0.5"],[1793468340,"0.5"],[1793468355,"0.5"],[1793468370,"0.4999909092561953"],[1793468385,"0.5"],[1793468400,"0.5"],[1793468415,"0.4999818188429512"],[1793468430,"0.4999909092561953"],[1793468445,"0.5"],[1793468460,"0.5"],[1793468475,"0.5"],[1793468490,"0.5"],[1793468505,"0.4999909092561953"],[1793468520,"0.5000272742149572"],[1793468535,"0.500054551405608"]]}]}}}]}
````

<!-- block: file t04-src-tauri-src-history-testdata-prometheus-3-15-1-range-vllm-8003-json step=4.3 path=src-tauri/src/history/testdata/prometheus-3.15.1/range-vllm-8003.json -->
````
{"start":1793467635,"end":1793468535,"step":15,"rate":"1m","queries":[{"expr":"sum(rate(vllm:prompt_tokens_total{job=\"vllm\",instance=\"127.0.0.1:8003\"}[1m]))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"4.3381940000000005"],[1793467815,"9.543194"],[1793467830,"14.748194"],[1793467845,"20.819999999999997"],[1793467860,"20.81924293662049"],[1793467875,"20.818750874947504"],[1793467890,"37.85454545454545"],[1793467905,"39.55800000000001"],[1793467920,"39.558"],[1793467935,"37.85454545454545"],[1793467950,"20.81833453323734"],[1793467965,"20.81810744477775"],[1793467980,"20.81962146142797"],[1793467995,"37.853857202596316"],[1793468010,"37.853857202596316"],[1793468025,"37.853857202596316"],[1793468040,"37.85316897567361"],[1793468055,"20.81962146142797"],[1793468070,"20.819999999999997"],[1793468085,"20.81924293662049"],[1793468100,"37.85454545454545"],[1793468115,"37.853857202596316"],[1793468130,"37.853857202596316"],[1793468145,"37.8552337315224"],[1793468160,"20.82"],[1793468175,"20.81962146142797"],[1793468190,"20.819621461427978"],[1793468205,"37.85454545454545"],[1793468220,"37.85454545454545"],[1793468235,"37.85316897567361"],[1793468250,"20.81962146142797"],[1793468265,"20.819621461427978"],[1793468280,"20.820378552337317"],[1793468295,"37.853857202596316"],[1793468310,"37.85454545454545"],[1793468325,"37.853857202596316"],[1793468340,"37.85454545454545"],[1793468355,"20.819999999999997"],[1793468370,"20.81962146142797"],[1793468385,"20.819621461427978"],[1793468400,"37.853857202596316"],[1793468415,"37.853857202596316"],[1793468430,"37.853857202596316"],[1793468445,"37.85454545454545"],[1793468460,"20.819621461427978"],[1793468475,"20.81962146142797"],[1793468490,"20.81924293662049"],[1793468505,"37.85454545454545"],[1793468520,"37.85592203352849"],[1793468535,"37.85798708973544"]]}]}}},{"expr":"histogram_quantile(0.95, sum by (le) (rate(vllm:time_to_first_token_seconds_bucket{job=\"vllm\",instance=\"127.0.0.1:8003\"}[1m])))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{},"values":[[1793467800,"0.23500000000000026"],[1793467815,"0.23500000000000004"],[1793467830,"0.2349999999999999"],[1793467845,"0.235"],[1793467860,"0.23500000000000026"],[1793467875,"0.23499999999999988"],[1793467890,"0.23499999999999996"],[1793467905,"0.23500000000000015"],[1793467920,"0.23500000000000015"],[1793467935,"0.23499999999999996"],[1793467950,"0.23500000000000004"],[1793467965,"0.23500000000000024"],[1793467980,"0.23499999999999996"],[1793467995,"0.23500000000000004"],[1793468010,"0.23499999999999988"],[1793468025,"0.23500000000000004"],[1793468040,"0.235"],[1793468055,"0.23499999999999996"],[1793468070,"0.235"],[1793468085,"0.2349999999999998"],[1793468100,"0.23499999999999996"],[1793468115,"0.23500000000000004"],[1793468130,"0.23500000000000004"],[1793468145,"0.2349999999999999"],[1793468160,"0.23499999999999982"],[1793468175,"0.23499999999999982"],[1793468190,"0.23500000000000013"],[1793468205,"0.23499999999999996"],[1793468220,"0.23499999999999996"],[1793468235,"0.235"],[1793468250,"0.23499999999999996"],[1793468265,"0.23500000000000013"],[1793468280,"0.23499999999999993"],[1793468295,"0.23500000000000004"],[1793468310,"0.23499999999999996"],[1793468325,"0.23500000000000004"],[1793468340,"0.23499999999999996"],[1793468355,"0.235"],[1793468370,"0.23499999999999996"],[1793468385,"0.2349999999999998"],[1793468400,"0.23500000000000004"],[1793468415,"0.23500000000000004"],[1793468430,"0.23500000000000004"],[1793468445,"0.23499999999999996"],[1793468460,"0.23499999999999982"],[1793468475,"0.23499999999999982"],[1793468490,"0.2350000000000003"],[1793468505,"0.23499999999999996"],[1793468520,"0.23500000000000004"],[1793468535,"0.23499999999999996"]]}]}}}]}
````

<!-- block: file t04-src-tauri-src-history-testdata-prometheus-3-15-1-targets-json step=4.3 path=src-tauri/src/history/testdata/prometheus-3.15.1/targets.json -->
````
{"status":"success","data":{"activeTargets":[{"discoveredLabels":{"__address__":"127.0.0.1:30000","__always_scrape_classic_histograms__":"false","__convert_classic_histograms_to_nhcb__":"false","__metrics_path__":"/metrics","__scheme__":"http","__scrape_interval__":"5s","__scrape_native_histograms__":"false","__scrape_timeout__":"5s","job":"sglang"},"labels":{"instance":"127.0.0.1:30000","job":"sglang"},"scrapePool":"sglang","scrapeUrl":"http://127.0.0.1:30000/metrics","globalUrl":"http://prometheus-host.local:30000/metrics","lastError":"","lastScrape":"2026-10-31T20:42:16.926755+03:00","lastScrapeDuration":0.001851209,"health":"up","scrapeInterval":"5s","scrapeTimeout":"5s"},{"discoveredLabels":{"__address__":"127.0.0.1:8000","__always_scrape_classic_histograms__":"false","__convert_classic_histograms_to_nhcb__":"false","__metrics_path__":"/metrics","__scheme__":"http","__scrape_interval__":"5s","__scrape_native_histograms__":"false","__scrape_timeout__":"5s","job":"vllm"},"labels":{"instance":"127.0.0.1:8000","job":"vllm"},"scrapePool":"vllm","scrapeUrl":"http://127.0.0.1:8000/metrics","globalUrl":"http://prometheus-host.local:8000/metrics","lastError":"","lastScrape":"2026-10-31T20:42:16.826047+03:00","lastScrapeDuration":0.001522084,"health":"up","scrapeInterval":"5s","scrapeTimeout":"5s"},{"discoveredLabels":{"__address__":"127.0.0.1:8001","__always_scrape_classic_histograms__":"false","__convert_classic_histograms_to_nhcb__":"false","__metrics_path__":"/metrics","__scheme__":"http","__scrape_interval__":"5s","__scrape_native_histograms__":"false","__scrape_timeout__":"5s","job":"vllm"},"labels":{"instance":"127.0.0.1:8001","job":"vllm"},"scrapePool":"vllm","scrapeUrl":"http://127.0.0.1:8001/metrics","globalUrl":"http://prometheus-host.local:8001/metrics","lastError":"","lastScrape":"2026-10-31T20:42:17.706472+03:00","lastScrapeDuration":0.00577975,"health":"up","scrapeInterval":"5s","scrapeTimeout":"5s"},{"discoveredLabels":{"__address__":"127.0.0.1:8003","__always_scrape_classic_histograms__":"false","__convert_classic_histograms_to_nhcb__":"false","__metrics_path__":"/metrics","__scheme__":"http","__scrape_interval__":"5s","__scrape_native_histograms__":"false","__scrape_timeout__":"5s","job":"vllm"},"labels":{"instance":"127.0.0.1:8003","job":"vllm"},"scrapePool":"vllm","scrapeUrl":"http://127.0.0.1:8003/metrics","globalUrl":"http://prometheus-host.local:8003/metrics","lastError":"","lastScrape":"2026-10-31T20:42:20.004305+03:00","lastScrapeDuration":0.00274375,"health":"up","scrapeInterval":"5s","scrapeTimeout":"5s"}],"droppedTargets":[],"droppedTargetCounts":null}}
````

<!-- block: file t04-src-tauri-src-history-testdata-prometheus-3-15-1-up-json step=4.3 path=src-tauri/src/history/testdata/prometheus-3.15.1/up.json -->
````
{"status":"success","data":{"resultType":"vector","result":[{"metric":{"__name__":"up","instance":"127.0.0.1:8001","job":"vllm"},"value":[1793468540.553,"1"]},{"metric":{"__name__":"up","instance":"127.0.0.1:8003","job":"vllm"},"value":[1793468540.553,"1"]},{"metric":{"__name__":"up","instance":"127.0.0.1:8000","job":"vllm"},"value":[1793468540.553,"1"]},{"metric":{"__name__":"up","instance":"127.0.0.1:30000","job":"sglang"},"value":[1793468540.553,"1"]}]}}
````

<!-- block: patch t04-src-tauri-src-servers-client-rs step=4.3 -->
````
diff --git a/src-tauri/src/servers/client.rs b/src-tauri/src/servers/client.rs
index 9f2c72f..8743198 100644
--- a/src-tauri/src/servers/client.rs
+++ b/src-tauri/src/servers/client.rs
@@ -92,6 +92,8 @@ pub struct ServerClient {
     http: reqwest::Client,
     base: url::Url,
     api_key: Option<String>,
+    /// Вход по имени и паролю (`Authorization: Basic`) — у источника истории (спека I1c §2).
+    basic: Option<(String, String)>,
     metrics_path: String,
 }
 
@@ -101,6 +103,7 @@ impl std::fmt::Debug for ServerClient {
         f.debug_struct("ServerClient")
             .field("base", &self.base.as_str())
             .field("api_key", &self.api_key.as_ref().map(|_| "***"))
+            .field("basic", &self.basic.as_ref().map(|(u, _)| (u, "***")))
             .finish()
     }
 }
@@ -161,10 +164,18 @@ impl ServerClient {
             http,
             base,
             api_key: api_key.filter(|k| !k.trim().is_empty()),
+            basic: None,
             metrics_path: s.metrics_path.clone(),
         })
     }
 
+    /// Вход по имени и паролю вместо ключа API; пароль не печатается и не попадает в ошибки.
+    pub fn with_basic_auth(mut self, username: &str, password: String) -> Self {
+        self.api_key = None;
+        self.basic = Some((username.to_string(), password));
+        self
+    }
+
     /// Путь метрик из профиля сервера.
     pub fn metrics_path(&self) -> &str {
         &self.metrics_path
@@ -188,7 +199,26 @@ impl ServerClient {
 
     /// GET `path`: ответ с любым кодом, тело — не больше `limit` байт.
     pub async fn get(&self, path: &str, limit: usize) -> AppResult<HttpReply> {
-        let mut req = self.http.get(self.url(path));
+        self.fetch(self.url(path), limit).await
+    }
+
+    /// GET `path` с параметрами запроса (`query=…&start=…`); остальное — как у `get`.
+    pub async fn get_query(
+        &self,
+        path: &str,
+        query: &[(&str, &str)],
+        limit: usize,
+    ) -> AppResult<HttpReply> {
+        let mut url = self.url(path);
+        url.query_pairs_mut().extend_pairs(query);
+        self.fetch(url, limit).await
+    }
+
+    async fn fetch(&self, url: url::Url, limit: usize) -> AppResult<HttpReply> {
+        let mut req = self.http.get(url);
+        if let Some((user, password)) = &self.basic {
+            req = req.basic_auth(user, Some(password));
+        }
         if let Some(key) = &self.api_key {
             let mut value =
                 reqwest::header::HeaderValue::from_str(&format!("Bearer {}", key.trim())).map_err(
````

<!-- block: patch t04-src-tauri-src-servers-testing-rs step=4.3 -->
````
diff --git a/src-tauri/src/servers/testing.rs b/src-tauri/src/servers/testing.rs
index 4b0f2ff..0568eb3 100644
--- a/src-tauri/src/servers/testing.rs
+++ b/src-tauri/src/servers/testing.rs
@@ -206,8 +206,11 @@ async fn serve<S: AsyncRead + AsyncWrite + Unpin>(
         authorization: header("authorization"),
         user_agent: header("user-agent"),
     });
+    // Сначала путь с параметрами запроса, затем без них: у API истории параметры разные.
+    let bare = path.split('?').next().unwrap_or_default();
     let reply = routes
         .get(&path)
+        .or_else(|| routes.get(bare))
         .cloned()
         .unwrap_or_else(|| Reply::status(404));
     if let Some(d) = reply.delay {
````

<!-- block: locales t04-locales-t04 step=4.3 -->
````
{
  "en": { "errors": { "history": {
    "unexpected": "Unexpected answer of the metrics history source to {{path}}: not the Prometheus HTTP API",
    "tooLarge": "The answer of the metrics history source is larger than 8 MB: choose a shorter window",
    "unauthorized": "The metrics history source rejected the sign-in (401): check the token or the user name and password",
    "forbidden": "The metrics history source denied access (403): the account may not read metrics",
    "timeout": "The metrics history source did not finish the query in time: choose a shorter window",
    "tooManyPoints": "The metrics history source refused the query: too many points per series",
    "badData": "The metrics history source rejected the query: {{message}}",
    "failed": "The metrics history source failed the query: {{message}}",
    "status": "The metrics history source answered {{path}} with {{status}}"
  } } },
  "ru": { "errors": { "history": {
    "unexpected": "Неожиданный ответ источника истории метрик на {{path}}: это не HTTP API Prometheus",
    "tooLarge": "Ответ источника истории метрик больше 8 МБ: выберите окно короче",
    "unauthorized": "Источник истории метрик отклонил вход (401): проверьте токен или имя пользователя и пароль",
    "forbidden": "Источник истории метрик запретил доступ (403): учётной записи, возможно, нельзя читать метрики",
    "timeout": "Источник истории метрик не успел выполнить запрос: выберите окно короче",
    "tooManyPoints": "Источник истории метрик отказал в запросе: слишком много точек на ряд",
    "badData": "Источник истории метрик отклонил запрос: {{message}}",
    "failed": "Источник истории метрик не выполнил запрос: {{message}}",
    "status": "Источник истории метрик ответил на {{path}} кодом {{status}}"
  } } }
}
````

- [ ] **Шаг 4.4: Проверки.**

Run: `cd src-tauri && cargo test --lib -q`
Expected: `test result: ok. 270 passed`
Run: `cd src-tauri && cargo clippy --lib --tests -q -- -D warnings && cargo fmt --check`
Expected: без вывода
Run: `pnpm i18n:check`
Expected: `✓ словари en, ru согласованы, вызовов из Rust: 124`

- [ ] **Шаг 4.5: Коммит.**

<!-- block: shell t04-commit step=4.5 -->
````
git add src-tauri/src/history/client.rs \
  src-tauri/src/history/mod.rs \
  src-tauri/src/history/testdata/prometheus-3.15.1/README.md \
  src-tauri/src/history/testdata/prometheus-3.15.1/bad-data.json \
  src-tauri/src/history/testdata/prometheus-3.15.1/buildinfo.json \
  src-tauri/src/history/testdata/prometheus-3.15.1/discover-sglang.json \
  src-tauri/src/history/testdata/prometheus-3.15.1/discover.json \
  src-tauri/src/history/testdata/prometheus-3.15.1/empty.json \
  src-tauri/src/history/testdata/prometheus-3.15.1/range-sglang-30000.json \
  src-tauri/src/history/testdata/prometheus-3.15.1/range-vllm-8000.json \
  src-tauri/src/history/testdata/prometheus-3.15.1/range-vllm-8003.json \
  src-tauri/src/history/testdata/prometheus-3.15.1/targets.json \
  src-tauri/src/history/testdata/prometheus-3.15.1/up.json \
  src-tauri/src/servers/client.rs \
  src-tauri/src/servers/testing.rs \
  src/locales/en.json \
  src/locales/ru.json
git commit -s -q -F - <<'EOF'
feat(history): Prometheus API client and real answers

The history client reuses the server HTTP client (CA, proxy, timeouts,
8 MB limit, no redirects) with bearer or basic auth and query parameters.
It parses vector and matrix answers (NaN and Inf are missing values),
maps 401/403, bad_data, timeout and other API errors to history keys, and
reads the scrape interval of a target. Tests use answers of a real
Prometheus 3.15.1 that scraped the mock servers.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
````

---

### Задача 5: Сборка точек истории

Матрицы запросов выравниваются по шагам окна (спека I1c §4): точка — `ServerMetrics` той же формы, что в окне
«Сейчас», поэтому графики не меняются. Шаг без значения — разрыв, метрика без значения во всём окне — `missing`,
причина завершения без метки — `unknown`, сбой запроса — запись `partial` его графика (одна на график).

**Files:**
- Create: `src-tauri/src/history/series.rs`
- Modify: `src-tauri/src/history/mod.rs`

**Interfaces:**
- Consumes: `history::client::RangeSeries`, `history::promql::{Query, QueryKey}`, `history::window::Range`,
  `metrics::derive::{ServerMetrics, Gauges, Rates, Latency, LatencySet}`.
- Produces: `history::series::{assemble, HistoryPoint { at, m }, Partial { chart, error }, QueryResult}`;
  `assemble(&Range, Vec<QueryResult>) -> (Vec<HistoryPoint>, Vec<Partial>)`.

- [ ] **Шаг 5.1: Тесты.** Тесты `history/series.rs` на снятых окнах vLLM, SGLang и перезапуска; модуль подключается.

<!-- block: patch t05-src-tauri-src-history-mod-rs step=5.1 -->
````
diff --git a/src-tauri/src/history/mod.rs b/src-tauri/src/history/mod.rs
index 78a6e88..4ff3ee8 100644
--- a/src-tauri/src/history/mod.rs
+++ b/src-tauri/src/history/mod.rs
@@ -3,5 +3,6 @@
 
 pub mod client;
 pub mod promql;
+pub mod series;
 pub mod targets;
 pub mod window;
````

<!-- block: file t05-src-tauri-src-history-series-rs-tests step=5.1 path=src-tauri/src/history/series.rs part=tests -->
````
#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::client::tests::snapshot;
    use crate::history::client::{parse_api, parse_matrix};
    use crate::history::promql::queries;
    use crate::history::window::duration;
    use crate::servers::adapters::adapter;
    use crate::servers::client::HttpReply;
    use crate::servers::detect::DetectedKind;

    /// Окно снимка `range-<цель>.json` и результаты шаблонов по настоящим ответам.
    fn captured(name: &str, kind: DetectedKind, selector: &str) -> (Range, Vec<QueryResult>) {
        let doc: serde_json::Value =
            serde_json::from_str(&snapshot(&format!("range-{name}.json"))).unwrap();
        let range = Range {
            start: doc["start"].as_u64().unwrap(),
            end: doc["end"].as_u64().unwrap(),
            step: doc["step"].as_u64().unwrap(),
            rate: 60,
        };
        assert_eq!(doc["rate"], duration(range.rate));
        let answers: HashMap<String, serde_json::Value> = doc["queries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|q| {
                (
                    q["expr"].as_str().unwrap().to_string(),
                    q["response"].clone(),
                )
            })
            .collect();
        let results = queries(adapter(kind).metric_map(), selector, "1m")
            .into_iter()
            .filter_map(|q| {
                let answer = answers.get(&q.expr)?;
                let reply = HttpReply {
                    status: 200,
                    content_type: None,
                    body: answer.to_string().into_bytes(),
                    truncated: false,
                };
                let parsed =
                    parse_api(&reply, "/api/v1/query_range").and_then(|d| parse_matrix(&d));
                Some((q, parsed))
            })
            .collect();
        (range, results)
    }

    #[test]
    fn vllm_window_from_a_real_prometheus() {
        let (range, results) = captured(
            "vllm-8000",
            DetectedKind::Vllm,
            r#"job="vllm",instance="127.0.0.1:8000""#,
        );
        assert_eq!(results.len(), 20, "все шаблоны сняты");
        let (points, partial) = assemble(&range, results);
        assert!(partial.is_empty());
        assert_eq!(points.len(), 61);
        assert!(points.windows(2).all(|w| w[1].at - w[0].at == 15_000));
        // До запуска Prometheus значений нет: разрыв, а не ноль.
        let first = &points[0].m;
        assert_eq!(first.gauges.kv_usage, None);
        assert_eq!(first.rates.as_ref().unwrap().prompt_tokens, None);
        assert!(first.missing.is_empty(), "{:?}", first.missing);
        let last = &points[60].m;
        let rates = last.rates.as_ref().unwrap();
        // Мок растит счётчики на 10 % снимка за опрос раз в 5 с: 20,8 токена/с на входе.
        assert!(
            (rates.prompt_tokens.unwrap() - 20.82).abs() < 0.05,
            "{rates:?}"
        );
        assert!((rates.generation_tokens.unwrap() - 10.1).abs() < 0.05);
        assert_eq!(
            rates.finished_by_reason.keys().collect::<Vec<_>>(),
            ["abort", "error", "length", "repetition", "stop"]
        );
        assert!((rates.requests.unwrap() - 0.6).abs() < 0.01, "{rates:?}");
        let ttft = last.latency.ttft.as_ref().unwrap();
        assert!((ttft.p95.unwrap() - 235.0).abs() < 1.0, "{ttft:?}");
        assert_eq!(last.latency_1m, last.latency);
        assert_eq!(last.latency.queue_time.as_ref().unwrap().p50, None);
        assert!(last.gauges.kv_usage.unwrap() > 0.0);
        assert_eq!(last.gauges.kv_usage, last.gauges.kv_usage_avg);
        assert!((last.prefix_cache_hit_ratio.unwrap() - 0.4457).abs() < 0.001);
        assert_eq!(last.preemptions_1m, Some(0.0));
        assert_eq!((last.engines, last.reset), (0, false));
    }

    #[test]
    fn sglang_reasons_without_a_label_are_unknown() {
        let (range, results) = captured(
            "sglang-30000",
            DetectedKind::Sglang,
            r#"job="sglang",instance="127.0.0.1:30000""#,
        );
        let (points, partial) = assemble(&range, results);
        assert!(partial.is_empty());
        let rates = points[60].m.rates.clone().unwrap();
        assert_eq!(
            rates.finished_by_reason.keys().collect::<Vec<_>>(),
            ["unknown"]
        );
        assert!(rates.aborted.unwrap() > 0.0);
        assert!(points[60].m.preemptions_1m.unwrap() > 0.0);
    }

    #[test]
    fn restart_has_no_negative_rates() {
        let (range, results) = captured(
            "vllm-8003",
            DetectedKind::Vllm,
            r#"job="vllm",instance="127.0.0.1:8003""#,
        );
        assert_eq!(results.len(), 2);
        let (points, _) = assemble(&range, results);
        let rates: Vec<f64> = points
            .iter()
            .filter_map(|p| p.m.rates.as_ref()?.prompt_tokens)
            .collect();
        assert!(rates.len() > 40);
        assert!(rates.iter().all(|r| *r >= 0.0), "{rates:?}");
        // Этих рядов не снимали: метрики — «нет данных» (missing не трогается).
        assert_eq!(points[60].m.gauges.kv_usage, None);
    }

    #[test]
    fn failed_query_is_partial_and_empty_series_is_missing() {
        let range = Range {
            start: 1_000,
            end: 1_030,
            step: 15,
            rate: 60,
        };
        let map = adapter(DetectedKind::Vllm).metric_map();
        let qs = queries(map, r#"job="j""#, "1m");
        let series = |v: Option<f64>| {
            vec![RangeSeries {
                labels: BTreeMap::new(),
                points: vec![(1_000, v), (1_015, v), (1_030, v)],
            }]
        };
        let results = qs
            .into_iter()
            .map(|q| {
                let r = match q.key {
                    QueryKey::Quantile {
                        canonical: Canonical::Ttft,
                        ..
                    } => Err(crate::history::client::parse_api(
                        &HttpReply {
                            status: 503,
                            content_type: None,
                            body: br#"{"status":"error","errorType":"timeout","error":"x"}"#
                                .to_vec(),
                            truncated: false,
                        },
                        "/q",
                    )
                    .unwrap_err()),
                    QueryKey::Preemptions => Ok(Vec::new()),
                    QueryKey::KvUsageAvg => Ok(series(None)),
                    _ => Ok(series(Some(1.0))),
                };
                (q, r)
            })
            .collect();
        let (points, partial) = assemble(&range, results);
        assert_eq!(points.len(), 3);
        assert_eq!(partial.len(), 1, "три квантиля TTFT — одна запись");
        assert_eq!(partial[0].chart, "ttft");
        assert_eq!(partial[0].error.key, "errors.history.timeout");
        let m = &points[1].m;
        assert_eq!(m.latency.ttft, None);
        assert_eq!(m.missing, [Canonical::PreemptionsTotal]);
        assert_eq!(m.gauges.kv_usage, Some(1.0));
        assert_eq!(m.gauges.kv_usage_avg, None);
        let json = serde_json::to_value(&partial[0]).unwrap();
        assert_eq!(json["chart"], "ttft");
        assert_eq!(json["error"]["key"], "errors.history.timeout");
    }
}
````

- [ ] **Шаг 5.2: Увидеть падение.**

Run: `cd src-tauri && cargo test --lib -q history`
Expected: FAIL — не компилируется: тестам не хватает типов и функций, которые появятся в шаге 3

- [ ] **Шаг 5.3: Реализация.** Сборка точек.

<!-- block: file t05-src-tauri-src-history-series-rs step=5.3 path=src-tauri/src/history/series.rs part=impl -->
````
//! Сборка точек истории (спека I1c §4): матрицы запросов выравниваются по меткам времени окна,
//! точка — `ServerMetrics` той же формы, что в окне «Сейчас», поэтому графики не меняются.

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::history::client::RangeSeries;
use crate::history::promql::{Query, QueryKey};
use crate::history::window::Range;
use crate::metrics::derive::{Gauges, Latency, LatencySet, Rates, ServerMetrics};
use crate::servers::adapters::Canonical;

/// Точка истории — как точка буфера «Сейчас» во фронтенде: время (мс) и метрики.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HistoryPoint {
    pub at: u64,
    pub m: ServerMetrics,
}

/// Ряд, который не удалось прочитать: график и ошибка.
#[derive(Debug, Clone, Serialize)]
pub struct Partial {
    pub chart: &'static str,
    pub error: AppError,
}

/// Запрос и его результат: ряды или ошибка.
pub type QueryResult = (Query, AppResult<Vec<RangeSeries>>);

/// Значения ряда по метке времени (с); у разбитого ряда — по значению метки.
type Values = HashMap<u64, Option<f64>>;

fn values(series: &RangeSeries) -> Values {
    series.points.iter().copied().collect()
}

/// Значение по ключу ряда в момент `t`.
struct Table {
    plain: HashMap<String, Values>,
    grouped: HashMap<String, BTreeMap<String, Values>>,
}

fn key_id(key: QueryKey) -> String {
    format!("{key:?}")
}

impl Table {
    fn get(&self, key: QueryKey, t: u64) -> Option<f64> {
        self.plain.get(&key_id(key))?.get(&t).copied().flatten()
    }

    fn latency(&self, c: Canonical, t: u64) -> Option<Latency> {
        let q = |q: f64| {
            self.get(QueryKey::Quantile { canonical: c, q }, t)
                .map(|v| v * 1000.0)
        };
        let (p50, p95, p99) = (q(0.5), q(0.95), q(0.99));
        // Число наблюдений история не запрашивает: `count` у её точек — 0.
        (p50.is_some() || p95.is_some() || p99.is_some()).then_some(Latency {
            p50,
            p95,
            p99,
            count: 0.0,
        })
    }
}

/// Точки окна `range` из результатов запросов и список рядов, которые не прочитались. Точка есть
/// на каждый шаг окна; без значения — `None` (на графике разрыв). Метрика, у которой нет ни
/// одного значения во всём окне, — в `missing` каждой точки (как отсутствующая серия в I1b).
pub fn assemble(range: &Range, results: Vec<QueryResult>) -> (Vec<HistoryPoint>, Vec<Partial>) {
    let mut table = Table {
        plain: HashMap::new(),
        grouped: HashMap::new(),
    };
    let mut partial: Vec<Partial> = Vec::new();
    // Метрика → есть ли у неё хоть одно значение (у KV — в любом из рядов: максимум, среднее).
    let mut present: BTreeMap<Canonical, bool> = BTreeMap::new();
    for (query, result) in results {
        let series = match result {
            Ok(s) => s,
            Err(error) => {
                let chart = query.key.chart();
                if !partial.iter().any(|p| p.chart == chart) {
                    partial.push(Partial { chart, error });
                }
                continue;
            }
        };
        let any = series
            .iter()
            .any(|s| s.points.iter().any(|(_, v)| v.is_some()));
        *present.entry(query.key.canonical()).or_default() |= any;
        match query.group_by {
            Some(label) => {
                let groups = table.grouped.entry(key_id(query.key)).or_default();
                for s in &series {
                    let name = s
                        .labels
                        .get(label)
                        .cloned()
                        .unwrap_or_else(|| "unknown".into());
                    groups.insert(name, values(s));
                }
            }
            None => {
                if let Some(s) = series.first() {
                    table.plain.insert(key_id(query.key), values(s));
                }
            }
        }
    }
    let missing: Vec<Canonical> = present
        .into_iter()
        .filter(|(_, any)| !any)
        .map(|(c, _)| c)
        .collect();

    let finished = table.grouped.get(&key_id(QueryKey::FinishedByReason));
    let mut points = Vec::new();
    let mut t = range.start;
    while t <= range.end {
        let get = |k: QueryKey| table.get(k, t);
        let finished_by_reason: BTreeMap<String, f64> = finished
            .map(|g| {
                g.iter()
                    .filter_map(|(reason, v)| Some((reason.clone(), (*v.get(&t)?)?)))
                    .collect()
            })
            .unwrap_or_default();
        let requests =
            (!finished_by_reason.is_empty()).then(|| finished_by_reason.values().sum::<f64>());
        let preemptions = get(QueryKey::Preemptions);
        let latency = LatencySet {
            ttft: table.latency(Canonical::Ttft, t),
            itl: table.latency(Canonical::Itl, t),
            e2e: table.latency(Canonical::E2e, t),
            queue_time: table.latency(Canonical::QueueTime, t),
        };
        let m = ServerMetrics {
            at: t * 1000,
            interval_sec: Some(range.step as f64),
            reset: false,
            engines: 0,
            gauges: Gauges {
                requests_running: get(QueryKey::RequestsRunning),
                requests_waiting: get(QueryKey::RequestsWaiting),
                kv_usage: get(QueryKey::KvUsage),
                kv_usage_avg: get(QueryKey::KvUsageAvg),
            },
            rates: Some(Rates {
                prompt_tokens: get(QueryKey::PromptTokens),
                generation_tokens: get(QueryKey::GenerationTokens),
                requests,
                preemptions,
                aborted: get(QueryKey::Aborted),
                finished_by_reason,
            }),
            prefix_cache_hit_ratio: get(QueryKey::PrefixCacheHitRatio),
            latency_1m: latency.clone(),
            latency,
            // Скорость за интервал `rate` (не меньше минуты) — в вытеснения за минуту.
            preemptions_1m: preemptions.map(|r| r * 60.0),
            totals: BTreeMap::new(),
            missing: missing.clone(),
            series: BTreeMap::new(),
        };
        points.push(HistoryPoint { at: t * 1000, m });
        t += range.step;
    }
    (points, partial)
}
````

- [ ] **Шаг 5.4: Проверки.**

Run: `cd src-tauri && cargo test --lib -q`
Expected: `test result: ok. 274 passed`
Run: `cd src-tauri && cargo clippy --lib --tests -q -- -D warnings && cargo fmt --check`
Expected: без вывода

- [ ] **Шаг 5.5: Коммит.**

<!-- block: shell t05-commit step=5.5 -->
````
git add src-tauri/src/history/mod.rs \
  src-tauri/src/history/series.rs
git commit -s -q -F - <<'EOF'
feat(history): assemble history points from query matrices

Matrices of the template queries are aligned to the window steps and become
ServerMetrics points of the same shape as the live "now" window: gauges,
rates, finished requests by reason (no label is "unknown"), prefix-cache
ratio and percentiles in ms. Steps without values are gaps, metrics without
a value in the whole window are missing, and a failed query becomes a
partial entry of its chart.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
````

---

### Задача 6: Команды check_history_source и read_server_history

Проверка источника (спека I1c §3): конфигурация → DNS/TCP → вход (`buildinfo`) → `up` → цели инференса → шаг
сопоставления на каждый сервер; ответ — шаги, найденные цели и автоподбор для серверов без селектора. Окно истории
сервера: шаблоны по карте адаптера, не больше 4 запросов одновременно, сборка точек; все запросы упали — ошибка
первого. Открытый источник (клиент и тип) кешируется в пуле подключений до `evict` инсталляции. Шаги проверки
сервера (`passed`, `failed_step`, `report`, `tcp_failure`) становятся общими.

**Files:**
- Create: `src-tauri/src/commands/history.rs`
- Create: `src-tauri/src/history/check.rs`
- Create: `src-tauri/src/history/read.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/commands/servers.rs`
- Modify: `src-tauri/src/history/client.rs`
- Modify: `src-tauri/src/history/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/servers/connectivity.rs`
- Modify: `src-tauri/src/servers/pool.rs`
- Modify: `src/locales/en.json`
- Modify: `src/locales/ru.json`

**Interfaces:**
- Consumes: всё из задач 1–5; `commands::servers::connected_server` (становится `pub(crate)`).
- Produces: `history::check::{check_source, HistoryCheck { report, source, targets, suggestions }, Suggestion,
  targets_from, DISCOVER, DISCOVER_SGLANG, no_source}`; `history::read::{read_history, HistoryAnswer { window,
  stepSec, source, points, partial }, Source { client, kind }, CONCURRENCY}`; `ServerPool::{history,
  insert_history_if_current}`; команды Tauri `check_history_source(input)` и `read_server_history(installationId,
  serverId, window)`.

- [ ] **Шаг 6.1: Тесты.** Тесты `history/check.rs`, `history/read.rs`, `commands/history.rs`; модули подключаются.

<!-- block: file t06-src-tauri-src-commands-history-rs-tests step=6.1 path=src-tauri/src/commands/history.rs part=tests -->
````
#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::MemoryStore;
    use crate::storage::profile::{sample_history, sample_profile};

    fn state(dir: &std::path::Path) -> AppState {
        AppState::new(dir, Arc::new(MemoryStore::default())).unwrap()
    }

    #[tokio::test]
    async fn without_a_source_or_a_selector_history_is_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let st = state(dir.path());
        st.installations.upsert(sample_profile("a")).unwrap();
        let e = read_server_history_inner(&st, "a", "s1", Window::Hour)
            .await
            .unwrap_err();
        assert_eq!(e.key, "errors.history.noSource");
        let mut p = sample_profile("a");
        p.history = Some(sample_history());
        st.installations.upsert(p).unwrap();
        let e = read_server_history_inner(&st, "a", "s1", Window::Hour)
            .await
            .unwrap_err();
        assert_eq!(e.key, "errors.history.notMatched");
    }

    #[tokio::test]
    async fn source_is_cached_until_the_installation_is_evicted() {
        let prom = crate::servers::testing::TestServer::start(vec![]).await;
        let dir = tempfile::tempdir().unwrap();
        let st = state(dir.path());
        let mut p = sample_profile("a");
        p.history = Some(crate::history::client::tests::source(
            &prom.url(),
            crate::storage::profile::HistoryAuth::None,
        ));
        st.installations.upsert(p).unwrap();
        let first = history_source(&st, "a").await.unwrap();
        // Без buildinfo (404) — Prometheus.
        assert_eq!(first.kind, SourceKind::Prometheus);
        let again = history_source(&st, "a").await.unwrap();
        assert!(Arc::ptr_eq(&first, &again));
        st.evict_installation("a");
        let fresh = history_source(&st, "a").await.unwrap();
        assert!(!Arc::ptr_eq(&first, &fresh));
        assert_eq!(prom.paths(), ["/api/v1/status/buildinfo"; 2]);
    }
}
````

<!-- block: patch t06-src-tauri-src-commands-mod-rs step=6.1 -->
````
diff --git a/src-tauri/src/commands/mod.rs b/src-tauri/src/commands/mod.rs
index 35cbca0..c5589eb 100644
--- a/src-tauri/src/commands/mod.rs
+++ b/src-tauri/src/commands/mod.rs
@@ -1,5 +1,6 @@
 pub mod app;
 pub mod folders;
+pub mod history;
 pub mod installations;
 pub mod servers;
 pub mod settings;
````

<!-- block: file t06-src-tauri-src-history-check-rs-tests step=6.1 path=src-tauri/src/history/check.rs part=tests -->
````
#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::client::tests::{snapshot, source};
    use crate::servers::testing::{Reply, TestServer};
    use crate::storage::profile::{HistoryAuth, ProxyConfig, sample_profile, sample_server};

    fn query_path(expr: &str) -> String {
        let q = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("query", expr)
            .finish();
        format!("/api/v1/query?{q}")
    }

    fn server(id: &str, url: &str, selector: Option<&str>) -> ServerConfig {
        let mut s = sample_server(id);
        s.name = id.into();
        s.base_url = url.into();
        s.proxy = ProxyConfig::None;
        s.history_selector = selector.map(String::from);
        s
    }

    async fn prometheus() -> TestServer {
        let one = r#"{"status":"success","data":{"resultType":"vector","result":[{"metric":{},"value":[1,"57"]}]}}"#;
        let routes: Vec<(String, Reply)> = vec![
            (
                "/api/v1/status/buildinfo".into(),
                Reply::json(&snapshot("buildinfo.json")),
            ),
            (query_path("up"), Reply::json(&snapshot("up.json"))),
            (
                query_path(DISCOVER),
                Reply::json(&snapshot("discover.json")),
            ),
            (
                query_path(DISCOVER_SGLANG),
                Reply::json(&snapshot("discover-sglang.json")),
            ),
            (
                query_path(
                    r#"count({job="vllm",instance="127.0.0.1:8001",__name__=~"vllm:.+|sglang:.+"})"#,
                ),
                Reply::json(one),
            ),
            (
                query_path(r#"count({job="gone",__name__=~"vllm:.+|sglang:.+"})"#),
                Reply::json(&snapshot("empty.json")),
            ),
        ];
        let routes: Vec<(&str, Reply)> = routes
            .iter()
            .map(|(p, r)| (p.as_str(), r.clone()))
            .collect();
        TestServer::start(routes).await
    }

    #[tokio::test]
    async fn steps_targets_and_suggestions() {
        let prom = prometheus().await;
        let mut p = sample_profile("a");
        p.history = Some(source(&prom.url(), HistoryAuth::None));
        p.servers = vec![
            server("auto", "http://localhost:8000", None),
            server(
                "chosen",
                "http://vllm-svc:8000",
                Some(r#"{job="vllm", instance="127.0.0.1:8001"}"#),
            ),
            server("stale", "http://10.0.0.9:8000", Some(r#"{job="gone"}"#)),
            server("unknown", "http://10.0.0.9:8000", None),
        ];
        let c = check_source(&p, None, &PolicyStatus::default()).await;
        let names: Vec<_> = c.report.steps.iter().map(|s| s.name).collect();
        assert_eq!(
            names,
            [
                "config", "tcp", "auth", "up", "targets", "match", "match", "match", "match"
            ]
        );
        assert!(c.report.ok, "{:?}", c.report.steps);
        assert_eq!(c.report.steps[2].message.params["version"], "3.15.1");
        assert_eq!(c.report.steps[3].message.params["alive"], "4");
        assert_eq!(c.targets.len(), 4);
        assert_eq!(
            c.targets
                .iter()
                .filter(|t| t.kind == TargetKind::Sglang)
                .count(),
            1
        );
        let keys: Vec<_> = c.report.steps[5..]
            .iter()
            .map(|s| s.message.key.as_str())
            .collect();
        assert_eq!(
            keys,
            [
                "messages.historyTest.matchAuto",
                "messages.historyTest.matchSelector",
                "messages.historyTest.noSeries",
                "messages.historyTest.noTarget"
            ]
        );
        assert_eq!(c.report.warnings.len(), 2);
        assert_eq!(
            c.suggestions,
            [
                Suggestion {
                    server_id: "auto".into(),
                    selector: Some(r#"{job="vllm", instance="127.0.0.1:8000"}"#.into()),
                    targets: vec![Target {
                        job: "vllm".into(),
                        instance: "127.0.0.1:8000".into(),
                        kind: TargetKind::Vllm
                    }],
                },
                Suggestion {
                    server_id: "unknown".into(),
                    selector: None,
                    targets: vec![],
                },
            ]
        );
    }

    #[tokio::test]
    async fn unauthorized_source_stops_at_sign_in() {
        let prom = TestServer::start(vec![("/api/v1/status/buildinfo", Reply::status(401))]).await;
        let mut p = sample_profile("a");
        p.history = Some(source(&prom.url(), HistoryAuth::Bearer));
        let c = check_source(&p, Some("t".into()), &PolicyStatus::default()).await;
        let last = c.report.steps.last().unwrap();
        assert_eq!((last.name, last.ok), ("auth", false));
        assert_eq!(
            last.error.as_ref().unwrap().key,
            "errors.history.unauthorized"
        );
        assert!(!c.report.ok);
    }

    #[tokio::test]
    async fn no_source_and_bad_config_fail_the_first_step() {
        let c = check_source(&sample_profile("a"), None, &PolicyStatus::default()).await;
        assert_eq!(
            c.report.steps[0].error.as_ref().unwrap().key,
            "errors.history.noSource"
        );
        let mut p = sample_profile("a");
        p.history = Some(source("prom:9090", HistoryAuth::None));
        let c = check_source(&p, None, &PolicyStatus::default()).await;
        assert_eq!(
            c.report.steps[0].error.as_ref().unwrap().key,
            "errors.history.badUrl"
        );
    }

    #[test]
    fn targets_are_typed_by_the_sglang_query() {
        let v = |name: &str| {
            let data: serde_json::Value = serde_json::from_str(&snapshot(name)).unwrap();
            crate::history::client::parse_vector(&data["data"]).unwrap()
        };
        let t = targets_from(&v("discover.json"), &v("discover-sglang.json"));
        assert_eq!(
            t.iter()
                .map(|t| (t.job.as_str(), t.kind))
                .collect::<Vec<_>>(),
            [
                ("sglang", TargetKind::Sglang),
                ("vllm", TargetKind::Vllm),
                ("vllm", TargetKind::Vllm),
                ("vllm", TargetKind::Vllm)
            ]
        );
    }
}
````

<!-- block: patch t06-src-tauri-src-history-mod-rs step=6.1 -->
````
diff --git a/src-tauri/src/history/mod.rs b/src-tauri/src/history/mod.rs
index 4ff3ee8..e1bac46 100644
--- a/src-tauri/src/history/mod.rs
+++ b/src-tauri/src/history/mod.rs
@@ -1,8 +1,10 @@
 //! История метрик из Prometheus или VictoriaMetrics (спека I1c): клиент источника, цели и
 //! селекторы серий, шаблоны PromQL и окна.
 
+pub mod check;
 pub mod client;
 pub mod promql;
+pub mod read;
 pub mod series;
 pub mod targets;
 pub mod window;
````

<!-- block: file t06-src-tauri-src-history-read-rs-tests step=6.1 path=src-tauri/src/history/read.rs part=tests -->
````
#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::client::tests::{snapshot, source};
    use crate::policy::PolicyStatus;
    use crate::servers::client::Timeouts;
    use crate::servers::testing::{Reply, TestServer};
    use crate::storage::profile::HistoryAuth;

    const TIMEOUTS: Timeouts = Timeouts {
        connect_ms: 2_000,
        request_ms: 2_000,
    };

    /// Путь запроса окна, как его строит клиент.
    fn range_path(expr: &str, start: u64, end: u64, step: u64) -> String {
        let q = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs([
                ("query", expr),
                ("start", &start.to_string()),
                ("end", &end.to_string()),
                ("step", &step.to_string()),
            ])
            .finish();
        format!("/api/v1/query_range?{q}")
    }

    /// Тестовый источник, который отвечает снятыми ответами на запросы окна `now` цели.
    async fn captured_source(name: &str, broken: Option<&str>) -> (TestServer, u64) {
        let doc: serde_json::Value =
            serde_json::from_str(&snapshot(&format!("range-{name}.json"))).unwrap();
        let (start, end) = (doc["start"].as_u64().unwrap(), doc["end"].as_u64().unwrap());
        let paths: Vec<(String, Reply)> = doc["queries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|q| {
                let expr = q["expr"].as_str().unwrap();
                let reply = if broken.is_some_and(|b| expr.contains(b)) {
                    Reply {
                        status: 422,
                        ..Reply::json(
                            r#"{"status":"error","errorType":"execution","error":"boom"}"#,
                        )
                    }
                } else {
                    Reply::json(&q["response"].to_string())
                };
                (range_path(expr, start, end, 15), reply)
            })
            .collect();
        let mut routes: Vec<(&str, Reply)> =
            paths.iter().map(|(p, r)| (p.as_str(), r.clone())).collect();
        let targets = snapshot("targets.json");
        routes.push(("/api/v1/targets", Reply::json(&targets)));
        (TestServer::start(routes).await, end)
    }

    fn client(server: &TestServer) -> HistoryClient {
        HistoryClient::new(
            &source(&server.url(), HistoryAuth::None),
            TIMEOUTS,
            None,
            &PolicyStatus::default(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn now_window_of_a_vllm_server() {
        let (server, end) = captured_source("vllm-8000", None).await;
        let a = read_history(
            &client(&server),
            SourceKind::Prometheus,
            DetectedKind::Vllm,
            r#"{job="vllm", instance="127.0.0.1:8000"}"#,
            Window::Now,
            end + 7,
        )
        .await
        .unwrap();
        assert_eq!((a.step_sec, a.points.len()), (15, 61));
        assert!(a.partial.is_empty(), "{:?}", a.partial);
        let rates = a.points[60].m.rates.clone().unwrap();
        assert!((rates.prompt_tokens.unwrap() - 20.82).abs() < 0.05);
        let json = serde_json::to_value(&a).unwrap();
        assert_eq!(json["window"], "now");
        assert_eq!(json["source"], "prometheus");
        assert_eq!(json["points"][60]["at"], json["points"][60]["m"]["at"]);
        // Интервал опроса цели — 5 с: rate по минуте, и все 20 запросов окна совпали со снятыми
        // (иначе были бы `partial`); плюс один запрос targets.
        let paths = server.paths();
        assert_eq!(
            paths.iter().filter(|p| p.contains("query_range")).count(),
            20
        );
        assert_eq!(paths.len(), 21);
    }

    #[tokio::test]
    async fn failed_query_is_partial_and_all_failed_is_an_error() {
        let (server, end) = captured_source("vllm-8000", Some("time_to_first_token")).await;
        let a = read_history(
            &client(&server),
            SourceKind::Prometheus,
            DetectedKind::Vllm,
            r#"{job="vllm", instance="127.0.0.1:8000"}"#,
            Window::Now,
            end,
        )
        .await
        .unwrap();
        assert_eq!(a.partial.len(), 1);
        assert_eq!(a.partial[0].chart, "ttft");
        assert_eq!(a.partial[0].error.key, "errors.history.failed");

        // Другой селектор — у теста нет ни одного ответа: все запросы — 404, окно — ошибка.
        let e = read_history(
            &client(&server),
            SourceKind::Prometheus,
            DetectedKind::Vllm,
            r#"{job="other"}"#,
            Window::Now,
            end,
        )
        .await
        .unwrap_err();
        assert_eq!(e.key, "errors.history.status");
    }

    #[tokio::test]
    async fn openai_and_bad_selector_are_errors() {
        let server = TestServer::start(vec![]).await;
        let read = |kind, sel: &'static str| {
            let c = client(&server);
            async move { read_history(&c, SourceKind::Prometheus, kind, sel, Window::Hour, 0).await }
        };
        assert_eq!(
            read(DetectedKind::Openai, r#"{job="x"}"#)
                .await
                .unwrap_err()
                .key,
            "errors.history.noMetrics"
        );
        assert_eq!(
            read(DetectedKind::Vllm, "job=x").await.unwrap_err().key,
            "errors.history.selector"
        );
        assert!(server.paths().is_empty());
    }
}
````

<!-- block: patch t06-src-tauri-src-lib-rs step=6.1 -->
````
diff --git a/src-tauri/src/lib.rs b/src-tauri/src/lib.rs
index d22004a..44d21ed 100644
--- a/src-tauri/src/lib.rs
+++ b/src-tauri/src/lib.rs
@@ -55,6 +55,8 @@ pub fn run() {
             commands::installations::disconnect_installation,
             commands::servers::read_server_metrics,
             commands::servers::read_server_config,
+            commands::history::check_history_source,
+            commands::history::read_server_history,
             commands::folders::list_folders,
             commands::folders::create_folder,
             commands::folders::rename_folder,
````

- [ ] **Шаг 6.2: Увидеть падение.**

Run: `cd src-tauri && cargo test --lib -q history`
Expected: FAIL — не компилируется: тестам не хватает типов и функций, которые появятся в шаге 3

- [ ] **Шаг 6.3: Реализация.** Проверка, чтение окна, кеш источника в пуле, команды и тексты шагов.

<!-- block: file t06-src-tauri-src-commands-history-rs step=6.3 path=src-tauri/src/commands/history.rs part=impl -->
````
//! Команды истории метрик (спека I1c §3–4): проверка источника из формы и окно истории сервера.

use std::sync::Arc;

use tauri::State;

use super::blocking;
use super::installations::InstallationInput;
use crate::error::{AppResult, ErrorCode};
use crate::history::check::{HistoryCheck, check_source, no_source};
use crate::history::client::{HistoryClient, SourceKind};
use crate::history::read::{HistoryAnswer, Source, read_history};
use crate::history::window::Window;
use crate::secrets::history_secret_key;
use crate::servers::client::Timeouts;
use crate::servers::live::now_ms;
use crate::state::AppState;

/// Проверяет источник истории несохранённого профиля: секрет из формы накладывается поверх
/// keychain (с той же семантикой, что и при сохранении).
#[tauri::command]
pub async fn check_history_source(
    state: State<'_, AppState>,
    input: InstallationInput,
) -> AppResult<HistoryCheck> {
    let st = state.inner().clone();
    let secret = match input.secrets.history_secret.clone() {
        Some(v) => Some(v),
        None => {
            let key = history_secret_key(&input.profile.id);
            blocking(move || st.secrets.get(&key)).await?
        }
    };
    Ok(check_source(&input.profile, secret, &state.policy.status).await)
}

/// Окно истории сервера: `now` (15 минут для «Сейчас»), `1h`, `24h`, `7d`, `30d`.
#[tauri::command]
pub async fn read_server_history(
    state: State<'_, AppState>,
    installation_id: String,
    server_id: String,
    window: Window,
) -> AppResult<HistoryAnswer> {
    read_server_history_inner(&state, &installation_id, &server_id, window).await
}

/// Клиент источника инсталляции из пула; нет — создаётся (секрет из keychain, тип источника по
/// `buildinfo`) и кладётся, если инсталляцию за это время не сбросили.
async fn history_source(st: &AppState, installation_id: &str) -> AppResult<Arc<Source>> {
    if let Some(s) = st.pool.history(installation_id) {
        return Ok(s);
    }
    let generation = st.pool.generation(installation_id);
    let profile = st.installations.get(installation_id)?;
    let h = profile.history.clone().ok_or_else(no_source)?;
    let secret = {
        let (st, key) = (st.clone(), history_secret_key(installation_id));
        blocking(move || st.secrets.get(&key)).await?
    };
    let client = HistoryClient::new(&h, Timeouts::from(&profile), secret, &st.policy.status)?;
    let kind = client
        .buildinfo()
        .await?
        .map_or(SourceKind::Prometheus, |b| b.kind);
    let source = Arc::new(Source { client, kind });
    st.pool
        .insert_history_if_current(installation_id, generation, source.clone());
    Ok(source)
}

pub async fn read_server_history_inner(
    st: &AppState,
    installation_id: &str,
    server_id: &str,
    window: Window,
) -> AppResult<HistoryAnswer> {
    let profile = st.installations.get(installation_id)?;
    if profile.history.is_none() {
        return Err(no_source());
    }
    let selector = profile
        .server(server_id)
        .and_then(|s| s.history_selector.clone())
        .ok_or_else(|| {
            app_err!(
                ErrorCode::InvalidInput,
                "errors.history.notMatched",
                "The series of the server are not matched with the metrics history: choose a target in the installation"
            )
        })?;
    let (_, kind) = super::servers::connected_server(st, installation_id, server_id).await?;
    let source = history_source(st, installation_id).await?;
    read_history(
        &source.client,
        source.kind,
        kind,
        &selector,
        window,
        now_ms() / 1000,
    )
    .await
}
````

<!-- block: patch t06-src-tauri-src-commands-servers-rs step=6.3 -->
````
diff --git a/src-tauri/src/commands/servers.rs b/src-tauri/src/commands/servers.rs
index 6da5375..6f028f1 100644
--- a/src-tauri/src/commands/servers.rs
+++ b/src-tauri/src/commands/servers.rs
@@ -37,7 +37,7 @@ pub async fn read_server_config(
 }
 
 /// Клиент и тип подключённого сервера; не подключился — его ошибка подключения.
-async fn connected_server(
+pub(crate) async fn connected_server(
     state: &AppState,
     installation_id: &str,
     server_id: &str,
````

<!-- block: file t06-src-tauri-src-history-check-rs step=6.3 path=src-tauri/src/history/check.rs part=impl -->
````
//! Проверка источника истории (спека I1c §3): конфигурация → DNS/TCP → вход (`buildinfo`) →
//! запрос `up` → цели инференса → сопоставление каждого сервера. Сбой источника не мешает
//! подключению серверов: проверка только сообщает его.

use std::time::Duration;

use serde::Serialize;

use crate::error::{AppError, ErrorCode};
use crate::history::client::{BuildInfo, HistoryClient, Sample};
use crate::history::targets::{Target, TargetKind, matching_targets, parse_selector, render};
use crate::i18n::LocalizedText;
use crate::policy::PolicyStatus;
use crate::servers::client::Timeouts;
use crate::servers::connectivity::{
    TestReport, TestStep, failed_step, passed, report, tcp_failure, tcp_probe, tcp_target,
};
use crate::storage::profile::{
    InstallationProfile, ServerConfig, parse_base_url, parse_history_url, validate_history,
};

/// Запрос целей инференса: пары `job`, `instance` с сериями vLLM или SGLang.
pub const DISCOVER: &str = r#"count by (job, instance) ({__name__=~"vllm:.+|sglang:.+"})"#;
/// Те же цели SGLang: остальные — vLLM.
pub const DISCOVER_SGLANG: &str = r#"count by (job, instance) ({__name__=~"sglang:.+"})"#;

/// Селектор, который форма может поставить серверу.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub server_id: String,
    /// Селектор подобранной цели; `None` — цель не найдена.
    pub selector: Option<String>,
    /// Подходящие цели (по `host:port`), первая — подобранная.
    pub targets: Vec<Target>,
}

/// Итог проверки для формы и проверки инсталляции.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryCheck {
    pub report: TestReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<BuildInfo>,
    /// Найденные цели инференса (пусто, если до шага поиска проверка не дошла).
    pub targets: Vec<Target>,
    /// Автоподбор для серверов без селектора.
    pub suggestions: Vec<Suggestion>,
}

/// Цели из ответов поиска: все и цели SGLang.
pub fn targets_from(all: &[Sample], sglang: &[Sample]) -> Vec<Target> {
    let pair = |s: &Sample| {
        Some((
            s.labels.get("job")?.clone(),
            s.labels.get("instance")?.clone(),
        ))
    };
    let sglang: Vec<_> = sglang.iter().filter_map(pair).collect();
    let mut out: Vec<Target> = all
        .iter()
        .filter_map(pair)
        .map(|(job, instance)| {
            let kind = if sglang.contains(&(job.clone(), instance.clone())) {
                TargetKind::Sglang
            } else {
                TargetKind::Vllm
            };
            Target {
                job,
                instance,
                kind,
            }
        })
        .collect();
    out.sort();
    out
}

fn warning_step(name: &'static str, message: LocalizedText) -> TestStep {
    TestStep {
        name,
        ok: true,
        message,
        error: None,
    }
}

/// Шаг сопоставления сервера: у сервера с селектором — есть ли под ним серии; без селектора —
/// подобранная цель.
async fn match_step(
    client: &HistoryClient,
    server: &ServerConfig,
    targets: &[Target],
    warnings: &mut Vec<LocalizedText>,
) -> (TestStep, Option<Suggestion>) {
    let name = &server.name;
    if let Some(selector) = &server.history_selector {
        let matchers = match parse_selector(selector) {
            Ok(m) => m,
            Err(e) => return (failed_step("match", e), None),
        };
        let expr = format!(
            r#"count({{{},__name__=~"vllm:.+|sglang:.+"}})"#,
            render(&matchers)
        );
        return match client.query(&expr).await {
            Ok(found) if found.iter().any(|s| s.value.is_some_and(|v| v > 0.0)) => (
                passed(
                    "match",
                    ltext!(
                        "messages.historyTest.matchSelector",
                        "“{{server}}”: {{selector}}",
                        server = name,
                        selector = selector
                    ),
                ),
                None,
            ),
            Ok(_) => {
                let w = ltext!(
                    "messages.historyTest.noSeries",
                    "“{{server}}”: there are no vLLM or SGLang series under the selector {{selector}}",
                    server = name,
                    selector = selector
                );
                warnings.push(w.clone());
                (warning_step("match", w), None)
            }
            Err(e) => (failed_step("match", e), None),
        };
    }
    let found: Vec<Target> = parse_base_url(server)
        .map(|url| {
            matching_targets(&url, targets)
                .into_iter()
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    match found.first() {
        Some(t) => (
            passed(
                "match",
                ltext!(
                    "messages.historyTest.matchAuto",
                    "“{{server}}”: {{job}} · {{instance}} (auto-match)",
                    server = name,
                    job = t.job,
                    instance = t.instance
                ),
            ),
            Some(Suggestion {
                server_id: server.id.clone(),
                selector: Some(t.selector()),
                targets: found.clone(),
            }),
        ),
        None => {
            let w = ltext!(
                "messages.historyTest.noTarget",
                "“{{server}}”: no target with the address of the server — choose the target manually",
                server = name
            );
            warnings.push(w.clone());
            (
                warning_step("match", w),
                Some(Suggestion {
                    server_id: server.id.clone(),
                    selector: None,
                    targets: Vec::new(),
                }),
            )
        }
    }
}

/// Проверка источника истории профиля `p`; `secret` — токен или пароль из формы или keychain.
pub async fn check_source(
    p: &InstallationProfile,
    secret: Option<String>,
    policy: &PolicyStatus,
) -> HistoryCheck {
    let finish = |steps, warnings, source, targets, suggestions| HistoryCheck {
        report: report(steps, warnings),
        source,
        targets,
        suggestions,
    };
    let Some(h) = &p.history else {
        let e = app_err!(
            ErrorCode::InvalidInput,
            "errors.history.noSource",
            "The installation has no metrics history source"
        );
        return finish(
            vec![failed_step("config", e)],
            Vec::new(),
            None,
            Vec::new(),
            Vec::new(),
        );
    };
    let timeouts = Timeouts::from(p);
    let prepared = validate_history(h)
        .and_then(|()| parse_history_url(h))
        .and_then(|url| Ok((url, HistoryClient::new(h, timeouts, secret, policy)?)));
    let (url, client) = match prepared {
        Ok(x) => x,
        Err(e) => {
            return finish(
                vec![failed_step("config", e)],
                Vec::new(),
                None,
                Vec::new(),
                Vec::new(),
            );
        }
    };
    let mut steps = vec![passed(
        "config",
        ltext!("messages.connectionTest.configOk", "Configuration is valid"),
    )];
    let mut warnings = Vec::new();

    let as_server = crate::history::client::as_server(h);
    if let Some((host, port, via_proxy)) = tcp_target(&as_server, &url) {
        let limit = Duration::from_millis(timeouts.connect_ms);
        match tcp_probe(&host, port, limit).await {
            Ok(address) => steps.push(passed(
                "tcp",
                ltext!(
                    "messages.connectionTest.tcpOk",
                    "TCP connection to {{address}} established",
                    address = address
                ),
            )),
            Err(e) => {
                steps.push(failed_step("tcp", tcp_failure(e, &host, port, via_proxy)));
                return finish(steps, warnings, None, Vec::new(), Vec::new());
            }
        }
    }

    let source = match client.buildinfo().await {
        Ok(info) => {
            let message = match &info {
                Some(BuildInfo {
                    version: Some(v), ..
                }) => ltext!(
                    "messages.historyTest.source",
                    "{{kind}} {{version}}",
                    kind = source_label(info.as_ref()),
                    version = v
                ),
                _ => ltext!(
                    "messages.historyTest.noBuildinfo",
                    "The source has no buildinfo — sign-in is checked by the next step"
                ),
            };
            steps.push(passed("auth", message));
            info
        }
        Err(e) => {
            let step = if e.code == ErrorCode::TlsError {
                "tls"
            } else {
                "auth"
            };
            steps.push(failed_step(step, e));
            return finish(steps, warnings, None, Vec::new(), Vec::new());
        }
    };

    match client.query("up").await {
        Ok(up) => {
            let alive = up.iter().filter(|s| s.value == Some(1.0)).count();
            steps.push(passed(
                "up",
                ltext!(
                    "messages.historyTest.up",
                    "Targets up: {{alive}} of {{total}}",
                    alive = alive,
                    total = up.len()
                ),
            ));
        }
        Err(e) => {
            steps.push(failed_step("up", e));
            return finish(steps, warnings, source, Vec::new(), Vec::new());
        }
    }

    let targets = match (
        client.query(DISCOVER).await,
        client.query(DISCOVER_SGLANG).await,
    ) {
        (Ok(all), Ok(sglang)) => targets_from(&all, &sglang),
        (Err(e), _) | (_, Err(e)) => {
            steps.push(failed_step("targets", e));
            return finish(steps, warnings, source, Vec::new(), Vec::new());
        }
    };
    if targets.is_empty() {
        let w = ltext!(
            "messages.historyTest.noTargets",
            "The source does not collect vLLM or SGLang metrics"
        );
        warnings.push(w.clone());
        steps.push(warning_step("targets", w));
    } else {
        steps.push(passed(
            "targets",
            ltext!(
                "messages.historyTest.targets",
                "Inference targets found: {{count}}",
                count = targets.len()
            ),
        ));
    }

    let mut suggestions = Vec::new();
    for server in &p.servers {
        let (step, suggestion) = match_step(&client, server, &targets, &mut warnings).await;
        steps.push(step);
        suggestions.extend(suggestion);
    }
    finish(steps, warnings, source, targets, suggestions)
}

/// Название источника для шага «Вход».
fn source_label(info: Option<&BuildInfo>) -> &'static str {
    match info.map(|i| i.kind) {
        Some(crate::history::client::SourceKind::Victoriametrics) => "VictoriaMetrics",
        _ => "Prometheus",
    }
}

/// Ошибка «нет источника истории» — для команд чтения.
pub fn no_source() -> AppError {
    app_err!(
        ErrorCode::InvalidInput,
        "errors.history.noSource",
        "The installation has no metrics history source"
    )
}
````

<!-- block: patch t06-src-tauri-src-history-client-rs step=6.3 -->
````
diff --git a/src-tauri/src/history/client.rs b/src-tauri/src/history/client.rs
index a5ab001..25d8461 100644
--- a/src-tauri/src/history/client.rs
+++ b/src-tauri/src/history/client.rs
@@ -275,6 +275,22 @@ pub fn scrape_interval_of(data: &Value, job: &str, instance: &str) -> Option<u64
         .and_then(parse_duration)
 }
 
+/// Источник истории как адрес с настройками соединения: так его открывают общий HTTP-клиент и
+/// шаг «DNS/TCP» проверки.
+pub fn as_server(h: &HistorySource) -> ServerConfig {
+    ServerConfig {
+        id: crate::secrets::HISTORY.into(),
+        name: HISTORY_NAME.into(),
+        base_url: h.url.clone(),
+        kind: ServerKind::Auto,
+        metrics_path: "/metrics".into(),
+        ca: h.ca.clone(),
+        proxy: h.proxy.clone(),
+        history_selector: None,
+        extra: Default::default(),
+    }
+}
+
 impl HistoryClient {
     /// Клиент источника `h`; `secret` — токен или пароль из формы или keychain.
     pub fn new(
@@ -284,17 +300,7 @@ impl HistoryClient {
         policy: &PolicyStatus,
     ) -> AppResult<Self> {
         parse_history_url(h)?;
-        let server = ServerConfig {
-            id: crate::secrets::HISTORY.into(),
-            name: HISTORY_NAME.into(),
-            base_url: h.url.clone(),
-            kind: ServerKind::Auto,
-            metrics_path: "/metrics".into(),
-            ca: h.ca.clone(),
-            proxy: h.proxy.clone(),
-            history_selector: None,
-            extra: Default::default(),
-        };
+        let server = as_server(h);
         let secret = secret.filter(|s| !s.is_empty());
         let http = match (&h.auth, secret) {
             (HistoryAuth::Bearer, token) => ServerClient::new(&server, timeouts, token, policy)?,
@@ -315,7 +321,7 @@ impl HistoryClient {
     /// Тип и версия источника; `None` — у источника нет `buildinfo` (404).
     pub async fn buildinfo(&self) -> AppResult<Option<BuildInfo>> {
         let path = "/api/v1/status/buildinfo";
-        let reply = self.http.get_query(path, &[], METRICS_LIMIT).await?;
+        let reply = self.http.get(path, METRICS_LIMIT).await?;
         if reply.status == 404 {
             return Ok(None);
         }
````

<!-- block: file t06-src-tauri-src-history-read-rs step=6.3 path=src-tauri/src/history/read.rs part=impl -->
````
//! Окно истории сервера (спека I1c §4): шаблоны по карте адаптера, не больше 4 запросов к
//! источнику одновременно, сборка точек; сбой одного запроса — `partial`, всех — ошибка.

use futures::StreamExt;
use serde::Serialize;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::history::client::{HistoryClient, SourceKind};
use crate::history::promql::queries;
use crate::history::series::{HistoryPoint, Partial, assemble};
use crate::history::targets::{MatchOp, parse_selector, render};
use crate::history::window::{Window, duration, range};
use crate::servers::adapters::adapter;
use crate::servers::detect::DetectedKind;

/// Сколько запросов к источнику идёт одновременно (спека I1c §4).
pub const CONCURRENCY: usize = 4;

/// Ответ `read_server_history`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryAnswer {
    pub window: Window,
    pub step_sec: u64,
    pub source: SourceKind,
    pub points: Vec<HistoryPoint>,
    pub partial: Vec<Partial>,
}

/// Открытый источник истории инсталляции: клиент и тип по `buildinfo`. Живёт в пуле подключений
/// до `evict` инсталляции.
#[derive(Debug)]
pub struct Source {
    pub client: HistoryClient,
    pub kind: SourceKind,
}

/// Значение метки селектора с условием `=`: по `job` и `instance` ищется интервал опроса цели.
fn exact(matchers: &[crate::history::targets::Matcher], label: &str) -> Option<String> {
    matchers
        .iter()
        .find(|m| m.label == label && m.op == MatchOp::Eq)
        .map(|m| m.value.clone())
}

/// Окно `window` сервера типа `kind` с селектором `selector`, которое кончается в `now_sec`.
pub async fn read_history(
    client: &HistoryClient,
    source: SourceKind,
    kind: DetectedKind,
    selector: &str,
    window: Window,
    now_sec: u64,
) -> AppResult<HistoryAnswer> {
    let map = adapter(kind).metric_map();
    if map.is_empty() {
        return Err(app_err!(
            ErrorCode::InvalidInput,
            "errors.history.noMetrics",
            "Glafira reads the metrics history only of vLLM and SGLang"
        ));
    }
    let matchers = parse_selector(selector)?;
    let scrape = match (exact(&matchers, "job"), exact(&matchers, "instance")) {
        (Some(job), Some(instance)) => client.scrape_interval(&job, &instance).await,
        _ => None,
    };
    let r = range(window, now_sec, scrape);
    let qs = queries(map, &render(&matchers), &duration(r.rate));
    let results: Vec<_> = futures::stream::iter(qs)
        .map(|q| async move {
            let result = client.query_range(&q.expr, &r).await;
            (q, result)
        })
        .buffered(CONCURRENCY)
        .collect()
        .await;
    if results.iter().all(|(_, res)| res.is_err()) {
        let first: Option<AppError> = results.into_iter().find_map(|(_, res)| res.err());
        return Err(first.unwrap_or_else(|| AppError::internal_with("no history queries")));
    }
    let (points, partial) = assemble(&r, results);
    Ok(HistoryAnswer {
        window,
        step_sec: r.step,
        source,
        points,
        partial,
    })
}
````

<!-- block: patch t06-src-tauri-src-servers-connectivity-rs step=6.3 -->
````
diff --git a/src-tauri/src/servers/connectivity.rs b/src-tauri/src/servers/connectivity.rs
index 7289848..2dc162a 100644
--- a/src-tauri/src/servers/connectivity.rs
+++ b/src-tauri/src/servers/connectivity.rs
@@ -36,7 +36,7 @@ pub struct TestReport {
     pub warnings: Vec<LocalizedText>,
 }
 
-fn passed(name: &'static str, message: LocalizedText) -> TestStep {
+pub(crate) fn passed(name: &'static str, message: LocalizedText) -> TestStep {
     TestStep {
         name,
         ok: true,
@@ -45,7 +45,7 @@ fn passed(name: &'static str, message: LocalizedText) -> TestStep {
     }
 }
 
-fn failed_step(name: &'static str, e: AppError) -> TestStep {
+pub(crate) fn failed_step(name: &'static str, e: AppError) -> TestStep {
     TestStep {
         name,
         ok: false,
@@ -54,7 +54,7 @@ fn failed_step(name: &'static str, e: AppError) -> TestStep {
     }
 }
 
-fn report(steps: Vec<TestStep>, warnings: Vec<LocalizedText>) -> TestReport {
+pub(crate) fn report(steps: Vec<TestStep>, warnings: Vec<LocalizedText>) -> TestReport {
     TestReport {
         ok: steps.iter().all(|s| s.ok),
         steps,
@@ -139,7 +139,7 @@ pub async fn tcp_probe(host: &str, port: u16, limit: Duration) -> AppResult<Stri
 /// Ошибка шага «DNS/TCP» для `host:port`, к которому подключались. При явном прокси это
 /// адрес прокси, а не сервера: ошибка называет прокси, а не винит имя или порт сервера.
 /// Адрес дописывается в `details` и в этом случае, и без прокси.
-fn tcp_failure(e: AppError, host: &str, port: u16, via_proxy: bool) -> AppError {
+pub(crate) fn tcp_failure(e: AppError, host: &str, port: u16, via_proxy: bool) -> AppError {
     let address = format!("{host}:{port}");
     let details = match &e.details {
         Some(d) => format!("{d}\n{address}"),
````

<!-- block: patch t06-src-tauri-src-servers-pool-rs step=6.3 -->
````
diff --git a/src-tauri/src/servers/pool.rs b/src-tauri/src/servers/pool.rs
index 3028ead..53f6096 100644
--- a/src-tauri/src/servers/pool.rs
+++ b/src-tauri/src/servers/pool.rs
@@ -59,6 +59,8 @@ pub struct ServerPool {
     locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
     /// Снимки метрик серверов за последнюю минуту (спец. I1b §4), по (инсталляция, сервер).
     live: Mutex<HashMap<(String, String), LiveHistory>>,
+    /// Открытые источники истории метрик (спека I1c §4), по инсталляции.
+    history: Mutex<HashMap<String, Arc<crate::history::read::Source>>>,
 }
 
 impl ServerPool {
@@ -79,6 +81,28 @@ impl ServerPool {
         self.live
             .lock_safe()
             .retain(|(installation, _), _| installation != id);
+        self.history.lock_safe().remove(id);
+    }
+
+    /// Открытый источник истории инсталляции; `evict` его забывает.
+    pub fn history(&self, id: &str) -> Option<Arc<crate::history::read::Source>> {
+        self.history.lock_safe().get(id).cloned()
+    }
+
+    /// Кладёт источник истории, только если с момента `generation` инсталляцию не сбрасывали:
+    /// иначе он открыт по устаревшим профилю и секрету.
+    pub fn insert_history_if_current(
+        &self,
+        id: &str,
+        generation: u64,
+        source: Arc<crate::history::read::Source>,
+    ) -> bool {
+        let generations = self.generations.lock_safe();
+        if generations.get(id).copied().unwrap_or(0) != generation {
+            return false;
+        }
+        self.history.lock_safe().insert(id.to_string(), source);
+        true
     }
 
     /// История снимков метрик сервера; `evict` инсталляции её сбрасывает.
````

<!-- block: locales t06-locales-t06 step=6.3 -->
````
{
  "en": {
    "errors": { "history": {
      "noMetrics": "Glafira reads the metrics history only of vLLM and SGLang",
      "noSource": "The installation has no metrics history source",
      "notMatched": "The series of the server are not matched with the metrics history: choose a target in the installation"
    } },
    "messages": { "historyTest": {
      "source": "{{kind}} {{version}}",
      "noBuildinfo": "The source has no buildinfo — sign-in is checked by the next step",
      "up": "Targets up: {{alive}} of {{total}}",
      "targets": "Inference targets found: {{count}}",
      "noTargets": "The source does not collect vLLM or SGLang metrics",
      "matchAuto": "“{{server}}”: {{job}} · {{instance}} (auto-match)",
      "matchSelector": "“{{server}}”: {{selector}}",
      "noSeries": "“{{server}}”: there are no vLLM or SGLang series under the selector {{selector}}",
      "noTarget": "“{{server}}”: no target with the address of the server — choose the target manually"
    } }
  },
  "ru": {
    "errors": { "history": {
      "noMetrics": "Историю метрик Glafira читает только у vLLM и SGLang",
      "noSource": "У инсталляции нет источника истории метрик",
      "notMatched": "Серии сервера не сопоставлены с историей метрик: выберите цель в инсталляции"
    } },
    "messages": { "historyTest": {
      "source": "{{kind}} {{version}}",
      "noBuildinfo": "У источника нет buildinfo — вход проверит следующий шаг",
      "up": "Целей в работе: {{alive}} из {{total}}",
      "targets": "Найдены цели инференса: {{count}}",
      "noTargets": "Источник не собирает метрики vLLM и SGLang",
      "matchAuto": "«{{server}}»: {{job}} · {{instance}} (автоподбор)",
      "matchSelector": "«{{server}}»: {{selector}}",
      "noSeries": "«{{server}}»: под селектором {{selector}} нет серий vLLM и SGLang",
      "noTarget": "«{{server}}»: цели с адресом сервера нет — выберите цель вручную"
    } }
  }
}
````

- [ ] **Шаг 6.4: Проверки.**

Run: `cd src-tauri && cargo test --lib -q`
Expected: `test result: ok. 283 passed`
Run: `cd src-tauri && cargo clippy --lib --tests -q -- -D warnings && cargo fmt --check`
Expected: без вывода
Run: `pnpm i18n:check`
Expected: `✓ словари en, ru согласованы, вызовов из Rust: 139`

- [ ] **Шаг 6.5: Коммит.**

<!-- block: shell t06-commit step=6.5 -->
````
git add src-tauri/src/commands/history.rs \
  src-tauri/src/commands/mod.rs \
  src-tauri/src/commands/servers.rs \
  src-tauri/src/history/check.rs \
  src-tauri/src/history/client.rs \
  src-tauri/src/history/mod.rs \
  src-tauri/src/history/read.rs \
  src-tauri/src/lib.rs \
  src-tauri/src/servers/connectivity.rs \
  src-tauri/src/servers/pool.rs \
  src/locales/en.json \
  src/locales/ru.json
git commit -s -q -F - <<'EOF'
feat(history): check_history_source and read_server_history commands

The source check runs configuration, DNS/TCP, sign-in (buildinfo), `up`,
target discovery and one matching step per server, and returns found
targets with auto-match suggestions. A history window runs the adapter
templates with at most four concurrent queries: a failed query is partial,
all failed is an error. The opened source is cached in the pool until the
installation is evicted.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
````

---

### Задача 7: Мок prometheus и конфиг Prometheus для смоука

Вид `prometheus` мок-сервера (спека I1c §6): `up`, поиск целей и проверка селектора — ответами настоящего
Prometheus, `query_range` шаблонов Glafira — снятыми рядами на сетке запроса (для любого селектора). Отказы:
`--api-key` (без токена — 401), `--slow N`, `--huge` (ответ больше 8 МБ), выражение с `__bad__` — `bad_data`.
`scripts/prometheus/smoke.yml` — Prometheus, который опрашивает моки раз в 5 с. README описывает вид и конфиг.

**Files:**
- Create: `scripts/prometheus/smoke.yml`
- Modify: `README.md`
- Modify: `scripts/mock-server.mjs`
- Modify: `scripts/mock-server.test.mjs`

**Interfaces:**
- Produces: `prometheusReply(url, opts)` (экспорт `scripts/mock-server.mjs`); параметры `--slow 1..600`, `--huge`;
  вид `prometheus` (`KINDS.prometheus.prometheus = true`); `scripts/prometheus/smoke.yml`.

- [ ] **Шаг 7.1: Тесты.** Тесты мока: параметры, ответы prometheus, сетка `query_range`, токен и задержка.

<!-- block: patch t07-scripts-mock-server-test-mjs step=7.1 -->
````
diff --git a/scripts/mock-server.test.mjs b/scripts/mock-server.test.mjs
index 741a735..472a043 100644
--- a/scripts/mock-server.test.mjs
+++ b/scripts/mock-server.test.mjs
@@ -2,7 +2,7 @@ import assert from "node:assert/strict";
 import { readFileSync } from "node:fs";
 import { createServer } from "node:http";
 import { test } from "node:test";
-import { handler, KINDS, liveMetrics, parseArgs } from "./mock-server.mjs";
+import { handler, KINDS, liveMetrics, parseArgs, prometheusReply } from "./mock-server.mjs";
 
 async function serve(opts) {
   const server = createServer(handler({ health: 200, apiKey: null, ...opts }));
@@ -12,7 +12,7 @@ async function serve(opts) {
 }
 
 test("параметры: значения по умолчанию, свои и ошибки", () => {
-  const defaults = { kind: "vllm", port: 8000, host: "127.0.0.1", apiKey: null, health: 200, models: "full", engines: 1, resetAfter: 0 };
+  const defaults = { kind: "vllm", port: 8000, host: "127.0.0.1", apiKey: null, health: 200, models: "full", engines: 1, resetAfter: 0, slow: 0, huge: false };
   assert.deepEqual(parseArgs([]), defaults);
   assert.deepEqual(
     parseArgs(["--kind", "sglang", "--port", "30000", "--api-key", "sk", "--health", "503", "--models", "empty", "--engines", "2", "--reset-after", "3"]),
@@ -20,6 +20,8 @@ test("параметры: значения по умолчанию, свои и
   );
   assert.throws(() => parseArgs(["--engines", "9"]), /--engines.*Использование/s);
   assert.throws(() => parseArgs(["--reset-after", "0"]), /--reset-after/);
+  assert.deepEqual(parseArgs(["--kind", "prometheus", "--huge", "--slow", "40"]), { ...defaults, kind: "prometheus", huge: true, slow: 40 });
+  assert.throws(() => parseArgs(["--slow", "0"]), /--slow/);
   assert.throws(() => parseArgs(["--kind", "tgi"]), /неизвестный вид/);
   assert.throws(() => parseArgs(["--verbose", "1"]), /неизвестный параметр/);
 });
@@ -152,7 +154,7 @@ test("ключ API у llama.cpp и Ollama — как у vLLM: только /v1",
 });
 
 test("--models empty: /v1/models отдаёт пустой список, остальное без изменений", async () => {
-  for (const kind of Object.keys(KINDS)) {
+  for (const kind of Object.keys(KINDS).filter((k) => !KINDS[k].prometheus)) {
     const s = await serve({ kind, models: "empty" });
     try {
       const r = await fetch(`${s.base}/v1/models`);
@@ -282,3 +284,53 @@ test("vllm-0.10: старые и новые имена рядом, два дви
     await s.close();
   }
 });
+
+
+const promql = (path, params) => new URL(`http://x${path}?${new URLSearchParams(params)}`);
+
+test("prometheus: служебные ответы, цели и сопоставление — ответами настоящего Prometheus", () => {
+  const body = (url) => JSON.parse(prometheusReply(url, {})[1]);
+  assert.equal(body(promql("/api/v1/query", { query: "up" })).data.result.length, 4);
+  const all = body(promql("/api/v1/query", { query: 'count by (job, instance) ({__name__=~"vllm:.+|sglang:.+"})' }));
+  assert.equal(all.data.result.length, 4);
+  const sglang = body(promql("/api/v1/query", { query: 'count by (job, instance) ({__name__=~"sglang:.+"})' }));
+  assert.deepEqual(sglang.data.result.map((r) => r.metric.job), ["sglang"]);
+  const one = body(promql("/api/v1/query", { query: 'count({job="vllm",__name__=~"vllm:.+|sglang:.+"})' }));
+  assert.equal(one.data.result[0].value[1], "57");
+  assert.deepEqual(body(promql("/api/v1/query", { query: "foo" })).data.result, []);
+  const [status, bad] = prometheusReply(promql("/api/v1/query_range", { query: 'sum(x{__bad__="1"})', start: 0, end: 60, step: 15 }), {});
+  assert.equal(status, 400);
+  assert.equal(JSON.parse(bad).errorType, "bad_data");
+});
+
+test("prometheus: query_range — снятый ряд шаблона на сетке запроса, для любого селектора", () => {
+  const expr = 'sum(rate(vllm:prompt_tokens_total{job="other",instance="10.0.0.5:8000"}[2m]))';
+  const [status, text] = prometheusReply(promql("/api/v1/query_range", { query: expr, start: 1000, end: 1180, step: 60 }), {});
+  assert.equal(status, 200);
+  const result = JSON.parse(text).data.result;
+  assert.equal(result.length, 1);
+  assert.deepEqual(result[0].values.map(([t]) => t), [1000, 1060, 1120, 1180]);
+  assert.ok(result[0].values.every(([, v]) => Number(v) > 0));
+  const reasons = JSON.parse(prometheusReply(promql("/api/v1/query_range", {
+    query: 'sum by (finished_reason) (rate(vllm:request_success_total{job="vllm"}[1m]))', start: 0, end: 30, step: 15,
+  }), {})[1]).data.result;
+  assert.ok(reasons.map((r) => r.metric.finished_reason).includes("stop"));
+  const unknown = JSON.parse(prometheusReply(promql("/api/v1/query_range", { query: "sum(nothing)", start: 0, end: 30, step: 15 }), {})[1]);
+  assert.deepEqual(unknown.data.result, []);
+  const huge = prometheusReply(promql("/api/v1/query_range", { query: expr, start: 0, end: 3600, step: 15 }), { huge: true })[1];
+  assert.ok(huge.length > 8 * 1024 * 1024, `${huge.length}`);
+});
+
+test("prometheus: --api-key закрывает всё, --slow задерживает ответ", async () => {
+  const s = await serve({ kind: "prometheus", apiKey: "prom-t0ken", slow: 1 });
+  try {
+    const started = Date.now();
+    assert.equal((await fetch(`${s.base}/api/v1/status/buildinfo`)).status, 401);
+    assert.ok(Date.now() - started >= 900, "ответ задержан");
+    const ok = await fetch(`${s.base}/api/v1/query?query=up`, { headers: { authorization: "Bearer prom-t0ken" } });
+    assert.equal(ok.status, 200);
+    assert.equal((await ok.json()).data.result.length, 4);
+  } finally {
+    await s.close();
+  }
+});
````

- [ ] **Шаг 7.2: Увидеть падение.**

Run: `node --test scripts/mock-server.test.mjs`
Expected: FAIL — `prometheusReply` не экспортируется (SyntaxError импорта)

- [ ] **Шаг 7.3: Реализация.** Вид prometheus, конфиг Prometheus, README.

<!-- block: patch t07-readme-md step=7.3 -->
````
diff --git a/README.md b/README.md
index 4820554..00861cb 100644
--- a/README.md
+++ b/README.md
@@ -53,11 +53,12 @@ cargo test --manifest-path src-tauri/Cargo.toml --lib     # Rust: unit-тест
 
 ## Мок-сервер
 `pnpm mock-server --kind <вид> [--port 8000] [--host 127.0.0.1] [--api-key sk-dev] [--health 200..599] [--models full|empty]
-[--engines 1..8] [--reset-after N]` отвечает как сервер инференса снимками ответов из `src-tauri/src/servers/testdata` —
+[--engines 1..8] [--reset-after N] [--slow N] [--huge]` отвечает как сервер инференса снимками ответов из `src-tauri/src/servers/testdata` —
 теми же, что проверяют тесты Rust. Виды: `vllm` (0.32, настоящий снимок), `vllm-0.11`, `vllm-0.10` (старые и новые имена
 метрик рядом, два движка), `vllm-0.8` (модель с адаптером LoRA; ниже границы поддержки), `vllm-old` (0.7), `sglang`
 (0.5.22), `sglang-no-metrics` (0.5.22 без `/metrics`, как без `--enable-metrics`), `sglang-0.5.4`, `sglang-0.4` (устаревшие пути `/get_*`, без метрик; ниже границы), `llamacpp`, `ollama`
-(без `/health`).
+(без `/health`), `prometheus` (источник истории метрик, ответами настоящего Prometheus 3.15.1 из
+`src-tauri/src/history/testdata`).
 - `/metrics` видов `vllm`, `vllm-0.10` и `sglang` «живой»: счётчики и гистограммы растут с каждым опросом, KV-кэш
   колеблется. `--engines N` — N движков data parallel, `--reset-after N` — после N опросов счётчики начинаются заново,
   как при перезапуске сервера.
@@ -66,6 +67,10 @@ cargo test --manifest-path src-tauri/Cargo.toml --lib     # Rust: unit-тест
 - `--health` — код ответа `/health`; у 3xx есть заголовок `Location` на `https://…/health`.
 - `--models empty` отдаёт пустой список моделей, `--models full` (по умолчанию) — снимок.
 - `--port 0` занимает любой свободный порт.
+- Вид `prometheus` — для отказов источника истории: с `--api-key` закрыто всё (без токена — 401), `--slow N` отвечает
+  через N секунд (таймаут), `--huge` отдаёт `query_range` больше 8 МБ, выражение с `__bad__` — ошибка `bad_data`.
+  `query_range` шаблонов Glafira отдаёт снятые ряды на сетке запроса. Настоящую историю даёт Prometheus с конфигом
+  `scripts/prometheus/smoke.yml`, который опрашивает моки раз в 5 с (`brew install prometheus`).
 
 ## Инсталляции и серверы
 - Инсталляция — серверы инференса одной системы (`storage/profile.rs`): имя, окружение, только чтение, таймауты и
````

<!-- block: patch t07-scripts-mock-server-mjs step=7.3 -->
````
diff --git a/scripts/mock-server.mjs b/scripts/mock-server.mjs
index 7216565..085067c 100755
--- a/scripts/mock-server.mjs
+++ b/scripts/mock-server.mjs
@@ -1,10 +1,11 @@
 #!/usr/bin/env node
 // Мок-сервер инференса для разработки и смоука без GPU: отвечает как vLLM, SGLang или OpenAI-совместимый
-// сервер снимками ответов из src-tauri/src/servers/testdata (те же снимки проверяют тесты Rust).
+// сервер снимками ответов из src-tauri/src/servers/testdata (те же снимки проверяют тесты Rust), а вид
+// prometheus — как источник истории, ответами настоящего Prometheus из src-tauri/src/history/testdata.
 // Только модули node:. Использование:
-//   node scripts/mock-server.mjs --kind vllm|vllm-0.11|vllm-0.10|vllm-0.8|vllm-old|sglang|sglang-no-metrics|sglang-0.5.4|sglang-0.4|llamacpp|ollama
+//   node scripts/mock-server.mjs --kind vllm|vllm-0.11|vllm-0.10|vllm-0.8|vllm-old|sglang|sglang-no-metrics|sglang-0.5.4|sglang-0.4|llamacpp|ollama|prometheus
 //        [--port 8000] [--host 127.0.0.1] [--api-key sk-dev]
-//        [--health 200..599] [--models full|empty] [--engines 1..8] [--reset-after N]
+//        [--health 200..599] [--models full|empty] [--engines 1..8] [--reset-after N] [--slow N] [--huge]
 //   --port    целое 0..65535 (0 — любой свободный порт).
 //   --health  код ответа /health, любое целое 200..599 (например 200, 301, 401, 503);
 //             у 3xx добавляется заголовок Location на https://<хост>:<порт>/health.
@@ -13,15 +14,20 @@
 //   KV-кэш колеблется. --engines N — N движков data parallel (vLLM — метка engine, SGLang — dp_rank;
 //   снимок, где уже несколько движков, например vllm-0.10, не размножается);
 //   --reset-after N — после N опросов /metrics счётчики начинаются заново, как при перезапуске сервера.
+//   Вид prometheus (смоук отказов источника истории): --api-key — токен (без него 401), --slow N — ответ через
+//   N секунд (таймаут), --huge — ответ query_range больше 8 МБ; выражение с __bad__ — ошибка bad_data.
+//   query_range отдаёт снятые ряды шаблонов Glafira на сетке запроса (значения по кругу), чужие — пусто.
 import { readFileSync } from "node:fs";
 import { createServer } from "node:http";
 import { join } from "node:path";
 import { fileURLToPath } from "node:url";
 
-const DATA = join(fileURLToPath(new URL("..", import.meta.url)), "src-tauri/src/servers/testdata");
+const ROOT = fileURLToPath(new URL("..", import.meta.url));
+const DATA = join(ROOT, "src-tauri/src/servers/testdata");
+const PROMETHEUS = join(ROOT, "src-tauri/src/history/testdata/prometheus-3.15.1");
 
 const USAGE =
-  "Использование: node scripts/mock-server.mjs --kind <вид> [--port 0..65535] [--host 127.0.0.1] [--api-key <ключ>] [--health 200..599] [--models full|empty] [--engines 1..8] [--reset-after N]";
+  "Использование: node scripts/mock-server.mjs --kind <вид> [--port 0..65535] [--host 127.0.0.1] [--api-key <ключ>] [--health 200..599] [--models full|empty] [--engines 1..8] [--reset-after N] [--slow N] [--huge]";
 
 /** vLLM (AuthenticationMiddleware): ключ нужен только путям с префиксом /v1, остальное (/version, /metrics, /health) открыто.
  *  Так же отвечают llama.cpp, Ollama и прочие OpenAI-совместимые виды: ключ защищает только /v1. */
@@ -98,6 +104,14 @@ export const KINDS = {
   llamacpp: { dir: "llamacpp", routes: { "/v1/models": "models.json" }, open: openOutsideV1 },
   // Ollama: /health нет.
   ollama: { dir: "ollama", routes: { "/v1/models": "models.json" }, noHealth: true, open: openOutsideV1 },
+  // Источник истории: с --api-key закрыто всё (вход по токену); /api/v1/query и query_range — prometheusReply.
+  prometheus: {
+    dir: "../../history/testdata/prometheus-3.15.1",
+    routes: { "/api/v1/status/buildinfo": "buildinfo.json", "/api/v1/targets": "targets.json" },
+    open: () => false,
+    noHealth: true,
+    prometheus: true,
+  },
 };
 
 /** Целое из значения параметра в пределах [min, max]; иначе ошибка с текстом использования. */
@@ -110,9 +124,13 @@ export function parseArgs(argv) {
   const fail = (message) => {
     throw new Error(`${message}\n${USAGE}`);
   };
-  const opts = { kind: "vllm", port: 8000, host: "127.0.0.1", apiKey: null, health: 200, models: "full", engines: 1, resetAfter: 0 };
+  const opts = { kind: "vllm", port: 8000, host: "127.0.0.1", apiKey: null, health: 200, models: "full", engines: 1, resetAfter: 0, slow: 0, huge: false };
   for (let i = 0; i < argv.length; i++) {
     const [name, value] = [argv[i], argv[i + 1]];
+    if (name === "--huge") {
+      opts.huge = true;
+      continue;
+    }
     if (name === "--kind") opts.kind = value;
     else if (name === "--port") opts.port = intArg(name, value, 0, 65535);
     else if (name === "--host") opts.host = value;
@@ -120,6 +138,7 @@ export function parseArgs(argv) {
     else if (name === "--health") opts.health = intArg(name, value, 200, 599);
     else if (name === "--engines") opts.engines = intArg(name, value, 1, 8);
     else if (name === "--reset-after") opts.resetAfter = intArg(name, value, 1, 1_000_000);
+    else if (name === "--slow") opts.slow = intArg(name, value, 1, 600);
     else if (name === "--models") {
       if (value !== "full" && value !== "empty") fail(`--models: нужно full или empty, получено ${value === undefined ? "ничего" : `«${value}»`}`);
       opts.models = value;
@@ -167,14 +186,68 @@ export function liveMetrics(text, poll, engines, kind) {
 
 /** Обработчик запросов: ключ проверяется по правилу вида (kind.open); /health отвечает заданным кодом,
  *  у 3xx — с Location на https, как у настоящего перенаправления. */
+/** Сетка `query_range`: точки от start до end с шагом step, секунды. */
+function grid(start, end, step) {
+  const out = [];
+  for (let t = start; t <= end; t += step) out.push(t);
+  return out;
+}
+
+/** Выражение без селекторов и интервалов: так шаблон для любого сервера находит снятый ряд. */
+const shape = (expr) => expr.replace(/\{[^}]*\}/g, "{}").replace(/\[[^\]]*\]/g, "[]");
+
+let captured;
+/** Снятые ответы query_range (range-*.json) по форме выражения; первый снимок формы — главный. */
+function capturedRanges() {
+  if (captured) return captured;
+  captured = new Map();
+  for (const name of ["range-vllm-8000.json", "range-sglang-30000.json", "range-vllm-8003.json"]) {
+    for (const { expr, response } of JSON.parse(readFileSync(join(PROMETHEUS, name), "utf8")).queries)
+      if (!captured.has(shape(expr))) captured.set(shape(expr), response.data.result);
+  }
+  return captured;
+}
+
+const vector = (result) => JSON.stringify({ status: "success", data: { resultType: "vector", result } });
+
+/** Ответ источника истории на /api/v1/query и /api/v1/query_range (вид prometheus). */
+export function prometheusReply(url, opts) {
+  const query = url.searchParams.get("query") ?? "";
+  if (query.includes("__bad__"))
+    return [400, JSON.stringify({ status: "error", errorType: "bad_data", error: `invalid parameter "query": 1:1: parse error: unexpected identifier "__bad__"` })];
+  if (url.pathname === "/api/v1/query") {
+    const file = (name) => readFileSync(join(PROMETHEUS, name), "utf8");
+    if (query === "up") return [200, file("up.json")];
+    if (query.startsWith("count by (job, instance)")) return [200, file(query.includes("vllm:") ? "discover.json" : "discover-sglang.json")];
+    if (query.startsWith("count({")) return [200, vector([{ metric: {}, value: [Date.now() / 1000, "57"] }])];
+    return [200, file("empty.json")];
+  }
+  const [start, end, step] = ["start", "end", "step"].map((k) => Number(url.searchParams.get(k)));
+  const times = grid(start, end, step);
+  if (opts.huge) {
+    const big = Array.from({ length: 4000 }, (_, i) => ({ metric: { pod: `vllm-${i}` }, values: times.map((t) => [t, "1"]) }));
+    return [200, JSON.stringify({ status: "success", data: { resultType: "matrix", result: big } })];
+  }
+  const result = (capturedRanges().get(shape(query)) ?? []).map((s) => {
+    const values = s.values.filter(([, v]) => v !== "NaN");
+    return { metric: s.metric, values: values.length ? times.map((t, i) => [t, values[i % values.length][1]]) : [] };
+  });
+  return [200, JSON.stringify({ status: "success", data: { resultType: "matrix", result } })];
+}
+
 export function handler(opts) {
   const kind = KINDS[opts.kind];
   let polls = 0;
   return (req, res) => {
-    const path = new URL(req.url ?? "/", "http://x").pathname;
+    const url = new URL(req.url ?? "/", "http://x");
+    const path = url.pathname;
     const reply = (status, type, body, headers = {}) => {
-      res.writeHead(status, { "content-type": type, ...headers });
-      res.end(body);
+      const send = () => {
+        res.writeHead(status, { "content-type": type, ...headers });
+        res.end(body);
+      };
+      if (opts.slow) setTimeout(send, opts.slow * 1000);
+      else send();
     };
     if (opts.apiKey && !kind.open(path) && req.headers.authorization !== `Bearer ${opts.apiKey}`)
       return reply(401, "application/json", JSON.stringify({ error: "Unauthorized" }));
@@ -183,6 +256,10 @@ export function handler(opts) {
       const authority = req.headers.host ?? `${opts.host}:${opts.port}`;
       return reply(opts.health, "text/plain", "", redirect ? { location: `https://${authority}/health` } : {});
     }
+    if (kind.prometheus && (path === "/api/v1/query" || path === "/api/v1/query_range")) {
+      const [status, body] = prometheusReply(url, opts);
+      return reply(status, "application/json", body);
+    }
     const file = kind.routes[path];
     if (!file) return reply(404, "application/json", JSON.stringify({ detail: "Not Found" }));
     // --models empty: сервер жив и отвечает, но модель не загружена — список пуст.
````

<!-- block: file t07-scripts-prometheus-smoke-yml step=7.3 path=scripts/prometheus/smoke.yml -->
````
# Prometheus для смоука истории метрик (docs/smoke/i1c.md): опрашивает мок-серверы раз в 5 с.
#   pnpm mock-server --kind vllm --port 8000
#   pnpm mock-server --kind vllm --engines 2 --port 8001
#   pnpm mock-server --kind vllm --reset-after 20 --port 8003
#   pnpm mock-server --kind sglang --port 30000
#   prometheus --config.file=scripts/prometheus/smoke.yml --storage.tsdb.path=<каталог> --web.listen-address=127.0.0.1:9090
# Настоящий vLLM на Apple Silicon (vllm-metal, порт 8100) — раскомментировать цель ниже.
global:
  scrape_interval: 5s
  evaluation_interval: 5s
scrape_configs:
  - job_name: vllm
    static_configs:
      - targets: ["127.0.0.1:8000", "127.0.0.1:8001", "127.0.0.1:8003"]
      # - targets: ["127.0.0.1:8100"]
  - job_name: sglang
    static_configs:
      - targets: ["127.0.0.1:30000"]
````

- [ ] **Шаг 7.4: Проверки.**

Run: `pnpm test:scripts`
Expected: `# pass 110`, `# fail 0`
Run: `promtool check config scripts/prometheus/smoke.yml`
Expected: `SUCCESS` (если Prometheus установлен)

- [ ] **Шаг 7.5: Коммит.**

<!-- block: shell t07-commit step=7.5 -->
````
git add README.md \
  scripts/mock-server.mjs \
  scripts/mock-server.test.mjs \
  scripts/prometheus/smoke.yml
git commit -s -q -F - <<'EOF'
feat(mock): prometheus kind for history source failures

The mock answers as a metrics history source with real Prometheus 3.15.1
answers: up, target discovery, selector checks and query_range of the
Glafira templates on the requested grid. --api-key closes every path,
--slow delays answers, --huge returns more than 8 MB and __bad__ in an
expression is bad_data. scripts/prometheus/smoke.yml scrapes the mocks.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
````

---

### Задача 8: Типы, клиент IPC и «Сейчас» из истории

Типы профиля (`history`, `historySelector`), секрета (`historySecret`), проверки и окна истории; вызовы
`checkHistorySource` и `readServerHistory` (спека I1c §5). `useServerHistory` читает окно при выборе и держит ответ
минуту. С `prefill` хук `useServerMetrics` один раз кладёт в начало буфера последние 15 минут истории, называет время
последней точки истории и до второго живого опроса берёт скорости и перцентили за минуту из неё.

**Files:**
- Modify: `src/api/client.test.ts`
- Modify: `src/api/client.ts`
- Modify: `src/api/types.ts`
- Modify: `src/components/servers/useServerMetrics.test.tsx`
- Modify: `src/components/servers/useServerMetrics.ts`

**Interfaces:**
- Produces: типы `HistoryAuth`, `HistorySource`, `HistoryWindow`, `HistorySourceKind`, `HistoryTarget`,
  `HistoryCheck`, `HistoryAnswer` (`src/api/types.ts`), `TestStepName` с `up`, `targets`, `match`;
  `api.checkHistorySource(profile, secrets)`, `api.readServerHistory(installationId, serverId, window)`;
  `useServerHistory(installationId, serverId, window, enabled)`, `historyQueryKey`, `prependHistory`, `withHistory`;
  `useServerMetrics(installationId, serverId, enabled = true, prefill = false)` →
  `{ latest, fromHistory, historyUntil, historyError, points, error, isPending, refetch }`.

- [ ] **Шаг 8.1: Тесты.** Тесты клиента, буфера с историей и хука.

<!-- block: patch t08-src-api-client-test-ts step=8.1 -->
````
diff --git a/src/api/client.test.ts b/src/api/client.test.ts
index 6eea0f0..376e86b 100644
--- a/src/api/client.test.ts
+++ b/src/api/client.test.ts
@@ -24,6 +24,13 @@ describe("api client", () => {
     expect(invoke).toHaveBeenLastCalledWith("save_installation", { input: { profile, secrets: {} } });
   });
 
+  it("checkHistorySource передаёт профиль формы и секрет источника", async () => {
+    invoke.mockResolvedValue({ report: { ok: true, steps: [], warnings: [] }, targets: [], suggestions: [] });
+    const profile = { id: "a" } as InstallationProfile;
+    await api.checkHistorySource(profile, { historySecret: "t" });
+    expect(invoke).toHaveBeenCalledWith("check_history_source", { input: { profile, secrets: { historySecret: "t" } } });
+  });
+
   it("testServer передаёт профиль формы, ключи и id сервера", async () => {
     invoke.mockResolvedValue({ ok: true, steps: [], warnings: [] });
     const profile = { id: "", name: "x" } as InstallationProfile;
@@ -48,6 +55,8 @@ describe("api client", () => {
     expect(invoke).toHaveBeenCalledWith("read_server_metrics", { installationId: "i1", serverId: "s1" });
     await api.readServerConfig("i1", "s2");
     expect(invoke).toHaveBeenLastCalledWith("read_server_config", { installationId: "i1", serverId: "s2" });
+    await api.readServerHistory("i1", "s2", "24h");
+    expect(invoke).toHaveBeenLastCalledWith("read_server_history", { installationId: "i1", serverId: "s2", window: "24h" });
   });
 
   it("ошибка из Rust приходит как есть, прочие — как INTERNAL", async () => {
````

<!-- block: patch t08-src-components-servers-useservermetrics-test-tsx step=8.1 -->
````
diff --git a/src/components/servers/useServerMetrics.test.tsx b/src/components/servers/useServerMetrics.test.tsx
index f114722..6188dba 100644
--- a/src/components/servers/useServerMetrics.test.tsx
+++ b/src/components/servers/useServerMetrics.test.tsx
@@ -5,13 +5,13 @@ import { beforeEach, describe, expect, it, vi } from "vitest";
 
 vi.mock("../../api/client", async (importOriginal) => {
   const orig = await importOriginal<typeof import("../../api/client")>();
-  return { ...orig, api: { readServerMetrics: vi.fn() } };
+  return { ...orig, api: { readServerMetrics: vi.fn(), readServerHistory: vi.fn() } };
 });
 
 import { api } from "../../api/client";
 import { TabActiveContext } from "../../lib/tabActive";
 import { metrics } from "./testMetrics";
-import { appendPoint, type MetricPoint, useServerMetrics, WINDOW_MS } from "./useServerMetrics";
+import { appendPoint, type MetricPoint, prependHistory, useServerMetrics, WINDOW_MS, withHistory } from "./useServerMetrics";
 
 const point = (at: number): MetricPoint => ({ at, m: metrics(at) });
 
@@ -35,6 +35,28 @@ describe("appendPoint", () => {
   });
 });
 
+describe("история в буфере и плитках", () => {
+  it("точки истории — только раньше первой живой", () => {
+    const b = prependHistory([point(30_000), point(35_000)], [point(15_000), point(30_000)]);
+    expect(b.map((p) => p.at)).toEqual([15_000, 30_000, 35_000]);
+    expect(prependHistory([], [point(1)]).map((p) => p.at)).toEqual([1]);
+  });
+
+  it("первый живой снимок берёт скорости и перцентили за минуту из истории", () => {
+    const hist = metrics(10_000);
+    const first = metrics(20_000, { intervalSec: null, rates: null, latency1m: { ttft: null, itl: null, e2e: null, queueTime: null }, preemptions1m: null });
+    const { latest, fromHistory } = withHistory(first, hist);
+    expect(fromHistory).toBe(true);
+    expect(latest?.at).toBe(20_000);
+    expect(latest?.gauges).toEqual(first.gauges);
+    expect(latest?.rates).toEqual(hist.rates);
+    expect(latest?.latency1m).toEqual(hist.latency1m);
+    expect(withHistory(metrics(25_000), hist)).toEqual({ latest: metrics(25_000), fromHistory: false });
+    expect(withHistory(undefined, hist)).toEqual({ latest: hist, fromHistory: true });
+    expect(withHistory(first, undefined)).toEqual({ latest: first, fromHistory: false });
+  });
+});
+
 function wrapper(active: boolean) {
   const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
   return ({ children }: { children: ReactNode }) => (
@@ -45,7 +67,37 @@ function wrapper(active: boolean) {
 }
 
 describe("useServerMetrics", () => {
-  beforeEach(() => vi.mocked(api.readServerMetrics).mockReset());
+  beforeEach(() => {
+    vi.mocked(api.readServerMetrics).mockReset();
+    vi.mocked(api.readServerHistory).mockReset();
+  });
+
+  it("с историей буфер начинается последними 15 минутами из неё", async () => {
+    const t0 = Date.now();
+    vi.mocked(api.readServerHistory).mockResolvedValue({
+      window: "now",
+      stepSec: 15,
+      source: "prometheus",
+      points: [{ at: t0 - 30_000, m: metrics(t0 - 30_000) }, { at: t0 - 15_000, m: metrics(t0 - 15_000) }],
+      partial: [],
+    });
+    vi.mocked(api.readServerMetrics).mockResolvedValue(metrics(t0, { intervalSec: null, rates: null }));
+    const { result } = renderHook(() => useServerMetrics("i1", "s1", true, true), { wrapper: wrapper(true) });
+    await waitFor(() => expect(result.current.points).toHaveLength(3));
+    expect(api.readServerHistory).toHaveBeenCalledWith("i1", "s1", "now");
+    expect(result.current.points.map((p) => p.at)).toEqual([t0 - 30_000, t0 - 15_000, t0]);
+    expect(result.current.historyUntil).toBe(t0 - 15_000);
+    expect(result.current.fromHistory).toBe(true);
+    expect(result.current.latest?.rates?.promptTokens).toBe(120);
+  });
+
+  it("без истории она не запрашивается", async () => {
+    vi.mocked(api.readServerMetrics).mockResolvedValue(metrics(Date.now()));
+    const { result } = renderHook(() => useServerMetrics("i1", "s1"), { wrapper: wrapper(true) });
+    await waitFor(() => expect(result.current.points).toHaveLength(1));
+    expect(api.readServerHistory).not.toHaveBeenCalled();
+    expect(result.current.historyUntil).toBeNull();
+  });
 
   it("копит точки опросов, ошибка — точка-разрыв", async () => {
     // Время снимков — настоящее: точка ошибки ставится по часам окна, а окно отсчитывается от последней точки.
````

- [ ] **Шаг 8.2: Увидеть падение.**

Run: `pnpm exec vitest run src/components/servers src/api`
Expected: FAIL — `api.readServerHistory is not a function`, нет `prependHistory` и `withHistory`

- [ ] **Шаг 8.3: Реализация.** Типы, клиент и хуки.

<!-- block: patch t08-src-api-client-ts step=8.3 -->
````
diff --git a/src/api/client.ts b/src/api/client.ts
index dc6c2a4..676f8d5 100644
--- a/src/api/client.ts
+++ b/src/api/client.ts
@@ -4,6 +4,9 @@ import type {
   AppInfo,
   ConfigEntry,
   ExportSummary,
+  HistoryAnswer,
+  HistoryCheck,
+  HistoryWindow,
   ImportPreview,
   InstallationFolder,
   InstallationInfo,
@@ -60,6 +63,11 @@ export const api = {
     call<ServerMetrics>("read_server_metrics", { installationId, serverId }),
   readServerConfig: (installationId: string, serverId: string) =>
     call<ConfigEntry[]>("read_server_config", { installationId, serverId }),
+  /** Проверка источника истории несохранённого профиля: секрет из формы — поверх keychain. */
+  checkHistorySource: (profile: InstallationProfile, secrets: SecretsInput = {}) =>
+    call<HistoryCheck>("check_history_source", { input: { profile, secrets } }),
+  readServerHistory: (installationId: string, serverId: string, window: HistoryWindow) =>
+    call<HistoryAnswer>("read_server_history", { installationId, serverId, window }),
 
   listFolders: () => call<InstallationFolder[]>("list_folders"),
   createFolder: (name: string, parentId: string | null) =>
````

<!-- block: patch t08-src-api-types-ts step=8.3 -->
````
diff --git a/src/api/types.ts b/src/api/types.ts
index 85a7b42..141bf8f 100644
--- a/src/api/types.ts
+++ b/src/api/types.ts
@@ -50,6 +50,19 @@ export interface ServerConfig {
   metricsPath: string;
   ca: CaConfig;
   proxy: ProxyConfig;
+  /** Селектор серий сервера в источнике истории (`{job="vllm", instance="10.0.0.5:8000"}`); нет — истории нет. */
+  historySelector?: string;
+}
+
+/** Вход в источник истории: токен или пароль — только в keychain (спека I1c §2). */
+export type HistoryAuth = { mode: "none" } | { mode: "bearer" } | { mode: "basic"; username: string };
+
+/** Источник истории метрик — Prometheus или VictoriaMetrics. */
+export interface HistorySource {
+  url: string;
+  auth: HistoryAuth;
+  ca: CaConfig;
+  proxy: ProxyConfig;
 }
 
 export interface InstallationProfile {
@@ -59,6 +72,8 @@ export interface InstallationProfile {
   environment: Environment;
   readOnly: boolean;
   servers: ServerConfig[];
+  /** Источник истории метрик; нет — окна истории недоступны. */
+  history?: HistorySource;
   requestTimeoutMs: number;
   connectTimeoutMs: number;
   folderId: string | null;
@@ -66,9 +81,11 @@ export interface InstallationProfile {
   order?: number;
 }
 
-/** Ключи API из формы: id сервера → значение; сервера нет — оставить сохранённый, "" — удалить. */
+/** Ключи API из формы: id сервера → значение; сервера нет — оставить сохранённый, "" — удалить. Секрет источника
+ *  истории (токен или пароль) — по тем же правилам: нет поля — оставить, "" — удалить. */
 export interface SecretsInput {
   apiKeys?: Record<string, string>;
+  historySecret?: string;
 }
 
 export interface InstallationFolder {
@@ -115,7 +132,7 @@ export interface TreeItemRef {
   id: string;
 }
 
-export type TestStepName = "config" | "tcp" | "tls" | "health" | "auth" | "kind" | "models";
+export type TestStepName = "config" | "tcp" | "tls" | "health" | "auth" | "kind" | "models" | "up" | "targets" | "match";
 
 export interface TestStep {
   name: TestStepName;
@@ -227,6 +244,34 @@ export interface ServerMetrics {
   series: Partial<Record<Canonical, string>>;
 }
 
+/** Окно истории: `now` — 15 минут для «Сейчас», остальные — окна вкладки «Метрики». */
+export type HistoryWindow = "now" | "1h" | "24h" | "7d" | "30d";
+export type HistorySourceKind = "prometheus" | "victoriametrics";
+
+/** Цель инференса в источнике истории. */
+export interface HistoryTarget {
+  job: string;
+  instance: string;
+  kind: "vllm" | "sglang";
+}
+
+/** Проверка источника истории: шаги, найденные цели и автоподбор серверов без селектора. */
+export interface HistoryCheck {
+  report: TestReport;
+  source?: { kind: HistorySourceKind; version: string | null };
+  targets: HistoryTarget[];
+  suggestions: { serverId: string; selector: string | null; targets: HistoryTarget[] }[];
+}
+
+/** Окно истории сервера: точки той же формы, что у «Сейчас»; `partial` — ряды, которые не прочитались. */
+export interface HistoryAnswer {
+  window: HistoryWindow;
+  stepSec: number;
+  source: HistorySourceKind;
+  points: { at: number; m: ServerMetrics }[];
+  partial: { chart: string; error: AppError }[];
+}
+
 export type ConfigSource = "server_info" | "models" | "metrics";
 
 /** Аргумент запуска сервера с источником (спец. I1b §5). */
````

<!-- block: patch t08-src-components-servers-useservermetrics-ts step=8.3 -->
````
diff --git a/src/components/servers/useServerMetrics.ts b/src/components/servers/useServerMetrics.ts
index f14ab43..be65d50 100644
--- a/src/components/servers/useServerMetrics.ts
+++ b/src/components/servers/useServerMetrics.ts
@@ -1,7 +1,7 @@
 import { useQuery } from "@tanstack/react-query";
 import { useEffect, useState } from "react";
 import { api, toAppError } from "../../api/client";
-import type { AppError, ServerFacts, ServerMetrics } from "../../api/types";
+import type { AppError, HistoryWindow, ServerFacts, ServerMetrics } from "../../api/types";
 import { useRefetchInterval } from "../../lib/tabActive";
 
 /** Опрос `/metrics` сервера — раз в 5 с, только пока вкладка видна (спец. §6.6). */
@@ -42,11 +42,58 @@ export function metricsQueryKey(installationId: string, serverId: string) {
   return ["server-metrics", installationId, serverId] as const;
 }
 
+export function historyQueryKey(installationId: string, serverId: string, window: HistoryWindow) {
+  return ["server-history", installationId, serverId, window] as const;
+}
+
+/**
+ * Окно истории сервера (спека I1c §5): запрашивается при выборе окна, ответ живёт минуту — повторный выбор окна в
+ * пределах минуты источник не нагружает. Автоматического перезапроса нет, свежее окно — по «Обновить».
+ */
+export function useServerHistory(installationId: string, serverId: string, window: HistoryWindow, enabled: boolean) {
+  return useQuery({
+    queryKey: historyQueryKey(installationId, serverId, window),
+    queryFn: () => api.readServerHistory(installationId, serverId, window),
+    enabled,
+    staleTime: 60_000,
+    retry: false,
+    refetchOnWindowFocus: false,
+  });
+}
+
+/** Точки истории в начало буфера: только те, что раньше первой живой точки. */
+export function prependHistory(buf: MetricPoint[], history: MetricPoint[]): MetricPoint[] {
+  const first = buf.length > 0 ? buf[0].at : Number.POSITIVE_INFINITY;
+  return [...history.filter((p) => p.at < first), ...buf];
+}
+
+/**
+ * Метрики для плиток: у первого живого снимка ещё нет скоростей и перцентилей за минуту — до второго опроса они
+ * берутся из последней точки истории (`fromHistory`). Без живого снимка — точка истории целиком.
+ */
+export function withHistory(live: ServerMetrics | undefined, last: ServerMetrics | undefined): { latest: ServerMetrics | undefined; fromHistory: boolean } {
+  if (!last) return { latest: live, fromHistory: false };
+  if (!live) return { latest: last, fromHistory: true };
+  if (live.intervalSec != null) return { latest: live, fromHistory: false };
+  return {
+    latest: {
+      ...live,
+      rates: last.rates,
+      prefixCacheHitRatio: last.prefixCacheHitRatio,
+      latency: last.latency,
+      latency1m: last.latency1m,
+      preemptions1m: last.preemptions1m,
+    },
+    fromHistory: true,
+  };
+}
+
 /**
  * Метрики сервера «сейчас» и буфер точек за час. Один запрос на сервер: карточка инсталляции и вкладка сервера
- * делят его (общий ключ react-query), сервер опрашивается один раз за интервал.
+ * делят его (общий ключ react-query), сервер опрашивается один раз за интервал. `prefill` — у сервера есть история:
+ * буфер один раз заполняется последними 15 минутами из неё (`historyUntil` — время последней точки истории).
  */
-export function useServerMetrics(installationId: string, serverId: string, enabled = true) {
+export function useServerMetrics(installationId: string, serverId: string, enabled = true, prefill = false) {
   const refetchInterval = useRefetchInterval(POLL_MS);
   const q = useQuery({
     queryKey: metricsQueryKey(installationId, serverId),
@@ -68,6 +115,31 @@ export function useServerMetrics(installationId: string, serverId: string, enabl
   useEffect(() => {
     if (q.error && q.errorUpdatedAt) push({ at: q.errorUpdatedAt, m: null, error: toAppError(q.error) });
   }, [key, q.error, q.errorUpdatedAt]);
+  // История запрашивается один раз (staleTime — бесконечность): дальше буфер пополняет опрос.
+  const history = useQuery({
+    queryKey: historyQueryKey(installationId, serverId, "now"),
+    queryFn: () => api.readServerHistory(installationId, serverId, "now"),
+    enabled: enabled && prefill,
+    staleTime: Number.POSITIVE_INFINITY,
+    retry: false,
+    refetchOnWindowFocus: false,
+  });
+  const historyPoints = enabled && prefill ? history.data?.points : undefined;
+  useEffect(() => {
+    if (historyPoints?.length)
+      setBuf((b) => ({ key, points: prependHistory(b.key === key ? b.points : [], historyPoints) }));
+  }, [key, historyPoints]);
   const points = buf.key === key ? buf.points : EMPTY;
-  return { latest: q.data, points, error: q.error ? toAppError(q.error) : null, isPending: q.isPending, refetch: q.refetch };
+  const last = historyPoints?.length ? historyPoints[historyPoints.length - 1] : undefined;
+  const { latest, fromHistory } = withHistory(q.data, last?.m);
+  return {
+    latest,
+    fromHistory,
+    historyUntil: last?.at ?? null,
+    historyError: prefill && history.error ? toAppError(history.error) : null,
+    points,
+    error: q.error ? toAppError(q.error) : null,
+    isPending: q.isPending,
+    refetch: q.refetch,
+  };
 }
````

- [ ] **Шаг 8.4: Проверки.**

Run: `pnpm test`
Expected: `Test Files  35 passed`, `Tests  238 passed`
Run: `pnpm exec tsc --noEmit`
Expected: без вывода

- [ ] **Шаг 8.5: Коммит.**

<!-- block: shell t08-commit step=8.5 -->
````
git add src/api/client.test.ts \
  src/api/client.ts \
  src/api/types.ts \
  src/components/servers/useServerMetrics.test.tsx \
  src/components/servers/useServerMetrics.ts
git commit -s -q -F - <<'EOF'
feat(history): history types, IPC calls and the prefilled "now" buffer

Profile types get the history source and server selectors, the client gets
checkHistorySource and readServerHistory. useServerHistory loads a history
window on demand and keeps it for a minute. With prefill, useServerMetrics
starts its buffer with the last 15 minutes of history, reports where the
history ends, and fills rates and minute percentiles of the first live
snapshot from the last history point.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
````

---

### Задача 9: Окна истории на вкладке «Метрики»

Окна 1 ч…30 д доступны, если у инсталляции есть источник, а у сервера — селектор; иначе подсказка называет
причину (спека I1c §5). Окно читается при выборе, прежние графики остаются под слоем загрузки, «Обновить» читает
заново, над графиками — шаг и источник, у частичного окна — какие ряды не прочитаны, ошибка окна — вместо графиков.
В «Сейчас» — пунктир и подпись «История до HH:MM». Строка «История — с этапа I1c» удаляется из словарей. В
`src/api/types.ts` комментарий `SecretsInput` переносится в многострочный JSDoc: проверка кириллицы пропускает только
строки JSDoc, начинающиеся со `*`.

**Files:**
- Modify: `docs/i18n/same-as-en.json`
- Modify: `src/api/types.ts`
- Modify: `src/app.css`
- Modify: `src/components/servers/MetricChart.test.tsx`
- Modify: `src/components/servers/MetricChart.tsx`
- Modify: `src/components/servers/ServerMetricsView.tsx`
- Modify: `src/components/servers/ServerPanel.test.tsx`
- Modify: `src/components/servers/ServerPanel.tsx`
- Modify: `src/components/servers/chart.ts`
- Modify: `src/components/servers/useServerMetrics.ts`
- Modify: `src/locales/en.json`
- Modify: `src/locales/ru.json`

**Interfaces:**
- Consumes: `useServerHistory`, `useServerMetrics(…, prefill)`.
- Produces: `ServerMetricsView({ installationId, serverId, points, historyUntil, historyError, historyUnavailable })`;
  `MetricChart` с `marker?: number | null`; `drawMarker(u, at, color)`, `MarkerTarget` (`chart.ts`); ключи
  `servers.metrics.noSource`, `notMatched`, `historyUntil`, `prefillFailed`, `step`, `sources.*`, `partial`,
  `servers.units.sec`, `min`, `common.refresh`.

- [ ] **Шаг 9.1: Тесты.** Тесты границы на графике, вкладки сервера с источником и без, окна, частичного окна и ошибки окна.

<!-- block: patch t09-src-components-servers-metricchart-test-tsx step=9.1 -->
````
diff --git a/src/components/servers/MetricChart.test.tsx b/src/components/servers/MetricChart.test.tsx
index 9f58d98..654d3ed 100644
--- a/src/components/servers/MetricChart.test.tsx
+++ b/src/components/servers/MetricChart.test.tsx
@@ -22,7 +22,8 @@ globalThis.ResizeObserver ??= class {
   unobserve() {}
 } as unknown as typeof ResizeObserver;
 
-import { axisIncrs, axisSize, chartData, hasValues } from "./chart";
+import { renderWithClient } from "../../test/render";
+import { axisIncrs, axisSize, chartData, drawMarker, hasValues } from "./chart";
 import { MetricChart } from "./MetricChart";
 import { ServerMetricsView } from "./ServerMetricsView";
 import { metrics } from "./testMetrics";
@@ -51,6 +52,32 @@ describe("ось Y", () => {
   });
 });
 
+describe("граница истории", () => {
+  const target = (x: number) => {
+    const ctx = { save: vi.fn(), restore: vi.fn(), setLineDash: vi.fn(), beginPath: vi.fn(), moveTo: vi.fn(), lineTo: vi.fn(), stroke: vi.fn(), strokeStyle: "" };
+    return { ctx, u: { ctx: ctx as unknown as CanvasRenderingContext2D, bbox: { left: 10, top: 5, width: 100, height: 50 }, valToPos: vi.fn(() => x) } };
+  };
+
+  it("пунктир на времени границы — во всю высоту области графика", () => {
+    const { ctx, u } = target(60);
+    drawMarker(u, 30_000, "#888");
+    expect(u.valToPos).toHaveBeenCalledWith(30, "x", true);
+    expect(ctx.setLineDash).toHaveBeenCalledWith([4, 4]);
+    expect(ctx.moveTo).toHaveBeenCalledWith(60, 5);
+    expect(ctx.lineTo).toHaveBeenCalledWith(60, 55);
+    expect(ctx.strokeStyle).toBe("#888");
+  });
+
+  it("без границы и вне области — ничего", () => {
+    const outside = target(500);
+    drawMarker(outside.u, 30_000, "#888");
+    expect(outside.ctx.stroke).not.toHaveBeenCalled();
+    const none = target(60);
+    drawMarker(none.u, null, "#888");
+    expect(none.u.valToPos).not.toHaveBeenCalled();
+  });
+});
+
 describe("chartData", () => {
   it("время в секундах, ошибка опроса и пропуск значения — null", () => {
     const pts: MetricPoint[] = [
@@ -116,16 +143,20 @@ describe("ServerMetricsView", () => {
   const finished = (reasons: Record<string, number>, aborted: number | null) =>
     metrics(1_000, { rates: { promptTokens: 1, generationTokens: 1, requests: 1, preemptions: 0, aborted, finishedByReason: reasons } });
   const finishedLabels = () => plots.filter((p) => p.opts.title === "Завершения в секунду").pop()?.opts.series.map((s) => s.label);
+  const view = (points: MetricPoint[]) =>
+    renderWithClient(
+      <ServerMetricsView installationId="i1" serverId="s1" points={points} historyUntil={null} historyError={null} historyUnavailable="servers.metrics.noSource" />,
+    );
 
   it("«Завершения»: прерванные запросы vLLM — одна линия, причина abort не дублирует «прерваны»", () => {
     plots.length = 0;
-    render(<ServerMetricsView points={[{ at: 1_000, m: finished({ stop: 2, abort: 1 }, 1) }]} />);
+    view([{ at: 1_000, m: finished({ stop: 2, abort: 1 }, 1) }]);
     expect(finishedLabels()).toEqual([undefined, "stop", "прерваны"]);
   });
 
   it("«Завершения»: без отдельного счётчика прерываний причина abort остаётся", () => {
     plots.length = 0;
-    render(<ServerMetricsView points={[{ at: 1_000, m: finished({ stop: 2, abort: 1 }, null) }]} />);
+    view([{ at: 1_000, m: finished({ stop: 2, abort: 1 }, null) }]);
     expect(finishedLabels()).toEqual([undefined, "abort", "stop", "прерваны"]);
   });
 });
````

<!-- block: patch t09-src-components-servers-serverpanel-test-tsx step=9.1 -->
````
diff --git a/src/components/servers/ServerPanel.test.tsx b/src/components/servers/ServerPanel.test.tsx
index 1e387a0..ffaa3b5 100644
--- a/src/components/servers/ServerPanel.test.tsx
+++ b/src/components/servers/ServerPanel.test.tsx
@@ -7,7 +7,7 @@ vi.mock("../../api/client", async (importOriginal) => {
   const orig = await importOriginal<typeof import("../../api/client")>();
   return {
     ...orig,
-    api: { listInstallations: vi.fn(), connectInstallation: vi.fn(), readServerMetrics: vi.fn(), readServerConfig: vi.fn() },
+    api: { listInstallations: vi.fn(), connectInstallation: vi.fn(), readServerMetrics: vi.fn(), readServerConfig: vi.fn(), readServerHistory: vi.fn() },
   };
 });
 vi.mock("uplot", () => ({
@@ -38,6 +38,15 @@ const PROFILE: InstallationProfile = {
   ],
 };
 
+const HISTORY = { url: "http://prometheus:9090", auth: { mode: "none" }, ca: { mode: "system" }, proxy: { mode: "system" } } as const;
+
+/** Профиль с источником истории и сопоставленным сервером s1. */
+const withHistory = (): InstallationProfile => ({
+  ...PROFILE,
+  history: HISTORY,
+  servers: PROFILE.servers.map((s) => (s.id === "s1" ? { ...s, historySelector: '{job="vllm", instance="gpu-1:8000"}' } : s)),
+});
+
 const INFO: InstallationInfo = {
   installationId: "i1",
   servers: [
@@ -51,6 +60,7 @@ const INFO: InstallationInfo = {
 
 beforeEach(() => {
   vi.clearAllMocks();
+  vi.mocked(api.readServerHistory).mockReset();
   vi.mocked(api.listInstallations).mockResolvedValue([PROFILE]);
   vi.mocked(api.connectInstallation).mockResolvedValue(INFO);
   vi.mocked(api.readServerMetrics).mockResolvedValue(
@@ -93,15 +103,64 @@ describe("ServerPanel", () => {
     expect(api.readServerConfig).toHaveBeenCalledWith("i1", "s1");
   });
 
-  it("«Метрики»: окно «Сейчас», история недоступна до I1c, графики строятся", async () => {
+  it("«Метрики»: окно «Сейчас»; без источника истории окна недоступны, графики строятся", async () => {
     const user = userEvent.setup();
     renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
     await user.click(await screen.findByRole("tab", { name: "Метрики" }));
     expect(screen.getByRole("button", { name: "Сейчас" })).toHaveAttribute("aria-pressed", "true");
     const day = screen.getByRole("button", { name: "24 ч" });
     expect(day).toBeDisabled();
-    expect(day).toHaveAttribute("title", "История — с этапа I1c");
+    expect(day).toHaveAttribute("title", "Добавьте источник истории в инсталляцию");
     expect(await screen.findByRole("region", { name: "Время до первого токена (TTFT)" })).toBeInTheDocument();
+    expect(api.readServerHistory).not.toHaveBeenCalled();
+  });
+
+  it("источник есть, а серии сервера не сопоставлены — окна недоступны с подсказкой", async () => {
+    vi.mocked(api.listInstallations).mockResolvedValue([{ ...PROFILE, history: HISTORY }]);
+    const user = userEvent.setup();
+    renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
+    await user.click(await screen.findByRole("tab", { name: "Метрики" }));
+    expect(screen.getByRole("button", { name: "7 д" })).toHaveAttribute("title", "Серии сервера не сопоставлены с историей");
+    expect(api.readServerHistory).not.toHaveBeenCalled();
+  });
+
+  it("окно истории: запрос при выборе, шаг и источник, частичное окно; «Сейчас» начинается из истории", async () => {
+    vi.mocked(api.listInstallations).mockResolvedValue([withHistory()]);
+    const now = Date.now();
+    vi.mocked(api.readServerHistory).mockImplementation(async (_i, _s, window) => ({
+      window,
+      stepSec: window === "now" ? 15 : 90,
+      source: "prometheus",
+      points: [{ at: now - 30_000, m: metrics(now - 30_000) }],
+      partial: window === "24h" ? [{ chart: "ttft", error: { code: "TIMEOUT", key: "errors.history.timeout", message: "late", retryable: true } }] : [],
+    }));
+    const user = userEvent.setup();
+    renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
+    await user.click(await screen.findByRole("tab", { name: "Метрики" }));
+    expect(api.readServerHistory).toHaveBeenCalledWith("i1", "s1", "now");
+    expect(await screen.findByText(/^История до \d\d:\d\d$/)).toBeInTheDocument();
+    await user.click(screen.getByRole("button", { name: "24 ч" }));
+    expect(api.readServerHistory).toHaveBeenLastCalledWith("i1", "s1", "24h");
+    expect(await screen.findByText("Шаг 90 с · Prometheus")).toBeInTheDocument();
+    expect(screen.getByRole("status")).toHaveTextContent("Часть рядов не прочитана: Время до первого токена (TTFT) — late");
+    expect(screen.getByRole("button", { name: "24 ч" })).toHaveAttribute("aria-pressed", "true");
+    await user.click(screen.getByRole("button", { name: "Обновить" }));
+    await waitFor(() => expect(vi.mocked(api.readServerHistory).mock.calls.filter((c) => c[2] === "24h")).toHaveLength(2));
+  });
+
+  it("ошибка окна истории — вместо графиков", async () => {
+    vi.mocked(api.listInstallations).mockResolvedValue([withHistory()]);
+    vi.mocked(api.readServerHistory).mockImplementation(async (_i, _s, window) => {
+      if (window === "now") throw { code: "CONNECTION_FAILED", key: "errors.history.status", message: "down", retryable: true };
+      throw { code: "AUTH_FAILED", key: "errors.history.unauthorized", message: "401", retryable: false };
+    });
+    const user = userEvent.setup();
+    renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
+    await user.click(await screen.findByRole("tab", { name: "Метрики" }));
+    await waitFor(() => expect(screen.getByRole("button", { name: "Сейчас" })).toHaveAttribute("title", "История не загружена: down"));
+    await user.click(screen.getByRole("button", { name: "1 ч" }));
+    expect(await screen.findByText(/Источник истории метрик отклонил вход \(401\)/)).toBeInTheDocument();
+    expect(screen.queryByRole("region", { name: "Время до первого токена (TTFT)" })).toBeNull();
   });
 
   it("сервер, который не подключился: его ошибка, без подвкладок и без опроса", async () => {
````

- [ ] **Шаг 9.2: Увидеть падение.**

Run: `pnpm exec vitest run src/components/servers`
Expected: FAIL — нет `drawMarker`, подсказка окон — «История — с этапа I1c», `readServerHistory` не вызывается

- [ ] **Шаг 9.3: Реализация.** Окна, граница, доступность в панели сервера, стиль загрузки, словари и исключение `same-as-en` для названий источников.

<!-- block: shell t09-prepare step=9.3 -->
````
node -e '
const fs = require("node:fs");
for (const code of ["en", "ru"]) {
  const file = `src/locales/${code}.json`;
  const dict = JSON.parse(fs.readFileSync(file, "utf8"));
  delete dict.servers.metrics.historyLater;
  fs.writeFileSync(file, JSON.stringify(dict, null, 2) + "\n");
}'
````

<!-- block: patch t09-docs-i18n-same-as-en-json step=9.3 -->
````
diff --git a/docs/i18n/same-as-en.json b/docs/i18n/same-as-en.json
index 6bdc309..5cdc462 100644
--- a/docs/i18n/same-as-en.json
+++ b/docs/i18n/same-as-en.json
@@ -6,6 +6,8 @@
     "installations.form.step.tcp",
     "installations.form.step.tls",
     "messages.connectionTest.kind",
-    "messages.connectionTest.kindVersion"
+    "messages.connectionTest.kindVersion",
+    "servers.metrics.sources.prometheus",
+    "servers.metrics.sources.victoriametrics"
   ]
 }
````

<!-- block: patch t09-src-api-types-ts step=9.3 -->
````
diff --git a/src/api/types.ts b/src/api/types.ts
index 141bf8f..eb11d25 100644
--- a/src/api/types.ts
+++ b/src/api/types.ts
@@ -81,8 +81,10 @@ export interface InstallationProfile {
   order?: number;
 }
 
-/** Ключи API из формы: id сервера → значение; сервера нет — оставить сохранённый, "" — удалить. Секрет источника
- *  истории (токен или пароль) — по тем же правилам: нет поля — оставить, "" — удалить. */
+/**
+ * Ключи API из формы: id сервера → значение; сервера нет — оставить сохранённый, "" — удалить. Секрет источника
+ * истории (токен или пароль) — по тем же правилам: нет поля — оставить, "" — удалить.
+ */
 export interface SecretsInput {
   apiKeys?: Record<string, string>;
   historySecret?: string;
````

<!-- block: patch t09-src-app-css step=9.3 -->
````
diff --git a/src/app.css b/src/app.css
index 8de1019..10e9e29 100644
--- a/src/app.css
+++ b/src/app.css
@@ -30,6 +30,7 @@ input::placeholder, textarea::placeholder { font-style: italic; color: var(--mut
 .tile-value { font-size: 18px; font-weight: 600; font-variant-numeric: tabular-nums; }
 .charts { display: grid; grid-template-columns: repeat(auto-fill, minmax(380px, 1fr)); gap: 12px; margin-top: 8px; }
 .metric-chart { min-width: 0; }
+.charts.loading { opacity: 0.5; }
 .metric-chart h4 { margin: 0 0 4px; font-size: 13px; }
 .server-metrics .toolbar button.active { border-color: var(--accent); font-weight: 600; }
 .badge { border: 1px solid var(--border); border-radius: 10px; padding: 0 6px; font-size: 12px; color: var(--muted); white-space: nowrap; }
````

<!-- block: patch t09-src-components-servers-metricchart-tsx step=9.3 -->
````
diff --git a/src/components/servers/MetricChart.tsx b/src/components/servers/MetricChart.tsx
index 8976a5a..4f76ab2 100644
--- a/src/components/servers/MetricChart.tsx
+++ b/src/components/servers/MetricChart.tsx
@@ -2,7 +2,7 @@ import { useEffect, useMemo, useRef } from "react";
 import { useTranslation } from "react-i18next";
 import uPlot from "uplot";
 import "uplot/dist/uPlot.min.css";
-import { type ChartPick, axisIncrs, axisSize, chartData, hasValues, themeColor } from "./chart";
+import { type ChartPick, axisIncrs, axisSize, chartData, drawMarker, hasValues, themeColor } from "./chart";
 import type { MetricPoint } from "./useServerMetrics";
 
 const PALETTE: [string, string][] = [
@@ -25,13 +25,15 @@ interface Props {
   format: (v: number) => string;
   /** Наименьший шаг делений оси Y — точность `format` (0,1 у десятых, 1 у целых). */
   step: number;
+  /** Вертикальная пунктирная линия на этом времени (мс): граница истории в окне «Сейчас». */
+  marker?: number | null;
 }
 
 /**
  * График метрики сервера за окно «сейчас» (спец. I1b §6): ряды по `picks`, разрывы на ошибках опроса, «нет данных»,
  * если значений нет. По образцу `UsageChart` Kubernetes-версии: цвета из CSS-переменных темы, ширина — по контейнеру.
  */
-export function MetricChart({ title, points, picks, format, step }: Props) {
+export function MetricChart({ title, points, picks, format, step, marker = null }: Props) {
   const { t } = useTranslation();
   const ref = useRef<HTMLDivElement>(null);
   const data = useMemo(() => chartData(points, picks), [points, picks]);
@@ -44,6 +46,8 @@ export function MetricChart({ title, points, picks, format, step }: Props) {
   dataRef.current = data;
   const formatRef = useRef(format);
   formatRef.current = format;
+  const markerRef = useRef(marker);
+  markerRef.current = marker;
 
   useEffect(() => {
     const el = ref.current;
@@ -56,6 +60,7 @@ export function MetricChart({ title, points, picks, format, step }: Props) {
         width: el.clientWidth || 600,
         height: HEIGHT,
         title,
+        hooks: { draw: [(u: uPlot) => drawMarker(u, markerRef.current, muted)] },
         axes: [
           axis,
           {
@@ -93,6 +98,9 @@ export function MetricChart({ title, points, picks, format, step }: Props) {
   useEffect(() => {
     plotRef.current?.setData(data as uPlot.AlignedData);
   }, [data]);
+  useEffect(() => {
+    plotRef.current?.redraw?.();
+  }, [marker]);
 
   return (
     <section className="metric-chart" aria-label={title}>
````

<!-- block: patch t09-src-components-servers-servermetricsview-tsx step=9.3 -->
````
diff --git a/src/components/servers/ServerMetricsView.tsx b/src/components/servers/ServerMetricsView.tsx
index ad6f996..19fe424 100644
--- a/src/components/servers/ServerMetricsView.tsx
+++ b/src/components/servers/ServerMetricsView.tsx
@@ -1,23 +1,48 @@
-import { useMemo } from "react";
+import { useMemo, useState } from "react";
 import { useTranslation } from "react-i18next";
-import type { ServerMetrics } from "../../api/types";
+import { toAppError } from "../../api/client";
+import type { AppError, HistoryWindow, ServerMetrics } from "../../api/types";
+import { currentLocale } from "../../i18n";
+import { ErrorBanner } from "../common/ErrorBanner";
 import type { ChartPick } from "./chart";
 import { fmtAdaptive, fmtNumber, fmtPercent } from "./format";
 import { MetricChart } from "./MetricChart";
-import type { MetricPoint } from "./useServerMetrics";
+import { type MetricPoint, useServerHistory } from "./useServerMetrics";
 
-/** Окна метрик: «сейчас» — из буфера опроса; история — с этапа I1c. */
-const WINDOWS = ["now", "1h", "24h", "7d", "30d"] as const;
+/** Окна метрик: «сейчас» — из буфера опроса (и истории до него), остальные — из источника истории. */
+const WINDOWS: HistoryWindow[] = ["now", "1h", "24h", "7d", "30d"];
 
 const lat = (k: "ttft" | "itl" | "e2e", q: "p50" | "p95" | "p99") => (m: ServerMetrics) => m.latency[k]?.[q];
 
-/** «Метрики»: графики за окно «сейчас» (до часа в памяти вкладки). */
-export function ServerMetricsView({ points }: { points: MetricPoint[] }) {
+const EMPTY: MetricPoint[] = [];
+
+/** Время «HH:MM» по языку интерфейса. */
+const hhmm = (at: number) => new Date(at).toLocaleTimeString(currentLocale(), { hour: "2-digit", minute: "2-digit" });
+
+interface Props {
+  installationId: string;
+  serverId: string;
+  /** Буфер «Сейчас». */
+  points: MetricPoint[];
+  /** До какого времени буфер заполнен из истории; `null` — не заполнялся. */
+  historyUntil: number | null;
+  /** Почему буфер не заполнен из истории. */
+  historyError: AppError | null;
+  /** `null` — история сервера доступна; иначе ключ подсказки, почему нет. */
+  historyUnavailable: string | null;
+}
+
+/** «Метрики» (спека I1c §5): графики за окно «сейчас» или окно истории 1 ч…30 д. */
+export function ServerMetricsView({ installationId, serverId, points, historyUntil, historyError, historyUnavailable }: Props) {
   const { t } = useTranslation();
-  // Причины завершения — из буфера; от точек зависит только их список, иначе графики пересоздавались бы каждый опрос.
+  const [win, setWin] = useState<HistoryWindow>("now");
+  const fromHistory = win !== "now";
+  const hist = useServerHistory(installationId, serverId, fromHistory ? win : "1h", fromHistory && !historyUnavailable);
+  const shown = fromHistory ? (hist.data?.points ?? EMPTY) : points;
+  // Причины завершения — из точек; от точек зависит только их список, иначе графики пересоздавались бы каждый опрос.
   // Прерывания vLLM приходят и причиной `abort`, и отдельным счётчиком: если счётчик есть, причину не рисуем второй раз.
-  const hasAborted = points.some((x) => x.m?.rates?.aborted != null);
-  const reasonsKey = [...new Set(points.flatMap((x) => Object.keys(x.m?.rates?.finishedByReason ?? {})))]
+  const hasAborted = shown.some((x) => x.m?.rates?.aborted != null);
+  const reasonsKey = [...new Set(shown.flatMap((x) => Object.keys(x.m?.rates?.finishedByReason ?? {})))]
     .filter((r) => !(hasAborted && r === "abort"))
     .sort()
     .join("\n");
@@ -43,20 +68,60 @@ export function ServerMetricsView({ points }: { points: MetricPoint[] }) {
       },
     ];
   }, [reasonsKey, t]);
+  const title = (w: HistoryWindow) => {
+    if (w === "now") return historyError ? t("servers.metrics.prefillFailed", { error: historyError.message }) : undefined;
+    return historyUnavailable ? t(historyUnavailable) : undefined;
+  };
+  const step = (sec: number) => (sec % 60 === 0 ? t("servers.units.min", { value: sec / 60 }) : t("servers.units.sec", { value: sec }));
   return (
     <div className="server-metrics">
       <div className="toolbar" role="group" aria-label={t("servers.metrics.window")}>
         {WINDOWS.map((w) => (
-          <button key={w} type="button" className={w === "now" ? "active" : undefined} aria-pressed={w === "now"} disabled={w !== "now"} title={w === "now" ? undefined : t("servers.metrics.historyLater")}>
+          <button
+            key={w}
+            type="button"
+            className={w === win ? "active" : undefined}
+            aria-pressed={w === win}
+            disabled={w !== "now" && !!historyUnavailable}
+            title={title(w)}
+            onClick={() => setWin(w)}
+          >
             {t(`servers.metrics.windows.${w}`)}
           </button>
         ))}
+        {fromHistory && (
+          <button type="button" onClick={() => void hist.refetch()} disabled={hist.isFetching}>
+            {t("common.refresh")}
+          </button>
+        )}
+        {!fromHistory && historyUntil != null && <span className="muted">{t("servers.metrics.historyUntil", { time: hhmm(historyUntil) })}</span>}
+        {fromHistory && hist.data && (
+          <span className="muted">{t("servers.metrics.step", { step: step(hist.data.stepSec), source: t(`servers.metrics.sources.${hist.data.source}`) })}</span>
+        )}
       </div>
-      <div className="charts">
-        {charts.map((c) => (
-          <MetricChart key={c.id} title={t(`servers.charts.${c.id}`)} points={points} picks={c.picks} format={c.format} step={c.step} />
-        ))}
-      </div>
+      {fromHistory && hist.error && <ErrorBanner error={toAppError(hist.error)} onRetry={() => void hist.refetch()} />}
+      {fromHistory && hist.data && hist.data.partial.length > 0 && (
+        <p className="warn" role="status">
+          {t("servers.metrics.partial", {
+            charts: hist.data.partial.map((x) => `${t(`servers.charts.${x.chart}`)} — ${x.error.message}`).join("; "),
+          })}
+        </p>
+      )}
+      {!(fromHistory && hist.error) && (
+        <div className={fromHistory && hist.isFetching ? "charts loading" : "charts"} aria-busy={fromHistory && hist.isFetching}>
+          {charts.map((c) => (
+            <MetricChart
+              key={c.id}
+              title={t(`servers.charts.${c.id}`)}
+              points={shown}
+              picks={c.picks}
+              format={c.format}
+              step={c.step}
+              marker={fromHistory ? null : historyUntil}
+            />
+          ))}
+        </div>
+      )}
     </div>
   );
 }
````

<!-- block: patch t09-src-components-servers-serverpanel-tsx step=9.3 -->
````
diff --git a/src/components/servers/ServerPanel.tsx b/src/components/servers/ServerPanel.tsx
index 2ee0c0e..9a3175f 100644
--- a/src/components/servers/ServerPanel.tsx
+++ b/src/components/servers/ServerPanel.tsx
@@ -34,7 +34,14 @@ export function ServerPanel({ installationId, serverId }: { installationId: stri
     enabled: !!server,
   });
   const state = info.data?.servers.find((s) => s.serverId === serverId);
-  const live = useServerMetrics(installationId, serverId, readsMetrics(state?.facts));
+  // История — если у инсталляции есть источник, а серии сервера сопоставлены (спека I1c §5).
+  const historyUnavailable = !profile?.history
+    ? "servers.metrics.noSource"
+    : !server?.historySelector
+      ? "servers.metrics.notMatched"
+      : null;
+  const reads = readsMetrics(state?.facts);
+  const live = useServerMetrics(installationId, serverId, reads, reads && !historyUnavailable);
   // Предупреждение о перезапуске живёт минуту с момента сброса, а не один опрос (ответ после сброса уже `reset: false`).
   const [resetAt, setResetAt] = useState<number | null>(null);
   const [, redraw] = useState(0);
@@ -84,7 +91,16 @@ export function ServerPanel({ installationId, serverId }: { installationId: stri
           </div>
           {sub === "overview" && <ServerOverview facts={state.facts} latest={live.error ? undefined : live.latest} restarted={restarted} error={!!live.error} />}
           {sub === "config" && <ServerConfigView installationId={installationId} serverId={serverId} />}
-          {sub === "metrics" && <ServerMetricsView points={live.points} />}
+          {sub === "metrics" && (
+            <ServerMetricsView
+              installationId={installationId}
+              serverId={serverId}
+              points={live.points}
+              historyUntil={live.historyUntil}
+              historyError={live.historyError}
+              historyUnavailable={historyUnavailable}
+            />
+          )}
         </>
       )}
     </div>
````

<!-- block: patch t09-src-components-servers-chart-ts step=9.3 -->
````
diff --git a/src/components/servers/chart.ts b/src/components/servers/chart.ts
index 77faf15..c7f273c 100644
--- a/src/components/servers/chart.ts
+++ b/src/components/servers/chart.ts
@@ -44,3 +44,27 @@ export function axisSize(labels: string[] | null, measure: (s: string) => number
   const widest = Math.max(0, ...(labels ?? []).map(measure));
   return Math.max(40, Math.ceil(widest) + 16);
 }
+
+/** Часть uPlot, которой рисуется граница: холст, область графика и перевод времени в пиксели. */
+export interface MarkerTarget {
+  ctx: CanvasRenderingContext2D;
+  bbox: { left: number; top: number; width: number; height: number };
+  valToPos: (v: number, scale: string, canvasPixels?: boolean) => number;
+}
+
+/** Пунктирная вертикаль на времени `at` (мс) внутри области графика; вне её и без `at` — ничего. */
+export function drawMarker(u: MarkerTarget, at: number | null, color: string) {
+  if (at == null) return;
+  const x = u.valToPos(at / 1000, "x", true);
+  const { left, top, width, height } = u.bbox;
+  if (!(x >= left && x <= left + width)) return;
+  const ctx = u.ctx;
+  ctx.save();
+  ctx.strokeStyle = color;
+  ctx.setLineDash([4, 4]);
+  ctx.beginPath();
+  ctx.moveTo(x, top);
+  ctx.lineTo(x, top + height);
+  ctx.stroke();
+  ctx.restore();
+}
````

<!-- block: patch t09-src-components-servers-useservermetrics-ts step=9.3 -->
````
diff --git a/src/components/servers/useServerMetrics.ts b/src/components/servers/useServerMetrics.ts
index be65d50..0cc0e91 100644
--- a/src/components/servers/useServerMetrics.ts
+++ b/src/components/servers/useServerMetrics.ts
@@ -1,4 +1,4 @@
-import { useQuery } from "@tanstack/react-query";
+import { keepPreviousData, useQuery } from "@tanstack/react-query";
 import { useEffect, useState } from "react";
 import { api, toAppError } from "../../api/client";
 import type { AppError, HistoryWindow, ServerFacts, ServerMetrics } from "../../api/types";
@@ -56,6 +56,8 @@ export function useServerHistory(installationId: string, serverId: string, windo
     queryFn: () => api.readServerHistory(installationId, serverId, window),
     enabled,
     staleTime: 60_000,
+    // Пока грузится другое окно, видны прежние графики (под слоем загрузки).
+    placeholderData: keepPreviousData,
     retry: false,
     refetchOnWindowFocus: false,
   });
````

<!-- block: locales t09-locales-t09 step=9.3 -->
````
{
  "en": {
    "common": { "refresh": "Refresh" },
    "servers": {
      "units": { "sec": "{{value}} s", "min": "{{value}} min" },
      "metrics": {
        "noSource": "Add a metrics history source to the installation",
        "notMatched": "The series of the server are not matched with the history",
        "historyUntil": "History until {{time}}",
        "prefillFailed": "History is not loaded: {{error}}",
        "step": "Step {{step}} · {{source}}",
        "sources": { "prometheus": "Prometheus", "victoriametrics": "VictoriaMetrics" },
        "partial": "Some series are not read: {{charts}}"
      }
    }
  },
  "ru": {
    "common": { "refresh": "Обновить" },
    "servers": {
      "units": { "sec": "{{value}} с", "min": "{{value}} мин" },
      "metrics": {
        "noSource": "Добавьте источник истории в инсталляцию",
        "notMatched": "Серии сервера не сопоставлены с историей",
        "historyUntil": "История до {{time}}",
        "prefillFailed": "История не загружена: {{error}}",
        "step": "Шаг {{step}} · {{source}}",
        "sources": { "prometheus": "Prometheus", "victoriametrics": "VictoriaMetrics" },
        "partial": "Часть рядов не прочитана: {{charts}}"
      }
    }
  }
}
````

- [ ] **Шаг 9.4: Проверки.**

Run: `pnpm test`
Expected: `Test Files  35 passed`, `Tests  243 passed`
Run: `pnpm exec tsc --noEmit`
Expected: без вывода
Run: `pnpm i18n:check`
Expected: `✓ словари en, ru согласованы, вызовов из Rust: 139`

- [ ] **Шаг 9.5: Коммит.**

<!-- block: shell t09-commit step=9.5 -->
````
git add docs/i18n/same-as-en.json \
  src/api/types.ts \
  src/app.css \
  src/components/servers/MetricChart.test.tsx \
  src/components/servers/MetricChart.tsx \
  src/components/servers/ServerMetricsView.tsx \
  src/components/servers/ServerPanel.test.tsx \
  src/components/servers/ServerPanel.tsx \
  src/components/servers/chart.ts \
  src/components/servers/useServerMetrics.ts \
  src/locales/en.json \
  src/locales/ru.json
git commit -s -q -F - <<'EOF'
feat(history): history windows on the Metrics tab

Windows 1h/24h/7d/30d are enabled when the installation has a history
source and the server a selector; otherwise a hint names the reason. A
window is read on selection, previous charts stay under a loading layer,
"Refresh" reads it again, and the caption shows the step and the source.
Partial windows list the charts that failed, a failed window shows its
error. In "now" the buffer starts from history with a dashed boundary.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
````

---

### Задача 10: Раздел «История метрик» в форме инсталляции

Раздел формы (спека I1c §5): адрес, вход (без входа, токен, имя и пароль) с секретом только в keychain, CA,
прокси, «Проверить источник» с шагами, «Серии в истории» — у каждого сервера найденные цели, свой селектор или «Не
сопоставлять». Автоподбор проверки ставит селектор только серверам без него.

**Files:**
- Create: `src/components/installations/HistoryFields.tsx`
- Modify: `src/app.css`
- Modify: `src/components/installations/InstallationForm.test.tsx`
- Modify: `src/components/installations/InstallationForm.tsx`
- Modify: `src/locales/en.json`
- Modify: `src/locales/ru.json`

**Interfaces:**
- Consumes: `api.checkHistorySource`, `withCaMode`, `withProxyMode`.
- Produces: `HistoryFields` (`src/components/installations/HistoryFields.tsx`), `newHistory`, `withAuthMode`,
  `targetSelector`, `SecretInput`; `InstallationForm` передаёт `historySecret` в `secrets()`.

- [ ] **Шаг 10.1: Тесты.** Тесты формы: проверка с автоподбором и сохранением, выбор человека, «Не сопоставлять», «Убрать источник».

<!-- block: patch t10-src-components-installations-installationform-test-tsx step=10.1 -->
````
diff --git a/src/components/installations/InstallationForm.test.tsx b/src/components/installations/InstallationForm.test.tsx
index a7f5524..da16c81 100644
--- a/src/components/installations/InstallationForm.test.tsx
+++ b/src/components/installations/InstallationForm.test.tsx
@@ -9,6 +9,7 @@ vi.mock("../../api/client", async (importOriginal) => {
     ...orig,
     api: {
       testServer: vi.fn(),
+      checkHistorySource: vi.fn(),
       saveInstallation: vi.fn(),
       deleteInstallation: vi.fn(),
       listInstallations: vi.fn().mockResolvedValue([]),
@@ -57,6 +58,100 @@ beforeEach(() => {
   useWorkspace.setState({ tabs: [], activeId: null });
 });
 
+const AUTO = '{job="vllm", instance="gpu-1:8000"}';
+
+const CHECK = {
+  report: {
+    ok: true,
+    steps: [
+      { name: "auth" as const, ok: true, message: msg("messages.historyTest.source", "Prometheus 3.15.1", { kind: "Prometheus", version: "3.15.1" }) },
+      { name: "targets" as const, ok: true, message: msg("messages.historyTest.targets", "Inference targets found: 2", { count: "2" }) },
+    ],
+    warnings: [msg("messages.historyTest.noTarget", "no target", { server: "gpu-2" })],
+  },
+  source: { kind: "prometheus" as const, version: "3.15.1" },
+  targets: [
+    { job: "sglang", instance: "gpu-3:30000", kind: "sglang" as const },
+    { job: "vllm", instance: "gpu-1:8000", kind: "vllm" as const },
+  ],
+  suggestions: [
+    { serverId: "s1", selector: AUTO, targets: [{ job: "vllm", instance: "gpu-1:8000", kind: "vllm" as const }] },
+    { serverId: "s2", selector: null, targets: [] },
+  ],
+};
+
+describe("InstallationForm: история метрик", () => {
+  it("источник, проверка с автоподбором, свой селектор — всё уходит в сохранение, токен — секретом", async () => {
+    vi.mocked(api.checkHistorySource).mockResolvedValue(CHECK);
+    vi.mocked(api.saveInstallation).mockImplementation(async (p) => p);
+    const user = userEvent.setup();
+    renderWithClient(<InstallationForm initial={saved()} tabId="form:i1" />);
+    const section = screen.getByRole("region", { name: "История метрик (Prometheus / VictoriaMetrics)" });
+    await user.click(within(section).getByRole("button", { name: "Добавить источник истории" }));
+    await user.type(within(section).getByLabelText("Адрес"), "http://prometheus:9090");
+    await user.selectOptions(within(section).getByLabelText("Вход"), "bearer");
+    await user.type(within(section).getByLabelText("Токен"), "prom-t0ken");
+    await user.click(within(section).getByRole("button", { name: "Проверить источник" }));
+    expect(api.checkHistorySource).toHaveBeenCalledWith(
+      expect.objectContaining({ history: { url: "http://prometheus:9090", auth: { mode: "bearer" }, ca: { mode: "system" }, proxy: { mode: "system" } } }),
+      { apiKeys: {}, historySecret: "prom-t0ken" },
+    );
+    const report = await within(section).findByRole("list", { name: "Проверка источника истории" });
+    expect(within(report).getByText("Цели:")).toBeInTheDocument();
+    expect(within(report).getByText(/«gpu-2»: цели с адресом сервера нет — выберите цель вручную/)).toBeInTheDocument();
+    const gpu1 = within(section).getByLabelText("gpu-1") as HTMLSelectElement;
+    expect(gpu1.value).toBe(AUTO);
+    expect(within(section).getByText("автоподбор")).toBeInTheDocument();
+    expect(within(gpu1).getByRole("option", { name: "sglang · gpu-3:30000 (SGLang)" })).toBeInTheDocument();
+    await user.selectOptions(within(section).getByLabelText("gpu-2"), "Свой селектор…");
+    await user.type(within(section).getByLabelText("Селектор серий «gpu-2»"), '{{job="x"}');
+    await user.click(screen.getByRole("button", { name: "Сохранить" }));
+    await waitFor(() => expect(api.saveInstallation).toHaveBeenCalled());
+    const [profile, secrets] = vi.mocked(api.saveInstallation).mock.calls[0];
+    expect(profile.servers.map((s) => s.historySelector)).toEqual([AUTO, '{job="x"}']);
+    expect(profile.history?.auth).toEqual({ mode: "bearer" });
+    expect(secrets).toEqual({ apiKeys: {}, historySecret: "prom-t0ken" });
+    expect(JSON.stringify(profile)).not.toContain("prom-t0ken");
+  });
+
+  it("выбор человека автоподбор не перезаписывает; «Не сопоставлять» убирает селектор", async () => {
+    vi.mocked(api.checkHistorySource).mockResolvedValue(CHECK);
+    vi.mocked(api.saveInstallation).mockImplementation(async (p) => p);
+    const initial = saved();
+    initial.history = { url: "http://prometheus:9090", auth: { mode: "basic", username: "reader" }, ca: { mode: "system" }, proxy: { mode: "system" } };
+    initial.servers[0].historySelector = '{job="mine"}';
+    const user = userEvent.setup();
+    renderWithClient(<InstallationForm initial={initial} tabId="form:i1" />);
+    const section = screen.getByRole("region", { name: "История метрик (Prometheus / VictoriaMetrics)" });
+    expect(within(section).getByLabelText("Пароль")).toHaveAttribute("placeholder", "оставьте пустым, чтобы не менять сохранённый");
+    await user.click(within(section).getByRole("button", { name: "Проверить источник" }));
+    await within(section).findByRole("list", { name: "Проверка источника истории" });
+    expect(within(section).getByLabelText("Селектор серий «gpu-1»")).toHaveValue('{job="mine"}');
+    expect(within(section).queryByText("автоподбор")).toBeNull();
+    await user.selectOptions(within(section).getByLabelText("gpu-1"), "Не сопоставлять");
+    await user.click(screen.getByRole("button", { name: "Сохранить" }));
+    await waitFor(() => expect(api.saveInstallation).toHaveBeenCalledTimes(1));
+    const [first, secrets] = vi.mocked(api.saveInstallation).mock.calls[0];
+    expect(first.servers[0].historySelector).toBeUndefined();
+    expect(secrets?.historySecret).toBeUndefined();
+  });
+
+  it("«Убрать источник» — без него и секрета", async () => {
+    vi.mocked(api.saveInstallation).mockImplementation(async (p) => p);
+    const initial = saved();
+    initial.history = { url: "http://prometheus:9090", auth: { mode: "bearer" }, ca: { mode: "system" }, proxy: { mode: "system" } };
+    const user = userEvent.setup();
+    renderWithClient(<InstallationForm initial={initial} tabId="form:i1" />);
+    await user.click(screen.getByRole("button", { name: "Убрать источник" }));
+    expect(screen.getByRole("button", { name: "Добавить источник истории" })).toBeInTheDocument();
+    await user.click(screen.getByRole("button", { name: "Сохранить" }));
+    await waitFor(() => expect(api.saveInstallation).toHaveBeenCalled());
+    const [profile, secrets] = vi.mocked(api.saveInstallation).mock.calls[0];
+    expect(profile.history).toBeUndefined();
+    expect(secrets).toEqual({ apiKeys: {} });
+  });
+});
+
 describe("InstallationForm", () => {
   it("сохраняет новую инсталляцию с ключом сервера и открывает её вкладку", async () => {
     vi.mocked(api.saveInstallation).mockImplementation(async (p) => ({ ...p, id: "new-id" }));
````

- [ ] **Шаг 10.2: Увидеть падение.**

Run: `pnpm exec vitest run src/components/installations/InstallationForm.test.tsx`
Expected: FAIL — нет раздела «История метрик (Prometheus / VictoriaMetrics)», `api.checkHistorySource` не вызывается

- [ ] **Шаг 10.3: Реализация.** Раздел формы, подключение к форме, стиль строк, словари.

<!-- block: patch t10-src-app-css step=10.3 -->
````
diff --git a/src/app.css b/src/app.css
index 10e9e29..06a3d48 100644
--- a/src/app.css
+++ b/src/app.css
@@ -31,6 +31,7 @@ input::placeholder, textarea::placeholder { font-style: italic; color: var(--mut
 .charts { display: grid; grid-template-columns: repeat(auto-fill, minmax(380px, 1fr)); gap: 12px; margin-top: 8px; }
 .metric-chart { min-width: 0; }
 .charts.loading { opacity: 0.5; }
+.history-source .contents { display: contents; }
 .metric-chart h4 { margin: 0 0 4px; font-size: 13px; }
 .server-metrics .toolbar button.active { border-color: var(--accent); font-weight: 600; }
 .badge { border: 1px solid var(--border); border-radius: 10px; padding: 0 6px; font-size: 12px; color: var(--muted); white-space: nowrap; }
````

<!-- block: file t10-src-components-installations-historyfields-tsx step=10.3 path=src/components/installations/HistoryFields.tsx -->
````
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { toAppError } from "../../api/client";
import type { AppError, HistoryAuth, HistoryCheck, HistorySource, HistoryTarget, InstallationProfile } from "../../api/types";
import { tr } from "../../i18n/text";
import { ErrorBanner } from "../common/ErrorBanner";
import { Hint } from "../common/Hint";
import { type CaMode, type ProxyMode, withCaMode, withProxyMode } from "./profile";

const CA_MODES: CaMode[] = ["system", "pem", "insecure"];
const PROXY_MODES: ProxyMode[] = ["system", "none", "url"];
const AUTH_MODES: HistoryAuth["mode"][] = ["none", "bearer", "basic"];

/** Значение списка «Серии в истории»: свой селектор и «не сопоставлять». */
const CUSTOM = "\u0000custom";
const NONE = "";

/** Секрет источника в форме: введённое значение и «удалить сохранённый». */
export interface SecretInput {
  value: string;
  clear: boolean;
}

export function newHistory(): HistorySource {
  return { url: "", auth: { mode: "none" }, ca: { mode: "system" }, proxy: { mode: "system" } };
}

/** Вид входа; имя пользователя сохраняется при переключении туда и обратно. */
export function withAuthMode(auth: HistoryAuth, mode: HistoryAuth["mode"], lastUser = ""): HistoryAuth {
  if (mode === "basic") return { mode, username: auth.mode === "basic" ? auth.username : lastUser };
  return { mode };
}

/** Подпись цели: `job · instance (vLLM)`. */
const targetLabel = (t: HistoryTarget) => `${t.job} · ${t.instance} (${t.kind === "sglang" ? "SGLang" : "vLLM"})`;

/** Селектор цели — как его строит Rust (`Target::selector`). */
export const targetSelector = (t: HistoryTarget) =>
  `{job="${t.job.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}", instance="${t.instance.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"}`;

interface Props {
  profile: InstallationProfile;
  idPrefix: string;
  /** У источника сохранён секрет: пустое поле его оставляет. */
  savedSecret: boolean;
  secret: SecretInput;
  insecureForbidden: boolean;
  busy: boolean;
  onHistory: (history: HistorySource | undefined) => void;
  onSecret: (patch: Partial<SecretInput>) => void;
  onSelector: (serverId: string, selector: string | undefined) => void;
  /** Проверка источника профиля формы с секретом формы. */
  check: () => Promise<HistoryCheck>;
}

/**
 * Раздел формы «История метрик» (спека I1c §5): источник, его проверка и сопоставление серверов с целями.
 * Автоподбор проверки ставит селектор только серверам без него — выбор человека не перезаписывается.
 */
export function HistoryFields({ profile, idPrefix, savedSecret, secret, insecureForbidden, busy, onHistory, onSecret, onSelector, check }: Props) {
  const { t } = useTranslation();
  const id = (field: string) => `${idPrefix}-${field}`;
  const h = profile.history;
  const [result, setResult] = useState<HistoryCheck | null>(null);
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState<AppError | null>(null);
  const [auto, setAuto] = useState<Set<string>>(new Set());
  const [custom, setCustom] = useState<Set<string>>(new Set());
  const [lastPem, setLastPem] = useState("");
  const [lastProxy, setLastProxy] = useState("");
  const [lastUser, setLastUser] = useState("");

  if (!h) {
    return (
      <section className="history-source" aria-label={t("installations.history.title")}>
        <h3>{t("installations.history.title")}</h3>
        <p className="muted">{t("installations.history.none")}</p>
        <div className="toolbar">
          <button type="button" disabled={busy} onClick={() => onHistory(newHistory())}>
            {t("installations.history.add")}
          </button>
        </div>
      </section>
    );
  }

  const set = (patch: Partial<HistorySource>) => onHistory({ ...h, ...patch });
  const onCheck = async () => {
    setChecking(true);
    setError(null);
    setResult(null);
    try {
      const r = await check();
      setResult(r);
      const picked = new Set<string>();
      for (const s of r.suggestions) {
        const server = profile.servers.find((x) => x.id === s.serverId);
        if (s.selector && server && !server.historySelector) {
          onSelector(s.serverId, s.selector);
          picked.add(s.serverId);
        }
      }
      setAuto((a) => new Set([...a, ...picked]));
    } catch (e) {
      setError(toAppError(e));
    } finally {
      setChecking(false);
    }
  };
  const targets = result?.targets ?? [];
  const choose = (serverId: string, value: string) => {
    setAuto((a) => new Set([...a].filter((x) => x !== serverId)));
    setCustom((c) => {
      const next = new Set(c);
      if (value === CUSTOM) next.add(serverId);
      else next.delete(serverId);
      return next;
    });
    if (value !== CUSTOM) onSelector(serverId, value === NONE ? undefined : value);
  };

  return (
    <section className="history-source" aria-label={t("installations.history.title")}>
      <h3>{t("installations.history.title")}</h3>
      <div className="form">
        <span className="field-label">
          <label htmlFor={id("url")}>{t("installations.history.url")}</label>
          <Hint k="hints.installations.historyUrl" label={t("installations.history.url")} />
        </span>
        <input id={id("url")} placeholder="http://prometheus.monitoring:9090" value={h.url} onChange={(e) => set({ url: e.target.value })} />

        <label htmlFor={id("auth")}>{t("installations.history.auth")}</label>
        <select
          id={id("auth")}
          value={h.auth.mode}
          onChange={(e) => {
            if (h.auth.mode === "basic") setLastUser(h.auth.username);
            set({ auth: withAuthMode(h.auth, e.target.value as HistoryAuth["mode"], lastUser) });
          }}
        >
          {AUTH_MODES.map((m) => (
            <option key={m} value={m}>
              {t(`installations.history.authMode.${m}`)}
            </option>
          ))}
        </select>
        {h.auth.mode === "basic" && (
          <>
            <label htmlFor={id("user")}>{t("installations.history.username")}</label>
            <input id={id("user")} autoComplete="off" value={h.auth.username} onChange={(e) => set({ auth: { mode: "basic", username: e.target.value } })} />
          </>
        )}
        {h.auth.mode !== "none" && (
          <>
            <label htmlFor={id("secret")}>{t(h.auth.mode === "bearer" ? "installations.history.token" : "installations.history.password")}</label>
            <input
              id={id("secret")}
              type="password"
              autoComplete="off"
              placeholder={savedSecret ? t("installations.form.apiKeyKeep") : ""}
              disabled={secret.clear}
              value={secret.value}
              onChange={(e) => onSecret({ value: e.target.value })}
            />
            {savedSecret && (
              <label className="inline full">
                <input type="checkbox" checked={secret.clear} onChange={(e) => onSecret({ clear: e.target.checked })} /> {t("installations.history.secretClear")}
              </label>
            )}
          </>
        )}

        <label htmlFor={id("ca")}>{t("installations.form.ca")}</label>
        <select
          id={id("ca")}
          value={h.ca.mode}
          onChange={(e) => {
            if (h.ca.mode === "pem") setLastPem(h.ca.pem);
            set({ ca: withCaMode(h.ca, e.target.value as CaMode, lastPem) });
          }}
        >
          {CA_MODES.map((m) => (
            <option key={m} value={m} disabled={m === "insecure" && insecureForbidden && h.ca.mode !== "insecure"}>
              {t(`installations.form.caMode.${m}`)}
            </option>
          ))}
        </select>
        {h.ca.mode === "pem" && (
          <textarea
            className="full"
            aria-label={t("installations.form.caPem")}
            rows={4}
            placeholder="-----BEGIN CERTIFICATE-----"
            value={h.ca.pem}
            onChange={(e) => set({ ca: { mode: "pem", pem: e.target.value } })}
          />
        )}
        {h.ca.mode === "insecure" && (
          <p className="warn full" role="alert">
            {insecureForbidden ? t("installations.form.insecureForbidden") : t("installations.form.insecureWarning")}
          </p>
        )}

        <label htmlFor={id("proxy")}>{t("installations.form.proxy")}</label>
        <select
          id={id("proxy")}
          value={h.proxy.mode}
          onChange={(e) => {
            if (h.proxy.mode === "url") setLastProxy(h.proxy.url);
            set({ proxy: withProxyMode(h.proxy, e.target.value as ProxyMode, lastProxy) });
          }}
        >
          {PROXY_MODES.map((m) => (
            <option key={m} value={m}>
              {t(`installations.form.proxyMode.${m}`)}
            </option>
          ))}
        </select>
        {h.proxy.mode === "url" && (
          <input
            className="full"
            aria-label={t("installations.form.proxyUrl")}
            placeholder="http://proxy.example.com:3128"
            value={h.proxy.url}
            onChange={(e) => set({ proxy: { mode: "url", url: e.target.value } })}
          />
        )}
      </div>
      <div className="toolbar">
        <button type="button" disabled={busy || checking} onClick={() => void onCheck()}>
          {t("installations.history.check")}
        </button>
        <button type="button" disabled={busy} onClick={() => onHistory(undefined)}>
          {t("installations.history.remove")}
        </button>
        {checking && <span className="muted">{t("installations.form.testing")}</span>}
      </div>
      {error && <ErrorBanner error={error} />}
      {result && (
        <ul className="steps" aria-label={t("installations.history.report")}>
          {result.report.steps.map((step, i) => (
            <li key={`${step.name}-${i}`}>
              {step.ok ? "✅" : "❌"} <strong>{t(`installations.form.step.${step.name}`)}:</strong> <span>{tr(step.message)}</span>
              {step.error?.details && <pre className="muted">{step.error.details}</pre>}
            </li>
          ))}
          {result.report.warnings.map((w, i) => (
            <li key={`${w.key}-${i}`} className="warn">
              ⚠ {tr(w)}
            </li>
          ))}
        </ul>
      )}

      <h4>{t("installations.history.series")}</h4>
      <div className="form">
        {profile.servers.map((s) => {
          const sel = s.historySelector;
          const known = targets.find((x) => targetSelector(x) === sel);
          const value = custom.has(s.id) || (sel && !known) ? CUSTOM : (sel ?? NONE);
          const name = s.name.trim() || s.baseUrl || s.id;
          return (
            <div key={s.id} className="contents">
              <span className="field-label">
                <label htmlFor={id(`sel-${s.id}`)}>{name}</label>
                {auto.has(s.id) && sel && <span className="muted"> {t("installations.history.auto")}</span>}
              </span>
              <select id={id(`sel-${s.id}`)} value={value} onChange={(e) => choose(s.id, e.target.value)}>
                <option value={NONE}>{t("installations.history.noMatch")}</option>
                {targets.map((x) => (
                  <option key={targetSelector(x)} value={targetSelector(x)}>
                    {targetLabel(x)}
                  </option>
                ))}
                <option value={CUSTOM}>{t("installations.history.custom")}</option>
              </select>
              {value === CUSTOM && (
                <input
                  className="full"
                  aria-label={t("installations.history.selector", { server: name })}
                  placeholder='{job="vllm", instance="10.0.0.5:8000"}'
                  value={sel ?? ""}
                  onChange={(e) => onSelector(s.id, e.target.value || undefined)}
                />
              )}
            </div>
          );
        })}
      </div>
    </section>
  );
}
````

<!-- block: patch t10-src-components-installations-installationform-tsx step=10.3 -->
````
diff --git a/src/components/installations/InstallationForm.tsx b/src/components/installations/InstallationForm.tsx
index c374d62..0e90623 100644
--- a/src/components/installations/InstallationForm.tsx
+++ b/src/components/installations/InstallationForm.tsx
@@ -7,6 +7,7 @@ import { tr } from "../../i18n/text";
 import { useWorkspace } from "../../state/workspace";
 import { ErrorBanner } from "../common/ErrorBanner";
 import { Hint } from "../common/Hint";
+import { HistoryFields, type SecretInput } from "./HistoryFields";
 import { type CaMode, isPlainHttp, MAX_SERVERS, newServer, type ProxyMode, withCaMode, withProxyMode } from "./profile";
 import { folderPaths } from "./tree/folderTree";
 
@@ -29,6 +30,8 @@ export function InstallationForm({ initial, tabId }: { initial: InstallationProf
   const [profile, setProfile] = useState(initial);
   // Ключи из формы: пустое поле — оставить сохранённый (у нового сервера его нет).
   const [keys, setKeys] = useState<Record<string, KeyInput>>({});
+  // Секрет источника истории: пустое поле — оставить сохранённый.
+  const [historySecret, setHistorySecret] = useState<SecretInput>({ value: "", clear: false });
   // Последние введённые PEM и адрес прокси по серверам: переключение режима туда и обратно их не теряет.
   const [lastPem, setLastPem] = useState<Record<string, string>>({});
   const [lastProxy, setLastProxy] = useState<Record<string, string>>({});
@@ -69,7 +72,12 @@ export function InstallationForm({ initial, tabId }: { initial: InstallationProf
       if (k?.clear) apiKeys[s.id] = "";
       else if (k?.value.trim()) apiKeys[s.id] = k.value;
     }
-    return { apiKeys };
+    const out: SecretsInput = { apiKeys };
+    if (profile.history && profile.history.auth.mode !== "none") {
+      if (historySecret.clear) out.historySecret = "";
+      else if (historySecret.value.trim()) out.historySecret = historySecret.value;
+    }
+    return out;
   };
 
   async function run(fn: () => Promise<void>) {
@@ -259,6 +267,19 @@ export function InstallationForm({ initial, tabId }: { initial: InstallationProf
         </button>
       </div>
 
+      <HistoryFields
+        profile={profile}
+        idPrefix={fid("history")}
+        savedSecret={!isNew && !!initial.history && initial.history.auth.mode !== "none"}
+        secret={historySecret}
+        insecureForbidden={insecureForbidden}
+        busy={busy}
+        onHistory={(history) => update({ history })}
+        onSecret={(patch) => setHistorySecret((s) => ({ ...s, ...patch }))}
+        onSelector={(serverId, historySelector) => updateServer(serverId, { historySelector })}
+        check={() => api.checkHistorySource(current(), secrets())}
+      />
+
       <div className="toolbar">
         <button type="button" className="primary" disabled={busy} onClick={onSave}>
           {t("common.save")}
````

<!-- block: locales t10-locales-t10 step=10.3 -->
````
{
  "en": {
    "installations": {
      "form": { "step": { "up": "Query up", "targets": "Targets", "match": "Matching" } },
      "history": {
        "title": "Metrics history (Prometheus / VictoriaMetrics)",
        "none": "Without a history source only the “Now” window is available.",
        "add": "Add a history source",
        "remove": "Remove the source",
        "url": "Address",
        "auth": "Sign-in",
        "authMode": { "none": "None", "bearer": "Token", "basic": "User name and password" },
        "username": "User name",
        "token": "Token",
        "password": "Password",
        "secretClear": "Delete the saved token or password",
        "check": "Check the source",
        "report": "Check of the history source",
        "series": "Series in the history",
        "auto": "auto-match",
        "noMatch": "Do not match",
        "custom": "Own selector…",
        "selector": "Series selector of “{{server}}”"
      }
    },
    "hints": { "installations": {
      "historyUrl": "Address of the Prometheus or VictoriaMetrics HTTP API that scrapes the servers: http://prometheus.monitoring:9090. Glafira only reads it with PromQL queries and keeps nothing on disk."
    } }
  },
  "ru": {
    "installations": {
      "form": { "step": { "up": "Запрос up", "targets": "Цели", "match": "Сопоставление" } },
      "history": {
        "title": "История метрик (Prometheus / VictoriaMetrics)",
        "none": "Без источника истории доступно только окно «Сейчас».",
        "add": "Добавить источник истории",
        "remove": "Убрать источник",
        "url": "Адрес",
        "auth": "Вход",
        "authMode": { "none": "Без входа", "bearer": "Токен", "basic": "Имя пользователя и пароль" },
        "username": "Имя пользователя",
        "token": "Токен",
        "password": "Пароль",
        "secretClear": "Удалить сохранённый токен или пароль",
        "check": "Проверить источник",
        "report": "Проверка источника истории",
        "series": "Серии в истории",
        "auto": "автоподбор",
        "noMatch": "Не сопоставлять",
        "custom": "Свой селектор…",
        "selector": "Селектор серий «{{server}}»"
      }
    },
    "hints": { "installations": {
      "historyUrl": "Адрес HTTP API Prometheus или VictoriaMetrics, который опрашивает серверы: http://prometheus.monitoring:9090. Glafira только читает его запросами PromQL и ничего не хранит на диске."
    } }
  }
}
````

- [ ] **Шаг 10.4: Проверки.**

Run: `pnpm test`
Expected: `Test Files  35 passed`, `Tests  246 passed`
Run: `pnpm exec tsc --noEmit`
Expected: без вывода
Run: `pnpm i18n:check`
Expected: `✓ словари en, ru согласованы, вызовов из Rust: 139`

- [ ] **Шаг 10.5: Коммит.**

<!-- block: shell t10-commit step=10.5 -->
````
git add src/app.css \
  src/components/installations/HistoryFields.tsx \
  src/components/installations/InstallationForm.test.tsx \
  src/components/installations/InstallationForm.tsx \
  src/locales/en.json \
  src/locales/ru.json
git commit -s -q -F - <<'EOF'
feat(history): history source section of the installation form

The form gets a "Metrics history" section: address, sign-in (none, token,
user name and password) with the secret kept in the keychain, CA and proxy,
a source check with its steps, and per-server "Series in the history":
found targets, own selector or no matching. Auto-match from the check sets
selectors only of servers without one.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
````

---

### Задача 11: Строка «История» на карточке и плитки «по истории»

С источником истории на карточке сервера — выбранная цель (`job · instance`) или «не сопоставлена» (спека I1c
§5). До второго живого опроса плитки скоростей и перцентилей за минуту берут значение из последней точки истории и
помечены «по истории».

**Files:**
- Modify: `src/app.css`
- Modify: `src/components/installations/HistoryFields.tsx`
- Modify: `src/components/installations/InstallationPanel.test.tsx`
- Modify: `src/components/installations/InstallationPanel.tsx`
- Modify: `src/components/servers/ServerOverview.tsx`
- Modify: `src/components/servers/ServerPanel.test.tsx`
- Modify: `src/components/servers/ServerPanel.tsx`
- Modify: `src/locales/en.json`
- Modify: `src/locales/ru.json`

**Interfaces:**
- Consumes: `useServerMetrics(…).fromHistory`.
- Produces: `selectorLabel(selector)` (`HistoryFields.tsx`); `ServerCard` с `history`; `ServerOverview` с
  `fromHistory`; ключи `installations.panel.history`, `historyNone`, `servers.tiles.fromHistory`.

- [ ] **Шаг 11.1: Тесты.** Тесты строки «История» на карточке и пометки на плитках.

<!-- block: patch t11-src-components-installations-installationpanel-test-tsx step=11.1 -->
````
diff --git a/src/components/installations/InstallationPanel.test.tsx b/src/components/installations/InstallationPanel.test.tsx
index 25a5f7b..e6db605 100644
--- a/src/components/installations/InstallationPanel.test.tsx
+++ b/src/components/installations/InstallationPanel.test.tsx
@@ -157,6 +157,27 @@ describe("InstallationPanel", () => {
     expect(useWorkspace.getState().activeId).toBe("server:i1:s1");
   });
 
+  it("строка «История»: цель сервера или «не сопоставлена»; без источника строки нет", async () => {
+    vi.mocked(api.listInstallations).mockResolvedValue([
+      {
+        ...PROFILE,
+        history: { url: "http://prometheus:9090", auth: { mode: "none" }, ca: { mode: "system" }, proxy: { mode: "system" } },
+        servers: [{ ...PROFILE.servers[0], historySelector: '{job="vllm", instance="gpu-1:8000"}' }, PROFILE.servers[1]],
+      },
+    ]);
+    renderWithClient(<InstallationPanel installationId="i1" />);
+    const first = await screen.findByRole("region", { name: "gpu-1" });
+    expect(within(first).getByText("История")).toBeInTheDocument();
+    expect(within(first).getByText("vllm · gpu-1:8000")).toBeInTheDocument();
+    expect(within(screen.getByRole("region", { name: "gpu-2" })).getByText("не сопоставлена")).toBeInTheDocument();
+  });
+
+  it("без источника истории строки «История» нет", async () => {
+    renderWithClient(<InstallationPanel installationId="i1" />);
+    const first = await screen.findByRole("region", { name: "gpu-1" });
+    expect(within(first).queryByText("История")).toBeNull();
+  });
+
   it("строка «сейчас» на карточке подключённого сервера: выполняются · ждут · KV", async () => {
     renderWithClient(<InstallationPanel installationId="i1" />);
     const first = await screen.findByRole("region", { name: "gpu-1" });
````

<!-- block: patch t11-src-components-servers-serverpanel-test-tsx step=11.1 -->
````
diff --git a/src/components/servers/ServerPanel.test.tsx b/src/components/servers/ServerPanel.test.tsx
index ffaa3b5..97693e7 100644
--- a/src/components/servers/ServerPanel.test.tsx
+++ b/src/components/servers/ServerPanel.test.tsx
@@ -148,6 +148,26 @@ describe("ServerPanel", () => {
     await waitFor(() => expect(vi.mocked(api.readServerHistory).mock.calls.filter((c) => c[2] === "24h")).toHaveLength(2));
   });
 
+  it("«Обзор» до второго опроса: скорости и перцентили — из истории, с пометкой", async () => {
+    vi.mocked(api.listInstallations).mockResolvedValue([withHistory()]);
+    const now = Date.now();
+    vi.mocked(api.readServerHistory).mockResolvedValue({
+      window: "now",
+      stepSec: 15,
+      source: "prometheus",
+      points: [{ at: now - 15_000, m: metrics(now - 15_000) }],
+      partial: [],
+    });
+    vi.mocked(api.readServerMetrics).mockResolvedValue(
+      metrics(now, { intervalSec: null, rates: null, prefixCacheHitRatio: null, latency1m: { ttft: null, itl: null, e2e: null, queueTime: null }, preemptions1m: null }),
+    );
+    renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
+    const ttft = await screen.findByRole("region", { name: "TTFT p95 за минуту" });
+    await waitFor(() => expect(within(ttft).getByText("250 мс")).toBeInTheDocument());
+    expect(within(ttft).getByText("по истории")).toBeInTheDocument();
+    expect(within(screen.getByRole("region", { name: "KV-кэш" })).queryByText("по истории")).toBeNull();
+  });
+
   it("ошибка окна истории — вместо графиков", async () => {
     vi.mocked(api.listInstallations).mockResolvedValue([withHistory()]);
     vi.mocked(api.readServerHistory).mockImplementation(async (_i, _s, window) => {
````

- [ ] **Шаг 11.2: Увидеть падение.**

Run: `pnpm exec vitest run src/components/installations/InstallationPanel.test.tsx src/components/servers/ServerPanel.test.tsx`
Expected: FAIL — строки «История» и пометки «по истории» нет

- [ ] **Шаг 11.3: Реализация.** Строка на карточке, пометка на плитках, стиль, словари.

<!-- block: patch t11-src-app-css step=11.3 -->
````
diff --git a/src/app.css b/src/app.css
index 06a3d48..70dac85 100644
--- a/src/app.css
+++ b/src/app.css
@@ -28,6 +28,7 @@ input::placeholder, textarea::placeholder { font-style: italic; color: var(--mut
 .tile { border: 1px solid var(--border); border-radius: 6px; padding: 8px; display: flex; flex-direction: column; gap: 4px; min-width: 0; }
 .tile-label { color: var(--muted); font-size: 12px; }
 .tile-value { font-size: 18px; font-weight: 600; font-variant-numeric: tabular-nums; }
+.tile-note { font-size: 11px; }
 .charts { display: grid; grid-template-columns: repeat(auto-fill, minmax(380px, 1fr)); gap: 12px; margin-top: 8px; }
 .metric-chart { min-width: 0; }
 .charts.loading { opacity: 0.5; }
````

<!-- block: patch t11-src-components-installations-historyfields-tsx step=11.3 -->
````
diff --git a/src/components/installations/HistoryFields.tsx b/src/components/installations/HistoryFields.tsx
index d1a7be2..340f8c0 100644
--- a/src/components/installations/HistoryFields.tsx
+++ b/src/components/installations/HistoryFields.tsx
@@ -34,6 +34,12 @@ export function withAuthMode(auth: HistoryAuth, mode: HistoryAuth["mode"], lastU
 /** Подпись цели: `job · instance (vLLM)`. */
 const targetLabel = (t: HistoryTarget) => `${t.job} · ${t.instance} (${t.kind === "sglang" ? "SGLang" : "vLLM"})`;
 
+/** Селектор для показа: `{job="vllm", instance="h:8000"}` — `vllm · h:8000`, остальные — как есть. */
+export function selectorLabel(selector: string): string {
+  const m = /^\{\s*job="([^"\\]*)",\s*instance="([^"\\]*)"\s*\}$/.exec(selector.trim());
+  return m ? `${m[1]} · ${m[2]}` : selector;
+}
+
 /** Селектор цели — как его строит Rust (`Target::selector`). */
 export const targetSelector = (t: HistoryTarget) =>
   `{job="${t.job.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}", instance="${t.instance.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"}`;
````

<!-- block: patch t11-src-components-installations-installationpanel-tsx step=11.3 -->
````
diff --git a/src/components/installations/InstallationPanel.tsx b/src/components/installations/InstallationPanel.tsx
index ddde160..6dc15ca 100644
--- a/src/components/installations/InstallationPanel.tsx
+++ b/src/components/installations/InstallationPanel.tsx
@@ -9,6 +9,7 @@ import { ErrorBanner } from "../common/ErrorBanner";
 import { Hint } from "../common/Hint";
 import { fmtNumber, fmtPercent } from "../servers/format";
 import { noMetricsHintKey, readsMetrics, useServerMetrics } from "../servers/useServerMetrics";
+import { selectorLabel } from "./HistoryFields";
 
 /** Ключ запроса сведений о подключённой инсталляции. */
 export const installationQueryKey = (id: string) => ["installation", id] as const;
@@ -64,7 +65,7 @@ export function InstallationPanel({ installationId }: { installationId: string }
       {q.data && (
         <div className="cards">
           {profile.servers.map((s) => (
-            <ServerCard key={s.id} installationId={installationId} server={s} state={states.get(s.id)} />
+            <ServerCard key={s.id} installationId={installationId} server={s} state={states.get(s.id)} history={!!profile.history} />
           ))}
         </div>
       )}
@@ -72,7 +73,18 @@ export function InstallationPanel({ installationId }: { installationId: string }
   );
 }
 
-function ServerCard({ installationId, server, state }: { installationId: string; server: ServerConfig; state?: ServerState }) {
+function ServerCard({
+  installationId,
+  server,
+  state,
+  history,
+}: {
+  installationId: string;
+  server: ServerConfig;
+  state?: ServerState;
+  /** У инсталляции есть источник истории: на карточке — строка «История». */
+  history: boolean;
+}) {
   const { t } = useTranslation();
   const open = useWorkspace((s) => s.open);
   const facts = state?.facts;
@@ -89,6 +101,12 @@ function ServerCard({ installationId, server, state }: { installationId: string;
         <dd>{server.baseUrl}</dd>
         {facts && <ServerFactsRows facts={facts} />}
         {facts && <LiveRow installationId={installationId} serverId={server.id} facts={facts} />}
+        {history && (
+          <>
+            <dt>{t("installations.panel.history")}</dt>
+            <dd>{server.historySelector ? <code>{selectorLabel(server.historySelector)}</code> : <span className="muted">{t("installations.panel.historyNone")}</span>}</dd>
+          </>
+        )}
       </dl>
       {state?.error && <ErrorBanner error={state.error} />}
       {!state && <p className="muted">{t("installations.panel.notConnected")}</p>}
````

<!-- block: patch t11-src-components-servers-serveroverview-tsx step=11.3 -->
````
diff --git a/src/components/servers/ServerOverview.tsx b/src/components/servers/ServerOverview.tsx
index e3ec849..a7dbcda 100644
--- a/src/components/servers/ServerOverview.tsx
+++ b/src/components/servers/ServerOverview.tsx
@@ -35,18 +35,24 @@ export function tiles(m: ServerMetrics | undefined, t: (k: string, o?: Record<st
   ];
 }
 
+/** Плитки, которые до второго живого опроса берут значение из последней точки истории. */
+const FROM_HISTORY = new Set(["promptTokens", "generationTokens", "ttft", "itl", "prefixCache", "preemptions"]);
+
 /** «Обзор» сервера: тип и версия, модели, число движков, плитки метрик. `restarted` — сервер перезапущен меньше минуты назад. */
 export function ServerOverview({
   facts,
   latest,
   restarted = false,
   error = false,
+  fromHistory = false,
 }: {
   facts: ServerFacts;
   latest?: ServerMetrics;
   restarted?: boolean;
   /** Опрос метрик не прошёл: плитки «—» и подсказка «нет ответа», а не «…» и не «серии нет». */
   error?: boolean;
+  /** Скорости и перцентили за минуту — из последней точки истории (спека I1c §5). */
+  fromHistory?: boolean;
 }) {
   const { t } = useTranslation();
   const kind = t(`installations.form.kinds.${facts.kind}`);
@@ -89,6 +95,7 @@ export function ServerOverview({
             <span className={tile.value == null ? "tile-value muted" : "tile-value"}>
               {tile.value ?? (error ? "—" : latest || !readable ? t("servers.metrics.noData") : "…")}
             </span>
+            {fromHistory && tile.value != null && FROM_HISTORY.has(tile.id) && <span className="tile-note muted">{t("servers.tiles.fromHistory")}</span>}
           </section>
         ))}
       </div>
````

<!-- block: patch t11-src-components-servers-serverpanel-tsx step=11.3 -->
````
diff --git a/src/components/servers/ServerPanel.tsx b/src/components/servers/ServerPanel.tsx
index 9a3175f..c58bf78 100644
--- a/src/components/servers/ServerPanel.tsx
+++ b/src/components/servers/ServerPanel.tsx
@@ -89,7 +89,9 @@ export function ServerPanel({ installationId, serverId }: { installationId: stri
               </button>
             ))}
           </div>
-          {sub === "overview" && <ServerOverview facts={state.facts} latest={live.error ? undefined : live.latest} restarted={restarted} error={!!live.error} />}
+          {sub === "overview" && (
+            <ServerOverview facts={state.facts} latest={live.error ? undefined : live.latest} restarted={restarted} error={!!live.error} fromHistory={live.fromHistory} />
+          )}
           {sub === "config" && <ServerConfigView installationId={installationId} serverId={serverId} />}
           {sub === "metrics" && (
             <ServerMetricsView
````

<!-- block: locales t11-locales-t11 step=11.3 -->
````
{
  "en": {
    "installations": { "panel": { "history": "History", "historyNone": "not matched" } },
    "servers": { "tiles": { "fromHistory": "from history" } }
  },
  "ru": {
    "installations": { "panel": { "history": "История", "historyNone": "не сопоставлена" } },
    "servers": { "tiles": { "fromHistory": "по истории" } }
  }
}
````

- [ ] **Шаг 11.4: Проверки.**

Run: `pnpm test`
Expected: `Test Files  35 passed`, `Tests  249 passed`
Run: `pnpm exec tsc --noEmit`
Expected: без вывода
Run: `pnpm i18n:check`
Expected: `✓ словари en, ru согласованы, вызовов из Rust: 139`

- [ ] **Шаг 11.5: Коммит.**

<!-- block: shell t11-commit step=11.5 -->
````
git add src/app.css \
  src/components/installations/HistoryFields.tsx \
  src/components/installations/InstallationPanel.test.tsx \
  src/components/installations/InstallationPanel.tsx \
  src/components/servers/ServerOverview.tsx \
  src/components/servers/ServerPanel.test.tsx \
  src/components/servers/ServerPanel.tsx \
  src/locales/en.json \
  src/locales/ru.json
git commit -s -q -F - <<'EOF'
feat(history): history row on server cards and tiles from history

With a history source, every server card shows its matched target
(`job · instance`) or "not matched". Until the second live poll, the
Overview tiles of rates and minute percentiles come from the last history
point and carry a "from history" note.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
````

---

### Задача 12: Глава «История метрик», тексты этапа, смоук

Глава руководства `history` (en, ru; спека I1c §5): источник, сопоставление, окна, «Сейчас», что делать, если не
сопоставилось, пример конфигурации Prometheus; глава «Серверы» ссылается на неё. README, «Начало работы» и
`about.summary` называют этап I1c. Смоук `docs/smoke/i1c.md` — с настоящим Prometheus. Спека I1c приводится к коду:
секрет одной записью keychain, `CaConfig::Insecure` вместо отдельного флага, одна команда проверки, ошибки сети —
общие `errors.server.*`, точки окна включительно, мок с `--api-key`, всплеск `rate` после сброса мока.

**Files:**
- Create: `docs/guide/en/history.md`
- Create: `docs/guide/ru/history.md`
- Create: `docs/smoke/i1c.md`
- Modify: `README.md`
- Modify: `docs/guide/en/getting-started.md`
- Modify: `docs/guide/en/servers.md`
- Modify: `docs/guide/ru/getting-started.md`
- Modify: `docs/guide/ru/servers.md`
- Modify: `docs/superpowers/specs/2026-10-31-glafira-inference-i1c-design.md`
- Modify: `scripts/i18n-check.mjs`
- Modify: `src/guide/chapters.test.ts`
- Modify: `src/guide/chapters.ts`
- Modify: `src/locales/en.json`
- Modify: `src/locales/ru.json`

**Interfaces:**
- Produces: `GUIDE_CHAPTERS` с `history` (`src/guide/chapters.ts` и `scripts/i18n-check.mjs`).

- [ ] **Шаг 12.1: Тесты.** Тест загрузки глав: у ru есть глава «История метрик».

<!-- block: patch t12-src-guide-chapters-test-ts step=12.1 -->
````
diff --git a/src/guide/chapters.test.ts b/src/guide/chapters.test.ts
index 5520253..cf4fae5 100644
--- a/src/guide/chapters.test.ts
+++ b/src/guide/chapters.test.ts
@@ -6,6 +6,7 @@ describe("loadGuide", () => {
     const ru = await loadGuide("ru");
     expect(Object.keys(ru).sort()).toEqual([...GUIDE_CHAPTERS].sort());
     expect(ru.installations).toMatch(/^# Инсталляции/);
+    expect(ru.history).toMatch(/^# История метрик/);
   });
 
   it("falls back to English for a language without chapters", async () => {
````

- [ ] **Шаг 12.2: Увидеть падение.**

Run: `pnpm exec vitest run src/guide`
Expected: FAIL — `ru.history` нет: глава не в списке

- [ ] **Шаг 12.3: Реализация.** Глава на двух языках, список глав, ссылка из «Серверов», тексты этапа (`about.summary` заменяется — `replace=1`), смоук и спека.

<!-- block: patch t12-readme-md step=12.3 -->
````
diff --git a/README.md b/README.md
index 00861cb..c8df5a9 100644
--- a/README.md
+++ b/README.md
@@ -3,10 +3,11 @@
 Desktop-приложение (Tauri 2 + Rust + React) для подключения к корпоративному инференсу open-source LLM, его
 мониторинга, диагностики и аудита; в фокусе — vLLM и SGLang на GPU NVIDIA.
 
-Состояние: готовы этапы I1a и I1b — подключение, проверка и метрики сервера. Это инсталляции из серверов, определение
-типа, версии и моделей сервера, пошаговая проверка сервера, папки, экспорт и импорт, а также вкладка сервера: плитки и
-графики метрик «сейчас» (опрос `/metrics` раз в 5 с) и аргументы запуска с источниками. История метрик, GPU,
-диагностика и аудит появятся в следующих этапах, I1c–I7 (спецификация, §20).
+Состояние: готовы этапы I1a, I1b и I1c — подключение, проверка, метрики сервера и их история. Это инсталляции из
+серверов, определение типа, версии и моделей сервера, пошаговая проверка сервера, папки, экспорт и импорт, вкладка
+сервера: плитки и графики метрик «сейчас» (опрос `/metrics` раз в 5 с) и аргументы запуска с источниками, — и история
+метрик из Prometheus или VictoriaMetrics: окна 1 ч / 24 ч / 7 д / 30 д и «Сейчас» с последних 15 минут. GPU,
+диагностика и аудит появятся в следующих этапах, I1d–I7 (спецификация, §20).
 
 vLLM, SGLang, LiteLLM, NVIDIA and other product names are trademarks of their respective owners. This project is not
 affiliated with them.
````

<!-- block: patch t12-docs-guide-en-getting-started-md step=12.3 -->
````
diff --git a/docs/guide/en/getting-started.md b/docs/guide/en/getting-started.md
index a149b79..5259b1f 100644
--- a/docs/guide/en/getting-started.md
+++ b/docs/guide/en/getting-started.md
@@ -1,9 +1,10 @@
 # Getting started
 
 Glafira for LLM Inference is a desktop app for connecting to corporate open-source LLM inference, monitoring,
-diagnosing and auditing it, with vLLM and SGLang in focus. This version (stage I1b) covers connecting and checking:
-installations of servers, the kind, version and models of each server, a step-by-step server check, and live server
-metrics: tiles, charts and launch arguments. Metric history, GPUs, diagnostics and audit come in the next stages.
+diagnosing and auditing it, with vLLM and SGLang in focus. This version (stage I1c) covers connecting and checking:
+installations of servers, the kind, version and models of each server, a step-by-step server check, live server
+metrics (tiles, charts and launch arguments) and metric history from Prometheus or VictoriaMetrics. GPUs,
+diagnostics and audit come in the next stages.
 Glafira changes nothing on the servers: it reads their answers with your API keys. Glafira never reads or shows the texts of your users' requests and answers.
 
 ## Installation {#install}
````

<!-- block: file t12-docs-guide-en-history-md step=12.3 path=docs/guide/en/history.md -->
````
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
list of the found targets:

- **auto-match** — the target whose `instance` equals the host and port of the server address (`localhost` and
  `127.0.0.1` are one address); auto-match never overwrites a choice made by a person;
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
````

<!-- block: patch t12-docs-guide-en-servers-md step=12.3 -->
````
diff --git a/docs/guide/en/servers.md b/docs/guide/en/servers.md
index 175e94c..213dc84 100644
--- a/docs/guide/en/servers.md
+++ b/docs/guide/en/servers.md
@@ -46,8 +46,8 @@ arguments, from the pod or the process, come with the Kubernetes and SSH connect
 
 Charts for the "Now" window, up to an hour kept in the tab's memory: requests, KV cache, preemptions, tokens per
 second, prefix cache hits, TTFT, ITL and end-to-end latency (p50, p95, p99), queue time and finished requests by
-reason. A failed poll is a gap in the chart, not a zero. The 1 h, 24 h, 7 d and 30 d windows come with a history
-source (Prometheus or VictoriaMetrics).
+reason. A failed poll is a gap in the chart, not a zero. The 1 h, 24 h, 7 d and 30 d windows are read from the
+history source (Prometheus or VictoriaMetrics) when the installation has one — see [Metric history](history.md#windows).
 
 ## Server versions {#versions}
 
````

<!-- block: patch t12-docs-guide-ru-getting-started-md step=12.3 -->
````
diff --git a/docs/guide/ru/getting-started.md b/docs/guide/ru/getting-started.md
index 022f217..3c2ab2b 100644
--- a/docs/guide/ru/getting-started.md
+++ b/docs/guide/ru/getting-started.md
@@ -1,9 +1,10 @@
 # Начало работы
 
 Glafira for LLM Inference — настольное приложение для подключения к корпоративному инференсу open-source LLM, его
-мониторинга, диагностики и аудита; в фокусе — vLLM и SGLang. В этой версии (этап I1b) готовы подключение и проверка:
-инсталляции из серверов, тип, версия и модели каждого сервера, пошаговая проверка сервера — и метрики сервера «сейчас»:
-плитки, графики и аргументы запуска. История метрик, GPU, диагностика и аудит появятся в следующих этапах. Glafira
+мониторинга, диагностики и аудита; в фокусе — vLLM и SGLang. В этой версии (этап I1c) готовы подключение и проверка:
+инсталляции из серверов, тип, версия и модели каждого сервера, пошаговая проверка сервера — метрики сервера «сейчас»:
+плитки, графики и аргументы запуска — и история метрик из Prometheus или VictoriaMetrics. GPU, диагностика и аудит
+появятся в следующих этапах. Glafira
 ничего не меняет в серверах: она читает их ответы с вашими ключами API. Тексты запросов и ответов ваших пользователей Glafira не читает и не показывает.
 
 ## Установка {#install}
````

<!-- block: file t12-docs-guide-ru-history-md step=12.3 path=docs/guide/ru/history.md -->
````
# История метрик

История приходит из Prometheus или VictoriaMetrics, который уже опрашивает серверы инференса. Glafira сама историю не
хранит: она только читает её запросами PromQL, пока открыта вкладка сервера, и ничего не пишет на диск.

## Источник {#source}

1. Откройте инсталляцию, нажмите [[ui:installations.panel.edit]], затем [[ui:installations.history.add]].
2. Укажите адрес HTTP API, например `http://prometheus.monitoring:9090` (путь допускается:
   `https://gw.example.com/prometheus`).
3. Выберите вход: без входа, токен или имя пользователя и пароль. Токен и пароль хранятся только в системном хранилище
   секретов, в файл инсталляции и в экспорт они не попадают.
4. CA и прокси — как у серверов. Без проверки сертификата источник можно открыть, только если это разрешает политика.
5. Нажмите [[ui:installations.history.check]]: проверка пройдёт DNS/TCP, вход, запрос `up`, поиск целей vLLM и
   SGLang и сопоставление каждого сервера.

Если источник недоступен, инсталляция всё равно подключается: недоступны только окна истории.

## Сопоставление серверов {#matching}

Серверу нужен селектор серий — набор меток, по которым его метрики видны в источнике, например
`{job="vllm", instance="10.0.0.5:8000"}`. После проверки у каждого сервера в разделе [[ui:installations.history.series]]
есть список найденных целей:

- **автоподбор** — цель, у которой `instance` совпадает с хостом и портом адреса сервера (`localhost` и `127.0.0.1` —
  один адрес); выбор человека автоподбор не перезаписывает;
- цель из списка — если адрес сервера и `instance` разные: в Kubernetes источник опрашивает IP пода, а в инсталляции
  записан адрес сервиса;
- [[ui:installations.history.custom]] — любые метки в двойных кавычках, без `__name__`: имена серий Glafira
  подставляет сама;
- [[ui:installations.history.noMatch]] — у сервера нет истории.

На карточке сервера строка [[ui:installations.panel.history]] показывает выбранную цель.

## Окна {#windows}

На вкладке сервера в разделе метрик окна 1 ч, 24 ч, 7 д и 30 д читаются из источника:

| Окно | Шаг |
|---|---|
| 1 ч | 1 мин |
| 24 ч | 90 с |
| 7 д | 12 мин |
| 30 д | 45 мин |

Скорости считаются функцией `rate`, перцентили — `histogram_quantile` по тем же гистограммам, что и в окне «Сейчас»;
перезапуски сервера `rate` учитывает сам. Окно читается при выборе, свежее — по кнопке [[ui:common.refresh]];
повторный выбор окна в пределах минуты источник не нагружает. Если часть рядов не прочиталась, остальные графики всё
равно видны, а над ними написано, какие ряды пропали и почему.

## Окно «Сейчас» {#now}

Если у сервера есть история, окно «Сейчас» при открытии вкладки сразу заполняется последними 15 минутами из неё, а
дальше его пополняет опрос `/metrics`. Пунктир на графиках отмечает, где кончается история. Пока не пришёл второй опрос,
скорости и перцентили на плитках обзора берутся из истории и помечены «по истории».

## Если не сопоставилось {#troubleshooting}

- «Источник не собирает метрики vLLM и SGLang» — в конфигурации Prometheus нет целей серверов (пример ниже) или у
  SGLang не включён `--enable-metrics`.
- «Цели с адресом сервера нет» — выберите цель из списка или напишите селектор: посмотрите метки `job` и `instance`
  в веб-интерфейсе Prometheus на странице Targets.
- «Под селектором нет серий» — селектор не совпадает с метками или сервер ещё не опрашивался.
- Окно пустое в начале — источник не хранил метрики за это время (например, его запустили позже).

## Пример конфигурации Prometheus {#prometheus-config}

```yaml
scrape_configs:
  - job_name: vllm
    static_configs:
      - targets: ["10.0.0.5:8000"]
  - job_name: sglang
    static_configs:
      - targets: ["10.0.0.7:30000"]
```

`/metrics` vLLM открыт без ключа API; SGLang отдаёт метрики, только если запущен с `--enable-metrics`.
````

<!-- block: patch t12-docs-guide-ru-servers-md step=12.3 -->
````
diff --git a/docs/guide/ru/servers.md b/docs/guide/ru/servers.md
index dd6d772..150d5e7 100644
--- a/docs/guide/ru/servers.md
+++ b/docs/guide/ru/servers.md
@@ -47,8 +47,8 @@ Glafira не подставляет ноль вместо неизвестног
 
 Графики за окно «Сейчас» — до часа в памяти вкладки: запросы, KV-кэш, вытеснения, токены в секунду, попадания в
 prefix-cache, TTFT, ITL и полное время запроса (p50, p95, p99), время в очереди и завершения по причинам. Ошибка
-опроса — разрыв на графике, а не ноль. Окна 1 ч, 24 ч, 7 д и 30 д появятся с источником истории (Prometheus или
-VictoriaMetrics).
+опроса — разрыв на графике, а не ноль. Окна 1 ч, 24 ч, 7 д и 30 д читаются из источника истории (Prometheus или
+VictoriaMetrics), если он есть у инсталляции, — подробнее в главе [История метрик](history.md#windows).
 
 ## Версии серверов {#versions}
 
````

<!-- block: file t12-docs-smoke-i1c-md step=12.3 path=docs/smoke/i1c.md -->
````
# Смоук I1c (история метрик)

Сборка: `pnpm tauri dev`. Источник — настоящий Prometheus (`brew install prometheus`) с конфигом
`scripts/prometheus/smoke.yml`; он опрашивает моки раз в 5 с. Каждый мок — в своём терминале:

```
pnpm mock-server --kind vllm --port 8000
pnpm mock-server --kind vllm --engines 2 --port 8001
pnpm mock-server --kind vllm --reset-after 20 --port 8003
pnpm mock-server --kind sglang --port 30000
prometheus --config.file=scripts/prometheus/smoke.yml --storage.tsdb.path=/tmp/glafira-prom --web.listen-address=127.0.0.1:9090
```

Перед смоуком дайте Prometheus собрать данные 15–20 минут. Отказы источника — мок `--kind prometheus` (README).

1. Источник. Инсталляция `dev-llm` с серверами `vllm-1` `http://localhost:8000`, `vllm-dp2` `http://127.0.0.1:8001`,
   `vllm-reset` `http://127.0.0.1:8003`, `sglang-1` `http://127.0.0.1:30000` (ключ `sk-sglang-s3cr3t`, если мок с ключом).
   «Изменить» → «Добавить источник истории», адрес `http://127.0.0.1:9090`, «Проверить источник»: «Вход» — «Prometheus
   3.15.1», «Запрос up» — «Целей в работе: 4 из 4», «Цели» — 4, у всех четырёх серверов «Сопоставление» с
   «(автоподбор)»; у `vllm-1` (`localhost`) подобрана цель `127.0.0.1:8000`.
2. Выбор цели. У `vllm-dp2` выбрать в «Серии в истории» другую цель, затем вернуть `vllm · 127.0.0.1:8001`. «Свой
   селектор…» `{job="vllm"` (без скобки) — «Сохранить» показывает ошибку с номером символа; `{job="nothing"}` —
   сохраняется, «Проверить источник» — ⚠ «под селектором … нет серий». Вернуть цель, «Сохранить». На карточках во
   вкладке инсталляции — строка «История» с `vllm · 127.0.0.1:8000` и т. д.
3. Окна. Вкладка `vllm-1` → «Метрики»: «Сейчас» сразу заполнено (до 15 минут точек), на графиках пунктир, над
   графиками «История до HH:MM». «1 ч» — графики за время жизни Prometheus, «Шаг 1 мин · Prometheus»; «24 ч» — шаг
   90 с; «7 д» и «30 д» — пусто или одна-две точки (Prometheus живёт меньше шага), без ошибки. Точка «1 ч» сверяется с
   веб-интерфейсом Prometheus (Graph) по тому же запросу, например
   `sum(rate(vllm:prompt_tokens_total{job="vllm",instance="127.0.0.1:8000"}[1m]))` — около 20,8.
4. Несколько движков. `vllm-dp2`, «1 ч»: токены на входе около 41,6/с — вдвое больше, чем у `vllm-1` (сумма движков).
5. «Обзор» сразу после открытия вкладки сервера с историей: TTFT, ITL, токены, prefix-cache и вытеснения заполнены с
   пометкой «по истории»; через 5–10 с пометка пропадает.
6. Перезапуск. `vllm-reset`, «1 ч»: скорости без отрицательных значений и разрывов. Мок после сброса отдаёт счётчики
   снимка, а не нули, поэтому `rate` показывает всплеск раз в 100 с — это особенность мока: у настоящего сервера
   счётчики после перезапуска начинаются с нуля.
7. Prometheus остановлен (Ctrl+C): «Сейчас» работает (опрос `/metrics`), «1 ч» — ошибка источника вместо графиков,
   инсталляция подключена, «Проверить источник» — ❌ на шаге «DNS/TCP». Запустить Prometheus снова — «Обновить»
   читает окно.
8. Отказы — `pnpm mock-server --kind prometheus --port 9091 --api-key prom-t0ken`, источник `http://127.0.0.1:9091`:
   без входа — ❌ «Вход» (401); «Токен» `prom-t0ken` — проверка проходит, цели из снимка. В `installations.json`, логах и
   выводе `pnpm tauri dev` строки `prom-t0ken` нет. `--slow 40` (таймаут запроса инсталляции 30 с) — ошибка таймаута;
   `--huge` — «больше 8 МБ» в окне «1 ч».
9. Без источника и без сопоставления. Убрать источник: окна неактивны с подсказкой «Добавьте источник истории в
   инсталляцию», строки «История» на карточках нет. Источник есть, у сервера «Не сопоставлять» — подсказка «Серии
   сервера не сопоставлены с историей».
10. Экспорт и импорт. Экспорт инсталляции с токеном источника: в файле нет токена, `needsSecret.history: true`.
    Импорт файла с другим адресом источника поверх инсталляции — предупреждение о смене адреса источника.
11. По желанию: VictoriaMetrics single-node с тем же конфигом опроса (`-promscrape.config=scripts/prometheus/smoke.yml`):
    «Вход» — «VictoriaMetrics …», окна строятся.
12. Справка и язык. F1 на вкладке сервера — глава «Серверы», ссылка в разделе метрик ведёт в главу «История метрик»;
    поиск находит «селектор». English — раздел формы, окна, подписи и глава на английском.
````

<!-- block: patch t12-docs-superpowers-specs-2026-10-31-glafira-inference-i1c-design-md step=12.3 -->
````
diff --git a/docs/superpowers/specs/2026-10-31-glafira-inference-i1c-design.md b/docs/superpowers/specs/2026-10-31-glafira-inference-i1c-design.md
index 5cccde2..23860c4 100644
--- a/docs/superpowers/specs/2026-10-31-glafira-inference-i1c-design.md
+++ b/docs/superpowers/specs/2026-10-31-glafira-inference-i1c-design.md
@@ -26,19 +26,22 @@
 - `InstallationProfile.history: Option<HistorySource>` — не больше одного источника на инсталляцию:
 
   ```
-  HistorySource { url, auth: none | bearer | basic { username }, ca: CaConfig, proxy: ProxyConfig,
-                  insecure_skip_tls_verify: bool }
+  HistorySource { url, auth: none | bearer | basic { username }, ca: CaConfig, proxy: ProxyConfig }
   ```
 
-  `insecure_skip_tls_verify` — как у серверов: постоянное предупреждение, запрещается политикой
-  `allowInsecureTls = false`. Тип источника (Prometheus или VictoriaMetrics) определяет проверка и только показывает.
+  Без проверки сертификата — режим `CaConfig::Insecure`, как у серверов: постоянное предупреждение, запрещается
+  политикой `allowInsecureTls = false` (и при сохранении, и при импорте). Тип источника (Prometheus или
+  VictoriaMetrics) определяет `buildinfo` и только показывает.
 - `ServerConfig.history_selector: Option<String>` — селектор серий сервера, `{job="vllm", instance="10.0.0.5:8000"}`.
   Заполняет автоподбор или человек; `None` — история сервера недоступна.
-- Токен (bearer) и пароль (basic) — только в keychain: `secret_key(installationId, "history", "token" | "password")`.
-  Удаление источника удаляет секрет.
+- Токен (bearer) или пароль (basic) — только в keychain, одной записью `history_secret_key(installationId)` =
+  `<инсталляция>/history/secret` (поля `secret` у серверов не бывает). Сохранённый секрет остаётся, только пока не
+  меняется вид входа: токен не становится паролем. Убранный источник, вход «без входа» и удаление инсталляции удаляют
+  секрет.
 - Экспорт и импорт — как в семействе: секрета в файле нет, у инсталляции флаг `needsSecret.history`; при обновлении
   импорт предупреждает, если у источника с сохранённым секретом меняются адрес, прокси или проверка сертификата
-  (секрет уйдёт на новый адрес). Селекторы серверов экспортируются как есть.
+  (секрет уйдёт на новый адрес). Если источника в файле нет или у него другой вид входа, сохранённый секрет
+  удаляется (об убранном источнике импорт предупреждает). Селекторы серверов экспортируются как есть.
 
 ## 3. Проверка источника и сопоставление
 
@@ -51,8 +54,9 @@
 4. Цели инференса — `count by (job, instance) ({__name__=~"vllm:.+|sglang:.+"})`: ✅ «Найдены цели инференса: N»,
    0 — ⚠️ «Источник не собирает метрики vLLM и SGLang». Тип цели — по второму запросу
    `count by (job, instance) ({__name__=~"sglang:.+"})`: цели из него — SGLang, остальные — vLLM.
-5. Сопоставление — шаг на каждый сервер: ✅ «`vllm · 10.0.0.5:8000` (автоподбор)» или «(выбрано)»; ⚠️ «Цель не
-   найдена — выберите вручную»; ⚠️ «Под селектором нет серий»; ❌ «Селектор не разобран: …».
+5. Сопоставление — шаг на каждый сервер: ✅ «`vllm · 10.0.0.5:8000` (автоподбор)» у сервера без селектора или
+   «селектор» у сервера с ним (под селектором есть серии — `count({…, __name__=~"vllm:.+|sglang:.+"})`); ⚠️ «цели с
+   адресом сервера нет — выберите цель вручную»; ⚠️ «под селектором нет серий»; ❌ ошибка разбора селектора.
 
 Сбой источника не мешает подключению серверов: инсталляция подключена, история — ❌ в проверке и «История недоступна»
 у окон.
@@ -67,9 +71,12 @@
 экранированием `\"` и `\\`, хотя бы один матчер, `__name__` запрещён (имя серии подставляет шаблон). Иначе —
 `errors.history.selector` с позицией.
 
-**Ошибки источника** — ключи `errors.history.*`: `unreachable`, `unauthorized`, `forbidden`, `timeout`, `badData`
-(текст источника до 512 символов), `tooManyPoints`, `tooLarge` (ответ больше 8 МБ), `unexpected` (не JSON или
-неизвестный `resultType`).
+**Ошибки источника.** Сеть (DNS, TCP, TLS, таймаут соединения) — те же ключи `errors.server.*`, что у серверов. Ответ
+источника — ключи `errors.history.*`: `unauthorized` (401), `forbidden` (403), `timeout` (`errorType: timeout`),
+`badData` (текст источника до 512 символов), `tooManyPoints`, `failed` (другой `errorType`), `status` (код не 2xx без
+JSON ошибки), `tooLarge` (ответ больше 8 МБ), `unexpected` (не JSON или неизвестный `resultType`); разбор —
+`badUrl`, `badUsername`, `selector`, `selectorName`; состояние — `noSource`, `notMatched`, `noMetrics` (тип без карты
+метрик).
 
 ## 4. Rust: модуль `history/`
 
@@ -96,25 +103,31 @@
 
   | Окно | Длина | Шаг | Точек |
   |---|---|---|---|
-  | `now` (предзаполнение) | 15 мин | 15 с | 60 |
-  | `1h` | 1 ч | 60 с | 60 |
-  | `24h` | 24 ч | 90 с | 960 |
-  | `7d` | 7 д | 12 мин | 840 |
-  | `30d` | 30 д | 45 мин | 960 |
-
-  Интервал `r = max(шаг, 4 × scrapeInterval)`, без `scrapeInterval` — `max(шаг, 1 мин)`. Конец окна — `now`,
-  округлённый вниз до шага (повторные запросы окна попадают в кеш источника).
+  | `now` (предзаполнение) | 15 мин | 15 с | 61 |
+  | `1h` | 1 ч | 60 с | 61 |
+  | `24h` | 24 ч | 90 с | 961 |
+  | `7d` | 7 д | 12 мин | 841 |
+  | `30d` | 30 д | 45 мин | 961 |
+
+  Точки — от начала до конца окна включительно. Интервал `r = max(шаг, 1 мин, 4 × scrapeInterval)`: «за минуту» у
+  окна `now` — настоящая минута. `scrapeInterval` берётся из `/api/v1/targets` по `job` и `instance` селектора (если
+  оба — условия `=`). Конец окна — `now`, округлённый вниз до шага (повторные запросы окна попадают в кеш источника).
 - `history/targets.rs` — поиск целей, автоподбор (раздел 3), разбор и сборка селектора.
 - `history/series.rs` — сборка точек: матрицы запросов выравниваются по меткам времени окна, точка — `ServerMetrics`
   той же формы, что в окне «Сейчас» (`gauges`, `rates`, `prefixCacheHitRatio`, `latency`, `latency1m = latency`,
-  `missing` — метрики без единого значения в окне, `series` — кандидат, давший ряд, если его можно определить;
-  `reset` всегда `false`, `engines` — 0: число движков знает только живой опрос, «Обзор» без него строку «Движки»
-  не показывает).
+  `preemptions1m` — скорость вытеснений × 60, `missing` — метрики без единого значения в окне; `series` и `totals`
+  пусты, `count` у перцентилей — 0: число наблюдений история не запрашивает; `reset` всегда `false`, `engines` — 0:
+  число движков знает только живой опрос, «Обзор» без него строку «Движки» не показывает). Причина завершения без
+  метки — `unknown`, как в I1b.
+- `history/check.rs` — проверка источника (раздел 3), `history/read.rs` — окно сервера: запросы, не больше 4
+  одновременно, сборка точек. Открытый источник (клиент и тип по `buildinfo`) живёт в пуле подключений
+  (`ServerPool::history`) до `evict` инсталляции.
 
 **Команды:**
 
-- `check_history_source(installationId | draft)` — шаги раздела 3 для формы и проверки инсталляции;
-- `discover_history_targets(draft)` — цели и автоподбор для формы;
+- `check_history_source(input)` (профиль формы и секреты, секрет источника — поверх keychain) →
+  `{ report, source?, targets, suggestions: [{ serverId, selector?, targets }] }`: шаги раздела 3, найденные цели и
+  автоподбор для серверов без селектора — одной командой;
 - `read_server_history(installationId, serverId, window)` →
   `{ window, stepSec, source: "prometheus" | "victoriametrics", points: [{ at, m }], partial: [{ chart, error }] }`.
 
@@ -142,9 +155,9 @@
 
 **Форма инсталляции.** Раздел «История метрик (Prometheus / VictoriaMetrics)», по умолчанию свёрнут: адрес,
 авторизация (без неё / токен / логин и пароль), CA, прокси, проверка сертификата; кнопка «Проверить источник»
-(шаги раздела 3). После проверки у каждого сервера — поле «Серии в истории»: список найденных целей
-`job · instance (vLLM | SGLang)`, «Свой селектор…» (поле ввода с разбором на лету), «Не сопоставлять»; отметка
-«автоподбор» у подобранной цели.
+(шаги раздела 3). Под ней — «Серии в истории»: у каждого сервера список найденных целей
+`job · instance (vLLM | SGLang)`, «Свой селектор…» (поле ввода; разбирает его Rust при сохранении и проверке —
+второго разбора в TS нет), «Не сопоставлять»; отметка «автоподбор» у подобранной цели.
 
 **Карточка сервера** во вкладке инсталляции — строка «История»: `job · instance` или «не сопоставлена»; без
 источника строки нет.
@@ -164,15 +177,16 @@
   матчеры, экранирование значений; шаги и интервалы окон; автоподбор (`localhost`, порты по умолчанию, несколько
   `job`); разбор селектора (правильные и неправильные, позиция ошибки); сборка точек из снятых матриц (выравнивание,
   `NaN` и `+Inf`, ряд без значений → `missing`, частичный сбой → `partial`); ошибки клиента на моке.
-- **Мок источника:** вид `prometheus` в `mock-server.mjs` отдаёт снятые снимки и умеет отказывать: `--token`
-  (401 без него), `--slow` (таймаут), `bad_data` на селектор с `__bad__`, `--huge` (ответ больше 8 МБ). Тесты скриптов
-  — на каждый отказ.
+- **Мок источника:** вид `prometheus` в `mock-server.mjs` отдаёт снятые снимки (`query_range` шаблонов — на сетке
+  запроса) и умеет отказывать: `--api-key` (токен; без него 401), `--slow N` (ответ через N секунд), `bad_data` на
+  выражение с `__bad__`, `--huge` (ответ больше 8 МБ). Тесты скриптов — на каждый отказ.
 - **Фронтенд:** доступность окон и подсказки, загрузка и `partial`, предзаполнение буфера и граница «История до…»,
   поле «Серии в истории» (автоподбор, выбор, свой селектор, ошибка разбора), строка «История» на карточке.
 - **Смоук `docs/smoke/i1c.md`:** подключение и проверка источника; автоподбор, ручной выбор цели для `--engines 2`,
   свой селектор и его ошибка; окна 1 ч…30 д (короткие заполнены, длинные — за время жизни Prometheus), сверка точки
-  с веб-интерфейсом Prometheus по тому же запросу; «Сейчас» заполнено при открытии; перезапуск мока без всплеска в
-  `rate`; Prometheus остановлен — «Сейчас» работает, окна истории — ошибка, инсталляция подключена; токен через мок
+  с веб-интерфейсом Prometheus по тому же запросу; «Сейчас» заполнено при открытии; перезапуск мока без отрицательных
+  скоростей (всплеск `rate` после сброса — особенность мока: он начинает со счётчиков снимка, а не с нуля); Prometheus
+  остановлен — «Сейчас» работает, окна истории — ошибка, инсталляция подключена; токен через мок
   `prometheus` — секрета нет на диске и в логах; справка и английский.
 
 ## 7. Риски этапа
@@ -183,7 +197,7 @@
 | `or` по кандидатам смешивает два имени в одной точке (обе серии есть в 0.10.x–0.11.x) | `or` берёт левую часть, где она есть — тот же порядок кандидатов, что в I1b |
 | Тяжёлые запросы за 30 дней на большом Prometheus | Не больше 4 одновременно, шаг 45 мин, таймаут инсталляции, кнопка «Обновить» вместо автоперезапроса |
 | VictoriaMetrics отличается от Prometheus (MetricsQL, `targets`) | Только совместимые функции; без `targets` — интервал по умолчанию; смоук VM по желанию |
-| Мок растит счётчики за опрос, а не за секунду | Скорости в смоуке сверяются с веб-интерфейсом Prometheus, а не с ожиданием «настоящих» чисел |
+| Мок растит счётчики за опрос, а не за секунду, и после «перезапуска» начинает со счётчиков снимка | Скорости в смоуке сверяются с веб-интерфейсом Prometheus, а не с ожиданием «настоящих» чисел; всплеск `rate` после сброса мока описан в смоуке |
 
 ## 8. Риски, перенесённые из I1b
 
````

<!-- block: patch t12-scripts-i18n-check-mjs step=12.3 -->
````
diff --git a/scripts/i18n-check.mjs b/scripts/i18n-check.mjs
index 1ed2582..6f255c6 100644
--- a/scripts/i18n-check.mjs
+++ b/scripts/i18n-check.mjs
@@ -562,7 +562,7 @@ export function findRustCyrillic(files) {
 }
 
 /** Главы руководства: тот же список, что `GUIDE_CHAPTERS` в `src/guide/chapters.ts`. */
-export const GUIDE_CHAPTERS = ["getting-started", "installations", "servers", "updates"];
+export const GUIDE_CHAPTERS = ["getting-started", "installations", "servers", "history", "updates"];
 
 const KEBAB = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
 
````

<!-- block: patch t12-src-guide-chapters-ts step=12.3 -->
````
diff --git a/src/guide/chapters.ts b/src/guide/chapters.ts
index 7e39608..d275656 100644
--- a/src/guide/chapters.ts
+++ b/src/guide/chapters.ts
@@ -1,5 +1,5 @@
 /** Главы руководства (спец. §16); каждый этап добавляет свои. Тот же список — в `scripts/i18n-check.mjs`. */
-export const GUIDE_CHAPTERS = ["getting-started", "installations", "servers", "updates"] as const;
+export const GUIDE_CHAPTERS = ["getting-started", "installations", "servers", "history", "updates"] as const;
 
 export type ChapterId = (typeof GUIDE_CHAPTERS)[number];
 
````

<!-- block: locales t12-locales-t12-replace step=12.3 replace=1 -->
````
{
  "en": { "about": {
    "summary": "Glafira is a desktop app for connecting to corporate LLM inference, monitoring, diagnosing and auditing it, with vLLM and SGLang in focus. This version (stage I1c) covers connecting and checking: installations of several servers, API keys in the system secret store, step-by-step server checks, folders, export and import, live server metrics with launch arguments, and metric history from Prometheus or VictoriaMetrics. GPUs, diagnostics and audit come in the next stages."
  } },
  "ru": { "about": {
    "summary": "Glafira — настольное приложение для подключения к корпоративному инференсу LLM, его мониторинга, диагностики и аудита; в фокусе — vLLM и SGLang. В этой версии (этап I1c) готовы подключение и проверка: инсталляции из нескольких серверов, ключи API в системном хранилище секретов, пошаговая проверка серверов, папки, экспорт и импорт, метрики сервера «сейчас» и аргументы запуска, а также история метрик из Prometheus или VictoriaMetrics. GPU, диагностика и аудит появятся в следующих этапах."
  } }
}
````

- [ ] **Шаг 12.4: Проверки.**

Run: `pnpm test`
Expected: `Test Files  35 passed`, `Tests  249 passed`
Run: `pnpm exec tsc --noEmit`
Expected: без вывода
Run: `pnpm i18n:check`
Expected: `✓ словари en, ru согласованы, вызовов из Rust: 139`
Run: `pnpm test:scripts`
Expected: `# pass 110`
Run: `pnpm build`
Expected: `✓ dist/assets: чанки совместимы с WebKit macOS 11+`

- [ ] **Шаг 12.5: Коммит.**

<!-- block: shell t12-commit step=12.5 -->
````
git add README.md \
  docs/guide/en/getting-started.md \
  docs/guide/en/history.md \
  docs/guide/en/servers.md \
  docs/guide/ru/getting-started.md \
  docs/guide/ru/history.md \
  docs/guide/ru/servers.md \
  docs/smoke/i1c.md \
  docs/superpowers/specs/2026-10-31-glafira-inference-i1c-design.md \
  scripts/i18n-check.mjs \
  src/guide/chapters.test.ts \
  src/guide/chapters.ts \
  src/locales/en.json \
  src/locales/ru.json
git commit -s -q -F - <<'EOF'
docs(history): Metric history chapter, stage texts, I1c smoke

The guide gets a "Metric history" chapter in English and Russian (source,
matching, windows, the "now" window, troubleshooting and a Prometheus
configuration example) linked from the Servers chapter. README, Getting
started and About name stage I1c. docs/smoke/i1c.md runs against a real
Prometheus scraping the mocks. The I1c spec follows the prototype.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
````

---

## Самопроверка плана

- Покрытие спеки I1c: §1 — границы этапа соблюдены (GPU — I1d, MCP и правила — I2); §2 — задача 1; §3 — задачи 2 (автоподбор, селектор) и 6 (шаги проверки); §4 — задачи 3–6; §5 — задачи 8–11 (окна, «Сейчас» из истории, форма, карточка, плитки), глава — задача 12; §6 — снятые ответы (задача 4), мок (задача 7), тесты всех задач, смоук (задача 12); §7–8 — меры в задачах 2, 3, 6, 7 и в смоуке; §9 — общая спека поправлена коммитом спеки I1c до плана, спека I1c — задачей 12.
- Плейсхолдеров нет: код каждого шага — блоки, извлекаемые скриптом; ожидаемые числа — по прогонам прототипа.
- Имена между задачами: `HistorySource`, `history_selector`, `history_secret_key` (задача 1) → `parse_selector`, `render`, `Target` (2) → `queries`, `QueryKey`, `Window`, `range` (3) → `HistoryClient`, `RangeSeries` (4) → `assemble`, `HistoryPoint`, `Partial` (5) → `check_source`, `read_history`, `ServerPool::history` (6) → TS-типы `HistoryCheck`, `HistoryAnswer`, `useServerHistory`, `useServerMetrics(…, prefill)` (8) → `ServerMetricsView`, `drawMarker` (9) → `HistoryFields` (10) → `selectorLabel`, `ServerOverview.fromHistory` (11).
