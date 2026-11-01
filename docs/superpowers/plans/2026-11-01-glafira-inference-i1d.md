# I1d «GPU» — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Карты NVIDIA инсталляции по метрикам DCGM: компонент GPU в профиле (экспортёры с паролем в keychain или серии источника истории под селектором карт), привязка серверов к картам с автоподбором, проверка компонента, снимок карт с приростами счётчиков, окна истории карт, вкладка GPU, блоки GPU в обзорах инсталляции и сервера, мок экспортёра DCGM, глава справки «GPU» и смоук.

**Architecture:** Подход A спеки I1d, как в I1b и I1c: опрашивает фронтенд, пока вкладка видна, разбор, приросты и PromQL — в Rust. Чистые функции `gpu/` — таблица полей DCGM с запасными сериями (`fields.rs`), карты из серий (`snapshot.rs`), автоподбор привязок и селекторов (`bind.rs`), шаблоны окон (`promql.rs`) и сборка рядов (`series.rs`); источник карт (`source.rs`: экспортёры через механику `ServerClient` или мгновенный запрос к источнику истории), приросты счётчиков (`live.rs`), проверка (`check.rs`) и окно истории (`read.rs`); команды `check_gpu_source`, `read_gpu_status`, `read_gpu_history`, открытый компонент — в пуле подключений до `evict`. Фронтенд — `useGpuStatus` (буфер «Сейчас» по картам), `useGpuHistory`, вкладка `GpuPanel` с карточками и графиками (`MetricChart` обобщается по типу точки), раздел формы `GpuFields`, блоки обзоров `GpuSummary`.

**Tech Stack:** Rust 2024 (tauri 2, reqwest, tokio, futures, serde, serde_json, url — новых крейтов нет), TypeScript (React 19, TanStack Query 5, i18next, uPlot 1.6.32 — новых зависимостей нет), Vitest + Testing Library, pnpm 11, Node 22 (мок-сервер без зависимостей), Prometheus 3.15.1 (`brew install prometheus`) — только для смоука и снятых ответов.

**Спецификация:** `docs/superpowers/specs/2026-11-01-glafira-inference-i1d-design.md` (спека этапа, разделы 1–8) поверх спек I1b, I1c и общей `docs/superpowers/specs/2026-10-29-glafira-inference-design.md` (§5.3, §5.7, §5.8, §6.2–6.4, §6.6, §7.2, §17, §18).

Решения, которые прототип уточнил (задача 12 вносит их в спеку I1d):
- метка хоста `hostname` — с dcgm-exporter 4.8, `Hostname` — в 4.1 и раньше (в спеке было «3.x»); вид мока — `dcgm-4.1`, а не `dcgm-3x`; мок умеет ещё `--pod ns/имя` (метки Kubernetes);
- признак экспортёра и счёт карт — `DCGM_FI_DEV_FB_USED`: в режиме MIG `DCGM_FI_DEV_GPU_UTIL` у карты нет;
- `GpuBinding` — только `host` и `indices`: подобранную проверкой привязку помнит форма, как селекторы истории в I1c, без поля `auto` в профиле;
- совпадение хостов: короткое имя узла совпадает с полным (`gpu-node-1` и `gpu-node-1.corp.local`), адреса IP — только целиком;
- шаблоны окон — в `gpu/promql.rs`, группировка по `hostname, Hostname, instance, gpu`, без экземпляров MIG; ошибки NVLink — подзапросом: `increase` по каждому из семи счётчиков убирает имя серии, и Prometheus отвечает 422 из-за одинаковых наборов меток;
- окно хоста сначала узнаёт, какой меткой подписан хост, запросом `count by (hostname, Hostname, instance)` под селектором карт или селекторами экспортёров;
- подсказка «полнота набора» — шаг проверки на каждую группу счётчиков; новые ошибки — `errors.gpu.noExporters`, `badBinding`, `unauthorized`, `notMatched`, `hostNotInHistory`, `tooManySeries`.

**Проверено до написания плана.** Весь код плана собран и прогнан в отдельной ветке репозитория (по коммиту на задачу, база `43a56f9` — `main` @ `1ce37f1` со спекой I1d); повторное применение всех блоков плана в чистом дереве `43a56f9` даёт то же дерево после каждой задачи побайтно, проверки после каждой задачи — зелёные:
- Rust — 345 unit-тестов (`cargo test --lib`; было 293), clippy с `-D warnings`, `cargo fmt --check`; новых крейтов нет;
- фронтенд — 289 тестов в 40 файлах (было 255 в 35), `tsc --noEmit`, `pnpm build` с проверкой совместимости бандла с WebKit macOS 11, `pnpm i18n:check` (en и ru, 187 вызовов из Rust, 6 глав руководства на двух языках);
- скрипты — 115 тестов (было 110), в том числе виды мока DCGM;
- настоящий Prometheus 3.15.1, который опрашивал моки `dcgm-full --gpus 8 --hostname gpu-node-1` и `dcgm --gpus 2`: поиск карт, мгновенный запрос и все 14 шаблонов окна отработали (после замены суммы ошибок NVLink подзапросом); его ответы — `src-tauri/src/gpu/testdata/prometheus-3.15.1`, на них проходят тесты источника, проверки, сборки рядов и чтения окна;
- сквозная проверка Rust против живых моков и Prometheus: проверка в режиме экспортёров — карты обоих, автоподбор привязки и целей истории ✅; в режиме «из истории» — автоподбор селектора ✅; снимок 10 карт; окна `now`, `1h`, `24h` читаются за ~8 мс.
Не проверено: окно приложения (`pnpm tauri dev`) и смоук `docs/smoke/i1d.md`; настоящий dcgm-exporter и карты NVIDIA (формат мока собран по исходникам dcgm-exporter 4.8.4, а не снят с карты).

## Global Constraints

- Glafira только читает экспортёры и источник истории; на диск метрики не пишутся, опрос — только пока вкладка видна (общая спека §6.6, спека I1d §1).
- Поле без серии — «нет данных», а не ноль; у полей — запасные серии (спека I1d §4).
- Не больше 16 экспортёров, не больше 64 карт на инсталляцию, не больше 16 рядов в окне хоста (§2–4).
- Не больше 4 одновременных запросов к экспортёрам и к источнику истории; сбой одного экспортёра — запись в `errors`, всех — ошибка (§4).
- Пароль экспортёров — только в keychain, одной записью; в профиле, экспорте, логах и ошибках его нет (§2).
- Сбой компонента GPU не мешает подключению серверов (§3).
- Привязка сервера к картам до I3 — `ServerConfig.gpus`; режим `ssh`, процессы на карте, правила `gpu.*` и MCP — не в этом этапе (§1).
- Все тексты интерфейса — в словарях en и ru; Rust — через `ltext!`/`app_err!` с ключом; глава руководства — на двух языках.
- Сообщения коммитов — по-английски, `git commit -s`, строка `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` перед `Signed-off-by`.

## Структура файлов

| Файл | Ответственность |
|---|---|
| `scripts/mock-server.mjs`, `scripts/prometheus/smoke.yml` | виды мока DCGM, цели DCGM в Prometheus смоука |
| `src-tauri/src/storage/profile.rs`, `storage/exchange.rs`, `commands/installations.rs`, `secrets.rs` | компонент GPU, привязка сервера, пароль экспортёров |
| `src-tauri/src/gpu/fields.rs`, `snapshot.rs` | поля DCGM, карты из серий |
| `src-tauri/src/gpu/bind.rs` | совпадение хостов, автоподбор привязок и селекторов |
| `src-tauri/src/gpu/source.rs`, `live.rs` | экспортёры и мгновенный запрос, приросты счётчиков |
| `src-tauri/src/gpu/promql.rs`, `series.rs`, `read.rs` | шаблоны, ряды и чтение окна истории |
| `src-tauri/src/gpu/check.rs`, `commands/gpu.rs` | проверка компонента, команды |
| `src-tauri/src/gpu/testdata/` | снимки `/metrics` мока и ответы настоящего Prometheus |
| `src/components/gpu/` | хуки, вкладка GPU, карточки, графики, значки, блоки обзоров |
| `src/components/installations/GpuFields.tsx` | раздел формы «GPU (DCGM)» и поле GPU сервера |
| `docs/guide/{en,ru}/gpu.md`, `docs/smoke/i1d.md` | глава и смоук |

## Блоки кода

Код переносится как есть. Перед каждым блоком стоит HTML-комментарий `<!-- block: <вид> <id> step=<задача>.<шаг> [path=…] [part=…] [mode=…] [eol=none] [replace=1] -->`: `shell` — команды bash из корня репозитория; `file` — новый файл по `path` (у Rust-файлов с тестами две части: `part=tests` — модуль тестов, `part=impl` — реализация, итоговый файл = реализация, пустая строка, модуль тестов); `patch` — патч `git diff` (применять `git apply --recount`); `locales` — фрагмент словарей для `scripts/merge-locales.mjs` (`replace=1` — с `--replace`: значение меняется). Ограждения — четыре и больше обратных кавычек: внутри встречаются тройные. Блоки не перепечатывайте: их применяет скрипт ниже, по шагам, в порядке появления в плане. Патчи рассчитаны на дерево `43a56f9` с применёнными предыдущими задачами плана.

Снимки `/metrics` экспортёра DCGM (`src-tauri/src/gpu/testdata/dcgm-4.8.4/*.txt`, задачи 3 и 5) не перепечатываются: блок `shell` шага 3 пересоздаёт их мок-сервером задачи 1 побайтно. Ответы Prometheus (`src-tauri/src/gpu/testdata/prometheus-3.15.1/*.json`) — однострочный JSON в блоках `file`.

В шаге 1 каждой задачи, кроме тестов, применяется подключение модулей (`mod.rs`, `lib.rs`): без него новые тесты Rust не компилировались бы вовсе, и падения в шаге 2 не было бы. У задачи 2 тесты — в изменённых модулях, поэтому шагов 1–2 у неё нет.

Скрипт извлекается из этого плана один раз, до задачи 1 (план лежит в `docs/superpowers/plans/` репозитория):

```bash
awk '/^<!-- block: helper i1d-blocks.py -->$/{f=1;next} f&&/^````/{n++; if(n==2) exit; next} f' \
  docs/superpowers/plans/2026-11-01-glafira-inference-i1d.md > /tmp/i1d-blocks.py
python3 /tmp/i1d-blocks.py list | head
```

<!-- block: helper i1d-blocks.py -->
````
#!/usr/bin/env python3
"""Блоки плана I1d: извлечение и применение по шагам (как у I1a; у locales — флаг replace=1).

Использование (из корня репозитория):
  python3 /tmp/i1d-blocks.py list                 — все блоки: id, вид, шаг, путь
  python3 /tmp/i1d-blocks.py get <id>             — содержимое блока в stdout
  python3 /tmp/i1d-blocks.py step <задача>.<шаг>  — применить блоки шага по порядку
План — $PLAN или docs/superpowers/plans/2026-11-01-glafira-inference-i1d.md.

Виды блоков: shell — команды bash из корня репозитория; file — файл по path (part=tests пишет модуль тестов,
part=impl ставит реализацию перед ним через пустую строку; mode — права; eol=none — файл без перевода строки в конце); patch — `git apply --recount`;
locales — строки словарей для `scripts/merge-locales.mjs` (replace=1 — заменить значения, `--replace`).
"""
import os
import re
import subprocess
import sys

PLAN = os.environ.get("PLAN", "docs/superpowers/plans/2026-11-01-glafira-inference-i1d.md")
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

Шаг задачи применяется так: `python3 /tmp/i1d-blocks.py step 3.1`, затем команды проверки из плана.

---

### Задача 1: Мок экспортёра DCGM и цели DCGM в Prometheus смоука

Виды мок-сервера `dcgm`, `dcgm-full`, `dcgm-4.1`, `dcgm-mig` отвечают `/metrics` в формате dcgm-exporter 4.8.4,
собранном по его исходникам (спека I1d §1, §6): метки карты, набор счётчиков по умолчанию и дополнительные счётчики,
метка `Hostname` версии 4.1, экземпляры MIG, метки Kubernetes (`--pod`). Значения меняются от опроса к опросу,
счётчики растут; `--throttle`, `--xid`, `--ecc` включают проблемы у карты 0 (троттлинг и ошибки NVLink и PCIe — у всех
карт). `--api-key` у видов DCGM — пароль basic с логином `glafira`. `scripts/prometheus/smoke.yml` опрашивает моки
DCGM на 9400–9402 (job `dcgm`); README описывает виды.

**Files:**
- Modify: `README.md`
- Modify: `scripts/mock-server.mjs`
- Modify: `scripts/mock-server.test.mjs`
- Modify: `scripts/prometheus/smoke.yml`

**Interfaces:**
- Produces: `dcgmMetrics(kind, opts, poll)` (экспорт `scripts/mock-server.mjs`; `kind` — `default`, `full`, `4.1`,
  `mig`); `parseArgs(argv).dcgm = { gpus, hostname, model, pod, throttle, xid, ecc }` у видов DCGM (у прочих видов
  поля нет, параметры DCGM — ошибка); `KINDS[<вид>].dcgm`.

- [ ] **Шаг 1.1: Тесты.** Тесты мока: параметры DCGM, набор по умолчанию, дополнительные виды, проблемы, вход basic.

<!-- block: patch t01-scripts-mock-server-test-mjs step=1.1 -->
````
diff --git a/scripts/mock-server.test.mjs b/scripts/mock-server.test.mjs
index 319e18f..acb6ace 100644
--- a/scripts/mock-server.test.mjs
+++ b/scripts/mock-server.test.mjs
@@ -2,7 +2,7 @@ import assert from "node:assert/strict";
 import { readFileSync } from "node:fs";
 import { createServer } from "node:http";
 import { test } from "node:test";
-import { handler, KINDS, liveMetrics, parseArgs, prometheusReply } from "./mock-server.mjs";
+import { dcgmMetrics, handler, KINDS, liveMetrics, parseArgs, prometheusReply } from "./mock-server.mjs";
 
 async function serve(opts) {
   const server = createServer(handler({ health: 200, apiKey: null, ...opts }));
@@ -154,7 +154,7 @@ test("ключ API у llama.cpp и Ollama — как у vLLM: только /v1",
 });
 
 test("--models empty: /v1/models отдаёт пустой список, остальное без изменений", async () => {
-  for (const kind of Object.keys(KINDS).filter((k) => !KINDS[k].prometheus)) {
+  for (const kind of Object.keys(KINDS).filter((k) => !KINDS[k].prometheus && !KINDS[k].dcgm)) {
     const s = await serve({ kind, models: "empty" });
     try {
       const r = await fetch(`${s.base}/v1/models`);
@@ -337,3 +337,96 @@ test("prometheus: --api-key закрывает всё, --slow задержива
     await s.close();
   }
 });
+
+const DCGM = { gpus: 2, hostname: "localhost", model: null, pod: null, throttle: false, xid: 0, ecc: false };
+
+/** Серии /metrics экспортёра DCGM: [{ name, labels, value }]. */
+function series(text) {
+  return text
+    .split("\n")
+    .filter((l) => l && !l.startsWith("#"))
+    .map((l) => {
+      const [, name, labels, value] = /^(\w+)\{(.*)\} (\S+)$/.exec(l);
+      return { name, labels: Object.fromEntries([...labels.matchAll(/(\w+)="([^"]*)"/g)].map((m) => [m[1], m[2]])), value: Number(value) };
+    });
+}
+
+test("параметры DCGM: значения по умолчанию, свои и ошибки", () => {
+  assert.deepEqual(parseArgs(["--kind", "dcgm"]).dcgm, DCGM);
+  assert.deepEqual(
+    parseArgs(["--kind", "dcgm-full", "--gpus", "8", "--hostname", "gpu-node-1", "--model", "NVIDIA H200", "--pod", "llm/vllm-0", "--throttle", "--xid", "79", "--ecc"]).dcgm,
+    { gpus: 8, hostname: "gpu-node-1", model: "NVIDIA H200", pod: "llm/vllm-0", throttle: true, xid: 79, ecc: true },
+  );
+  assert.equal(parseArgs(["--kind", "vllm"]).dcgm, undefined);
+  assert.throws(() => parseArgs(["--kind", "vllm", "--gpus", "2"]), /только у видов dcgm/);
+  assert.throws(() => parseArgs(["--kind", "dcgm", "--gpus", "73"]), /--gpus.*Использование/s);
+  assert.throws(() => parseArgs(["--kind", "dcgm", "--pod", "vllm-0"]), /--pod/);
+  assert.throws(() => parseArgs(["--kind", "dcgm", "--hostname"]), /--hostname/);
+  assert.throws(() => parseArgs(["--kind", "dcgm", "--xid", "0"]), /--xid/);
+});
+
+test("dcgm: набор по умолчанию dcgm-exporter 4.8.4 — метки карты, без SM_ACTIVE, ECC и причин троттлинга", () => {
+  const all = series(dcgmMetrics("default", DCGM, 0));
+  const names = new Set(all.map((s) => s.name));
+  for (const name of ["DCGM_FI_DEV_GPU_UTIL", "DCGM_FI_DEV_FB_USED", "DCGM_FI_PROF_GR_ENGINE_ACTIVE", "DCGM_FI_DEV_XID_ERRORS", "DCGM_FI_DEV_PCIE_REPLAY_COUNTER"])
+    assert.ok(names.has(name), name);
+  for (const name of ["DCGM_FI_PROF_SM_ACTIVE", "DCGM_FI_DEV_ECC_DBE_VOL_TOTAL", "DCGM_FI_DEV_CLOCKS_EVENT_REASONS", "DCGM_EXP_XID_ERRORS_COUNT"])
+    assert.ok(!names.has(name), name);
+  const util = all.filter((s) => s.name === "DCGM_FI_DEV_GPU_UTIL");
+  assert.deepEqual(util.map((s) => s.labels.gpu), ["0", "1"]);
+  assert.deepEqual(Object.keys(util[0].labels), ["gpu", "UUID", "pci_bus_id", "device", "modelName", "hostname", "DCGM_FI_DRIVER_VERSION"]);
+  assert.equal(util[0].labels.modelName, "NVIDIA H100 80GB HBM3");
+  const mem = (name) => all.find((s) => s.name === name && s.labels.gpu === "0").value;
+  assert.equal(mem("DCGM_FI_DEV_FB_USED") + mem("DCGM_FI_DEV_FB_FREE") + mem("DCGM_FI_DEV_FB_RESERVED"), 81559);
+});
+
+test("dcgm-full, dcgm-4.1, dcgm-mig и --pod: дополнительные серии, метка Hostname, экземпляры MIG, метки Kubernetes", () => {
+  const full = new Set(series(dcgmMetrics("full", DCGM, 0)).map((s) => s.name));
+  for (const name of ["DCGM_FI_PROF_SM_ACTIVE", "DCGM_FI_DEV_ECC_SBE_VOL_TOTAL", "DCGM_FI_DEV_ECC_DBE_VOL_TOTAL", "DCGM_FI_DEV_CLOCKS_EVENT_REASONS", "DCGM_FI_DEV_POWER_VIOLATION", "DCGM_FI_DEV_NVLINK_CRC_ERROR_TOTAL"])
+    assert.ok(full.has(name), name);
+  const old = series(dcgmMetrics("4.1", { ...DCGM, hostname: "gpu-old" }, 0))[0];
+  assert.equal(old.labels.Hostname, "gpu-old");
+  assert.equal(old.labels.hostname, undefined);
+  assert.equal(old.labels.modelName, "NVIDIA A100-SXM4-80GB");
+  const mig = series(dcgmMetrics("mig", DCGM, 0));
+  assert.deepEqual(mig.filter((s) => s.name === "DCGM_FI_DEV_GPU_UTIL").map((s) => s.labels.gpu), ["1"], "у карты MIG нет утилизации");
+  const used = mig.filter((s) => s.name === "DCGM_FI_DEV_FB_USED" && s.labels.gpu === "0");
+  assert.deepEqual(used.map((s) => [s.labels.GPU_I_PROFILE, s.labels.GPU_I_ID]), [["3g.40gb", "1"], ["3g.40gb", "2"]]);
+  assert.ok(mig.some((s) => s.name === "DCGM_FI_DEV_GPU_TEMP" && s.labels.gpu === "0" && !s.labels.GPU_I_ID), "температура — у карты");
+  const pod = series(dcgmMetrics("default", { ...DCGM, pod: "llm/vllm-0" }, 0))[0].labels;
+  assert.deepEqual([pod.container, pod.namespace, pod.pod], ["vllm", "llm", "vllm-0"]);
+});
+
+test("dcgm: датчики меняются, счётчики растут; --throttle, --xid и --ecc включают проблемы", () => {
+  const at = (kind, o, poll, name, gpu = "0") => series(dcgmMetrics(kind, o, poll)).find((s) => s.name === name && s.labels.gpu === gpu)?.value;
+  assert.notEqual(at("default", DCGM, 0, "DCGM_FI_DEV_GPU_UTIL"), at("default", DCGM, 2, "DCGM_FI_DEV_GPU_UTIL"));
+  assert.ok(at("default", DCGM, 3, "DCGM_FI_DEV_TOTAL_ENERGY_CONSUMPTION") > at("default", DCGM, 1, "DCGM_FI_DEV_TOTAL_ENERGY_CONSUMPTION"));
+  assert.equal(at("full", DCGM, 5, "DCGM_FI_DEV_CLOCKS_EVENT_REASONS"), 0);
+  assert.equal(at("full", DCGM, 5, "DCGM_FI_DEV_XID_ERRORS"), 0);
+  const bad = { ...DCGM, throttle: true, xid: 79, ecc: true };
+  assert.equal(at("full", bad, 5, "DCGM_FI_DEV_CLOCKS_EVENT_REASONS"), 0x48, "HW_SLOWDOWN | HW_THERMAL");
+  assert.ok(at("full", bad, 5, "DCGM_FI_DEV_PCIE_REPLAY_COUNTER") > at("full", bad, 2, "DCGM_FI_DEV_PCIE_REPLAY_COUNTER"));
+  assert.ok(at("full", bad, 5, "DCGM_FI_DEV_NVLINK_CRC_ERROR_TOTAL") > 0);
+  assert.equal(at("full", bad, 5, "DCGM_FI_DEV_XID_ERRORS"), 79);
+  assert.equal(at("full", bad, 5, "DCGM_FI_DEV_XID_ERRORS", "1"), 0, "проблемы — только у карты 0");
+  assert.equal(at("full", bad, 5, "DCGM_FI_DEV_ECC_DBE_VOL_TOTAL"), 2);
+  assert.equal(at("full", bad, 5, "DCGM_EXP_XID_ERRORS_COUNT"), 1);
+  assert.equal(at("default", bad, 5, "DCGM_EXP_XID_ERRORS_COUNT"), undefined, "в наборе по умолчанию счётчика XID нет");
+});
+
+test("dcgm: /metrics по опросам; --api-key — пароль basic с логином glafira", async () => {
+  const s = await serve({ kind: "dcgm", dcgm: { ...DCGM, gpus: 3 }, apiKey: "dcgm-pa55" });
+  try {
+    assert.equal((await fetch(`${s.base}/metrics`)).status, 401);
+    assert.equal((await fetch(`${s.base}/metrics`, { headers: { authorization: "Bearer dcgm-pa55" } })).status, 401);
+    const auth = { authorization: `Basic ${Buffer.from("glafira:dcgm-pa55").toString("base64")}` };
+    const r = await fetch(`${s.base}/metrics`, { headers: auth });
+    assert.equal(r.status, 200);
+    assert.match(r.headers.get("content-type"), /text\/plain/);
+    const util = series(await r.text()).filter((x) => x.name === "DCGM_FI_DEV_GPU_UTIL");
+    assert.equal(util.length, 3);
+    assert.equal((await fetch(`${s.base}/nope`, { headers: auth })).status, 404);
+  } finally {
+    await s.close();
+  }
+});
````

- [ ] **Шаг 1.2: Увидеть падение.**

Run: `node --test scripts/mock-server.test.mjs`
Expected: FAIL — `dcgmMetrics` не экспортируется (SyntaxError импорта)

- [ ] **Шаг 1.3: Реализация.** Виды DCGM, генератор `/metrics`, вход basic, конфиг Prometheus, README.

<!-- block: patch t01-readme-md step=1.3 -->
````
diff --git a/README.md b/README.md
index c8df5a9..a3d30d7 100644
--- a/README.md
+++ b/README.md
@@ -54,12 +54,13 @@ cargo test --manifest-path src-tauri/Cargo.toml --lib     # Rust: unit-тест
 
 ## Мок-сервер
 `pnpm mock-server --kind <вид> [--port 8000] [--host 127.0.0.1] [--api-key sk-dev] [--health 200..599] [--models full|empty]
-[--engines 1..8] [--reset-after N] [--slow N] [--huge]` отвечает как сервер инференса снимками ответов из `src-tauri/src/servers/testdata` —
+[--engines 1..8] [--reset-after N] [--slow N] [--huge] [--gpus 1..72] [--hostname <имя>] [--model <модель>] [--pod <namespace>/<pod>]
+[--throttle] [--xid <код>] [--ecc]` отвечает как сервер инференса снимками ответов из `src-tauri/src/servers/testdata` —
 теми же, что проверяют тесты Rust. Виды: `vllm` (0.32, настоящий снимок), `vllm-0.11`, `vllm-0.10` (старые и новые имена
 метрик рядом, два движка), `vllm-0.8` (модель с адаптером LoRA; ниже границы поддержки), `vllm-old` (0.7), `sglang`
 (0.5.22), `sglang-no-metrics` (0.5.22 без `/metrics`, как без `--enable-metrics`), `sglang-0.5.4`, `sglang-0.4` (устаревшие пути `/get_*`, без метрик; ниже границы), `llamacpp`, `ollama`
 (без `/health`), `prometheus` (источник истории метрик, ответами настоящего Prometheus 3.15.1 из
-`src-tauri/src/history/testdata`).
+`src-tauri/src/history/testdata`), `dcgm`, `dcgm-full`, `dcgm-4.1`, `dcgm-mig` (экспортёры DCGM).
 - `/metrics` видов `vllm`, `vllm-0.10` и `sglang` «живой»: счётчики и гистограммы растут с каждым опросом, KV-кэш
   колеблется. `--engines N` — N движков data parallel, `--reset-after N` — после N опросов счётчики начинаются заново,
   как при перезапуске сервера.
@@ -72,6 +73,14 @@ cargo test --manifest-path src-tauri/Cargo.toml --lib     # Rust: unit-тест
   через N секунд (таймаут), `--huge` отдаёт `query_range` больше 8 МБ, выражение с `__bad__` — ошибка `bad_data`.
   `query_range` шаблонов Glafira отдаёт снятые ряды на сетке запроса. Настоящую историю даёт Prometheus с конфигом
   `scripts/prometheus/smoke.yml`, который опрашивает моки раз в 5 с (`brew install prometheus`).
+- Виды `dcgm*` отвечают как dcgm-exporter: `/metrics` собирается на лету в формате dcgm-exporter 4.8.4 (собран по
+  его исходникам, а не снят с карты). `dcgm` — набор счётчиков по умолчанию (без активности SM, ECC и причин
+  троттлинга), `dcgm-full` — с ними и с ошибками NVLink, `dcgm-4.1` — метка `Hostname` вместо `hostname` и A100,
+  `dcgm-mig` — карта 0 с двумя экземплярами MIG. `--gpus` — число карт (по умолчанию 2), `--hostname` — хост в метках
+  (`localhost`), `--model` — модель, `--pod ns/имя` — метки Kubernetes; `--throttle`, `--xid <код>` и `--ecc` включают
+  троттлинг (с ошибками NVLink и PCIe), XID и ECC у карты 0. Датчики колеблются, счётчики растут с каждым опросом.
+  `--api-key` — пароль basic с логином `glafira` (как `web-config.yml` экспортёра). Prometheus смоука опрашивает моки
+  DCGM на 9400–9402 (job `dcgm`).
 
 ## Инсталляции и серверы
 - Инсталляция — серверы инференса одной системы (`storage/profile.rs`): имя, окружение, только чтение, таймауты и
````

<!-- block: patch t01-scripts-mock-server-mjs step=1.3 -->
````
diff --git a/scripts/mock-server.mjs b/scripts/mock-server.mjs
index 69c33b2..bc8243e 100755
--- a/scripts/mock-server.mjs
+++ b/scripts/mock-server.mjs
@@ -1,11 +1,13 @@
 #!/usr/bin/env node
 // Мок-сервер инференса для разработки и смоука без GPU: отвечает как vLLM, SGLang или OpenAI-совместимый
 // сервер снимками ответов из src-tauri/src/servers/testdata (те же снимки проверяют тесты Rust), а вид
-// prometheus — как источник истории, ответами настоящего Prometheus из src-tauri/src/history/testdata.
+// prometheus — как источник истории, ответами настоящего Prometheus из src-tauri/src/history/testdata, виды dcgm* —
+// как экспортёр DCGM (формат dcgm-exporter 4.8.4, собран по его исходникам).
 // Только модули node:. Использование:
-//   node scripts/mock-server.mjs --kind vllm|vllm-0.11|vllm-0.10|vllm-0.8|vllm-old|sglang|sglang-no-metrics|sglang-0.5.4|sglang-0.4|llamacpp|ollama|prometheus
+//   node scripts/mock-server.mjs --kind vllm|vllm-0.11|vllm-0.10|vllm-0.8|vllm-old|sglang|sglang-no-metrics|sglang-0.5.4|sglang-0.4|llamacpp|ollama|prometheus|dcgm|dcgm-full|dcgm-4.1|dcgm-mig
 //        [--port 8000] [--host 127.0.0.1] [--api-key sk-dev]
 //        [--health 200..599] [--models full|empty] [--engines 1..8] [--reset-after N] [--slow N] [--huge]
+//        [--gpus 1..72] [--hostname <имя>] [--model <модель>] [--pod <namespace>/<pod>] [--throttle] [--xid <код>] [--ecc]
 //   --port    целое 0..65535 (0 — любой свободный порт).
 //   --health  код ответа /health, любое целое 200..599 (например 200, 301, 401, 503);
 //             у 3xx добавляется заголовок Location на https://<хост>:<порт>/health.
@@ -17,6 +19,12 @@
 //   Вид prometheus (смоук отказов источника истории): --api-key — токен (без него 401), --slow N — ответ через
 //   N секунд (таймаут), --huge — ответ query_range больше 8 МБ; выражение с __bad__ — ошибка bad_data.
 //   query_range отдаёт снятые ряды шаблонов Glafira на сетке запроса (значения по кругу), чужие — пусто.
+//   Виды dcgm (набор счётчиков по умолчанию), dcgm-full (плюс SM_ACTIVE, ECC, причины троттлинга, ошибки NVLink),
+//   dcgm-4.1 (метка Hostname, A100), dcgm-mig (карта 0 с двумя экземплярами MIG): --gpus — число карт (2),
+//   --hostname — хост в метках (localhost), --model — модель, --pod — метки Kubernetes, --throttle — троттлинг
+//   и ошибки NVLink/PCIe, --xid — код XID у карты 0, --ecc — ошибки ECC у карты 0; --api-key — пароль basic
+//   (логин glafira). Датчики колеблются, счётчики растут с каждым опросом.
+import { Buffer } from "node:buffer";
 import { readFileSync } from "node:fs";
 import { createServer } from "node:http";
 import { join } from "node:path";
@@ -27,7 +35,7 @@ const DATA = join(ROOT, "src-tauri/src/servers/testdata");
 const PROMETHEUS = join(ROOT, "src-tauri/src/history/testdata/prometheus-3.15.1");
 
 const USAGE =
-  "Использование: node scripts/mock-server.mjs --kind <вид> [--port 0..65535] [--host 127.0.0.1] [--api-key <ключ>] [--health 200..599] [--models full|empty] [--engines 1..8] [--reset-after N] [--slow N] [--huge]";
+  "Использование: node scripts/mock-server.mjs --kind <вид> [--port 0..65535] [--host 127.0.0.1] [--api-key <ключ>] [--health 200..599] [--models full|empty] [--engines 1..8] [--reset-after N] [--slow N] [--huge] [--gpus 1..72] [--hostname <имя>] [--model <модель>] [--pod <namespace>/<pod>] [--throttle] [--xid <код>] [--ecc]";
 
 /** vLLM (AuthenticationMiddleware): ключ нужен только путям с префиксом /v1, остальное (/version, /metrics, /health) открыто.
  *  Так же отвечают llama.cpp, Ollama и прочие OpenAI-совместимые виды: ключ защищает только /v1. */
@@ -112,6 +120,11 @@ export const KINDS = {
     noHealth: true,
     prometheus: true,
   },
+  // Экспортёры DCGM: /metrics собирается на лету (dcgmMetrics), вход — basic с логином glafira.
+  dcgm: { dcgm: "default", routes: {}, open: () => false, noHealth: true },
+  "dcgm-full": { dcgm: "full", routes: {}, open: () => false, noHealth: true },
+  "dcgm-4.1": { dcgm: "4.1", routes: {}, open: () => false, noHealth: true },
+  "dcgm-mig": { dcgm: "mig", routes: {}, open: () => false, noHealth: true },
 };
 
 /** Целое из значения параметра в пределах [min, max]; иначе ошибка с текстом использования. */
@@ -125,12 +138,29 @@ export function parseArgs(argv) {
     throw new Error(`${message}\n${USAGE}`);
   };
   const opts = { kind: "vllm", port: 8000, host: "127.0.0.1", apiKey: null, health: 200, models: "full", engines: 1, resetAfter: 0, slow: 0, huge: false };
+  const dcgm = { gpus: 2, hostname: "localhost", model: null, pod: null, throttle: false, xid: 0, ecc: false };
+  let dcgmSet = false;
   for (let i = 0; i < argv.length; i++) {
     const [name, value] = [argv[i], argv[i + 1]];
     if (name === "--huge") {
       opts.huge = true;
       continue;
     }
+    if (name === "--throttle" || name === "--ecc") {
+      dcgm[name.slice(2)] = true;
+      dcgmSet = true;
+      continue;
+    }
+    if (["--gpus", "--hostname", "--model", "--pod", "--xid"].includes(name)) {
+      if (name === "--gpus") dcgm.gpus = intArg(name, value, 1, 72);
+      else if (name === "--xid") dcgm.xid = intArg(name, value, 1, 1000);
+      else if (name === "--pod" && !/^[^/\s]+\/[^/\s]+$/.test(value ?? "")) fail(`--pod: нужно <namespace>/<pod>, получено ${value === undefined ? "ничего" : `«${value}»`}`);
+      else if (value === undefined || value === "") fail(`${name}: значение не указано`);
+      else dcgm[name.slice(2)] = value;
+      dcgmSet = true;
+      i++;
+      continue;
+    }
     if (name === "--kind") opts.kind = value;
     else if (name === "--port") opts.port = intArg(name, value, 0, 65535);
     else if (name === "--host") opts.host = value;
@@ -146,6 +176,8 @@ export function parseArgs(argv) {
     i++;
   }
   if (!KINDS[opts.kind]) fail(`неизвестный вид ${opts.kind}: ${Object.keys(KINDS).join(", ")}`);
+  if (KINDS[opts.kind].dcgm) opts.dcgm = dcgm;
+  else if (dcgmSet) fail(`параметры --gpus, --hostname, --model, --pod, --throttle, --xid и --ecc — только у видов dcgm`);
   return opts;
 }
 
@@ -184,6 +216,139 @@ export function liveMetrics(text, poll, engines, kind) {
   return out.join("\n");
 }
 
+/** Серии набора счётчиков по умолчанию dcgm-exporter 4.8.4 (etc/default-counters.csv): имя, тип, справка. */
+const DCGM_DEFAULT = [
+  ["DCGM_FI_DEV_SM_CLOCK", "gauge", "SM clock frequency (in MHz)."],
+  ["DCGM_FI_DEV_MEM_CLOCK", "gauge", "Memory clock frequency (in MHz)."],
+  ["DCGM_FI_DEV_MEMORY_TEMP", "gauge", "Memory temperature (in C)."],
+  ["DCGM_FI_DEV_GPU_TEMP", "gauge", "GPU temperature (in C)."],
+  ["DCGM_FI_DEV_POWER_USAGE", "gauge", "Power draw (in W)."],
+  ["DCGM_FI_DEV_TOTAL_ENERGY_CONSUMPTION", "counter", "Total energy consumption since the driver was last reloaded (in mJ)."],
+  ["DCGM_FI_DEV_PCIE_REPLAY_COUNTER", "counter", "Total number of PCIe retries."],
+  ["DCGM_FI_DEV_GPU_UTIL", "gauge", "GPU utilization (in %)."],
+  ["DCGM_FI_DEV_MEM_COPY_UTIL", "gauge", "Memory utilization (in %)."],
+  ["DCGM_FI_DEV_ENC_UTIL", "gauge", "Encoder utilization (in %)."],
+  ["DCGM_FI_DEV_DEC_UTIL", "gauge", "Decoder utilization (in %)."],
+  ["DCGM_FI_DEV_XID_ERRORS", "gauge", "Value of the last XID error encountered."],
+  ["DCGM_FI_DEV_FB_FREE", "gauge", "Framebuffer memory free (in MiB)."],
+  ["DCGM_FI_DEV_FB_USED", "gauge", "Framebuffer memory used (in MiB)."],
+  ["DCGM_FI_DEV_FB_RESERVED", "gauge", "Framebuffer memory reserved (in MiB)."],
+  ["DCGM_FI_DEV_NVLINK_BANDWIDTH_TOTAL", "gauge", "Aggregate NVLink throughput across all lanes (in MB/s)."],
+  ["DCGM_FI_DEV_VGPU_LICENSE_STATUS", "gauge", "vGPU License status"],
+  ["DCGM_FI_DEV_UNCORRECTABLE_REMAPPED_ROWS", "counter", "Number of remapped rows for uncorrectable errors"],
+  ["DCGM_FI_DEV_CORRECTABLE_REMAPPED_ROWS", "counter", "Number of remapped rows for correctable errors"],
+  ["DCGM_FI_DEV_ROW_REMAP_FAILURE", "gauge", "Whether remapping of rows has failed"],
+  ["DCGM_FI_PROF_GR_ENGINE_ACTIVE", "gauge", "Ratio of time the graphics engine is active."],
+  ["DCGM_FI_PROF_PIPE_TENSOR_ACTIVE", "gauge", "Ratio of cycles the tensor (HMMA) pipe is active."],
+  ["DCGM_FI_PROF_DRAM_ACTIVE", "gauge", "Ratio of cycles the device memory interface is active sending or receiving data."],
+  ["DCGM_FI_PROF_PCIE_TX_BYTES", "gauge", "The rate of data transmitted over the PCIe bus - including both protocol headers and data payloads - in bytes per second."],
+  ["DCGM_FI_PROF_PCIE_RX_BYTES", "gauge", "The rate of data received over the PCIe bus - including both protocol headers and data payloads - in bytes per second."],
+];
+
+/** Серии, которые набор по умолчанию не собирает (закомментированы в default-counters.csv), — у вида dcgm-full. */
+const DCGM_EXTRA = [
+  ["DCGM_FI_PROF_SM_ACTIVE", "gauge", "The ratio of cycles an SM has at least 1 warp assigned."],
+  ["DCGM_FI_DEV_ECC_SBE_VOL_TOTAL", "counter", "Total number of single-bit volatile ECC errors."],
+  ["DCGM_FI_DEV_ECC_DBE_VOL_TOTAL", "counter", "Total number of double-bit volatile ECC errors."],
+  ["DCGM_FI_DEV_CLOCKS_EVENT_REASONS", "gauge", "Current clock event reasons (bitmask of DCGM_CLOCKS_EVENT_REASON_*)."],
+  ["DCGM_FI_DEV_POWER_VIOLATION", "counter", "Throttling duration due to power constraints (in ns)."],
+  ["DCGM_FI_DEV_THERMAL_VIOLATION", "counter", "Throttling duration due to thermal constraints (in ns)."],
+  ["DCGM_FI_DEV_NVLINK_CRC_ERROR_TOTAL", "counter", "Total number of NVLink data-link CRC errors."],
+  ["DCGM_FI_DEV_NVLINK_RECOVERY_TOTAL", "counter", "Total number of NVLink data-link recovery errors."],
+  ["DCGM_FI_DEV_NVLINK_REPLAY_TOTAL", "counter", "Total number of NVLink data-link replay errors."],
+];
+
+/** Серии, которые есть у экземпляра MIG (у карты в режиме MIG нет GPU_UTIL). */
+const MIG_FIELDS = new Set(["DCGM_FI_DEV_FB_FREE", "DCGM_FI_DEV_FB_USED", "DCGM_FI_DEV_FB_RESERVED", "DCGM_FI_PROF_GR_ENGINE_ACTIVE", "DCGM_FI_PROF_PIPE_TENSOR_ACTIVE", "DCGM_FI_PROF_DRAM_ACTIVE"]);
+
+const DCGM_MODELS = { default: "NVIDIA H100 80GB HBM3", full: "NVIDIA H100 80GB HBM3", "4.1": "NVIDIA A100-SXM4-80GB", mig: "NVIDIA A100-SXM4-80GB" };
+
+/** Значение поля карты `gpu` на опросе `poll`: датчики колеблются, счётчики растут с каждым опросом. */
+function dcgmValue(name, gpu, poll, o) {
+  const wave = (period, phase = 0) => Math.abs(Math.sin((poll + gpu * 1.7 + phase) / period));
+  const total = 81559;
+  const used = Math.round(52000 + 20000 * wave(5));
+  switch (name) {
+    case "DCGM_FI_DEV_SM_CLOCK": return o.throttle ? 1095 : 1980;
+    case "DCGM_FI_DEV_MEM_CLOCK": return 2619;
+    case "DCGM_FI_DEV_MEMORY_TEMP": return Math.round(48 + 10 * wave(6));
+    case "DCGM_FI_DEV_GPU_TEMP": return o.throttle ? 89 : Math.round(45 + 25 * wave(6));
+    case "DCGM_FI_DEV_POWER_USAGE": return Number((180 + 450 * wave(4)).toFixed(3));
+    case "DCGM_FI_DEV_TOTAL_ENERGY_CONSUMPTION": return 9_000_000_000 + gpu * 1_000_000 + poll * 2_500_000;
+    case "DCGM_FI_DEV_PCIE_REPLAY_COUNTER": return o.throttle ? poll * 3 : 0;
+    case "DCGM_FI_DEV_GPU_UTIL": return Math.round(40 + 55 * wave(4));
+    case "DCGM_FI_DEV_MEM_COPY_UTIL": return Math.round(20 + 50 * wave(4, 1));
+    case "DCGM_FI_DEV_UNCORRECTABLE_REMAPPED_ROWS": return o.ecc && gpu === 0 ? 1 : 0;
+    case "DCGM_FI_DEV_XID_ERRORS": return o.xid && gpu === 0 ? o.xid : 0;
+    case "DCGM_FI_DEV_FB_USED": return used;
+    case "DCGM_FI_DEV_FB_RESERVED": return 541;
+    case "DCGM_FI_DEV_FB_FREE": return total - used - 541;
+    case "DCGM_FI_DEV_NVLINK_BANDWIDTH_TOTAL": return Math.round(30000 + 90000 * wave(5, 2));
+    case "DCGM_FI_PROF_GR_ENGINE_ACTIVE": return Number((0.4 + 0.55 * wave(4)).toFixed(6));
+    case "DCGM_FI_PROF_SM_ACTIVE": return Number((0.35 + 0.55 * wave(4)).toFixed(6));
+    case "DCGM_FI_PROF_PIPE_TENSOR_ACTIVE": return Number((0.1 + 0.4 * wave(4)).toFixed(6));
+    case "DCGM_FI_PROF_DRAM_ACTIVE": return Number((0.2 + 0.5 * wave(4, 1)).toFixed(6));
+    case "DCGM_FI_PROF_PCIE_TX_BYTES": return Math.round(2e8 + 6e8 * wave(3));
+    case "DCGM_FI_PROF_PCIE_RX_BYTES": return Math.round(5e8 + 9e8 * wave(3, 1));
+    case "DCGM_FI_DEV_ECC_SBE_VOL_TOTAL": return o.ecc && gpu === 0 ? 12 + poll : 0;
+    case "DCGM_FI_DEV_ECC_DBE_VOL_TOTAL": return o.ecc && gpu === 0 ? 2 : 0;
+    // HW_SLOWDOWN (0x8) | HW_THERMAL (0x40).
+    case "DCGM_FI_DEV_CLOCKS_EVENT_REASONS": return o.throttle ? 0x48 : 0;
+    case "DCGM_FI_DEV_POWER_VIOLATION": return o.throttle ? poll * 2_000_000_000 : 0;
+    case "DCGM_FI_DEV_THERMAL_VIOLATION": return o.throttle ? poll * 3_000_000_000 : 0;
+    case "DCGM_FI_DEV_NVLINK_CRC_ERROR_TOTAL":
+    case "DCGM_FI_DEV_NVLINK_RECOVERY_TOTAL":
+    case "DCGM_FI_DEV_NVLINK_REPLAY_TOTAL":
+      return o.throttle ? poll : 0;
+    default: return 0;
+  }
+}
+
+/** /metrics экспортёра DCGM в формате dcgm-exporter (internal/pkg/rendermetrics): метки карты gpu, UUID,
+ *  pci_bus_id, device, modelName, у MIG — GPU_I_PROFILE, GPU_I_ID; хост — hostname (4.8) или Hostname (до 4.1);
+ *  версия драйвера — меткой DCGM_FI_DRIVER_VERSION; --pod добавляет метки Kubernetes. */
+export function dcgmMetrics(kind, o, poll) {
+  const fields = kind === "full" ? [...DCGM_DEFAULT, ...DCGM_EXTRA] : DCGM_DEFAULT;
+  const model = o.model ?? DCGM_MODELS[kind];
+  const hostLabel = kind === "4.1" || kind === "mig" ? "Hostname" : "hostname";
+  const [namespace, pod] = o.pod ? o.pod.split("/") : [];
+  const labels = (gpu, mig) => {
+    const hex = gpu.toString(16).padStart(2, "0");
+    const parts = [
+      `gpu="${gpu}"`,
+      `UUID="GPU-6c2d1a3e-${hex}00-4b1c-9f3e-0a1b2c3d4e${hex}"`,
+      `pci_bus_id="00000000:${(0x18 + gpu * 0x10).toString(16).toUpperCase().padStart(2, "0")}:00.0"`,
+      `device="nvidia${gpu}"`,
+      `modelName="${model}"`,
+    ];
+    if (mig) parts.push(`GPU_I_PROFILE="3g.40gb"`, `GPU_I_ID="${mig}"`);
+    parts.push(`${hostLabel}="${o.hostname}"`, `DCGM_FI_DRIVER_VERSION="570.124.06"`);
+    if (pod) parts.push(`container="vllm"`, `namespace="${namespace}"`, `pod="${pod}"`);
+    return `{${parts.join(",")}}`;
+  };
+  const out = [];
+  for (const [name, type, help] of fields) {
+    out.push(`# HELP ${name} ${help}`, `# TYPE ${name} ${type}`);
+    for (let gpu = 0; gpu < o.gpus; gpu++) {
+      if (kind === "mig" && gpu === 0) {
+        // Карта в режиме MIG: память и профилировочные серии — по экземплярам, утилизации карты нет.
+        if (MIG_FIELDS.has(name))
+          for (const instance of [1, 2]) {
+            const v = name === "DCGM_FI_DEV_FB_USED" ? 15000 + instance * 4000 : name === "DCGM_FI_DEV_FB_FREE" ? 25000 : dcgmValue(name, gpu, poll, o);
+            out.push(`${name}${labels(gpu, String(instance))} ${v}`);
+          }
+        if (name === "DCGM_FI_DEV_GPU_UTIL" || MIG_FIELDS.has(name)) continue;
+      }
+      out.push(`${name}${labels(gpu)} ${dcgmValue(name, gpu, poll, o)}`);
+    }
+  }
+  if (kind === "full" && o.xid) {
+    out.push("# HELP DCGM_EXP_XID_ERRORS_COUNT XID errors observed during the configured time window.", "# TYPE DCGM_EXP_XID_ERRORS_COUNT gauge");
+    out.push(`DCGM_EXP_XID_ERRORS_COUNT${labels(0).replace(/}$/, `,window_size_in_ms="300000",xid="${o.xid}"}`)} 1`);
+  }
+  return out.join("\n") + "\n";
+}
+
 /** Обработчик запросов: ключ проверяется по правилу вида (kind.open); /health отвечает заданным кодом,
  *  у 3xx — с Location на https, как у настоящего перенаправления. */
 /** Сетка `query_range`: точки от start до end с шагом step, секунды. */
@@ -252,8 +417,11 @@ export function handler(opts) {
       if (opts.slow) setTimeout(send, opts.slow * 1000);
       else send();
     };
-    if (opts.apiKey && !kind.open(path) && req.headers.authorization !== `Bearer ${opts.apiKey}`)
+    // Экспортёр DCGM (web-config.yml dcgm-exporter) закрывается basic с логином glafira, остальные виды — токеном.
+    const expected = kind.dcgm ? `Basic ${Buffer.from(`glafira:${opts.apiKey}`).toString("base64")}` : `Bearer ${opts.apiKey}`;
+    if (opts.apiKey && !kind.open(path) && req.headers.authorization !== expected)
       return reply(401, "application/json", JSON.stringify({ error: "Unauthorized" }));
+    if (kind.dcgm && path === "/metrics") return reply(200, "text/plain; version=0.0.4", dcgmMetrics(kind.dcgm, opts.dcgm, polls++));
     if (path === "/health" && !kind.noHealth) {
       const redirect = opts.health >= 300 && opts.health < 400;
       const authority = req.headers.host ?? `${opts.host}:${opts.port}`;
@@ -286,7 +454,7 @@ function main() {
     process.exit(1);
   });
   server.listen(opts.port, opts.host, () => {
-    const key = opts.apiKey ? `, ключ API ${opts.apiKey}` : "";
+    const key = !opts.apiKey ? "" : KINDS[opts.kind].dcgm ? `, вход glafira / ${opts.apiKey}` : `, ключ API ${opts.apiKey}`;
     console.log(`✓ мок ${opts.kind}: http://${opts.host}:${opts.port}${key}`);
   });
 }
````

<!-- block: patch t01-scripts-prometheus-smoke-yml step=1.3 -->
````
diff --git a/scripts/prometheus/smoke.yml b/scripts/prometheus/smoke.yml
index f97877e..9b49a57 100644
--- a/scripts/prometheus/smoke.yml
+++ b/scripts/prometheus/smoke.yml
@@ -1,8 +1,11 @@
-# Prometheus для смоука истории метрик (docs/smoke/i1c.md): опрашивает мок-серверы раз в 5 с.
+# Prometheus для смоука истории метрик и GPU (docs/smoke/i1c.md, docs/smoke/i1d.md): опрашивает мок-серверы раз в 5 с.
 #   pnpm mock-server --kind vllm --port 8000
 #   pnpm mock-server --kind vllm --engines 2 --port 8001
 #   pnpm mock-server --kind vllm --reset-after 20 --port 8003
 #   pnpm mock-server --kind sglang --port 30000
+#   pnpm mock-server --kind dcgm-full --gpus 8 --hostname gpu-node-1 --port 9400
+#   pnpm mock-server --kind dcgm --gpus 2 --port 9401
+#   pnpm mock-server --kind dcgm --gpus 72 --hostname big-node --port 9402   # только для проверки предела 64 карт
 #   prometheus --config.file=scripts/prometheus/smoke.yml --storage.tsdb.path=<каталог> --web.listen-address=127.0.0.1:9090
 # Настоящий vLLM на Apple Silicon (vllm-metal, порт 8100) — раскомментировать цель ниже.
 # Тот же конфиг подходит VictoriaMetrics (-promscrape.config): правил нет, поэтому evaluation_interval не нужен —
@@ -17,3 +20,6 @@ scrape_configs:
   - job_name: sglang
     static_configs:
       - targets: ["127.0.0.1:30000"]
+  - job_name: dcgm
+    static_configs:
+      - targets: ["127.0.0.1:9400", "127.0.0.1:9401", "127.0.0.1:9402"]
````

- [ ] **Шаг 1.4: Проверки.**

Run: `pnpm test:scripts`
Expected: `# pass 115`, `# fail 0`
Run: `promtool check config scripts/prometheus/smoke.yml`
Expected: `SUCCESS` (если Prometheus установлен)

- [ ] **Шаг 1.5: Коммит.**

<!-- block: shell t01-commit step=1.5 -->
````
git add README.md \
  scripts/mock-server.mjs \
  scripts/mock-server.test.mjs \
  scripts/prometheus/smoke.yml
git commit -s -q -F - <<'EOF'
feat(mock): DCGM exporter kinds for the GPU smoke

Kinds dcgm, dcgm-full, dcgm-4.1 and dcgm-mig answer /metrics in the
dcgm-exporter 4.8.4 format built from its sources: card labels, the
default counter set and the extra counters, the old Hostname label, MIG
instances and Kubernetes labels. --gpus, --hostname, --model, --pod,
--throttle, --xid and --ecc shape the cards; --api-key is a basic
password. The smoke Prometheus scrapes the DCGM mocks on 9400-9402.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
````

---

### Задача 2: Компонент GPU и привязка сервера в профиле

Профиль инсталляции получает необязательный компонент `gpu` (спека I1d §2): экспортёры DCGM (`direct`: адреса,
вход basic, CA, прокси, селектор серий каждого экспортёра в истории) или серии DCGM источника истории (`prometheus`:
селектор карт); сервер — необязательную привязку `gpus { host, indices? }`. Пароль экспортёров — одной записью
keychain `<инсталляция>/gpu/secret` по правилам секрета истории: сохранение формы задаёт и удаляет его, удаление
инсталляции удаляет, экспорт ставит `needsSecret.gpu`, импорт предупреждает о смене адресов экспортёров и удаляет
ненужный пароль. Экспортёры без проверки сертификата запрещает политика `allowInsecureTls = false`. Без компонента GPU
нормализация убирает привязки серверов.

**Files:**
- Modify: `src-tauri/src/commands/folders.rs`
- Modify: `src-tauri/src/commands/installations.rs`
- Modify: `src-tauri/src/history/client.rs`
- Modify: `src-tauri/src/secrets.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/storage/exchange.rs`
- Modify: `src-tauri/src/storage/profile.rs`
- Modify: `src/locales/en.json`
- Modify: `src/locales/ru.json`

**Interfaces:**
- Produces: `storage::profile::{GpuSource, DirectGpu, PrometheusGpu, GpuExporter, GpuAuth, GpuBinding, MAX_EXPORTERS,
  GPU_NAME, parse_exporter_url, validate_gpu}`; поля `InstallationProfile.gpu: Option<GpuSource>` (JSON `gpu`, тег
  `mode`: `direct` | `prometheus`) и `ServerConfig.gpus: Option<GpuBinding>` (JSON `gpus`); `secrets::{GPU,
  GPU_SECRET, gpu_secret_key}`; `SecretsInput.gpu_secret` (JSON `gpuSecret`); `commands::installations::{gpu_op,
  gpu_has_login}`; `NeedsSecret.gpu`; `ImportPlan.removed_gpu`; тестовые `profile::{sample_gpu, sample_exporter}`.

- [ ] **Шаг 2.3: Реализация.** Профиль, нормализация и проверка компонента, пароль в keychain при сохранении и удалении, экспорт и импорт, тексты
ошибок и предупреждений. Тесты — в изменённых модулях (`profile.rs`, `commands/installations.rs`, `exchange.rs`,
`secrets.rs`), поэтому отдельного шага с падением у задачи нет.

<!-- block: patch t02-src-tauri-src-commands-folders-rs step=2.3 -->
````
diff --git a/src-tauri/src/commands/folders.rs b/src-tauri/src/commands/folders.rs
index 7bee482..81e99d9 100644
--- a/src-tauri/src/commands/folders.rs
+++ b/src-tauri/src/commands/folders.rs
@@ -165,10 +165,15 @@ fn import_from_file(
     if !apply {
         return Ok(plan_import(&snapshot, &parsed, &opts, &stored)?.preview);
     }
-    let (preview, removed, removed_history) = st.installations.update(|data| {
+    let (preview, removed, removed_history, removed_gpu) = st.installations.update(|data| {
         let plan = plan_import(data, &parsed, &opts, &stored)?;
         *data = plan.next;
-        Ok((plan.preview, plan.removed_servers, plan.removed_history))
+        Ok((
+            plan.preview,
+            plan.removed_servers,
+            plan.removed_history,
+            plan.removed_gpu,
+        ))
     })?;
     // Секреты серверов, которых больше нет в профиле, не нужны: удаляются все, как при
     // сохранении и удалении инсталляции. Сбой keychain импорт не отменяет: профиль уже
@@ -186,6 +191,12 @@ fn import_from_file(
             tracing::warn!("history secret of installation {installation} was not deleted: {e}");
         }
     }
+    for installation in removed_gpu {
+        let key = crate::secrets::gpu_secret_key(&installation);
+        if let Err(e) = st.secrets.delete(&key) {
+            tracing::warn!("GPU secret of installation {installation} was not deleted: {e}");
+        }
+    }
     // Обновлённые инсталляции подключатся заново с новым профилем — как при сохранении формы.
     for item in &preview.updated {
         st.evict_installation(&item.id);
````

<!-- block: patch t02-src-tauri-src-commands-installations-rs step=2.3 -->
````
diff --git a/src-tauri/src/commands/installations.rs b/src-tauri/src/commands/installations.rs
index 7ad3b3b..32bd01b 100644
--- a/src-tauri/src/commands/installations.rs
+++ b/src-tauri/src/commands/installations.rs
@@ -5,17 +5,18 @@ use tauri::State;
 
 use super::blocking;
 use crate::error::AppResult;
-use crate::secrets::{API_KEY, history_secret_key, secret_key};
+use crate::secrets::{API_KEY, gpu_secret_key, history_secret_key, secret_key};
 use crate::servers::client::{Timeouts, ensure_allowed, ensure_ca_allowed};
 use crate::servers::connectivity::{TestReport, test_server};
 use crate::servers::pool::InstallationInfo;
 use crate::state::AppState;
 use crate::storage::exchange::{NeedsSecret, needs_secret};
-use crate::storage::profile::{HistoryAuth, InstallationProfile};
+use crate::storage::profile::{GpuAuth, GpuSource, HistoryAuth, InstallationProfile};
 
 /// Секреты из формы. Ключи API: id сервера → значение; сервера нет в списке — оставить
-/// сохранённый, `""` — удалить, иное — сохранить. Секрет источника истории (токен или пароль) —
-/// по тем же правилам: `None` — оставить, `""` — удалить. `Debug` печатает вместо значений `***`.
+/// сохранённый, `""` — удалить, иное — сохранить. Секрет источника истории (токен или пароль) и
+/// пароль экспортёров DCGM — по тем же правилам: `None` — оставить, `""` — удалить. `Debug`
+/// печатает вместо значений `***`.
 #[derive(Default, Clone, Deserialize)]
 #[serde(rename_all = "camelCase")]
 pub struct SecretsInput {
@@ -23,6 +24,8 @@ pub struct SecretsInput {
     pub api_keys: BTreeMap<String, String>,
     #[serde(default)]
     pub history_secret: Option<String>,
+    #[serde(default)]
+    pub gpu_secret: Option<String>,
 }
 
 /// Ключи не печатаются: структура может попасть в лог или сообщение об ошибке.
@@ -35,6 +38,7 @@ impl std::fmt::Debug for SecretsInput {
                     .as_ref()
                     .map(|_| ("historySecret", "***")),
             )
+            .entries(self.gpu_secret.as_ref().map(|_| ("gpuSecret", "***")))
             .finish()
     }
 }
@@ -117,6 +121,29 @@ pub(crate) fn history_op(
     }
 }
 
+/// Экспортёры DCGM с входом basic: только им нужен пароль.
+pub(crate) fn gpu_has_login(p: &InstallationProfile) -> bool {
+    matches!(&p.gpu, Some(GpuSource::Direct(d)) if d.auth != GpuAuth::None)
+}
+
+/// Пароль экспортёров DCGM — по данным формы, как секрет источника истории: без входа basic он не
+/// нужен, сохранённый остаётся, только если вход basic был и в прежнем профиле.
+pub(crate) fn gpu_op(
+    saved: &InstallationProfile,
+    previous: Option<&InstallationProfile>,
+    s: &SecretsInput,
+) -> SecretOp {
+    if !gpu_has_login(saved) {
+        return SecretOp::Delete;
+    }
+    match s.gpu_secret.as_deref().map(str::trim) {
+        Some("") => SecretOp::Delete,
+        Some(v) => SecretOp::Set(v.to_string()),
+        None if previous.is_some_and(gpu_has_login) => SecretOp::Keep,
+        None => SecretOp::Delete,
+    }
+}
+
 /// Серверы прежнего профиля, которых нет в сохранённом: их ключи больше не нужны.
 fn removed_servers(
     saved: &InstallationProfile,
@@ -204,6 +231,9 @@ pub fn save_installation_blocking(
     if let Some(h) = &profile.history {
         ensure_ca_allowed(&h.ca, &st.policy.status)?;
     }
+    if let Some(GpuSource::Direct(d)) = &profile.gpu {
+        ensure_ca_allowed(&d.ca, &st.policy.status)?;
+    }
     if profile.id.is_empty() {
         profile.id = uuid::Uuid::new_v4().to_string();
     }
@@ -220,6 +250,10 @@ pub fn save_installation_blocking(
         history_secret_key(&saved.id),
         history_op(&saved, previous.as_ref(), &secrets),
     ));
+    ops.push((
+        gpu_secret_key(&saved.id),
+        gpu_op(&saved, previous.as_ref(), &secrets),
+    ));
     let result = apply_secret_ops(st, ops);
     if result.is_err() {
         let rollback = match previous {
@@ -321,7 +355,8 @@ mod tests {
     use crate::error::{AppError, ErrorCode};
     use crate::secrets::{MemoryStore, SecretStore};
     use crate::storage::profile::{
-        CaConfig, HistoryAuth, HistorySource, sample_history, sample_profile, sample_server,
+        CaConfig, DirectGpu, GpuAuth, GpuSource, HistoryAuth, HistorySource, PrometheusGpu,
+        sample_gpu, sample_history, sample_profile, sample_server,
     };
 
     fn state(dir: &std::path::Path) -> AppState {
@@ -337,6 +372,7 @@ mod tests {
                     .map(|(k, v)| (k.to_string(), v.to_string()))
                     .collect(),
                 history_secret: None,
+                gpu_secret: None,
             },
         }
     }
@@ -359,6 +395,7 @@ mod tests {
                 .map(|(k, v)| (k.to_string(), v.to_string()))
                 .collect(),
             history_secret: None,
+            gpu_secret: None,
         };
         // Убранный сервер (`gone`) в операции не входит: его ключ чистится после фиксации.
         // Новый сервер без ключа в форме (`s4`) получает `Delete`, а не `Keep`: осиротевший
@@ -448,6 +485,88 @@ mod tests {
         assert_eq!(read(), None);
     }
 
+    fn with_gpu(auth: GpuAuth) -> InstallationProfile {
+        let mut p = sample_profile("a");
+        let GpuSource::Direct(d) = sample_gpu() else {
+            unreachable!()
+        };
+        p.gpu = Some(GpuSource::Direct(DirectGpu { auth, ..d }));
+        p
+    }
+
+    #[test]
+    fn gpu_secret_ops() {
+        let basic = with_gpu(GpuAuth::Basic {
+            username: "glafira".into(),
+        });
+        let none = with_gpu(GpuAuth::None);
+        let mut prom = sample_profile("a");
+        prom.history = Some(sample_history());
+        prom.gpu = Some(GpuSource::Prometheus(PrometheusGpu {
+            selector: None,
+            extra: Default::default(),
+        }));
+        let typed = |v: Option<&str>| SecretsInput {
+            gpu_secret: v.map(String::from),
+            ..SecretsInput::default()
+        };
+        assert_eq!(gpu_op(&basic, Some(&basic), &typed(None)), SecretOp::Keep);
+        assert_eq!(
+            gpu_op(&basic, None, &typed(Some(" p "))),
+            SecretOp::Set("p".into())
+        );
+        assert_eq!(
+            gpu_op(&basic, Some(&basic), &typed(Some(""))),
+            SecretOp::Delete
+        );
+        // Пароль не берётся неоткуда: вход basic появился без пароля в форме.
+        assert_eq!(gpu_op(&basic, Some(&none), &typed(None)), SecretOp::Delete);
+        for without in [&none, &prom, &sample_profile("a")] {
+            assert_eq!(
+                gpu_op(without, Some(&basic), &typed(Some("p"))),
+                SecretOp::Delete
+            );
+        }
+        let s = typed(Some("p"));
+        assert_eq!(format!("{s:?}"), r#"{"gpuSecret": "***"}"#);
+    }
+
+    #[test]
+    fn gpu_secret_is_saved_and_deleted_with_the_installation() {
+        let dir = tempfile::tempdir().unwrap();
+        let st = state(dir.path());
+        let mut i = input(
+            with_gpu(GpuAuth::Basic {
+                username: "glafira".into(),
+            }),
+            &[],
+        );
+        i.secrets.gpu_secret = Some("dcgm-pa55".into());
+        let saved = save_installation_blocking(&st, i).unwrap();
+        let read = || st.secrets.get(&gpu_secret_key(&saved.id)).unwrap();
+        assert_eq!(read().as_deref(), Some("dcgm-pa55"));
+        let profile = std::fs::read_to_string(dir.path().join("installations.json")).unwrap();
+        assert!(!profile.contains("dcgm-pa55"), "{profile}");
+        save_installation_blocking(&st, input(saved.clone(), &[])).unwrap();
+        assert_eq!(read().as_deref(), Some("dcgm-pa55"));
+        st.delete_installation_full(&saved.id).unwrap();
+        assert_eq!(read(), None);
+    }
+
+    #[test]
+    fn policy_forbids_insecure_dcgm_exporters() {
+        let dir = tempfile::tempdir().unwrap();
+        let mut policy = crate::policy::Policy::default();
+        policy.status.allow_insecure_tls = false;
+        let st = state(dir.path()).with_policy(policy);
+        let mut p = with_gpu(GpuAuth::None);
+        if let Some(GpuSource::Direct(d)) = &mut p.gpu {
+            d.ca = CaConfig::Insecure;
+        }
+        let e = save_installation_blocking(&st, input(p, &[])).unwrap_err();
+        assert_eq!(e.key, "errors.installations.insecureForbidden");
+    }
+
     #[test]
     fn policy_forbids_an_insecure_history_source() {
         let dir = tempfile::tempdir().unwrap();
````

<!-- block: patch t02-src-tauri-src-history-client-rs step=2.3 -->
````
diff --git a/src-tauri/src/history/client.rs b/src-tauri/src/history/client.rs
index bb4b1f6..faf193f 100644
--- a/src-tauri/src/history/client.rs
+++ b/src-tauri/src/history/client.rs
@@ -349,6 +349,7 @@ pub fn as_server(h: &HistorySource) -> ServerConfig {
         ca: h.ca.clone(),
         proxy: h.proxy.clone(),
         history_selector: None,
+        gpus: None,
         extra: Default::default(),
     }
 }
````

<!-- block: patch t02-src-tauri-src-secrets-rs step=2.3 -->
````
diff --git a/src-tauri/src/secrets.rs b/src-tauri/src/secrets.rs
index 280f658..cfddc1e 100644
--- a/src-tauri/src/secrets.rs
+++ b/src-tauri/src/secrets.rs
@@ -32,6 +32,17 @@ pub fn history_secret_key(installation_id: &str) -> String {
     secret_key(installation_id, HISTORY, HISTORY_SECRET)
 }
 
+/// Компонент GPU в ключе keychain (спека I1d §2); как у источника истории, поле `secret` у серверов
+/// не бывает.
+pub const GPU: &str = "gpu";
+/// Пароль basic экспортёров DCGM — общий для всех экспортёров компонента.
+pub const GPU_SECRET: &str = "secret";
+
+/// Ключ keychain пароля экспортёров DCGM инсталляции.
+pub fn gpu_secret_key(installation_id: &str) -> String {
+    secret_key(installation_id, GPU, GPU_SECRET)
+}
+
 /// OS keychain: macOS Keychain, Windows Credential Manager, Secret Service.
 /// `service` — имя сервиса записей; прежние имена нужны только для миграции.
 pub struct KeyringStore {
@@ -155,6 +166,7 @@ mod tests {
     fn key_format() {
         assert_eq!(secret_key("i1", "s1", API_KEY), "i1/s1/apiKey");
         assert_eq!(history_secret_key("i1"), "i1/history/secret");
+        assert_eq!(gpu_secret_key("i1"), "i1/gpu/secret");
     }
 
     #[test]
````

<!-- block: patch t02-src-tauri-src-state-rs step=2.3 -->
````
diff --git a/src-tauri/src/state.rs b/src-tauri/src/state.rs
index 7d37ec6..cf7e4e0 100644
--- a/src-tauri/src/state.rs
+++ b/src-tauri/src/state.rs
@@ -96,6 +96,9 @@ impl AppState {
         if let Err(e) = self.secrets.delete(&crate::secrets::history_secret_key(id)) {
             tracing::warn!("history secret of the deleted installation {id} was not deleted: {e}");
         }
+        if let Err(e) = self.secrets.delete(&crate::secrets::gpu_secret_key(id)) {
+            tracing::warn!("GPU secret of the deleted installation {id} was not deleted: {e}");
+        }
         Ok(())
     }
 }
````

<!-- block: patch t02-src-tauri-src-storage-exchange-rs step=2.3 -->
````
diff --git a/src-tauri/src/storage/exchange.rs b/src-tauri/src/storage/exchange.rs
index c98530e..0c3eec8 100644
--- a/src-tauri/src/storage/exchange.rs
+++ b/src-tauri/src/storage/exchange.rs
@@ -7,13 +7,14 @@ use serde::{Deserialize, Serialize};
 
 use crate::error::{AppResult, ErrorCode};
 use crate::i18n::LocalizedText;
-use crate::secrets::{API_KEY, SecretStore, history_secret_key, secret_key};
+use crate::secrets::{API_KEY, SecretStore, gpu_secret_key, history_secret_key, secret_key};
 use crate::storage::installations::{
     InstallationFolder, InstallationProfile, InstallationStore, StoreData, check_tree,
     folder_not_found, unique_name, validate_profile,
 };
 use crate::storage::profile::{
-    CaConfig, HistoryAuth, HistorySource, ServerConfig, normalize_profile, strip_private_keys,
+    CaConfig, DirectGpu, GpuAuth, GpuSource, HistoryAuth, HistorySource, ServerConfig,
+    normalize_profile, strip_private_keys,
 };
 use crate::storage::tree::TreeItemKind;
 
@@ -30,6 +31,9 @@ pub struct NeedsSecret {
     /// Был сохранён секрет источника истории (токен или пароль, спека I1c §2).
     #[serde(default, skip_serializing_if = "std::ops::Not::not")]
     pub history: bool,
+    /// Был сохранён пароль экспортёров DCGM (спека I1d §2).
+    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
+    pub gpu: bool,
 }
 
 #[derive(Serialize)]
@@ -79,6 +83,21 @@ fn history_with_secret(p: &InstallationProfile) -> Option<&HistorySource> {
     p.history.as_ref().filter(|h| h.auth != HistoryAuth::None)
 }
 
+/// Сохранён ли непустой пароль экспортёров DCGM инсталляции `id`.
+fn has_gpu_secret(secrets: &dyn SecretStore, id: &str) -> AppResult<bool> {
+    Ok(secrets
+        .get(&gpu_secret_key(id))?
+        .is_some_and(|v| !v.is_empty()))
+}
+
+/// Экспортёры DCGM с входом basic: только им нужен пароль.
+fn gpu_with_secret(p: &InstallationProfile) -> Option<&DirectGpu> {
+    match &p.gpu {
+        Some(GpuSource::Direct(d)) if d.auth != GpuAuth::None => Some(d),
+        _ => None,
+    }
+}
+
 /// Серверы профиля, у которых сохранён ключ API, и сохранён ли секрет источника истории:
 /// устаревшие записи keychain (сервер удалён, вход в источник выключен) не требуют ввода у
 /// получателя.
@@ -93,11 +112,16 @@ pub(crate) fn needs_secret(
         }
     }
     let history = history_with_secret(p).is_some() && has_history_secret(secrets, &p.id)?;
-    Ok(NeedsSecret { api_keys, history })
+    let gpu = gpu_with_secret(p).is_some() && has_gpu_secret(secrets, &p.id)?;
+    Ok(NeedsSecret {
+        api_keys,
+        history,
+        gpu,
+    })
 }
 
-/// Неизвестные поля профиля, серверов и источника истории — локальные: в файл обмена и из него
-/// не переносятся.
+/// Неизвестные поля профиля, серверов, источника истории и компонента GPU — локальные: в файл
+/// обмена и из него не переносятся.
 fn clear_extra(p: &mut InstallationProfile) {
     p.extra.clear();
     for s in &mut p.servers {
@@ -106,6 +130,16 @@ fn clear_extra(p: &mut InstallationProfile) {
     if let Some(h) = &mut p.history {
         h.extra.clear();
     }
+    match &mut p.gpu {
+        Some(GpuSource::Direct(d)) => {
+            d.extra.clear();
+            for e in &mut d.exporters {
+                e.extra.clear();
+            }
+        }
+        Some(GpuSource::Prometheus(g)) => g.extra.clear(),
+        None => {}
+    }
 }
 
 /// Приватный ключ из поля CA в файл обмена не попадает, сертификаты остаются (спец. §5.1:
@@ -113,7 +147,17 @@ fn clear_extra(p: &mut InstallationProfile) {
 /// править руками), поэтому ключ вырезается и здесь.
 fn strip_pem_keys_for_export(p: &mut InstallationProfile) {
     let history = p.history.as_mut().map(|h| &mut h.ca);
-    for ca in p.servers.iter_mut().map(|s| &mut s.ca).chain(history) {
+    let gpu = match &mut p.gpu {
+        Some(GpuSource::Direct(d)) => Some(&mut d.ca),
+        _ => None,
+    };
+    for ca in p
+        .servers
+        .iter_mut()
+        .map(|s| &mut s.ca)
+        .chain(history)
+        .chain(gpu)
+    {
         if let CaConfig::Pem { pem } = ca {
             *pem = strip_private_keys(pem);
         }
@@ -366,6 +410,9 @@ pub(crate) struct ImportPlan {
     /// Обновлённые инсталляции, у которых источник истории убран или сменил вид входа: их
     /// секрет источника удаляется из keychain после записи.
     pub removed_history: Vec<String>,
+    /// Обновлённые инсталляции, у которых экспортёры DCGM лишились входа basic (компонент убран,
+    /// режим или вход сменились): их пароль удаляется из keychain после записи.
+    pub removed_gpu: Vec<String>,
 }
 
 /// Имена папок от корня до `folder` включительно.
@@ -459,6 +506,28 @@ fn history_secret_survives(local: &InstallationProfile, new: &InstallationProfil
     }
 }
 
+/// Меняется ли то, куда уходит пароль экспортёров DCGM: адреса экспортёров, прокси или проверка
+/// сертификата.
+fn gpu_moves(local: &InstallationProfile, new: &InstallationProfile) -> bool {
+    match (gpu_with_secret(local), gpu_with_secret(new)) {
+        (Some(l), Some(n)) => {
+            let urls = |d: &DirectGpu| {
+                d.exporters
+                    .iter()
+                    .map(|e| e.url.clone())
+                    .collect::<Vec<_>>()
+            };
+            urls(l) != urls(n) || l.proxy != n.proxy || l.ca != n.ca
+        }
+        _ => false,
+    }
+}
+
+/// Остаётся ли сохранённый пароль экспортёров после обновления: вход basic есть и до, и после.
+fn gpu_secret_survives(local: &InstallationProfile, new: &InstallationProfile) -> bool {
+    gpu_with_secret(local).is_some() && gpu_with_secret(new).is_some()
+}
+
 /// Пары «сервер до импорта — сервер из файла» с тем же id, у которых меняется направление.
 fn moved_servers<'a>(
     local: &'a InstallationProfile,
@@ -482,6 +551,8 @@ pub(crate) struct StoredSecrets {
     by_server: HashMap<(String, String), bool>,
     /// id инсталляции → есть ли сохранённый секрет источника истории.
     by_history: HashMap<String, bool>,
+    /// id инсталляции → есть ли сохранённый пароль экспортёров DCGM.
+    by_gpu: HashMap<String, bool>,
 }
 
 impl StoredSecrets {
@@ -495,6 +566,7 @@ impl StoredSecrets {
     ) -> Self {
         let mut by_server = HashMap::new();
         let mut by_history = HashMap::new();
+        let mut by_gpu = HashMap::new();
         for (incoming, _) in &parsed.installations {
             let Some(local) = data.installations.iter().find(|l| l.id == incoming.id) else {
                 continue;
@@ -507,10 +579,15 @@ impl StoredSecrets {
                 let stored = has_history_secret(secrets, &incoming.id).unwrap_or(true);
                 by_history.insert(incoming.id.clone(), stored);
             }
+            if gpu_moves(local, incoming) {
+                let stored = has_gpu_secret(secrets, &incoming.id).unwrap_or(true);
+                by_gpu.insert(incoming.id.clone(), stored);
+            }
         }
         Self {
             by_server,
             by_history,
+            by_gpu,
         }
     }
 
@@ -527,6 +604,11 @@ impl StoredSecrets {
     fn has_history(&self, id: &str) -> bool {
         self.by_history.get(id).copied().unwrap_or(true)
     }
+
+    /// Сохранён ли пароль экспортёров DCGM инсталляции `id` (не прочитанное — сохранён).
+    fn has_gpu(&self, id: &str) -> bool {
+        self.by_gpu.get(id).copied().unwrap_or(true)
+    }
 }
 
 /// Предупреждения об обновлении инсталляции, у сервера которой меняется то, куда уходит
@@ -579,6 +661,13 @@ fn destination_warnings(
             name = new.name
         ));
     }
+    if gpu_moves(local, new) && stored.has_gpu(&new.id) {
+        out.push(ltext!(
+            "messages.import.gpuChanged",
+            "“{{name}}”: the addresses, proxy or certificate verification of the DCGM exporters change. The saved password will be sent with the new settings.",
+            name = new.name
+        ));
+    }
     out
 }
 
@@ -604,6 +693,7 @@ pub(crate) fn plan_import(
     };
     let mut removed_servers = Vec::new();
     let mut removed_history = Vec::new();
+    let mut removed_gpu = Vec::new();
     // id папок файла совпадают с id в хранилище: существующие берутся как есть, новые создаются с тем же id.
     let file_folders: HashSet<&str> = parsed.folders.iter().map(|f| f.id.as_str()).collect();
     let local_folders: HashSet<&str> = data.folders.iter().map(|f| f.id.as_str()).collect();
@@ -654,8 +744,12 @@ pub(crate) fn plan_import(
             .history
             .as_ref()
             .is_some_and(|h| h.ca == CaConfig::Insecure);
+        let insecure_gpu =
+            matches!(&incoming.gpu, Some(GpuSource::Direct(d)) if d.ca == CaConfig::Insecure);
         if !opts.allow_insecure_tls
-            && (insecure_history || incoming.servers.iter().any(|s| s.ca == CaConfig::Insecure))
+            && (insecure_history
+                || insecure_gpu
+                || incoming.servers.iter().any(|s| s.ca == CaConfig::Insecure))
         {
             preview.skipped.push(Skipped {
                 name: incoming.name.clone(),
@@ -684,6 +778,31 @@ pub(crate) fn plan_import(
             if let (Some(h), Some(old)) = (&mut profile.history, &slot.history) {
                 h.extra = old.extra.clone();
             }
+            match (&mut profile.gpu, &slot.gpu) {
+                (Some(GpuSource::Direct(d)), Some(GpuSource::Direct(old))) => {
+                    d.extra = old.extra.clone();
+                    for e in &mut d.exporters {
+                        if let Some(o) = old.exporters.iter().find(|o| o.url == e.url) {
+                            e.extra = o.extra.clone();
+                        }
+                    }
+                }
+                (Some(GpuSource::Prometheus(g)), Some(GpuSource::Prometheus(old))) => {
+                    g.extra = old.extra.clone();
+                }
+                _ => {}
+            }
+            // Пароль экспортёров, который больше не нужен, удаляется, как секрет источника.
+            if gpu_with_secret(slot).is_some() && !gpu_secret_survives(slot, &profile) {
+                removed_gpu.push(profile.id.clone());
+                if profile.gpu.is_none() {
+                    preview.warnings.push(ltext!(
+                        "messages.import.gpuRemoved",
+                        "“{{name}}”: the GPU component is not in the file and will be removed together with the saved password of the DCGM exporters",
+                        name = profile.name
+                    ));
+                }
+            }
             // Секрет источника истории, который больше не нужен, удаляется: источника нет в
             // файле или у него другой вид входа.
             if history_with_secret(slot).is_some() && !history_secret_survives(slot, &profile) {
@@ -751,7 +870,7 @@ pub(crate) fn plan_import(
         profile.folder_id = folder;
         profile.name = name;
         clear_extra(&mut profile);
-        if !needs.api_keys.is_empty() || needs.history {
+        if !needs.api_keys.is_empty() || needs.history || needs.gpu {
             preview.needs_secrets += 1;
         }
         added.push((TreeItemKind::Installation, profile.id.clone()));
@@ -792,6 +911,7 @@ pub(crate) fn plan_import(
         preview,
         removed_servers,
         removed_history,
+        removed_gpu,
     })
 }
 
@@ -1228,6 +1348,107 @@ mod tests {
         assert!(warning_keys(&p.preview).is_empty());
     }
 
+    fn with_gpu(auth: GpuAuth) -> InstallationProfile {
+        let mut p = sample_profile("a");
+        let GpuSource::Direct(d) = crate::storage::profile::sample_gpu() else {
+            unreachable!()
+        };
+        p.gpu = Some(GpuSource::Direct(DirectGpu { auth, ..d }));
+        p
+    }
+
+    fn gpu_json(auth: serde_json::Value) -> serde_json::Value {
+        serde_json::json!({
+            "mode": "direct",
+            "exporters": [{ "url": "http://gpu-1.dev.example.com:9400/metrics" }],
+            "auth": auth
+        })
+    }
+
+    fn glafira() -> GpuAuth {
+        GpuAuth::Basic {
+            username: "glafira".into(),
+        }
+    }
+
+    #[test]
+    fn gpu_secret_is_flagged_but_never_exported() {
+        let (_d, s) = store();
+        s.upsert(with_gpu(glafira())).unwrap();
+        let mut off = with_gpu(GpuAuth::None);
+        off.id = "off".into();
+        s.upsert(off).unwrap();
+        let secrets = MemoryStore::default();
+        secrets.set(&gpu_secret_key("a"), "dcgm-pa55").unwrap();
+        secrets.set(&gpu_secret_key("off"), "old").unwrap();
+        let out = build_export(&s, &secrets, None, "1", "now").unwrap();
+        let json = serde_json::to_string(&out).unwrap();
+        assert!(!json.contains("dcgm-pa55"), "{json}");
+        let flags = |id: &str| {
+            out.installations
+                .iter()
+                .find(|c| c.profile.id == id)
+                .unwrap()
+                .needs_secret
+                .clone()
+        };
+        assert!(flags("a").gpu);
+        assert!(
+            !flags("off").gpu,
+            "пароль без входа basic — устаревшая запись"
+        );
+        assert!(!json.contains(r#""gpu":false"#), "{json}");
+    }
+
+    #[test]
+    fn exporter_change_with_a_saved_password_warns() {
+        let secrets = MemoryStore::default();
+        secrets.set(&gpu_secret_key("a"), "p").unwrap();
+        let moved = |c: &mut serde_json::Value| {
+            c["gpu"] = gpu_json(serde_json::json!({ "mode": "basic", "username": "glafira" }));
+            c["gpu"]["exporters"][0]["url"] = "http://gpu-2:9400/metrics".into();
+        };
+        let p = plan_update(with_gpu(glafira()), &secrets, moved);
+        assert_eq!(warning_keys(&p), ["messages.import.gpuChanged"]);
+        let p = plan_update(with_gpu(glafira()), &MemoryStore::default(), moved);
+        assert!(
+            warning_keys(&p).is_empty(),
+            "без сохранённого пароля предупреждать не о чем"
+        );
+    }
+
+    #[test]
+    fn removed_gpu_component_drops_its_password() {
+        let data = StoreData {
+            installations: vec![with_gpu(glafira())],
+            ..Default::default()
+        };
+        let parsed = parse_exchange(&file(vec![], vec![inst_json("a", "inst-a", None)])).unwrap();
+        let p = plan_with(&data, &parsed, &opts(None), &MemoryStore::default()).unwrap();
+        assert_eq!(p.removed_gpu, ["a"]);
+        assert_eq!(warning_keys(&p.preview), ["messages.import.gpuRemoved"]);
+        // Вход выключен — пароль не нужен, компонент остаётся: без предупреждения.
+        let mut none = inst_json("a", "inst-a", None);
+        none["gpu"] = gpu_json(serde_json::json!({ "mode": "none" }));
+        let parsed = parse_exchange(&file(vec![], vec![none])).unwrap();
+        let p = plan_with(&data, &parsed, &opts(None), &MemoryStore::default()).unwrap();
+        assert_eq!(p.removed_gpu, ["a"]);
+        assert!(warning_keys(&p.preview).is_empty());
+    }
+
+    #[test]
+    fn insecure_dcgm_exporters_are_skipped_when_forbidden() {
+        let mut v = inst_json("i", "Insecure DCGM", None);
+        v["gpu"] = gpu_json(serde_json::json!({ "mode": "none" }));
+        v["gpu"]["ca"] = serde_json::json!({ "mode": "insecure" });
+        let o = ImportOptions {
+            allow_insecure_tls: false,
+            ..opts(None)
+        };
+        let p = plan(&StoreData::default(), &file(vec![], vec![v]), &o);
+        assert_eq!(p.preview.skipped[0].name, "Insecure DCGM");
+    }
+
     #[test]
     fn insecure_history_source_is_skipped_when_forbidden() {
         let mut v = inst_json("i", "Insecure history", None);
````

<!-- block: patch t02-src-tauri-src-storage-profile-rs step=2.3 -->
````
diff --git a/src-tauri/src/storage/profile.rs b/src-tauri/src/storage/profile.rs
index 593f3fd..2c42831 100644
--- a/src-tauri/src/storage/profile.rs
+++ b/src-tauri/src/storage/profile.rs
@@ -1,7 +1,8 @@
 //! Профиль инсталляции инференса (спец. §5.1, §5.2, §5.3): окружение, серверы, источник истории
-//! метрик и их настройки. Ключи API серверов и секрет источника истории в профиле не хранятся, они
-//! лежат в keychain (`secrets::API_KEY`, `secrets::HISTORY_SECRET`). Проверки PEM, адресов и
-//! прокси — те же, что у профиля подключения Glafira for Kubernetes.
+//! метрик, компонент GPU и их настройки. Ключи API серверов, секрет источника истории и пароль
+//! экспортёров DCGM в профиле не хранятся, они лежат в keychain (`secrets::API_KEY`,
+//! `secrets::HISTORY_SECRET`, `secrets::GPU_SECRET`). Проверки PEM, адресов и прокси — те же, что у
+//! профиля подключения Glafira for Kubernetes.
 
 use serde::{Deserialize, Serialize};
 
@@ -97,11 +98,24 @@ pub struct ServerConfig {
     /// I1c §3). `None` — история сервера недоступна.
     #[serde(default, skip_serializing_if = "Option::is_none")]
     pub history_selector: Option<String>,
+    /// Карты GPU сервера (спека I1d §2); `None` — сервер к GPU не привязан.
+    #[serde(default, skip_serializing_if = "Option::is_none")]
+    pub gpus: Option<GpuBinding>,
     /// Поля из более новой версии Glafira: сохраняются при записи как есть.
     #[serde(flatten)]
     pub extra: serde_json::Map<String, serde_json::Value>,
 }
 
+/// Привязка сервера к картам (спека I1d §2): хост карт и номера; `indices = None` — все карты
+/// хоста. В I3 привязка станет частью связей компонентов (§5.7 общей спеки).
+#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
+#[serde(rename_all = "camelCase")]
+pub struct GpuBinding {
+    pub host: String,
+    #[serde(default, skip_serializing_if = "Option::is_none")]
+    pub indices: Option<Vec<u32>>,
+}
+
 /// Вход в источник истории (спека I1c §2). Токен и пароль — только в keychain
 /// (`secrets::HISTORY_SECRET`), в профиле — только вид входа и имя пользователя.
 #[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
@@ -133,6 +147,72 @@ pub struct HistorySource {
     pub extra: serde_json::Map<String, serde_json::Value>,
 }
 
+/// Вход в экспортёры DCGM (спека I1d §2): общий для всех экспортёров компонента. Пароль — только в
+/// keychain (`secrets::GPU_SECRET`).
+#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
+#[serde(tag = "mode", rename_all = "camelCase")]
+pub enum GpuAuth {
+    #[default]
+    None,
+    Basic {
+        username: String,
+    },
+}
+
+/// Экспортёр DCGM в режиме `direct`: адрес `/metrics` и селектор его серий в источнике истории.
+#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
+#[serde(rename_all = "camelCase")]
+pub struct GpuExporter {
+    /// Адрес метрик экспортёра: `http(s)://host:port/metrics` (без пути — `/metrics`).
+    pub url: String,
+    /// Селектор серий экспортёра в источнике истории (`{job="dcgm", instance="10.0.0.5:9400"}`);
+    /// `None` — окна истории его карт недоступны.
+    #[serde(default, skip_serializing_if = "Option::is_none")]
+    pub history_selector: Option<String>,
+    /// Поля из более новой версии Glafira: сохраняются при записи как есть.
+    #[serde(flatten)]
+    pub extra: serde_json::Map<String, serde_json::Value>,
+}
+
+/// Экспортёры DCGM, которые Glafira опрашивает сама (режим `direct`).
+#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
+#[serde(rename_all = "camelCase")]
+pub struct DirectGpu {
+    pub exporters: Vec<GpuExporter>,
+    #[serde(default)]
+    pub auth: GpuAuth,
+    #[serde(default = "default_ca")]
+    pub ca: CaConfig,
+    #[serde(default = "default_proxy")]
+    pub proxy: ProxyConfig,
+    /// Поля из более новой версии Glafira: сохраняются при записи как есть.
+    #[serde(flatten)]
+    pub extra: serde_json::Map<String, serde_json::Value>,
+}
+
+/// Серии DCGM из источника истории (режим `prometheus`): клиент и вход — источника истории.
+#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
+#[serde(rename_all = "camelCase")]
+pub struct PrometheusGpu {
+    /// Матчеры меток карт инсталляции (`{hostname=~"gpu-1|gpu-2"}`); `None` — все карты источника.
+    #[serde(default, skip_serializing_if = "Option::is_none")]
+    pub selector: Option<String>,
+    /// Поля из более новой версии Glafira: сохраняются при записи как есть.
+    #[serde(flatten)]
+    pub extra: serde_json::Map<String, serde_json::Value>,
+}
+
+/// Компонент GPU инсталляции (спека I1d §2): не больше одного; режим `ssh` появится в I3.
+#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
+#[serde(tag = "mode", rename_all = "camelCase")]
+pub enum GpuSource {
+    Direct(DirectGpu),
+    Prometheus(PrometheusGpu),
+}
+
+/// Сколько экспортёров DCGM может быть у компонента GPU.
+pub const MAX_EXPORTERS: usize = 16;
+
 #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
 #[serde(rename_all = "camelCase")]
 pub struct InstallationProfile {
@@ -146,6 +226,9 @@ pub struct InstallationProfile {
     /// Источник истории метрик (спека I1c §2); `None` — окна истории недоступны.
     #[serde(default, skip_serializing_if = "Option::is_none")]
     pub history: Option<HistorySource>,
+    /// Компонент GPU (спека I1d §2); `None` — карт у инсталляции нет.
+    #[serde(default, skip_serializing_if = "Option::is_none")]
+    pub gpu: Option<GpuSource>,
     #[serde(default = "default_request_timeout")]
     pub request_timeout_ms: u64,
     #[serde(default = "default_connect_timeout")]
@@ -167,8 +250,25 @@ impl InstallationProfile {
     }
 }
 
+/// Пустая строка селектора — его отсутствие.
+fn trimmed_selector(v: Option<String>) -> Option<String> {
+    v.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
+}
+
+/// Адрес экспортёра DCGM без пробелов и `/` на конце; без пути — `/metrics`. Неразбираемый адрес
+/// остаётся как есть: его отклонит проверка.
+fn normalize_exporter_url(raw: &str) -> String {
+    let url = raw.trim().trim_end_matches('/');
+    match url::Url::parse(url) {
+        Ok(u) if u.host_str().is_some() && u.path() == "/" => format!("{url}/metrics"),
+        _ => url.to_string(),
+    }
+}
+
 /// Приводит поля к каноническому виду до проверки: имена и адреса без пробелов по краям, адрес
 /// без `/` на конце, путь метрик — с `/` в начале, пустой селектор истории — его отсутствие.
+/// Номера карт привязки — по возрастанию без повторов, пустой список — все карты; без компонента
+/// GPU привязок нет.
 pub fn normalize_profile(p: &mut InstallationProfile) {
     p.name = p.name.trim().to_string();
     if let Some(h) = &mut p.history {
@@ -177,14 +277,37 @@ pub fn normalize_profile(p: &mut InstallationProfile) {
             *username = username.trim().to_string();
         }
     }
+    match &mut p.gpu {
+        Some(GpuSource::Direct(d)) => {
+            for e in &mut d.exporters {
+                e.url = normalize_exporter_url(&e.url);
+                e.history_selector = trimmed_selector(e.history_selector.take());
+            }
+            if let GpuAuth::Basic { username } = &mut d.auth {
+                *username = username.trim().to_string();
+            }
+        }
+        Some(GpuSource::Prometheus(g)) => g.selector = trimmed_selector(g.selector.take()),
+        None => {}
+    }
+    let has_gpu = p.gpu.is_some();
     for s in &mut p.servers {
         s.name = s.name.trim().to_string();
         s.base_url = s.base_url.trim().trim_end_matches('/').to_string();
-        s.history_selector = s
-            .history_selector
-            .take()
-            .map(|v| v.trim().to_string())
-            .filter(|v| !v.is_empty());
+        s.history_selector = trimmed_selector(s.history_selector.take());
+        if !has_gpu {
+            s.gpus = None;
+        }
+        if let Some(b) = &mut s.gpus {
+            b.host = b.host.trim().to_string();
+            if let Some(list) = &mut b.indices {
+                list.sort_unstable();
+                list.dedup();
+            }
+            if b.indices.as_ref().is_some_and(Vec::is_empty) {
+                b.indices = None;
+            }
+        }
         let path = s.metrics_path.trim();
         s.metrics_path = if path.is_empty() {
             default_metrics_path()
@@ -368,6 +491,15 @@ pub fn validate_server(s: &ServerConfig) -> AppResult<()> {
     if let Some(selector) = &s.history_selector {
         crate::history::targets::parse_selector(selector)?;
     }
+    if let Some(b) = &s.gpus
+        && b.host.is_empty()
+    {
+        return Err(invalid(ltext!(
+            "errors.gpu.badBinding",
+            "Choose the GPU host of server “{{server}}”",
+            server = s.name
+        )));
+    }
     validate_ca(&s.ca, &s.name)?;
     validate_proxy(&s.proxy)
 }
@@ -453,6 +585,90 @@ pub fn validate_history(h: &HistorySource) -> AppResult<()> {
     validate_proxy(&h.proxy)
 }
 
+/// Имя компонента GPU в текстах ошибок CA (`errors.installations.caPemInvalid`).
+pub const GPU_NAME: &str = "DCGM";
+
+/// Разобранный адрес экспортёра DCGM: `http(s)://`, хост, без имени и пароля, запроса и якоря.
+pub fn parse_exporter_url(e: &GpuExporter) -> AppResult<url::Url> {
+    let bad = || {
+        invalid(ltext!(
+            "errors.gpu.badUrl",
+            "Invalid DCGM exporter address “{{url}}”: use http://host:9400/metrics",
+            url = redact_url(&e.url)
+        ))
+    };
+    let url = url::Url::parse(e.url.trim()).map_err(|_| bad())?;
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
+/// Проверка компонента GPU: и при сохранении профиля, и при проверке из формы. Режиму
+/// `prometheus` нужен источник истории инсталляции.
+pub fn validate_gpu(g: &GpuSource, has_history: bool) -> AppResult<()> {
+    match g {
+        GpuSource::Direct(d) => {
+            if d.exporters.is_empty() {
+                return Err(invalid(ltext!(
+                    "errors.gpu.noExporters",
+                    "Add at least one DCGM exporter address"
+                )));
+            }
+            if d.exporters.len() > MAX_EXPORTERS {
+                return Err(invalid(ltext!(
+                    "errors.gpu.tooManyExporters",
+                    "The GPU component can have at most {{max}} DCGM exporters",
+                    max = MAX_EXPORTERS
+                )));
+            }
+            let mut seen = std::collections::HashSet::new();
+            for e in &d.exporters {
+                parse_exporter_url(e)?;
+                if !seen.insert(e.url.to_lowercase()) {
+                    return Err(invalid(ltext!(
+                        "errors.gpu.duplicateExporter",
+                        "The DCGM exporter “{{url}}” is listed twice",
+                        url = redact_url(&e.url)
+                    )));
+                }
+                if let Some(selector) = &e.history_selector {
+                    crate::history::targets::parse_selector(selector)?;
+                }
+            }
+            if let GpuAuth::Basic { username } = &d.auth
+                && (username.trim().is_empty() || username.contains(':'))
+            {
+                return Err(invalid(ltext!(
+                    "errors.gpu.badUsername",
+                    "Enter the user name of the DCGM exporters (without “:”)"
+                )));
+            }
+            validate_ca(&d.ca, GPU_NAME)?;
+            validate_proxy(&d.proxy)
+        }
+        GpuSource::Prometheus(g) => {
+            if !has_history {
+                return Err(invalid(ltext!(
+                    "errors.gpu.noHistorySource",
+                    "GPU metrics from the history source need a metrics history source: add it to the installation"
+                )));
+            }
+            if let Some(selector) = &g.selector {
+                crate::history::targets::parse_selector(selector)?;
+            }
+            Ok(())
+        }
+    }
+}
+
 pub fn validate_profile(p: &InstallationProfile) -> AppResult<()> {
     if p.id.trim().is_empty() {
         return Err(invalid(ltext!(
@@ -501,6 +717,9 @@ pub fn validate_profile(p: &InstallationProfile) -> AppResult<()> {
     if let Some(h) = &p.history {
         validate_history(h)?;
     }
+    if let Some(g) = &p.gpu {
+        validate_gpu(g, p.history.is_some())?;
+    }
     if p.request_timeout_ms < 1_000 || p.connect_timeout_ms < 1_000 {
         return Err(invalid(ltext!(
             "errors.installations.timeoutTooSmall",
@@ -521,6 +740,7 @@ pub fn sample_server(id: &str) -> ServerConfig {
         ca: CaConfig::System,
         proxy: ProxyConfig::System,
         history_selector: None,
+        gpus: None,
         extra: Default::default(),
     }
 }
@@ -536,6 +756,27 @@ pub fn sample_history() -> HistorySource {
     }
 }
 
+#[cfg(test)]
+pub fn sample_exporter(url: &str) -> GpuExporter {
+    GpuExporter {
+        url: url.into(),
+        history_selector: None,
+        extra: Default::default(),
+    }
+}
+
+/// Компонент GPU с одним экспортёром без входа.
+#[cfg(test)]
+pub fn sample_gpu() -> GpuSource {
+    GpuSource::Direct(DirectGpu {
+        exporters: vec![sample_exporter("http://gpu-1.dev.example.com:9400/metrics")],
+        auth: GpuAuth::None,
+        ca: CaConfig::System,
+        proxy: ProxyConfig::System,
+        extra: Default::default(),
+    })
+}
+
 #[cfg(test)]
 pub fn sample_profile(id: &str) -> InstallationProfile {
     InstallationProfile {
@@ -546,6 +787,7 @@ pub fn sample_profile(id: &str) -> InstallationProfile {
         read_only: false,
         servers: vec![sample_server("s1")],
         history: None,
+        gpu: None,
         request_timeout_ms: 30_000,
         connect_timeout_ms: 10_000,
         folder_id: None,
@@ -888,6 +1130,240 @@ mod tests {
         assert!(!s.contains("password") && !s.contains("token"), "{s}");
     }
 
+    #[test]
+    fn gpu_json_shape_and_defaults() {
+        let mut p = sample_profile("a");
+        p.gpu = Some(GpuSource::Direct(DirectGpu {
+            exporters: vec![GpuExporter {
+                history_selector: Some(r#"{job="dcgm"}"#.into()),
+                ..sample_exporter("http://gpu-1:9400/metrics")
+            }],
+            auth: GpuAuth::Basic {
+                username: "glafira".into(),
+            },
+            ca: CaConfig::System,
+            proxy: ProxyConfig::None,
+            extra: Default::default(),
+        }));
+        p.servers[0].gpus = Some(GpuBinding {
+            host: "gpu-1".into(),
+            indices: Some(vec![0, 1]),
+        });
+        let v = serde_json::to_value(&p).unwrap();
+        assert_eq!(
+            v["gpu"],
+            serde_json::json!({
+                "mode": "direct",
+                "exporters": [{ "url": "http://gpu-1:9400/metrics", "historySelector": r#"{job="dcgm"}"# }],
+                "auth": { "mode": "basic", "username": "glafira" },
+                "ca": { "mode": "system" },
+                "proxy": { "mode": "none" }
+            })
+        );
+        assert_eq!(
+            v["servers"][0]["gpus"],
+            serde_json::json!({ "host": "gpu-1", "indices": [0, 1] })
+        );
+        let prom: GpuSource =
+            serde_json::from_value(serde_json::json!({ "mode": "prometheus" })).unwrap();
+        assert_eq!(
+            prom,
+            GpuSource::Prometheus(PrometheusGpu {
+                selector: None,
+                extra: Default::default()
+            })
+        );
+        let direct: GpuSource = serde_json::from_value(serde_json::json!({
+            "mode": "direct", "exporters": [{ "url": "http://h:9400/metrics", "future": 1 }], "later": true
+        }))
+        .unwrap();
+        let GpuSource::Direct(d) = &direct else {
+            panic!("{direct:?}")
+        };
+        assert_eq!(d.auth, GpuAuth::None);
+        assert_eq!((&d.ca, &d.proxy), (&CaConfig::System, &ProxyConfig::System));
+        assert_eq!(
+            d.exporters[0].extra["future"], 1,
+            "поле экспортёра из новой версии"
+        );
+        assert_eq!(d.extra["later"], true, "поле компонента из новой версии");
+        assert!(!d.extra.contains_key("mode"), "{:?}", d.extra);
+        let back = serde_json::to_value(&direct).unwrap();
+        assert_eq!(
+            (back["mode"].clone(), back["later"].clone()),
+            ("direct".into(), true.into())
+        );
+        let s = serde_json::to_string(&p).unwrap();
+        assert!(!s.contains("password"), "{s}");
+    }
+
+    #[test]
+    fn gpu_is_normalized_and_bindings_need_the_component() {
+        let mut p = sample_profile("a");
+        p.gpu = Some(GpuSource::Direct(DirectGpu {
+            exporters: vec![
+                GpuExporter {
+                    history_selector: Some("  ".into()),
+                    ..sample_exporter(" http://gpu-1:9400/ ")
+                },
+                sample_exporter("https://gpu-2:9400/dcgm/metrics/"),
+            ],
+            auth: GpuAuth::Basic {
+                username: " glafira ".into(),
+            },
+            ..match sample_gpu() {
+                GpuSource::Direct(d) => d,
+                GpuSource::Prometheus(_) => unreachable!(),
+            }
+        }));
+        p.servers[0].gpus = Some(GpuBinding {
+            host: " gpu-1 ".into(),
+            indices: Some(vec![3, 1, 3]),
+        });
+        normalize_profile(&mut p);
+        let Some(GpuSource::Direct(d)) = &p.gpu else {
+            panic!()
+        };
+        assert_eq!(d.exporters[0].url, "http://gpu-1:9400/metrics");
+        assert_eq!(d.exporters[0].history_selector, None);
+        assert_eq!(d.exporters[1].url, "https://gpu-2:9400/dcgm/metrics");
+        assert_eq!(
+            d.auth,
+            GpuAuth::Basic {
+                username: "glafira".into()
+            }
+        );
+        assert_eq!(
+            p.servers[0].gpus,
+            Some(GpuBinding {
+                host: "gpu-1".into(),
+                indices: Some(vec![1, 3])
+            })
+        );
+        assert!(validate_profile(&p).is_ok());
+        p.servers[0].gpus.as_mut().unwrap().indices = Some(vec![]);
+        normalize_profile(&mut p);
+        assert_eq!(
+            p.servers[0].gpus.as_ref().unwrap().indices,
+            None,
+            "пустой список — все карты"
+        );
+        p.gpu = None;
+        normalize_profile(&mut p);
+        assert_eq!(p.servers[0].gpus, None, "без компонента GPU привязок нет");
+        p.gpu = Some(GpuSource::Prometheus(PrometheusGpu {
+            selector: Some("  ".into()),
+            extra: Default::default(),
+        }));
+        p.history = Some(sample_history());
+        normalize_profile(&mut p);
+        assert_eq!(
+            p.gpu,
+            Some(GpuSource::Prometheus(PrometheusGpu {
+                selector: None,
+                extra: Default::default()
+            }))
+        );
+    }
+
+    #[test]
+    fn gpu_is_validated() {
+        let direct = |change: &dyn Fn(&mut DirectGpu)| {
+            let mut p = sample_profile("a");
+            let GpuSource::Direct(mut d) = sample_gpu() else {
+                unreachable!()
+            };
+            change(&mut d);
+            p.gpu = Some(GpuSource::Direct(d));
+            p
+        };
+        assert!(validate_profile(&direct(&|_| {})).is_ok());
+        assert_invalid(&direct(&|d| d.exporters.clear()), "errors.gpu.noExporters");
+        assert_invalid(
+            &direct(&|d| {
+                d.exporters = (0..=MAX_EXPORTERS)
+                    .map(|i| sample_exporter(&format!("http://gpu-{i}:9400/metrics")))
+                    .collect()
+            }),
+            "errors.gpu.tooManyExporters",
+        );
+        assert_invalid(
+            &direct(&|d| {
+                d.exporters
+                    .push(sample_exporter("http://GPU-1.dev.example.com:9400/metrics"))
+            }),
+            "errors.gpu.duplicateExporter",
+        );
+        for url in [
+            "gpu-1:9400",
+            "ftp://gpu-1",
+            "http://gpu-1:9400/metrics?x=1",
+            "http://",
+        ] {
+            assert_invalid(
+                &direct(&|d| d.exporters[0].url = url.into()),
+                "errors.gpu.badUrl",
+            );
+        }
+        let e = assert_invalid(
+            &direct(&|d| d.exporters[0].url = "http://u:s3cr3t@gpu-1:9400/metrics".into()),
+            "errors.installations.credentialsInUrl",
+        );
+        assert!(!e.message.contains("s3cr3t"), "{}", e.message);
+        assert_invalid(
+            &direct(&|d| d.exporters[0].history_selector = Some("job=dcgm".into())),
+            "errors.history.selector",
+        );
+        for username in ["", " ", "a:b"] {
+            assert_invalid(
+                &direct(&|d| {
+                    d.auth = GpuAuth::Basic {
+                        username: username.into(),
+                    }
+                }),
+                "errors.gpu.badUsername",
+            );
+        }
+        assert_invalid(
+            &direct(&|d| {
+                d.ca = CaConfig::Pem {
+                    pem: cert_with_key(),
+                }
+            }),
+            "errors.installations.pemContainsKey",
+        );
+        assert_invalid(
+            &direct(&|d| {
+                d.proxy = ProxyConfig::Url {
+                    url: "socks://x".into(),
+                }
+            }),
+            "errors.installations.badProxyUrl",
+        );
+
+        let mut p = sample_profile("a");
+        p.gpu = Some(GpuSource::Prometheus(PrometheusGpu {
+            selector: Some(r#"{hostname=~"gpu-1|gpu-2"}"#.into()),
+            extra: Default::default(),
+        }));
+        assert_invalid(&p, "errors.gpu.noHistorySource");
+        p.history = Some(sample_history());
+        assert!(validate_profile(&p).is_ok());
+        p.gpu = Some(GpuSource::Prometheus(PrometheusGpu {
+            selector: Some(r#"{__name__="x"}"#.into()),
+            extra: Default::default(),
+        }));
+        assert_invalid(&p, "errors.history.selectorName");
+
+        let mut p = direct(&|_| {});
+        p.servers[0].gpus = Some(GpuBinding {
+            host: String::new(),
+            indices: None,
+        });
+        let e = assert_invalid(&p, "errors.gpu.badBinding");
+        assert_eq!(e.params["server"], "vllm-s1");
+    }
+
     #[test]
     fn history_is_normalized_and_validated() {
         let mut p = sample_profile("a");
````

<!-- block: locales t02-locales-t02 step=2.3 -->
````
{
  "en": {
    "errors": { "gpu": {
      "badBinding": "Choose the GPU host of server “{{server}}”",
      "badUrl": "Invalid DCGM exporter address “{{url}}”: use http://host:9400/metrics",
      "badUsername": "Enter the user name of the DCGM exporters (without “:”)",
      "duplicateExporter": "The DCGM exporter “{{url}}” is listed twice",
      "noExporters": "Add at least one DCGM exporter address",
      "noHistorySource": "GPU metrics from the history source need a metrics history source: add it to the installation",
      "tooManyExporters": "The GPU component can have at most {{max}} DCGM exporters"
    } },
    "messages": { "import": {
      "gpuChanged": "“{{name}}”: the addresses, proxy or certificate verification of the DCGM exporters change. The saved password will be sent with the new settings.",
      "gpuRemoved": "“{{name}}”: the GPU component is not in the file and will be removed together with the saved password of the DCGM exporters"
    } }
  },
  "ru": {
    "errors": { "gpu": {
      "badBinding": "Выберите хост GPU сервера «{{server}}»",
      "badUrl": "Неверный адрес экспортёра DCGM «{{url}}»: используйте http://хост:9400/metrics",
      "badUsername": "Укажите имя пользователя экспортёров DCGM (без «:»)",
      "duplicateExporter": "Экспортёр DCGM «{{url}}» указан дважды",
      "noExporters": "Добавьте хотя бы один адрес экспортёра DCGM",
      "noHistorySource": "Метрикам GPU из источника истории нужен источник истории метрик: добавьте его в инсталляцию",
      "tooManyExporters": "У компонента GPU может быть не больше {{max}} экспортёров DCGM"
    } },
    "messages": { "import": {
      "gpuChanged": "«{{name}}»: у экспортёров DCGM меняются адреса, прокси или проверка сертификата. Сохранённый пароль уйдёт с новыми настройками.",
      "gpuRemoved": "«{{name}}»: компонента GPU нет в файле — он будет удалён вместе с сохранённым паролем экспортёров DCGM"
    } }
  }
}
````

- [ ] **Шаг 2.4: Проверки.**

Run: `cd src-tauri && cargo test --lib -q`
Expected: `test result: ok. 303 passed`
Run: `cd src-tauri && cargo clippy --lib --tests -q -- -D warnings && cargo fmt --check`
Expected: без вывода
Run: `pnpm i18n:check`
Expected: `✓ словари en, ru согласованы, вызовов из Rust: 157`

- [ ] **Шаг 2.5: Коммит.**

<!-- block: shell t02-commit step=2.5 -->
````
git add src-tauri/src/commands/folders.rs \
  src-tauri/src/commands/installations.rs \
  src-tauri/src/history/client.rs \
  src-tauri/src/secrets.rs \
  src-tauri/src/state.rs \
  src-tauri/src/storage/exchange.rs \
  src-tauri/src/storage/profile.rs \
  src/locales/en.json \
  src/locales/ru.json
git commit -s -q -F - <<'EOF'
feat(installations): GPU component and server GPU binding in the profile

The installation gets an optional GPU component: DCGM exporters with a
shared basic login, CA and proxy, or DCGM series of the history source
narrowed by a selector. A server gets an optional GPU binding: host and
card indices. The exporter password lives in one keychain entry and
follows the history secret rules on save, delete, export and import;
insecure exporters are forbidden by allowInsecureTls = false.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
````

---

### Задача 3: Поля DCGM и снимок карт

Модуль `gpu` и в нём `fields.rs` и `snapshot.rs` (спека I1d §4): таблица полей карты с сериями-кандидатами
(активность SM без `PROF_SM_ACTIVE` — по graphics engine, два имени маски троттлинга, ошибки NVLink до Hopper и с
Hopper — суммой), разряды маски причин, группы счётчиков для подсказки «полнота набора»; карты из серий DCGM — хост по
меткам `hostname`/`Hostname` или адресу, номер, модель, UUID, драйвер, нагрузка Kubernetes, экземпляры MIG (серии с
`GPU_I_ID` полей карты не дают), всего памяти — суммой; хосты по порядку и не больше 64 карт. Снимки `/metrics` —
с мока задачи 1: шаг 3 пересоздаёт их командой, README снимков её повторяет.

**Files:**
- Create: `src-tauri/src/gpu/fields.rs`
- Create: `src-tauri/src/gpu/mod.rs`
- Create: `src-tauri/src/gpu/snapshot.rs`
- Create: `src-tauri/src/gpu/testdata/dcgm-4.8.4/4.1.txt`
- Create: `src-tauri/src/gpu/testdata/dcgm-4.8.4/README.md`
- Create: `src-tauri/src/gpu/testdata/dcgm-4.8.4/default.txt`
- Create: `src-tauri/src/gpu/testdata/dcgm-4.8.4/full.txt`
- Create: `src-tauri/src/gpu/testdata/dcgm-4.8.4/mig.txt`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Produces: `gpu::fields::{Field, Kind, Combine, FieldSpec, SPECS, SM_ACTIVE, PRESENCE, spec_of, all_names, SmSource,
  Group, REASONS, ALERT_REASONS, reasons}`; JSON `Field` — camelCase (`smActive`, `memUsed`); `gpu::snapshot::{GpuCard,
  GpuHost, Workload, MigInstance, MAX_GPUS, host_of, cards_from, missing, group}`; `cards_from(&[Sample], &dyn
  Fn(&Sample) -> String, Option<&str>) -> Vec<GpuCard>`; тестовые `gpu::snapshot::tests::{DEFAULT, FULL, OLD, MIG,
  cards}`.

- [ ] **Шаг 3.1: Тесты.** Тесты `gpu/fields.rs` и `gpu/snapshot.rs`; модуль `gpu` подключается в `lib.rs`.

<!-- block: file t03-src-tauri-src-gpu-fields-rs-tests step=3.1 path=src-tauri/src/gpu/fields.rs part=tests -->
````
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_series_belongs_to_one_field() {
        let mut seen = std::collections::HashSet::new();
        for name in all_names() {
            assert!(seen.insert(name), "{name} дважды");
            assert!(name.starts_with("DCGM_"), "{name}");
        }
        assert_eq!(
            spec_of("DCGM_FI_PROF_GR_ENGINE_ACTIVE").map(|(s, i)| (s.field, i)),
            Some((Field::SmActive, 1))
        );
        assert_eq!(
            spec_of(SM_ACTIVE).map(|(s, i)| (s.field, i)),
            Some((Field::SmActive, 0))
        );
        assert_eq!(
            spec_of("DCGM_FI_DEV_NVLINK_REPLAY_TOTAL").map(|(s, _)| s.combine),
            Some(Combine::Sum)
        );
        assert!(
            spec_of("DCGM_FI_DEV_ENC_UTIL").is_none(),
            "кодировщик экрану не нужен"
        );
        assert!(
            SPECS.iter().all(|s| s.field != Field::MemTotal),
            "всего памяти считается"
        );
    }

    #[test]
    fn throttle_mask_names_reasons_in_bit_order() {
        assert_eq!(reasons(0), Vec::<&str>::new());
        assert_eq!(reasons(0x48), ["HW_SLOWDOWN", "HW_THERMAL"]);
        assert_eq!(
            reasons(0x1 | 0x4 | 0x80 | 0x100),
            [
                "GPU_IDLE",
                "SW_POWER_CAP",
                "HW_POWER_BRAKE",
                "DISPLAY_CLOCKS"
            ]
        );
        assert_eq!(reasons(0x1000), Vec::<&str>::new(), "неизвестный разряд");
        assert!(
            ALERT_REASONS
                .iter()
                .all(|a| REASONS.iter().any(|(_, r)| r == a))
        );
    }

    #[test]
    fn json_names() {
        assert_eq!(serde_json::to_value(Field::SmActive).unwrap(), "smActive");
        assert_eq!(
            serde_json::to_value(Field::NvlinkErrors).unwrap(),
            "nvlinkErrors"
        );
        assert_eq!(
            serde_json::to_value(Group::NvlinkErrors).unwrap(),
            "nvlinkErrors"
        );
        assert_eq!(serde_json::to_value(SmSource::Gr).unwrap(), "gr");
    }
}
````

<!-- block: file t03-src-tauri-src-gpu-mod-rs step=3.1 path=src-tauri/src/gpu/mod.rs -->
````
//! GPU инсталляции из экспортёров DCGM (спека I1d): поля карт, снимок карт по хостам.

pub mod fields;
pub mod snapshot;
````

<!-- block: file t03-src-tauri-src-gpu-snapshot-rs-tests step=3.1 path=src-tauri/src/gpu/snapshot.rs part=tests -->
````
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::metrics::parse::parse;

    pub const DEFAULT: &str = include_str!("testdata/dcgm-4.8.4/default.txt");
    pub const FULL: &str = include_str!("testdata/dcgm-4.8.4/full.txt");
    pub const OLD: &str = include_str!("testdata/dcgm-4.8.4/4.1.txt");
    pub const MIG: &str = include_str!("testdata/dcgm-4.8.4/mig.txt");

    pub fn cards(text: &str, fallback: &str) -> Vec<GpuCard> {
        let f = fallback.to_string();
        cards_from(&parse(text).unwrap().samples, &move |_| f.clone(), None)
    }

    #[test]
    fn default_counter_set_gives_cards_with_fallback_sm_and_missing_groups() {
        let cards = cards(DEFAULT, "exporter-host");
        assert_eq!(cards.len(), 2);
        let c = &cards[0];
        assert_eq!(
            (c.host.as_str(), c.index),
            ("localhost", 0),
            "метка hostname, а не адрес"
        );
        assert_eq!(c.model.as_deref(), Some("NVIDIA H100 80GB HBM3"));
        assert_eq!(
            c.uuid.as_deref(),
            Some("GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00")
        );
        assert_eq!(c.pci_bus_id.as_deref(), Some("00000000:18:00.0"));
        assert_eq!(c.driver.as_deref(), Some("570.124.06"));
        assert_eq!(c.workload, None);
        assert_eq!(
            c.sm_source,
            Some(SmSource::Gr),
            "без SM_ACTIVE — graphics engine"
        );
        let v = &c.values;
        assert_eq!(v[&Field::MemTotal], 81559.0);
        assert!(
            v[&Field::SmActive] > 1.0 && v[&Field::SmActive] <= 100.0,
            "доля → проценты: {}",
            v[&Field::SmActive]
        );
        assert!(
            v.contains_key(&Field::Util)
                && v.contains_key(&Field::Power)
                && v.contains_key(&Field::PcieReplay)
        );
        for absent in [
            Field::Throttle,
            Field::EccDbe,
            Field::XidCount,
            Field::NvlinkErrors,
            Field::PowerViolation,
        ] {
            assert!(
                !v.contains_key(&absent),
                "{absent:?}: нет данных, а не ноль"
            );
        }
        assert_eq!(v[&Field::Xid], 0.0);
        assert_eq!(
            missing(&cards),
            [Group::Sm, Group::Ecc, Group::Throttle, Group::NvlinkErrors]
        );
    }

    #[test]
    fn full_counter_set_has_every_field_and_kubernetes_workload() {
        let cards = cards(FULL, "x");
        assert_eq!(cards.len(), 8);
        let c = &cards[0];
        assert_eq!(c.host, "gpu-node-1");
        assert_eq!(c.sm_source, Some(SmSource::Sm));
        assert_eq!(
            c.workload,
            Some(Workload {
                namespace: "llm".into(),
                pod: "vllm-0".into(),
                container: Some("vllm".into())
            })
        );
        let v = &c.values;
        assert_eq!(v[&Field::Throttle], 72.0, "HW_SLOWDOWN | HW_THERMAL");
        assert_eq!(v[&Field::Xid], 79.0);
        assert_eq!(v[&Field::XidCount], 1.0);
        assert_eq!(v[&Field::EccDbe], 2.0);
        assert_eq!(
            v[&Field::NvlinkErrors],
            12.0,
            "сумма трёх счётчиков Hopper на опросе 4"
        );
        assert_eq!(cards[1].values[&Field::Xid], 0.0);
        assert!(!cards[1].values.contains_key(&Field::XidCount));
        assert_eq!(missing(&cards), Vec::<Group>::new());
    }

    #[test]
    fn old_exporter_hostname_label_mig_and_fallback_host() {
        let old = cards(OLD, "x");
        assert_eq!(old[0].host, "gpu-old", "метка Hostname до 4.1");
        assert_eq!(old[0].model.as_deref(), Some("NVIDIA A100-SXM4-80GB"));
        let mig = cards(MIG, "x");
        assert_eq!(mig.len(), 2, "экземпляры MIG — не отдельные карты");
        let c = &mig[0];
        assert_eq!(
            c.mig,
            [
                MigInstance {
                    profile: "3g.40gb".into(),
                    instance: "1".into()
                },
                MigInstance {
                    profile: "3g.40gb".into(),
                    instance: "2".into()
                }
            ]
        );
        assert!(
            !c.values.contains_key(&Field::Util) && !c.values.contains_key(&Field::MemUsed),
            "{:?}",
            c.values
        );
        assert!(c.values.contains_key(&Field::Temp), "поля карты остаются");
        assert!(mig[1].mig.is_empty() && mig[1].values.contains_key(&Field::Util));
        let text = DEFAULT.replace("hostname=\"localhost\",", "");
        let fallback = cards(&text, "10.0.0.5");
        assert_eq!(
            fallback[0].host, "10.0.0.5",
            "без метки хоста — адрес экспортёра"
        );
    }

    #[test]
    fn group_sorts_hosts_and_cards_and_limits_the_count() {
        let mut all = cards(FULL, "x");
        all.extend(cards(DEFAULT, "x"));
        all.reverse();
        let (hosts, truncated) = group(all.clone(), MAX_GPUS);
        assert!(!truncated);
        assert_eq!(
            hosts
                .iter()
                .map(|h| (h.host.as_str(), h.cards.len()))
                .collect::<Vec<_>>(),
            [("gpu-node-1", 8), ("localhost", 2)]
        );
        assert_eq!(hosts[0].driver.as_deref(), Some("570.124.06"));
        assert_eq!(
            hosts[0].cards.iter().map(|c| c.index).collect::<Vec<_>>(),
            (0..8).collect::<Vec<_>>()
        );
        let (hosts, truncated) = group(all, 9);
        assert!(truncated);
        assert_eq!(hosts.iter().map(|h| h.cards.len()).sum::<usize>(), 9);
        assert_eq!(hosts[1].cards.len(), 1);
    }

    #[test]
    fn json_shape() {
        let (hosts, _) = group(cards(FULL, "x"), 1);
        let v = serde_json::to_value(&hosts[0]).unwrap();
        assert_eq!(v["host"], "gpu-node-1");
        assert_eq!(v["driver"], "570.124.06");
        let card = &v["cards"][0];
        assert_eq!(card["index"], 0);
        assert_eq!(card["smSource"], "sm");
        assert_eq!(card["workload"]["namespace"], "llm");
        assert_eq!(card["values"]["throttle"], 72.0);
        assert!(
            card.get("host").is_none()
                && card.get("mig").is_none()
                && card.get("exporter").is_none(),
            "{card}"
        );
    }
}
````

<!-- block: patch t03-src-tauri-src-lib-rs step=3.1 -->
````
diff --git a/src-tauri/src/lib.rs b/src-tauri/src/lib.rs
index 7808ed7..41ec6f1 100644
--- a/src-tauri/src/lib.rs
+++ b/src-tauri/src/lib.rs
@@ -3,6 +3,7 @@ pub mod i18n;
 pub mod commands;
 mod dock_name;
 pub mod error;
+pub mod gpu;
 pub mod history;
 pub mod logging;
 pub mod metrics;
````

- [ ] **Шаг 3.2: Увидеть падение.**

Run: `cd src-tauri && cargo test --lib -q gpu`
Expected: FAIL — не компилируется: тестам не хватает типов и функций, которые появятся в шаге 3

- [ ] **Шаг 3.3: Реализация.** Таблица полей, снимок карт, снимки `/metrics` с мока (команда) и их README.

<!-- block: shell t03-prepare step=3.3 -->
````
mkdir -p src-tauri/src/gpu/testdata/dcgm-4.8.4
node --input-type=module -e '
import { dcgmMetrics, parseArgs } from "./scripts/mock-server.mjs";
import { writeFileSync } from "node:fs";
const kinds = { dcgm: "default", "dcgm-full": "full", "dcgm-4.1": "4.1", "dcgm-mig": "mig" };
const make = (file, args, poll) => {
  const o = parseArgs(args);
  writeFileSync(`src-tauri/src/gpu/testdata/dcgm-4.8.4/${file}`, dcgmMetrics(kinds[o.kind], o.dcgm, poll));
};
const full = ["--kind", "dcgm-full", "--gpus", "8", "--hostname", "gpu-node-1", "--pod", "llm/vllm-0", "--throttle", "--xid", "79", "--ecc"];
make("default.txt", ["--kind", "dcgm", "--gpus", "2"], 0);
make("full.txt", full, 4);
make("4.1.txt", ["--kind", "dcgm-4.1", "--gpus", "2", "--hostname", "gpu-old"], 0);
make("mig.txt", ["--kind", "dcgm-mig", "--gpus", "2", "--hostname", "gpu-mig"], 0);
'
````

<!-- block: file t03-src-tauri-src-gpu-fields-rs step=3.3 path=src-tauri/src/gpu/fields.rs part=impl -->
````
//! Канонические поля карты GPU (спека I1d §4): имена серий DCGM-кандидатов по порядку, вид серии
//! и единица. Отсутствующая серия — «нет данных», а не ноль. Набор счётчиков dcgm-exporter по
//! умолчанию не собирает активность SM, ECC и причины троттлинга, поэтому у полей есть запасные
//! серии, а группы без единой серии называет шаг проверки «полнота набора».

use serde::Serialize;

/// Поле карты. JSON — camelCase (`smActive`, `memUsed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Field {
    /// Утилизация, %.
    Util,
    /// Активность SM, % (без `PROF_SM_ACTIVE` — активность graphics engine, `SmSource::Gr`).
    SmActive,
    TensorActive,
    DramActive,
    MemCopyUtil,
    /// Память, МиБ.
    MemUsed,
    MemFree,
    MemReserved,
    /// Всего памяти: занято + свободно + зарезервировано, МиБ (считается, серии нет).
    MemTotal,
    /// Температура карты и памяти, °C.
    Temp,
    MemTemp,
    /// Мощность, Вт.
    Power,
    /// Такты SM и памяти, МГц.
    SmClock,
    MemClock,
    /// Битовая маска причин снижения тактов (`CLOCKS_EVENT_REASONS`).
    Throttle,
    /// Время троттлинга по мощности и температуре, нс (счётчики).
    PowerViolation,
    ThermalViolation,
    /// Код последней XID-ошибки (0 — не было) и число XID за окно экспортёра.
    Xid,
    XidCount,
    /// Ошибки ECC (volatile) — некорректируемые и корректируемые (счётчики).
    EccDbe,
    EccSbe,
    /// Переназначенные строки памяти и сбой переназначения.
    RemapUncorrectable,
    RemapCorrectable,
    RemapFailure,
    /// Повторы PCIe (счётчик) и трафик PCIe, Б/с.
    PcieReplay,
    PcieTx,
    PcieRx,
    /// Трафик NVLink, МБ/с, и ошибки NVLink (сумма счётчиков).
    NvlinkBandwidth,
    NvlinkErrors,
}

/// Вид серии: датчик, доля (на выходе — проценты), битовая маска, счётчик.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Gauge,
    Ratio,
    Mask,
    Counter,
}

/// Как брать значение из кандидатов: первый найденный или сумма всех найденных.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combine {
    First,
    Sum,
}

#[derive(Debug, Clone, Copy)]
pub struct FieldSpec {
    pub field: Field,
    /// Полные имена серий по порядку предпочтения.
    pub names: &'static [&'static str],
    pub kind: Kind,
    pub combine: Combine,
}

const fn spec(field: Field, names: &'static [&'static str], kind: Kind) -> FieldSpec {
    FieldSpec {
        field,
        names,
        kind,
        combine: Combine::First,
    }
}

/// Серия активности SM; без неё `SmActive` берётся из активности graphics engine.
pub const SM_ACTIVE: &str = "DCGM_FI_PROF_SM_ACTIVE";
/// Серия, по которой узнаётся экспортёр DCGM и считаются карты: она есть в наборе по умолчанию и
/// в режиме MIG, где утилизации карты нет.
pub const PRESENCE: &str = "DCGM_FI_DEV_FB_USED";

/// Таблица полей (спека I1d §4).
pub const SPECS: &[FieldSpec] = &[
    spec(Field::Util, &["DCGM_FI_DEV_GPU_UTIL"], Kind::Gauge),
    spec(
        Field::SmActive,
        &[SM_ACTIVE, "DCGM_FI_PROF_GR_ENGINE_ACTIVE"],
        Kind::Ratio,
    ),
    spec(
        Field::TensorActive,
        &["DCGM_FI_PROF_PIPE_TENSOR_ACTIVE"],
        Kind::Ratio,
    ),
    spec(
        Field::DramActive,
        &["DCGM_FI_PROF_DRAM_ACTIVE"],
        Kind::Ratio,
    ),
    spec(
        Field::MemCopyUtil,
        &["DCGM_FI_DEV_MEM_COPY_UTIL"],
        Kind::Gauge,
    ),
    spec(Field::MemUsed, &[PRESENCE], Kind::Gauge),
    spec(Field::MemFree, &["DCGM_FI_DEV_FB_FREE"], Kind::Gauge),
    spec(
        Field::MemReserved,
        &["DCGM_FI_DEV_FB_RESERVED"],
        Kind::Gauge,
    ),
    spec(Field::Temp, &["DCGM_FI_DEV_GPU_TEMP"], Kind::Gauge),
    spec(Field::MemTemp, &["DCGM_FI_DEV_MEMORY_TEMP"], Kind::Gauge),
    spec(Field::Power, &["DCGM_FI_DEV_POWER_USAGE"], Kind::Gauge),
    spec(Field::SmClock, &["DCGM_FI_DEV_SM_CLOCK"], Kind::Gauge),
    spec(Field::MemClock, &["DCGM_FI_DEV_MEM_CLOCK"], Kind::Gauge),
    spec(
        Field::Throttle,
        &[
            "DCGM_FI_DEV_CLOCKS_EVENT_REASONS",
            "DCGM_FI_DEV_CLOCK_THROTTLE_REASONS",
        ],
        Kind::Mask,
    ),
    spec(
        Field::PowerViolation,
        &["DCGM_FI_DEV_POWER_VIOLATION"],
        Kind::Counter,
    ),
    spec(
        Field::ThermalViolation,
        &["DCGM_FI_DEV_THERMAL_VIOLATION"],
        Kind::Counter,
    ),
    spec(Field::Xid, &["DCGM_FI_DEV_XID_ERRORS"], Kind::Gauge),
    FieldSpec {
        field: Field::XidCount,
        names: &["DCGM_EXP_XID_ERRORS_COUNT"],
        kind: Kind::Gauge,
        combine: Combine::Sum,
    },
    spec(
        Field::EccDbe,
        &["DCGM_FI_DEV_ECC_DBE_VOL_TOTAL"],
        Kind::Counter,
    ),
    spec(
        Field::EccSbe,
        &["DCGM_FI_DEV_ECC_SBE_VOL_TOTAL"],
        Kind::Counter,
    ),
    spec(
        Field::RemapUncorrectable,
        &["DCGM_FI_DEV_UNCORRECTABLE_REMAPPED_ROWS"],
        Kind::Gauge,
    ),
    spec(
        Field::RemapCorrectable,
        &["DCGM_FI_DEV_CORRECTABLE_REMAPPED_ROWS"],
        Kind::Gauge,
    ),
    spec(
        Field::RemapFailure,
        &["DCGM_FI_DEV_ROW_REMAP_FAILURE"],
        Kind::Gauge,
    ),
    spec(
        Field::PcieReplay,
        &["DCGM_FI_DEV_PCIE_REPLAY_COUNTER"],
        Kind::Counter,
    ),
    spec(Field::PcieTx, &["DCGM_FI_PROF_PCIE_TX_BYTES"], Kind::Gauge),
    spec(Field::PcieRx, &["DCGM_FI_PROF_PCIE_RX_BYTES"], Kind::Gauge),
    spec(
        Field::NvlinkBandwidth,
        &["DCGM_FI_DEV_NVLINK_BANDWIDTH_TOTAL"],
        Kind::Gauge,
    ),
    FieldSpec {
        field: Field::NvlinkErrors,
        names: &[
            // До Hopper.
            "DCGM_FI_DEV_NVLINK_CRC_FLIT_ERROR_TOTAL",
            "DCGM_FI_DEV_NVLINK_CRC_DATA_ERROR_TOTAL",
            "DCGM_FI_DEV_NVLINK_REPLAY_ERROR_TOTAL",
            "DCGM_FI_DEV_NVLINK_RECOVERY_ERROR_TOTAL",
            // Hopper.
            "DCGM_FI_DEV_NVLINK_CRC_ERROR_TOTAL",
            "DCGM_FI_DEV_NVLINK_RECOVERY_TOTAL",
            "DCGM_FI_DEV_NVLINK_REPLAY_TOTAL",
        ],
        kind: Kind::Counter,
        combine: Combine::Sum,
    },
];

/// Поле, которому принадлежит серия `name`, и номер кандидата.
pub fn spec_of(name: &str) -> Option<(&'static FieldSpec, usize)> {
    SPECS
        .iter()
        .find_map(|s| s.names.iter().position(|n| *n == name).map(|i| (s, i)))
}

/// Все имена серий таблицы — для мгновенного запроса к источнику истории.
pub fn all_names() -> impl Iterator<Item = &'static str> {
    SPECS.iter().flat_map(|s| s.names.iter().copied())
}

/// Откуда взята активность SM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SmSource {
    /// `DCGM_FI_PROF_SM_ACTIVE`.
    Sm,
    /// `DCGM_FI_PROF_GR_ENGINE_ACTIVE` — запасная серия.
    Gr,
}

/// Группа счётчиков, которой нет ни у одной карты (шаг проверки «полнота набора»).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Group {
    Sm,
    Ecc,
    Throttle,
    NvlinkErrors,
}

/// Разряды маски причин снижения тактов (`DCGM_CLOCKS_EVENT_REASON_*`).
pub const REASONS: &[(u64, &str)] = &[
    (0x1, "GPU_IDLE"),
    (0x2, "CLOCKS_SETTING"),
    (0x4, "SW_POWER_CAP"),
    (0x8, "HW_SLOWDOWN"),
    (0x10, "SYNC_BOOST"),
    (0x20, "SW_THERMAL"),
    (0x40, "HW_THERMAL"),
    (0x80, "HW_POWER_BRAKE"),
    (0x100, "DISPLAY_CLOCKS"),
];

/// Причины, которые отмечаются значком «троттлинг»; остальные — только в подсказке.
pub const ALERT_REASONS: &[&str] = &[
    "SW_POWER_CAP",
    "HW_SLOWDOWN",
    "SW_THERMAL",
    "HW_THERMAL",
    "HW_POWER_BRAKE",
];

/// Причины из маски по порядку разрядов; неизвестные разряды не называются.
pub fn reasons(mask: u64) -> Vec<&'static str> {
    REASONS
        .iter()
        .filter(|(bit, _)| mask & bit != 0)
        .map(|(_, name)| *name)
        .collect()
}
````

<!-- block: file t03-src-tauri-src-gpu-snapshot-rs step=3.3 path=src-tauri/src/gpu/snapshot.rs part=impl -->
````
//! Карты GPU из серий DCGM (спека I1d §2, §4): хост, номер, метки карты и значения полей.
//! Серии приходят из `/metrics` экспортёра или из мгновенного запроса к источнику истории — в обоих
//! случаях это `metrics::parse::Sample`.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::gpu::fields::{Combine, Field, Group, Kind, SM_ACTIVE, SmSource, spec_of};
use crate::metrics::parse::Sample;

/// Сколько карт может быть у инсталляции (спека I1d §1).
pub const MAX_GPUS: usize = 64;

/// Нагрузка на карте по меткам Kubernetes экспортёра.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Workload {
    pub namespace: String,
    pub pod: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
}

/// Экземпляр MIG карты: профиль и номер.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigInstance {
    pub profile: String,
    pub instance: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuCard {
    /// Хост карты; в JSON — у группы хоста.
    #[serde(skip)]
    pub host: String,
    /// Версия драйвера; в JSON — у группы хоста.
    #[serde(skip)]
    pub driver: Option<String>,
    pub index: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pci_bus_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workload: Option<Workload>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mig: Vec<MigInstance>,
    /// Адрес экспортёра (режим `direct`): подпись у второй карты с тем же хостом и номером.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exporter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sm_source: Option<SmSource>,
    pub values: BTreeMap<Field, f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuHost {
    pub host: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,
    pub cards: Vec<GpuCard>,
}

/// Метка непуста.
fn label<'a>(s: &'a Sample, name: &str) -> Option<&'a str> {
    s.label(name).filter(|v| !v.is_empty())
}

/// Хост карты: метка `hostname` (dcgm-exporter 4.8), `Hostname` (до 4.1), иначе `fallback`.
pub fn host_of(s: &Sample, fallback: &dyn Fn(&Sample) -> String) -> String {
    label(s, "hostname")
        .or_else(|| label(s, "Hostname"))
        .map(str::to_string)
        .unwrap_or_else(|| fallback(s))
}

/// Карты из серий DCGM. `fallback` — хост серии без метки хоста (адрес экспортёра, `instance`);
/// `exporter` — адрес экспортёра для подписи карты. Серии без номера карты и не из таблицы полей
/// пропускаются; серии экземпляров MIG (`GPU_I_ID`) только перечисляют экземпляры.
pub fn cards_from(
    samples: &[Sample],
    fallback: &dyn Fn(&Sample) -> String,
    exporter: Option<&str>,
) -> Vec<GpuCard> {
    // (хост, номер) → карта и номер кандидата, которым взято поле First.
    let mut cards: BTreeMap<(String, u32), (GpuCard, BTreeMap<Field, usize>)> = BTreeMap::new();
    for s in samples {
        if !s.name.starts_with("DCGM_") {
            continue;
        }
        let Some(index) = s.label("gpu").and_then(|g| g.parse::<u32>().ok()) else {
            continue;
        };
        let host = host_of(s, fallback);
        let (card, taken) = cards.entry((host.clone(), index)).or_insert_with(|| {
            let card = GpuCard {
                host,
                driver: None,
                index,
                uuid: None,
                model: None,
                pci_bus_id: None,
                workload: None,
                mig: Vec::new(),
                exporter: exporter.map(str::to_string),
                sm_source: None,
                values: BTreeMap::new(),
            };
            (card, BTreeMap::new())
        });
        fill_labels(card, s);
        if let Some(instance) = label(s, "GPU_I_ID") {
            let m = MigInstance {
                profile: label(s, "GPU_I_PROFILE").unwrap_or_default().to_string(),
                instance: instance.to_string(),
            };
            if !card.mig.contains(&m) {
                card.mig.push(m);
            }
            continue;
        }
        let Some((spec, rank)) = spec_of(&s.name) else {
            continue;
        };
        if !s.value.is_finite() {
            continue;
        }
        let v = if spec.kind == Kind::Ratio {
            s.value * 100.0
        } else {
            s.value
        };
        match spec.combine {
            Combine::Sum => *card.values.entry(spec.field).or_insert(0.0) += v,
            Combine::First => {
                if taken.get(&spec.field).is_none_or(|r| rank < *r) {
                    taken.insert(spec.field, rank);
                    card.values.insert(spec.field, v);
                    if spec.field == Field::SmActive {
                        card.sm_source = Some(if s.name == SM_ACTIVE {
                            SmSource::Sm
                        } else {
                            SmSource::Gr
                        });
                    }
                }
            }
        }
    }
    cards
        .into_values()
        .map(|(mut card, _)| {
            let v = &card.values;
            if let (Some(used), Some(free)) = (v.get(&Field::MemUsed), v.get(&Field::MemFree)) {
                let total = used + free + v.get(&Field::MemReserved).copied().unwrap_or(0.0);
                card.values.insert(Field::MemTotal, total);
            }
            card.mig.sort();
            card
        })
        .collect()
}

/// Метки карты из первой серии, где они есть.
fn fill_labels(card: &mut GpuCard, s: &Sample) {
    let set = |slot: &mut Option<String>, v: Option<&str>| {
        if slot.is_none() {
            *slot = v.map(str::to_string);
        }
    };
    set(
        &mut card.uuid,
        label(s, "UUID").or_else(|| label(s, "uuid")),
    );
    set(&mut card.model, label(s, "modelName"));
    set(&mut card.pci_bus_id, label(s, "pci_bus_id"));
    set(&mut card.driver, label(s, "DCGM_FI_DRIVER_VERSION"));
    if card.workload.is_none()
        && let (Some(namespace), Some(pod)) = (label(s, "namespace"), label(s, "pod"))
    {
        card.workload = Some(Workload {
            namespace: namespace.to_string(),
            pod: pod.to_string(),
            container: label(s, "container").map(str::to_string),
        });
    }
}

/// Группы счётчиков, которых нет ни у одной карты. Ошибки NVLink без трафика NVLink не нужны:
/// у карт без NVLink их и не бывает.
pub fn missing(cards: &[GpuCard]) -> Vec<Group> {
    let any = |f: &dyn Fn(&GpuCard) -> bool| cards.iter().any(f);
    let has = |field: Field| move |c: &GpuCard| c.values.contains_key(&field);
    let mut out = Vec::new();
    if !any(&|c| c.sm_source == Some(SmSource::Sm)) {
        out.push(Group::Sm);
    }
    if !any(&has(Field::EccDbe)) {
        out.push(Group::Ecc);
    }
    if !any(&|c| {
        [
            Field::Throttle,
            Field::PowerViolation,
            Field::ThermalViolation,
        ]
        .iter()
        .any(|f| c.values.contains_key(f))
    }) {
        out.push(Group::Throttle);
    }
    if !any(&has(Field::NvlinkErrors))
        && any(&|c| {
            c.values
                .get(&Field::NvlinkBandwidth)
                .is_some_and(|v| *v > 0.0)
        })
    {
        out.push(Group::NvlinkErrors);
    }
    out
}

/// Карты по хостам: хосты и карты по порядку, не больше `limit` карт (по хосту и номеру); второй
/// результат — карт было больше.
pub fn group(mut cards: Vec<GpuCard>, limit: usize) -> (Vec<GpuHost>, bool) {
    cards.sort_by(|a, b| (&a.host, a.index, &a.exporter).cmp(&(&b.host, b.index, &b.exporter)));
    let truncated = cards.len() > limit;
    cards.truncate(limit);
    let mut hosts: Vec<GpuHost> = Vec::new();
    for card in cards {
        match hosts.last_mut() {
            Some(h) if h.host == card.host => {
                if h.driver.is_none() {
                    h.driver.clone_from(&card.driver);
                }
                h.cards.push(card);
            }
            _ => hosts.push(GpuHost {
                host: card.host.clone(),
                driver: card.driver.clone(),
                cards: vec![card],
            }),
        }
    }
    (hosts, truncated)
}
````

<!-- block: file t03-src-tauri-src-gpu-testdata-dcgm-4-8-4-readme-md step=3.3 path=src-tauri/src/gpu/testdata/dcgm-4.8.4/README.md -->
````
# Снимки `/metrics` экспортёра DCGM

Сняты с мок-сервера (`scripts/mock-server.mjs`, виды `dcgm*`), а не с карты: стенда с GPU у разработки нет. Формат —
dcgm-exporter 4.8.4, собран по его исходникам (`etc/default-counters.csv`, `internal/pkg/rendermetrics/render_metrics.go`):
метки карты `gpu`, `UUID`, `pci_bus_id`, `device`, `modelName`, хост — `hostname` (у 4.1 — `Hostname`), версия драйвера —
меткой `DCGM_FI_DRIVER_VERSION`, у MIG — `GPU_I_PROFILE` и `GPU_I_ID`, с `--pod` — `container`, `namespace`, `pod`.

| Файл | Вид мока и параметры | Опрос |
|---|---|---|
| `default.txt` | `dcgm --gpus 2` (набор по умолчанию: без активности SM, ECC и причин троттлинга) | 0 |
| `full.txt` | `dcgm-full --gpus 8 --hostname gpu-node-1 --pod llm/vllm-0 --throttle --xid 79 --ecc` | 4 |
| `4.1.txt` | `dcgm-4.1 --gpus 2 --hostname gpu-old` (метка `Hostname`, A100) | 0 |
| `mig.txt` | `dcgm-mig --gpus 2 --hostname gpu-mig` (карта 0 — два экземпляра MIG) | 0 |

Пересоздать (из корня репозитория):

```bash
node --input-type=module -e '
import { dcgmMetrics, parseArgs } from "./scripts/mock-server.mjs";
import { writeFileSync } from "node:fs";
const kinds = { dcgm: "default", "dcgm-full": "full", "dcgm-4.1": "4.1", "dcgm-mig": "mig" };
const make = (file, args, poll) => {
  const o = parseArgs(args);
  writeFileSync(`src-tauri/src/gpu/testdata/dcgm-4.8.4/${file}`, dcgmMetrics(kinds[o.kind], o.dcgm, poll));
};
const full = ["--kind", "dcgm-full", "--gpus", "8", "--hostname", "gpu-node-1", "--pod", "llm/vllm-0", "--throttle", "--xid", "79", "--ecc"];
make("default.txt", ["--kind", "dcgm", "--gpus", "2"], 0);
make("full.txt", full, 4);
make("4.1.txt", ["--kind", "dcgm-4.1", "--gpus", "2", "--hostname", "gpu-old"], 0);
make("mig.txt", ["--kind", "dcgm-mig", "--gpus", "2", "--hostname", "gpu-mig"], 0);
'
```
````

- [ ] **Шаг 3.4: Проверки.**

Run: `cd src-tauri && cargo test --lib -q`
Expected: `test result: ok. 311 passed`
Run: `cd src-tauri && cargo clippy --lib --tests -q -- -D warnings && cargo fmt --check`
Expected: без вывода

- [ ] **Шаг 3.5: Коммит.**

<!-- block: shell t03-commit step=3.5 -->
````
git add src-tauri/src/gpu/fields.rs \
  src-tauri/src/gpu/mod.rs \
  src-tauri/src/gpu/snapshot.rs \
  src-tauri/src/gpu/testdata/dcgm-4.8.4/4.1.txt \
  src-tauri/src/gpu/testdata/dcgm-4.8.4/README.md \
  src-tauri/src/gpu/testdata/dcgm-4.8.4/default.txt \
  src-tauri/src/gpu/testdata/dcgm-4.8.4/full.txt \
  src-tauri/src/gpu/testdata/dcgm-4.8.4/mig.txt \
  src-tauri/src/lib.rs
git commit -s -q -F - <<'EOF'
feat(gpu): DCGM fields and card snapshots

The gpu module maps DCGM series to card fields with fallbacks (graphics
engine activity without SM activity, two throttle mask names, NVLink
error counters before and since Hopper), reads card labels, hosts from
hostname or Hostname, MIG instances and Kubernetes workloads, names
counter groups no card has, and groups at most 64 cards by host.
Snapshots come from the DCGM mock.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
````

---

### Задача 4: Автоподбор привязок, селектора карт и целей экспортёров

`gpu/bind.rs` (спека I1d §3): совпадение хостов (без учёта регистра, короткое имя узла совпадает с полным,
адреса IP — только целиком, `localhost` = `127.0.0.1` = `::1`), привязка сервера без привязки к хосту его адреса,
сверка сохранённой привязки с найденными хостами и номерами, селектор карт по хостам серверов меткой, которой
подписаны найденные карты (`hostname`, `Hostname` или `instance` с любым портом; значения экранируются для RE2),
цель экспортёра в истории по `host:port` его адреса.

**Files:**
- Create: `src-tauri/src/gpu/bind.rs`
- Modify: `src-tauri/src/gpu/mod.rs`

**Interfaces:**
- Consumes: `history::targets::{escape, instance_matches}`, `storage::profile::{GpuBinding, parse_base_url}`.
- Produces: `gpu::bind::{canonical_host, same_host, server_host, matching_host, suggest_binding, BindingState,
  binding_state, bound_indices, HostLabel, FoundCard { host, label, index }, regex_escape, hosts_selector,
  suggest_selector, DcgmTarget { job, instance }, matching_exporter_targets}`; `DcgmTarget::selector()` →
  `{job="…", instance="…"}`.

- [ ] **Шаг 4.1: Тесты.** Тесты `gpu/bind.rs`; модуль подключается в `gpu/mod.rs`.

<!-- block: file t04-src-tauri-src-gpu-bind-rs-tests step=4.1 path=src-tauri/src/gpu/bind.rs part=tests -->
````
#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::profile::sample_server;

    fn server(url: &str) -> ServerConfig {
        ServerConfig {
            base_url: url.into(),
            ..sample_server("s1")
        }
    }

    fn found(host: &str, label: HostLabel, index: u32) -> FoundCard {
        FoundCard {
            host: host.into(),
            label,
            index,
        }
    }

    #[test]
    fn hosts_compare_by_name_short_name_and_loopback() {
        assert!(same_host("GPU-Node-1", "gpu-node-1"));
        assert!(same_host("gpu-node-1", "gpu-node-1.corp.local"));
        assert!(same_host("gpu-node-1.corp.local", "gpu-node-1"));
        assert!(
            !same_host("gpu-node-1.a.local", "gpu-node-1.b.local"),
            "два полных имени — целиком"
        );
        assert!(!same_host("gpu-node-10", "gpu-node-1"));
        assert!(same_host("localhost", "127.0.0.1"));
        assert!(same_host("[::1]", "localhost"));
        assert!(!same_host("10.0.0.5", "10.0.0.50"));
        assert!(!same_host("10.0.0.5", "10"), "адрес IP не короткое имя");
    }

    #[test]
    fn server_binding_is_suggested_by_host() {
        let hosts = vec!["gpu-node-1".to_string(), "localhost".to_string()];
        assert_eq!(
            suggest_binding(&server("http://gpu-node-1.corp.local:8000"), &hosts),
            Some(GpuBinding {
                host: "gpu-node-1".into(),
                indices: None
            })
        );
        assert_eq!(
            suggest_binding(&server("http://127.0.0.1:8000/v1"), &hosts).map(|b| b.host),
            Some("localhost".into())
        );
        assert_eq!(
            suggest_binding(&server("http://lb.corp:8000"), &hosts),
            None,
            "балансировщик"
        );
        let mut bound = server("http://gpu-node-1:8000");
        bound.gpus = Some(GpuBinding {
            host: "other".into(),
            indices: Some(vec![1]),
        });
        assert_eq!(
            suggest_binding(&bound, &hosts),
            None,
            "сохранённая привязка не меняется"
        );
    }

    #[test]
    fn binding_is_checked_against_the_cards() {
        let cards = vec![("gpu-node-1".to_string(), vec![0, 1, 2, 3])];
        let b = |host: &str, indices: Option<Vec<u32>>| GpuBinding {
            host: host.into(),
            indices,
        };
        assert_eq!(
            binding_state(&b("GPU-NODE-1", None), &cards),
            BindingState::Ok
        );
        assert_eq!(
            binding_state(&b("gpu-node-1", Some(vec![1, 3])), &cards),
            BindingState::Ok
        );
        assert_eq!(
            binding_state(&b("gpu-node-1", Some(vec![1, 4, 5])), &cards),
            BindingState::NoIndices(vec![4, 5])
        );
        assert_eq!(
            binding_state(&b("gpu-node-2", None), &cards),
            BindingState::NoHost
        );
        assert_eq!(bound_indices(&b("gpu-node-1", None), &[0, 1, 2]), [0, 1, 2]);
        assert_eq!(
            bound_indices(&b("gpu-node-1", Some(vec![2, 7])), &[0, 1, 2]),
            [2]
        );
    }

    #[test]
    fn selector_is_built_from_server_hosts_with_the_found_label() {
        let servers = vec![
            server("http://gpu-node-2:8000"),
            server("http://gpu-node-1.corp:8000"),
            server("http://lb:8000"),
        ];
        let cards = vec![
            found("gpu-node-1", HostLabel::Lower, 0),
            found("gpu-node-2", HostLabel::Lower, 0),
            found("gpu-node-3", HostLabel::Lower, 0),
        ];
        assert_eq!(
            suggest_selector(&servers, &cards).as_deref(),
            Some(r#"{hostname=~"gpu-node-1|gpu-node-2"}"#)
        );
        let old = vec![found("gpu-node-1", HostLabel::Upper, 0)];
        assert_eq!(
            suggest_selector(&servers, &old).as_deref(),
            Some(r#"{Hostname=~"gpu-node-1"}"#)
        );
        let by_instance = vec![found("10.0.0.5", HostLabel::Instance, 0)];
        assert_eq!(
            suggest_selector(&[server("http://10.0.0.5:8000")], &by_instance).as_deref(),
            Some(r#"{instance=~"(10\\.0\\.0\\.5)(:[0-9]+)?"}"#)
        );
        assert_eq!(suggest_selector(&[server("http://lb:8000")], &cards), None);
        assert_eq!(regex_escape("a.b+c(d)|e"), r"a\.b\+c\(d\)\|e");
        let mixed = vec![
            found("h", HostLabel::Upper, 0),
            found("h", HostLabel::Lower, 1),
        ];
        assert_eq!(
            hosts_selector(&["h".into()], &mixed).as_deref(),
            Some(r#"{hostname=~"h"}"#)
        );
        let parsed =
            crate::history::targets::parse_selector(&suggest_selector(&servers, &cards).unwrap())
                .unwrap();
        assert_eq!(
            parsed[0].value, "gpu-node-1|gpu-node-2",
            "селектор разбирается строгим разбором"
        );
    }

    #[test]
    fn exporter_target_is_matched_by_host_and_port() {
        let t = |job: &str, instance: &str| DcgmTarget {
            job: job.into(),
            instance: instance.into(),
        };
        let targets = vec![
            t("gpu", "10.0.0.5:9400"),
            t("dcgm", "10.0.0.5:9400"),
            t("dcgm", "10.0.0.6:9400"),
            t("dcgm", "localhost:9401"),
        ];
        let url = url::Url::parse("http://10.0.0.5:9400/metrics").unwrap();
        let found = matching_exporter_targets(&url, &targets);
        assert_eq!(
            found.iter().map(|t| t.job.as_str()).collect::<Vec<_>>(),
            ["dcgm", "gpu"]
        );
        assert_eq!(
            found[0].selector(),
            r#"{job="dcgm", instance="10.0.0.5:9400"}"#
        );
        let local = url::Url::parse("http://127.0.0.1:9401/metrics").unwrap();
        assert_eq!(matching_exporter_targets(&local, &targets).len(), 1);
    }
}
````

<!-- block: patch t04-src-tauri-src-gpu-mod-rs step=4.1 -->
````
diff --git a/src-tauri/src/gpu/mod.rs b/src-tauri/src/gpu/mod.rs
index df5f7ee..2a71693 100644
--- a/src-tauri/src/gpu/mod.rs
+++ b/src-tauri/src/gpu/mod.rs
@@ -1,4 +1,6 @@
-//! GPU инсталляции из экспортёров DCGM (спека I1d): поля карт, снимок карт по хостам.
+//! GPU инсталляции из экспортёров DCGM (спека I1d): поля карт, снимок карт по хостам, автоподбор
+//! привязок.
 
+pub mod bind;
 pub mod fields;
 pub mod snapshot;
````

- [ ] **Шаг 4.2: Увидеть падение.**

Run: `cd src-tauri && cargo test --lib -q gpu`
Expected: FAIL — не компилируется: тестам не хватает типов и функций, которые появятся в шаге 3

- [ ] **Шаг 4.3: Реализация.** Совпадение хостов и автоподбор.

<!-- block: file t04-src-tauri-src-gpu-bind-rs step=4.3 path=src-tauri/src/gpu/bind.rs part=impl -->
````
//! Автоподбор и проверка привязок GPU (спека I1d §3): хост сервера среди хостов карт, селектор
//! карт режима `prometheus`, селектор экспортёра в источнике истории, номера карт привязки.

use serde::Serialize;

use crate::history::targets::{escape, instance_matches};
use crate::storage::profile::{GpuBinding, ServerConfig, parse_base_url};

/// Хост для сравнения: без скобок IPv6, в нижнем регистре, `localhost`, `127.0.0.1` и `::1` — один
/// адрес (как автоподбор целей в I1c).
pub fn canonical_host(h: &str) -> String {
    let h = h
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_lowercase();
    match h.as_str() {
        "localhost" | "127.0.0.1" | "::1" => "loopback".into(),
        _ => h,
    }
}

fn is_ip(h: &str) -> bool {
    h.parse::<std::net::IpAddr>().is_ok()
}

/// Один ли это хост: имена без учёта регистра; короткое имя совпадает с полным, если это его
/// первая часть (`gpu-node-1` и `gpu-node-1.corp.local`: метка `hostname` экспортёра — обычно
/// короткое имя узла). Адреса IP сравниваются только целиком.
pub fn same_host(a: &str, b: &str) -> bool {
    let (a, b) = (canonical_host(a), canonical_host(b));
    if a == b {
        return true;
    }
    if is_ip(&a) || is_ip(&b) {
        return false;
    }
    let short =
        |full: &str, short: &str| !short.contains('.') && full.split('.').next() == Some(short);
    short(&a, &b) || short(&b, &a)
}

/// Хост адреса сервера.
pub fn server_host(s: &ServerConfig) -> Option<String> {
    parse_base_url(s).ok()?.host_str().map(str::to_string)
}

/// Хост карт, который совпал с хостом сервера (первый по порядку `hosts`).
pub fn matching_host<'a>(s: &ServerConfig, hosts: &'a [String]) -> Option<&'a String> {
    let own = server_host(s)?;
    hosts.iter().find(|h| same_host(h, &own))
}

/// Привязка для сервера без привязки: все карты хоста, совпавшего с хостом сервера.
pub fn suggest_binding(s: &ServerConfig, hosts: &[String]) -> Option<GpuBinding> {
    if s.gpus.is_some() {
        return None;
    }
    matching_host(s, hosts).map(|host| GpuBinding {
        host: host.clone(),
        indices: None,
    })
}

/// Что не так с привязкой сервера среди найденных карт.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingState {
    Ok,
    /// Хоста привязки нет среди карт.
    NoHost,
    /// Номеров нет на хосте.
    NoIndices(Vec<u32>),
}

/// Сверка привязки с картами: `cards` — хост и номера его карт.
pub fn binding_state(b: &GpuBinding, cards: &[(String, Vec<u32>)]) -> BindingState {
    let Some((_, have)) = cards.iter().find(|(h, _)| same_host(h, &b.host)) else {
        return BindingState::NoHost;
    };
    let absent: Vec<u32> = b
        .indices
        .iter()
        .flatten()
        .copied()
        .filter(|i| !have.contains(i))
        .collect();
    if absent.is_empty() {
        BindingState::Ok
    } else {
        BindingState::NoIndices(absent)
    }
}

/// Карты привязки: номера карт хоста, которые относятся к серверу (все или перечисленные).
pub fn bound_indices(b: &GpuBinding, have: &[u32]) -> Vec<u32> {
    match &b.indices {
        None => have.to_vec(),
        Some(list) => have.iter().copied().filter(|i| list.contains(i)).collect(),
    }
}

/// Какой меткой подписан хост карты в источнике истории.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum HostLabel {
    /// `hostname` (dcgm-exporter 4.8).
    #[serde(rename = "hostname")]
    Lower,
    /// `Hostname` (до 4.1).
    #[serde(rename = "Hostname")]
    Upper,
    /// Нет метки хоста — хост из `instance` без порта.
    #[serde(rename = "instance")]
    Instance,
}

impl HostLabel {
    pub fn name(self) -> &'static str {
        match self {
            Self::Lower => "hostname",
            Self::Upper => "Hostname",
            Self::Instance => "instance",
        }
    }
}

/// Карта, найденная в источнике истории: хост, метка хоста и номер.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FoundCard {
    pub host: String,
    pub label: HostLabel,
    pub index: u32,
}

/// Значение для регулярного выражения PromQL (RE2): метасимволы экранируются.
pub fn regex_escape(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    for c in v.chars() {
        if "\\.+*?()|[]{}^$".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Селектор карт по хостам: `{hostname=~"h1|h2"}` — меткой, которой подписаны найденные карты
/// этих хостов (`hostname` побеждает `Hostname`, `instance` — последней, с любым портом).
pub fn hosts_selector(hosts: &[String], found: &[FoundCard]) -> Option<String> {
    if hosts.is_empty() {
        return None;
    }
    let label = found
        .iter()
        .filter(|c| hosts.iter().any(|h| same_host(h, &c.host)))
        .map(|c| c.label)
        .min()
        .unwrap_or(HostLabel::Lower);
    let mut names: Vec<String> = hosts.iter().map(|h| regex_escape(h)).collect();
    names.sort();
    names.dedup();
    let alt = names.join("|");
    let value = match label {
        HostLabel::Instance => format!("({alt})(:[0-9]+)?"),
        _ => alt,
    };
    Some(format!("{{{}=~\"{}\"}}", label.name(), escape(&value)))
}

/// Автоподбор селектора карт (спека I1d §3): хосты карт, совпавшие с хостами серверов, по
/// найденной метке; ни один не совпал — `None`.
pub fn suggest_selector(servers: &[ServerConfig], found: &[FoundCard]) -> Option<String> {
    let own: Vec<String> = servers.iter().filter_map(server_host).collect();
    let mut hosts: Vec<String> = found
        .iter()
        .filter(|c| own.iter().any(|h| same_host(h, &c.host)))
        .map(|c| c.host.clone())
        .collect();
    hosts.sort();
    hosts.dedup();
    hosts_selector(&hosts, found)
}

/// Цель экспортёра DCGM в источнике истории.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DcgmTarget {
    pub job: String,
    pub instance: String,
}

impl DcgmTarget {
    /// Селектор серий цели: `{job="…", instance="…"}`.
    pub fn selector(&self) -> String {
        format!(
            "{{job=\"{}\", instance=\"{}\"}}",
            escape(&self.job),
            escape(&self.instance)
        )
    }
}

/// Цели с `instance` — хостом и портом адреса экспортёра, по порядку `job`; первая — подобранная.
pub fn matching_exporter_targets<'a>(
    exporter: &url::Url,
    targets: &'a [DcgmTarget],
) -> Vec<&'a DcgmTarget> {
    let mut found: Vec<&DcgmTarget> = targets
        .iter()
        .filter(|t| instance_matches(&t.instance, exporter))
        .collect();
    found.sort();
    found
}
````

- [ ] **Шаг 4.4: Проверки.**

Run: `cd src-tauri && cargo test --lib -q`
Expected: `test result: ok. 316 passed`
Run: `cd src-tauri && cargo clippy --lib --tests -q -- -D warnings && cargo fmt --check`
Expected: без вывода

- [ ] **Шаг 4.5: Коммит.**

<!-- block: shell t04-commit step=4.5 -->
````
git add src-tauri/src/gpu/bind.rs \
  src-tauri/src/gpu/mod.rs
git commit -s -q -F - <<'EOF'
feat(gpu): auto-match of GPU bindings, card selectors and exporter targets

A server is bound to the card host that matches its address (case,
short and full names, loopback). The card selector of the history
source mode is built from the server hosts with the label the cards
carry (hostname, Hostname or instance). An exporter is matched to the
history target with its host and port. Bindings are checked against the
found hosts and card indices.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
````

---

### Задача 5: Источник карт и приросты счётчиков

`gpu/source.rs` и `gpu/live.rs` (спека I1d §4). Экспортёры опрашиваются через механику `ServerClient` (CA, прокси,
таймауты, 8 МБ, без перенаправлений), не больше 4 одновременно, с общим входом basic; сбой экспортёра — запись в
`errors`, остальные карты показываются, сбой всех — ошибка первого; отказ во входе — `errors.gpu.unauthorized`, а не
«проверьте ключ API». Режим `prometheus` читает все серии таблицы одним мгновенным запросом под селектором карт через
клиент истории и общий предел запросов источника. Снимок — не больше 64 карт (лишние — запись `errors.gpu.tooManyGpus`).
Приросты за минуту опросов: ECC, повторы PCIe, ошибки NVLink; у времени троттлинга — доля времени, %; уменьшение
счётчика — перезапуск, прирост 0. Тесты разбирают ответ настоящего Prometheus 3.15.1, который опрашивал моки DCGM.

**Files:**
- Create: `src-tauri/src/gpu/live.rs`
- Create: `src-tauri/src/gpu/source.rs`
- Create: `src-tauri/src/gpu/testdata/dcgm-4.8.4/full-next.txt`
- Create: `src-tauri/src/gpu/testdata/prometheus-3.15.1/README.md`
- Create: `src-tauri/src/gpu/testdata/prometheus-3.15.1/cards.json`
- Create: `src-tauri/src/gpu/testdata/prometheus-3.15.1/instant.json`
- Create: `src-tauri/src/gpu/testdata/prometheus-3.15.1/targets.json`
- Modify: `src-tauri/src/gpu/mod.rs`
- Modify: `src-tauri/src/gpu/snapshot.rs`
- Modify: `src-tauri/src/gpu/testdata/dcgm-4.8.4/README.md`
- Modify: `src-tauri/src/history/read.rs`
- Modify: `src/locales/en.json`
- Modify: `src/locales/ru.json`

**Interfaces:**
- Consumes: `gpu::snapshot::*`, `history::read::{Source, permit}` (`permit` становится `pub(crate)`),
  `history::client::Sample`, `metrics::parse::{parse, Sample}`.
- Produces: `gpu::source::{OpenGpu, Exporter, SourceError { exporter?, error }, GpuSnapshot { hosts, errors, missing },
  exporter_server, as_exporter_error, instance_host, instant_expr, from_history, snapshot, too_many_gpus, CONCURRENCY}`;
  `OpenGpu::direct(&DirectGpu, Timeouts, Option<String>, &PolicyStatus)`, `OpenGpu::prometheus(&PrometheusGpu,
  Arc<Source>)`, `OpenGpu::read() -> AppResult<(Vec<GpuCard>, Vec<SourceError>)>`; `gpu::live::{Timed, GpuStatus { at,
  ...snapshot, deltaSec }, record, COUNTERS, VIOLATIONS}`; поле `GpuCard.deltas`; тестовый
  `gpu::source::tests::snapshot_file`.

- [ ] **Шаг 5.1: Тесты.** Тесты `gpu/source.rs` и `gpu/live.rs`; модули подключаются в `gpu/mod.rs`.

<!-- block: file t05-src-tauri-src-gpu-live-rs-tests step=5.1 path=src-tauri/src/gpu/live.rs part=tests -->
````
#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::snapshot::tests::{FULL, cards};
    use crate::gpu::source::snapshot;

    const FULL_NEXT: &str = include_str!("testdata/dcgm-4.8.4/full-next.txt");

    fn snap(text: &str) -> GpuSnapshot {
        snapshot(cards(text, "x"), Vec::new())
    }

    #[test]
    fn first_poll_has_no_deltas_the_next_one_counts_growth() {
        let mut h = VecDeque::new();
        let first = record(&mut h, snap(FULL), 100_000);
        assert_eq!(first.delta_sec, None);
        assert!(first.snapshot.hosts[0].cards[0].deltas.is_empty());
        let next = record(&mut h, snap(FULL_NEXT), 105_000);
        assert_eq!(next.delta_sec, Some(5.0));
        let d = &next.snapshot.hosts[0].cards[0].deltas;
        // Мок на опросе 5 против 4: повторы PCIe +3, ошибки NVLink +3 (три счётчика по +1),
        // ECC SBE +1, DBE без изменений; троттлинг по мощности +2 с, по температуре +3 с за 5 с.
        assert_eq!(d[&Field::PcieReplay], 3.0);
        assert_eq!(d[&Field::NvlinkErrors], 3.0);
        assert_eq!(d[&Field::EccSbe], 1.0);
        assert_eq!(d[&Field::EccDbe], 0.0);
        assert_eq!(d[&Field::PowerViolation], 40.0);
        assert_eq!(d[&Field::ThermalViolation], 60.0);
        assert_eq!(
            next.snapshot.hosts[0].cards[1].deltas[&Field::EccSbe],
            0.0,
            "ECC — только у карты 0"
        );
    }

    #[test]
    fn a_counter_that_went_down_is_a_restart_and_old_snapshots_expire() {
        let mut h = VecDeque::new();
        record(&mut h, snap(FULL_NEXT), 0);
        let restarted = record(&mut h, snap(FULL), 5_000);
        assert_eq!(
            restarted.snapshot.hosts[0].cards[0].deltas[&Field::PcieReplay],
            0.0
        );
        let late = record(&mut h, snap(FULL_NEXT), 200_000);
        assert_eq!(late.delta_sec, None, "снимки старше минуты отброшены");
        assert_eq!(h.len(), 1);
    }

    #[test]
    fn json_flattens_the_snapshot() {
        let mut h = VecDeque::new();
        record(&mut h, snap(FULL), 0);
        let v = serde_json::to_value(record(&mut h, snap(FULL_NEXT), 5_000)).unwrap();
        assert_eq!(v["at"], 5000);
        assert_eq!(v["deltaSec"], 5.0);
        assert_eq!(v["hosts"][0]["cards"][0]["deltas"]["pcieReplay"], 3.0);
        assert_eq!(v["missing"], serde_json::json!([]));
        assert_eq!(v["errors"], serde_json::json!([]));
    }
}
````

<!-- block: patch t05-src-tauri-src-gpu-mod-rs step=5.1 -->
````
diff --git a/src-tauri/src/gpu/mod.rs b/src-tauri/src/gpu/mod.rs
index 2a71693..cf19833 100644
--- a/src-tauri/src/gpu/mod.rs
+++ b/src-tauri/src/gpu/mod.rs
@@ -1,6 +1,8 @@
 //! GPU инсталляции из экспортёров DCGM (спека I1d): поля карт, снимок карт по хостам, автоподбор
-//! привязок.
+//! привязок, источник карт и приросты счётчиков.
 
 pub mod bind;
 pub mod fields;
+pub mod live;
 pub mod snapshot;
+pub mod source;
````

<!-- block: file t05-src-tauri-src-gpu-source-rs-tests step=5.1 path=src-tauri/src/gpu/source.rs part=tests -->
````
#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::fields::Field;
    use crate::gpu::snapshot::tests::{DEFAULT, FULL};
    use crate::history::client::HistoryClient;
    use crate::history::client::tests::source as history_source;
    use crate::servers::testing::{Reply, TestServer};
    use crate::storage::profile::{HistoryAuth, ProxyConfig, sample_exporter, sample_gpu};

    const TIMEOUTS: Timeouts = Timeouts {
        connect_ms: 2_000,
        request_ms: 2_000,
    };

    fn direct(urls: &[String], auth: GpuAuth) -> DirectGpu {
        let GpuSource::Direct(d) = sample_gpu() else {
            unreachable!()
        };
        DirectGpu {
            exporters: urls.iter().map(|u| sample_exporter(u)).collect(),
            auth,
            proxy: ProxyConfig::None,
            ..d
        }
    }

    use crate::storage::profile::GpuSource;

    pub(crate) fn snapshot_file(name: &str) -> String {
        std::fs::read_to_string(format!(
            "{}/src/gpu/testdata/prometheus-3.15.1/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    #[test]
    fn exporter_address_becomes_origin_and_metrics_path() {
        let d = direct(&[], GpuAuth::None);
        let s = exporter_server(&sample_exporter("https://gpu-1:9400/dcgm/metrics"), &d).unwrap();
        assert_eq!(
            (s.base_url.as_str(), s.metrics_path.as_str()),
            ("https://gpu-1:9400", "/dcgm/metrics")
        );
        assert_eq!(instance_host("10.0.0.5:9400"), "10.0.0.5");
        assert_eq!(instance_host("[::1]:9400"), "::1");
        assert_eq!(instance_host("gpu-node-1"), "gpu-node-1");
    }

    #[test]
    fn instant_expression_lists_every_field_series_under_the_selector() {
        let all = instant_expr(None).unwrap();
        assert!(
            all.starts_with(r#"{__name__=~"DCGM_FI_DEV_GPU_UTIL|DCGM_FI_PROF_SM_ACTIVE|"#),
            "{all}"
        );
        assert!(
            all.ends_with(r#"|DCGM_FI_DEV_NVLINK_REPLAY_TOTAL"}"#),
            "{all}"
        );
        let narrowed = instant_expr(Some(r#"{hostname=~"gpu-1|gpu-2", job="dcgm"}"#)).unwrap();
        assert!(
            narrowed.ends_with(r#"_TOTAL",hostname=~"gpu-1|gpu-2",job="dcgm"}"#),
            "{narrowed}"
        );
        assert_eq!(
            instant_expr(Some("job=dcgm")).unwrap_err().key,
            "errors.history.selector"
        );
    }

    #[tokio::test]
    async fn direct_reads_every_exporter_and_survives_a_failed_one() {
        let a = TestServer::start(vec![("/metrics", Reply::text(FULL))]).await;
        let b = TestServer::start(vec![("/dcgm/metrics", Reply::text(DEFAULT))]).await;
        let down = TestServer::start(vec![("/metrics", Reply::status(503))]).await;
        let urls = [
            format!("{}/metrics", a.url()),
            format!("{}/dcgm/metrics", b.url()),
            format!("{}/metrics", down.url()),
        ];
        let open = OpenGpu::direct(
            &direct(&urls, GpuAuth::None),
            TIMEOUTS,
            None,
            &PolicyStatus::default(),
        )
        .unwrap();
        let (cards, errors) = open.read().await.unwrap();
        assert_eq!(cards.len(), 10);
        assert_eq!(
            cards
                .iter()
                .filter(|c| c.exporter.as_deref() == Some(urls[1].as_str()))
                .count(),
            2
        );
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].exporter.as_deref(), Some(urls[2].as_str()));
        assert_eq!(errors[0].error.key, "errors.server.serverError");
        let snap = snapshot(cards, errors);
        assert_eq!(
            snap.hosts
                .iter()
                .map(|h| h.host.as_str())
                .collect::<Vec<_>>(),
            ["gpu-node-1", "localhost"]
        );
        assert!(
            snap.missing.is_empty(),
            "полный набор есть у одного экспортёра"
        );

        let only_down = OpenGpu::direct(
            &direct(&urls[2..], GpuAuth::None),
            TIMEOUTS,
            None,
            &PolicyStatus::default(),
        )
        .unwrap();
        assert_eq!(
            only_down.read().await.unwrap_err().key,
            "errors.server.serverError",
            "сбой всех — ошибка"
        );
    }

    #[tokio::test]
    async fn basic_login_goes_to_exporters_and_a_refusal_names_the_password() {
        let s = TestServer::start(vec![("/metrics", Reply::text(DEFAULT))]).await;
        let urls = [format!("{}/metrics", s.url())];
        let auth = GpuAuth::Basic {
            username: "glafira".into(),
        };
        let open = OpenGpu::direct(
            &direct(&urls, auth.clone()),
            TIMEOUTS,
            Some("dcgm-pa55".into()),
            &PolicyStatus::default(),
        )
        .unwrap();
        open.read().await.unwrap();
        let seen = s.seen.lock().unwrap().last().cloned().unwrap();
        assert_eq!(
            seen.authorization.as_deref(),
            Some("Basic Z2xhZmlyYTpkY2dtLXBhNTU=")
        );
        assert!(!format!("{open:?}").contains("dcgm-pa55"));

        let refused = TestServer::start(vec![("/metrics", Reply::status(401))]).await;
        let urls = [format!("{}/metrics", refused.url())];
        let open = OpenGpu::direct(
            &direct(&urls, auth),
            TIMEOUTS,
            Some("wrong".into()),
            &PolicyStatus::default(),
        )
        .unwrap();
        let e = open.read().await.unwrap_err();
        assert_eq!(
            (e.key.as_str(), e.params["status"].as_str()),
            ("errors.gpu.unauthorized", "401")
        );
    }

    #[tokio::test]
    async fn too_large_metrics_are_an_error() {
        let big = "x".repeat(METRICS_LIMIT + 10);
        let s = TestServer::start(vec![("/metrics", Reply::text(&big))]).await;
        let urls = [format!("{}/metrics", s.url())];
        let open = OpenGpu::direct(
            &direct(&urls, GpuAuth::None),
            TIMEOUTS,
            None,
            &PolicyStatus::default(),
        )
        .unwrap();
        assert_eq!(
            open.read().await.unwrap_err().key,
            "errors.metrics.tooLarge"
        );
    }

    #[tokio::test]
    async fn prometheus_reads_cards_from_a_real_instant_answer() {
        let p = TestServer::start(vec![(
            "/api/v1/query",
            Reply::json(&snapshot_file("instant.json")),
        )])
        .await;
        let client = HistoryClient::new(
            &history_source(&p.url(), HistoryAuth::None),
            TIMEOUTS,
            None,
            &PolicyStatus::default(),
        )
        .unwrap();
        let source = Arc::new(Source::new(
            client,
            crate::history::client::SourceKind::Prometheus,
            std::time::Duration::from_secs(5),
        ));
        let open = OpenGpu::prometheus(
            &PrometheusGpu {
                selector: Some(r#"{job="dcgm"}"#.into()),
                extra: Default::default(),
            },
            source,
        );
        let (cards, errors) = open.read().await.unwrap();
        assert!(errors.is_empty());
        let snap = snapshot(cards, errors);
        assert_eq!(
            snap.hosts
                .iter()
                .map(|h| (h.host.as_str(), h.cards.len()))
                .collect::<Vec<_>>(),
            [("gpu-node-1", 8), ("localhost", 2)]
        );
        let card = &snap.hosts[0].cards[0];
        assert_eq!(card.model.as_deref(), Some("NVIDIA H100 80GB HBM3"));
        assert_eq!(card.values[&Field::MemTotal], 81559.0);
        assert!(card.values.contains_key(&Field::SmActive));
        assert_eq!(card.exporter, None);
        let path = p.paths().pop().unwrap();
        assert!(
            path.contains("job%3D%22dcgm%22") || path.contains("job=%22dcgm%22"),
            "{path}"
        );
    }

    #[test]
    fn more_than_64_cards_are_cut_with_an_error() {
        let text: String = (0..9)
            .map(|n| FULL.replace("gpu-node-1", &format!("gpu-{n}")))
            .collect();
        let cards = crate::gpu::snapshot::tests::cards(&text, "x");
        assert_eq!(cards.len(), 72);
        let snap = snapshot(cards, Vec::new());
        assert_eq!(
            snap.hosts.iter().map(|h| h.cards.len()).sum::<usize>(),
            MAX_GPUS
        );
        assert_eq!(snap.errors[0].error.key, "errors.gpu.tooManyGpus");
        assert_eq!(snap.errors[0].error.params["found"], "72");
        let v = serde_json::to_value(&snap.errors[0]).unwrap();
        assert!(v.get("exporter").is_none(), "{v}");
    }
}
````

- [ ] **Шаг 5.2: Увидеть падение.**

Run: `cd src-tauri && cargo test --lib -q gpu`
Expected: FAIL — не компилируется: тестам не хватает типов и функций, которые появятся в шаге 3

- [ ] **Шаг 5.3: Реализация.** Источник, приросты, ответы Prometheus, второй снимок мока (команда), тексты ошибок.

<!-- block: shell t05-prepare step=5.3 -->
````
mkdir -p src-tauri/src/gpu/testdata/dcgm-4.8.4
node --input-type=module -e '
import { dcgmMetrics, parseArgs } from "./scripts/mock-server.mjs";
import { writeFileSync } from "node:fs";
const kinds = { dcgm: "default", "dcgm-full": "full", "dcgm-4.1": "4.1", "dcgm-mig": "mig" };
const make = (file, args, poll) => {
  const o = parseArgs(args);
  writeFileSync(`src-tauri/src/gpu/testdata/dcgm-4.8.4/${file}`, dcgmMetrics(kinds[o.kind], o.dcgm, poll));
};
const full = ["--kind", "dcgm-full", "--gpus", "8", "--hostname", "gpu-node-1", "--pod", "llm/vllm-0", "--throttle", "--xid", "79", "--ecc"];
make("full-next.txt", full, 5);
'
````

<!-- block: file t05-src-tauri-src-gpu-live-rs step=5.3 path=src-tauri/src/gpu/live.rs part=impl -->
````
//! Приросты счётчиков карт за последнюю минуту опросов (спека I1d §4): ошибки ECC, повторы PCIe,
//! ошибки NVLink и доля времени троттлинга. Снимки за минуту хранит пул подключений на каждую
//! инсталляцию, как `servers/live.rs`. Уменьшение счётчика — перезапуск экспортёра или драйвера,
//! прирост 0.

use std::collections::{BTreeMap, HashMap, VecDeque};

use serde::Serialize;

use crate::gpu::fields::Field;
use crate::gpu::source::GpuSnapshot;
use crate::metrics::derive::MINUTE_MS;

/// Предел истории: при опросе из нескольких вкладок снимков за минуту больше обычного.
pub const HISTORY_MAX: usize = 64;

/// Счётчики, у которых считается прирост.
pub const COUNTERS: [Field; 4] = [
    Field::EccDbe,
    Field::EccSbe,
    Field::PcieReplay,
    Field::NvlinkErrors,
];

/// Время троттлинга (нс): прирост — доля времени, %.
pub const VIOLATIONS: [Field; 2] = [Field::PowerViolation, Field::ThermalViolation];

/// Карта в истории: хост, номер, экспортёр.
type CardKey = (String, u32, Option<String>);

/// Счётчики карт на момент опроса.
#[derive(Debug, Clone)]
pub struct Timed {
    pub at_ms: u64,
    counters: HashMap<CardKey, BTreeMap<Field, f64>>,
}

/// Ответ `read_gpu_status`: снимок с приростами, время опроса и длина окна приростов (`None` —
/// первый опрос, приростов нет).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuStatus {
    pub at: u64,
    #[serde(flatten)]
    pub snapshot: GpuSnapshot,
    pub delta_sec: Option<f64>,
}

fn timed(at_ms: u64, snapshot: &GpuSnapshot) -> Timed {
    let mut counters = HashMap::new();
    for host in &snapshot.hosts {
        for c in &host.cards {
            let values: BTreeMap<Field, f64> = COUNTERS
                .iter()
                .chain(&VIOLATIONS)
                .filter_map(|f| c.values.get(f).map(|v| (*f, *v)))
                .collect();
            counters.insert((host.host.clone(), c.index, c.exporter.clone()), values);
        }
    }
    Timed { at_ms, counters }
}

/// Приросты нового снимка относительно самого старого снимка за минуту; снимок уходит в историю,
/// снимки старше минуты отбрасываются.
pub fn record(history: &mut VecDeque<Timed>, snapshot: GpuSnapshot, at_ms: u64) -> GpuStatus {
    let cur = timed(at_ms, &snapshot);
    let base = history
        .iter()
        .find(|t| t.at_ms < at_ms && at_ms - t.at_ms <= MINUTE_MS)
        .cloned();
    let mut snapshot = snapshot;
    let delta_sec = base.as_ref().map(|b| (at_ms - b.at_ms) as f64 / 1000.0);
    if let Some(base) = &base {
        let span_ns = (at_ms - base.at_ms) as f64 * 1e6;
        for host in &mut snapshot.hosts {
            for c in &mut host.cards {
                let key = (host.host.clone(), c.index, c.exporter.clone());
                let Some(old) = base.counters.get(&key) else {
                    continue;
                };
                for f in COUNTERS.iter().chain(&VIOLATIONS) {
                    let (Some(now), Some(was)) = (c.values.get(f), old.get(f)) else {
                        continue;
                    };
                    let grew = if now < was { 0.0 } else { now - was };
                    let v = if VIOLATIONS.contains(f) {
                        (grew / span_ns * 100.0).clamp(0.0, 100.0)
                    } else {
                        grew
                    };
                    c.deltas.insert(*f, v);
                }
            }
        }
    }
    history.push_back(cur);
    while history
        .front()
        .is_some_and(|t| at_ms.saturating_sub(t.at_ms) > MINUTE_MS)
        || history.len() > HISTORY_MAX
    {
        history.pop_front();
    }
    GpuStatus {
        at: at_ms,
        snapshot,
        delta_sec,
    }
}
````

<!-- block: patch t05-src-tauri-src-gpu-snapshot-rs step=5.3 -->
````
diff --git a/src-tauri/src/gpu/snapshot.rs b/src-tauri/src/gpu/snapshot.rs
index a51e5ae..47fbe22 100644
--- a/src-tauri/src/gpu/snapshot.rs
+++ b/src-tauri/src/gpu/snapshot.rs
@@ -56,6 +56,10 @@ pub struct GpuCard {
     #[serde(skip_serializing_if = "Option::is_none")]
     pub sm_source: Option<SmSource>,
     pub values: BTreeMap<Field, f64>,
+    /// Приросты счётчиков за последнюю минуту опросов (`gpu::live`); у времени троттлинга —
+    /// доля времени, %.
+    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
+    pub deltas: BTreeMap<Field, f64>,
 }
 
 #[derive(Debug, Clone, PartialEq, Serialize)]
@@ -111,6 +115,7 @@ pub fn cards_from(
                 exporter: exporter.map(str::to_string),
                 sm_source: None,
                 values: BTreeMap::new(),
+                deltas: BTreeMap::new(),
             };
             (card, BTreeMap::new())
         });
````

<!-- block: file t05-src-tauri-src-gpu-source-rs step=5.3 path=src-tauri/src/gpu/source.rs part=impl -->
````
//! Источник карт GPU (спека I1d §4): экспортёры DCGM (`direct`) или мгновенный запрос к источнику
//! истории (`prometheus`). Сбой одного экспортёра — запись в `errors`, карты остальных
//! показываются; сбой всех — ошибка первого.

use std::sync::Arc;

use serde::Serialize;
use tokio::sync::Semaphore;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::gpu::fields::{Group, all_names};
use crate::gpu::snapshot::{GpuCard, GpuHost, MAX_GPUS, cards_from, group, missing};
use crate::history::read::{Source, permit};
use crate::history::targets::{parse_selector, render};
use crate::metrics::parse::{Sample, parse, too_large};
use crate::policy::PolicyStatus;
use crate::servers::client::{METRICS_LIMIT, ServerClient, Timeouts};
use crate::storage::profile::{
    DirectGpu, GPU_NAME, GpuAuth, GpuExporter, PrometheusGpu, ServerConfig, ServerKind,
    parse_exporter_url, redact_url,
};

/// Сколько экспортёров опрашивается одновременно.
pub const CONCURRENCY: usize = 4;

/// Сбой части источника: экспортёра (`exporter` — его адрес) или всего источника.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceError {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exporter: Option<String>,
    pub error: AppError,
}

/// Снимок карт инсталляции: хосты, сбои части источника и группы счётчиков без серий.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuSnapshot {
    pub hosts: Vec<GpuHost>,
    pub errors: Vec<SourceError>,
    pub missing: Vec<Group>,
}

/// Экспортёр как адрес с настройками соединения: так его открывает общий HTTP-клиент и шаг
/// «DNS/TCP» проверки. Путь адреса — путь метрик.
pub fn exporter_server(e: &GpuExporter, d: &DirectGpu) -> AppResult<ServerConfig> {
    let url = parse_exporter_url(e)?;
    Ok(ServerConfig {
        id: crate::secrets::GPU.into(),
        name: GPU_NAME.into(),
        base_url: url.origin().ascii_serialization(),
        kind: ServerKind::Auto,
        metrics_path: url.path().to_string(),
        ca: d.ca.clone(),
        proxy: d.proxy.clone(),
        history_selector: None,
        gpus: None,
        extra: Default::default(),
    })
}

/// Отказ во входе у экспортёра называет логин и пароль экспортёров, а не ключ API сервера.
pub fn as_exporter_error(e: AppError) -> AppError {
    if e.key == "errors.server.unauthorized" {
        let status = e.params.get("status").cloned().unwrap_or_default();
        return app_err!(
            ErrorCode::AuthFailed,
            "errors.gpu.unauthorized",
            "The DCGM exporter rejected the request ({{status}}) — check the user name and password",
            status = status
        );
    }
    e
}

/// Открытый экспортёр: адрес для подписей (без имени и пароля), хост адреса и клиент.
#[derive(Debug)]
pub struct Exporter {
    pub url: String,
    pub host: String,
    client: ServerClient,
}

impl Exporter {
    pub fn new(
        e: &GpuExporter,
        d: &DirectGpu,
        timeouts: Timeouts,
        password: Option<String>,
        policy: &PolicyStatus,
    ) -> AppResult<Self> {
        let server = exporter_server(e, d)?;
        let client = ServerClient::new(&server, timeouts, None, policy)?;
        let client = match (&d.auth, password.filter(|p| !p.is_empty())) {
            (GpuAuth::Basic { username }, Some(p)) => client.with_basic_auth(username, p),
            _ => client,
        };
        let host = parse_exporter_url(e)?
            .host_str()
            .unwrap_or_default()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_string();
        Ok(Self {
            url: redact_url(&e.url),
            host,
            client,
        })
    }

    /// Серии `/metrics` экспортёра.
    pub async fn samples(&self) -> AppResult<Vec<Sample>> {
        let reply = self
            .client
            .get_ok(self.client.metrics_path(), METRICS_LIMIT)
            .await
            .map_err(as_exporter_error)?;
        if reply.truncated {
            return Err(too_large());
        }
        Ok(parse(&String::from_utf8_lossy(&reply.body))?.samples)
    }

    /// Карты экспортёра; хост карты без метки хоста — хост адреса экспортёра.
    pub async fn cards(&self) -> AppResult<Vec<GpuCard>> {
        let samples = self.samples().await?;
        let host = self.host.clone();
        Ok(cards_from(
            &samples,
            &move |_| host.clone(),
            Some(&self.url),
        ))
    }
}

/// Хост из `instance` (`host:port`, `[::1]:9400`) — без порта и скобок.
pub fn instance_host(instance: &str) -> String {
    if let Some(rest) = instance.strip_prefix('[') {
        return rest.split(']').next().unwrap_or(rest).to_string();
    }
    match instance.rsplit_once(':') {
        Some((h, p)) if !h.contains(':') && p.parse::<u16>().is_ok() => h.to_string(),
        _ => instance.to_string(),
    }
}

/// Мгновенный запрос карт к источнику истории: все серии таблицы полей под селектором карт.
pub fn instant_expr(selector: Option<&str>) -> AppResult<String> {
    let names = all_names().collect::<Vec<_>>().join("|");
    let extra = match selector {
        Some(s) => format!(",{}", render(&parse_selector(s)?)),
        None => String::new(),
    };
    Ok(format!("{{__name__=~\"{names}\"{extra}}}"))
}

/// Серии ответа источника истории в виде серий `/metrics`: имя — метка `__name__`.
pub fn from_history(samples: Vec<crate::history::client::Sample>) -> Vec<Sample> {
    samples
        .into_iter()
        .filter_map(|mut s| {
            let name = s.labels.remove("__name__")?;
            Some(Sample {
                name,
                labels: s.labels.into_iter().collect(),
                value: s.value.unwrap_or(f64::NAN),
            })
        })
        .collect()
}

/// Открытый компонент GPU инсталляции; живёт в пуле подключений до `evict` инсталляции.
#[derive(Debug)]
pub enum OpenGpu {
    Direct {
        exporters: Vec<Exporter>,
        limit: Arc<Semaphore>,
    },
    Prometheus {
        source: Arc<Source>,
        selector: Option<String>,
    },
}

impl OpenGpu {
    pub fn direct(
        d: &DirectGpu,
        timeouts: Timeouts,
        password: Option<String>,
        policy: &PolicyStatus,
    ) -> AppResult<Self> {
        let exporters = d
            .exporters
            .iter()
            .map(|e| Exporter::new(e, d, timeouts, password.clone(), policy))
            .collect::<AppResult<Vec<_>>>()?;
        Ok(Self::Direct {
            exporters,
            limit: Arc::new(Semaphore::new(CONCURRENCY)),
        })
    }

    pub fn prometheus(g: &PrometheusGpu, source: Arc<Source>) -> Self {
        Self::Prometheus {
            source,
            selector: g.selector.clone(),
        }
    }

    /// Карты и сбои экспортёров; все экспортёры (или источник) отказали — ошибка первого.
    pub async fn read(&self) -> AppResult<(Vec<GpuCard>, Vec<SourceError>)> {
        match self {
            Self::Direct { exporters, limit } => {
                let reads = exporters.iter().map(|e| {
                    let limit = limit.clone();
                    async move {
                        let _permit = limit.acquire_owned().await.expect("never closed");
                        (e, e.cards().await)
                    }
                });
                let mut cards = Vec::new();
                let mut errors = Vec::new();
                for (e, result) in futures::future::join_all(reads).await {
                    match result {
                        Ok(c) => cards.extend(c),
                        Err(error) => errors.push(SourceError {
                            exporter: Some(e.url.clone()),
                            error,
                        }),
                    }
                }
                if !errors.is_empty() && errors.len() == exporters.len() {
                    return Err(errors.swap_remove(0).error);
                }
                Ok((cards, errors))
            }
            Self::Prometheus { source, selector } => {
                let expr = instant_expr(selector.as_deref())?;
                let samples = {
                    let _permit = permit(source).await;
                    source.client.query(&expr).await?
                };
                let samples = from_history(samples);
                let cards = cards_from(
                    &samples,
                    &|s| instance_host(s.label("instance").unwrap_or_default()),
                    None,
                );
                Ok((cards, Vec::new()))
            }
        }
    }
}

/// Снимок из карт: не больше 64 карт (лишние — запись `errors.gpu.tooManyGpus`), группы
/// счётчиков без серий.
pub fn snapshot(cards: Vec<GpuCard>, mut errors: Vec<SourceError>) -> GpuSnapshot {
    let absent = missing(&cards);
    let total = cards.len();
    let (hosts, truncated) = group(cards, MAX_GPUS);
    if truncated {
        errors.push(SourceError {
            exporter: None,
            error: too_many_gpus(total),
        });
    }
    GpuSnapshot {
        hosts,
        errors,
        missing: absent,
    }
}

pub fn too_many_gpus(found: usize) -> AppError {
    app_err!(
        ErrorCode::InvalidInput,
        "errors.gpu.tooManyGpus",
        "Found {{found}} GPUs, an installation can have at most {{max}}: narrow the card selector or remove exporters",
        found = found,
        max = MAX_GPUS
    )
}
````

<!-- block: patch t05-src-tauri-src-gpu-testdata-dcgm-4-8-4-readme-md step=5.3 -->
````
diff --git a/src-tauri/src/gpu/testdata/dcgm-4.8.4/README.md b/src-tauri/src/gpu/testdata/dcgm-4.8.4/README.md
index bd73da0..7f0a190 100644
--- a/src-tauri/src/gpu/testdata/dcgm-4.8.4/README.md
+++ b/src-tauri/src/gpu/testdata/dcgm-4.8.4/README.md
@@ -9,6 +9,7 @@ dcgm-exporter 4.8.4, собран по его исходникам (`etc/default
 |---|---|---|
 | `default.txt` | `dcgm --gpus 2` (набор по умолчанию: без активности SM, ECC и причин троттлинга) | 0 |
 | `full.txt` | `dcgm-full --gpus 8 --hostname gpu-node-1 --pod llm/vllm-0 --throttle --xid 79 --ecc` | 4 |
+| `full-next.txt` | то же | 5 |
 | `4.1.txt` | `dcgm-4.1 --gpus 2 --hostname gpu-old` (метка `Hostname`, A100) | 0 |
 | `mig.txt` | `dcgm-mig --gpus 2 --hostname gpu-mig` (карта 0 — два экземпляра MIG) | 0 |
 
@@ -26,6 +27,7 @@ const make = (file, args, poll) => {
 const full = ["--kind", "dcgm-full", "--gpus", "8", "--hostname", "gpu-node-1", "--pod", "llm/vllm-0", "--throttle", "--xid", "79", "--ecc"];
 make("default.txt", ["--kind", "dcgm", "--gpus", "2"], 0);
 make("full.txt", full, 4);
+make("full-next.txt", full, 5);
 make("4.1.txt", ["--kind", "dcgm-4.1", "--gpus", "2", "--hostname", "gpu-old"], 0);
 make("mig.txt", ["--kind", "dcgm-mig", "--gpus", "2", "--hostname", "gpu-mig"], 0);
 '
````

<!-- block: file t05-src-tauri-src-gpu-testdata-prometheus-3-15-1-readme-md step=5.3 path=src-tauri/src/gpu/testdata/prometheus-3.15.1/README.md -->
````
# Ответы Prometheus 3.15.1 с сериями DCGM

Настоящие ответы HTTP API Prometheus 3.15.1 (Homebrew, macOS), сняты 2026-11-01. Prometheus опрашивал раз в 5 с
мок-серверы Glafira (`scripts/prometheus/smoke.yml`, job `dcgm`): `dcgm-full --gpus 8 --hostname gpu-node-1` на 9400
и `dcgm --gpus 2` на 9401; цель 9402 не запущена.

| Файл | Запрос |
|---|---|
| `instant.json` | мгновенный запрос карт `gpu::source::instant_expr` с селектором `{job="dcgm"}` |
| `cards.json` | `count by (hostname, Hostname, instance, gpu) (DCGM_FI_DEV_FB_USED)` |
| `targets.json` | `count by (job, instance) (DCGM_FI_DEV_FB_USED)` |
````

<!-- block: file t05-src-tauri-src-gpu-testdata-prometheus-3-15-1-cards-json step=5.3 path=src-tauri/src/gpu/testdata/prometheus-3.15.1/cards.json -->
````
{"status":"success","data":{"resultType":"vector","result":[{"metric":{"gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"value":[1793556218.623,"1"]},{"metric":{"gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"value":[1793556218.623,"1"]},{"metric":{"gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"value":[1793556218.623,"1"]},{"metric":{"gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"value":[1793556218.623,"1"]},{"metric":{"gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"value":[1793556218.623,"1"]},{"metric":{"gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"value":[1793556218.623,"1"]},{"metric":{"gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"value":[1793556218.623,"1"]},{"metric":{"gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"value":[1793556218.623,"1"]},{"metric":{"gpu":"0","hostname":"localhost","instance":"127.0.0.1:9401"},"value":[1793556218.623,"1"]},{"metric":{"gpu":"1","hostname":"localhost","instance":"127.0.0.1:9401"},"value":[1793556218.623,"1"]}]}}
````

<!-- block: file t05-src-tauri-src-gpu-testdata-prometheus-3-15-1-instant-json step=5.3 path=src-tauri/src/gpu/testdata/prometheus-3.15.1/instant.json -->
````
{"status":"success","data":{"resultType":"vector","result":[{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_SM_CLOCK","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"1980"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_SM_CLOCK","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"1980"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_DEV_SM_CLOCK","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"1980"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_DEV_SM_CLOCK","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"1980"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_DEV_SM_CLOCK","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"1980"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_DEV_SM_CLOCK","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"1980"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_DEV_SM_CLOCK","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"1980"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_DEV_SM_CLOCK","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"1980"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_MEM_CLOCK","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"2619"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_MEM_CLOCK","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"2619"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_DEV_MEM_CLOCK","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"2619"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_DEV_MEM_CLOCK","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"2619"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_DEV_MEM_CLOCK","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"2619"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_DEV_MEM_CLOCK","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"2619"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_DEV_MEM_CLOCK","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"2619"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_DEV_MEM_CLOCK","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"2619"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_MEMORY_TEMP","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"54"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_MEMORY_TEMP","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"52"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_DEV_MEMORY_TEMP","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"49"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_DEV_MEMORY_TEMP","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"50"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_DEV_MEMORY_TEMP","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"53"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_DEV_MEMORY_TEMP","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"55"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_DEV_MEMORY_TEMP","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"57"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_DEV_MEMORY_TEMP","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"58"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_GPU_TEMP","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"60"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_GPU_TEMP","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"54"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_DEV_GPU_TEMP","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"47"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_DEV_GPU_TEMP","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"50"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_DEV_GPU_TEMP","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"57"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_DEV_GPU_TEMP","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"62"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_DEV_GPU_TEMP","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"67"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_DEV_GPU_TEMP","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"69"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_POWER_USAGE","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"437.203"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_POWER_USAGE","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"566.572"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_DEV_POWER_USAGE","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"627.161"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_DEV_POWER_USAGE","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"608.19"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_DEV_POWER_USAGE","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"513.035"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_DEV_POWER_USAGE","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"358.625"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_DEV_POWER_USAGE","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"187.566"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_DEV_POWER_USAGE","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"372.411"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_PCIE_REPLAY_COUNTER","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_PCIE_REPLAY_COUNTER","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_DEV_PCIE_REPLAY_COUNTER","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_DEV_PCIE_REPLAY_COUNTER","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_DEV_PCIE_REPLAY_COUNTER","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_DEV_PCIE_REPLAY_COUNTER","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_DEV_PCIE_REPLAY_COUNTER","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_DEV_PCIE_REPLAY_COUNTER","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_GPU_UTIL","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"71"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_GPU_UTIL","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"87"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_DEV_GPU_UTIL","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"95"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_DEV_GPU_UTIL","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"92"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_DEV_GPU_UTIL","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"81"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_DEV_GPU_UTIL","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"62"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_DEV_GPU_UTIL","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"41"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_DEV_GPU_UTIL","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"64"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_MEM_COPY_UTIL","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"58"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_MEM_COPY_UTIL","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"68"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_DEV_MEM_COPY_UTIL","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"70"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_DEV_MEM_COPY_UTIL","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"62"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_DEV_MEM_COPY_UTIL","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"48"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_DEV_MEM_COPY_UTIL","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"28"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_DEV_MEM_COPY_UTIL","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"33"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_DEV_MEM_COPY_UTIL","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"52"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_XID_ERRORS","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_XID_ERRORS","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_DEV_XID_ERRORS","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_DEV_XID_ERRORS","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_DEV_XID_ERRORS","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_DEV_XID_ERRORS","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_DEV_XID_ERRORS","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_DEV_XID_ERRORS","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_FB_FREE","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"26196"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_FB_FREE","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"25076"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_DEV_FB_FREE","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"18763"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_DEV_FB_FREE","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"13624"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_DEV_FB_FREE","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"10247"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_DEV_FB_FREE","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"9020"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_DEV_FB_FREE","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"10082"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_DEV_FB_FREE","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"13312"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_FB_USED","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"54822"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_FB_USED","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"55942"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_DEV_FB_USED","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"62255"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_DEV_FB_USED","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"67394"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_DEV_FB_USED","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"70771"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_DEV_FB_USED","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"71998"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_DEV_FB_USED","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"70936"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_DEV_FB_USED","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"67706"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_FB_RESERVED","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"541"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_FB_RESERVED","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"541"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_DEV_FB_RESERVED","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"541"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_DEV_FB_RESERVED","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"541"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_DEV_FB_RESERVED","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"541"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_DEV_FB_RESERVED","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"541"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_DEV_FB_RESERVED","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"541"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_DEV_FB_RESERVED","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"541"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_NVLINK_BANDWIDTH_TOTAL","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"52999"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_NVLINK_BANDWIDTH_TOTAL","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"80699"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_DEV_NVLINK_BANDWIDTH_TOTAL","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"102596"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_DEV_NVLINK_BANDWIDTH_TOTAL","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"116180"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_DEV_NVLINK_BANDWIDTH_TOTAL","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"119898"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_DEV_NVLINK_BANDWIDTH_TOTAL","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"113323"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_DEV_NVLINK_BANDWIDTH_TOTAL","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"97209"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_DEV_NVLINK_BANDWIDTH_TOTAL","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"73400"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_UNCORRECTABLE_REMAPPED_ROWS","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_UNCORRECTABLE_REMAPPED_ROWS","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_DEV_UNCORRECTABLE_REMAPPED_ROWS","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_DEV_UNCORRECTABLE_REMAPPED_ROWS","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_DEV_UNCORRECTABLE_REMAPPED_ROWS","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_DEV_UNCORRECTABLE_REMAPPED_ROWS","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_DEV_UNCORRECTABLE_REMAPPED_ROWS","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_DEV_UNCORRECTABLE_REMAPPED_ROWS","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_CORRECTABLE_REMAPPED_ROWS","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_CORRECTABLE_REMAPPED_ROWS","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_DEV_CORRECTABLE_REMAPPED_ROWS","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_DEV_CORRECTABLE_REMAPPED_ROWS","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_DEV_CORRECTABLE_REMAPPED_ROWS","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_DEV_CORRECTABLE_REMAPPED_ROWS","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_DEV_CORRECTABLE_REMAPPED_ROWS","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_DEV_CORRECTABLE_REMAPPED_ROWS","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_ROW_REMAP_FAILURE","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_ROW_REMAP_FAILURE","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_DEV_ROW_REMAP_FAILURE","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_DEV_ROW_REMAP_FAILURE","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_DEV_ROW_REMAP_FAILURE","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_DEV_ROW_REMAP_FAILURE","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_DEV_ROW_REMAP_FAILURE","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_DEV_ROW_REMAP_FAILURE","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_PROF_GR_ENGINE_ACTIVE","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0.714359"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_PROF_GR_ENGINE_ACTIVE","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0.872476"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_PROF_GR_ENGINE_ACTIVE","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"0.94653"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_PROF_GR_ENGINE_ACTIVE","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"0.923344"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_PROF_GR_ENGINE_ACTIVE","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"0.807043"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_PROF_GR_ENGINE_ACTIVE","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"0.618319"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_PROF_GR_ENGINE_ACTIVE","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"0.409248"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_PROF_GR_ENGINE_ACTIVE","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"0.635169"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_PROF_PIPE_TENSOR_ACTIVE","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0.328625"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_PROF_PIPE_TENSOR_ACTIVE","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0.443619"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_PROF_PIPE_TENSOR_ACTIVE","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"0.497476"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_PROF_PIPE_TENSOR_ACTIVE","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"0.480614"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_PROF_PIPE_TENSOR_ACTIVE","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"0.396031"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_PROF_PIPE_TENSOR_ACTIVE","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"0.258778"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_PROF_PIPE_TENSOR_ACTIVE","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"0.106726"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_PROF_PIPE_TENSOR_ACTIVE","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"0.271032"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_PROF_DRAM_ACTIVE","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0.578401"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_PROF_DRAM_ACTIVE","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0.679494"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_PROF_DRAM_ACTIVE","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"0.695273"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_PROF_DRAM_ACTIVE","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"0.622933"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_PROF_DRAM_ACTIVE","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"0.475343"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_PROF_DRAM_ACTIVE","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"0.278763"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_PROF_DRAM_ACTIVE","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"0.33183"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_PROF_DRAM_ACTIVE","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"0.518968"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_PROF_PCIE_TX_BYTES","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"775354565"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_PROF_PCIE_TX_BYTES","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"594058025"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_PROF_PCIE_TX_BYTES","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"289575060"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_PROF_PCIE_TX_BYTES","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"442909952"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_PROF_PCIE_TX_BYTES","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"699458918"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_PROF_PCIE_TX_BYTES","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"799872099"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_PROF_PCIE_TX_BYTES","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"712759345"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_PROF_PCIE_TX_BYTES","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"465352962"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_PROF_PCIE_RX_BYTES","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"1231996452"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_PROF_PCIE_RX_BYTES","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"836488998"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_PROF_PCIE_RX_BYTES","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"664208236"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_PROF_PCIE_RX_BYTES","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"1113572355"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_PROF_PCIE_RX_BYTES","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"1371127705"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_PROF_PCIE_RX_BYTES","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"1356359941"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_PROF_PCIE_RX_BYTES","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"1073885613"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_PROF_PCIE_RX_BYTES","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"612008981"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_PROF_SM_ACTIVE","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0.664359"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_PROF_SM_ACTIVE","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0.822476"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_PROF_SM_ACTIVE","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"0.89653"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_PROF_SM_ACTIVE","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"0.873344"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_PROF_SM_ACTIVE","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"0.757043"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_PROF_SM_ACTIVE","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"0.568319"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_PROF_SM_ACTIVE","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"0.359248"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_PROF_SM_ACTIVE","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"0.585169"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_ECC_SBE_VOL_TOTAL","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_ECC_SBE_VOL_TOTAL","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_DEV_ECC_SBE_VOL_TOTAL","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_DEV_ECC_SBE_VOL_TOTAL","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_DEV_ECC_SBE_VOL_TOTAL","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_DEV_ECC_SBE_VOL_TOTAL","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_DEV_ECC_SBE_VOL_TOTAL","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_DEV_ECC_SBE_VOL_TOTAL","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_ECC_DBE_VOL_TOTAL","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_ECC_DBE_VOL_TOTAL","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_DEV_ECC_DBE_VOL_TOTAL","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_DEV_ECC_DBE_VOL_TOTAL","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_DEV_ECC_DBE_VOL_TOTAL","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_DEV_ECC_DBE_VOL_TOTAL","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_DEV_ECC_DBE_VOL_TOTAL","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_DEV_ECC_DBE_VOL_TOTAL","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_CLOCKS_EVENT_REASONS","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_CLOCKS_EVENT_REASONS","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_DEV_CLOCKS_EVENT_REASONS","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_DEV_CLOCKS_EVENT_REASONS","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_DEV_CLOCKS_EVENT_REASONS","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_DEV_CLOCKS_EVENT_REASONS","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_DEV_CLOCKS_EVENT_REASONS","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_DEV_CLOCKS_EVENT_REASONS","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_POWER_VIOLATION","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_POWER_VIOLATION","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_DEV_POWER_VIOLATION","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_DEV_POWER_VIOLATION","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_DEV_POWER_VIOLATION","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_DEV_POWER_VIOLATION","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_DEV_POWER_VIOLATION","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_DEV_POWER_VIOLATION","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_THERMAL_VIOLATION","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_THERMAL_VIOLATION","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_DEV_THERMAL_VIOLATION","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_DEV_THERMAL_VIOLATION","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_DEV_THERMAL_VIOLATION","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_DEV_THERMAL_VIOLATION","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_DEV_THERMAL_VIOLATION","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_DEV_THERMAL_VIOLATION","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_NVLINK_CRC_ERROR_TOTAL","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_NVLINK_CRC_ERROR_TOTAL","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_DEV_NVLINK_CRC_ERROR_TOTAL","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_DEV_NVLINK_CRC_ERROR_TOTAL","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_DEV_NVLINK_CRC_ERROR_TOTAL","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_DEV_NVLINK_CRC_ERROR_TOTAL","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_DEV_NVLINK_CRC_ERROR_TOTAL","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_DEV_NVLINK_CRC_ERROR_TOTAL","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_NVLINK_RECOVERY_TOTAL","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_NVLINK_RECOVERY_TOTAL","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_DEV_NVLINK_RECOVERY_TOTAL","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_DEV_NVLINK_RECOVERY_TOTAL","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_DEV_NVLINK_RECOVERY_TOTAL","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_DEV_NVLINK_RECOVERY_TOTAL","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_DEV_NVLINK_RECOVERY_TOTAL","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_DEV_NVLINK_RECOVERY_TOTAL","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_NVLINK_REPLAY_TOTAL","device":"nvidia0","gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_NVLINK_REPLAY_TOTAL","device":"nvidia1","gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0200-4b1c-9f3e-0a1b2c3d4e02","__name__":"DCGM_FI_DEV_NVLINK_REPLAY_TOTAL","device":"nvidia2","gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:38:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0300-4b1c-9f3e-0a1b2c3d4e03","__name__":"DCGM_FI_DEV_NVLINK_REPLAY_TOTAL","device":"nvidia3","gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:48:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0400-4b1c-9f3e-0a1b2c3d4e04","__name__":"DCGM_FI_DEV_NVLINK_REPLAY_TOTAL","device":"nvidia4","gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:58:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0500-4b1c-9f3e-0a1b2c3d4e05","__name__":"DCGM_FI_DEV_NVLINK_REPLAY_TOTAL","device":"nvidia5","gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:68:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0600-4b1c-9f3e-0a1b2c3d4e06","__name__":"DCGM_FI_DEV_NVLINK_REPLAY_TOTAL","device":"nvidia6","gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:78:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0700-4b1c-9f3e-0a1b2c3d4e07","__name__":"DCGM_FI_DEV_NVLINK_REPLAY_TOTAL","device":"nvidia7","gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:88:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_SM_CLOCK","device":"nvidia0","gpu":"0","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"1980"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_SM_CLOCK","device":"nvidia1","gpu":"1","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"1980"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_MEM_CLOCK","device":"nvidia0","gpu":"0","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"2619"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_MEM_CLOCK","device":"nvidia1","gpu":"1","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"2619"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_MEMORY_TEMP","device":"nvidia0","gpu":"0","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"55"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_MEMORY_TEMP","device":"nvidia1","gpu":"1","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"53"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_GPU_TEMP","device":"nvidia0","gpu":"0","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"63"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_GPU_TEMP","device":"nvidia1","gpu":"1","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"58"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_POWER_USAGE","device":"nvidia0","gpu":"0","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"337.852"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_POWER_USAGE","device":"nvidia1","gpu":"1","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"497.564"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_PCIE_REPLAY_COUNTER","device":"nvidia0","gpu":"0","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_PCIE_REPLAY_COUNTER","device":"nvidia1","gpu":"1","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_GPU_UTIL","device":"nvidia0","gpu":"0","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"59"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_GPU_UTIL","device":"nvidia1","gpu":"1","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"79"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_MEM_COPY_UTIL","device":"nvidia0","gpu":"0","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"49"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_MEM_COPY_UTIL","device":"nvidia1","gpu":"1","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"63"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_XID_ERRORS","device":"nvidia0","gpu":"0","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_XID_ERRORS","device":"nvidia1","gpu":"1","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_FB_FREE","device":"nvidia0","gpu":"0","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"22318"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_FB_FREE","device":"nvidia1","gpu":"1","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"28986"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_FB_USED","device":"nvidia0","gpu":"0","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"58700"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_FB_USED","device":"nvidia1","gpu":"1","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"52032"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_FB_RESERVED","device":"nvidia0","gpu":"0","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"541"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_FB_RESERVED","device":"nvidia1","gpu":"1","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"541"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_NVLINK_BANDWIDTH_TOTAL","device":"nvidia0","gpu":"0","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"35254"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_NVLINK_BANDWIDTH_TOTAL","device":"nvidia1","gpu":"1","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"64916"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_UNCORRECTABLE_REMAPPED_ROWS","device":"nvidia0","gpu":"0","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_UNCORRECTABLE_REMAPPED_ROWS","device":"nvidia1","gpu":"1","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_CORRECTABLE_REMAPPED_ROWS","device":"nvidia0","gpu":"0","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_CORRECTABLE_REMAPPED_ROWS","device":"nvidia1","gpu":"1","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_DEV_ROW_REMAP_FAILURE","device":"nvidia0","gpu":"0","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_DEV_ROW_REMAP_FAILURE","device":"nvidia1","gpu":"1","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_PROF_GR_ENGINE_ACTIVE","device":"nvidia0","gpu":"0","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0.592931"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_PROF_GR_ENGINE_ACTIVE","device":"nvidia1","gpu":"1","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0.788134"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_PROF_PIPE_TENSOR_ACTIVE","device":"nvidia0","gpu":"0","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0.240313"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_PROF_PIPE_TENSOR_ACTIVE","device":"nvidia1","gpu":"1","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0.382279"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_PROF_DRAM_ACTIVE","device":"nvidia0","gpu":"0","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"0.485781"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_PROF_DRAM_ACTIVE","device":"nvidia1","gpu":"1","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"0.629524"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_PROF_PCIE_TX_BYTES","device":"nvidia0","gpu":"0","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"799372950"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_PROF_PCIE_TX_BYTES","device":"nvidia1","gpu":"1","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"720409738"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0000-4b1c-9f3e-0a1b2c3d4e00","__name__":"DCGM_FI_PROF_PCIE_RX_BYTES","device":"nvidia0","gpu":"0","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:18:00.0"},"value":[1793556218.589,"1363031847"]},{"metric":{"DCGM_FI_DRIVER_VERSION":"570.124.06","UUID":"GPU-6c2d1a3e-0100-4b1c-9f3e-0a1b2c3d4e01","__name__":"DCGM_FI_PROF_PCIE_RX_BYTES","device":"nvidia1","gpu":"1","hostname":"localhost","instance":"127.0.0.1:9401","job":"dcgm","modelName":"NVIDIA H100 80GB HBM3","pci_bus_id":"00000000:28:00.0"},"value":[1793556218.589,"1091087038"]}]}}
````

<!-- block: file t05-src-tauri-src-gpu-testdata-prometheus-3-15-1-targets-json step=5.3 path=src-tauri/src/gpu/testdata/prometheus-3.15.1/targets.json -->
````
{"status":"success","data":{"resultType":"vector","result":[{"metric":{"instance":"127.0.0.1:9400","job":"dcgm"},"value":[1793556218.646,"8"]},{"metric":{"instance":"127.0.0.1:9401","job":"dcgm"},"value":[1793556218.646,"2"]}]}}
````

<!-- block: patch t05-src-tauri-src-history-read-rs step=5.3 -->
````
diff --git a/src-tauri/src/history/read.rs b/src-tauri/src/history/read.rs
index 08bbf6f..9cc8193 100644
--- a/src-tauri/src/history/read.rs
+++ b/src-tauri/src/history/read.rs
@@ -65,7 +65,7 @@ fn exact(matchers: &[crate::history::targets::Matcher], label: &str) -> Option<S
 }
 
 /// Разрешение на один запрос к источнику; держится, пока запрос идёт.
-async fn permit(source: &Source) -> tokio::sync::OwnedSemaphorePermit {
+pub(crate) async fn permit(source: &Source) -> tokio::sync::OwnedSemaphorePermit {
     source
         .limit
         .clone()
````

<!-- block: locales t05-locales-t05 step=5.3 -->
````
{
  "en": {
    "errors": { "gpu": {
      "tooManyGpus": "Found {{found}} GPUs, an installation can have at most {{max}}: narrow the card selector or remove exporters",
      "unauthorized": "The DCGM exporter rejected the request ({{status}}) — check the user name and password"
    } }
  },
  "ru": {
    "errors": { "gpu": {
      "tooManyGpus": "Найдено карт: {{found}}, у инсталляции их может быть не больше {{max}} — сузьте селектор карт или уберите экспортёры",
      "unauthorized": "Экспортёр DCGM отклонил запрос ({{status}}) — проверьте имя пользователя и пароль"
    } }
  }
}
````

- [ ] **Шаг 5.4: Проверки.**

Run: `cd src-tauri && cargo test --lib -q`
Expected: `test result: ok. 326 passed`
Run: `cd src-tauri && cargo clippy --lib --tests -q -- -D warnings && cargo fmt --check`
Expected: без вывода
Run: `pnpm i18n:check`
Expected: `✓ словари en, ru согласованы, вызовов из Rust: 159`

- [ ] **Шаг 5.5: Коммит.**

<!-- block: shell t05-commit step=5.5 -->
````
git add src-tauri/src/gpu/live.rs \
  src-tauri/src/gpu/mod.rs \
  src-tauri/src/gpu/snapshot.rs \
  src-tauri/src/gpu/source.rs \
  src-tauri/src/gpu/testdata/dcgm-4.8.4/README.md \
  src-tauri/src/gpu/testdata/dcgm-4.8.4/full-next.txt \
  src-tauri/src/gpu/testdata/prometheus-3.15.1/README.md \
  src-tauri/src/gpu/testdata/prometheus-3.15.1/cards.json \
  src-tauri/src/gpu/testdata/prometheus-3.15.1/instant.json \
  src-tauri/src/gpu/testdata/prometheus-3.15.1/targets.json \
  src-tauri/src/history/read.rs \
  src/locales/en.json \
  src/locales/ru.json
git commit -s -q -F - <<'EOF'
feat(gpu): card source and counter deltas

DCGM exporters are scraped through the server HTTP client, at most four
at a time, with the shared basic login; a failed exporter becomes an
entry in errors while the other cards stay, a refused login names the
exporter password. The history source mode reads all field series in
one instant query under the card selector, sharing the query limit of
the source. Snapshots keep at most 64 cards; ECC, PCIe replay and
NVLink error counters get deltas over the last minute of polls, the
throttle durations a share of time. Answers of a real Prometheus.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
````

---

### Задача 6: Шаблоны PromQL и ряды окон истории GPU

`gpu/promql.rs` и `gpu/series.rs` (спека I1d §4): 14 запросов на окно — среднее датчиков и долей, максимум маски
троттлинга, прирост счётчиков, группировка по `hostname, Hostname, instance, gpu`, без серий экземпляров MIG
(`GPU_I_ID=""`). Сумма счётчиков ошибок NVLink — подзапросом `increase((sum by (…) (…))[r:r/4])`: `increase` по каждому
счётчику убрал бы имя серии, и одинаковые наборы меток столкнулись бы (Prometheus отвечает 422). Ряды выравниваются по
шагам окна: ряд на карту, не больше 16; активность SM — в процентах, всего памяти — суммой; сбой запроса — `partial`
его графика.

**Files:**
- Create: `src-tauri/src/gpu/promql.rs`
- Create: `src-tauri/src/gpu/series.rs`
- Create: `src-tauri/src/gpu/testdata/prometheus-3.15.1/range-gpu-node-1.json`
- Modify: `src-tauri/src/gpu/mod.rs`
- Modify: `src-tauri/src/gpu/source.rs`
- Modify: `src-tauri/src/gpu/testdata/prometheus-3.15.1/README.md`

**Interfaces:**
- Consumes: `gpu::fields::SPECS`, `history::window::{Range, duration}`, `history::client::RangeSeries`,
  `history::series::Partial`, `gpu::source::instance_host`.
- Produces: `gpu::promql::{queries, GpuQuery { field, expr }, HISTORY_FIELDS, BY, chart_of, hosts_expr}`;
  `queries(matchers: &str, rate_sec: u64)`; `gpu::series::{assemble, GpuSeries { host, gpu, points }, GpuPoint { at,
  values }, QueryResult, MAX_SERIES, series_host}`; `assemble(&Range, Vec<QueryResult>) -> (Vec<GpuSeries>,
  Vec<Partial>, bool)`.

- [ ] **Шаг 6.1: Тесты.** Тесты `gpu/promql.rs` (точный текст шаблонов) и `gpu/series.rs` (снятое окно); модули подключаются.

<!-- block: patch t06-src-tauri-src-gpu-mod-rs step=6.1 -->
````
diff --git a/src-tauri/src/gpu/mod.rs b/src-tauri/src/gpu/mod.rs
index cf19833..2919535 100644
--- a/src-tauri/src/gpu/mod.rs
+++ b/src-tauri/src/gpu/mod.rs
@@ -1,8 +1,10 @@
 //! GPU инсталляции из экспортёров DCGM (спека I1d): поля карт, снимок карт по хостам, автоподбор
-//! привязок, источник карт и приросты счётчиков.
+//! привязок, источник карт и приросты счётчиков, окна истории.
 
 pub mod bind;
 pub mod fields;
 pub mod live;
+pub mod promql;
+pub mod series;
 pub mod snapshot;
 pub mod source;
````

<!-- block: file t06-src-tauri-src-gpu-promql-rs-tests step=6.1 path=src-tauri/src/gpu/promql.rs part=tests -->
````
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_have_exact_text() {
        let q = queries(r#"job="dcgm",hostname="gpu-node-1""#, 60);
        assert_eq!(q.len(), 14);
        let expr = |f: Field| q.iter().find(|x| x.field == f).unwrap().expr.clone();
        let s = r#"{job="dcgm",hostname="gpu-node-1",GPU_I_ID=""}"#;
        assert_eq!(
            expr(Field::Util),
            format!(
                "avg by (hostname, Hostname, instance, gpu) (avg_over_time(DCGM_FI_DEV_GPU_UTIL{s}[1m]))"
            )
        );
        assert_eq!(
            expr(Field::SmActive),
            format!(
                "avg by (hostname, Hostname, instance, gpu) (avg_over_time(DCGM_FI_PROF_SM_ACTIVE{s}[1m])) or avg by (hostname, Hostname, instance, gpu) (avg_over_time(DCGM_FI_PROF_GR_ENGINE_ACTIVE{s}[1m]))"
            )
        );
        assert_eq!(
            expr(Field::Throttle),
            format!(
                "max by (hostname, Hostname, instance, gpu) (max_over_time(DCGM_FI_DEV_CLOCKS_EVENT_REASONS{s}[1m])) or max by (hostname, Hostname, instance, gpu) (max_over_time(DCGM_FI_DEV_CLOCK_THROTTLE_REASONS{s}[1m]))"
            )
        );
        assert_eq!(
            expr(Field::PcieReplay),
            format!(
                "sum by (hostname, Hostname, instance, gpu) (increase(DCGM_FI_DEV_PCIE_REPLAY_COUNTER{s}[1m]))"
            )
        );
        assert!(
            expr(Field::NvlinkErrors).starts_with(
                r#"increase((sum by (hostname, Hostname, instance, gpu) ({__name__=~"DCGM_FI_DEV_NVLINK_CRC_FLIT_ERROR_TOTAL|"#
            ),
            "{}",
            expr(Field::NvlinkErrors)
        );
        assert!(expr(Field::NvlinkErrors).ends_with(r#"|DCGM_FI_DEV_NVLINK_REPLAY_TOTAL",job="dcgm",hostname="gpu-node-1",GPU_I_ID=""}))[1m:15s])"#));
        let month = queries("", 2700);
        assert_eq!(
            month[0].expr,
            r#"avg by (hostname, Hostname, instance, gpu) (avg_over_time(DCGM_FI_DEV_GPU_UTIL{GPU_I_ID=""}[45m]))"#
        );
        assert!(
            month[13].expr.ends_with("[45m:675s])"),
            "{}",
            month[13].expr
        );
        assert_eq!(
            hosts_expr(r#"job="dcgm""#),
            r#"count by (hostname, Hostname, instance) (DCGM_FI_DEV_FB_USED{job="dcgm"})"#
        );
    }

    #[test]
    fn every_field_has_a_chart() {
        assert_eq!(chart_of(Field::SmActive), "util");
        assert_eq!(chart_of(Field::MemReserved), "memory");
        assert_eq!(chart_of(Field::MemClock), "clocks");
        assert_eq!(chart_of(Field::NvlinkErrors), "errors");
        let charts: std::collections::BTreeSet<_> =
            HISTORY_FIELDS.iter().map(|f| chart_of(*f)).collect();
        assert_eq!(charts.len(), 6);
    }
}
````

<!-- block: file t06-src-tauri-src-gpu-series-rs-tests step=6.1 path=src-tauri/src/gpu/series.rs part=tests -->
````
#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{AppError, ErrorCode};
    use crate::history::client::parse_matrix;

    /// Снятое окно `now` хоста gpu-node-1: `{ start, end, step, rate, queries: [{ expr, response }] }`.
    fn captured() -> (Range, Vec<QueryResult>) {
        let raw: serde_json::Value = serde_json::from_str(
            &crate::gpu::source::tests::snapshot_file("range-gpu-node-1.json"),
        )
        .unwrap();
        let n = |k: &str| raw[k].as_u64().unwrap();
        let range = Range {
            start: n("start"),
            end: n("end"),
            step: n("step"),
            rate: n("rate"),
        };
        let queries = crate::gpu::promql::queries(r#"job="dcgm",hostname="gpu-node-1""#, 60);
        let results = raw["queries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|q| {
                let query = queries
                    .iter()
                    .find(|x| x.expr == q["expr"].as_str().unwrap())
                    .cloned()
                    .expect("снят текущий шаблон");
                (query, parse_matrix(&q["response"]["data"]))
            })
            .collect();
        (range, results)
    }

    #[test]
    fn a_captured_window_gives_a_series_per_card() {
        let (range, results) = captured();
        assert_eq!(results.len(), 14);
        let (series, partial, truncated) = assemble(&range, results);
        assert!(partial.is_empty() && !truncated);
        assert_eq!(
            series
                .iter()
                .map(|s| (s.host.as_str(), s.gpu))
                .collect::<Vec<_>>(),
            (0..8).map(|g| ("gpu-node-1", g)).collect::<Vec<_>>()
        );
        let points = &series[0].points;
        assert_eq!(points.len(), 61, "15 мин с шагом 15 с, концы включительно");
        assert_eq!(points[0].at, range.start * 1000);
        let last = points.iter().rev().find(|p| !p.values.is_empty()).unwrap();
        assert_eq!(last.values[&Field::MemTotal], 81559.0);
        assert!(last.values[&Field::SmActive] > 1.0 && last.values[&Field::SmActive] <= 100.0);
        assert!(last.values[&Field::Util] > 0.0 && last.values.contains_key(&Field::Power));
        assert_eq!(last.values[&Field::Throttle], 0.0);
        assert_eq!(last.values[&Field::PcieReplay], 0.0);
        assert!(!last.values.contains_key(&Field::MemTemp) || last.values[&Field::MemTemp] > 0.0);
    }

    #[test]
    fn a_failed_query_is_partial_and_many_cards_are_cut() {
        let (range, mut results) = captured();
        let err = || {
            AppError::localized(
                ErrorCode::Timeout,
                ltext!(
                    "errors.history.timeout",
                    "The history source did not answer in time"
                ),
            )
        };
        for (q, r) in results.iter_mut() {
            if matches!(q.field, Field::MemUsed | Field::MemFree) {
                *r = Err(err());
            }
        }
        let (series, partial, _) = assemble(&range, results);
        assert_eq!(
            partial.iter().map(|p| p.chart).collect::<Vec<_>>(),
            ["memory"]
        );
        assert!(
            series[0]
                .points
                .iter()
                .all(|p| !p.values.contains_key(&Field::MemTotal))
        );

        let one = |host: &str, gpu: u32| RangeSeries {
            labels: [
                ("instance".to_string(), format!("{host}:9400")),
                ("gpu".to_string(), gpu.to_string()),
            ]
            .into(),
            points: vec![(range.start, Some(1.0))],
        };
        let many: Vec<RangeSeries> = (0..20).map(|g| one("10.0.0.5", g)).collect();
        let q = crate::gpu::promql::queries("", 60).remove(0);
        let (series, _, truncated) = assemble(&range, vec![(q, Ok(many))]);
        assert!(truncated);
        assert_eq!(series.len(), MAX_SERIES);
        assert_eq!(series[0].host, "10.0.0.5", "хост из instance без порта");
        assert_eq!(series[0].points[0].values[&Field::Util], 1.0);
    }

    #[test]
    fn json_shape() {
        let p = GpuPoint {
            at: 1000,
            values: [(Field::Util, 50.0)].into(),
        };
        assert_eq!(
            serde_json::to_value(&p).unwrap(),
            serde_json::json!({ "at": 1000, "values": { "util": 50.0 } })
        );
    }
}
````

- [ ] **Шаг 6.2: Увидеть падение.**

Run: `cd src-tauri && cargo test --lib -q gpu`
Expected: FAIL — не компилируется: тестам не хватает типов и функций, которые появятся в шаге 3

- [ ] **Шаг 6.3: Реализация.** Шаблоны, сборка рядов, снятое окно Prometheus.

<!-- block: file t06-src-tauri-src-gpu-promql-rs step=6.3 path=src-tauri/src/gpu/promql.rs part=impl -->
````
//! Шаблоны PromQL окон истории GPU (спека I1d §4) — чистые функции над таблицей полей. Ряды
//! группируются по `hostname`, `Hostname`, `instance` и `gpu`: хост карты Rust берёт из ответа
//! по тому же порядку, что у снимка. Серии экземпляров MIG (`GPU_I_ID`) в окна не входят.
//! Сумма нескольких счётчиков (ошибки NVLink) — подзапросом: `increase` по каждому убрал бы имя
//! серии, и одинаковые наборы меток разных счётчиков столкнулись бы (Prometheus отвечает 422).

use crate::gpu::fields::{Combine, Field, Kind, PRESENCE, SPECS};
use crate::history::targets::escape;
use crate::history::window::duration;

/// Группировка рядов окна.
pub const BY: &str = "hostname, Hostname, instance, gpu";

/// Поля окон истории: графики вкладки GPU и счётчики проблем.
pub const HISTORY_FIELDS: [Field; 14] = [
    Field::Util,
    Field::SmActive,
    Field::MemUsed,
    Field::MemFree,
    Field::MemReserved,
    Field::Temp,
    Field::MemTemp,
    Field::Power,
    Field::SmClock,
    Field::MemClock,
    Field::Throttle,
    Field::EccDbe,
    Field::PcieReplay,
    Field::NvlinkErrors,
];

/// График вкладки GPU, которому принадлежит поле (для записи `partial`).
pub fn chart_of(field: Field) -> &'static str {
    match field {
        Field::Util | Field::SmActive => "util",
        Field::MemUsed | Field::MemFree | Field::MemReserved | Field::MemTotal => "memory",
        Field::Temp | Field::MemTemp => "temp",
        Field::Power => "power",
        Field::SmClock | Field::MemClock => "clocks",
        _ => "errors",
    }
}

/// Запрос окна: поле и выражение.
#[derive(Debug, Clone, PartialEq)]
pub struct GpuQuery {
    pub field: Field,
    pub expr: String,
}

/// Условия селектора серий карт: `matchers` (без скобок, может быть пустым) и запрет экземпляров
/// MIG.
fn selector(matchers: &str) -> String {
    if matchers.is_empty() {
        "GPU_I_ID=\"\"".into()
    } else {
        format!("{matchers},GPU_I_ID=\"\"")
    }
}

/// Запросы окна для условий `matchers` и интервала `rate_sec` (`rate` окна, I1c §4).
pub fn queries(matchers: &str, rate_sec: u64) -> Vec<GpuQuery> {
    let s = selector(matchers);
    let rate = duration(rate_sec);
    // Разрешение подзапроса суммы счётчиков: четыре точки на интервал.
    let resolution = duration((rate_sec / 4).max(1));
    HISTORY_FIELDS
        .iter()
        .map(|field| {
            let spec = SPECS
                .iter()
                .find(|x| x.field == *field)
                .expect("every history field is in the table");
            let one = |name: &str| match spec.kind {
                Kind::Gauge | Kind::Ratio => {
                    format!("avg by ({BY}) (avg_over_time({name}{{{s}}}[{rate}]))")
                }
                Kind::Mask => format!("max by ({BY}) (max_over_time({name}{{{s}}}[{rate}]))"),
                Kind::Counter => format!("sum by ({BY}) (increase({name}{{{s}}}[{rate}]))"),
            };
            let expr = match spec.combine {
                Combine::First => spec
                    .names
                    .iter()
                    .map(|n| one(n))
                    .collect::<Vec<_>>()
                    .join(" or "),
                Combine::Sum => format!(
                    "increase((sum by ({BY}) ({{__name__=~\"{}\",{s}}}))[{rate}:{resolution}])",
                    escape(&spec.names.join("|"))
                ),
            };
            GpuQuery {
                field: *field,
                expr,
            }
        })
        .collect()
}

/// Поиск хостов под базовым селектором окна: по какой метке подписан нужный хост.
pub fn hosts_expr(matchers: &str) -> String {
    format!("count by (hostname, Hostname, instance) ({PRESENCE}{{{matchers}}})")
}
````

<!-- block: file t06-src-tauri-src-gpu-series-rs step=6.3 path=src-tauri/src/gpu/series.rs part=impl -->
````
//! Сборка рядов окна истории GPU (спека I1d §4): ряд на карту, точка на каждый шаг окна, значения
//! полей — только те, что есть (на графике разрыв). Активность SM — в процентах, всего памяти —
//! сумма занятой, свободной и зарезервированной.

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;

use crate::error::AppResult;
use crate::gpu::fields::Field;
use crate::gpu::promql::{GpuQuery, chart_of};
use crate::gpu::source::instance_host;
use crate::history::client::RangeSeries;
use crate::history::series::Partial;
use crate::history::window::Range;

/// Запрос окна и его результат: ряды или ошибка.
pub type QueryResult = (GpuQuery, AppResult<Vec<RangeSeries>>);

/// Сколько рядов (карт) может быть в окне: на графике хоста — ряд на карту.
pub const MAX_SERIES: usize = 16;

/// Точка ряда: время (мс) и значения полей.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GpuPoint {
    pub at: u64,
    pub values: BTreeMap<Field, f64>,
}

/// Ряд карты в окне.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuSeries {
    pub host: String,
    pub gpu: u32,
    pub points: Vec<GpuPoint>,
}

/// Хост ряда — как у снимка: `hostname`, `Hostname`, иначе `instance` без порта.
pub fn series_host(labels: &BTreeMap<String, String>) -> Option<String> {
    let get = |k: &str| labels.get(k).filter(|v| !v.is_empty());
    get("hostname")
        .or_else(|| get("Hostname"))
        .cloned()
        .or_else(|| get("instance").map(|i| instance_host(i)))
}

/// Ряды окна `range` из результатов запросов: не больше `MAX_SERIES` карт (по хосту и номеру);
/// третий результат — карт было больше. Сбой запроса — запись `partial` его графика (одна на
/// график).
pub fn assemble(range: &Range, results: Vec<QueryResult>) -> (Vec<GpuSeries>, Vec<Partial>, bool) {
    // (хост, номер) → поле → время (с) → значение.
    let mut table: BTreeMap<(String, u32), HashMap<Field, HashMap<u64, f64>>> = BTreeMap::new();
    let mut partial: Vec<Partial> = Vec::new();
    for (query, result) in results {
        let series = match result {
            Ok(s) => s,
            Err(error) => {
                let chart = chart_of(query.field);
                if !partial.iter().any(|p| p.chart == chart) {
                    partial.push(Partial { chart, error });
                }
                continue;
            }
        };
        for s in series {
            let (Some(host), Some(gpu)) = (
                series_host(&s.labels),
                s.labels.get("gpu").and_then(|g| g.parse::<u32>().ok()),
            ) else {
                continue;
            };
            let field = table
                .entry((host, gpu))
                .or_default()
                .entry(query.field)
                .or_default();
            for (t, v) in s.points {
                if let Some(v) = v.filter(|v| v.is_finite()) {
                    let v = if query.field == Field::SmActive {
                        v * 100.0
                    } else {
                        v
                    };
                    field.insert(t, v);
                }
            }
        }
    }
    let truncated = table.len() > MAX_SERIES;
    let times: Vec<u64> = (range.start..=range.end)
        .step_by(range.step.max(1) as usize)
        .collect();
    let series = table
        .into_iter()
        .take(MAX_SERIES)
        .map(|((host, gpu), fields)| {
            let points = times
                .iter()
                .map(|t| {
                    let mut values: BTreeMap<Field, f64> = fields
                        .iter()
                        .filter_map(|(f, by_t)| by_t.get(t).map(|v| (*f, *v)))
                        .collect();
                    if let (Some(used), Some(free)) =
                        (values.get(&Field::MemUsed), values.get(&Field::MemFree))
                    {
                        let total =
                            used + free + values.get(&Field::MemReserved).copied().unwrap_or(0.0);
                        values.insert(Field::MemTotal, total);
                    }
                    GpuPoint {
                        at: t * 1000,
                        values,
                    }
                })
                .collect();
            GpuSeries { host, gpu, points }
        })
        .collect();
    (series, partial, truncated)
}
````

<!-- block: patch t06-src-tauri-src-gpu-source-rs step=6.3 -->
````
diff --git a/src-tauri/src/gpu/source.rs b/src-tauri/src/gpu/source.rs
index bd310b6..81ef90f 100644
--- a/src-tauri/src/gpu/source.rs
+++ b/src-tauri/src/gpu/source.rs
@@ -282,7 +282,7 @@ pub fn too_many_gpus(found: usize) -> AppError {
 }
 
 #[cfg(test)]
-mod tests {
+pub(crate) mod tests {
     use super::*;
     use crate::gpu::fields::Field;
     use crate::gpu::snapshot::tests::{DEFAULT, FULL};
````

<!-- block: patch t06-src-tauri-src-gpu-testdata-prometheus-3-15-1-readme-md step=6.3 -->
````
diff --git a/src-tauri/src/gpu/testdata/prometheus-3.15.1/README.md b/src-tauri/src/gpu/testdata/prometheus-3.15.1/README.md
index 9b338e2..bd78676 100644
--- a/src-tauri/src/gpu/testdata/prometheus-3.15.1/README.md
+++ b/src-tauri/src/gpu/testdata/prometheus-3.15.1/README.md
@@ -9,3 +9,7 @@
 | `instant.json` | мгновенный запрос карт `gpu::source::instant_expr` с селектором `{job="dcgm"}` |
 | `cards.json` | `count by (hostname, Hostname, instance, gpu) (DCGM_FI_DEV_FB_USED)` |
 | `targets.json` | `count by (job, instance) (DCGM_FI_DEV_FB_USED)` |
+| `range-gpu-node-1.json` | `query_range` шаблонов `gpu::promql::queries` для `job="dcgm",hostname="gpu-node-1"` (окно `now`: 15 мин, шаг 15 с, интервал 1m) |
+
+`range-gpu-node-1.json` — `{ start, end, step, rate, queries: [{ expr, response }] }`. Prometheus запущен примерно за
+6 мин до конца окна, поэтому первые точки окна пустые.
````

<!-- block: file t06-src-tauri-src-gpu-testdata-prometheus-3-15-1-range-gpu-node-1-json step=6.3 path=src-tauri/src/gpu/testdata/prometheus-3.15.1/range-gpu-node-1.json -->
````
{"start":1793555610,"end":1793556510,"step":15,"rate":60,"queries":[{"expr":"avg by (hostname, Hostname, instance, gpu) (avg_over_time(DCGM_FI_DEV_GPU_UTIL{job=\"dcgm\",hostname=\"gpu-node-1\",GPU_I_ID=\"\"}[1m]))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{"gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"47"],[1793556165,"64.6"],[1793556180,"75.5"],[1793556195,"77.27272727272727"],[1793556210,"75.91666666666667"],[1793556225,"74.5"],[1793556240,"74.25"],[1793556255,"75.33333333333333"],[1793556270,"76.16666666666667"],[1793556285,"74.58333333333333"],[1793556300,"74.08333333333333"],[1793556315,"74.91666666666667"],[1793556330,"76.75"],[1793556345,"75"],[1793556360,"74.08333333333333"],[1793556375,"74.66666666666667"],[1793556390,"76.33333333333333"],[1793556405,"75.08333333333333"],[1793556420,"74.16666666666667"],[1793556435,"74.5"],[1793556450,"76"],[1793556465,"75.33333333333333"],[1793556480,"74"],[1793556495,"74.16666666666667"],[1793556510,"75.5"]]},{"metric":{"gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"68.5"],[1793556165,"81.2"],[1793556180,"84.75"],[1793556195,"79.18181818181819"],[1793556210,"74.91666666666667"],[1793556225,"74.08333333333333"],[1793556240,"74.58333333333333"],[1793556255,"76.16666666666667"],[1793556270,"75.33333333333333"],[1793556285,"74.25"],[1793556300,"74.5"],[1793556315,"75.91666666666667"],[1793556330,"75.33333333333333"],[1793556345,"73.91666666666667"],[1793556360,"73.91666666666667"],[1793556375,"75.33333333333333"],[1793556390,"75.91666666666667"],[1793556405,"74.5"],[1793556420,"74.25"],[1793556435,"75.33333333333333"],[1793556450,"76.16666666666667"],[1793556465,"74.58333333333333"],[1793556480,"74.08333333333333"],[1793556495,"74.91666666666667"],[1793556510,"76.75"]]},{"metric":{"gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"85"],[1793556165,"90.4"],[1793556180,"85.75"],[1793556195,"75.9090909090909"],[1793556210,"74.16666666666667"],[1793556225,"74"],[1793556240,"75.33333333333333"],[1793556255,"76"],[1793556270,"74.5"],[1793556285,"74.16666666666667"],[1793556300,"75.08333333333333"],[1793556315,"76.33333333333333"],[1793556330,"74.66666666666667"],[1793556345,"74.08333333333333"],[1793556360,"75"],[1793556375,"76.75"],[1793556390,"74.91666666666667"],[1793556405,"74.08333333333333"],[1793556420,"74.58333333333333"],[1793556435,"76.16666666666667"],[1793556450,"75.33333333333333"],[1793556465,"74.25"],[1793556480,"74.5"],[1793556495,"75.91666666666667"],[1793556510,"75.33333333333333"]]},{"metric":{"gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"94"],[1793556165,"90.6"],[1793556180,"78.875"],[1793556195,"73.81818181818181"],[1793556210,"74.25"],[1793556225,"75"],[1793556240,"76.75"],[1793556255,"74.91666666666667"],[1793556270,"73.91666666666667"],[1793556285,"74.33333333333333"],[1793556300,"75.83333333333333"],[1793556315,"75.33333333333333"],[1793556330,"74.16666666666667"],[1793556345,"74.41666666666667"],[1793556360,"75.83333333333333"],[1793556375,"75.5"],[1793556390,"74.16666666666667"],[1793556405,"74"],[1793556420,"75.33333333333333"],[1793556435,"76"],[1793556450,"74.5"],[1793556465,"74.16666666666667"],[1793556480,"75.08333333333333"],[1793556495,"76.33333333333333"],[1793556510,"74.66666666666667"]]},{"metric":{"gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"93"],[1793556165,"81.8"],[1793556180,"69.875"],[1793556195,"72.36363636363636"],[1793556210,"74.25"],[1793556225,"75.66666666666667"],[1793556240,"75.83333333333333"],[1793556255,"74.41666666666667"],[1793556270,"74.33333333333333"],[1793556285,"75.5"],[1793556300,"76"],[1793556315,"74.41666666666667"],[1793556330,"74"],[1793556345,"74.91666666666667"],[1793556360,"76.58333333333333"],[1793556375,"74.91666666666667"],[1793556390,"74.25"],[1793556405,"75"],[1793556420,"76.75"],[1793556435,"74.91666666666667"],[1793556450,"73.91666666666667"],[1793556465,"74.33333333333333"],[1793556480,"75.83333333333333"],[1793556495,"75.33333333333333"],[1793556510,"74.16666666666667"]]},{"metric":{"gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"82.5"],[1793556165,"65.4"],[1793556180,"65.375"],[1793556195,"72.36363636363636"],[1793556210,"74.83333333333333"],[1793556225,"76.75"],[1793556240,"74.91666666666667"],[1793556255,"74.16666666666667"],[1793556270,"74.91666666666667"],[1793556285,"76.5"],[1793556300,"75.16666666666667"],[1793556315,"74.08333333333333"],[1793556330,"74.5"],[1793556345,"76"],[1793556360,"75.33333333333333"],[1793556375,"74.16666666666667"],[1793556390,"74.16666666666667"],[1793556405,"75.66666666666667"],[1793556420,"75.83333333333333"],[1793556435,"74.41666666666667"],[1793556450,"74.33333333333333"],[1793556465,"75.5"],[1793556480,"76"],[1793556495,"74.5"],[1793556510,"74.08333333333333"]]},{"metric":{"gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"64.5"],[1793556165,"57"],[1793556180,"66.5"],[1793556195,"73.9090909090909"],[1793556210,"76.08333333333333"],[1793556225,"75.5"],[1793556240,"74.25"],[1793556255,"74.25"],[1793556270,"75.5"],[1793556285,"75.91666666666667"],[1793556300,"74.41666666666667"],[1793556315,"74.25"],[1793556330,"75.33333333333333"],[1793556345,"76"],[1793556360,"74.5"],[1793556375,"74"],[1793556390,"74.83333333333333"],[1793556405,"76.75"],[1793556420,"74.91666666666667"],[1793556435,"74.16666666666667"],[1793556450,"74.91666666666667"],[1793556465,"76.5"],[1793556480,"75.16666666666667"],[1793556495,"74.08333333333333"],[1793556510,"74.5"]]},{"metric":{"gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"47"],[1793556165,"60.6"],[1793556180,"72.5"],[1793556195,"76.45454545454545"],[1793556210,"76.33333333333333"],[1793556225,"74.66666666666667"],[1793556240,"74.08333333333333"],[1793556255,"74.91666666666667"],[1793556270,"76.66666666666667"],[1793556285,"74.91666666666667"],[1793556300,"74.08333333333333"],[1793556315,"74.66666666666667"],[1793556330,"76.33333333333333"],[1793556345,"75.33333333333333"],[1793556360,"74.25"],[1793556375,"74.5"],[1793556390,"76.08333333333333"],[1793556405,"75.5"],[1793556420,"74.25"],[1793556435,"74.25"],[1793556450,"75.5"],[1793556465,"75.91666666666667"],[1793556480,"74.41666666666667"],[1793556495,"74.25"],[1793556510,"75.33333333333333"]]}]}}},{"expr":"avg by (hostname, Hostname, instance, gpu) (avg_over_time(DCGM_FI_PROF_SM_ACTIVE{job=\"dcgm\",hostname=\"gpu-node-1\",GPU_I_ID=\"\"}[1m])) or avg by (hostname, Hostname, instance, gpu) (avg_over_time(DCGM_FI_PROF_GR_ENGINE_ACTIVE{job=\"dcgm\",hostname=\"gpu-node-1\",GPU_I_ID=\"\"}[1m]))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{"gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0.41803599999999996"],[1793556165,"0.5974932"],[1793556180,"0.70615275"],[1793556195,"0.7233123636363636"],[1793556210,"0.7097834166666667"],[1793556225,"0.6949615"],[1793556240,"0.69227325"],[1793556255,"0.7031611666666667"],[1793556270,"0.7118648333333333"],[1793556285,"0.6955025"],[1793556300,"0.6901796666666667"],[1793556315,"0.6987528333333334"],[1793556330,"0.71662125"],[1793556345,"0.6990460833333333"],[1793556360,"0.6911955"],[1793556375,"0.69728225"],[1793556390,"0.7140401666666666"],[1793556405,"0.7016833333333334"],[1793556420,"0.69146175"],[1793556435,"0.6949401666666667"],[1793556450,"0.7102518333333333"],[1793556465,"0.70417"],[1793556480,"0.6917823333333333"],[1793556495,"0.6925829166666667"],[1793556510,"0.7061421666666666"]]},{"metric":{"gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0.635235"],[1793556165,"0.7622709999999999"],[1793556180,"0.797106875"],[1793556195,"0.7412470909090909"],[1793556210,"0.6987561666666666"],[1793556225,"0.6901868333333333"],[1793556240,"0.6955136666666667"],[1793556255,"0.7118781666666667"],[1793556270,"0.7031550833333333"],[1793556285,"0.6922705833333334"],[1793556300,"0.694963"],[1793556315,"0.7097875833333332"],[1793556330,"0.7035473333333333"],[1793556345,"0.6905655"],[1793556360,"0.6905696666666666"],[1793556375,"0.70355775"],[1793556390,"0.7097793333333334"],[1793556405,"0.69496"],[1793556420,"0.6922759166666667"],[1793556435,"0.7031673333333334"],[1793556450,"0.7118515833333334"],[1793556465,"0.6954914166666667"],[1793556480,"0.69017275"],[1793556495,"0.6987495833333334"],[1793556510,"0.7166195833333333"]]},{"metric":{"gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0.8016840000000001"],[1793556165,"0.8536964"],[1793556180,"0.808510375"],[1793556195,"0.7102600909090908"],[1793556210,"0.6925645833333333"],[1793556225,"0.6917725833333334"],[1793556240,"0.7041667500000001"],[1793556255,"0.71025025"],[1793556270,"0.6949435833333334"],[1793556285,"0.6914735"],[1793556300,"0.701702"],[1793556315,"0.7140204166666666"],[1793556330,"0.6972666666666667"],[1793556345,"0.6911879166666667"],[1793556360,"0.6990464166666667"],[1793556375,"0.7166247499999999"],[1793556390,"0.6987591666666666"],[1793556405,"0.6901935833333332"],[1793556420,"0.6955244166666668"],[1793556435,"0.7118911666666666"],[1793556450,"0.7031488333333333"],[1793556465,"0.69226775"],[1793556480,"0.6949644166666666"],[1793556495,"0.70979175"],[1793556510,"0.7035523333333333"]]},{"metric":{"gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0.8877685"],[1793556165,"0.8555031999999999"],[1793556180,"0.738334875"],[1793556195,"0.6873748181818181"],[1793556210,"0.6916065833333334"],[1793556225,"0.6987326666666667"],[1793556240,"0.715989"],[1793556255,"0.6986129999999999"],[1793556270,"0.6893436666666667"],[1793556285,"0.6939053333333334"],[1793556300,"0.7098501666666667"],[1793556315,"0.7040065833333333"],[1793556330,"0.6924813333333333"],[1793556345,"0.6943874166666667"],[1793556360,"0.7087017499999999"],[1793556375,"0.7061059166666667"],[1793556390,"0.6925557499999999"],[1793556405,"0.6917678333333334"],[1793556420,"0.7041651666666667"],[1793556435,"0.7102495000000001"],[1793556450,"0.69494525"],[1793556465,"0.69147925"],[1793556480,"0.7017115"],[1793556495,"0.7140105833333333"],[1793556510,"0.69725875"]]},{"metric":{"gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0.878172"],[1793556165,"0.7673694000000001"],[1793556180,"0.648829125"],[1793556195,"0.6735602727272727"],[1793556210,"0.69320825"],[1793556225,"0.7069846666666667"],[1793556240,"0.7080776666666666"],[1793556255,"0.6939794166666666"],[1793556270,"0.6923966666666667"],[1793556285,"0.7041784999999999"],[1793556300,"0.710094"],[1793556315,"0.6943290833333333"],[1793556330,"0.690083"],[1793556345,"0.6996345833333333"],[1793556360,"0.71554625"],[1793556375,"0.69843"],[1793556390,"0.6916058333333334"],[1793556405,"0.6987359166666667"],[1793556420,"0.7159939166666667"],[1793556435,"0.6985988333333334"],[1793556450,"0.68933325"],[1793556465,"0.693899"],[1793556480,"0.70984625"],[1793556495,"0.7040035"],[1793556510,"0.6924816666666667"]]},{"metric":{"gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0.7746014999999999"],[1793556165,"0.604976"],[1793556180,"0.6032379999999999"],[1793556195,"0.6727829090909091"],[1793556210,"0.6984674166666667"],[1793556225,"0.7164429999999999"],[1793556240,"0.6989920833333333"],[1793556255,"0.6914305000000001"],[1793556270,"0.6978159166666668"],[1793556285,"0.7147217499999999"],[1793556300,"0.7009286666666666"],[1793556315,"0.6909753333333333"],[1793556330,"0.6947640833333333"],[1793556345,"0.7102618333333334"],[1793556360,"0.7042459166666667"],[1793556375,"0.6921"],[1793556390,"0.6932161666666667"],[1793556405,"0.7069954166666667"],[1793556420,"0.7080688333333334"],[1793556435,"0.6939734999999999"],[1793556450,"0.6923949166666666"],[1793556465,"0.7041801666666667"],[1793556480,"0.7100965"],[1793556495,"0.6943338333333333"],[1793556510,"0.690092"]]},{"metric":{"gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0.5954845"],[1793556165,"0.5192932"],[1793556180,"0.615071"],[1793556195,"0.6891068181818181"],[1793556210,"0.7100471666666667"],[1793556225,"0.7038685833333332"],[1793556240,"0.6911198333333334"],[1793556255,"0.6914400833333333"],[1793556270,"0.7046575833333333"],[1793556285,"0.7094179166666668"],[1793556300,"0.6947993333333333"],[1793556315,"0.6924285833333333"],[1793556330,"0.7035779166666667"],[1793556345,"0.7108155833333334"],[1793556360,"0.6946194166666667"],[1793556375,"0.6896049166666667"],[1793556390,"0.6984628333333334"],[1793556405,"0.7164399166666666"],[1793556420,"0.6989909166666667"],[1793556435,"0.69143325"],[1793556450,"0.6978225833333332"],[1793556465,"0.7147304166666666"],[1793556480,"0.700918"],[1793556495,"0.6909682500000001"],[1793556510,"0.6947610833333333"]]},{"metric":{"gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0.418512"],[1793556165,"0.5554248"],[1793556180,"0.6747015"],[1793556195,"0.7141573636363637"],[1793556210,"0.7132356666666666"],[1793556225,"0.6966345"],[1793556240,"0.69085625"],[1793556255,"0.6990014166666667"],[1793556270,"0.71669925"],[1793556285,"0.6989459166666667"],[1793556300,"0.6906639166666667"],[1793556315,"0.6962975833333334"],[1793556330,"0.71282375"],[1793556345,"0.7026373333333333"],[1793556360,"0.6920170833333333"],[1793556375,"0.69502625"],[1793556390,"0.7100499166666667"],[1793556405,"0.7038720833333334"],[1793556420,"0.6911265833333333"],[1793556435,"0.6914510833333334"],[1793556450,"0.7046715833333334"],[1793556465,"0.7094124166666667"],[1793556480,"0.6947964166666667"],[1793556495,"0.6924298333333333"],[1793556510,"0.7035825833333332"]]}]}}},{"expr":"avg by (hostname, Hostname, instance, gpu) (avg_over_time(DCGM_FI_DEV_FB_USED{job=\"dcgm\",hostname=\"gpu-node-1\",GPU_I_ID=\"\"}[1m]))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{"gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"53986.5"],[1793556165,"59480.2"],[1793556180,"63572.5"],[1793556195,"65657.63636363637"],[1793556210,"67520.83333333333"],[1793556225,"65625.91666666667"],[1793556240,"63210.833333333336"],[1793556255,"62706.333333333336"],[1793556270,"64288.5"],[1793556285,"67210.16666666667"],[1793556300,"66405.5"],[1793556315,"63642.5"],[1793556330,"62643.5"],[1793556345,"63757.416666666664"],[1793556360,"66595.16666666667"],[1793556375,"67091.33333333333"],[1793556390,"64118.5"],[1793556405,"62645"],[1793556420,"63268.333333333336"],[1793556435,"65770.83333333333"],[1793556450,"67475.33333333333"],[1793556465,"64740.333333333336"],[1793556480,"62821.75"],[1793556495,"62941.833333333336"],[1793556510,"65058.75"]]},{"metric":{"gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"60476.5"],[1793556165,"64952.6"],[1793556180,"67492.25"],[1793556195,"67800.54545454546"],[1793556210,"66999.58333333333"],[1793556225,"63993.166666666664"],[1793556240,"62595.583333333336"],[1793556255,"63300.333333333336"],[1793556270,"65861.16666666667"],[1793556285,"67433.41666666667"],[1793556300,"64655.583333333336"],[1793556315,"62807.583333333336"],[1793556330,"63010.583333333336"],[1793556345,"65193.666666666664"],[1793556360,"67558.25"],[1793556375,"65346.833333333336"],[1793556390,"63085.5"],[1793556405,"62782.75"],[1793556420,"64544.5"],[1793556435,"67372"],[1793556450,"66048.25"],[1793556465,"63418.666666666664"],[1793556480,"62616.25"],[1793556495,"63921.333333333336"],[1793556510,"66878"]]},{"metric":{"gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"65995.5"],[1793556165,"68941.6"],[1793556180,"69638"],[1793556195,"68134.18181818182"],[1793556210,"65220.583333333336"],[1793556225,"63023.916666666664"],[1793556240,"62803.666666666664"],[1793556255,"64637.166666666664"],[1793556270,"67423.66666666667"],[1793556285,"65889.5"],[1793556300,"63316.833333333336"],[1793556315,"62595.333333333336"],[1793556330,"63977.166666666664"],[1793556345,"66979.5"],[1793556360,"66740.25"],[1793556375,"63842.75"],[1793556390,"62634.333333333336"],[1793556405,"63537.083333333336"],[1793556420,"66235.5"],[1793556435,"67296"],[1793556450,"64419.5"],[1793556465,"62748.333333333336"],[1793556480,"63153.666666666664"],[1793556495,"65494.083333333336"],[1793556510,"67545.58333333333"]]},{"metric":{"gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"69912"],[1793556165,"70991"],[1793556180,"69764.25"],[1793556195,"66620.63636363637"],[1793556210,"63772.75"],[1793556225,"62642.583333333336"],[1793556240,"63625.166666666664"],[1793556255,"66377.16666666667"],[1793556270,"67225.66666666667"],[1793556285,"64311.666666666664"],[1793556300,"62714.25"],[1793556315,"63201.916666666664"],[1793556330,"65604.33333333333"],[1793556345,"67526.08333333333"],[1793556360,"64881.25"],[1793556375,"62848.333333333336"],[1793556390,"62831.333333333336"],[1793556405,"64836"],[1793556420,"67515.58333333333"],[1793556435,"65647.66666666667"],[1793556450,"63219.916666666664"],[1793556465,"62698.333333333336"],[1793556480,"64265.083333333336"],[1793556495,"67194.25"],[1793556510,"66433.41666666667"]]},{"metric":{"gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"71778"],[1793556165,"70866.2"],[1793556180,"67856.75"],[1793556195,"64288"],[1793556210,"62827.833333333336"],[1793556225,"62893.583333333336"],[1793556240,"64966.416666666664"],[1793556255,"67538.58333333333"],[1793556270,"65540.41666666667"],[1793556285,"63174.333333333336"],[1793556300,"62735"],[1793556315,"64375.916666666664"],[1793556330,"67268.16666666667"],[1793556345,"66295"],[1793556360,"63574.333333333336"],[1793556375,"62638.666666666664"],[1793556390,"63814.833333333336"],[1793556405,"66692.16666666667"],[1793556420,"67019.25"],[1793556435,"64019.5"],[1793556450,"62606.166666666664"],[1793556465,"63294.083333333336"],[1793556480,"65843.08333333333"],[1793556495,"67442.75"],[1793556510,"64673.666666666664"]]},{"metric":{"gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"71379.5"],[1793556165,"68581.2"],[1793556180,"64133.875"],[1793556195,"62712.63636363636"],[1793556210,"62627"],[1793556225,"63881.25"],[1793556240,"66806.83333333333"],[1793556255,"66920.75"],[1793556270,"63945"],[1793556285,"62608.25"],[1793556300,"63377.416666666664"],[1793556315,"65983.75"],[1793556330,"67394.25"],[1793556345,"64583.416666666664"],[1793556360,"62792.083333333336"],[1793556375,"63060.833333333336"],[1793556390,"65295.75"],[1793556405,"67559.83333333333"],[1793556420,"65247.333333333336"],[1793556435,"63037.083333333336"],[1793556450,"62800"],[1793556465,"64618.583333333336"],[1793556480,"67413.66666666667"],[1793556495,"65922.83333333333"],[1793556510,"63338.166666666664"]]},{"metric":{"gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"68762.5"],[1793556165,"64398.2"],[1793556180,"60983"],[1793556195,"61993.818181818184"],[1793556210,"63119"],[1793556225,"65417.916666666664"],[1793556240,"67554"],[1793556255,"65115.166666666664"],[1793556270,"62970.75"],[1793556285,"62816.5"],[1793556300,"64706.083333333336"],[1793556315,"67458.91666666667"],[1793556330,"65808.75"],[1793556345,"63282.083333333336"],[1793556360,"62625.25"],[1793556375,"64067.583333333336"],[1793556390,"67054.5"],[1793556405,"66646.08333333333"],[1793556420,"63787.666666666664"],[1793556435,"62641.416666666664"],[1793556450,"63607.583333333336"],[1793556465,"66348.66666666667"],[1793556480,"67240.83333333333"],[1793556495,"64334.25"],[1793556510,"62721.583333333336"]]},{"metric":{"gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"64226"],[1793556165,"59102.6"],[1793556180,"59580.5"],[1793556195,"62150.27272727273"],[1793556210,"64189"],[1793556225,"67141.58333333333"],[1793556240,"66520.25"],[1793556255,"63712.5"],[1793556270,"62645.083333333336"],[1793556285,"63690.75"],[1793556300,"66484.41666666667"],[1793556315,"67164.08333333333"],[1793556330,"64221.166666666664"],[1793556345,"62682.916666666664"],[1793556360,"63235.75"],[1793556375,"65686.58333333333"],[1793556390,"67504.66666666667"],[1793556405,"64807.833333333336"],[1793556420,"62829.416666666664"],[1793556435,"62878.083333333336"],[1793556450,"64936.916666666664"],[1793556465,"67534.5"],[1793556480,"65562.91666666667"],[1793556495,"63184.083333333336"],[1793556510,"62727.833333333336"]]}]}}},{"expr":"avg by (hostname, Hostname, instance, gpu) (avg_over_time(DCGM_FI_DEV_FB_FREE{job=\"dcgm\",hostname=\"gpu-node-1\",GPU_I_ID=\"\"}[1m]))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{"gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"27031.5"],[1793556165,"21537.8"],[1793556180,"17445.5"],[1793556195,"15360.363636363636"],[1793556210,"13497.166666666666"],[1793556225,"15392.083333333334"],[1793556240,"17807.166666666668"],[1793556255,"18311.666666666668"],[1793556270,"16729.5"],[1793556285,"13807.833333333334"],[1793556300,"14612.5"],[1793556315,"17375.5"],[1793556330,"18374.5"],[1793556345,"17260.583333333332"],[1793556360,"14422.833333333334"],[1793556375,"13926.666666666666"],[1793556390,"16899.5"],[1793556405,"18373"],[1793556420,"17749.666666666668"],[1793556435,"15247.166666666666"],[1793556450,"13542.666666666666"],[1793556465,"16277.666666666666"],[1793556480,"18196.25"],[1793556495,"18076.166666666668"],[1793556510,"15959.25"]]},{"metric":{"gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"20541.5"],[1793556165,"16065.4"],[1793556180,"13525.75"],[1793556195,"13217.454545454546"],[1793556210,"14018.416666666666"],[1793556225,"17024.833333333332"],[1793556240,"18422.416666666668"],[1793556255,"17717.666666666668"],[1793556270,"15156.833333333334"],[1793556285,"13584.583333333334"],[1793556300,"16362.416666666666"],[1793556315,"18210.416666666668"],[1793556330,"18007.416666666668"],[1793556345,"15824.333333333334"],[1793556360,"13459.75"],[1793556375,"15671.166666666666"],[1793556390,"17932.5"],[1793556405,"18235.25"],[1793556420,"16473.5"],[1793556435,"13646"],[1793556450,"14969.75"],[1793556465,"17599.333333333332"],[1793556480,"18401.75"],[1793556495,"17096.666666666668"],[1793556510,"14140"]]},{"metric":{"gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"15022.5"],[1793556165,"12076.4"],[1793556180,"11380"],[1793556195,"12883.818181818182"],[1793556210,"15797.416666666666"],[1793556225,"17994.083333333332"],[1793556240,"18214.333333333332"],[1793556255,"16380.833333333334"],[1793556270,"13594.333333333334"],[1793556285,"15128.5"],[1793556300,"17701.166666666668"],[1793556315,"18422.666666666668"],[1793556330,"17040.833333333332"],[1793556345,"14038.5"],[1793556360,"14277.75"],[1793556375,"17175.25"],[1793556390,"18383.666666666668"],[1793556405,"17480.916666666668"],[1793556420,"14782.5"],[1793556435,"13722"],[1793556450,"16598.5"],[1793556465,"18269.666666666668"],[1793556480,"17864.333333333332"],[1793556495,"15523.916666666666"],[1793556510,"13472.416666666666"]]},{"metric":{"gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"11106"],[1793556165,"10027"],[1793556180,"11253.75"],[1793556195,"14397.363636363636"],[1793556210,"17245.25"],[1793556225,"18375.416666666668"],[1793556240,"17392.833333333332"],[1793556255,"14640.833333333334"],[1793556270,"13792.333333333334"],[1793556285,"16706.333333333332"],[1793556300,"18303.75"],[1793556315,"17816.083333333332"],[1793556330,"15413.666666666666"],[1793556345,"13491.916666666666"],[1793556360,"16136.75"],[1793556375,"18169.666666666668"],[1793556390,"18186.666666666668"],[1793556405,"16182"],[1793556420,"13502.416666666666"],[1793556435,"15370.333333333334"],[1793556450,"17798.083333333332"],[1793556465,"18319.666666666668"],[1793556480,"16752.916666666668"],[1793556495,"13823.75"],[1793556510,"14584.583333333334"]]},{"metric":{"gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"9240"],[1793556165,"10151.8"],[1793556180,"13161.25"],[1793556195,"16730"],[1793556210,"18190.166666666668"],[1793556225,"18124.416666666668"],[1793556240,"16051.583333333334"],[1793556255,"13479.416666666666"],[1793556270,"15477.583333333334"],[1793556285,"17843.666666666668"],[1793556300,"18283"],[1793556315,"16642.083333333332"],[1793556330,"13749.833333333334"],[1793556345,"14723"],[1793556360,"17443.666666666668"],[1793556375,"18379.333333333332"],[1793556390,"17203.166666666668"],[1793556405,"14325.833333333334"],[1793556420,"13998.75"],[1793556435,"16998.5"],[1793556450,"18411.833333333332"],[1793556465,"17723.916666666668"],[1793556480,"15174.916666666666"],[1793556495,"13575.25"],[1793556510,"16344.333333333334"]]},{"metric":{"gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"9638.5"],[1793556165,"12436.8"],[1793556180,"16884.125"],[1793556195,"18305.363636363636"],[1793556210,"18391"],[1793556225,"17136.75"],[1793556240,"14211.166666666666"],[1793556255,"14097.25"],[1793556270,"17073"],[1793556285,"18409.75"],[1793556300,"17640.583333333332"],[1793556315,"15034.25"],[1793556330,"13623.75"],[1793556345,"16434.583333333332"],[1793556360,"18225.916666666668"],[1793556375,"17957.166666666668"],[1793556390,"15722.25"],[1793556405,"13458.166666666666"],[1793556420,"15770.666666666666"],[1793556435,"17980.916666666668"],[1793556450,"18218"],[1793556465,"16399.416666666668"],[1793556480,"13604.333333333334"],[1793556495,"15095.166666666666"],[1793556510,"17679.833333333332"]]},{"metric":{"gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"12255.5"],[1793556165,"16619.8"],[1793556180,"20035"],[1793556195,"19024.18181818182"],[1793556210,"17899"],[1793556225,"15600.083333333334"],[1793556240,"13464"],[1793556255,"15902.833333333334"],[1793556270,"18047.25"],[1793556285,"18201.5"],[1793556300,"16311.916666666666"],[1793556315,"13559.083333333334"],[1793556330,"15209.25"],[1793556345,"17735.916666666668"],[1793556360,"18392.75"],[1793556375,"16950.416666666668"],[1793556390,"13963.5"],[1793556405,"14371.916666666666"],[1793556420,"17230.333333333332"],[1793556435,"18376.583333333332"],[1793556450,"17410.416666666668"],[1793556465,"14669.333333333334"],[1793556480,"13777.166666666666"],[1793556495,"16683.75"],[1793556510,"18296.416666666668"]]},{"metric":{"gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"16792"],[1793556165,"21915.4"],[1793556180,"21437.5"],[1793556195,"18867.727272727272"],[1793556210,"16829"],[1793556225,"13876.416666666666"],[1793556240,"14497.75"],[1793556255,"17305.5"],[1793556270,"18372.916666666668"],[1793556285,"17327.25"],[1793556300,"14533.583333333334"],[1793556315,"13853.916666666666"],[1793556330,"16796.833333333332"],[1793556345,"18335.083333333332"],[1793556360,"17782.25"],[1793556375,"15331.416666666666"],[1793556390,"13513.333333333334"],[1793556405,"16210.166666666666"],[1793556420,"18188.583333333332"],[1793556435,"18139.916666666668"],[1793556450,"16081.083333333334"],[1793556465,"13483.5"],[1793556480,"15455.083333333334"],[1793556495,"17833.916666666668"],[1793556510,"18290.166666666668"]]}]}}},{"expr":"avg by (hostname, Hostname, instance, gpu) (avg_over_time(DCGM_FI_DEV_FB_RESERVED{job=\"dcgm\",hostname=\"gpu-node-1\",GPU_I_ID=\"\"}[1m]))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{"gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"541"],[1793556165,"541"],[1793556180,"541"],[1793556195,"541"],[1793556210,"541"],[1793556225,"541"],[1793556240,"541"],[1793556255,"541"],[1793556270,"541"],[1793556285,"541"],[1793556300,"541"],[1793556315,"541"],[1793556330,"541"],[1793556345,"541"],[1793556360,"541"],[1793556375,"541"],[1793556390,"541"],[1793556405,"541"],[1793556420,"541"],[1793556435,"541"],[1793556450,"541"],[1793556465,"541"],[1793556480,"541"],[1793556495,"541"],[1793556510,"541"]]},{"metric":{"gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"541"],[1793556165,"541"],[1793556180,"541"],[1793556195,"541"],[1793556210,"541"],[1793556225,"541"],[1793556240,"541"],[1793556255,"541"],[1793556270,"541"],[1793556285,"541"],[1793556300,"541"],[1793556315,"541"],[1793556330,"541"],[1793556345,"541"],[1793556360,"541"],[1793556375,"541"],[1793556390,"541"],[1793556405,"541"],[1793556420,"541"],[1793556435,"541"],[1793556450,"541"],[1793556465,"541"],[1793556480,"541"],[1793556495,"541"],[1793556510,"541"]]},{"metric":{"gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"541"],[1793556165,"541"],[1793556180,"541"],[1793556195,"541"],[1793556210,"541"],[1793556225,"541"],[1793556240,"541"],[1793556255,"541"],[1793556270,"541"],[1793556285,"541"],[1793556300,"541"],[1793556315,"541"],[1793556330,"541"],[1793556345,"541"],[1793556360,"541"],[1793556375,"541"],[1793556390,"541"],[1793556405,"541"],[1793556420,"541"],[1793556435,"541"],[1793556450,"541"],[1793556465,"541"],[1793556480,"541"],[1793556495,"541"],[1793556510,"541"]]},{"metric":{"gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"541"],[1793556165,"541"],[1793556180,"541"],[1793556195,"541"],[1793556210,"541"],[1793556225,"541"],[1793556240,"541"],[1793556255,"541"],[1793556270,"541"],[1793556285,"541"],[1793556300,"541"],[1793556315,"541"],[1793556330,"541"],[1793556345,"541"],[1793556360,"541"],[1793556375,"541"],[1793556390,"541"],[1793556405,"541"],[1793556420,"541"],[1793556435,"541"],[1793556450,"541"],[1793556465,"541"],[1793556480,"541"],[1793556495,"541"],[1793556510,"541"]]},{"metric":{"gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"541"],[1793556165,"541"],[1793556180,"541"],[1793556195,"541"],[1793556210,"541"],[1793556225,"541"],[1793556240,"541"],[1793556255,"541"],[1793556270,"541"],[1793556285,"541"],[1793556300,"541"],[1793556315,"541"],[1793556330,"541"],[1793556345,"541"],[1793556360,"541"],[1793556375,"541"],[1793556390,"541"],[1793556405,"541"],[1793556420,"541"],[1793556435,"541"],[1793556450,"541"],[1793556465,"541"],[1793556480,"541"],[1793556495,"541"],[1793556510,"541"]]},{"metric":{"gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"541"],[1793556165,"541"],[1793556180,"541"],[1793556195,"541"],[1793556210,"541"],[1793556225,"541"],[1793556240,"541"],[1793556255,"541"],[1793556270,"541"],[1793556285,"541"],[1793556300,"541"],[1793556315,"541"],[1793556330,"541"],[1793556345,"541"],[1793556360,"541"],[1793556375,"541"],[1793556390,"541"],[1793556405,"541"],[1793556420,"541"],[1793556435,"541"],[1793556450,"541"],[1793556465,"541"],[1793556480,"541"],[1793556495,"541"],[1793556510,"541"]]},{"metric":{"gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"541"],[1793556165,"541"],[1793556180,"541"],[1793556195,"541"],[1793556210,"541"],[1793556225,"541"],[1793556240,"541"],[1793556255,"541"],[1793556270,"541"],[1793556285,"541"],[1793556300,"541"],[1793556315,"541"],[1793556330,"541"],[1793556345,"541"],[1793556360,"541"],[1793556375,"541"],[1793556390,"541"],[1793556405,"541"],[1793556420,"541"],[1793556435,"541"],[1793556450,"541"],[1793556465,"541"],[1793556480,"541"],[1793556495,"541"],[1793556510,"541"]]},{"metric":{"gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"541"],[1793556165,"541"],[1793556180,"541"],[1793556195,"541"],[1793556210,"541"],[1793556225,"541"],[1793556240,"541"],[1793556255,"541"],[1793556270,"541"],[1793556285,"541"],[1793556300,"541"],[1793556315,"541"],[1793556330,"541"],[1793556345,"541"],[1793556360,"541"],[1793556375,"541"],[1793556390,"541"],[1793556405,"541"],[1793556420,"541"],[1793556435,"541"],[1793556450,"541"],[1793556465,"541"],[1793556480,"541"],[1793556495,"541"],[1793556510,"541"]]}]}}},{"expr":"avg by (hostname, Hostname, instance, gpu) (avg_over_time(DCGM_FI_DEV_GPU_TEMP{job=\"dcgm\",hostname=\"gpu-node-1\",GPU_I_ID=\"\"}[1m]))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{"gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"47"],[1793556165,"52.8"],[1793556180,"57.75"],[1793556195,"61"],[1793556210,"65"],[1793556225,"65.75"],[1793556240,"61.583333333333336"],[1793556255,"57.666666666666664"],[1793556270,"56.666666666666664"],[1793556285,"59"],[1793556300,"63.833333333333336"],[1793556315,"65.91666666666667"],[1793556330,"63"],[1793556345,"58.25"],[1793556360,"56.333333333333336"],[1793556375,"57.833333333333336"],[1793556390,"62.25"],[1793556405,"65.91666666666667"],[1793556420,"64.41666666666667"],[1793556435,"59.416666666666664"],[1793556450,"56.666666666666664"],[1793556465,"57.25"],[1793556480,"61"],[1793556495,"65.66666666666667"],[1793556510,"65.58333333333333"]]},{"metric":{"gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"54"],[1793556165,"59"],[1793556180,"62.625"],[1793556195,"64.36363636363636"],[1793556210,"65.91666666666667"],[1793556225,"63.833333333333336"],[1793556240,"59"],[1793556255,"56.666666666666664"],[1793556270,"57.666666666666664"],[1793556285,"61.583333333333336"],[1793556300,"65.75"],[1793556315,"65"],[1793556330,"60"],[1793556345,"56.75"],[1793556360,"56.75"],[1793556375,"60"],[1793556390,"65"],[1793556405,"65.75"],[1793556420,"61.583333333333336"],[1793556435,"57.666666666666664"],[1793556450,"56.666666666666664"],[1793556465,"59"],[1793556480,"63.833333333333336"],[1793556495,"65.91666666666667"],[1793556510,"63"]]},{"metric":{"gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"60"],[1793556165,"64.2"],[1793556180,"66.375"],[1793556195,"66.54545454545455"],[1793556210,"65.66666666666667"],[1793556225,"61"],[1793556240,"57.25"],[1793556255,"56.666666666666664"],[1793556270,"59.416666666666664"],[1793556285,"64.41666666666667"],[1793556300,"65.91666666666667"],[1793556315,"62.25"],[1793556330,"57.833333333333336"],[1793556345,"56.333333333333336"],[1793556360,"58.25"],[1793556375,"63"],[1793556390,"65.91666666666667"],[1793556405,"63.833333333333336"],[1793556420,"59"],[1793556435,"56.666666666666664"],[1793556450,"57.666666666666664"],[1793556465,"61.583333333333336"],[1793556480,"65.66666666666667"],[1793556495,"64.91666666666667"],[1793556510,"59.916666666666664"]]},{"metric":{"gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"65"],[1793556165,"67.4"],[1793556180,"68"],[1793556195,"66.54545454545455"],[1793556210,"63.166666666666664"],[1793556225,"58.416666666666664"],[1793556240,"56.416666666666664"],[1793556255,"57.833333333333336"],[1793556270,"62.25"],[1793556285,"65.91666666666667"],[1793556300,"64.5"],[1793556315,"59.5"],[1793556330,"56.666666666666664"],[1793556345,"57.25"],[1793556360,"60.916666666666664"],[1793556375,"65.58333333333333"],[1793556390,"65.66666666666667"],[1793556405,"61"],[1793556420,"57.25"],[1793556435,"56.666666666666664"],[1793556450,"59.416666666666664"],[1793556465,"64.41666666666667"],[1793556480,"65.91666666666667"],[1793556495,"62.25"],[1793556510,"57.833333333333336"]]},{"metric":{"gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"68.5"],[1793556165,"69.2"],[1793556180,"68"],[1793556195,"65"],[1793556210,"60.083333333333336"],[1793556225,"56.833333333333336"],[1793556240,"56.75"],[1793556255,"59.916666666666664"],[1793556270,"64.91666666666667"],[1793556285,"65.75"],[1793556300,"61.583333333333336"],[1793556315,"57.583333333333336"],[1793556330,"56.583333333333336"],[1793556345,"58.833333333333336"],[1793556360,"63.833333333333336"],[1793556375,"66.08333333333333"],[1793556390,"63.166666666666664"],[1793556405,"58.416666666666664"],[1793556420,"56.416666666666664"],[1793556435,"57.833333333333336"],[1793556450,"62.25"],[1793556465,"65.91666666666667"],[1793556480,"64.5"],[1793556495,"59.5"],[1793556510,"56.666666666666664"]]},{"metric":{"gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"70"],[1793556165,"69.2"],[1793556180,"66.25"],[1793556195,"62"],[1793556210,"58.166666666666664"],[1793556225,"56.5"],[1793556240,"58.333333333333336"],[1793556255,"62.916666666666664"],[1793556270,"65.83333333333333"],[1793556285,"63.833333333333336"],[1793556300,"58.833333333333336"],[1793556315,"56.5"],[1793556330,"57.5"],[1793556345,"61.416666666666664"],[1793556360,"65.75"],[1793556375,"65.08333333333333"],[1793556390,"60.083333333333336"],[1793556405,"56.833333333333336"],[1793556420,"56.75"],[1793556435,"59.916666666666664"],[1793556450,"64.91666666666667"],[1793556465,"65.75"],[1793556480,"61.583333333333336"],[1793556495,"57.583333333333336"],[1793556510,"56.583333333333336"]]},{"metric":{"gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"69.5"],[1793556165,"66.6"],[1793556180,"62.5"],[1793556195,"58.63636363636363"],[1793556210,"56.75"],[1793556225,"57.25"],[1793556240,"60.833333333333336"],[1793556255,"65.5"],[1793556270,"65.58333333333333"],[1793556285,"61"],[1793556300,"57.25"],[1793556315,"56.666666666666664"],[1793556330,"59.416666666666664"],[1793556345,"64.5"],[1793556360,"66.08333333333333"],[1793556375,"62.583333333333336"],[1793556390,"58.166666666666664"],[1793556405,"56.5"],[1793556420,"58.333333333333336"],[1793556435,"62.916666666666664"],[1793556450,"65.83333333333333"],[1793556465,"63.833333333333336"],[1793556480,"58.833333333333336"],[1793556495,"56.5"],[1793556510,"57.5"]]},{"metric":{"gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"67"],[1793556165,"62.8"],[1793556180,"57.625"],[1793556195,"56.36363636363637"],[1793556210,"56.416666666666664"],[1793556225,"58.666666666666664"],[1793556240,"63.666666666666664"],[1793556255,"66.16666666666667"],[1793556270,"63.25"],[1793556285,"58.5"],[1793556300,"56.5"],[1793556315,"57.666666666666664"],[1793556330,"62.083333333333336"],[1793556345,"65.75"],[1793556360,"64.41666666666667"],[1793556375,"59.583333333333336"],[1793556390,"56.75"],[1793556405,"57.25"],[1793556420,"60.833333333333336"],[1793556435,"65.5"],[1793556450,"65.58333333333333"],[1793556465,"61"],[1793556480,"57.25"],[1793556495,"56.666666666666664"],[1793556510,"59.416666666666664"]]}]}}},{"expr":"avg by (hostname, Hostname, instance, gpu) (avg_over_time(DCGM_FI_DEV_MEMORY_TEMP{job=\"dcgm\",hostname=\"gpu-node-1\",GPU_I_ID=\"\"}[1m]))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{"gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"49"],[1793556165,"51.2"],[1793556180,"53"],[1793556195,"54.36363636363637"],[1793556210,"55.916666666666664"],[1793556225,"56.25"],[1793556240,"54.583333333333336"],[1793556255,"53"],[1793556270,"52.666666666666664"],[1793556285,"53.583333333333336"],[1793556300,"55.666666666666664"],[1793556315,"56.5"],[1793556330,"55.25"],[1793556345,"53.416666666666664"],[1793556360,"52.666666666666664"],[1793556375,"53.25"],[1793556390,"55.083333333333336"],[1793556405,"56.5"],[1793556420,"55.916666666666664"],[1793556435,"53.916666666666664"],[1793556450,"52.75"],[1793556465,"52.916666666666664"],[1793556480,"54.333333333333336"],[1793556495,"56.166666666666664"],[1793556510,"56.166666666666664"]]},{"metric":{"gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"51.5"],[1793556165,"53.6"],[1793556180,"55.125"],[1793556195,"55.81818181818182"],[1793556210,"56.5"],[1793556225,"55.666666666666664"],[1793556240,"53.583333333333336"],[1793556255,"52.666666666666664"],[1793556270,"53"],[1793556285,"54.583333333333336"],[1793556300,"56.25"],[1793556315,"55.916666666666664"],[1793556330,"54"],[1793556345,"52.666666666666664"],[1793556360,"52.666666666666664"],[1793556375,"54"],[1793556390,"55.916666666666664"],[1793556405,"56.25"],[1793556420,"54.583333333333336"],[1793556435,"53"],[1793556450,"52.666666666666664"],[1793556465,"53.583333333333336"],[1793556480,"55.666666666666664"],[1793556495,"56.5"],[1793556510,"55.25"]]},{"metric":{"gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"54"],[1793556165,"55.6"],[1793556180,"56.5"],[1793556195,"56.54545454545455"],[1793556210,"56.166666666666664"],[1793556225,"54.333333333333336"],[1793556240,"52.916666666666664"],[1793556255,"52.75"],[1793556270,"53.916666666666664"],[1793556285,"55.916666666666664"],[1793556300,"56.5"],[1793556315,"55"],[1793556330,"53.166666666666664"],[1793556345,"52.583333333333336"],[1793556360,"53.333333333333336"],[1793556375,"55.25"],[1793556390,"56.5"],[1793556405,"55.666666666666664"],[1793556420,"53.583333333333336"],[1793556435,"52.666666666666664"],[1793556450,"53"],[1793556465,"54.583333333333336"],[1793556480,"56.25"],[1793556495,"55.916666666666664"],[1793556510,"54"]]},{"metric":{"gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"56.5"],[1793556165,"57.2"],[1793556180,"57.375"],[1793556195,"56.72727272727273"],[1793556210,"55.25"],[1793556225,"53.333333333333336"],[1793556240,"52.583333333333336"],[1793556255,"53.166666666666664"],[1793556270,"55"],[1793556285,"56.5"],[1793556300,"55.916666666666664"],[1793556315,"53.916666666666664"],[1793556330,"52.75"],[1793556345,"52.916666666666664"],[1793556360,"54.333333333333336"],[1793556375,"56.166666666666664"],[1793556390,"56.166666666666664"],[1793556405,"54.333333333333336"],[1793556420,"52.916666666666664"],[1793556435,"52.75"],[1793556450,"53.916666666666664"],[1793556465,"55.916666666666664"],[1793556480,"56.5"],[1793556495,"55"],[1793556510,"53.166666666666664"]]},{"metric":{"gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"57.5"],[1793556165,"57.8"],[1793556180,"57.125"],[1793556195,"55.90909090909091"],[1793556210,"54"],[1793556225,"52.666666666666664"],[1793556240,"52.666666666666664"],[1793556255,"54"],[1793556270,"55.916666666666664"],[1793556285,"56.25"],[1793556300,"54.583333333333336"],[1793556315,"52.916666666666664"],[1793556330,"52.583333333333336"],[1793556345,"53.5"],[1793556360,"55.583333333333336"],[1793556375,"56.5"],[1793556390,"55.25"],[1793556405,"53.333333333333336"],[1793556420,"52.583333333333336"],[1793556435,"53.166666666666664"],[1793556450,"55"],[1793556465,"56.5"],[1793556480,"55.916666666666664"],[1793556495,"53.916666666666664"],[1793556510,"52.75"]]},{"metric":{"gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"58"],[1793556165,"57.6"],[1793556180,"56.5"],[1793556195,"54.81818181818182"],[1793556210,"53.25"],[1793556225,"52.666666666666664"],[1793556240,"53.416666666666664"],[1793556255,"55.25"],[1793556270,"56.5"],[1793556285,"55.666666666666664"],[1793556300,"53.583333333333336"],[1793556315,"52.666666666666664"],[1793556330,"53"],[1793556345,"54.583333333333336"],[1793556360,"56.25"],[1793556375,"55.916666666666664"],[1793556390,"54"],[1793556405,"52.666666666666664"],[1793556420,"52.666666666666664"],[1793556435,"54"],[1793556450,"55.916666666666664"],[1793556465,"56.25"],[1793556480,"54.583333333333336"],[1793556495,"52.916666666666664"],[1793556510,"52.583333333333336"]]},{"metric":{"gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"58"],[1793556165,"56.8"],[1793556180,"55.125"],[1793556195,"53.54545454545455"],[1793556210,"52.75"],[1793556225,"52.916666666666664"],[1793556240,"54.333333333333336"],[1793556255,"56.166666666666664"],[1793556270,"56.25"],[1793556285,"54.416666666666664"],[1793556300,"52.916666666666664"],[1793556315,"52.75"],[1793556330,"53.75"],[1793556345,"55.75"],[1793556360,"56.416666666666664"],[1793556375,"55"],[1793556390,"53.25"],[1793556405,"52.666666666666664"],[1793556420,"53.416666666666664"],[1793556435,"55.25"],[1793556450,"56.5"],[1793556465,"55.666666666666664"],[1793556480,"53.583333333333336"],[1793556495,"52.666666666666664"],[1793556510,"53"]]},{"metric":{"gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"56.5"],[1793556165,"55"],[1793556180,"53"],[1793556195,"52.54545454545455"],[1793556210,"52.5"],[1793556225,"53.416666666666664"],[1793556240,"55.416666666666664"],[1793556255,"56.333333333333336"],[1793556270,"55.25"],[1793556285,"53.333333333333336"],[1793556300,"52.5"],[1793556315,"53.083333333333336"],[1793556330,"54.916666666666664"],[1793556345,"56.416666666666664"],[1793556360,"55.916666666666664"],[1793556375,"53.916666666666664"],[1793556390,"52.75"],[1793556405,"52.916666666666664"],[1793556420,"54.333333333333336"],[1793556435,"56.166666666666664"],[1793556450,"56.25"],[1793556465,"54.416666666666664"],[1793556480,"52.916666666666664"],[1793556495,"52.75"],[1793556510,"53.75"]]}]}}},{"expr":"avg by (hostname, Hostname, instance, gpu) (avg_over_time(DCGM_FI_DEV_POWER_USAGE{job=\"dcgm\",hostname=\"gpu-node-1\",GPU_I_ID=\"\"}[1m]))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{"gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"235.666"],[1793556165,"382.4944"],[1793556180,"471.39775000000003"],[1793556195,"485.43736363636367"],[1793556210,"474.3681666666667"],[1793556225,"462.24116666666663"],[1793556240,"460.0416666666667"],[1793556255,"468.95008333333334"],[1793556270,"476.0713333333333"],[1793556285,"462.68399999999997"],[1793556300,"458.32891666666666"],[1793556315,"465.34325"],[1793556330,"479.9628333333333"],[1793556345,"465.5830833333333"],[1793556360,"459.1598333333333"],[1793556375,"464.1399166666667"],[1793556390,"477.8509166666667"],[1793556405,"467.74083333333334"],[1793556420,"459.37783333333334"],[1793556435,"462.22375"],[1793556450,"474.7515833333334"],[1793556465,"469.7755"],[1793556480,"459.64008333333334"],[1793556495,"460.29516666666666"],[1793556510,"471.389"]]},{"metric":{"gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"413.374"],[1793556165,"517.3126"],[1793556180,"545.814625"],[1793556195,"500.1111818181818"],[1793556210,"465.3459166666667"],[1793556225,"458.33466666666664"],[1793556240,"462.693"],[1793556255,"476.08216666666664"],[1793556270,"468.9450833333333"],[1793556285,"460.0395833333333"],[1793556300,"462.2425"],[1793556315,"474.37166666666667"],[1793556330,"469.26608333333337"],[1793556345,"458.64458333333334"],[1793556360,"458.6480833333333"],[1793556375,"469.27458333333334"],[1793556390,"474.36483333333337"],[1793556405,"462.23991666666666"],[1793556420,"460.04383333333334"],[1793556435,"468.95508333333333"],[1793556450,"476.06050000000005"],[1793556465,"462.67491666666666"],[1793556480,"458.32325000000003"],[1793556495,"465.34075"],[1793556510,"479.9615833333333"]]},{"metric":{"gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"549.5595000000001"],[1793556165,"592.1154"],[1793556180,"555.145"],[1793556195,"474.75836363636364"],[1793556210,"460.28025"],[1793556225,"459.6321666666667"],[1793556240,"469.7728333333333"],[1793556255,"474.75025"],[1793556270,"462.22658333333334"],[1793556285,"459.38733333333334"],[1793556300,"467.7561666666667"],[1793556315,"477.83483333333334"],[1793556330,"464.12716666666665"],[1793556345,"459.15366666666665"],[1793556360,"465.5833333333333"],[1793556375,"479.96566666666666"],[1793556390,"465.34841666666665"],[1793556405,"458.3403333333333"],[1793556420,"462.7018333333333"],[1793556435,"476.09275"],[1793556450,"468.9399166666667"],[1793556465,"460.0371666666666"],[1793556480,"462.24358333333333"],[1793556495,"474.375"],[1793556510,"469.27"]]},{"metric":{"gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"619.9925000000001"],[1793556165,"593.5934"],[1793556180,"497.728375"],[1793556195,"456.0338181818182"],[1793556210,"459.49608333333333"],[1793556225,"465.3265"],[1793556240,"479.44533333333334"],[1793556255,"465.2285833333333"],[1793556270,"457.64458333333334"],[1793556285,"461.37691666666666"],[1793556300,"474.42266666666666"],[1793556315,"469.64149999999995"],[1793556330,"460.2119166666667"],[1793556345,"461.77133333333336"],[1793556360,"473.48316666666665"],[1793556375,"471.3593333333333"],[1793556390,"460.2726666666667"],[1793556405,"459.62808333333334"],[1793556420,"469.77133333333336"],[1793556435,"474.74949999999995"],[1793556450,"462.22791666666666"],[1793556465,"459.39216666666664"],[1793556480,"467.76391666666666"],[1793556495,"477.8268333333333"],[1793556510,"464.12075000000004"]]},{"metric":{"gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"612.1405"],[1793556165,"521.4838"],[1793556180,"424.496375"],[1793556195,"444.731"],[1793556210,"460.8065833333333"],[1793556225,"472.07824999999997"],[1793556240,"472.9725833333333"],[1793556255,"461.43775"],[1793556270,"460.14275"],[1793556285,"469.78249999999997"],[1793556300,"474.6223333333333"],[1793556315,"461.7236666666667"],[1793556330,"458.24974999999995"],[1793556345,"466.06466666666665"],[1793556360,"479.08341666666666"],[1793556375,"465.07916666666665"],[1793556390,"459.49583333333334"],[1793556405,"465.32941666666665"],[1793556420,"479.4495833333333"],[1793556435,"465.21725"],[1793556450,"457.63625"],[1793556465,"461.3719166666667"],[1793556480,"474.41966666666667"],[1793556495,"469.63941666666665"],[1793556510,"460.2124166666667"]]},{"metric":{"gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"527.4010000000001"],[1793556165,"388.6166"],[1793556180,"387.1945"],[1793556195,"444.095"],[1793556210,"465.10974999999996"],[1793556225,"479.8170833333333"],[1793556240,"465.53925000000004"],[1793556255,"459.35241666666667"],[1793556270,"464.57675"],[1793556285,"478.40883333333335"],[1793556300,"467.1234166666667"],[1793556315,"458.97974999999997"],[1793556330,"462.07958333333335"],[1793556345,"474.7595"],[1793556360,"469.8374166666667"],[1793556375,"459.8999166666666"],[1793556390,"460.81325"],[1793556405,"472.08725"],[1793556420,"472.9655"],[1793556435,"461.43291666666664"],[1793556450,"460.14133333333336"],[1793556465,"469.78375000000005"],[1793556480,"474.6245"],[1793556495,"461.72783333333336"],[1793556510,"458.2571666666667"]]},{"metric":{"gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"380.8515"],[1793556165,"318.51300000000003"],[1793556180,"396.8765"],[1793556195,"457.4511818181818"],[1793556210,"474.5840833333333"],[1793556225,"469.52875"],[1793556240,"459.09791666666666"],[1793556255,"459.36"],[1793556270,"470.1743333333333"],[1793556285,"474.06925"],[1793556300,"462.1086666666667"],[1793556315,"460.16883333333334"],[1793556330,"469.2909166666667"],[1793556345,"475.2125833333333"],[1793556360,"461.96116666666666"],[1793556375,"457.8585"],[1793556390,"465.1059166666667"],[1793556405,"479.81441666666666"],[1793556420,"465.53791666666666"],[1793556435,"459.35425000000004"],[1793556450,"464.5819166666667"],[1793556465,"478.41558333333336"],[1793556480,"467.11449999999996"],[1793556495,"458.97383333333335"],[1793556510,"462.07708333333335"]]},{"metric":{"gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"236.055"],[1793556165,"348.07460000000003"],[1793556180,"445.66475"],[1793556195,"477.9469090909091"],[1793556210,"477.1929166666667"],[1793556225,"463.61016666666666"],[1793556240,"458.8825"],[1793556255,"465.54675"],[1793556270,"480.02683333333334"],[1793556285,"465.5013333333333"],[1793556300,"458.7250833333333"],[1793556315,"463.33441666666664"],[1793556330,"476.85583333333335"],[1793556345,"468.5215"],[1793556360,"459.83233333333334"],[1793556375,"462.2943333333334"],[1793556390,"474.58633333333336"],[1793556405,"469.53175000000005"],[1793556420,"459.1035"],[1793556435,"459.369"],[1793556450,"470.18575"],[1793556465,"474.0646666666667"],[1793556480,"462.10625"],[1793556495,"460.17"],[1793556510,"469.295"]]}]}}},{"expr":"avg by (hostname, Hostname, instance, gpu) (avg_over_time(DCGM_FI_DEV_SM_CLOCK{job=\"dcgm\",hostname=\"gpu-node-1\",GPU_I_ID=\"\"}[1m]))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{"gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"1980"],[1793556165,"1980"],[1793556180,"1980"],[1793556195,"1980"],[1793556210,"1980"],[1793556225,"1980"],[1793556240,"1980"],[1793556255,"1980"],[1793556270,"1980"],[1793556285,"1980"],[1793556300,"1980"],[1793556315,"1980"],[1793556330,"1980"],[1793556345,"1980"],[1793556360,"1980"],[1793556375,"1980"],[1793556390,"1980"],[1793556405,"1980"],[1793556420,"1980"],[1793556435,"1980"],[1793556450,"1980"],[1793556465,"1980"],[1793556480,"1980"],[1793556495,"1980"],[1793556510,"1980"]]},{"metric":{"gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"1980"],[1793556165,"1980"],[1793556180,"1980"],[1793556195,"1980"],[1793556210,"1980"],[1793556225,"1980"],[1793556240,"1980"],[1793556255,"1980"],[1793556270,"1980"],[1793556285,"1980"],[1793556300,"1980"],[1793556315,"1980"],[1793556330,"1980"],[1793556345,"1980"],[1793556360,"1980"],[1793556375,"1980"],[1793556390,"1980"],[1793556405,"1980"],[1793556420,"1980"],[1793556435,"1980"],[1793556450,"1980"],[1793556465,"1980"],[1793556480,"1980"],[1793556495,"1980"],[1793556510,"1980"]]},{"metric":{"gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"1980"],[1793556165,"1980"],[1793556180,"1980"],[1793556195,"1980"],[1793556210,"1980"],[1793556225,"1980"],[1793556240,"1980"],[1793556255,"1980"],[1793556270,"1980"],[1793556285,"1980"],[1793556300,"1980"],[1793556315,"1980"],[1793556330,"1980"],[1793556345,"1980"],[1793556360,"1980"],[1793556375,"1980"],[1793556390,"1980"],[1793556405,"1980"],[1793556420,"1980"],[1793556435,"1980"],[1793556450,"1980"],[1793556465,"1980"],[1793556480,"1980"],[1793556495,"1980"],[1793556510,"1980"]]},{"metric":{"gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"1980"],[1793556165,"1980"],[1793556180,"1980"],[1793556195,"1980"],[1793556210,"1980"],[1793556225,"1980"],[1793556240,"1980"],[1793556255,"1980"],[1793556270,"1980"],[1793556285,"1980"],[1793556300,"1980"],[1793556315,"1980"],[1793556330,"1980"],[1793556345,"1980"],[1793556360,"1980"],[1793556375,"1980"],[1793556390,"1980"],[1793556405,"1980"],[1793556420,"1980"],[1793556435,"1980"],[1793556450,"1980"],[1793556465,"1980"],[1793556480,"1980"],[1793556495,"1980"],[1793556510,"1980"]]},{"metric":{"gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"1980"],[1793556165,"1980"],[1793556180,"1980"],[1793556195,"1980"],[1793556210,"1980"],[1793556225,"1980"],[1793556240,"1980"],[1793556255,"1980"],[1793556270,"1980"],[1793556285,"1980"],[1793556300,"1980"],[1793556315,"1980"],[1793556330,"1980"],[1793556345,"1980"],[1793556360,"1980"],[1793556375,"1980"],[1793556390,"1980"],[1793556405,"1980"],[1793556420,"1980"],[1793556435,"1980"],[1793556450,"1980"],[1793556465,"1980"],[1793556480,"1980"],[1793556495,"1980"],[1793556510,"1980"]]},{"metric":{"gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"1980"],[1793556165,"1980"],[1793556180,"1980"],[1793556195,"1980"],[1793556210,"1980"],[1793556225,"1980"],[1793556240,"1980"],[1793556255,"1980"],[1793556270,"1980"],[1793556285,"1980"],[1793556300,"1980"],[1793556315,"1980"],[1793556330,"1980"],[1793556345,"1980"],[1793556360,"1980"],[1793556375,"1980"],[1793556390,"1980"],[1793556405,"1980"],[1793556420,"1980"],[1793556435,"1980"],[1793556450,"1980"],[1793556465,"1980"],[1793556480,"1980"],[1793556495,"1980"],[1793556510,"1980"]]},{"metric":{"gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"1980"],[1793556165,"1980"],[1793556180,"1980"],[1793556195,"1980"],[1793556210,"1980"],[1793556225,"1980"],[1793556240,"1980"],[1793556255,"1980"],[1793556270,"1980"],[1793556285,"1980"],[1793556300,"1980"],[1793556315,"1980"],[1793556330,"1980"],[1793556345,"1980"],[1793556360,"1980"],[1793556375,"1980"],[1793556390,"1980"],[1793556405,"1980"],[1793556420,"1980"],[1793556435,"1980"],[1793556450,"1980"],[1793556465,"1980"],[1793556480,"1980"],[1793556495,"1980"],[1793556510,"1980"]]},{"metric":{"gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"1980"],[1793556165,"1980"],[1793556180,"1980"],[1793556195,"1980"],[1793556210,"1980"],[1793556225,"1980"],[1793556240,"1980"],[1793556255,"1980"],[1793556270,"1980"],[1793556285,"1980"],[1793556300,"1980"],[1793556315,"1980"],[1793556330,"1980"],[1793556345,"1980"],[1793556360,"1980"],[1793556375,"1980"],[1793556390,"1980"],[1793556405,"1980"],[1793556420,"1980"],[1793556435,"1980"],[1793556450,"1980"],[1793556465,"1980"],[1793556480,"1980"],[1793556495,"1980"],[1793556510,"1980"]]}]}}},{"expr":"avg by (hostname, Hostname, instance, gpu) (avg_over_time(DCGM_FI_DEV_MEM_CLOCK{job=\"dcgm\",hostname=\"gpu-node-1\",GPU_I_ID=\"\"}[1m]))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{"gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"2619"],[1793556165,"2619"],[1793556180,"2619"],[1793556195,"2619"],[1793556210,"2619"],[1793556225,"2619"],[1793556240,"2619"],[1793556255,"2619"],[1793556270,"2619"],[1793556285,"2619"],[1793556300,"2619"],[1793556315,"2619"],[1793556330,"2619"],[1793556345,"2619"],[1793556360,"2619"],[1793556375,"2619"],[1793556390,"2619"],[1793556405,"2619"],[1793556420,"2619"],[1793556435,"2619"],[1793556450,"2619"],[1793556465,"2619"],[1793556480,"2619"],[1793556495,"2619"],[1793556510,"2619"]]},{"metric":{"gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"2619"],[1793556165,"2619"],[1793556180,"2619"],[1793556195,"2619"],[1793556210,"2619"],[1793556225,"2619"],[1793556240,"2619"],[1793556255,"2619"],[1793556270,"2619"],[1793556285,"2619"],[1793556300,"2619"],[1793556315,"2619"],[1793556330,"2619"],[1793556345,"2619"],[1793556360,"2619"],[1793556375,"2619"],[1793556390,"2619"],[1793556405,"2619"],[1793556420,"2619"],[1793556435,"2619"],[1793556450,"2619"],[1793556465,"2619"],[1793556480,"2619"],[1793556495,"2619"],[1793556510,"2619"]]},{"metric":{"gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"2619"],[1793556165,"2619"],[1793556180,"2619"],[1793556195,"2619"],[1793556210,"2619"],[1793556225,"2619"],[1793556240,"2619"],[1793556255,"2619"],[1793556270,"2619"],[1793556285,"2619"],[1793556300,"2619"],[1793556315,"2619"],[1793556330,"2619"],[1793556345,"2619"],[1793556360,"2619"],[1793556375,"2619"],[1793556390,"2619"],[1793556405,"2619"],[1793556420,"2619"],[1793556435,"2619"],[1793556450,"2619"],[1793556465,"2619"],[1793556480,"2619"],[1793556495,"2619"],[1793556510,"2619"]]},{"metric":{"gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"2619"],[1793556165,"2619"],[1793556180,"2619"],[1793556195,"2619"],[1793556210,"2619"],[1793556225,"2619"],[1793556240,"2619"],[1793556255,"2619"],[1793556270,"2619"],[1793556285,"2619"],[1793556300,"2619"],[1793556315,"2619"],[1793556330,"2619"],[1793556345,"2619"],[1793556360,"2619"],[1793556375,"2619"],[1793556390,"2619"],[1793556405,"2619"],[1793556420,"2619"],[1793556435,"2619"],[1793556450,"2619"],[1793556465,"2619"],[1793556480,"2619"],[1793556495,"2619"],[1793556510,"2619"]]},{"metric":{"gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"2619"],[1793556165,"2619"],[1793556180,"2619"],[1793556195,"2619"],[1793556210,"2619"],[1793556225,"2619"],[1793556240,"2619"],[1793556255,"2619"],[1793556270,"2619"],[1793556285,"2619"],[1793556300,"2619"],[1793556315,"2619"],[1793556330,"2619"],[1793556345,"2619"],[1793556360,"2619"],[1793556375,"2619"],[1793556390,"2619"],[1793556405,"2619"],[1793556420,"2619"],[1793556435,"2619"],[1793556450,"2619"],[1793556465,"2619"],[1793556480,"2619"],[1793556495,"2619"],[1793556510,"2619"]]},{"metric":{"gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"2619"],[1793556165,"2619"],[1793556180,"2619"],[1793556195,"2619"],[1793556210,"2619"],[1793556225,"2619"],[1793556240,"2619"],[1793556255,"2619"],[1793556270,"2619"],[1793556285,"2619"],[1793556300,"2619"],[1793556315,"2619"],[1793556330,"2619"],[1793556345,"2619"],[1793556360,"2619"],[1793556375,"2619"],[1793556390,"2619"],[1793556405,"2619"],[1793556420,"2619"],[1793556435,"2619"],[1793556450,"2619"],[1793556465,"2619"],[1793556480,"2619"],[1793556495,"2619"],[1793556510,"2619"]]},{"metric":{"gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"2619"],[1793556165,"2619"],[1793556180,"2619"],[1793556195,"2619"],[1793556210,"2619"],[1793556225,"2619"],[1793556240,"2619"],[1793556255,"2619"],[1793556270,"2619"],[1793556285,"2619"],[1793556300,"2619"],[1793556315,"2619"],[1793556330,"2619"],[1793556345,"2619"],[1793556360,"2619"],[1793556375,"2619"],[1793556390,"2619"],[1793556405,"2619"],[1793556420,"2619"],[1793556435,"2619"],[1793556450,"2619"],[1793556465,"2619"],[1793556480,"2619"],[1793556495,"2619"],[1793556510,"2619"]]},{"metric":{"gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"2619"],[1793556165,"2619"],[1793556180,"2619"],[1793556195,"2619"],[1793556210,"2619"],[1793556225,"2619"],[1793556240,"2619"],[1793556255,"2619"],[1793556270,"2619"],[1793556285,"2619"],[1793556300,"2619"],[1793556315,"2619"],[1793556330,"2619"],[1793556345,"2619"],[1793556360,"2619"],[1793556375,"2619"],[1793556390,"2619"],[1793556405,"2619"],[1793556420,"2619"],[1793556435,"2619"],[1793556450,"2619"],[1793556465,"2619"],[1793556480,"2619"],[1793556495,"2619"],[1793556510,"2619"]]}]}}},{"expr":"max by (hostname, Hostname, instance, gpu) (max_over_time(DCGM_FI_DEV_CLOCKS_EVENT_REASONS{job=\"dcgm\",hostname=\"gpu-node-1\",GPU_I_ID=\"\"}[1m])) or max by (hostname, Hostname, instance, gpu) (max_over_time(DCGM_FI_DEV_CLOCK_THROTTLE_REASONS{job=\"dcgm\",hostname=\"gpu-node-1\",GPU_I_ID=\"\"}[1m]))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{"gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0"],[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0"],[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0"],[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0"],[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0"],[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0"],[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0"],[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0"],[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]}]}}},{"expr":"sum by (hostname, Hostname, instance, gpu) (increase(DCGM_FI_DEV_ECC_DBE_VOL_TOTAL{job=\"dcgm\",hostname=\"gpu-node-1\",GPU_I_ID=\"\"}[1m]))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{"gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0"],[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0"],[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0"],[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0"],[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0"],[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0"],[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0"],[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0"],[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]}]},"infos":["PromQL info: metric might not be a counter, name does not end in _total/_sum/_count/_bucket: \"DCGM_FI_DEV_ECC_DBE_VOL_TOTAL\" (1:54)"]}},{"expr":"sum by (hostname, Hostname, instance, gpu) (increase(DCGM_FI_DEV_PCIE_REPLAY_COUNTER{job=\"dcgm\",hostname=\"gpu-node-1\",GPU_I_ID=\"\"}[1m]))","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{"gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0"],[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0"],[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0"],[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0"],[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0"],[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0"],[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0"],[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556150,"0"],[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]}]},"infos":["PromQL info: metric might not be a counter, name does not end in _total/_sum/_count/_bucket: \"DCGM_FI_DEV_PCIE_REPLAY_COUNTER\" (1:54)"]}},{"expr":"increase((sum by (hostname, Hostname, instance, gpu) ({__name__=~\"DCGM_FI_DEV_NVLINK_CRC_FLIT_ERROR_TOTAL|DCGM_FI_DEV_NVLINK_CRC_DATA_ERROR_TOTAL|DCGM_FI_DEV_NVLINK_REPLAY_ERROR_TOTAL|DCGM_FI_DEV_NVLINK_RECOVERY_ERROR_TOTAL|DCGM_FI_DEV_NVLINK_CRC_ERROR_TOTAL|DCGM_FI_DEV_NVLINK_RECOVERY_TOTAL|DCGM_FI_DEV_NVLINK_REPLAY_TOTAL\",job=\"dcgm\",hostname=\"gpu-node-1\",GPU_I_ID=\"\"}))[1m:15s])","response":{"status":"success","data":{"resultType":"matrix","result":[{"metric":{"gpu":"0","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"1","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"2","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"3","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"4","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"5","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"6","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]},{"metric":{"gpu":"7","hostname":"gpu-node-1","instance":"127.0.0.1:9400"},"values":[[1793556165,"0"],[1793556180,"0"],[1793556195,"0"],[1793556210,"0"],[1793556225,"0"],[1793556240,"0"],[1793556255,"0"],[1793556270,"0"],[1793556285,"0"],[1793556300,"0"],[1793556315,"0"],[1793556330,"0"],[1793556345,"0"],[1793556360,"0"],[1793556375,"0"],[1793556390,"0"],[1793556405,"0"],[1793556420,"0"],[1793556435,"0"],[1793556450,"0"],[1793556465,"0"],[1793556480,"0"],[1793556495,"0"],[1793556510,"0"]]}]}}}]}
````

- [ ] **Шаг 6.4: Проверки.**

Run: `cd src-tauri && cargo test --lib -q`
Expected: `test result: ok. 331 passed`
Run: `cd src-tauri && cargo clippy --lib --tests -q -- -D warnings && cargo fmt --check`
Expected: без вывода

- [ ] **Шаг 6.5: Коммит.**

<!-- block: shell t06-commit step=6.5 -->
````
git add src-tauri/src/gpu/mod.rs \
  src-tauri/src/gpu/promql.rs \
  src-tauri/src/gpu/series.rs \
  src-tauri/src/gpu/source.rs \
  src-tauri/src/gpu/testdata/prometheus-3.15.1/README.md \
  src-tauri/src/gpu/testdata/prometheus-3.15.1/range-gpu-node-1.json
git commit -s -q -F - <<'EOF'
feat(gpu): PromQL templates and series of GPU history windows

Window templates for 14 fields average gauges and ratios, take the
maximum of the throttle mask and the increase of counters, grouped by
hostname, Hostname, instance and gpu, without MIG instance series. The
sum of NVLink error counters goes through a subquery: increase over
each counter would drop the series name and collide label sets. Series
are aligned to the window steps, at most 16 cards, a failed query is a
partial chart; answers of a real Prometheus.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
````

---

### Задача 7: Проверка GPU и команды check_gpu_source, read_gpu_status, read_gpu_history

Проверка компонента (спека I1d §3): конфигурация → каждый экспортёр (DNS/TCP, `/metrics`, карты) или карты в
источнике истории с автоподбором селектора и пределом 64 → привязка каждого сервера → сопоставление экспортёров с
целями истории → шаг-подсказка на каждую группу счётчиков без серий. Окно истории ищет, какой меткой подписан хост,
под селектором карт или селекторами экспортёров и читает шаблоны в срок источника. Открытый компонент GPU и снимки
счётчиков за минуту живут в пуле подключений до `evict`; опросы одной инсталляции идут по очереди. `form_secret` и
`history_source` команд истории становятся `pub(crate)`, у `Source` — `deadline()`.

**Files:**
- Create: `src-tauri/src/commands/gpu.rs`
- Create: `src-tauri/src/gpu/check.rs`
- Create: `src-tauri/src/gpu/read.rs`
- Modify: `src-tauri/src/commands/history.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/gpu/mod.rs`
- Modify: `src-tauri/src/history/read.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/servers/pool.rs`
- Modify: `src/locales/en.json`
- Modify: `src/locales/ru.json`

**Interfaces:**
- Consumes: всё из задач 2–6; `commands::history::{form_secret, history_source}`, `history::check::no_source`.
- Produces: `gpu::check::{check_gpu, GpuCheck { report, hosts, available, selector, bindings, exporters }, FoundHost,
  BindingSuggestion, ExporterSuggestion, found_cards, CARDS, TARGETS}`; `gpu::read::{read_history, base,
  GpuHistoryAnswer { window, stepSec, source, series, partial }}`; `ServerPool::{gpu, insert_gpu_if_current, gpu_live}`,
  `GpuLive`; команды Tauri `check_gpu_source(input)`, `read_gpu_status(installationId)`, `read_gpu_history(installationId,
  window, host, gpu)`; имена шагов `metrics`, `cards`, `bind`, `history`, `counters`.

- [ ] **Шаг 7.1: Тесты.** Тесты `gpu/check.rs`, `gpu/read.rs`, `commands/gpu.rs`; модули и команды подключаются.

<!-- block: file t07-src-tauri-src-commands-gpu-rs-tests step=7.1 path=src-tauri/src/commands/gpu.rs part=tests -->
````
#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::installations::SecretsInput;
    use crate::gpu::fields::Field;
    use crate::gpu::snapshot::tests::FULL;
    use crate::secrets::MemoryStore;
    use crate::servers::testing::{Reply, TestServer};
    use crate::storage::profile::{
        DirectGpu, GpuAuth, PrometheusGpu, ProxyConfig, sample_exporter, sample_gpu,
        sample_history, sample_profile,
    };

    fn state(dir: &std::path::Path) -> AppState {
        AppState::new(dir, Arc::new(MemoryStore::default())).unwrap()
    }

    fn direct(urls: &[String], auth: GpuAuth) -> GpuSource {
        let GpuSource::Direct(d) = sample_gpu() else {
            unreachable!()
        };
        GpuSource::Direct(DirectGpu {
            exporters: urls.iter().map(|u| sample_exporter(u)).collect(),
            auth,
            proxy: ProxyConfig::None,
            ..d
        })
    }

    fn glafira() -> GpuAuth {
        GpuAuth::Basic {
            username: "glafira".into(),
        }
    }

    #[tokio::test]
    async fn the_form_password_follows_the_save_rules() {
        let dir = tempfile::tempdir().unwrap();
        let st = state(dir.path());
        st.secrets.set(&gpu_secret_key("a"), "saved").unwrap();
        let mut p = sample_profile("a");
        p.gpu = Some(direct(&["http://gpu:9400/metrics".into()], glafira()));
        let input = |secret: Option<&str>, p: &crate::storage::profile::InstallationProfile| {
            InstallationInput {
                profile: p.clone(),
                secrets: SecretsInput {
                    gpu_secret: secret.map(str::to_string),
                    ..Default::default()
                },
            }
        };
        assert_eq!(
            form_password(&st, &input(None, &p)).await.unwrap(),
            None,
            "новой инсталляции сохранённый не положен"
        );
        st.installations.upsert(p.clone()).unwrap();
        assert_eq!(
            form_password(&st, &input(None, &p))
                .await
                .unwrap()
                .as_deref(),
            Some("saved")
        );
        assert_eq!(
            form_password(&st, &input(Some(" typed "), &p))
                .await
                .unwrap()
                .as_deref(),
            Some("typed")
        );
        let mut open = p.clone();
        open.gpu = Some(direct(&["http://gpu:9400/metrics".into()], GpuAuth::None));
        assert_eq!(form_password(&st, &input(None, &open)).await.unwrap(), None);
    }

    #[tokio::test]
    async fn status_reads_the_cards_and_counts_deltas_between_polls() {
        let exporter = TestServer::start(vec![("/metrics", Reply::text(FULL))]).await;
        let dir = tempfile::tempdir().unwrap();
        let st = state(dir.path());
        let mut p = sample_profile("a");
        p.gpu = Some(direct(
            &[format!("{}/metrics", exporter.url())],
            GpuAuth::None,
        ));
        st.installations.upsert(p).unwrap();
        let first = read_gpu_status_inner(&st, "a").await.unwrap();
        assert_eq!(first.snapshot.hosts[0].cards.len(), 8);
        assert_eq!(first.delta_sec, None);
        let second = read_gpu_status_inner(&st, "a").await.unwrap();
        assert!(second.delta_sec.is_some(), "второй опрос — с приростами");
        assert_eq!(
            second.snapshot.hosts[0].cards[0].deltas[&Field::PcieReplay],
            0.0,
            "снимок тот же — прироста нет"
        );
        let open = st.pool.gpu("a").unwrap();
        st.evict_installation("a");
        assert!(st.pool.gpu("a").is_none());
        let third = read_gpu_status_inner(&st, "a").await.unwrap();
        assert_eq!(
            third.delta_sec, None,
            "после сброса инсталляции приросты начинаются заново"
        );
        assert!(!Arc::ptr_eq(&open, &st.pool.gpu("a").unwrap()));
    }

    #[tokio::test]
    async fn without_the_component_or_history_there_is_nothing_to_read() {
        let dir = tempfile::tempdir().unwrap();
        let st = state(dir.path());
        st.installations.upsert(sample_profile("a")).unwrap();
        assert_eq!(
            read_gpu_status_inner(&st, "a").await.unwrap_err().key,
            "errors.gpu.notConfigured"
        );
        let e = read_gpu_history_inner(&st, "a", Window::Hour, "h", None)
            .await
            .unwrap_err();
        assert_eq!(e.key, "errors.history.noSource");
        let mut p = sample_profile("a");
        p.history = Some(sample_history());
        p.gpu = Some(direct(&["http://gpu:9400/metrics".into()], GpuAuth::None));
        st.installations.upsert(p).unwrap();
        let e = read_gpu_history_inner(&st, "a", Window::Hour, "h", None)
            .await
            .unwrap_err();
        assert_eq!(
            e.key, "errors.gpu.notMatched",
            "экспортёры без селекторов истории"
        );
    }

    #[tokio::test]
    async fn prometheus_mode_reads_status_through_the_history_source() {
        let prom = TestServer::start(vec![(
            "/api/v1/query",
            Reply::json(&crate::gpu::source::tests::snapshot_file("instant.json")),
        )])
        .await;
        let dir = tempfile::tempdir().unwrap();
        let st = state(dir.path());
        let mut p = sample_profile("a");
        p.history = Some(crate::history::client::tests::source(
            &prom.url(),
            crate::storage::profile::HistoryAuth::None,
        ));
        p.gpu = Some(GpuSource::Prometheus(PrometheusGpu {
            selector: None,
            extra: Default::default(),
        }));
        st.installations.upsert(p).unwrap();
        let status = read_gpu_status_inner(&st, "a").await.unwrap();
        assert_eq!(status.snapshot.hosts.len(), 2);
        assert!(
            st.pool.history("a").is_some(),
            "источник истории открыт общим для окон сервера"
        );
    }
}
````

<!-- block: patch t07-src-tauri-src-commands-mod-rs step=7.1 -->
````
diff --git a/src-tauri/src/commands/mod.rs b/src-tauri/src/commands/mod.rs
index c5589eb..9dbbf59 100644
--- a/src-tauri/src/commands/mod.rs
+++ b/src-tauri/src/commands/mod.rs
@@ -1,5 +1,6 @@
 pub mod app;
 pub mod folders;
+pub mod gpu;
 pub mod history;
 pub mod installations;
 pub mod servers;
````

<!-- block: file t07-src-tauri-src-gpu-check-rs-tests step=7.1 path=src-tauri/src/gpu/check.rs part=tests -->
````
#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::snapshot::tests::{DEFAULT, FULL};
    use crate::history::client::tests::source as history_source;
    use crate::servers::testing::{Reply, TestServer};
    use crate::storage::profile::{
        GpuAuth, HistoryAuth, ProxyConfig, ServerConfig, sample_exporter, sample_gpu,
        sample_profile, sample_server,
    };

    fn snap(name: &str) -> String {
        crate::gpu::source::tests::snapshot_file(name)
    }

    fn server(id: &str, url: &str) -> ServerConfig {
        ServerConfig {
            name: format!("vllm-{id}"),
            base_url: url.into(),
            ..sample_server(id)
        }
    }

    fn direct_profile(urls: &[String]) -> InstallationProfile {
        let mut p = sample_profile("a");
        let GpuSource::Direct(d) = sample_gpu() else {
            unreachable!()
        };
        p.gpu = Some(GpuSource::Direct(DirectGpu {
            exporters: urls.iter().map(|u| sample_exporter(u)).collect(),
            auth: GpuAuth::None,
            proxy: ProxyConfig::None,
            ..d
        }));
        p.servers = vec![
            server("s1", "http://gpu-node-1.corp:8000"),
            server("s2", "http://lb.corp:8000"),
        ];
        p
    }

    fn names(c: &GpuCheck) -> Vec<(&'static str, bool, bool)> {
        c.report
            .steps
            .iter()
            .map(|s| (s.name, s.ok, s.warning))
            .collect()
    }

    fn keys(c: &GpuCheck) -> Vec<&str> {
        c.report
            .steps
            .iter()
            .map(|s| s.message.key.as_str())
            .collect()
    }

    #[tokio::test]
    async fn direct_exporters_cards_bindings_and_missing_counters() {
        let a = TestServer::start(vec![("/metrics", Reply::text(FULL))]).await;
        let b = TestServer::start(vec![("/metrics", Reply::text(DEFAULT))]).await;
        let p = direct_profile(&[
            format!("{}/metrics", a.url()),
            format!("{}/metrics", b.url()),
        ]);
        let c = check_gpu(&p, None, None, &PolicyStatus::default()).await;
        assert!(c.report.ok, "{:?}", c.report);
        assert_eq!(
            names(&c),
            [
                ("config", true, false),
                ("cards", true, false),
                ("cards", true, false),
                ("bind", true, false),
                ("bind", true, true),
            ],
            "полный набор у одного экспортёра — подсказок о счётчиках нет"
        );
        assert_eq!(c.report.steps[1].message.params["count"], "8");
        assert_eq!(c.report.steps[1].message.params["hosts"], "gpu-node-1");
        assert_eq!(c.report.steps[1].message.params["driver"], "570.124.06");
        assert_eq!(
            c.hosts,
            [
                FoundHost {
                    host: "gpu-node-1".into(),
                    cards: (0..8).collect()
                },
                FoundHost {
                    host: "localhost".into(),
                    cards: vec![0, 1]
                }
            ]
        );
        assert_eq!(
            c.bindings,
            [
                BindingSuggestion {
                    server_id: "s1".into(),
                    binding: Some(GpuBinding {
                        host: "gpu-node-1".into(),
                        indices: None
                    })
                },
                BindingSuggestion {
                    server_id: "s2".into(),
                    binding: None
                },
            ]
        );
        assert_eq!(keys(&c)[4], "messages.gpuTest.unbound");

        let only_default = direct_profile(&[format!("{}/metrics", b.url())]);
        let c = check_gpu(&only_default, None, None, &PolicyStatus::default()).await;
        let counters: Vec<&str> = c
            .report
            .steps
            .iter()
            .filter(|s| s.name == "counters")
            .map(|s| s.message.key.as_str())
            .collect();
        assert_eq!(
            counters,
            [
                "messages.gpuTest.missingSm",
                "messages.gpuTest.missingEcc",
                "messages.gpuTest.missingThrottle",
                "messages.gpuTest.missingNvlink"
            ]
        );
        assert!(c.report.ok, "подсказки — не ошибки");
    }

    #[tokio::test]
    async fn exporter_failures_are_steps_and_saved_bindings_are_checked() {
        let ok = TestServer::start(vec![("/metrics", Reply::text(FULL))]).await;
        let refused = TestServer::start(vec![("/metrics", Reply::status(401))]).await;
        let other = TestServer::start(vec![("/metrics", Reply::text("# no dcgm\nup 1\n"))]).await;
        let mut p = direct_profile(&[
            format!("{}/metrics", ok.url()),
            format!("{}/metrics", refused.url()),
            format!("{}/metrics", other.url()),
            "http://127.0.0.1:1/metrics".into(),
        ]);
        p.servers[0].gpus = Some(GpuBinding {
            host: "gpu-node-1".into(),
            indices: Some(vec![1, 9]),
        });
        p.servers[1].gpus = Some(GpuBinding {
            host: "gpu-node-7".into(),
            indices: None,
        });
        let c = check_gpu(&p, None, None, &PolicyStatus::default()).await;
        assert!(!c.report.ok);
        assert_eq!(
            names(&c),
            [
                ("config", true, false),
                ("cards", true, false),
                ("auth", false, false),
                ("metrics", false, false),
                ("tcp", false, false),
                ("bind", true, true),
                ("bind", true, true),
            ]
        );
        assert_eq!(
            c.report.steps[2].error.as_ref().unwrap().key,
            "errors.gpu.unauthorized"
        );
        assert_eq!(
            c.report.steps[3].error.as_ref().unwrap().key,
            "errors.gpu.noDcgm"
        );
        assert_eq!(keys(&c)[5], "messages.gpuTest.boundCardsMissing");
        assert_eq!(c.report.steps[5].message.params["cards"], "9");
        assert_eq!(keys(&c)[6], "messages.gpuTest.boundHostMissing");
        assert!(
            c.bindings.is_empty(),
            "сохранённые привязки не подбираются заново"
        );
    }

    #[tokio::test]
    async fn exporters_are_matched_with_history_targets() {
        let a = TestServer::start(vec![("/metrics", Reply::text(FULL))]).await;
        let prom =
            TestServer::start(vec![("/api/v1/query", Reply::json(&snap("targets.json")))]).await;
        let mut p = direct_profile(&["http://127.0.0.1:9400/metrics".into()]);
        // Адрес экспортёра в профиле — как цель Prometheus; ответы — тестового сервера.
        p.gpu = match p.gpu.take() {
            Some(GpuSource::Direct(mut d)) => {
                d.exporters = vec![
                    sample_exporter(&format!("{}/metrics", a.url())),
                    sample_exporter("http://127.0.0.1:9401/metrics"),
                ];
                d.exporters[1].history_selector = None;
                Some(GpuSource::Direct(d))
            }
            other => other,
        };
        p.history = Some(history_source(&prom.url(), HistoryAuth::None));
        let c = check_gpu(&p, None, None, &PolicyStatus::default()).await;
        let history: Vec<&str> = c
            .report
            .steps
            .iter()
            .filter(|s| s.name == "history")
            .map(|s| s.message.key.as_str())
            .collect();
        assert_eq!(
            history,
            [
                "messages.gpuTest.historyNoTarget",
                "messages.gpuTest.historyAuto"
            ]
        );
        assert_eq!(
            c.exporters[1].history_selector.as_deref(),
            Some(r#"{job="dcgm", instance="127.0.0.1:9401"}"#)
        );
        assert_eq!(c.exporters[0].history_selector, None);
    }

    #[tokio::test]
    async fn prometheus_mode_auto_matches_the_selector_and_limits_the_count() {
        let prom =
            TestServer::start(vec![("/api/v1/query", Reply::json(&snap("cards.json")))]).await;
        let mut p = sample_profile("a");
        p.history = Some(history_source(&prom.url(), HistoryAuth::None));
        p.gpu = Some(GpuSource::Prometheus(PrometheusGpu {
            selector: None,
            extra: Default::default(),
        }));
        p.servers = vec![server("s1", "http://gpu-node-1:8000")];
        let c = check_gpu(&p, None, None, &PolicyStatus::default()).await;
        assert!(c.report.ok, "{:?}", c.report);
        assert_eq!(
            keys(&c),
            [
                "messages.connectionTest.configOk",
                "messages.gpuTest.selectorAuto",
                "messages.gpuTest.historyCards",
                "messages.gpuTest.boundAuto"
            ]
        );
        assert_eq!(c.selector.as_deref(), Some(r#"{hostname=~"gpu-node-1"}"#));
        assert_eq!(c.available, ["gpu-node-1", "localhost"]);
        assert_eq!(
            c.hosts,
            [FoundHost {
                host: "gpu-node-1".into(),
                cards: (0..8).collect()
            }]
        );

        p.servers = vec![server("s1", "http://lb:8000")];
        let c = check_gpu(&p, None, None, &PolicyStatus::default()).await;
        assert_eq!(keys(&c)[1], "messages.gpuTest.noServerHosts");
        assert_eq!(c.selector, None);
        assert_eq!(c.hosts.len(), 2, "без селектора — все карты источника");

        let many: Vec<serde_json::Value> = (0..65)
            .map(|i| serde_json::json!({ "metric": { "hostname": format!("h{}", i / 8), "gpu": (i % 8).to_string() }, "value": [0, "1"] }))
            .collect();
        let body = serde_json::json!({ "status": "success", "data": { "resultType": "vector", "result": many } }).to_string();
        let big = TestServer::start(vec![("/api/v1/query", Reply::json(&body))]).await;
        p.history = Some(history_source(&big.url(), HistoryAuth::None));
        p.gpu = Some(GpuSource::Prometheus(PrometheusGpu {
            selector: Some(r#"{job="dcgm"}"#.into()),
            extra: Default::default(),
        }));
        let c = check_gpu(&p, None, None, &PolicyStatus::default()).await;
        assert!(!c.report.ok);
        assert_eq!(
            c.report.steps.last().unwrap().error.as_ref().unwrap().key,
            "errors.gpu.tooManyGpus"
        );
    }

    #[tokio::test]
    async fn configuration_errors_stop_the_check() {
        let mut p = sample_profile("a");
        let c = check_gpu(&p, None, None, &PolicyStatus::default()).await;
        assert_eq!(
            c.report.steps[0].error.as_ref().unwrap().key,
            "errors.gpu.notConfigured"
        );
        p.gpu = Some(GpuSource::Prometheus(PrometheusGpu {
            selector: None,
            extra: Default::default(),
        }));
        let c = check_gpu(&p, None, None, &PolicyStatus::default()).await;
        assert_eq!(
            c.report.steps[0].error.as_ref().unwrap().key,
            "errors.gpu.noHistorySource"
        );
        assert_eq!(c.report.steps.len(), 1);
    }

    #[test]
    fn found_cards_take_the_host_label_in_order() {
        let samples: Vec<Sample> = serde_json::from_str::<serde_json::Value>(&snap("cards.json"))
            .map(|v| crate::history::client::parse_vector(&v["data"]).unwrap())
            .unwrap();
        let cards = found_cards(&samples);
        assert_eq!(cards.len(), 10);
        assert!(cards.iter().all(|c| c.label == HostLabel::Lower));
        let s = |labels: &[(&str, &str)]| Sample {
            labels: labels
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            value: Some(1.0),
        };
        let mixed = found_cards(&[
            s(&[("Hostname", "old"), ("gpu", "0")]),
            s(&[("instance", "10.0.0.5:9400"), ("gpu", "1")]),
        ]);
        assert_eq!(
            mixed,
            [
                FoundCard {
                    host: "10.0.0.5".into(),
                    label: HostLabel::Instance,
                    index: 1
                },
                FoundCard {
                    host: "old".into(),
                    label: HostLabel::Upper,
                    index: 0
                }
            ]
        );
    }
}
````

<!-- block: patch t07-src-tauri-src-gpu-mod-rs step=7.1 -->
````
diff --git a/src-tauri/src/gpu/mod.rs b/src-tauri/src/gpu/mod.rs
index 2919535..4d79d32 100644
--- a/src-tauri/src/gpu/mod.rs
+++ b/src-tauri/src/gpu/mod.rs
@@ -2,9 +2,11 @@
 //! привязок, источник карт и приросты счётчиков, окна истории.
 
 pub mod bind;
+pub mod check;
 pub mod fields;
 pub mod live;
 pub mod promql;
+pub mod read;
 pub mod series;
 pub mod snapshot;
 pub mod source;
````

<!-- block: file t07-src-tauri-src-gpu-read-rs-tests step=7.1 path=src-tauri/src/gpu/read.rs part=tests -->
````
#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::history::client::HistoryClient;
    use crate::history::client::tests::source as history_source;
    use crate::policy::PolicyStatus;
    use crate::servers::client::Timeouts;
    use crate::servers::testing::{Reply, TestServer};
    use crate::storage::profile::HistoryAuth;

    fn snap(name: &str) -> String {
        crate::gpu::source::tests::snapshot_file(name)
    }

    async fn open(url: &str) -> Source {
        let t = Timeouts {
            connect_ms: 2_000,
            request_ms: 2_000,
        };
        let client = HistoryClient::new(
            &history_source(url, HistoryAuth::None),
            t,
            None,
            &PolicyStatus::default(),
        )
        .unwrap();
        Source::new(client, SourceKind::Prometheus, Duration::from_secs(5))
    }

    /// Ответ `query_range` одного шаблона из снятого окна; поиск хостов — снятые карты.
    fn routes() -> Vec<(&'static str, Reply)> {
        let raw: serde_json::Value = serde_json::from_str(&snap("range-gpu-node-1.json")).unwrap();
        let util = raw["queries"][0]["response"].to_string();
        vec![
            ("/api/v1/query", Reply::json(&snap("cards.json"))),
            ("/api/v1/query_range", Reply::json(&util)),
        ]
    }

    #[tokio::test]
    async fn window_of_a_host_finds_its_label_and_reads_every_field() {
        let p = TestServer::start(routes()).await;
        let source = open(&p.url()).await;
        let a = read_history(
            &source,
            &[r#"job="dcgm""#.into()],
            "gpu-node-1",
            Some(3),
            Window::Hour,
            1_793_556_000,
        )
        .await
        .unwrap();
        assert_eq!((a.window, a.step_sec), (Window::Hour, 60));
        assert!(a.partial.is_empty());
        assert_eq!(
            a.series.len(),
            8,
            "тестовый сервер отдаёт один снятый ряд на любой шаблон"
        );
        let paths = p.paths();
        assert_eq!(paths.len(), 15, "поиск хоста и 14 шаблонов");
        let range = paths.iter().find(|x| x.contains("query_range")).unwrap();
        let decoded = url::form_urlencoded::parse(range.split_once('?').unwrap().1.as_bytes())
            .find(|(k, _)| k == "query")
            .unwrap()
            .1
            .to_string();
        assert!(
            decoded.contains(r#"{job="dcgm",hostname="gpu-node-1",gpu="3",GPU_I_ID=""}"#),
            "{decoded}"
        );
        assert!(p.peak_concurrency() <= CONCURRENCY);
    }

    #[tokio::test]
    async fn unknown_host_and_failed_source_are_errors() {
        let p = TestServer::start(routes()).await;
        let source = open(&p.url()).await;
        let e = read_history(
            &source,
            &[r#"job="dcgm""#.into()],
            "gpu-node-9",
            None,
            Window::Hour,
            1_793_556_000,
        )
        .await
        .unwrap_err();
        assert_eq!(
            (e.key.as_str(), e.params["host"].as_str()),
            ("errors.gpu.hostNotInHistory", "gpu-node-9")
        );
        let e = read_history(&source, &[], "gpu-node-1", None, Window::Hour, 0)
            .await
            .unwrap_err();
        assert_eq!(
            e.key, "errors.gpu.hostNotInHistory",
            "без базовых селекторов хоста не найти"
        );

        let down = TestServer::start(vec![
            ("/api/v1/query", Reply::json(&snap("cards.json"))),
            ("/api/v1/query_range", Reply::status(503)),
        ])
        .await;
        let source = open(&down.url()).await;
        let e = read_history(
            &source,
            &[r#"job="dcgm""#.into()],
            "gpu-node-1",
            None,
            Window::Hour,
            0,
        )
        .await
        .unwrap_err();
        assert_eq!(
            e.key, "errors.history.status",
            "все запросы упали — ошибка первого"
        );
    }

    #[tokio::test]
    async fn slow_source_misses_the_deadline() {
        let slow = TestServer::start(vec![(
            "/api/v1/query",
            Reply::json(&snap("cards.json")).delayed(Duration::from_millis(300)),
        )])
        .await;
        let t = Timeouts {
            connect_ms: 2_000,
            request_ms: 2_000,
        };
        let client = HistoryClient::new(
            &history_source(&slow.url(), HistoryAuth::None),
            t,
            None,
            &PolicyStatus::default(),
        )
        .unwrap();
        let source = Source::new(client, SourceKind::Prometheus, Duration::from_millis(100));
        let e = read_history(
            &source,
            &[r#"job="dcgm""#.into()],
            "gpu-node-1",
            None,
            Window::Hour,
            0,
        )
        .await
        .unwrap_err();
        assert_eq!(e.key, "errors.history.deadline");
    }

    #[test]
    fn base_conditions_come_from_the_selector() {
        assert_eq!(base(None).unwrap(), "");
        assert_eq!(
            base(Some(r#"{job="dcgm", hostname=~"a|b"}"#)).unwrap(),
            r#"job="dcgm",hostname=~"a|b""#
        );
        assert_eq!(
            base(Some("job=dcgm")).unwrap_err().key,
            "errors.history.selector"
        );
    }
}
````

<!-- block: patch t07-src-tauri-src-lib-rs step=7.1 -->
````
diff --git a/src-tauri/src/lib.rs b/src-tauri/src/lib.rs
index 41ec6f1..935616a 100644
--- a/src-tauri/src/lib.rs
+++ b/src-tauri/src/lib.rs
@@ -59,6 +59,9 @@ pub fn run() {
             commands::servers::read_server_config,
             commands::history::check_history_source,
             commands::history::read_server_history,
+            commands::gpu::check_gpu_source,
+            commands::gpu::read_gpu_status,
+            commands::gpu::read_gpu_history,
             commands::folders::list_folders,
             commands::folders::create_folder,
             commands::folders::rename_folder,
````

- [ ] **Шаг 7.2: Увидеть падение.**

Run: `cd src-tauri && cargo test --lib -q gpu`
Expected: FAIL — не компилируется: тестам не хватает типов и функций, которые появятся в шаге 3

- [ ] **Шаг 7.3: Реализация.** Проверка, окно истории, пул, команды и тексты шагов.

<!-- block: file t07-src-tauri-src-commands-gpu-rs step=7.3 path=src-tauri/src/commands/gpu.rs part=impl -->
````
//! Команды GPU (спека I1d §3–4): проверка компонента из формы, снимок карт для окна «Сейчас» и
//! окно истории карт хоста.

use std::sync::Arc;

use tauri::State;

use super::blocking;
use super::history::{form_secret, history_source};
use super::installations::{InstallationInput, SecretOp, gpu_op};
use crate::error::{AppResult, ErrorCode};
use crate::gpu::check::{GpuCheck, check_gpu};
use crate::gpu::live::{GpuStatus, record};
use crate::gpu::read::{GpuHistoryAnswer, base, read_history};
use crate::gpu::source::{OpenGpu, snapshot};
use crate::history::check::no_source;
use crate::history::window::Window;
use crate::secrets::gpu_secret_key;
use crate::servers::client::Timeouts;
use crate::servers::live::now_ms;
use crate::state::AppState;
use crate::storage::profile::GpuSource;

/// Проверяет компонент GPU несохранённого профиля. Пароль экспортёров и секрет источника
/// истории — как при сохранении (`gpu_op`, `history_op`): из формы, без значения — сохранённый,
/// только если вид входа не менялся.
#[tauri::command]
pub async fn check_gpu_source(
    state: State<'_, AppState>,
    input: InstallationInput,
) -> AppResult<GpuCheck> {
    let st = state.inner();
    let password = form_password(st, &input).await?;
    let history = form_secret(st, &input).await?;
    Ok(check_gpu(&input.profile, password, history, &st.policy.status).await)
}

/// Пароль экспортёров для проверки: `gpu_op` над сохранённым профилем инсталляции.
async fn form_password(st: &AppState, input: &InstallationInput) -> AppResult<Option<String>> {
    let previous = st.installations.get(&input.profile.id).ok();
    match gpu_op(&input.profile, previous.as_ref(), &input.secrets) {
        SecretOp::Set(v) => Ok(Some(v)),
        SecretOp::Keep => {
            let (st, key) = (st.clone(), gpu_secret_key(&input.profile.id));
            blocking(move || st.secrets.get(&key)).await
        }
        SecretOp::Delete => Ok(None),
    }
}

fn not_configured() -> crate::error::AppError {
    app_err!(
        ErrorCode::InvalidInput,
        "errors.gpu.notConfigured",
        "The installation has no GPU component"
    )
}

/// Компонент GPU инсталляции из пула; нет — открывается (пароль из keychain, источник истории —
/// из пула) и кладётся, если инсталляцию за это время не сбросили.
async fn open_gpu(st: &AppState, installation_id: &str) -> AppResult<Arc<OpenGpu>> {
    if let Some(open) = st.pool.gpu(installation_id) {
        return Ok(open);
    }
    let generation = st.pool.generation(installation_id);
    let profile = st.installations.get(installation_id)?;
    let open = match profile.gpu.clone().ok_or_else(not_configured)? {
        GpuSource::Direct(d) => {
            let password = {
                let (st, key) = (st.clone(), gpu_secret_key(installation_id));
                blocking(move || st.secrets.get(&key)).await?
            };
            OpenGpu::direct(&d, Timeouts::from(&profile), password, &st.policy.status)?
        }
        GpuSource::Prometheus(g) => {
            OpenGpu::prometheus(&g, history_source(st, installation_id).await?)
        }
    };
    let open = Arc::new(open);
    st.pool
        .insert_gpu_if_current(installation_id, generation, open.clone());
    Ok(open)
}

/// Снимок карт инсталляции с приростами счётчиков за последнюю минуту опросов.
#[tauri::command]
pub async fn read_gpu_status(
    state: State<'_, AppState>,
    installation_id: String,
) -> AppResult<GpuStatus> {
    read_gpu_status_inner(&state, &installation_id).await
}

pub async fn read_gpu_status_inner(st: &AppState, installation_id: &str) -> AppResult<GpuStatus> {
    let open = open_gpu(st, installation_id).await?;
    let live = st.pool.gpu_live(installation_id);
    // Опросы из нескольких вкладок идут по очереди: приросты считаются по порядку снимков.
    let mut history = live.lock().await;
    let (cards, errors) = open.read().await?;
    Ok(record(&mut history, snapshot(cards, errors), now_ms()))
}

/// Окно истории карт хоста `host`: одной карты `gpu` или всех (не больше 16).
#[tauri::command]
pub async fn read_gpu_history(
    state: State<'_, AppState>,
    installation_id: String,
    window: Window,
    host: String,
    gpu: Option<u32>,
) -> AppResult<GpuHistoryAnswer> {
    read_gpu_history_inner(&state, &installation_id, window, &host, gpu).await
}

pub async fn read_gpu_history_inner(
    st: &AppState,
    installation_id: &str,
    window: Window,
    host: &str,
    gpu: Option<u32>,
) -> AppResult<GpuHistoryAnswer> {
    let profile = st.installations.get(installation_id)?;
    if profile.history.is_none() {
        return Err(no_source());
    }
    let bases = match profile.gpu.as_ref().ok_or_else(not_configured)? {
        GpuSource::Prometheus(g) => vec![base(g.selector.as_deref())?],
        GpuSource::Direct(d) => d
            .exporters
            .iter()
            .filter_map(|e| e.history_selector.as_deref())
            .map(|s| base(Some(s)))
            .collect::<AppResult<Vec<_>>>()?,
    };
    if bases.is_empty() {
        return Err(app_err!(
            ErrorCode::InvalidInput,
            "errors.gpu.notMatched",
            "The DCGM exporters are not matched with the metrics history: choose their targets in the installation"
        ));
    }
    let source = history_source(st, installation_id).await?;
    read_history(&source, &bases, host, gpu, window, now_ms() / 1000).await
}
````

<!-- block: patch t07-src-tauri-src-commands-history-rs step=7.3 -->
````
diff --git a/src-tauri/src/commands/history.rs b/src-tauri/src/commands/history.rs
index 4ef6994..88753b8 100644
--- a/src-tauri/src/commands/history.rs
+++ b/src-tauri/src/commands/history.rs
@@ -32,7 +32,10 @@ pub async fn check_history_source(
 
 /// Секрет источника для проверки: `history_op` над сохранённым профилем инсталляции (новой
 /// инсталляции в хранилище ещё нет — прежнего профиля нет, и сохранённый секрет не берётся).
-async fn form_secret(st: &AppState, input: &InstallationInput) -> AppResult<Option<String>> {
+pub(crate) async fn form_secret(
+    st: &AppState,
+    input: &InstallationInput,
+) -> AppResult<Option<String>> {
     let previous = st.installations.get(&input.profile.id).ok();
     match history_op(&input.profile, previous.as_ref(), &input.secrets) {
         SecretOp::Set(v) => Ok(Some(v)),
@@ -57,7 +60,7 @@ pub async fn read_server_history(
 
 /// Клиент источника инсталляции из пула; нет — создаётся (секрет из keychain, тип источника по
 /// `buildinfo`) и кладётся, если инсталляцию за это время не сбросили.
-async fn history_source(st: &AppState, installation_id: &str) -> AppResult<Arc<Source>> {
+pub(crate) async fn history_source(st: &AppState, installation_id: &str) -> AppResult<Arc<Source>> {
     if let Some(s) = st.pool.history(installation_id) {
         return Ok(s);
     }
````

<!-- block: file t07-src-tauri-src-gpu-check-rs step=7.3 path=src-tauri/src/gpu/check.rs part=impl -->
````
//! Проверка компонента GPU (спека I1d §3): конфигурация → экспортёры (DNS/TCP, `/metrics`, карты)
//! или карты в источнике истории → привязка каждого сервера → сопоставление экспортёров с
//! историей → полнота набора счётчиков. Сбой GPU не мешает подключению серверов: проверка только
//! сообщает его.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::Serialize;

use crate::error::{AppError, ErrorCode};
use crate::gpu::bind::{
    BindingState, DcgmTarget, FoundCard, HostLabel, binding_state, matching_exporter_targets,
    same_host, server_host, suggest_binding, suggest_selector,
};
use crate::gpu::fields::{Group, PRESENCE};
use crate::gpu::snapshot::{GpuCard, MAX_GPUS, missing};
use crate::gpu::source::{Exporter, exporter_server, instance_host, too_many_gpus};
use crate::history::client::{HistoryClient, Sample};
use crate::history::targets::{parse_selector, render, selector_label};
use crate::i18n::LocalizedText;
use crate::policy::PolicyStatus;
use crate::servers::client::Timeouts;
use crate::servers::connectivity::{
    TestReport, TestStep, failed_step, passed, report, tcp_failure, tcp_probe, tcp_target,
};
use crate::storage::profile::{
    DirectGpu, GpuBinding, GpuSource, InstallationProfile, PrometheusGpu, parse_exporter_url,
    redact_url, validate_gpu,
};

/// Карты в источнике истории: хост (по метке), метка хоста и номер.
pub const CARDS: &str = "count by (hostname, Hostname, instance, gpu) (DCGM_FI_DEV_FB_USED)";
/// Цели экспортёров DCGM в источнике истории.
pub const TARGETS: &str = "count by (job, instance) (DCGM_FI_DEV_FB_USED)";

/// Хост и номера его карт.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FoundHost {
    pub host: String,
    pub cards: Vec<u32>,
}

/// Привязка, которую форма может поставить серверу без привязки.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BindingSuggestion {
    pub server_id: String,
    pub binding: Option<GpuBinding>,
}

/// Селектор экспортёра в истории, который может поставить форма, и подходящие цели.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExporterSuggestion {
    pub url: String,
    pub history_selector: Option<String>,
    pub targets: Vec<DcgmTarget>,
}

/// Итог проверки для формы.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuCheck {
    pub report: TestReport,
    /// Хосты карт компонента (для выбора привязки сервера).
    pub hosts: Vec<FoundHost>,
    /// Все хосты карт источника истории (режим `prometheus`, для выбора хостов селектора).
    pub available: Vec<String>,
    /// Автоподбор селектора карт (режим `prometheus`, селектор пуст).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
    pub bindings: Vec<BindingSuggestion>,
    pub exporters: Vec<ExporterSuggestion>,
}

/// Шаг с предупреждением: проверку не валит, показывается ⚠.
fn warning_step(name: &'static str, message: LocalizedText) -> TestStep {
    TestStep {
        name,
        ok: true,
        warning: true,
        message,
        error: None,
    }
}

/// Хосты и номера карт по порядку из пар «хост, номер».
fn hosts_of<'a>(pairs: impl Iterator<Item = (&'a str, u32)>) -> Vec<FoundHost> {
    let mut by_host: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    for (host, index) in pairs {
        by_host.entry(host.to_string()).or_default().push(index);
    }
    by_host
        .into_iter()
        .map(|(host, mut cards)| {
            cards.sort_unstable();
            cards.dedup();
            FoundHost { host, cards }
        })
        .collect()
}

/// Карты из ответа `CARDS`: метка хоста — `hostname`, `Hostname`, иначе `instance` без порта.
pub fn found_cards(samples: &[Sample]) -> Vec<FoundCard> {
    let mut out: Vec<FoundCard> = samples
        .iter()
        .filter_map(|s| {
            let get = |k: &str| s.labels.get(k).filter(|v| !v.is_empty());
            let index = get("gpu")?.parse().ok()?;
            let (host, label) = if let Some(h) = get("hostname") {
                (h.clone(), HostLabel::Lower)
            } else if let Some(h) = get("Hostname") {
                (h.clone(), HostLabel::Upper)
            } else {
                (instance_host(get("instance")?), HostLabel::Instance)
            };
            Some(FoundCard { host, label, index })
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Цели из ответа `TARGETS`.
fn targets_from(samples: &[Sample]) -> Vec<DcgmTarget> {
    let mut out: Vec<DcgmTarget> = samples
        .iter()
        .filter_map(|s| {
            Some(DcgmTarget {
                job: s.labels.get("job")?.clone(),
                instance: s.labels.get("instance")?.clone(),
            })
        })
        .collect();
    out.sort();
    out
}

fn hosts_text(hosts: &[FoundHost]) -> String {
    let mut names: Vec<&str> = hosts.iter().map(|h| h.host.as_str()).take(20).collect();
    if hosts.len() > 20 {
        names.push("…");
    }
    names.join(", ")
}

/// Шаги одного экспортёра: DNS/TCP, `/metrics`, карты. Карты — при успехе.
async fn exporter_steps(
    e: &crate::storage::profile::GpuExporter,
    d: &DirectGpu,
    timeouts: Timeouts,
    password: Option<String>,
    policy: &PolicyStatus,
    steps: &mut Vec<TestStep>,
) -> Vec<GpuCard> {
    let shown = redact_url(&e.url);
    let opened = exporter_server(e, d).and_then(|server| {
        let url = parse_exporter_url(e)?;
        Ok((
            server,
            url,
            Exporter::new(e, d, timeouts, password, policy)?,
        ))
    });
    let (server, url, exporter) = match opened {
        Ok(x) => x,
        Err(err) => {
            steps.push(failed_step("config", err));
            return Vec::new();
        }
    };
    if let Some((host, port, via_proxy)) = tcp_target(&server, &url) {
        let limit = Duration::from_millis(timeouts.connect_ms);
        if let Err(err) = tcp_probe(&host, port, limit).await {
            steps.push(failed_step("tcp", tcp_failure(err, &host, port, via_proxy)));
            return Vec::new();
        }
    }
    let cards = match exporter.cards().await {
        Ok(c) => c,
        Err(err) => {
            let step = match err.code {
                ErrorCode::AuthFailed => "auth",
                ErrorCode::TlsError => "tls",
                _ => "metrics",
            };
            steps.push(failed_step(step, err));
            return Vec::new();
        }
    };
    if cards.is_empty() {
        steps.push(failed_step(
            "metrics",
            app_err!(
                ErrorCode::InvalidInput,
                "errors.gpu.noDcgm",
                "{{url}} answers, but there are no DCGM metrics (DCGM_FI_DEV_FB_USED): check the exporter address",
                url = shown
            ),
        ));
        return Vec::new();
    }
    let hosts = hosts_of(cards.iter().map(|c| (c.host.as_str(), c.index)));
    let driver = cards
        .iter()
        .find_map(|c| c.driver.clone())
        .unwrap_or_else(|| "—".into());
    steps.push(passed(
        "cards",
        ltext!(
            "messages.gpuTest.exporterCards",
            "{{url}}: {{count}} GPUs on {{hosts}}, driver {{driver}}",
            url = shown,
            count = cards.len(),
            hosts = hosts_text(&hosts),
            driver = driver
        ),
    ));
    cards
}

/// Ошибка «нет источника истории» для режима `prometheus`.
fn no_history() -> AppError {
    app_err!(
        ErrorCode::InvalidInput,
        "errors.gpu.noHistorySource",
        "GPU metrics from the history source need a metrics history source: add it to the installation"
    )
}

/// Шаги режима `prometheus`: карты под селектором (или под подобранным), их число.
async fn prometheus_steps(
    g: &PrometheusGpu,
    p: &InstallationProfile,
    client: &HistoryClient,
    steps: &mut Vec<TestStep>,
    out: &mut GpuCheck,
) -> Vec<FoundCard> {
    let all = match client.query(CARDS).await {
        Ok(s) => found_cards(&s),
        Err(e) => {
            steps.push(failed_step("history", e));
            return Vec::new();
        }
    };
    let mut available: Vec<String> = all.iter().map(|c| c.host.clone()).collect();
    available.dedup();
    out.available = available;
    let chosen: Vec<FoundCard> = match &g.selector {
        Some(selector) => {
            let matchers = match parse_selector(selector) {
                Ok(m) => m,
                Err(e) => {
                    steps.push(failed_step("cards", e));
                    return Vec::new();
                }
            };
            let expr = format!(
                "count by (hostname, Hostname, instance, gpu) ({PRESENCE}{{{}}})",
                render(&matchers)
            );
            match client.query(&expr).await {
                Ok(s) => found_cards(&s),
                Err(e) => {
                    steps.push(failed_step("cards", e));
                    return Vec::new();
                }
            }
        }
        None => match suggest_selector(&p.servers, &all) {
            Some(selector) => {
                let own: Vec<String> = p.servers.iter().filter_map(server_host).collect();
                let hosts: Vec<String> = all
                    .iter()
                    .filter(|c| own.iter().any(|h| same_host(h, &c.host)))
                    .map(|c| c.host.clone())
                    .collect();
                out.selector = Some(selector.clone());
                steps.push(passed(
                    "cards",
                    ltext!(
                        "messages.gpuTest.selectorAuto",
                        "Card selector {{selector}} (auto-match)",
                        selector = selector
                    ),
                ));
                all.iter()
                    .filter(|c| hosts.contains(&c.host))
                    .cloned()
                    .collect()
            }
            None if all.is_empty() => Vec::new(),
            None => {
                let hosts = hosts_of(all.iter().map(|c| (c.host.as_str(), c.index)));
                steps.push(warning_step(
                    "cards",
                    ltext!(
                        "messages.gpuTest.noServerHosts",
                        "The hosts of the servers are not among the GPU hosts — choose the hosts: {{hosts}}",
                        hosts = hosts_text(&hosts)
                    ),
                ));
                all.clone()
            }
        },
    };
    if chosen.is_empty() {
        let w = if g.selector.is_some() {
            ltext!(
                "messages.gpuTest.noCardsUnderSelector",
                "There are no GPUs under the card selector"
            )
        } else {
            ltext!(
                "messages.gpuTest.noDcgmInHistory",
                "The history source does not collect DCGM metrics"
            )
        };
        steps.push(warning_step("cards", w));
        return chosen;
    }
    if chosen.len() > MAX_GPUS {
        steps.push(failed_step("cards", too_many_gpus(chosen.len())));
        return Vec::new();
    }
    let hosts = hosts_of(chosen.iter().map(|c| (c.host.as_str(), c.index)));
    steps.push(passed(
        "cards",
        ltext!(
            "messages.gpuTest.historyCards",
            "{{count}} GPUs on {{hosts}}",
            count = chosen.len(),
            hosts = hosts_text(&hosts)
        ),
    ));
    chosen
}

/// Шаг привязки сервера.
fn bind_step(
    server: &crate::storage::profile::ServerConfig,
    hosts: &[FoundHost],
    out: &mut Vec<BindingSuggestion>,
) -> TestStep {
    let name = &server.name;
    let pairs: Vec<(String, Vec<u32>)> = hosts
        .iter()
        .map(|h| (h.host.clone(), h.cards.clone()))
        .collect();
    if let Some(b) = &server.gpus {
        return match binding_state(b, &pairs) {
            BindingState::Ok => passed(
                "bind",
                ltext!(
                    "messages.gpuTest.bound",
                    "“{{server}}”: {{host}}, {{cards}}",
                    server = name,
                    host = b.host,
                    cards = cards_text(b)
                ),
            ),
            BindingState::NoHost => warning_step(
                "bind",
                ltext!(
                    "messages.gpuTest.boundHostMissing",
                    "“{{server}}”: the bound host {{host}} is not among the GPU hosts",
                    server = name,
                    host = b.host
                ),
            ),
            BindingState::NoIndices(absent) => warning_step(
                "bind",
                ltext!(
                    "messages.gpuTest.boundCardsMissing",
                    "“{{server}}”: host {{host}} has no GPUs {{cards}}",
                    server = name,
                    host = b.host,
                    cards = absent
                        .iter()
                        .map(u32::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ),
        };
    }
    let names: Vec<String> = hosts.iter().map(|h| h.host.clone()).collect();
    let binding = suggest_binding(server, &names);
    out.push(BindingSuggestion {
        server_id: server.id.clone(),
        binding: binding.clone(),
    });
    match binding {
        Some(b) => passed(
            "bind",
            ltext!(
                "messages.gpuTest.boundAuto",
                "“{{server}}”: {{host}}, all GPUs (auto-match)",
                server = name,
                host = b.host
            ),
        ),
        None => warning_step(
            "bind",
            ltext!(
                "messages.gpuTest.unbound",
                "“{{server}}”: the host of the server is not among the GPU hosts — choose the GPU host manually",
                server = name
            ),
        ),
    }
}

/// Номера карт привязки для текста шага.
fn cards_text(b: &GpuBinding) -> String {
    match &b.indices {
        None => "*".into(),
        Some(list) => list
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// Шаг сопоставления экспортёра с историей: селектор или автоподбор цели.
async fn history_step(
    client: &HistoryClient,
    e: &crate::storage::profile::GpuExporter,
    targets: &[DcgmTarget],
    out: &mut Vec<ExporterSuggestion>,
) -> TestStep {
    let shown = redact_url(&e.url);
    if let Some(selector) = &e.history_selector {
        let matchers = match parse_selector(selector) {
            Ok(m) => m,
            Err(err) => return failed_step("history", err),
        };
        let expr = format!("count({PRESENCE}{{{}}})", render(&matchers));
        return match client.query(&expr).await {
            Ok(found) if found.iter().any(|s| s.value.is_some_and(|v| v > 0.0)) => passed(
                "history",
                ltext!(
                    "messages.gpuTest.historySelector",
                    "{{url}}: {{selector}}",
                    url = shown,
                    selector = selector_label(selector)
                ),
            ),
            Ok(_) => warning_step(
                "history",
                ltext!(
                    "messages.gpuTest.historyNoSeries",
                    "{{url}}: there are no DCGM series under the selector {{selector}}",
                    url = shown,
                    selector = selector_label(selector)
                ),
            ),
            Err(err) => failed_step("history", err),
        };
    }
    let found: Vec<DcgmTarget> = parse_exporter_url(e)
        .map(|url| {
            matching_exporter_targets(&url, targets)
                .into_iter()
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    out.push(ExporterSuggestion {
        url: e.url.clone(),
        history_selector: found.first().map(DcgmTarget::selector),
        targets: found.clone(),
    });
    match found.first() {
        Some(t) => passed(
            "history",
            ltext!(
                "messages.gpuTest.historyAuto",
                "{{url}}: {{job}} · {{instance}} (auto-match)",
                url = shown,
                job = t.job,
                instance = t.instance
            ),
        ),
        None => warning_step(
            "history",
            ltext!(
                "messages.gpuTest.historyNoTarget",
                "{{url}}: no history target with the address of the exporter — GPU history windows are unavailable",
                url = shown
            ),
        ),
    }
}

/// Шаги-подсказки о группах счётчиков, которых нет ни у одной карты.
fn counter_steps(groups: &[Group]) -> Vec<TestStep> {
    groups
        .iter()
        .map(|g| {
            let message = match g {
                Group::Sm => ltext!(
                    "messages.gpuTest.missingSm",
                    "The exporter does not collect SM activity (DCGM_FI_PROF_SM_ACTIVE): graphics engine activity is shown instead"
                ),
                Group::Ecc => ltext!(
                    "messages.gpuTest.missingEcc",
                    "The exporter does not collect ECC errors (DCGM_FI_DEV_ECC_DBE_VOL_TOTAL, DCGM_FI_DEV_ECC_SBE_VOL_TOTAL)"
                ),
                Group::Throttle => ltext!(
                    "messages.gpuTest.missingThrottle",
                    "The exporter does not collect throttle reasons (DCGM_FI_DEV_CLOCKS_EVENT_REASONS)"
                ),
                Group::NvlinkErrors => ltext!(
                    "messages.gpuTest.missingNvlink",
                    "The exporter does not collect NVLink errors (DCGM_FI_DEV_NVLINK_*_ERROR_TOTAL)"
                ),
            };
            warning_step("counters", message)
        })
        .collect()
}

/// Проверка компонента GPU профиля `p`; `password` — пароль экспортёров, `history_secret` —
/// секрет источника истории (из формы или keychain).
pub async fn check_gpu(
    p: &InstallationProfile,
    password: Option<String>,
    history_secret: Option<String>,
    policy: &PolicyStatus,
) -> GpuCheck {
    let mut out = GpuCheck {
        report: report(Vec::new(), Vec::new()),
        hosts: Vec::new(),
        available: Vec::new(),
        selector: None,
        bindings: Vec::new(),
        exporters: Vec::new(),
    };
    let finish = |mut out: GpuCheck, steps: Vec<TestStep>| {
        out.report = report(steps, Vec::new());
        out
    };
    let Some(g) = &p.gpu else {
        let e = app_err!(
            ErrorCode::InvalidInput,
            "errors.gpu.notConfigured",
            "The installation has no GPU component"
        );
        return finish(out, vec![failed_step("config", e)]);
    };
    if let Err(e) = validate_gpu(g, p.history.is_some()) {
        return finish(out, vec![failed_step("config", e)]);
    }
    let timeouts = Timeouts::from(p);
    let history = match &p.history {
        Some(h) => match HistoryClient::new(h, timeouts, history_secret, policy) {
            Ok(c) => Some(c),
            Err(e) if matches!(g, GpuSource::Prometheus(_)) => {
                return finish(out, vec![failed_step("config", e)]);
            }
            Err(_) => None,
        },
        None => None,
    };
    let mut steps = vec![passed(
        "config",
        ltext!("messages.connectionTest.configOk", "Configuration is valid"),
    )];
    match g {
        GpuSource::Direct(d) => {
            let mut cards = Vec::new();
            for e in &d.exporters {
                cards.extend(
                    exporter_steps(e, d, timeouts, password.clone(), policy, &mut steps).await,
                );
            }
            if cards.len() > MAX_GPUS {
                steps.push(failed_step("cards", too_many_gpus(cards.len())));
            }
            let groups = missing(&cards);
            let hosts = hosts_of(cards.iter().map(|c| (c.host.as_str(), c.index)));
            out.hosts = hosts.clone();
            if !cards.is_empty() {
                for server in &p.servers {
                    let step = bind_step(server, &hosts, &mut out.bindings);
                    steps.push(step);
                }
            }
            if let Some(client) = &history {
                match client.query(TARGETS).await {
                    Ok(t) => {
                        let targets = targets_from(&t);
                        for e in &d.exporters {
                            let step = history_step(client, e, &targets, &mut out.exporters).await;
                            steps.push(step);
                        }
                    }
                    Err(e) => steps.push(failed_step("history", e)),
                }
            }
            if !cards.is_empty() {
                steps.extend(counter_steps(&groups));
            }
        }
        GpuSource::Prometheus(pg) => {
            let Some(client) = &history else {
                return finish(out, vec![failed_step("config", no_history())]);
            };
            let chosen = prometheus_steps(pg, p, client, &mut steps, &mut out).await;
            let hosts = hosts_of(chosen.iter().map(|c| (c.host.as_str(), c.index)));
            out.hosts = hosts.clone();
            if !chosen.is_empty() {
                for server in &p.servers {
                    let step = bind_step(server, &hosts, &mut out.bindings);
                    steps.push(step);
                }
            }
        }
    }
    finish(out, steps)
}
````

<!-- block: file t07-src-tauri-src-gpu-read-rs step=7.3 path=src-tauri/src/gpu/read.rs part=impl -->
````
//! Окно истории GPU (спека I1d §4): по какой метке подписан хост в источнике истории, шаблоны
//! полей, не больше 4 запросов одновременно (общий предел источника), сборка рядов; сбой одного
//! запроса — `partial`, всех — ошибка. На всё окно — срок источника, как у окна сервера.

use futures::StreamExt;
use serde::Serialize;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::gpu::bind::same_host;
use crate::gpu::promql::{hosts_expr, queries};
use crate::gpu::series::{GpuSeries, assemble, series_host};
use crate::history::client::SourceKind;
use crate::history::read::{CONCURRENCY, Source, permit};
use crate::history::series::Partial;
use crate::history::targets::{escape, parse_selector, render};
use crate::history::window::{Window, range};

/// Ответ `read_gpu_history`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuHistoryAnswer {
    pub window: Window,
    pub step_sec: u64,
    pub source: SourceKind,
    pub series: Vec<GpuSeries>,
    pub partial: Vec<Partial>,
}

fn host_not_in_history(host: &str) -> AppError {
    app_err!(
        ErrorCode::NotFound,
        "errors.gpu.hostNotInHistory",
        "The metrics history has no GPU series of host {{host}}",
        host = host
    )
}

/// Базовые условия окна из селектора (`None` — без условий, все карты источника).
pub fn base(selector: Option<&str>) -> AppResult<String> {
    selector.map_or(Ok(String::new()), |s| Ok(render(&parse_selector(s)?)))
}

/// Условия серий хоста `host` под базовыми условиями `bases` (`base`): первая база, где хост
/// нашёлся, плюс точное условие на метку, которой он подписан (`hostname`, `Hostname` или
/// `instance`).
async fn host_matchers(source: &Source, bases: &[String], host: &str) -> AppResult<String> {
    for matchers in bases {
        let found = {
            let _permit = permit(source).await;
            source.client.query(&hosts_expr(matchers)).await?
        };
        for s in found {
            if !series_host(&s.labels).is_some_and(|h| same_host(&h, host)) {
                continue;
            }
            let get = |k: &str| s.labels.get(k).filter(|v| !v.is_empty());
            let (label, value) = match (get("hostname"), get("Hostname"), get("instance")) {
                (Some(v), _, _) => ("hostname", v),
                (None, Some(v), _) => ("Hostname", v),
                (None, None, Some(v)) => ("instance", v),
                _ => continue,
            };
            let exact = format!("{label}=\"{}\"", escape(value));
            return Ok(if matchers.is_empty() {
                exact
            } else {
                format!("{matchers},{exact}")
            });
        }
    }
    Err(host_not_in_history(host))
}

/// Окно `window` карт хоста `host` (одной карты `gpu` или всех), которое кончается в `now_sec`.
pub async fn read_history(
    source: &Source,
    bases: &[String],
    host: &str,
    gpu: Option<u32>,
    window: Window,
    now_sec: u64,
) -> AppResult<GpuHistoryAnswer> {
    let read = read_window(source, bases, host, gpu, window, now_sec);
    match tokio::time::timeout(source.deadline(), read).await {
        Ok(answer) => answer,
        Err(_) => Err(app_err!(
            ErrorCode::Timeout,
            "errors.history.deadline",
            "The metrics history source did not return the window in {{seconds}} s: choose a shorter window or raise the request timeout",
            seconds = source.deadline().as_secs()
        )),
    }
}

async fn read_window(
    source: &Source,
    bases: &[String],
    host: &str,
    gpu: Option<u32>,
    window: Window,
    now_sec: u64,
) -> AppResult<GpuHistoryAnswer> {
    let mut matchers = host_matchers(source, bases, host).await?;
    if let Some(g) = gpu {
        matchers.push_str(&format!(",gpu=\"{g}\""));
    }
    let r = range(window, now_sec, None);
    let results: Vec<_> = futures::stream::iter(queries(&matchers, r.rate))
        .map(|q| async move {
            let _permit = permit(source).await;
            let result = source.client.query_range(&q.expr, &r).await;
            (q, result)
        })
        .buffered(CONCURRENCY)
        .collect()
        .await;
    if results.iter().all(|(_, res)| res.is_err()) {
        let first: Option<AppError> = results.into_iter().find_map(|(_, res)| res.err());
        return Err(first.unwrap_or_else(|| AppError::internal_with("no GPU history queries")));
    }
    let (series, mut partial, truncated) = assemble(&r, results);
    if truncated {
        partial.push(Partial {
            chart: "all",
            error: app_err!(
                ErrorCode::InvalidInput,
                "errors.gpu.tooManySeries",
                "The host has more than {{max}} GPUs: the first {{max}} are shown",
                max = crate::gpu::series::MAX_SERIES
            ),
        });
    }
    Ok(GpuHistoryAnswer {
        window,
        step_sec: r.step,
        source: source.kind,
        series,
        partial,
    })
}
````

<!-- block: patch t07-src-tauri-src-history-read-rs step=7.3 -->
````
diff --git a/src-tauri/src/history/read.rs b/src-tauri/src/history/read.rs
index 9cc8193..fe9c59d 100644
--- a/src-tauri/src/history/read.rs
+++ b/src-tauri/src/history/read.rs
@@ -54,6 +54,11 @@ impl Source {
             deadline,
         }
     }
+
+    /// Срок чтения одного окна.
+    pub fn deadline(&self) -> Duration {
+        self.deadline
+    }
 }
 
 /// Значение метки селектора с условием `=`: по `job` и `instance` ищется интервал опроса цели.
````

<!-- block: patch t07-src-tauri-src-servers-pool-rs step=7.3 -->
````
diff --git a/src-tauri/src/servers/pool.rs b/src-tauri/src/servers/pool.rs
index 53f6096..fb0fd64 100644
--- a/src-tauri/src/servers/pool.rs
+++ b/src-tauri/src/servers/pool.rs
@@ -50,6 +50,9 @@ impl Connected {
 /// Снимки метрик одного сервера за последнюю минуту.
 pub type LiveHistory = Arc<tokio::sync::Mutex<VecDeque<Timed>>>;
 
+/// Снимки счётчиков карт инсталляции за последнюю минуту (спека I1d §4).
+pub type GpuLive = Arc<tokio::sync::Mutex<VecDeque<crate::gpu::live::Timed>>>;
+
 #[derive(Default)]
 pub struct ServerPool {
     connected: Mutex<HashMap<String, Arc<Connected>>>,
@@ -61,6 +64,10 @@ pub struct ServerPool {
     live: Mutex<HashMap<(String, String), LiveHistory>>,
     /// Открытые источники истории метрик (спека I1c §4), по инсталляции.
     history: Mutex<HashMap<String, Arc<crate::history::read::Source>>>,
+    /// Открытые компоненты GPU (спека I1d §4), по инсталляции.
+    gpu: Mutex<HashMap<String, Arc<crate::gpu::source::OpenGpu>>>,
+    /// Снимки счётчиков карт за последнюю минуту, по инсталляции.
+    gpu_live: Mutex<HashMap<String, GpuLive>>,
 }
 
 impl ServerPool {
@@ -82,6 +89,37 @@ impl ServerPool {
             .lock_safe()
             .retain(|(installation, _), _| installation != id);
         self.history.lock_safe().remove(id);
+        self.gpu.lock_safe().remove(id);
+        self.gpu_live.lock_safe().remove(id);
+    }
+
+    /// Открытый компонент GPU инсталляции; `evict` его забывает.
+    pub fn gpu(&self, id: &str) -> Option<Arc<crate::gpu::source::OpenGpu>> {
+        self.gpu.lock_safe().get(id).cloned()
+    }
+
+    /// Кладёт компонент GPU, только если с момента `generation` инсталляцию не сбрасывали.
+    pub fn insert_gpu_if_current(
+        &self,
+        id: &str,
+        generation: u64,
+        open: Arc<crate::gpu::source::OpenGpu>,
+    ) -> bool {
+        let generations = self.generations.lock_safe();
+        if generations.get(id).copied().unwrap_or(0) != generation {
+            return false;
+        }
+        self.gpu.lock_safe().insert(id.to_string(), open);
+        true
+    }
+
+    /// Снимки счётчиков карт инсталляции; `evict` их сбрасывает.
+    pub fn gpu_live(&self, id: &str) -> GpuLive {
+        self.gpu_live
+            .lock_safe()
+            .entry(id.to_string())
+            .or_default()
+            .clone()
     }
 
     /// Открытый источник истории инсталляции; `evict` его забывает.
````

<!-- block: locales t07-locales-t07 step=7.3 -->
````
{
  "en": {
    "errors": { "gpu": {
      "hostNotInHistory": "The metrics history has no GPU series of host {{host}}",
      "noDcgm": "{{url}} answers, but there are no DCGM metrics (DCGM_FI_DEV_FB_USED): check the exporter address",
      "notConfigured": "The installation has no GPU component",
      "notMatched": "The DCGM exporters are not matched with the metrics history: choose their targets in the installation",
      "tooManySeries": "The host has more than {{max}} GPUs: the first {{max}} are shown"
    } },
    "messages": { "gpuTest": {
      "bound": "“{{server}}”: {{host}}, {{cards}}",
      "boundAuto": "“{{server}}”: {{host}}, all GPUs (auto-match)",
      "boundCardsMissing": "“{{server}}”: host {{host}} has no GPUs {{cards}}",
      "boundHostMissing": "“{{server}}”: the bound host {{host}} is not among the GPU hosts",
      "exporterCards": "{{url}}: {{count}} GPUs on {{hosts}}, driver {{driver}}",
      "historyAuto": "{{url}}: {{job}} · {{instance}} (auto-match)",
      "historyCards": "{{count}} GPUs on {{hosts}}",
      "historyNoSeries": "{{url}}: there are no DCGM series under the selector {{selector}}",
      "historyNoTarget": "{{url}}: no history target with the address of the exporter — GPU history windows are unavailable",
      "historySelector": "{{url}}: {{selector}}",
      "missingEcc": "The exporter does not collect ECC errors (DCGM_FI_DEV_ECC_DBE_VOL_TOTAL, DCGM_FI_DEV_ECC_SBE_VOL_TOTAL)",
      "missingNvlink": "The exporter does not collect NVLink errors (DCGM_FI_DEV_NVLINK_*_ERROR_TOTAL)",
      "missingSm": "The exporter does not collect SM activity (DCGM_FI_PROF_SM_ACTIVE): graphics engine activity is shown instead",
      "missingThrottle": "The exporter does not collect throttle reasons (DCGM_FI_DEV_CLOCKS_EVENT_REASONS)",
      "noCardsUnderSelector": "There are no GPUs under the card selector",
      "noDcgmInHistory": "The history source does not collect DCGM metrics",
      "noServerHosts": "The hosts of the servers are not among the GPU hosts — choose the hosts: {{hosts}}",
      "selectorAuto": "Card selector {{selector}} (auto-match)",
      "unbound": "“{{server}}”: the host of the server is not among the GPU hosts — choose the GPU host manually"
    } }
  },
  "ru": {
    "errors": { "gpu": {
      "hostNotInHistory": "В истории метрик нет серий GPU хоста {{host}}",
      "noDcgm": "{{url}} отвечает, но метрик DCGM нет (DCGM_FI_DEV_FB_USED): проверьте адрес экспортёра",
      "notConfigured": "У инсталляции нет компонента GPU",
      "notMatched": "Экспортёры DCGM не сопоставлены с историей метрик: выберите их цели в инсталляции",
      "tooManySeries": "На хосте больше {{max}} карт: показаны первые {{max}}"
    } },
    "messages": { "gpuTest": {
      "bound": "«{{server}}»: {{host}}, {{cards}}",
      "boundAuto": "«{{server}}»: {{host}}, все карты (автоподбор)",
      "boundCardsMissing": "«{{server}}»: на хосте {{host}} нет карт {{cards}}",
      "boundHostMissing": "«{{server}}»: хоста привязки {{host}} нет среди хостов GPU",
      "exporterCards": "{{url}}: карт {{count}} на {{hosts}}, драйвер {{driver}}",
      "historyAuto": "{{url}}: {{job}} · {{instance}} (автоподбор)",
      "historyCards": "Карт {{count}} на {{hosts}}",
      "historyNoSeries": "{{url}}: под селектором {{selector}} нет серий DCGM",
      "historyNoTarget": "{{url}}: цели с адресом экспортёра в истории нет — окна истории GPU недоступны",
      "historySelector": "{{url}}: {{selector}}",
      "missingEcc": "Экспортёр не собирает ошибки ECC (DCGM_FI_DEV_ECC_DBE_VOL_TOTAL, DCGM_FI_DEV_ECC_SBE_VOL_TOTAL)",
      "missingNvlink": "Экспортёр не собирает ошибки NVLink (DCGM_FI_DEV_NVLINK_*_ERROR_TOTAL)",
      "missingSm": "Экспортёр не собирает активность SM (DCGM_FI_PROF_SM_ACTIVE): вместо неё показана активность graphics engine",
      "missingThrottle": "Экспортёр не собирает причины троттлинга (DCGM_FI_DEV_CLOCKS_EVENT_REASONS)",
      "noCardsUnderSelector": "Под селектором карт нет ни одной карты",
      "noDcgmInHistory": "Источник истории не собирает метрики DCGM",
      "noServerHosts": "Хостов серверов нет среди хостов GPU — выберите хосты: {{hosts}}",
      "selectorAuto": "Селектор карт {{selector}} (автоподбор)",
      "unbound": "«{{server}}»: хоста сервера нет среди хостов GPU — выберите хост GPU вручную"
    } }
  }
}
````

- [ ] **Шаг 7.4: Проверки.**

Run: `cd src-tauri && cargo test --lib -q`
Expected: `test result: ok. 345 passed`
Run: `cd src-tauri && cargo clippy --lib --tests -q -- -D warnings && cargo fmt --check`
Expected: без вывода
Run: `pnpm i18n:check`
Expected: `✓ словари en, ru согласованы, вызовов из Rust: 187`

- [ ] **Шаг 7.5: Коммит.**

<!-- block: shell t07-commit step=7.5 -->
````
git add src-tauri/src/commands/gpu.rs \
  src-tauri/src/commands/history.rs \
  src-tauri/src/commands/mod.rs \
  src-tauri/src/gpu/check.rs \
  src-tauri/src/gpu/mod.rs \
  src-tauri/src/gpu/read.rs \
  src-tauri/src/history/read.rs \
  src-tauri/src/lib.rs \
  src-tauri/src/servers/pool.rs \
  src/locales/en.json \
  src/locales/ru.json
git commit -s -q -F - <<'EOF'
feat(gpu): check_gpu_source, read_gpu_status and read_gpu_history commands

The GPU check goes through the configuration, each exporter (DNS/TCP,
/metrics, cards) or the cards of the history source with an auto-matched
card selector and the 64 GPU limit, then the binding of every server,
the history targets of the exporters and hints for counter groups no
card has. The open GPU component and minute counter snapshots live in
the connection pool until the installation is evicted. A history window
finds the label of the host under the card or exporter selectors and
reads the field templates within the source deadline.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
````

---

### Задача 8: Типы, клиент IPC и хуки GPU

Типы компонента GPU, привязки, снимка карт, проверки и окна истории; вызовы `checkGpuSource`, `readGpuStatus`,
`readGpuHistory` (спека I1d §5). `useGpuStatus` опрашивает раз в 5 с, пока вкладка видна, и держит час точек на карту;
`useGpuHistory` держит окно минуту; `nowSeries` ставит историю окна `now` до первой живой точки карты.

**Files:**
- Create: `src/components/gpu/testGpu.ts`
- Create: `src/components/gpu/useGpu.test.tsx`
- Create: `src/components/gpu/useGpu.ts`
- Modify: `src/api/client.test.ts`
- Modify: `src/api/client.ts`
- Modify: `src/api/types.ts`

**Interfaces:**
- Produces: типы `GpuBinding`, `GpuAuth`, `GpuExporter`, `GpuSource`, `GpuField`, `GpuValues`, `GpuGroup`,
  `GpuCard`, `GpuHost`, `GpuStatus`, `DcgmTarget`, `GpuCheck`, `GpuHistoryAnswer` (`src/api/types.ts`), поля
  `ServerConfig.gpus`, `InstallationProfile.gpu`, `SecretsInput.gpuSecret`, `SavedSecrets.gpu`, шаги `TestStepName`;
  `api.checkGpuSource(profile, secrets)`, `api.readGpuStatus(installationId)`, `api.readGpuHistory(installationId,
  window, host, gpu = null)`; `src/components/gpu/useGpu.ts`: `useGpuStatus(installationId, enabled = true)` →
  `{ status, error, isPending, buffer, refetch }`, `useGpuHistory(installationId, window, host, gpu, enabled)`,
  `GpuNowPoint`, `GpuBuffer`, `cardKey`, `appendStatus`, `appendError`, `gpuStatusKey`, `gpuHistoryKey`, `nowSeries`;
  тестовые `card`, `host`, `status` (`testGpu.ts`).

- [ ] **Шаг 8.1: Тесты.** Тесты клиента, буфера «Сейчас» и хуков.

<!-- block: patch t08-src-api-client-test-ts step=8.1 -->
````
diff --git a/src/api/client.test.ts b/src/api/client.test.ts
index 376e86b..dab6f5d 100644
--- a/src/api/client.test.ts
+++ b/src/api/client.test.ts
@@ -31,6 +31,23 @@ describe("api client", () => {
     expect(invoke).toHaveBeenCalledWith("check_history_source", { input: { profile, secrets: { historySecret: "t" } } });
   });
 
+  it("checkGpuSource передаёт профиль формы и пароль экспортёров", async () => {
+    invoke.mockResolvedValue({ report: { ok: true, steps: [], warnings: [] }, hosts: [], available: [], bindings: [], exporters: [] });
+    const profile = { id: "a" } as InstallationProfile;
+    await api.checkGpuSource(profile, { gpuSecret: "p" });
+    expect(invoke).toHaveBeenCalledWith("check_gpu_source", { input: { profile, secrets: { gpuSecret: "p" } } });
+  });
+
+  it("readGpuStatus и readGpuHistory: инсталляция, окно, хост и карта (без карты — null)", async () => {
+    invoke.mockResolvedValue({});
+    await api.readGpuStatus("i1");
+    expect(invoke).toHaveBeenLastCalledWith("read_gpu_status", { installationId: "i1" });
+    await api.readGpuHistory("i1", "1h", "gpu-node-1", 3);
+    expect(invoke).toHaveBeenLastCalledWith("read_gpu_history", { installationId: "i1", window: "1h", host: "gpu-node-1", gpu: 3 });
+    await api.readGpuHistory("i1", "now", "gpu-node-1");
+    expect(invoke).toHaveBeenLastCalledWith("read_gpu_history", { installationId: "i1", window: "now", host: "gpu-node-1", gpu: null });
+  });
+
   it("testServer передаёт профиль формы, ключи и id сервера", async () => {
     invoke.mockResolvedValue({ ok: true, steps: [], warnings: [] });
     const profile = { id: "", name: "x" } as InstallationProfile;
````

<!-- block: file t08-src-components-gpu-testgpu-ts step=8.1 path=src/components/gpu/testGpu.ts -->
````
import type { GpuCard, GpuHost, GpuStatus } from "../../api/types";

/** Карта для тестов: H100 с полным набором полей; `over` — свои значения. */
export function card(index: number, over: Partial<GpuCard> = {}): GpuCard {
  return {
    index,
    uuid: `GPU-${index}`,
    model: "NVIDIA H100 80GB HBM3",
    smSource: "sm",
    values: { util: 60 + index, smActive: 55, memUsed: 61440, memTotal: 81559, temp: 60, power: 450, smClock: 1980, memClock: 2619, throttle: 0, xid: 0, eccDbe: 0, nvlinkErrors: 0, pcieReplay: 0 },
    deltas: { eccDbe: 0, pcieReplay: 0, nvlinkErrors: 0 },
    ...over,
  };
}

export function host(name: string, cards: GpuCard[], driver = "570.124.06"): GpuHost {
  return { host: name, driver, cards };
}

export function status(at: number, hosts: GpuHost[], over: Partial<GpuStatus> = {}): GpuStatus {
  return { at, hosts, errors: [], missing: [], deltaSec: 5, ...over };
}
````

<!-- block: file t08-src-components-gpu-usegpu-test-tsx step=8.1 path=src/components/gpu/useGpu.test.tsx -->
````
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api/client", async (importOriginal) => {
  const orig = await importOriginal<typeof import("../../api/client")>();
  return { ...orig, api: { readGpuStatus: vi.fn(), readGpuHistory: vi.fn() } };
});

import { api } from "../../api/client";
import type { GpuHistoryAnswer } from "../../api/types";
import { TabActiveContext } from "../../lib/tabActive";
import { card, host, status } from "./testGpu";
import { appendError, appendStatus, cardKey, nowSeries, useGpuHistory, useGpuStatus } from "./useGpu";

describe("буфер «Сейчас» карт", () => {
  it("точка на каждую карту, старше окна отбрасываются, повтор снимка не дублируется", () => {
    let b = {};
    for (const at of [0, 5_000, 10_000, 10_000]) b = appendStatus(b, status(at, [host("gpu-1", [card(0), card(1)])]), 7_000);
    expect(Object.keys(b)).toEqual([cardKey("gpu-1", 0), cardKey("gpu-1", 1)]);
    expect((b as Record<string, { at: number }[]>)[cardKey("gpu-1", 1)].map((p) => p.at)).toEqual([5_000, 10_000]);
  });

  it("неудачный опрос — разрыв у всех карт", () => {
    const b = appendError(appendStatus({}, status(0, [host("gpu-1", [card(0)])])), 5_000);
    expect(b[cardKey("gpu-1", 0)]).toEqual([{ at: 0, values: card(0).values }, { at: 5_000, values: null }]);
  });

  it("ряды окна «Сейчас»: история раньше первой живой точки, затем буфер", () => {
    const b = appendStatus({}, status(30_000, [host("gpu-1", [card(0), card(1)])]));
    const history: GpuHistoryAnswer = {
      window: "now",
      stepSec: 15,
      source: "prometheus",
      series: [{ host: "gpu-1", gpu: 0, points: [{ at: 15_000, values: { util: 1 } }, { at: 30_000, values: { util: 2 } }] }],
      partial: [],
    };
    const s = nowSeries(b, "gpu-1", [0, 1, 2], history);
    expect(s[0].map((p) => p.at)).toEqual([15_000, 30_000]);
    expect(s[0][0].values).toEqual({ util: 1 });
    expect(s[1].map((p) => p.at)).toEqual([30_000]);
    expect(s[2]).toEqual([]);
    expect(nowSeries(b, "gpu-1", [0])[0]).toHaveLength(1);
  });
});

function wrapper(active: boolean) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>
      <TabActiveContext.Provider value={active}>{children}</TabActiveContext.Provider>
    </QueryClientProvider>
  );
}

describe("useGpuStatus и useGpuHistory", () => {
  beforeEach(() => {
    vi.mocked(api.readGpuStatus).mockReset();
    vi.mocked(api.readGpuHistory).mockReset();
  });

  it("снимок попадает в буфер; ошибка опроса — в error", async () => {
    vi.mocked(api.readGpuStatus).mockResolvedValue(status(1_000, [host("gpu-1", [card(0)])]));
    const { result } = renderHook(() => useGpuStatus("i1"), { wrapper: wrapper(true) });
    await waitFor(() => expect(result.current.status?.at).toBe(1_000));
    await waitFor(() => expect(result.current.buffer[cardKey("gpu-1", 0)]).toHaveLength(1));
    expect(api.readGpuStatus).toHaveBeenCalledWith("i1");

    vi.mocked(api.readGpuStatus).mockRejectedValue({ code: "CONNECTION_FAILED", message: "down", retryable: true });
    const failed = renderHook(() => useGpuStatus("i2"), { wrapper: wrapper(true) });
    await waitFor(() => expect(failed.result.current.error?.code).toBe("CONNECTION_FAILED"));
  });

  it("выключенный хук не опрашивает; окно истории — по хосту и карте", async () => {
    renderHook(() => useGpuStatus("i1", false), { wrapper: wrapper(true) });
    vi.mocked(api.readGpuHistory).mockResolvedValue({ window: "1h", stepSec: 60, source: "prometheus", series: [], partial: [] });
    const { result } = renderHook(() => useGpuHistory("i1", "1h", "gpu-1", 2, true), { wrapper: wrapper(true) });
    await waitFor(() => expect(result.current.data?.window).toBe("1h"));
    expect(api.readGpuStatus).not.toHaveBeenCalled();
    expect(api.readGpuHistory).toHaveBeenCalledWith("i1", "1h", "gpu-1", 2);
  });
});
````

- [ ] **Шаг 8.2: Увидеть падение.**

Run: `pnpm exec vitest run src/components/gpu src/api`
Expected: FAIL — нет `./useGpu`, `api.checkGpuSource is not a function`

- [ ] **Шаг 8.3: Реализация.** Типы, клиент и хуки.

<!-- block: patch t08-src-api-client-ts step=8.3 -->
````
diff --git a/src/api/client.ts b/src/api/client.ts
index e9b2497..caf1a43 100644
--- a/src/api/client.ts
+++ b/src/api/client.ts
@@ -4,6 +4,9 @@ import type {
   AppInfo,
   ConfigEntry,
   ExportSummary,
+  GpuCheck,
+  GpuHistoryAnswer,
+  GpuStatus,
   HistoryAnswer,
   HistoryCheck,
   HistoryWindow,
@@ -71,6 +74,11 @@ export const api = {
     call<HistoryCheck>("check_history_source", { input: { profile, secrets } }),
   readServerHistory: (installationId: string, serverId: string, window: HistoryWindow) =>
     call<HistoryAnswer>("read_server_history", { installationId, serverId, window }),
+  checkGpuSource: (profile: InstallationProfile, secrets: SecretsInput = {}) =>
+    call<GpuCheck>("check_gpu_source", { input: { profile, secrets } }),
+  readGpuStatus: (installationId: string) => call<GpuStatus>("read_gpu_status", { installationId }),
+  readGpuHistory: (installationId: string, window: HistoryWindow, host: string, gpu: number | null = null) =>
+    call<GpuHistoryAnswer>("read_gpu_history", { installationId, window, host, gpu }),
 
   listFolders: () => call<InstallationFolder[]>("list_folders"),
   createFolder: (name: string, parentId: string | null) =>
````

<!-- block: patch t08-src-api-types-ts step=8.3 -->
````
diff --git a/src/api/types.ts b/src/api/types.ts
index 77c6e0b..4e8e0ed 100644
--- a/src/api/types.ts
+++ b/src/api/types.ts
@@ -52,8 +52,30 @@ export interface ServerConfig {
   proxy: ProxyConfig;
   /** Селектор серий сервера в источнике истории (`{job="vllm", instance="10.0.0.5:8000"}`); нет — истории нет. */
   historySelector?: string;
+  /** Карты GPU сервера (спека I1d §2); нет — сервер к GPU не привязан. */
+  gpus?: GpuBinding;
 }
 
+/** Привязка сервера к картам: хост и номера; без `indices` — все карты хоста. */
+export interface GpuBinding {
+  host: string;
+  indices?: number[];
+}
+
+/** Вход в экспортёры DCGM: пароль — только в keychain (спека I1d §2). */
+export type GpuAuth = { mode: "none" } | { mode: "basic"; username: string };
+
+/** Экспортёр DCGM: адрес `/metrics` и селектор его серий в источнике истории. */
+export interface GpuExporter {
+  url: string;
+  historySelector?: string;
+}
+
+/** Компонент GPU: экспортёры DCGM или серии DCGM источника истории под селектором карт. */
+export type GpuSource =
+  | { mode: "direct"; exporters: GpuExporter[]; auth: GpuAuth; ca: CaConfig; proxy: ProxyConfig }
+  | { mode: "prometheus"; selector?: string };
+
 /** Вход в источник истории: токен или пароль — только в keychain (спека I1c §2). */
 export type HistoryAuth = { mode: "none" } | { mode: "bearer" } | { mode: "basic"; username: string };
 
@@ -74,6 +96,8 @@ export interface InstallationProfile {
   servers: ServerConfig[];
   /** Источник истории метрик; нет — окна истории недоступны. */
   history?: HistorySource;
+  /** Компонент GPU; нет — карт у инсталляции нет. */
+  gpu?: GpuSource;
   requestTimeoutMs: number;
   connectTimeoutMs: number;
   folderId: string | null;
@@ -83,11 +107,12 @@ export interface InstallationProfile {
 
 /**
  * Ключи API из формы: id сервера → значение; сервера нет — оставить сохранённый, "" — удалить. Секрет источника
- * истории (токен или пароль) — по тем же правилам: нет поля — оставить, "" — удалить.
+ * истории (токен или пароль) и пароль экспортёров DCGM — по тем же правилам: нет поля — оставить, "" — удалить.
  */
 export interface SecretsInput {
   apiKeys?: Record<string, string>;
   historySecret?: string;
+  gpuSecret?: string;
 }
 
 /** Какие секреты сохранённой инсталляции уже лежат в keychain — только признаки, без значений. */
@@ -96,6 +121,8 @@ export interface SavedSecrets {
   apiKeys: string[];
   /** Сохранён секрет источника истории. */
   history?: boolean;
+  /** Сохранён пароль экспортёров DCGM. */
+  gpu?: boolean;
 }
 
 export interface InstallationFolder {
@@ -142,7 +169,22 @@ export interface TreeItemRef {
   id: string;
 }
 
-export type TestStepName = "config" | "tcp" | "tls" | "health" | "auth" | "kind" | "models" | "up" | "targets" | "match";
+export type TestStepName =
+  | "config"
+  | "tcp"
+  | "tls"
+  | "health"
+  | "auth"
+  | "kind"
+  | "models"
+  | "up"
+  | "targets"
+  | "match"
+  | "metrics"
+  | "cards"
+  | "bind"
+  | "history"
+  | "counters";
 
 export interface TestStep {
   name: TestStepName;
@@ -284,6 +326,102 @@ export interface HistoryAnswer {
   partial: { chart: string; error: AppError }[];
 }
 
+/** Поле карты GPU (спека I1d §4): проценты, МиБ, °C, Вт, МГц, Б/с, МБ/с; `throttle` — битовая маска причин. */
+export type GpuField =
+  | "util"
+  | "smActive"
+  | "tensorActive"
+  | "dramActive"
+  | "memCopyUtil"
+  | "memUsed"
+  | "memFree"
+  | "memReserved"
+  | "memTotal"
+  | "temp"
+  | "memTemp"
+  | "power"
+  | "smClock"
+  | "memClock"
+  | "throttle"
+  | "powerViolation"
+  | "thermalViolation"
+  | "xid"
+  | "xidCount"
+  | "eccDbe"
+  | "eccSbe"
+  | "remapUncorrectable"
+  | "remapCorrectable"
+  | "remapFailure"
+  | "pcieReplay"
+  | "pcieTx"
+  | "pcieRx"
+  | "nvlinkBandwidth"
+  | "nvlinkErrors";
+
+export type GpuValues = Partial<Record<GpuField, number>>;
+
+/** Группа счётчиков, которой нет ни у одной карты. */
+export type GpuGroup = "sm" | "ecc" | "throttle" | "nvlinkErrors";
+
+export interface GpuCard {
+  index: number;
+  uuid?: string;
+  model?: string;
+  pciBusId?: string;
+  workload?: { namespace: string; pod: string; container?: string };
+  mig?: { profile: string; instance: string }[];
+  /** Адрес экспортёра (режим `direct`). */
+  exporter?: string;
+  /** Откуда активность SM: `gr` — запасная серия graphics engine. */
+  smSource?: "sm" | "gr";
+  values: GpuValues;
+  /** Приросты за последнюю минуту опросов; у `powerViolation`, `thermalViolation` — доля времени, %. */
+  deltas?: GpuValues;
+}
+
+export interface GpuHost {
+  host: string;
+  driver?: string;
+  cards: GpuCard[];
+}
+
+/** Снимок карт инсталляции (`read_gpu_status`). */
+export interface GpuStatus {
+  at: number;
+  hosts: GpuHost[];
+  /** Сбои части источника: экспортёра (`exporter`) или всего источника. */
+  errors: { exporter?: string; error: AppError }[];
+  missing: GpuGroup[];
+  /** Длина окна приростов, с; `null` — первый опрос. */
+  deltaSec: number | null;
+}
+
+/** Цель экспортёра DCGM в источнике истории. */
+export interface DcgmTarget {
+  job: string;
+  instance: string;
+}
+
+/** Проверка компонента GPU: шаги, хосты карт и автоподбор. */
+export interface GpuCheck {
+  report: TestReport;
+  hosts: { host: string; cards: number[] }[];
+  /** Все хосты карт источника истории (режим `prometheus`). */
+  available: string[];
+  selector?: string;
+  bindings: { serverId: string; binding: GpuBinding | null }[];
+  exporters: { url: string; historySelector: string | null; targets: DcgmTarget[] }[];
+}
+
+/** Окно истории карт хоста: ряд на карту. */
+export interface GpuHistoryAnswer {
+  window: HistoryWindow;
+  stepSec: number;
+  source: HistorySourceKind;
+  series: { host: string; gpu: number; points: { at: number; values: GpuValues }[] }[];
+  partial: { chart: string; error: AppError }[];
+}
+
 export type ConfigSource = "server_info" | "models" | "metrics";
 
 /** Аргумент запуска сервера с источником (спец. I1b §5). */
````

<!-- block: file t08-src-components-gpu-usegpu-ts step=8.3 path=src/components/gpu/useGpu.ts -->
````
import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { api, toAppError } from "../../api/client";
import type { GpuHistoryAnswer, GpuStatus, GpuValues, HistoryWindow } from "../../api/types";
import { useRefetchInterval } from "../../lib/tabActive";
import { POLL_MS, WINDOW_MS } from "../servers/useServerMetrics";

/** Точка буфера «Сейчас» карты: значения полей или `null` — опрос не удался (на графике разрыв). */
export interface GpuNowPoint {
  at: number;
  values: GpuValues | null;
}

/** Буфер «Сейчас»: карта (`cardKey`) → точки за последний час. */
export type GpuBuffer = Record<string, GpuNowPoint[]>;

export const cardKey = (host: string, index: number) => `${host}\u0000${index}`;

function append(points: GpuNowPoint[] | undefined, p: GpuNowPoint, windowMs: number): GpuNowPoint[] {
  const list = points ?? [];
  if (list.length > 0 && list[list.length - 1].at === p.at) return list;
  const from = p.at - windowMs;
  const kept = list[0] && list[0].at < from ? list.filter((x) => x.at >= from) : list;
  return [...kept, p];
}

/** Снимок карт в буфер: точка каждой карты, старше окна отбрасываются, повтор того же снимка не добавляется. */
export function appendStatus(buf: GpuBuffer, status: GpuStatus, windowMs = WINDOW_MS): GpuBuffer {
  const next: GpuBuffer = { ...buf };
  for (const h of status.hosts)
    for (const c of h.cards) {
      const key = cardKey(h.host, c.index);
      next[key] = append(buf[key], { at: status.at, values: c.values }, windowMs);
    }
  return next;
}

/** Неудачный опрос: разрыв у всех карт буфера. */
export function appendError(buf: GpuBuffer, at: number, windowMs = WINDOW_MS): GpuBuffer {
  const next: GpuBuffer = {};
  for (const [key, points] of Object.entries(buf)) next[key] = append(points, { at, values: null }, windowMs);
  return next;
}

export function gpuStatusKey(installationId: string) {
  return ["gpu-status", installationId] as const;
}

export function gpuHistoryKey(installationId: string, window: HistoryWindow, host: string, gpu: number | null) {
  return ["gpu-history", installationId, window, host, gpu] as const;
}

/**
 * Карты инсталляции «сейчас» (спека I1d §5): опрос раз в 5 с, только пока вкладка видна. Вкладка GPU, «Обзор»
 * инсталляции и «Обзор» сервера делят запрос (общий ключ react-query); буфер точек за час — у каждого хука свой.
 */
export function useGpuStatus(installationId: string, enabled = true) {
  const refetchInterval = useRefetchInterval(POLL_MS);
  const q = useQuery({
    queryKey: gpuStatusKey(installationId),
    queryFn: () => api.readGpuStatus(installationId),
    enabled,
    refetchInterval: enabled ? refetchInterval : false,
    retry: false,
    refetchOnWindowFocus: false,
  });
  const [buf, setBuf] = useState<{ id: string; points: GpuBuffer }>({ id: installationId, points: {} });
  useEffect(() => {
    if (q.data) {
      const status = q.data;
      setBuf((b) => ({ id: installationId, points: appendStatus(b.id === installationId ? b.points : {}, status) }));
    }
  }, [installationId, q.data, q.dataUpdatedAt]);
  useEffect(() => {
    if (q.error && q.errorUpdatedAt) {
      const at = q.errorUpdatedAt;
      setBuf((b) => ({ id: installationId, points: appendError(b.id === installationId ? b.points : {}, at) }));
    }
  }, [installationId, q.error, q.errorUpdatedAt]);
  return {
    status: q.data,
    error: q.error ? toAppError(q.error) : null,
    isPending: q.isPending,
    buffer: buf.id === installationId ? buf.points : {},
    refetch: q.refetch,
  };
}

/**
 * Окно истории карт хоста (одной карты `gpu` или всех): запрашивается при выборе, ответ живёт минуту, как окно
 * истории сервера; свежее — по «Обновить».
 */
export function useGpuHistory(installationId: string, window: HistoryWindow, host: string, gpu: number | null, enabled: boolean) {
  return useQuery({
    queryKey: gpuHistoryKey(installationId, window, host, gpu),
    queryFn: () => api.readGpuHistory(installationId, window, host, gpu),
    enabled,
    staleTime: 60_000,
    gcTime: 60_000,
    placeholderData: keepPreviousData,
    retry: false,
    refetchOnWindowFocus: false,
  });
}

/**
 * Ряды окна «Сейчас» карт `indices` хоста `host`: точки истории (окно `now`) раньше первой живой точки карты, затем
 * буфер опроса.
 */
export function nowSeries(buffer: GpuBuffer, host: string, indices: number[], history?: GpuHistoryAnswer): Record<number, GpuNowPoint[]> {
  const out: Record<number, GpuNowPoint[]> = {};
  for (const index of indices) {
    const live = buffer[cardKey(host, index)] ?? [];
    const first = live.length > 0 ? live[0].at : Number.POSITIVE_INFINITY;
    const past = history?.series.find((s) => s.host === host && s.gpu === index)?.points.filter((p) => p.at < first) ?? [];
    out[index] = [...past, ...live];
  }
  return out;
}
````

- [ ] **Шаг 8.4: Проверки.**

Run: `pnpm test`
Expected: `Test Files  36 passed`, `Tests  267 passed`
Run: `pnpm exec tsc --noEmit`
Expected: без вывода

- [ ] **Шаг 8.5: Коммит.**

<!-- block: shell t08-commit step=8.5 -->
````
git add src/api/client.test.ts \
  src/api/client.ts \
  src/api/types.ts \
  src/components/gpu/testGpu.ts \
  src/components/gpu/useGpu.test.tsx \
  src/components/gpu/useGpu.ts
git commit -s -q -F - <<'EOF'
feat(gpu): frontend types, IPC calls and GPU hooks

Types of the GPU component, server binding, card snapshots, the check
and history windows; checkGpuSource, readGpuStatus and readGpuHistory.
useGpuStatus polls every 5 s while the tab is visible and keeps an hour
of points per card; useGpuHistory keeps a window for a minute;
nowSeries puts the history before the first live point of each card.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
````

---

### Задача 9: Вкладка GPU и узел дерева

Вкладка GPU (спека I1d §5): окна «Сейчас / 1 ч / 24 ч / 7 д / 30 д» (без источника истории или сопоставления —
неактивны с подсказкой), сбои экспортёров, подсказка о неполном наборе счётчиков, группы по хостам (драйвер, число
карт, привязанные серверы) с карточками карт: утилизация, SM (запасная — с пометкой «по GR engine»), память,
температура, мощность, значки троттлинга, XID, ECC, NVLink и PCIe с расшифровкой, нагрузка Kubernetes, MIG. Щелчок по
карточке или хосту раскрывает графики: у карты поля рядом, у хоста — ряд на карту. `MetricChart` и `chartData`
обобщаются по типу точки (`ChartPoint<T>`, `ChartPick<T>`; по умолчанию — метрики сервера). В дереве у инсталляции с
компонентом GPU — пункт «GPU» с числом карт последнего снимка. Помощники графиков — `gpuPoints.ts`: на macOS имена
`gpuCharts.ts` и `GpuCharts.tsx` совпадают.

**Files:**
- Create: `src/components/gpu/GpuCardView.tsx`
- Create: `src/components/gpu/GpuCharts.tsx`
- Create: `src/components/gpu/GpuPanel.test.tsx`
- Create: `src/components/gpu/GpuPanel.tsx`
- Create: `src/components/gpu/alerts.test.ts`
- Create: `src/components/gpu/alerts.ts`
- Create: `src/components/gpu/gpuPoints.test.ts`
- Create: `src/components/gpu/gpuPoints.ts`
- Modify: `docs/i18n/same-as-en.json`
- Modify: `src/app.css`
- Modify: `src/components/Layout.tsx`
- Modify: `src/components/TabContent.tsx`
- Modify: `src/components/installations/tree/InstallationNode.tsx`
- Modify: `src/components/installations/tree/InstallationTree.test.tsx`
- Modify: `src/components/servers/MetricChart.tsx`
- Modify: `src/components/servers/chart.ts`
- Modify: `src/locales/en.json`
- Modify: `src/locales/ru.json`
- Modify: `src/state/workspace.test.ts`
- Modify: `src/state/workspace.ts`

**Interfaces:**
- Consumes: `useGpuStatus`, `useGpuHistory`, `nowSeries`, `MetricChart`.
- Produces: `TabSpec { kind: "gpu"; installationId }` (`tabId` — `gpu:<id>`); `GpuPanel`, `gpuHistoryUnavailable`,
  `boundServers` (`GpuPanel.tsx`); `GpuCardView`; `GpuCharts`; `cardAlerts`, `reasons`, `REASONS`, `ALERT_REASONS`,
  `CardAlert` (`alerts.ts`); `mergeCards`, `historyCards`, `gpuCharts`, `Cards` (`gpuPoints.ts`); `ChartPoint<T>`
  (`chart.ts`); ключи `gpu.*`, `sidebar.gpu`, `sidebar.gpuCount`, `tabs.gpu`.

- [ ] **Шаг 9.1: Тесты.** Тесты значков, точек графиков, вкладки GPU, пункта дерева и `tabId`.

<!-- block: file t09-src-components-gpu-gpupanel-test-tsx step=9.1 path=src/components/gpu/GpuPanel.test.tsx -->
````
import { act, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { InstallationProfile } from "../../api/types";
import { renderWithClient } from "../../test/render";
import { newInstallation, newServer } from "../installations/profile";

vi.mock("../../api/client", async (importOriginal) => {
  const orig = await importOriginal<typeof import("../../api/client")>();
  return { ...orig, api: { listInstallations: vi.fn(), readGpuStatus: vi.fn(), readGpuHistory: vi.fn() } };
});

vi.mock("uplot", () => ({
  default: vi.fn().mockImplementation(function () {
    return { destroy: vi.fn(), setSize: vi.fn(), setData: vi.fn(), redraw: vi.fn() };
  }),
}));
globalThis.ResizeObserver ??= class {
  observe() {}
  disconnect() {}
  unobserve() {}
} as unknown as typeof ResizeObserver;

import { api } from "../../api/client";
import { boundServers, GpuPanel, gpuHistoryUnavailable } from "./GpuPanel";
import { card, host, status } from "./testGpu";

const direct = (historySelector?: string): InstallationProfile["gpu"] => ({
  mode: "direct",
  exporters: [{ url: "http://gpu-1:9400/metrics", historySelector }],
  auth: { mode: "none" },
  ca: { mode: "system" },
  proxy: { mode: "system" },
});

function profile(over: Partial<InstallationProfile> = {}): InstallationProfile {
  const s = { ...newServer(), id: "s1", name: "vllm-1", gpus: { host: "GPU-1" } };
  return { ...newInstallation(), id: "i1", name: "prod LLM", servers: [s], gpu: direct(), ...over };
}

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.listInstallations).mockResolvedValue([profile()]);
  vi.mocked(api.readGpuHistory).mockResolvedValue({ window: "1h", stepSec: 60, source: "prometheus", series: [], partial: [] });
});

describe("GpuPanel", () => {
  it("хосты и карточки карт; «нет данных», запасная SM, значки и нагрузка", async () => {
    vi.mocked(api.readGpuStatus).mockResolvedValue(
      status(1, [
        host("gpu-1", [
          card(0, { workload: { namespace: "llm", pod: "vllm-0" }, values: { util: 91, smActive: 40, memUsed: 40960, memTotal: 81920, temp: 89, power: 650, throttle: 0x48, xid: 79 } }),
          card(1, { smSource: "gr", values: { util: 20, smActive: 18 }, deltas: {} }),
        ]),
      ]),
    );
    renderWithClient(<GpuPanel installationId="i1" />);
    const hostSection = await screen.findByRole("region", { name: "gpu-1" });
    expect(within(hostSection).getByText(/Карт: 2/)).toBeInTheDocument();
    expect(within(hostSection).getByText(/драйвер 570\.124\.06/)).toBeInTheDocument();
    expect(within(hostSection).getByText(/Серверы: vllm-1/)).toBeInTheDocument();
    const c0 = screen.getByRole("region", { name: "GPU 0" });
    expect(within(c0).getByText(/Троттлинг: HW_SLOWDOWN, HW_THERMAL/)).toBeInTheDocument();
    expect(within(c0).getByText(/XID 79/)).toBeInTheDocument();
    expect(within(c0).getByText("Нагрузка: llm/vllm-0")).toBeInTheDocument();
    expect(within(c0).getByText(/40 \/ 80 ГиБ/)).toBeInTheDocument();
    const c1 = screen.getByRole("region", { name: "GPU 1" });
    expect(within(c1).getByText("по GR engine")).toBeInTheDocument();
    expect(within(c1).getAllByText("нет данных")).toHaveLength(3);
    expect(screen.getByText("Щёлкните карту или хост, чтобы открыть графики")).toBeInTheDocument();
  });

  it("сбой экспортёра, неполный набор счётчиков и сбой всего опроса", async () => {
    vi.mocked(api.readGpuStatus).mockResolvedValue(
      status(1, [host("gpu-1", [card(0)])], {
        errors: [{ exporter: "http://gpu-2:9400/metrics", error: { code: "CONNECTION_FAILED", key: "errors.server.serverError", params: { path: "/metrics", status: "503" }, message: "x", retryable: true } }],
        missing: ["sm", "ecc"],
      }),
    );
    renderWithClient(<GpuPanel installationId="i1" />);
    expect(await screen.findByText(/Экспортёр http:\/\/gpu-2:9400\/metrics:/)).toBeInTheDocument();
    expect(screen.getByText(/Экспортёр не собирает: активность SM, ошибки ECC/)).toBeInTheDocument();

    vi.mocked(api.readGpuStatus).mockRejectedValue({ code: "CONNECTION_FAILED", key: "errors.server.serverError", params: { path: "/metrics", status: "503" }, message: "x", retryable: true });
    vi.mocked(api.listInstallations).mockResolvedValue([profile({ id: "i2" })]);
    renderWithClient(<GpuPanel installationId="i2" />);
    expect(await screen.findByRole("alert")).toBeInTheDocument();
  });

  it("щелчок по хосту и карте раскрывает графики; окна истории — по профилю", async () => {
    vi.mocked(api.readGpuStatus).mockResolvedValue(status(1, [host("gpu-1", [card(0), card(1)])]));
    const user = userEvent.setup();
    renderWithClient(<GpuPanel installationId="i1" />);
    await screen.findByRole("region", { name: "gpu-1" });
    expect(screen.getByRole("button", { name: "1 ч" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "gpu-1" }));
    expect(screen.getByRole("region", { name: "Утилизация" })).toBeInTheDocument();
    await user.click(within(screen.getByRole("region", { name: "GPU 1" })).getByRole("button", { name: /GPU 1/ }));
    expect(screen.getByRole("region", { name: "Утилизация и SM" })).toBeInTheDocument();
    await user.click(within(screen.getByRole("region", { name: "GPU 1" })).getByRole("button", { name: /GPU 1/ }));
    expect(screen.queryByRole("region", { name: "Утилизация и SM" })).not.toBeInTheDocument();
    expect(api.readGpuHistory).not.toHaveBeenCalled();
  });

  it("с историей: окно 1 ч запрашивается по хосту и карте; «Сейчас» предзаполняется", async () => {
    vi.mocked(api.listInstallations).mockResolvedValue([
      profile({ history: { url: "http://prom:9090", auth: { mode: "none" }, ca: { mode: "system" }, proxy: { mode: "system" } }, gpu: direct('{job="dcgm"}') }),
    ]);
    vi.mocked(api.readGpuStatus).mockResolvedValue(status(1, [host("gpu-1", [card(0)])]));
    const user = userEvent.setup();
    renderWithClient(<GpuPanel installationId="i1" />);
    await screen.findByRole("region", { name: "gpu-1" });
    await user.click(screen.getByRole("button", { name: "gpu-1" }));
    expect(api.readGpuHistory).toHaveBeenCalledWith("i1", "now", "gpu-1", null);
    await user.click(screen.getByRole("button", { name: "1 ч" }));
    await act(async () => {});
    expect(api.readGpuHistory).toHaveBeenCalledWith("i1", "1h", "gpu-1", null);
    expect(await screen.findByText(/Шаг 1 мин · Prometheus/)).toBeInTheDocument();
  });

  it("без компонента GPU — подсказка; помощники доступности истории и привязанных серверов", async () => {
    vi.mocked(api.listInstallations).mockResolvedValue([profile({ gpu: undefined })]);
    renderWithClient(<GpuPanel installationId="i1" />);
    expect(await screen.findByText("У инсталляции нет компонента GPU — добавьте его в форме инсталляции")).toBeInTheDocument();
    expect(gpuHistoryUnavailable(profile())).toBe("gpu.noSource");
    const history = { url: "http://p:9090", auth: { mode: "none" as const }, ca: { mode: "system" as const }, proxy: { mode: "system" as const } };
    expect(gpuHistoryUnavailable(profile({ history }))).toBe("gpu.notMatched");
    expect(gpuHistoryUnavailable(profile({ history, gpu: { mode: "prometheus" } }))).toBeNull();
    expect(boundServers(profile(), "gpu-1")).toEqual(["vllm-1"]);
    expect(boundServers(profile(), "gpu-2")).toEqual([]);
  });
});
````

<!-- block: file t09-src-components-gpu-alerts-test-ts step=9.1 path=src/components/gpu/alerts.test.ts -->
````
import { describe, expect, it } from "vitest";
import { cardAlerts, reasons } from "./alerts";
import { card } from "./testGpu";

describe("значки карты", () => {
  it("маска причин — по разрядам", () => {
    expect(reasons(0x48)).toEqual(["HW_SLOWDOWN", "HW_THERMAL"]);
    expect(reasons(0)).toEqual([]);
  });

  it("здоровая карта и простой без троттлинга — без значков", () => {
    expect(cardAlerts(card(0))).toEqual([]);
    expect(cardAlerts(card(0, { values: { throttle: 0x1 | 0x2 } }))).toEqual([]);
    expect(cardAlerts(card(0, { values: {} }))).toEqual([]);
  });

  it("троттлинг, XID, ECC, NVLink и PCIe", () => {
    const bad = card(0, {
      values: { throttle: 0x48, xid: 79, xidCount: 1, eccDbe: 2, remapUncorrectable: 1 },
      deltas: { powerViolation: 40, eccSbe: 3, nvlinkErrors: 3, pcieReplay: 6 },
    });
    expect(cardAlerts(bad)).toEqual([
      { kind: "throttle", severity: "warn", params: { reasons: "HW_SLOWDOWN, HW_THERMAL, POWER 40 %" } },
      { kind: "xid", severity: "danger", params: { code: "79", count: "1" } },
      { kind: "ecc", severity: "danger", params: { dbe: "2", remap: "1" } },
      { kind: "nvlink", severity: "warn", params: { count: "3" } },
      { kind: "pcie", severity: "warn", params: { count: "6" } },
    ]);
    expect(cardAlerts(card(0, { values: {}, deltas: { eccSbe: 2 } }))).toEqual([{ kind: "eccCorrectable", severity: "warn", params: { count: "2" } }]);
    expect(cardAlerts(card(0, { values: {}, deltas: { thermalViolation: 12.345 } }))[0].params.reasons).toBe("THERMAL 12.3 %");
  });
});
````

<!-- block: file t09-src-components-gpu-gpupoints-test-ts step=9.1 path=src/components/gpu/gpuPoints.test.ts -->
````
import type { TFunction } from "i18next";
import { describe, expect, it } from "vitest";
import { fmtNumber } from "../servers/format";
import { gpuCharts, historyCards, mergeCards } from "./gpuPoints";

const t = ((key: string, o?: { value?: string }) => (o?.value != null ? `${o.value} ${key}` : key)) as unknown as TFunction;

describe("графики GPU", () => {
  it("точки карт объединяются по времени; ошибка у всех карт — разрыв", () => {
    const pts = mergeCards({
      0: [{ at: 10, values: { util: 1 } }, { at: 20, values: null }, { at: 30, values: { util: 3 } }],
      1: [{ at: 20, values: null }, { at: 30, values: { util: 4 } }],
    });
    expect(pts).toEqual([
      { at: 10, m: { 0: { util: 1 } } },
      { at: 20, m: null },
      { at: 30, m: { 0: { util: 3 }, 1: { util: 4 } } },
    ]);
  });

  it("ряды истории хоста — по номерам карт", () => {
    const h = historyCards(
      { window: "1h", stepSec: 60, source: "prometheus", partial: [], series: [{ host: "a", gpu: 2, points: [{ at: 1, values: {} }] }, { host: "b", gpu: 0, points: [] }] },
      "a",
    );
    expect(Object.keys(h)).toEqual(["2"]);
    expect(historyCards(undefined, "a")).toEqual({});
  });

  it("у карты — поля рядом, у хоста — ряд на карту; память в ГиБ", () => {
    const single = gpuCharts(t, [3], true);
    expect(single.map((c) => c.id)).toEqual(["util", "memory", "temp", "power", "clocks"]);
    expect(single[0].picks.map((p) => p.label)).toEqual(["gpu.charts.util", "gpu.charts.sm"]);
    expect(single[0].picks[1].pick({ 3: { smActive: 55 } })).toBe(55);
    expect(single[1].format(81920)).toBe(`${fmtNumber(80, 1)} gpu.units.gib`);
    const host = gpuCharts(t, [0, 1, 2], false);
    expect(host[0].picks.map((p) => p.label)).toEqual(["GPU 0", "GPU 1", "GPU 2"]);
    expect(host[3].picks[2].pick({ 2: { power: 400 } })).toBe(400);
    expect(host[3].picks[1].pick({ 2: { power: 400 } })).toBeUndefined();
  });
});
````

<!-- block: patch t09-src-components-installations-tree-installationtree-test-tsx step=9.1 -->
````
diff --git a/src/components/installations/tree/InstallationTree.test.tsx b/src/components/installations/tree/InstallationTree.test.tsx
index de010fc..4fff0dd 100644
--- a/src/components/installations/tree/InstallationTree.test.tsx
+++ b/src/components/installations/tree/InstallationTree.test.tsx
@@ -4,6 +4,8 @@ import { StrictMode } from "react";
 import { beforeEach, describe, expect, it, vi } from "vitest";
 import type { InstallationFolder, InstallationInfo, InstallationProfile, ServerConfig } from "../../../api/types";
 import { renderWithClient } from "../../../test/render";
+import { gpuStatusKey } from "../../gpu/useGpu";
+import { card, host, status } from "../../gpu/testGpu";
 import { installationQueryKey } from "../InstallationPanel";
 import { newInstallation, newServer } from "../profile";
 
@@ -22,6 +24,7 @@ vi.mock("../../../api/client", async (importOriginal) => {
       previewImport: vi.fn(),
       applyImport: vi.fn(),
       connectInstallation: vi.fn(),
+      readGpuStatus: vi.fn(),
     },
   };
 });
@@ -97,6 +100,19 @@ describe("InstallationTree", () => {
     expect(useWorkspace.getState().activeId).toBe("installation:y");
   });
 
+  it("«GPU» — у инсталляции с компонентом GPU: число карт из снимка, щелчок открывает вкладку GPU", async () => {
+    const withGpu: InstallationProfile = { ...CONNS[1], gpu: { mode: "prometheus" } };
+    vi.mocked(api.listInstallations).mockResolvedValue([withGpu, { ...CONNS[0], folderId: null }]);
+    const user = userEvent.setup();
+    const { client } = renderWithClient(<InstallationTree />);
+    expect(await screen.findByRole("button", { name: "GPU" })).toBeInTheDocument();
+    expect(screen.getAllByRole("button", { name: /^GPU/ })).toHaveLength(1);
+    act(() => client.setQueryData(gpuStatusKey("y"), status(1, [host("gpu-1", [card(0), card(1)]), host("gpu-2", [card(0)])])));
+    await user.click(await screen.findByRole("button", { name: "GPU · 3" }));
+    expect(useWorkspace.getState().activeId).toBe("gpu:y");
+    expect(api.readGpuStatus).not.toHaveBeenCalled();
+  });
+
   it("сворачивание папки сохраняется между перерисовками", async () => {
     const user = userEvent.setup();
     const first = renderWithClient(<InstallationTree />);
````

<!-- block: patch t09-src-state-workspace-test-ts step=9.1 -->
````
diff --git a/src/state/workspace.test.ts b/src/state/workspace.test.ts
index 1c0726b..6e1f3c4 100644
--- a/src/state/workspace.test.ts
+++ b/src/state/workspace.test.ts
@@ -35,6 +35,7 @@ describe("workspace", () => {
     expect(tabId({ kind: "installation-form", installationId: "a" })).toBe("installation-form:a");
     expect(tabId({ kind: "installation", installationId: "a" })).toBe("installation:a");
     expect(tabId({ kind: "server", installationId: "a", serverId: "s1" })).toBe("server:a:s1");
+    expect(tabId({ kind: "gpu", installationId: "a" })).toBe("gpu:a");
     expect(tabInstallation({ kind: "server", installationId: "a", serverId: "s1" })).toBe("a");
     expect(tabId({ kind: "help", chapter: "installations" })).toBe("help");
     expect(tabId({ kind: "about" })).toBe("about");
````

- [ ] **Шаг 9.2: Увидеть падение.**

Run: `pnpm exec vitest run src/components/gpu src/components/installations/tree src/state`
Expected: FAIL — нет `./alerts`, `./gpuPoints`, `./GpuPanel`, пункта «GPU» в дереве

- [ ] **Шаг 9.3: Реализация.** Вкладка, карточки, графики, пункт дерева, заголовок вкладки, стили, словари и исключения `same-as-en`.

<!-- block: patch t09-docs-i18n-same-as-en-json step=9.3 -->
````
diff --git a/docs/i18n/same-as-en.json b/docs/i18n/same-as-en.json
index 5cdc462..b3f5934 100644
--- a/docs/i18n/same-as-en.json
+++ b/docs/i18n/same-as-en.json
@@ -1,6 +1,19 @@
 {
   "ru": [
     "errors.generic",
+    "gpu.alerts.ecc",
+    "gpu.alerts.eccCorrectable",
+    "gpu.alerts.nvlink",
+    "gpu.alerts.pcie",
+    "gpu.alerts.xid",
+    "gpu.card.mig",
+    "gpu.card.sm",
+    "gpu.card.title",
+    "gpu.charts.sm",
+    "gpu.charts.smClock",
+    "gpu.panel.title",
+    "gpu.units.celsius",
+    "gpu.units.percent",
     "installations.form.kinds.sglang",
     "installations.form.kinds.vllm",
     "installations.form.step.tcp",
@@ -8,6 +21,9 @@
     "messages.connectionTest.kind",
     "messages.connectionTest.kindVersion",
     "servers.metrics.sources.prometheus",
-    "servers.metrics.sources.victoriametrics"
+    "servers.metrics.sources.victoriametrics",
+    "sidebar.gpu",
+    "sidebar.gpuCount",
+    "tabs.gpu"
   ]
 }
````

<!-- block: patch t09-src-app-css step=9.3 -->
````
diff --git a/src/app.css b/src/app.css
index e6f50ae..befcc5e 100644
--- a/src/app.css
+++ b/src/app.css
@@ -42,3 +42,14 @@ input::placeholder, textarea::placeholder { font-style: italic; color: var(--mut
 .metric-chart h4 { margin: 0 0 4px; font-size: 13px; }
 .server-metrics .toolbar button.active { border-color: var(--accent); font-weight: 600; }
 .badge { border: 1px solid var(--border); border-radius: 10px; padding: 0 6px; font-size: 12px; color: var(--muted); white-space: nowrap; }
+/* Вкладка GPU: группы по хостам, карточки карт с полосами, графики под группой. */
+.gpu-host { border: 1px solid var(--border); border-radius: 6px; padding: 8px; display: flex; flex-direction: column; gap: 8px; }
+.gpu-host-title { font-size: 14px; }
+.gpu-cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 8px; }
+.gpu-card { border: 1px solid var(--border); border-radius: 6px; padding: 8px; display: flex; flex-direction: column; gap: 4px; min-width: 0; }
+.gpu-card.selected { border-color: var(--accent); box-shadow: inset 0 0 0 1px var(--accent); }
+.gpu-card-title { text-align: left; }
+.gpu-card p { margin: 0; font-size: 12px; }
+.gpu-bar { display: inline-block; width: 60px; height: 6px; border-radius: 3px; background: var(--bg-alt); vertical-align: middle; overflow: hidden; }
+.gpu-bar > span { display: block; height: 100%; background: var(--accent); }
+.gpu-alerts li { font-size: 12px; }
````

<!-- block: patch t09-src-components-layout-tsx step=9.3 -->
````
diff --git a/src/components/Layout.tsx b/src/components/Layout.tsx
index 2fcbed8..04f1998 100644
--- a/src/components/Layout.tsx
+++ b/src/components/Layout.tsx
@@ -30,6 +30,8 @@ function tabTitle(tab: Tab, byId: Map<string, InstallationProfile>): string {
       const server = byId.get(tab.installationId)?.servers.find((s) => s.id === tab.serverId)?.name ?? "?";
       return `${server} · ${name}`;
     }
+    case "gpu":
+      return i18n.t("tabs.gpu", { name });
   }
 }
 
````

<!-- block: patch t09-src-components-tabcontent-tsx step=9.3 -->
````
diff --git a/src/components/TabContent.tsx b/src/components/TabContent.tsx
index 5a4600b..d11dbb8 100644
--- a/src/components/TabContent.tsx
+++ b/src/components/TabContent.tsx
@@ -2,6 +2,7 @@ import { Component, lazy, type ReactNode, Suspense } from "react";
 import { useTranslation } from "react-i18next";
 import type { Tab } from "../state/workspace";
 import { AboutPanel } from "./about/AboutPanel";
+import { GpuPanel } from "./gpu/GpuPanel";
 import { InstallationFormTab } from "./installations/InstallationFormTab";
 import { InstallationPanel } from "./installations/InstallationPanel";
 import { ServerPanel } from "./servers/ServerPanel";
@@ -46,5 +47,7 @@ export function TabContent({ tab }: { tab: Tab }) {
       return <InstallationPanel installationId={tab.installationId} />;
     case "server":
       return <ServerPanel installationId={tab.installationId} serverId={tab.serverId} />;
+    case "gpu":
+      return <GpuPanel installationId={tab.installationId} />;
   }
 }
````

<!-- block: file t09-src-components-gpu-gpucardview-tsx step=9.3 path=src/components/gpu/GpuCardView.tsx -->
````
import { useTranslation } from "react-i18next";
import type { GpuCard } from "../../api/types";
import { fmtNumber } from "../servers/format";
import { cardAlerts, reasons } from "./alerts";

/** Полоса заполнения 0…100 %. */
function Bar({ pct }: { pct: number }) {
  return (
    <span className="gpu-bar" aria-hidden="true">
      <span style={{ width: `${Math.max(0, Math.min(100, pct))}%` }} />
    </span>
  );
}

/**
 * Карточка карты (спека I1d §5): номер и модель, утилизация, активность SM (запасная — с пометкой), память, температура,
 * мощность, значки проблем с расшифровкой, нагрузка Kubernetes и экземпляры MIG. Поле без серии — «нет данных».
 */
export function GpuCardView({ card, selected, onSelect }: { card: GpuCard; selected: boolean; onSelect: () => void }) {
  const { t } = useTranslation();
  const v = card.values;
  const none = <span className="muted">{t("gpu.card.noData")}</span>;
  const pct = (x: number | undefined) => (x == null ? none : t("gpu.units.percent", { value: fmtNumber(x) }));
  const alerts = cardAlerts(card);
  const allReasons = v.throttle != null ? reasons(v.throttle) : [];
  return (
    <section className={selected ? "gpu-card selected" : "gpu-card"} aria-label={t("gpu.card.title", { index: card.index })}>
      <button type="button" className="link gpu-card-title" aria-pressed={selected} onClick={onSelect} title={card.uuid}>
        <strong>{t("gpu.card.title", { index: card.index })}</strong> <span className="muted">{card.model ?? ""}</span>
      </button>
      <dl className="facts">
        <dt>{t("gpu.card.util")}</dt>
        <dd>
          {v.util != null && <Bar pct={v.util} />} {pct(v.util)}
        </dd>
        <dt>{t("gpu.card.sm")}</dt>
        <dd>
          {v.smActive != null && <Bar pct={v.smActive} />} {pct(v.smActive)}
          {card.smSource === "gr" && <span className="muted" title={t("gpu.card.smGrHint")}> {t("gpu.card.smGr")}</span>}
        </dd>
        <dt>{t("gpu.card.memory")}</dt>
        <dd>
          {v.memUsed != null && v.memTotal ? <Bar pct={(v.memUsed / v.memTotal) * 100} /> : null}{" "}
          {v.memUsed != null && v.memTotal != null
            ? t("gpu.card.memoryValue", { used: fmtNumber(v.memUsed / 1024, 1), total: fmtNumber(v.memTotal / 1024, 1) })
            : none}
        </dd>
        <dt>{t("gpu.card.temp")}</dt>
        <dd>{v.temp == null ? none : t("gpu.units.celsius", { value: fmtNumber(v.temp) })}</dd>
        <dt>{t("gpu.card.power")}</dt>
        <dd>{v.power == null ? none : t("gpu.units.watt", { value: fmtNumber(v.power) })}</dd>
      </dl>
      {alerts.length > 0 && (
        <ul className="kv gpu-alerts">
          {alerts.map((a) => (
            <li key={a.kind} className={a.severity === "danger" ? "srv-error" : "warn"} title={t(`gpu.alerts.${a.kind}Hint`, a.params)}>
              ⚠ {t(`gpu.alerts.${a.kind}`, a.params)}
            </li>
          ))}
        </ul>
      )}
      {alerts.length === 0 && allReasons.length > 0 && <p className="muted">{t("gpu.card.clockReasons", { reasons: allReasons.join(", ") })}</p>}
      {card.workload && <p className="muted">{t("gpu.card.workload", { namespace: card.workload.namespace, pod: card.workload.pod })}</p>}
      {card.mig && card.mig.length > 0 && (
        <p className="muted">{t("gpu.card.mig", { instances: card.mig.map((m) => `${m.profile} #${m.instance}`).join(", ") })}</p>
      )}
    </section>
  );
}
````

<!-- block: file t09-src-components-gpu-gpucharts-tsx step=9.3 path=src/components/gpu/GpuCharts.tsx -->
````
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { toAppError } from "../../api/client";
import type { HistoryWindow } from "../../api/types";
import { currentLocale } from "../../i18n";
import { tr } from "../../i18n/text";
import { ErrorBanner } from "../common/ErrorBanner";
import { MetricChart } from "../servers/MetricChart";
import { type Cards, gpuCharts, historyCards, mergeCards } from "./gpuPoints";
import { type GpuBuffer, nowSeries, useGpuHistory } from "./useGpu";

/** Время «HH:MM» по языку интерфейса. */
const hhmm = (at: number) => new Date(at).toLocaleTimeString(currentLocale(), { hour: "2-digit", minute: "2-digit" });

interface Props {
  installationId: string;
  host: string;
  /** Карта; `null` — все карты хоста (не больше 16). */
  gpu: number | null;
  /** Номера карт хоста. */
  indices: number[];
  buffer: GpuBuffer;
  window: HistoryWindow;
  /** `null` — история доступна; иначе ключ подсказки, почему нет. */
  historyUnavailable: string | null;
}

/**
 * Графики карты или хоста (спека I1d §5): окно «Сейчас» — буфер опроса с историей за 15 минут до него, окна 1 ч…30 д —
 * из источника истории. Частичное окно и ошибка — как у графиков сервера.
 */
export function GpuCharts({ installationId, host, gpu, indices, buffer, window: win, historyUnavailable }: Props) {
  const { t } = useTranslation();
  const now = win === "now";
  const shownIndices = gpu == null ? indices.slice(0, 16) : [gpu];
  const prefill = useGpuHistory(installationId, "now", host, null, now && !historyUnavailable);
  const hist = useGpuHistory(installationId, now ? "1h" : win, host, gpu, !now && !historyUnavailable);
  const series = now ? nowSeries(buffer, host, shownIndices, prefill.data) : historyCards(hist.data, host);
  const points = mergeCards(Object.fromEntries(shownIndices.map((i) => [i, series[i] ?? []])));
  const indicesKey = shownIndices.join(",");
  const charts = useMemo(() => gpuCharts(t, indicesKey.split(",").map(Number), gpu != null), [t, indicesKey, gpu]);
  const until = now ? (prefill.data?.series.flatMap((s) => (s.host === host ? s.points.map((p) => p.at) : [])) ?? []) : [];
  const marker = until.length > 0 ? Math.max(...until) : null;
  const firstLoad = !now && !hist.error && !hist.data && hist.isFetching;
  const step = (sec: number) => (sec % 60 === 0 ? t("servers.units.min", { value: sec / 60 }) : t("servers.units.sec", { value: sec }));
  return (
    <div className="gpu-charts">
      {now && marker != null && <p className="muted">{t("servers.metrics.historyUntil", { time: hhmm(marker) })}</p>}
      {!now && hist.data && (
        <p className="muted">{t("servers.metrics.step", { step: step(hist.data.stepSec), source: t(`servers.metrics.sources.${hist.data.source}`) })}</p>
      )}
      {!now && hist.error && (
        <ErrorBanner error={toAppError(hist.error)} onRetry={toAppError(hist.error).retryable ? () => void hist.refetch() : undefined} />
      )}
      {!now && hist.data && hist.data.partial.length > 0 && (
        <p className="warn" role="status">
          {t("gpu.partial", {
            charts: hist.data.partial.map((x) => `${t(`gpu.charts.${x.chart}`)} — ${tr(x.error)}`).join("; "),
          })}
        </p>
      )}
      {now && prefill.error && <p className="muted">{t("servers.metrics.prefillFailed", { error: tr(toAppError(prefill.error)) })}</p>}
      {firstLoad && (
        <p className="muted" role="status">
          {t("common.loading")}
        </p>
      )}
      {!(!now && hist.error) && !firstLoad && (
        <div className={!now && hist.isFetching ? "charts loading" : "charts"} aria-busy={!now && hist.isFetching}>
          {charts.map((c) => (
            <MetricChart<Cards> key={c.id} title={t(gpu != null && c.id === "util" ? "gpu.charts.utilSm" : `gpu.charts.${c.id}`)} points={points} picks={c.picks} format={c.format} step={c.step} marker={now ? marker : null} />
          ))}
        </div>
      )}
    </div>
  );
}
````

<!-- block: file t09-src-components-gpu-gpupanel-tsx step=9.3 path=src/components/gpu/GpuPanel.tsx -->
````
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { api, toAppError } from "../../api/client";
import type { HistoryWindow, InstallationProfile } from "../../api/types";
import { tr } from "../../i18n/text";
import { useWorkspace } from "../../state/workspace";
import { EnvBadge } from "../common/EnvBadge";
import { ErrorBanner } from "../common/ErrorBanner";
import { GpuCardView } from "./GpuCardView";
import { GpuCharts } from "./GpuCharts";
import { useGpuStatus } from "./useGpu";

const WINDOWS: HistoryWindow[] = ["now", "1h", "24h", "7d", "30d"];

/** Почему окна истории GPU недоступны (`null` — доступны). */
export function gpuHistoryUnavailable(p: InstallationProfile): string | null {
  if (!p.history) return "gpu.noSource";
  if (p.gpu?.mode === "direct" && !p.gpu.exporters.some((e) => e.historySelector)) return "gpu.notMatched";
  return null;
}

/** Серверы, привязанные к хосту карт (без учёта регистра). */
export function boundServers(p: InstallationProfile, host: string): string[] {
  return p.servers.filter((s) => s.gpus && s.gpus.host.toLowerCase() === host.toLowerCase()).map((s) => s.name);
}

/**
 * Вкладка GPU (спека I1d §5): окна «Сейчас / 1 ч / 24 ч / 7 д / 30 д», сбои экспортёров, подсказка о неполном наборе
 * счётчиков, группы по хостам с карточками карт. Щелчок по карточке или заголовку хоста раскрывает графики под группой.
 */
export function GpuPanel({ installationId }: { installationId: string }) {
  const { t } = useTranslation();
  const open = useWorkspace((s) => s.open);
  const queryClient = useQueryClient();
  const installations = useQuery({ queryKey: ["installations"], queryFn: api.listInstallations });
  const profile = installations.data?.find((c) => c.id === installationId);
  const live = useGpuStatus(installationId, !!profile?.gpu);
  const [win, setWin] = useState<HistoryWindow>("now");
  const [selected, setSelected] = useState<{ host: string; gpu: number | null } | null>(null);

  if (!profile) {
    if (installations.error) return <ErrorBanner error={toAppError(installations.error)} onRetry={() => void installations.refetch()} />;
    return <p className="empty">{installations.isSuccess ? t("installations.form.deleted") : t("common.loading")}</p>;
  }
  if (!profile.gpu) return <p className="empty">{t("gpu.panel.noComponent")}</p>;
  const unavailable = gpuHistoryUnavailable(profile);
  const status = live.status;
  const pick = (host: string, gpu: number | null) =>
    setSelected((s) => (s && s.host === host && s.gpu === gpu ? null : { host, gpu }));

  return (
    <div className="page">
      <div className="toolbar">
        <h2>{t("gpu.panel.title")}</h2>
        <EnvBadge environment={profile.environment} readOnly={profile.readOnly} />
        <button type="button" className="link" onClick={() => open({ kind: "installation", installationId })}>
          {profile.name}
        </button>
      </div>
      <div className="toolbar server-metrics" role="group" aria-label={t("servers.metrics.window")}>
        {WINDOWS.map((w) => (
          <button
            key={w}
            type="button"
            className={w === win ? "active" : undefined}
            aria-pressed={w === win}
            disabled={w !== "now" && !!unavailable}
            title={w !== "now" && unavailable ? t(unavailable) : undefined}
            onClick={() => setWin(w)}
          >
            {t(`servers.metrics.windows.${w}`)}
          </button>
        ))}
        {win !== "now" && (
          <button type="button" onClick={() => void queryClient.invalidateQueries({ queryKey: ["gpu-history", installationId] })}>
            {t("common.refresh")}
          </button>
        )}
      </div>
      {live.isPending && !live.error && <p className="muted">{t("common.loading")}</p>}
      {live.error && <ErrorBanner error={live.error} onRetry={() => void live.refetch()} />}
      {status?.errors.map((e, i) => (
        <p key={`${e.exporter ?? ""}-${i}`} className="warn" role="status">
          {e.exporter ? t("gpu.panel.exporterError", { exporter: e.exporter, error: tr(e.error) }) : tr(e.error)}
        </p>
      ))}
      {status && status.missing.length > 0 && (
        <p className="muted">{t("gpu.panel.missing", { groups: status.missing.map((g) => t(`gpu.groups.${g}`)).join(", ") })}</p>
      )}
      {status && status.hosts.length === 0 && <p className="empty">{t("gpu.panel.noCards")}</p>}
      {status?.hosts.map((h) => {
        const servers = boundServers(profile, h.host);
        const indices = h.cards.map((c) => c.index);
        return (
          <section key={h.host} className="gpu-host" aria-label={h.host}>
            <button type="button" className="link gpu-host-title" aria-pressed={selected?.host === h.host && selected.gpu == null} onClick={() => pick(h.host, null)}>
              <strong>{h.host}</strong>
            </button>
            <span className="muted">
              {" "}
              {[
                t("gpu.panel.cards", { count: h.cards.length }),
                h.driver ? t("gpu.panel.driver", { driver: h.driver }) : null,
                servers.length > 0 ? t("gpu.panel.servers", { servers: servers.join(", ") }) : null,
              ]
                .filter(Boolean)
                .join(" · ")}
            </span>
            <div className="gpu-cards">
              {h.cards.map((c) => (
                <GpuCardView
                  key={`${c.index}-${c.exporter ?? ""}`}
                  card={c}
                  selected={selected?.host === h.host && selected.gpu === c.index}
                  onSelect={() => pick(h.host, c.index)}
                />
              ))}
            </div>
            {selected?.host === h.host && (
              <GpuCharts
                installationId={installationId}
                host={h.host}
                gpu={selected.gpu}
                indices={indices}
                buffer={live.buffer}
                window={win}
                historyUnavailable={unavailable}
              />
            )}
          </section>
        );
      })}
      {status && status.hosts.length > 0 && !selected && <p className="muted">{t("gpu.panel.selectHint")}</p>}
    </div>
  );
}
````

<!-- block: file t09-src-components-gpu-alerts-ts step=9.3 path=src/components/gpu/alerts.ts -->
````
import type { GpuCard } from "../../api/types";

/** Разряды маски причин снижения тактов (`DCGM_CLOCKS_EVENT_REASON_*`), как в `gpu/fields.rs`. */
export const REASONS: [number, string][] = [
  [0x1, "GPU_IDLE"],
  [0x2, "CLOCKS_SETTING"],
  [0x4, "SW_POWER_CAP"],
  [0x8, "HW_SLOWDOWN"],
  [0x10, "SYNC_BOOST"],
  [0x20, "SW_THERMAL"],
  [0x40, "HW_THERMAL"],
  [0x80, "HW_POWER_BRAKE"],
  [0x100, "DISPLAY_CLOCKS"],
];

/** Причины, которые отмечаются значком «троттлинг»; остальные — только в подсказке. */
export const ALERT_REASONS = new Set(["SW_POWER_CAP", "HW_SLOWDOWN", "SW_THERMAL", "HW_THERMAL", "HW_POWER_BRAKE"]);

/** Причины из маски по порядку разрядов. */
export function reasons(mask: number): string[] {
  return REASONS.filter(([bit]) => (mask & bit) !== 0).map(([, name]) => name);
}

export type AlertKind = "throttle" | "xid" | "ecc" | "eccCorrectable" | "nvlink" | "pcie";

/** Значок проблемы карты: вид, серьёзность и параметры подсказки (`gpu.alerts.<вид>`). */
export interface CardAlert {
  kind: AlertKind;
  severity: "warn" | "danger";
  params: Record<string, string>;
}

const fmt = (v: number) => String(Math.round(v * 10) / 10);

/**
 * Значки карты (спека I1d §5): троттлинг (причины из маски или доля времени по счётчикам), последняя XID-ошибка,
 * некорректируемые ECC и сбои переназначения строк, корректируемые ECC, ошибки NVLink и повторы PCIe за минуту.
 */
export function cardAlerts(c: GpuCard): CardAlert[] {
  const v = c.values;
  const d = c.deltas ?? {};
  const out: CardAlert[] = [];
  const throttling = v.throttle != null ? reasons(v.throttle).filter((r) => ALERT_REASONS.has(r)) : [];
  const violations = [
    ["POWER", d.powerViolation],
    ["THERMAL", d.thermalViolation],
  ].filter((x): x is [string, number] => typeof x[1] === "number" && x[1] > 0);
  if (throttling.length > 0 || violations.length > 0)
    out.push({
      kind: "throttle",
      severity: "warn",
      params: { reasons: [...throttling, ...violations.map(([name, share]) => `${name} ${fmt(share)} %`)].join(", ") },
    });
  if (v.xid != null && v.xid > 0) out.push({ kind: "xid", severity: "danger", params: { code: String(v.xid), count: String(v.xidCount ?? "—") } });
  const dbe = v.eccDbe ?? 0;
  const remap = (v.remapUncorrectable ?? 0) + (v.remapFailure ?? 0);
  if (dbe > 0 || remap > 0) out.push({ kind: "ecc", severity: "danger", params: { dbe: String(dbe), remap: String(v.remapUncorrectable ?? 0) } });
  else if ((d.eccSbe ?? 0) > 0) out.push({ kind: "eccCorrectable", severity: "warn", params: { count: String(d.eccSbe) } });
  if ((d.nvlinkErrors ?? 0) > 0) out.push({ kind: "nvlink", severity: "warn", params: { count: String(d.nvlinkErrors) } });
  if ((d.pcieReplay ?? 0) > 0) out.push({ kind: "pcie", severity: "warn", params: { count: String(d.pcieReplay) } });
  return out;
}
````

<!-- block: file t09-src-components-gpu-gpupoints-ts step=9.3 path=src/components/gpu/gpuPoints.ts -->
````
import type { TFunction } from "i18next";
import type { GpuHistoryAnswer, GpuValues } from "../../api/types";
import type { ChartPick, ChartPoint } from "../servers/chart";
import { fmtNumber } from "../servers/format";
import type { GpuNowPoint } from "./useGpu";

/** Значения карт в момент времени: номер карты → поля. */
export type Cards = Record<number, GpuValues>;

/**
 * Точки графика из рядов карт: объединение моментов времени по порядку; карта без точки в момент — без значения.
 * Момент, где у всех карт ошибка опроса, — точка без данных (разрыв).
 */
export function mergeCards(series: Record<number, GpuNowPoint[]>): ChartPoint<Cards>[] {
  const at = new Map<number, Cards | null>();
  for (const [index, points] of Object.entries(series))
    for (const p of points) {
      if (!at.has(p.at)) at.set(p.at, null);
      if (p.values) at.set(p.at, { ...(at.get(p.at) ?? {}), [Number(index)]: p.values });
    }
  return [...at.entries()].sort((a, b) => a[0] - b[0]).map(([t, m]) => ({ at: t, m }));
}

/** Ряды окна истории хоста — в форме рядов «Сейчас». */
export function historyCards(answer: GpuHistoryAnswer | undefined, host: string): Record<number, GpuNowPoint[]> {
  const out: Record<number, GpuNowPoint[]> = {};
  for (const s of answer?.series ?? []) if (s.host === host) out[s.gpu] = s.points;
  return out;
}

export interface GpuChart {
  id: "util" | "memory" | "temp" | "power" | "clocks";
  picks: ChartPick<Cards>[];
  format: (v: number) => string;
  step: number;
}

/**
 * Графики вкладки GPU (спека I1d §5). У одной карты — поля рядом (утилизация и SM, занято и всего, температура карты
 * и памяти, такты SM и памяти); у хоста — ряд на карту по главному полю графика.
 */
export function gpuCharts(t: TFunction, indices: number[], single: boolean): GpuChart[] {
  const pct = (v: number) => t("gpu.units.percent", { value: fmtNumber(v) });
  const gib = (v: number) => t("gpu.units.gib", { value: fmtNumber(v / 1024, 1) });
  const deg = (v: number) => t("gpu.units.celsius", { value: fmtNumber(v) });
  const watt = (v: number) => t("gpu.units.watt", { value: fmtNumber(v) });
  const mhz = (v: number) => t("gpu.units.mhz", { value: fmtNumber(v) });
  const one = indices[0];
  const field = (label: string, f: keyof GpuValues): ChartPick<Cards> => ({ label, pick: (m) => m[one]?.[f] });
  const perCard = (f: keyof GpuValues): ChartPick<Cards>[] => indices.map((i) => ({ label: `GPU ${i}`, pick: (m) => m[i]?.[f] }));
  const charts: GpuChart[] = single
    ? [
        { id: "util", picks: [field(t("gpu.charts.util"), "util"), field(t("gpu.charts.sm"), "smActive")], format: pct, step: 1 },
        { id: "memory", picks: [field(t("gpu.charts.used"), "memUsed"), field(t("gpu.charts.total"), "memTotal")], format: gib, step: 102.4 },
        { id: "temp", picks: [field(t("gpu.charts.gpuTemp"), "temp"), field(t("gpu.charts.memTemp"), "memTemp")], format: deg, step: 1 },
        { id: "power", picks: [field(t("gpu.charts.power"), "power")], format: watt, step: 1 },
        { id: "clocks", picks: [field(t("gpu.charts.smClock"), "smClock"), field(t("gpu.charts.memClock"), "memClock")], format: mhz, step: 1 },
      ]
    : [
        { id: "util", picks: perCard("util"), format: pct, step: 1 },
        { id: "memory", picks: perCard("memUsed"), format: gib, step: 102.4 },
        { id: "temp", picks: perCard("temp"), format: deg, step: 1 },
        { id: "power", picks: perCard("power"), format: watt, step: 1 },
        { id: "clocks", picks: perCard("smClock"), format: mhz, step: 1 },
      ];
  return charts;
}
````

<!-- block: patch t09-src-components-installations-tree-installationnode-tsx step=9.3 -->
````
diff --git a/src/components/installations/tree/InstallationNode.tsx b/src/components/installations/tree/InstallationNode.tsx
index 703fef9..dea4f85 100644
--- a/src/components/installations/tree/InstallationNode.tsx
+++ b/src/components/installations/tree/InstallationNode.tsx
@@ -5,17 +5,21 @@ import { api } from "../../../api/client";
 import type { InstallationProfile } from "../../../api/types";
 import { useWorkspace } from "../../../state/workspace";
 import { EnvBadge } from "../../common/EnvBadge";
+import { gpuStatusKey } from "../../gpu/useGpu";
 import { installationQueryKey } from "../InstallationPanel";
 
 /**
- * Узел инсталляции в дереве: точка цвета, имя, окружение, «Обзор» и серверы (клик — вкладка сервера). Состояние серверов — из
- * подключения вкладки инсталляции; само дерево не подключается. rowProps — атрибуты строки (перетаскивание).
+ * Узел инсталляции в дереве: точка цвета, имя, окружение, «Обзор», серверы (клик — вкладка сервера) и «GPU» у инсталляции
+ * с компонентом GPU. Состояние серверов — из подключения вкладки инсталляции, число карт — из последнего снимка GPU;
+ * само дерево не подключается и не опрашивает. rowProps — атрибуты строки (перетаскивание).
  */
 export function InstallationNode({ installation: c, rowProps }: { installation: InstallationProfile; rowProps?: HTMLAttributes<HTMLDivElement> }) {
   const { t } = useTranslation();
   const open = useWorkspace((s) => s.open);
   const info = useQuery({ queryKey: installationQueryKey(c.id), queryFn: () => api.connectInstallation(c.id, false), enabled: false });
   const states = new Map((info.data?.servers ?? []).map((s) => [s.serverId, s]));
+  const gpu = useQuery({ queryKey: gpuStatusKey(c.id), queryFn: () => api.readGpuStatus(c.id), enabled: false });
+  const gpuCount = gpu.data?.hosts.reduce((n, h) => n + h.cards.length, 0);
   const openTab = () => open({ kind: "installation", installationId: c.id });
 
   return (
@@ -49,6 +53,13 @@ export function InstallationNode({ installation: c, rowProps }: { installation:
             </li>
           );
         })}
+        {c.gpu && (
+          <li>
+            <button type="button" className="link" onClick={() => open({ kind: "gpu", installationId: c.id })}>
+              {gpuCount == null ? t("sidebar.gpu") : t("sidebar.gpuCount", { count: gpuCount })}
+            </button>
+          </li>
+        )}
       </ul>
     </li>
   );
````

<!-- block: patch t09-src-components-servers-metricchart-tsx step=9.3 -->
````
diff --git a/src/components/servers/MetricChart.tsx b/src/components/servers/MetricChart.tsx
index 4f76ab2..a10ac87 100644
--- a/src/components/servers/MetricChart.tsx
+++ b/src/components/servers/MetricChart.tsx
@@ -2,8 +2,8 @@ import { useEffect, useMemo, useRef } from "react";
 import { useTranslation } from "react-i18next";
 import uPlot from "uplot";
 import "uplot/dist/uPlot.min.css";
-import { type ChartPick, axisIncrs, axisSize, chartData, drawMarker, hasValues, themeColor } from "./chart";
-import type { MetricPoint } from "./useServerMetrics";
+import type { ServerMetrics } from "../../api/types";
+import { type ChartPick, type ChartPoint, axisIncrs, axisSize, chartData, drawMarker, hasValues, themeColor } from "./chart";
 
 const PALETTE: [string, string][] = [
   ["--chart-1", "#3b82f6"],
@@ -18,10 +18,10 @@ const HEIGHT = 200;
 /** Шрифт подписей осей uPlot по умолчанию — по нему считается ширина оси Y. */
 const AXIS_FONT = '12px system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, "Noto Sans", sans-serif';
 
-interface Props {
+interface Props<T> {
   title: string;
-  points: MetricPoint[];
-  picks: ChartPick[];
+  points: ChartPoint<T>[];
+  picks: ChartPick<T>[];
   format: (v: number) => string;
   /** Наименьший шаг делений оси Y — точность `format` (0,1 у десятых, 1 у целых). */
   step: number;
@@ -33,7 +33,7 @@ interface Props {
  * График метрики сервера за окно «сейчас» (спец. I1b §6): ряды по `picks`, разрывы на ошибках опроса, «нет данных»,
  * если значений нет. По образцу `UsageChart` Kubernetes-версии: цвета из CSS-переменных темы, ширина — по контейнеру.
  */
-export function MetricChart({ title, points, picks, format, step, marker = null }: Props) {
+export function MetricChart<T = ServerMetrics>({ title, points, picks, format, step, marker = null }: Props<T>) {
   const { t } = useTranslation();
   const ref = useRef<HTMLDivElement>(null);
   const data = useMemo(() => chartData(points, picks), [points, picks]);
````

<!-- block: patch t09-src-components-servers-chart-ts step=9.3 -->
````
diff --git a/src/components/servers/chart.ts b/src/components/servers/chart.ts
index d08a479..01bf3df 100644
--- a/src/components/servers/chart.ts
+++ b/src/components/servers/chart.ts
@@ -1,17 +1,22 @@
 import type { ServerMetrics } from "../../api/types";
-import type { MetricPoint } from "./useServerMetrics";
 
-/** Ряд графика: подпись и значение из метрик точки (`null` — нет данных). */
-export interface ChartPick {
+/** Точка графика: время (мс) и данные; `null` — опрос не удался. У сервера данные — его метрики, у GPU — значения карт. */
+export interface ChartPoint<T> {
+  at: number;
+  m: T | null;
+}
+
+/** Ряд графика: подпись и значение из данных точки (`null` — нет данных). */
+export interface ChartPick<T = ServerMetrics> {
   label: string;
-  pick: (m: ServerMetrics) => number | null | undefined;
+  pick: (m: T) => number | null | undefined;
 }
 
 /**
  * Данные uPlot из буфера точек: ось времени в секундах и ряды по `picks`. Точка с ошибкой опроса или без значения —
  * `null`: на графике разрыв, а не ноль.
  */
-export function chartData(points: MetricPoint[], picks: ChartPick[]): (number | null)[][] {
+export function chartData<T>(points: ChartPoint<T>[], picks: ChartPick<T>[]): (number | null)[][] {
   const xs = points.map((p) => p.at / 1000);
   return [xs, ...picks.map((c) => points.map((p) => (p.m ? (c.pick(p.m) ?? null) : null)))];
 }
````

<!-- block: patch t09-src-state-workspace-ts step=9.3 -->
````
diff --git a/src/state/workspace.ts b/src/state/workspace.ts
index 996a045..5b67c8a 100644
--- a/src/state/workspace.ts
+++ b/src/state/workspace.ts
@@ -5,6 +5,8 @@ export type TabSpec =
   | { kind: "installation"; installationId: string }
   /** Сервер инсталляции: обзор, аргументы запуска, метрики «сейчас». */
   | { kind: "server"; installationId: string; serverId: string }
+  /** GPU инсталляции: карты по хостам и их графики (спека I1d §5). */
+  | { kind: "gpu"; installationId: string }
   /** folderId — папка новой инсталляции («Новая инсталляция здесь»). */
   | { kind: "installation-form"; installationId: string | null; folderId?: string | null }
   | { kind: "about" }
@@ -22,6 +24,8 @@ export function tabId(spec: TabSpec): string {
       return `installation:${spec.installationId}`;
     case "server":
       return `server:${spec.installationId}:${spec.serverId}`;
+    case "gpu":
+      return `gpu:${spec.installationId}`;
     default:
       return spec.kind;
   }
````

<!-- block: locales t09-locales-t09 step=9.3 -->
````
{
  "en": {
    "sidebar": { "gpu": "GPU", "gpuCount": "GPU · {{count}}" },
    "tabs": { "gpu": "GPU · {{name}}" },
    "gpu": {
      "noSource": "Add a metrics history source to the installation",
      "notMatched": "The DCGM exporters are not matched with the metrics history",
      "partial": "Some series were not read: {{charts}}",
      "panel": {
        "title": "GPU",
        "noComponent": "The installation has no GPU component — add it in the installation form",
        "exporterError": "Exporter {{exporter}}: {{error}}",
        "missing": "The exporter does not collect: {{groups}}. See the GPU chapter of the guide for the counters to enable.",
        "noCards": "No GPUs found",
        "cards": "GPUs: {{count}}",
        "driver": "driver {{driver}}",
        "servers": "Servers: {{servers}}",
        "selectHint": "Click a GPU or a host to open the charts"
      },
      "groups": { "sm": "SM activity", "ecc": "ECC errors", "throttle": "throttle reasons", "nvlinkErrors": "NVLink errors" },
      "card": {
        "title": "GPU {{index}}",
        "util": "Utilization",
        "sm": "SM",
        "smGr": "by GR engine",
        "smGrHint": "The exporter does not collect DCGM_FI_PROF_SM_ACTIVE: the graphics engine activity is shown",
        "memory": "Memory",
        "memoryValue": "{{used}} / {{total}} GiB",
        "temp": "Temperature",
        "power": "Power",
        "noData": "no data",
        "clockReasons": "Clock reasons: {{reasons}}",
        "workload": "Workload: {{namespace}}/{{pod}}",
        "mig": "MIG: {{instances}}"
      },
      "alerts": {
        "throttle": "Throttling: {{reasons}}",
        "throttleHint": "Clocks are reduced by power or temperature: {{reasons}}",
        "xid": "XID {{code}}",
        "xidHint": "The last XID error of the driver is {{code}} (errors in the exporter window: {{count}}); it stays until the driver reloads",
        "ecc": "ECC: {{dbe}}",
        "eccHint": "Uncorrectable ECC errors: {{dbe}}, remapped rows for uncorrectable errors: {{remap}}",
        "eccCorrectable": "ECC +{{count}}",
        "eccCorrectableHint": "Correctable ECC errors in the last minute: {{count}}",
        "nvlink": "NVLink +{{count}}",
        "nvlinkHint": "NVLink errors in the last minute: {{count}}",
        "pcie": "PCIe +{{count}}",
        "pcieHint": "PCIe replays in the last minute: {{count}}"
      },
      "charts": {
        "util": "Utilization",
        "utilSm": "Utilization and SM",
        "memory": "Memory",
        "temp": "Temperature",
        "power": "Power",
        "clocks": "Clocks",
        "errors": "Errors",
        "all": "All GPUs",
        "sm": "SM",
        "used": "Used",
        "total": "Total",
        "gpuTemp": "GPU",
        "memTemp": "Memory",
        "smClock": "SM",
        "memClock": "Memory"
      },
      "units": {
        "percent": "{{value}} %",
        "gib": "{{value}} GiB",
        "celsius": "{{value}} °C",
        "watt": "{{value}} W",
        "mhz": "{{value}} MHz"
      }
    }
  },
  "ru": {
    "sidebar": { "gpu": "GPU", "gpuCount": "GPU · {{count}}" },
    "tabs": { "gpu": "GPU · {{name}}" },
    "gpu": {
      "noSource": "Добавьте источник истории метрик в инсталляцию",
      "notMatched": "Экспортёры DCGM не сопоставлены с историей метрик",
      "partial": "Часть рядов не прочитана: {{charts}}",
      "panel": {
        "title": "GPU",
        "noComponent": "У инсталляции нет компонента GPU — добавьте его в форме инсталляции",
        "exporterError": "Экспортёр {{exporter}}: {{error}}",
        "missing": "Экспортёр не собирает: {{groups}}. Какие счётчики включить — в главе руководства о GPU.",
        "noCards": "Карты не найдены",
        "cards": "Карт: {{count}}",
        "driver": "драйвер {{driver}}",
        "servers": "Серверы: {{servers}}",
        "selectHint": "Щёлкните карту или хост, чтобы открыть графики"
      },
      "groups": { "sm": "активность SM", "ecc": "ошибки ECC", "throttle": "причины троттлинга", "nvlinkErrors": "ошибки NVLink" },
      "card": {
        "title": "GPU {{index}}",
        "util": "Утилизация",
        "sm": "SM",
        "smGr": "по GR engine",
        "smGrHint": "Экспортёр не собирает DCGM_FI_PROF_SM_ACTIVE: показана активность graphics engine",
        "memory": "Память",
        "memoryValue": "{{used}} / {{total}} ГиБ",
        "temp": "Температура",
        "power": "Мощность",
        "noData": "нет данных",
        "clockReasons": "Причины тактов: {{reasons}}",
        "workload": "Нагрузка: {{namespace}}/{{pod}}",
        "mig": "MIG: {{instances}}"
      },
      "alerts": {
        "throttle": "Троттлинг: {{reasons}}",
        "throttleHint": "Такты снижены по мощности или температуре: {{reasons}}",
        "xid": "XID {{code}}",
        "xidHint": "Последняя XID-ошибка драйвера — {{code}} (ошибок за окно экспортёра: {{count}}); держится до перезагрузки драйвера",
        "ecc": "ECC: {{dbe}}",
        "eccHint": "Некорректируемых ошибок ECC: {{dbe}}, переназначенных строк по некорректируемым ошибкам: {{remap}}",
        "eccCorrectable": "ECC +{{count}}",
        "eccCorrectableHint": "Корректируемых ошибок ECC за последнюю минуту: {{count}}",
        "nvlink": "NVLink +{{count}}",
        "nvlinkHint": "Ошибок NVLink за последнюю минуту: {{count}}",
        "pcie": "PCIe +{{count}}",
        "pcieHint": "Повторов PCIe за последнюю минуту: {{count}}"
      },
      "charts": {
        "util": "Утилизация",
        "utilSm": "Утилизация и SM",
        "memory": "Память",
        "temp": "Температура",
        "power": "Мощность",
        "clocks": "Такты",
        "errors": "Ошибки",
        "all": "Все карты",
        "sm": "SM",
        "used": "Занято",
        "total": "Всего",
        "gpuTemp": "Карта",
        "memTemp": "Память",
        "smClock": "SM",
        "memClock": "Память"
      },
      "units": {
        "percent": "{{value}} %",
        "gib": "{{value}} ГиБ",
        "celsius": "{{value}} °C",
        "watt": "{{value}} Вт",
        "mhz": "{{value}} МГц"
      }
    }
  }
}
````

- [ ] **Шаг 9.4: Проверки.**

Run: `pnpm test`
Expected: `Test Files  39 passed`, `Tests  279 passed`
Run: `pnpm exec tsc --noEmit`
Expected: без вывода
Run: `pnpm i18n:check`
Expected: `✓ словари en, ru согласованы, вызовов из Rust: 187`

- [ ] **Шаг 9.5: Коммит.**

<!-- block: shell t09-commit step=9.5 -->
````
git add docs/i18n/same-as-en.json \
  src/app.css \
  src/components/Layout.tsx \
  src/components/TabContent.tsx \
  src/components/gpu/GpuCardView.tsx \
  src/components/gpu/GpuCharts.tsx \
  src/components/gpu/GpuPanel.test.tsx \
  src/components/gpu/GpuPanel.tsx \
  src/components/gpu/alerts.test.ts \
  src/components/gpu/alerts.ts \
  src/components/gpu/gpuPoints.test.ts \
  src/components/gpu/gpuPoints.ts \
  src/components/installations/tree/InstallationNode.tsx \
  src/components/installations/tree/InstallationTree.test.tsx \
  src/components/servers/MetricChart.tsx \
  src/components/servers/chart.ts \
  src/locales/en.json \
  src/locales/ru.json \
  src/state/workspace.test.ts \
  src/state/workspace.ts
git commit -s -q -F - <<'EOF'
feat(gpu): GPU tab and tree node

The GPU tab shows windows now, 1 h, 24 h, 7 d and 30 d, exporter
failures, a hint about missing counter groups and hosts with their
driver, bound servers and card tiles: utilization, SM activity (the
graphics engine fallback is marked), memory, temperature, power, badges
for throttling, XID, ECC, NVLink and PCIe, the Kubernetes workload and
MIG instances. A click on a card or a host opens its charts: fields side
by side for a card, a series per card for a host. MetricChart takes any
point data. The tree shows GPU with the card count of the last snapshot.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
````

---

### Задача 10: Раздел GPU в форме инсталляции и поле GPU сервера

Раздел формы «GPU (DCGM)» (спека I1d §5): «Нет / Экспортёры DCGM / Из источника истории» (третий — только с
источником истории), экспортёры (до 16) с входом basic по правилам сохранённого секрета, CA, прокси и «Серии в истории»
каждого экспортёра; селектор карт и найденные хосты флажками; «Проверить GPU» с шагами. Автоподбор проверки ставит
привязку, селектор экспортёра и селектор карт только там, где их нет; подобранная под старый адрес привязка
сбрасывается; форма с сохранённым компонентом проверяет его сама. У каждого сервера — поле «GPU»: хост из найденных и
все или выбранные карты.

**Files:**
- Create: `src/components/installations/GpuFields.tsx`
- Modify: `docs/i18n/same-as-en.json`
- Modify: `src/components/installations/InstallationForm.test.tsx`
- Modify: `src/components/installations/InstallationForm.tsx`
- Modify: `src/locales/en.json`
- Modify: `src/locales/ru.json`

**Interfaces:**
- Consumes: `api.checkGpuSource`, `withCaMode`, `withProxyMode`, `selectorLabel`, `SecretInput`.
- Produces: `GpuFields`, `GpuBindingField`, `newDirectGpu`, `withGpuMode`, `dcgmSelector`, `selectorHosts`,
  `hostsSelector`, `MAX_EXPORTERS` (`src/components/installations/GpuFields.tsx`); `InstallationForm` передаёт
  `gpuSecret` в `secrets()`; ключи `installations.gpu.*`, `installations.form.step.{metrics,cards,bind,history,counters}`,
  `hints.installations.gpu*`.

- [ ] **Шаг 10.1: Тесты.** Тесты формы: экспортёры с входом, проверка с автоподбором, выбор карт сервера, режим «из истории» и хосты.

<!-- block: patch t10-src-components-installations-installationform-test-tsx step=10.1 -->
````
diff --git a/src/components/installations/InstallationForm.test.tsx b/src/components/installations/InstallationForm.test.tsx
index f6ce3b5..097d30d 100644
--- a/src/components/installations/InstallationForm.test.tsx
+++ b/src/components/installations/InstallationForm.test.tsx
@@ -10,6 +10,7 @@ vi.mock("../../api/client", async (importOriginal) => {
     api: {
       testServer: vi.fn(),
       checkHistorySource: vi.fn(),
+      checkGpuSource: vi.fn(),
       saveInstallation: vi.fn(),
       deleteInstallation: vi.fn(),
       listInstallations: vi.fn().mockResolvedValue([]),
@@ -83,6 +84,105 @@ const CHECK = {
   ],
 };
 
+const GPU_CHECK = {
+  report: {
+    ok: true,
+    steps: [
+      { name: "cards" as const, ok: true, message: msg("messages.gpuTest.exporterCards", "x", { url: "http://gpu-1:9400/metrics", count: "8", hosts: "gpu-1", driver: "570" }) },
+      { name: "bind" as const, ok: true, warning: true, message: msg("messages.gpuTest.unbound", "x", { server: "gpu-2" }) },
+    ],
+    warnings: [],
+  },
+  hosts: [
+    { host: "gpu-1", cards: [0, 1, 2, 3] },
+    { host: "gpu-3", cards: [0, 1] },
+  ],
+  available: [],
+  bindings: [
+    { serverId: "s1", binding: { host: "gpu-1" } },
+    { serverId: "s2", binding: null },
+  ],
+  exporters: [{ url: "http://gpu-1:9400/metrics", historySelector: '{job="dcgm", instance="gpu-1:9400"}', targets: [{ job: "dcgm", instance: "gpu-1:9400" }] }],
+};
+
+const HISTORY = { url: "http://prometheus:9090", auth: { mode: "none" as const }, ca: { mode: "system" as const }, proxy: { mode: "system" as const } };
+
+describe("InstallationForm: GPU", () => {
+  it("экспортёры с входом, проверка с автоподбором привязки, выбор карт — всё уходит в сохранение, пароль — секретом", async () => {
+    vi.mocked(api.checkGpuSource).mockResolvedValue(GPU_CHECK);
+    vi.mocked(api.saveInstallation).mockImplementation(async (p) => p);
+    const user = userEvent.setup();
+    renderWithClient(<InstallationForm initial={saved()} tabId="form:i1" />);
+    const section = screen.getByRole("region", { name: "GPU (DCGM)" });
+    expect(within(section).getByRole("option", { name: "Из источника истории" })).toBeDisabled();
+    expect(screen.queryByLabelText("GPU")).toBeNull();
+    await user.selectOptions(within(section).getByLabelText("Источник"), "direct");
+    await user.type(within(section).getByLabelText("Экспортёр 1"), "http://gpu-1:9400/metrics");
+    await user.selectOptions(within(section).getByLabelText("Вход"), "basic");
+    await user.type(within(section).getByLabelText("Имя пользователя"), "glafira");
+    await user.type(within(section).getByLabelText("Пароль"), "dcgm-pa55");
+    const server2 = screen.getByRole("group", { name: "Сервер 2" });
+    expect(within(server2).getByText("Найденные хосты появятся после «Проверить GPU»")).toBeInTheDocument();
+    await user.click(within(section).getByRole("button", { name: "Проверить GPU" }));
+    expect(api.checkGpuSource).toHaveBeenCalledWith(
+      expect.objectContaining({ gpu: { mode: "direct", exporters: [{ url: "http://gpu-1:9400/metrics" }], auth: { mode: "basic", username: "glafira" }, ca: { mode: "system" }, proxy: { mode: "system" } } }),
+      { apiKeys: {}, gpuSecret: "dcgm-pa55" },
+    );
+    const report = await within(section).findByRole("list", { name: "Проверка GPU" });
+    expect(within(report).getByText("Карты:")).toBeInTheDocument();
+    expect(within(report).getByText(/«gpu-2»: хоста сервера нет среди хостов GPU/).closest("li")).toHaveClass("warn");
+    const server1 = screen.getByRole("group", { name: "Сервер 1" });
+    expect(within(server1).getByLabelText("GPU")).toHaveValue("gpu-1");
+    await user.selectOptions(within(server2).getByLabelText("GPU"), "gpu-3");
+    await user.click(within(server2).getByLabelText("Все карты"));
+    await user.click(within(server2).getByLabelText("GPU 0"));
+    await user.click(screen.getByRole("button", { name: "Сохранить" }));
+    await waitFor(() => expect(api.saveInstallation).toHaveBeenCalled());
+    const [profile, secrets] = vi.mocked(api.saveInstallation).mock.calls[0];
+    expect(profile.servers.map((s) => s.gpus)).toEqual([{ host: "gpu-1" }, { host: "gpu-3", indices: [1] }]);
+    expect(secrets).toEqual({ apiKeys: {}, gpuSecret: "dcgm-pa55" });
+    expect(JSON.stringify(profile)).not.toContain("dcgm-pa55");
+  });
+
+  it("с источником истории: селектор экспортёра подбирается; режим «из истории» — автоподбор и выбор хостов", async () => {
+    const initial = saved();
+    initial.history = HISTORY;
+    initial.gpu = { mode: "direct", exporters: [{ url: "http://gpu-1:9400/metrics" }], auth: { mode: "none" }, ca: { mode: "system" }, proxy: { mode: "system" } };
+    vi.mocked(api.checkHistorySource).mockResolvedValue(CHECK);
+    vi.mocked(api.checkGpuSource).mockResolvedValue(GPU_CHECK);
+    const user = userEvent.setup();
+    renderWithClient(<InstallationForm initial={initial} tabId="form:i1" />);
+    const section = screen.getByRole("region", { name: "GPU (DCGM)" });
+    await waitFor(() => expect(within(section).getByLabelText("Серии в истории")).toHaveValue('{job="dcgm", instance="gpu-1:9400"}'));
+    expect(api.checkGpuSource).toHaveBeenCalledTimes(1);
+    expect(within(section).getByText("автоподбор")).toBeInTheDocument();
+
+    vi.mocked(api.checkGpuSource).mockResolvedValue({ ...GPU_CHECK, available: ["gpu-1", "gpu-3", "gpu-4"], selector: '{hostname=~"gpu-1"}' });
+    await user.selectOptions(within(section).getByLabelText("Источник"), "prometheus");
+    await user.click(within(section).getByRole("button", { name: "Проверить GPU" }));
+    await waitFor(() => expect(within(section).getByLabelText("Селектор карт")).toHaveValue('{hostname=~"gpu-1"}'));
+    const hosts = within(section).getByRole("group", { name: "Хосты" });
+    expect(within(hosts).getByLabelText("gpu-1")).toBeChecked();
+    await user.click(within(hosts).getByLabelText("gpu-4"));
+    expect(within(section).getByLabelText("Селектор карт")).toHaveValue('{hostname=~"gpu-1|gpu-4"}');
+    await user.click(within(hosts).getByLabelText("gpu-1"));
+    await user.click(within(hosts).getByLabelText("gpu-4"));
+    expect(within(section).getByLabelText("Селектор карт")).toHaveValue("");
+  });
+
+  it("помощники селектора хостов: разбор и сборка с экранированием", async () => {
+    const { hostsSelector, selectorHosts, withGpuMode } = await import("./GpuFields");
+    expect(selectorHosts('{hostname=~"gpu\\\\.corp|b"}')).toEqual({ label: "hostname", hosts: ["gpu.corp", "b"] });
+    expect(selectorHosts('{job="dcgm"}')).toBeNull();
+    expect(hostsSelector("Hostname", ["b", "gpu.corp"])).toBe('{Hostname=~"b|gpu\\\\.corp"}');
+    expect(hostsSelector("hostname", [])).toBeUndefined();
+    const direct = withGpuMode(undefined, "direct");
+    expect(direct?.mode).toBe("direct");
+    expect(withGpuMode(direct, "none")).toBeUndefined();
+    expect(withGpuMode({ mode: "prometheus" }, "direct", direct)).toBe(direct);
+  });
+});
+
 describe("InstallationForm: история метрик", () => {
   it("источник, проверка с автоподбором, свой селектор — всё уходит в сохранение, токен — секретом", async () => {
     vi.mocked(api.checkHistorySource).mockResolvedValue(CHECK);
````

- [ ] **Шаг 10.2: Увидеть падение.**

Run: `pnpm exec vitest run src/components/installations/InstallationForm.test.tsx`
Expected: FAIL — не находится `./GpuFields` (Failed to resolve import): тест помощников селектора хостов импортирует модуль

- [ ] **Шаг 10.3: Реализация.** Раздел формы, поле сервера, подключение к форме, словари и исключения `same-as-en`.

<!-- block: patch t10-docs-i18n-same-as-en-json step=10.3 -->
````
diff --git a/docs/i18n/same-as-en.json b/docs/i18n/same-as-en.json
index b3f5934..67b2be2 100644
--- a/docs/i18n/same-as-en.json
+++ b/docs/i18n/same-as-en.json
@@ -18,6 +18,8 @@
     "installations.form.kinds.vllm",
     "installations.form.step.tcp",
     "installations.form.step.tls",
+    "installations.gpu.binding",
+    "installations.gpu.title",
     "messages.connectionTest.kind",
     "messages.connectionTest.kindVersion",
     "servers.metrics.sources.prometheus",
````

<!-- block: file t10-src-components-installations-gpufields-tsx step=10.3 path=src/components/installations/GpuFields.tsx -->
````
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { toAppError } from "../../api/client";
import type { AppError, DcgmTarget, GpuAuth, GpuBinding, GpuCheck, GpuExporter, GpuSource, InstallationProfile } from "../../api/types";
import { tr } from "../../i18n/text";
import { ErrorBanner } from "../common/ErrorBanner";
import { Hint } from "../common/Hint";
import type { SecretInput } from "./HistoryFields";
import { selectorLabel } from "./HistoryFields";
import { type CaMode, type ProxyMode, stepMark, withCaMode, withProxyMode } from "./profile";

const CA_MODES: CaMode[] = ["system", "pem", "insecure"];
const PROXY_MODES: ProxyMode[] = ["system", "none", "url"];
type Mode = "none" | GpuSource["mode"];
const MODES: Mode[] = ["none", "direct", "prometheus"];
/** Сколько экспортёров может быть у компонента (как `MAX_EXPORTERS` в Rust). */
export const MAX_EXPORTERS = 16;
/** Сколько найденных хостов показывает выбор хостов селектора. */
const MAX_HOSTS = 20;

const CUSTOM = "\u0000custom";
const NONE = "";

export function newDirectGpu(): GpuSource {
  return { mode: "direct", exporters: [{ url: "" }], auth: { mode: "none" }, ca: { mode: "system" }, proxy: { mode: "system" } };
}

/** Компонент в режиме `mode`; адреса экспортёров сохраняются при переключении туда и обратно. */
export function withGpuMode(gpu: GpuSource | undefined, mode: Mode, lastDirect?: GpuSource): GpuSource | undefined {
  if (mode === "none") return undefined;
  if (mode === "prometheus") return gpu?.mode === "prometheus" ? gpu : { mode: "prometheus" };
  if (gpu?.mode === "direct") return gpu;
  return lastDirect?.mode === "direct" ? lastDirect : newDirectGpu();
}

/** Селектор цели экспортёра — как его строит Rust (`DcgmTarget::selector`). */
export const dcgmSelector = (t: DcgmTarget) =>
  `{job="${t.job.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}", instance="${t.instance.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"}`;

const reEscape = (v: string) => v.replace(/[\\.+*?()|[\]{}^$]/g, "\\$&");

/** Хосты селектора вида `{label=~"a|b"}` (как его строит автоподбор); иной селектор — `null`. */
export function selectorHosts(selector: string | undefined): { label: string; hosts: string[] } | null {
  const m = /^\{\s*(hostname|Hostname)\s*=~\s*"((?:[^"\\]|\\.)*)"\s*\}$/.exec(selector?.trim() ?? "");
  if (!m) return null;
  const value = m[2].replace(/\\\\/g, "\\");
  return { label: m[1], hosts: value ? value.split("|").map((h) => h.replace(/\\(.)/g, "$1")) : [] };
}

/** Селектор хостов: `{hostname=~"a|b"}`; без хостов — `undefined` (все карты источника). */
export function hostsSelector(label: string, hosts: string[]): string | undefined {
  if (hosts.length === 0) return undefined;
  const value = [...hosts].sort().map(reEscape).join("|").replace(/\\/g, "\\\\");
  return `{${label}=~"${value}"}`;
}

interface Props {
  profile: InstallationProfile;
  idPrefix: string;
  /** У экспортёров сохранён пароль: пустое поле его оставляет. */
  savedSecret: boolean;
  secret: SecretInput;
  insecureForbidden: boolean;
  busy: boolean;
  result: GpuCheck | null;
  onResult: (r: GpuCheck | null) => void;
  onGpu: (gpu: GpuSource | undefined) => void;
  onSecret: (patch: Partial<SecretInput>) => void;
  onBinding: (serverId: string, binding: GpuBinding | undefined) => void;
  check: () => Promise<GpuCheck>;
}

/**
 * Раздел формы «GPU (DCGM)» (спека I1d §5): режим, экспортёры с входом, CA и прокси или селектор карт источника
 * истории, проверка и автоподбор. Автоподбор ставит привязку серверам без неё, селектор истории — экспортёрам без него и
 * селектор карт — пустому: выбор человека не перезаписывается. Подобранная под старый адрес привязка сбрасывается.
 */
export function GpuFields({ profile, idPrefix, savedSecret, secret, insecureForbidden, busy, result, onResult, onGpu, onSecret, onBinding, check }: Props) {
  const { t } = useTranslation();
  const id = (field: string) => `${idPrefix}-${field}`;
  const g = profile.gpu;
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState<AppError | null>(null);
  // Серверы с подобранной привязкой и адрес, под который она подобрана; экспортёры с подобранным селектором.
  const [auto, setAuto] = useState<Map<string, string>>(new Map());
  const [autoExporters, setAutoExporters] = useState<Set<string>>(new Set());
  const [autoSelector, setAutoSelector] = useState(false);
  const [custom, setCustom] = useState<Set<number>>(new Set());
  const [lastDirect, setLastDirect] = useState<GpuSource | undefined>(undefined);
  const [lastPem, setLastPem] = useState("");
  const [lastProxy, setLastProxy] = useState("");

  useEffect(() => {
    const stale = profile.servers.filter((s) => auto.has(s.id) && auto.get(s.id) !== s.baseUrl);
    if (stale.length === 0) return;
    for (const s of stale) onBinding(s.id, undefined);
    setAuto((a) => new Map([...a].filter(([sid]) => !stale.some((s) => s.id === sid))));
  }, [profile.servers, auto, onBinding]);

  const onCheck = async () => {
    setChecking(true);
    setError(null);
    onResult(null);
    try {
      const r = await check();
      onResult(r);
      const picked = new Map<string, string>();
      for (const b of r.bindings) {
        const server = profile.servers.find((x) => x.id === b.serverId);
        if (b.binding && server && !server.gpus) {
          onBinding(b.serverId, b.binding);
          picked.set(b.serverId, server.baseUrl);
        }
      }
      setAuto((a) => new Map([...a, ...picked]));
      if (profile.gpu?.mode === "prometheus" && !profile.gpu.selector && r.selector) {
        onGpu({ ...profile.gpu, selector: r.selector });
        setAutoSelector(true);
      }
      if (profile.gpu?.mode === "direct") {
        const urls = new Set<string>();
        const exporters = profile.gpu.exporters.map((e) => {
          const s = r.exporters.find((x) => x.url === e.url);
          if (!e.historySelector && s?.historySelector) {
            urls.add(e.url);
            return { ...e, historySelector: s.historySelector };
          }
          return e;
        });
        if (urls.size > 0) {
          onGpu({ ...profile.gpu, exporters });
          setAutoExporters((a) => new Set([...a, ...urls]));
        }
      }
    } catch (e) {
      setError(toAppError(e));
    } finally {
      setChecking(false);
    }
  };

  // Форма открыта с сохранённым компонентом — проверка запускается сама: без неё нет списков хостов и целей.
  const autoChecked = useRef(false);
  useEffect(() => {
    if (autoChecked.current) return;
    autoChecked.current = true;
    if (g) void onCheck();
    // Только при открытии формы: дальше проверку запускает человек.
  }, []);

  const setMode = (mode: Mode) => {
    if (g?.mode === "direct") setLastDirect(g);
    if (mode === "none") onSecret({ value: "", clear: false });
    onResult(null);
    onGpu(withGpuMode(g, mode, lastDirect));
  };

  return (
    <section className="history-source" aria-label={t("installations.gpu.title")}>
      <h3 className="form-section">{t("installations.gpu.title")}</h3>
      <div className="form">
        <span className="field-label">
          <label htmlFor={id("mode")}>{t("installations.gpu.mode")}</label>
          <Hint k="hints.installations.gpuMode" label={t("installations.gpu.mode")} />
        </span>
        <select id={id("mode")} value={g?.mode ?? "none"} onChange={(e) => setMode(e.target.value as Mode)}>
          {MODES.map((m) => (
            <option key={m} value={m} disabled={m === "prometheus" && !profile.history && g?.mode !== "prometheus"}>
              {t(`installations.gpu.modes.${m}`)}
            </option>
          ))}
        </select>
        {!profile.history && g?.mode !== "prometheus" && <p className="muted full">{t("installations.gpu.prometheusNeedsHistory")}</p>}
        {!profile.history && g?.mode === "prometheus" && (
          <p className="warn full" role="alert">
            {t("installations.gpu.historyRemoved")}
          </p>
        )}
      </div>
      {g?.mode === "direct" && (
        <DirectFields
          gpu={g}
          id={id}
          withHistory={!!profile.history}
          result={result}
          savedSecret={savedSecret}
          secret={secret}
          insecureForbidden={insecureForbidden}
          busy={busy}
          autoExporters={autoExporters}
          custom={custom}
          lastPem={lastPem}
          lastProxy={lastProxy}
          onGpu={onGpu}
          onSecret={onSecret}
          onCustom={(index, on) =>
            setCustom((c) => {
              const next = new Set(c);
              if (on) next.add(index);
              else next.delete(index);
              return next;
            })
          }
          onManual={(url) => setAutoExporters((a) => new Set([...a].filter((x) => x !== url)))}
          onLastPem={setLastPem}
          onLastProxy={setLastProxy}
        />
      )}
      {g?.mode === "prometheus" && (
        <div className="form">
          <span className="field-label">
            <label htmlFor={id("selector")}>{t("installations.gpu.selector")}</label>
            <Hint k="hints.installations.gpuSelector" label={t("installations.gpu.selector")} />
          </span>
          <input
            id={id("selector")}
            placeholder='{hostname=~"gpu-node-1|gpu-node-2"}'
            value={g.selector ?? ""}
            onChange={(e) => {
              setAutoSelector(false);
              onGpu({ ...g, selector: e.target.value || undefined });
            }}
          />
          {autoSelector && g.selector && <span className="muted full">{t("installations.gpu.auto")}</span>}
          {result && result.available.length > 0 && (
            <fieldset className="full" aria-label={t("installations.gpu.hosts")}>
              <legend>{t("installations.gpu.hosts")}</legend>
              {result.available.slice(0, MAX_HOSTS).map((h) => {
                const parsed = selectorHosts(g.selector);
                const checked = parsed?.hosts.includes(h) ?? false;
                return (
                  <label key={h} className="inline">
                    <input
                      type="checkbox"
                      checked={checked}
                      onChange={(e) => {
                        const label = parsed?.label ?? selectorHosts(result.selector)?.label ?? "hostname";
                        const hosts = new Set(parsed?.hosts ?? []);
                        if (e.target.checked) hosts.add(h);
                        else hosts.delete(h);
                        setAutoSelector(false);
                        onGpu({ ...g, selector: hostsSelector(label, [...hosts]) });
                      }}
                    />{" "}
                    {h}
                  </label>
                );
              })}
              {result.available.length > MAX_HOSTS && <span className="muted">{t("installations.gpu.moreHosts", { count: result.available.length - MAX_HOSTS })}</span>}
            </fieldset>
          )}
        </div>
      )}
      {g && (
        <div className="toolbar">
          <button type="button" disabled={busy || checking} onClick={() => void onCheck()}>
            {t("installations.gpu.check")}
          </button>
          <Hint k="hints.installations.gpuCheck" label={t("installations.gpu.check")} />
          {checking && <span className="muted">{t("installations.form.testing")}</span>}
        </div>
      )}
      {error && <ErrorBanner error={error} />}
      {result && (
        <ul className="steps" aria-label={t("installations.gpu.report")}>
          {result.report.steps.map((step, i) => (
            <li key={`${step.name}-${i}`} className={step.warning ? "warn" : undefined}>
              {stepMark(step)} <strong>{t(`installations.form.step.${step.name}`)}:</strong> <span>{tr(step.message)}</span>
              {step.error?.details && <pre className="muted">{step.error.details}</pre>}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

/** Поля режима `direct`: экспортёры, вход, CA, прокси и серии экспортёров в истории. */
function DirectFields({
  gpu,
  id,
  withHistory,
  result,
  savedSecret,
  secret,
  insecureForbidden,
  busy,
  autoExporters,
  custom,
  lastPem,
  lastProxy,
  onGpu,
  onSecret,
  onCustom,
  onManual,
  onLastPem,
  onLastProxy,
}: {
  gpu: Extract<GpuSource, { mode: "direct" }>;
  id: (field: string) => string;
  withHistory: boolean;
  result: GpuCheck | null;
  savedSecret: boolean;
  secret: SecretInput;
  insecureForbidden: boolean;
  busy: boolean;
  autoExporters: Set<string>;
  custom: Set<number>;
  lastPem: string;
  lastProxy: string;
  onGpu: (gpu: GpuSource) => void;
  onSecret: (patch: Partial<SecretInput>) => void;
  onCustom: (index: number, on: boolean) => void;
  onManual: (url: string) => void;
  onLastPem: (pem: string) => void;
  onLastProxy: (url: string) => void;
}) {
  const { t } = useTranslation();
  const set = (patch: Partial<typeof gpu>) => onGpu({ ...gpu, ...patch });
  const setExporter = (index: number, patch: Partial<GpuExporter>) =>
    set({ exporters: gpu.exporters.map((e, i) => (i === index ? { ...e, ...patch } : e)) });
  const auth = gpu.auth;
  return (
    <>
      <h4>{t("installations.gpu.exporters")}</h4>
      <div className="form">
        {gpu.exporters.map((e, i) => {
          const targets = result?.exporters.find((x) => x.url === e.url)?.targets ?? [];
          const sel = e.historySelector;
          const known = targets.some((x) => dcgmSelector(x) === sel);
          const saved = sel && !known && selectorLabel(sel) !== sel ? sel : undefined;
          const value = custom.has(i) || (sel && !known && !saved) ? CUSTOM : (sel ?? NONE);
          return (
            <div key={i} className="contents">
              <label htmlFor={id(`exp-${i}`)}>{t("installations.gpu.exporter", { n: i + 1 })}</label>
              <span className="path-field">
                <input id={id(`exp-${i}`)} placeholder="http://gpu-node-1:9400/metrics" value={e.url} onChange={(ev) => setExporter(i, { url: ev.target.value })} />
                {gpu.exporters.length > 1 && (
                  <button type="button" disabled={busy} onClick={() => set({ exporters: gpu.exporters.filter((_, j) => j !== i) })}>
                    {t("installations.gpu.removeExporter")}
                  </button>
                )}
              </span>
              {withHistory && (
                <>
                  <span className="field-label">
                    <label htmlFor={id(`exp-sel-${i}`)}>{t("installations.gpu.exporterSeries")}</label>
                    {autoExporters.has(e.url) && sel && <span className="muted"> {t("installations.gpu.auto")}</span>}
                  </span>
                  <select
                    id={id(`exp-sel-${i}`)}
                    value={value}
                    onChange={(ev) => {
                      onManual(e.url);
                      onCustom(i, ev.target.value === CUSTOM);
                      if (ev.target.value !== CUSTOM) setExporter(i, { historySelector: ev.target.value || undefined });
                    }}
                  >
                    <option value={NONE}>{t("installations.history.noMatch")}</option>
                    {saved && <option value={saved}>{selectorLabel(saved)}</option>}
                    {targets.map((x) => (
                      <option key={dcgmSelector(x)} value={dcgmSelector(x)}>
                        {`${x.job} · ${x.instance}`}
                      </option>
                    ))}
                    <option value={CUSTOM}>{t("installations.history.custom")}</option>
                  </select>
                  {value === CUSTOM && (
                    <input
                      className="full"
                      aria-label={t("installations.gpu.exporterSelector", { n: i + 1 })}
                      placeholder='{job="dcgm", instance="10.0.0.5:9400"}'
                      value={sel ?? ""}
                      onChange={(ev) => setExporter(i, { historySelector: ev.target.value || undefined })}
                    />
                  )}
                </>
              )}
            </div>
          );
        })}
      </div>
      <div className="toolbar">
        <button type="button" disabled={busy || gpu.exporters.length >= MAX_EXPORTERS} onClick={() => set({ exporters: [...gpu.exporters, { url: "" }] })}>
          {t("installations.gpu.addExporter")}
        </button>
      </div>
      <div className="form">
        <label htmlFor={id("auth")}>{t("installations.gpu.auth")}</label>
        <select
          id={id("auth")}
          value={auth.mode}
          onChange={(e) => set({ auth: e.target.value === "basic" ? ({ mode: "basic", username: auth.mode === "basic" ? auth.username : "" } as GpuAuth) : { mode: "none" } })}
        >
          <option value="none">{t("installations.history.authMode.none")}</option>
          <option value="basic">{t("installations.history.authMode.basic")}</option>
        </select>
        {auth.mode === "basic" && (
          <>
            <label htmlFor={id("user")}>{t("installations.history.username")}</label>
            <input id={id("user")} autoComplete="off" value={auth.username} onChange={(e) => set({ auth: { mode: "basic", username: e.target.value } })} />
            <label htmlFor={id("secret")}>{t("installations.history.password")}</label>
            <input
              id={id("secret")}
              type="password"
              autoComplete="off"
              placeholder={savedSecret ? t("installations.form.apiKeyKeep") : ""}
              disabled={savedSecret && secret.clear}
              value={secret.value}
              onChange={(e) => onSecret({ value: e.target.value })}
            />
            {savedSecret && (
              <label className="inline full">
                <input type="checkbox" checked={secret.clear} onChange={(e) => onSecret({ clear: e.target.checked })} /> {t("installations.gpu.secretClear")}
              </label>
            )}
          </>
        )}
        <label htmlFor={id("ca")}>{t("installations.form.ca")}</label>
        <select
          id={id("ca")}
          value={gpu.ca.mode}
          onChange={(e) => {
            if (gpu.ca.mode === "pem") onLastPem(gpu.ca.pem);
            set({ ca: withCaMode(gpu.ca, e.target.value as CaMode, lastPem) });
          }}
        >
          {CA_MODES.map((m) => (
            <option key={m} value={m} disabled={m === "insecure" && insecureForbidden && gpu.ca.mode !== "insecure"}>
              {t(`installations.form.caMode.${m}`)}
            </option>
          ))}
        </select>
        {gpu.ca.mode === "pem" && (
          <textarea
            className="full"
            aria-label={t("installations.form.caPem")}
            rows={4}
            placeholder="-----BEGIN CERTIFICATE-----"
            value={gpu.ca.pem}
            onChange={(e) => set({ ca: { mode: "pem", pem: e.target.value } })}
          />
        )}
        {gpu.ca.mode === "insecure" && (
          <p className="warn full" role="alert">
            {insecureForbidden ? t("installations.form.insecureForbidden") : t("installations.form.insecureWarning")}
          </p>
        )}
        <label htmlFor={id("proxy")}>{t("installations.form.proxy")}</label>
        <select
          id={id("proxy")}
          value={gpu.proxy.mode}
          onChange={(e) => {
            if (gpu.proxy.mode === "url") onLastProxy(gpu.proxy.url);
            set({ proxy: withProxyMode(gpu.proxy, e.target.value as ProxyMode, lastProxy) });
          }}
        >
          {PROXY_MODES.map((m) => (
            <option key={m} value={m}>
              {t(`installations.form.proxyMode.${m}`)}
            </option>
          ))}
        </select>
        {gpu.proxy.mode === "url" && (
          <input
            className="full"
            aria-label={t("installations.form.proxyUrl")}
            placeholder="http://proxy.example.com:3128"
            value={gpu.proxy.url}
            onChange={(e) => set({ proxy: { mode: "url", url: e.target.value } })}
          />
        )}
      </div>
    </>
  );
}

/**
 * Поле «GPU» сервера (спека I1d §5): хост из найденных проверкой и карты — все или выбранные. Сохранённая привязка к
 * хосту, которого нет среди найденных, — своим пунктом с пометкой «не найден».
 */
export function GpuBindingField({
  binding,
  hosts,
  checked,
  idPrefix,
  onChange,
}: {
  binding: GpuBinding | undefined;
  hosts: { host: string; cards: number[] }[];
  /** Проверка GPU была: без неё списка хостов нет. */
  checked: boolean;
  idPrefix: string;
  onChange: (b: GpuBinding | undefined) => void;
}) {
  const { t } = useTranslation();
  const known = hosts.find((h) => binding && h.host.toLowerCase() === binding.host.toLowerCase());
  const cards = known?.cards ?? binding?.indices ?? [];
  return (
    <>
      <span className="field-label">
        <label htmlFor={`${idPrefix}-gpu`}>{t("installations.gpu.binding")}</label>
        <Hint k="hints.installations.gpuBinding" label={t("installations.gpu.binding")} />
      </span>
      <select
        id={`${idPrefix}-gpu`}
        value={binding?.host ?? ""}
        onChange={(e) => onChange(e.target.value ? { host: e.target.value } : undefined)}
      >
        <option value="">{t("installations.gpu.noBinding")}</option>
        {binding && !known && <option value={binding.host}>{t("installations.gpu.hostMissing", { host: binding.host })}</option>}
        {hosts.map((h) => (
          <option key={h.host} value={h.host}>
            {h.host}
          </option>
        ))}
      </select>
      {!checked && <p className="muted full">{t("installations.gpu.checkForHosts")}</p>}
      {binding && cards.length > 0 && (
        <fieldset className="full" aria-label={t("installations.gpu.cards")}>
          <label className="inline">
            <input type="checkbox" checked={!binding.indices} onChange={(e) => onChange({ host: binding.host, indices: e.target.checked ? undefined : [...cards] })} />{" "}
            {t("installations.gpu.allCards")}
          </label>
          {binding.indices &&
            cards.map((i) => (
              <label key={i} className="inline">
                <input
                  type="checkbox"
                  checked={binding.indices?.includes(i) ?? false}
                  onChange={(e) => {
                    const set = new Set(binding.indices ?? []);
                    if (e.target.checked) set.add(i);
                    else set.delete(i);
                    onChange({ host: binding.host, indices: [...set].sort((a, b) => a - b) });
                  }}
                />{" "}
                {t("gpu.card.title", { index: i })}
              </label>
            ))}
        </fieldset>
      )}
    </>
  );
}
````

<!-- block: patch t10-src-components-installations-installationform-tsx step=10.3 -->
````
diff --git a/src/components/installations/InstallationForm.tsx b/src/components/installations/InstallationForm.tsx
index 4f7d3cc..be08dc5 100644
--- a/src/components/installations/InstallationForm.tsx
+++ b/src/components/installations/InstallationForm.tsx
@@ -2,11 +2,12 @@ import { useId, useMemo, useState } from "react";
 import { useTranslation } from "react-i18next";
 import { useQuery, useQueryClient } from "@tanstack/react-query";
 import { api, toAppError } from "../../api/client";
-import type { AppError, Environment, InstallationProfile, SecretsInput, ServerConfig, ServerKind, TestReport } from "../../api/types";
+import type { AppError, Environment, GpuCheck, InstallationProfile, SecretsInput, ServerConfig, ServerKind, TestReport } from "../../api/types";
 import { tr } from "../../i18n/text";
 import { useWorkspace } from "../../state/workspace";
 import { ErrorBanner } from "../common/ErrorBanner";
 import { Hint } from "../common/Hint";
+import { GpuBindingField, GpuFields } from "./GpuFields";
 import { HistoryFields, type SecretInput } from "./HistoryFields";
 import { type CaMode, isPlainHttp, MAX_SERVERS, newServer, type ProxyMode, stepMark, withCaMode, withProxyMode } from "./profile";
 import { folderPaths } from "./tree/folderTree";
@@ -32,6 +33,9 @@ export function InstallationForm({ initial, tabId }: { initial: InstallationProf
   const [keys, setKeys] = useState<Record<string, KeyInput>>({});
   // Секрет источника истории: пустое поле — оставить сохранённый.
   const [historySecret, setHistorySecret] = useState<SecretInput>({ value: "", clear: false });
+  // Пароль экспортёров DCGM и итог проверки GPU: хосты из него нужны полю «GPU» каждого сервера.
+  const [gpuSecret, setGpuSecret] = useState<SecretInput>({ value: "", clear: false });
+  const [gpuCheck, setGpuCheck] = useState<GpuCheck | null>(null);
   // Последние введённые PEM и адрес прокси по серверам: переключение режима туда и обратно их не теряет.
   const [lastPem, setLastPem] = useState<Record<string, string>>({});
   const [lastProxy, setLastProxy] = useState<Record<string, string>>({});
@@ -71,6 +75,9 @@ export function InstallationForm({ initial, tabId }: { initial: InstallationProf
     setKeys((k) => ({ ...k, [id]: { value: k[id]?.value ?? "", clear: k[id]?.clear ?? false, ...patch } }));
   // Сохранённый секрет источника есть, пока вид входа тот же; «удалить» действует только при нём.
   const savedHistorySecret = !isNew && !!initial.history && initial.history.auth.mode !== "none" && initial.history.auth.mode === profile.history?.auth.mode;
+  const gpuLogin = (p: InstallationProfile) => p.gpu?.mode === "direct" && p.gpu.auth.mode === "basic";
+  // Сохранённый пароль экспортёров есть, пока у них вход basic.
+  const savedGpuSecret = !isNew && gpuLogin(initial) && gpuLogin(profile);
   const current = (): InstallationProfile => ({ ...profile, folderId });
   const secrets = (): SecretsInput => {
     const apiKeys: Record<string, string> = {};
@@ -84,6 +91,10 @@ export function InstallationForm({ initial, tabId }: { initial: InstallationProf
       if (savedHistorySecret && historySecret.clear) out.historySecret = "";
       else if (historySecret.value.trim()) out.historySecret = historySecret.value;
     }
+    if (gpuLogin(profile)) {
+      if (savedGpuSecret && gpuSecret.clear) out.gpuSecret = "";
+      else if (gpuSecret.value.trim()) out.gpuSecret = gpuSecret.value;
+    }
     return out;
   };
 
@@ -266,6 +277,7 @@ export function InstallationForm({ initial, tabId }: { initial: InstallationProf
           }}
           onTest={() => onTest(s.id)}
           onRemove={() => removeServer(s.id)}
+          gpu={profile.gpu ? { hosts: gpuCheck?.hosts ?? [], checked: !!gpuCheck } : undefined}
         />
       ))}
       <div className="toolbar">
@@ -290,6 +302,21 @@ export function InstallationForm({ initial, tabId }: { initial: InstallationProf
         check={() => api.checkHistorySource(current(), secrets())}
       />
 
+      <GpuFields
+        profile={profile}
+        idPrefix={fid("gpu")}
+        savedSecret={savedGpuSecret}
+        secret={gpuSecret}
+        insecureForbidden={insecureForbidden}
+        busy={busy}
+        result={gpuCheck}
+        onResult={setGpuCheck}
+        onGpu={(gpu) => update({ gpu })}
+        onSecret={(patch) => setGpuSecret((s) => ({ ...s, ...patch }))}
+        onBinding={(serverId, gpus) => updateServer(serverId, { gpus })}
+        check={() => api.checkGpuSource(current(), secrets())}
+      />
+
       <div className="toolbar">
         <button type="button" className="primary" disabled={busy} onClick={onSave}>
           {t("common.save")}
@@ -335,6 +362,7 @@ function ServerFields({
   onProxyMode,
   onTest,
   onRemove,
+  gpu,
 }: {
   server: ServerConfig;
   index: number;
@@ -354,6 +382,8 @@ function ServerFields({
   onProxyMode: (mode: ProxyMode) => void;
   onTest: () => void;
   onRemove: () => void;
+  /** Поле «GPU»: хосты из проверки GPU; без компонента GPU поля нет. */
+  gpu?: { hosts: GpuCheck["hosts"]; checked: boolean };
 }) {
   const { t } = useTranslation();
   const id = (field: string) => `${idPrefix}-${field}`;
@@ -461,6 +491,7 @@ function ServerFields({
             onChange={(e) => onChange({ proxy: { mode: "url", url: e.target.value } })}
           />
         )}
+        {gpu && <GpuBindingField binding={s.gpus} hosts={gpu.hosts} checked={gpu.checked} idPrefix={idPrefix} onChange={(gpus) => onChange({ gpus })} />}
       </div>
       <div className="toolbar">
         <button type="button" disabled={busy} onClick={onTest}>
````

<!-- block: locales t10-locales-t10 step=10.3 -->
````
{
  "en": {
    "installations": {
      "form": {
        "step": {
          "metrics": "Metrics",
          "cards": "GPUs",
          "bind": "Binding",
          "history": "History",
          "counters": "Counters"
        }
      },
      "gpu": {
        "title": "GPU (DCGM)",
        "mode": "Source",
        "modes": {
          "none": "None",
          "direct": "DCGM exporters",
          "prometheus": "From the history source"
        },
        "prometheusNeedsHistory": "GPU metrics from the history source need a metrics history source",
        "historyRemoved": "Without the metrics history source this GPU component will not be saved: choose DCGM exporters or add the source back",
        "selector": "Card selector",
        "auto": "auto-match",
        "hosts": "Hosts",
        "moreHosts": "and {{count}} more",
        "check": "Check GPU",
        "report": "GPU check",
        "exporters": "DCGM exporters",
        "exporter": "Exporter {{n}}",
        "removeExporter": "Remove",
        "exporterSeries": "Series in history",
        "exporterSelector": "Series selector of exporter {{n}}",
        "addExporter": "Add exporter",
        "auth": "Sign-in",
        "secretClear": "Delete the saved password",
        "binding": "GPU",
        "noBinding": "None",
        "hostMissing": "{{host}} (not found)",
        "checkForHosts": "The found hosts appear after “Check GPU”",
        "cards": "GPUs",
        "allCards": "All GPUs"
      }
    },
    "hints": {
      "installations": {
        "gpuMode": "DCGM exporters: Glafira reads /metrics of dcgm-exporter itself. From the history source: the same DCGM series from Prometheus or VictoriaMetrics.",
        "gpuSelector": "Label matchers that pick the GPUs of the installation, such as {hostname=~\"gpu-1|gpu-2\"}. Empty — the check picks the hosts of the servers. At most 64 GPUs.",
        "gpuCheck": "Checks the exporters or the history source, finds the GPU hosts, binds servers whose address matches a host and matches the exporters with history targets.",
        "gpuBinding": "The GPUs of the server: a host and all or some cards. The check binds a server whose address matches a GPU host; behind a load balancer choose the host manually."
      }
    }
  },
  "ru": {
    "installations": {
      "form": {
        "step": {
          "metrics": "Метрики",
          "cards": "Карты",
          "bind": "Привязка",
          "history": "История",
          "counters": "Счётчики"
        }
      },
      "gpu": {
        "title": "GPU (DCGM)",
        "mode": "Источник",
        "modes": {
          "none": "Нет",
          "direct": "Экспортёры DCGM",
          "prometheus": "Из источника истории"
        },
        "prometheusNeedsHistory": "Метрикам GPU из источника истории нужен источник истории метрик",
        "historyRemoved": "Без источника истории метрик этот компонент GPU не сохранится: выберите экспортёры DCGM или верните источник",
        "selector": "Селектор карт",
        "auto": "автоподбор",
        "hosts": "Хосты",
        "moreHosts": "и ещё {{count}}",
        "check": "Проверить GPU",
        "report": "Проверка GPU",
        "exporters": "Экспортёры DCGM",
        "exporter": "Экспортёр {{n}}",
        "removeExporter": "Убрать",
        "exporterSeries": "Серии в истории",
        "exporterSelector": "Селектор серий экспортёра {{n}}",
        "addExporter": "Добавить экспортёр",
        "auth": "Вход",
        "secretClear": "Удалить сохранённый пароль",
        "binding": "GPU",
        "noBinding": "Нет",
        "hostMissing": "{{host}} (не найден)",
        "checkForHosts": "Найденные хосты появятся после «Проверить GPU»",
        "cards": "Карты",
        "allCards": "Все карты"
      }
    },
    "hints": {
      "installations": {
        "gpuMode": "Экспортёры DCGM: Glafira сама читает /metrics dcgm-exporter. Из источника истории: те же серии DCGM из Prometheus или VictoriaMetrics.",
        "gpuSelector": "Условия на метки, выбирающие карты инсталляции, например {hostname=~\"gpu-1|gpu-2\"}. Пусто — проверка подберёт хосты серверов. Не больше 64 карт.",
        "gpuCheck": "Проверяет экспортёры или источник истории, находит хосты карт, привязывает серверы с совпавшим адресом и сопоставляет экспортёры с целями истории.",
        "gpuBinding": "Карты сервера: хост и все или некоторые карты. Проверка привязывает сервер, адрес которого совпал с хостом карт; за балансировщиком выберите хост вручную."
      }
    }
  }
}
````

- [ ] **Шаг 10.4: Проверки.**

Run: `pnpm test`
Expected: `Test Files  39 passed`, `Tests  282 passed`
Run: `pnpm exec tsc --noEmit`
Expected: без вывода
Run: `pnpm i18n:check`
Expected: `✓ словари en, ru согласованы, вызовов из Rust: 187`

- [ ] **Шаг 10.5: Коммит.**

<!-- block: shell t10-commit step=10.5 -->
````
git add docs/i18n/same-as-en.json \
  src/components/installations/GpuFields.tsx \
  src/components/installations/InstallationForm.test.tsx \
  src/components/installations/InstallationForm.tsx \
  src/locales/en.json \
  src/locales/ru.json
git commit -s -q -F - <<'EOF'
feat(installations): GPU section of the installation form and server GPU field

The GPU (DCGM) section picks no GPU, DCGM exporters (addresses, basic
login with the saved password rule, CA, proxy, series of each exporter
in history) or the history source (card selector, found hosts as
checkboxes). Check GPU shows the steps and applies auto-matched
bindings, exporter selectors and the card selector only where none is
set; a binding matched under an old server address is reset. A saved
component is checked when the form opens. Each server gets a GPU field:
a found host and all or some of its cards.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
````

---

### Задача 11: Блоки GPU в обзорах инсталляции и сервера

«Обзор» инсталляции с компонентом GPU — блок «GPU по хостам»: модель или «разные», число карт, средняя
утилизация, память, максимальная температура, число карт с проблемами; щелчок открывает вкладку GPU. «Обзор» сервера —
блок «GPU сервера»: привязанные карты с утилизацией, памятью, температурой и значками и «Открыть GPU»; без привязки —
одна подсказка и без опроса (спека I1d §5).

**Files:**
- Create: `src/components/gpu/GpuSummary.test.tsx`
- Create: `src/components/gpu/GpuSummary.tsx`
- Create: `src/components/gpu/summary.ts`
- Modify: `src/app.css`
- Modify: `src/components/installations/InstallationPanel.test.tsx`
- Modify: `src/components/installations/InstallationPanel.tsx`
- Modify: `src/components/servers/ServerPanel.test.tsx`
- Modify: `src/components/servers/ServerPanel.tsx`
- Modify: `src/locales/en.json`
- Modify: `src/locales/ru.json`

**Interfaces:**
- Consumes: `useGpuStatus`, `cardAlerts`.
- Produces: `GpuHostsBlock({ profile })`, `ServerGpuBlock({ profile, server })` (`GpuSummary.tsx`); `hostSummary`,
  `boundCards`, `HostSummary` (`summary.ts`); ключи `gpu.overview.*`.

- [ ] **Шаг 11.1: Тесты.** Тесты сводки хоста, карт привязки, обоих блоков и их подключения к панелям.

<!-- block: file t11-src-components-gpu-gpusummary-test-tsx step=11.1 path=src/components/gpu/GpuSummary.test.tsx -->
````
import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { InstallationProfile } from "../../api/types";
import { useWorkspace } from "../../state/workspace";
import { renderWithClient } from "../../test/render";
import { newInstallation, newServer } from "../installations/profile";

vi.mock("../../api/client", async (importOriginal) => {
  const orig = await importOriginal<typeof import("../../api/client")>();
  return { ...orig, api: { readGpuStatus: vi.fn() } };
});

import { api } from "../../api/client";
import { GpuHostsBlock, ServerGpuBlock } from "./GpuSummary";
import { boundCards, hostSummary } from "./summary";
import { card, host, status } from "./testGpu";

const profile = (): InstallationProfile => ({ ...newInstallation(), id: "i1", servers: [{ ...newServer(), id: "s1", name: "vllm-1" }], gpu: { mode: "prometheus" } });

beforeEach(() => {
  vi.clearAllMocks();
  useWorkspace.setState({ tabs: [], activeId: null });
});

describe("сводка GPU", () => {
  it("строка хоста: модель или «разные», средняя утилизация, сумма памяти, максимум температуры, карты с проблемами", () => {
    const h = host("gpu-1", [
      card(0, { values: { util: 80, memUsed: 1024, memTotal: 2048, temp: 70 } }),
      card(1, { values: { util: 40, memUsed: 1024, memTotal: 2048, temp: 85, xid: 79 } }),
    ]);
    expect(hostSummary(h)).toEqual({ host: "gpu-1", model: "NVIDIA H100 80GB HBM3", count: 2, avgUtil: 60, memUsed: 2048, memTotal: 4096, maxTemp: 85, troubled: 1 });
    const mixed = hostSummary(host("x", [card(0), card(1, { model: "NVIDIA A100", values: {} })]));
    expect(mixed.model).toBeNull();
    expect(hostSummary(host("y", [card(0, { values: {} })]))).toMatchObject({ avgUtil: null, memUsed: null, maxTemp: null });
  });

  it("карты привязки — по хосту без учёта регистра и номерам", () => {
    const s = status(1, [host("gpu-1", [card(0), card(1), card(2)])]);
    expect(boundCards(s, { host: "GPU-1" }).map((c) => c.index)).toEqual([0, 1, 2]);
    expect(boundCards(s, { host: "gpu-1", indices: [2, 5] }).map((c) => c.index)).toEqual([2]);
    expect(boundCards(s, { host: "gpu-9" })).toEqual([]);
    expect(boundCards(undefined, { host: "gpu-1" })).toEqual([]);
  });

  it("«GPU по хостам» в обзоре инсталляции открывает вкладку GPU", async () => {
    vi.mocked(api.readGpuStatus).mockResolvedValue(status(1, [host("gpu-1", [card(0, { values: { util: 50, memUsed: 40960, memTotal: 81920, temp: 61 } })])]));
    const user = userEvent.setup();
    renderWithClient(<GpuHostsBlock profile={profile()} />);
    const block = screen.getByRole("region", { name: "GPU по хостам" });
    const row = (await within(block).findByRole("button", { name: "gpu-1" })).closest("tr") as HTMLElement;
    expect(within(row).getByText("50 %")).toBeInTheDocument();
    expect(within(row).getByText("40 / 80 ГиБ")).toBeInTheDocument();
    expect(within(row).getByText("61 °C")).toBeInTheDocument();
    await user.click(within(row).getByRole("button", { name: "gpu-1" }));
    expect(useWorkspace.getState().activeId).toBe("gpu:i1");
  });

  it("«GPU сервера»: привязанные карты и значки; без привязки — подсказка без опроса", async () => {
    vi.mocked(api.readGpuStatus).mockResolvedValue(status(1, [host("gpu-1", [card(0), card(1, { values: { util: 12, xid: 79 } })])]));
    const p = profile();
    renderWithClient(<ServerGpuBlock profile={p} server={{ ...p.servers[0], gpus: { host: "gpu-1", indices: [1] } }} />);
    const block = screen.getByRole("region", { name: "GPU сервера" });
    expect(await within(block).findByText("GPU 1")).toBeInTheDocument();
    expect(within(block).queryByText("GPU 0")).toBeNull();
    expect(within(block).getByText(/утилизация 12 %/)).toBeInTheDocument();
    expect(within(block).getByText(/XID 79/)).toBeInTheDocument();
    expect(within(block).getByRole("button", { name: "Открыть GPU" })).toBeInTheDocument();

    vi.mocked(api.readGpuStatus).mockClear();
    renderWithClient(<ServerGpuBlock profile={p} server={p.servers[0]} />);
    expect(screen.getByText("Привяжите GPU в форме сервера")).toBeInTheDocument();
    expect(api.readGpuStatus).not.toHaveBeenCalled();
  });
});
````

<!-- block: patch t11-src-components-installations-installationpanel-test-tsx step=11.1 -->
````
diff --git a/src/components/installations/InstallationPanel.test.tsx b/src/components/installations/InstallationPanel.test.tsx
index e6db605..71c5eb0 100644
--- a/src/components/installations/InstallationPanel.test.tsx
+++ b/src/components/installations/InstallationPanel.test.tsx
@@ -5,7 +5,7 @@ import { renderWithClient } from "../../test/render";
 
 vi.mock("../../api/client", async (importOriginal) => {
   const orig = await importOriginal<typeof import("../../api/client")>();
-  return { ...orig, api: { listInstallations: vi.fn(), connectInstallation: vi.fn(), readServerMetrics: vi.fn() } };
+  return { ...orig, api: { listInstallations: vi.fn(), connectInstallation: vi.fn(), readServerMetrics: vi.fn(), readGpuStatus: vi.fn() } };
 });
 
 import { api } from "../../api/client";
@@ -75,6 +75,22 @@ describe("InstallationPanel", () => {
     expect(api.connectInstallation).toHaveBeenCalledWith("i1", false);
   });
 
+  it("с компонентом GPU — блок «GPU по хостам», без него — нет", async () => {
+    vi.mocked(api.listInstallations).mockResolvedValue([{ ...PROFILE, gpu: { mode: "prometheus" } }]);
+    vi.mocked(api.readGpuStatus).mockResolvedValue({ at: 1, hosts: [{ host: "gpu-node-1", cards: [{ index: 0, values: { util: 10 } }] }], errors: [], missing: [], deltaSec: null });
+    const { unmount } = renderWithClient(<InstallationPanel installationId="i1" />);
+    const block = await screen.findByRole("region", { name: "GPU по хостам" });
+    expect(await within(block).findByRole("button", { name: "gpu-node-1" })).toBeInTheDocument();
+    unmount();
+
+    vi.mocked(api.listInstallations).mockResolvedValue([PROFILE]);
+    vi.mocked(api.readGpuStatus).mockClear();
+    renderWithClient(<InstallationPanel installationId="i1" />);
+    await screen.findByRole("region", { name: "gpu-1" });
+    expect(screen.queryByRole("region", { name: "GPU по хостам" })).toBeNull();
+    expect(api.readGpuStatus).not.toHaveBeenCalled();
+  });
+
   it("предупреждения и тип, выбранный вручную", async () => {
     vi.mocked(api.connectInstallation).mockResolvedValue({
       installationId: "i1",
````

<!-- block: patch t11-src-components-servers-serverpanel-test-tsx step=11.1 -->
````
diff --git a/src/components/servers/ServerPanel.test.tsx b/src/components/servers/ServerPanel.test.tsx
index 11a4cb7..98f6aa6 100644
--- a/src/components/servers/ServerPanel.test.tsx
+++ b/src/components/servers/ServerPanel.test.tsx
@@ -90,6 +90,13 @@ describe("ServerPanel", () => {
     expect(api.readServerMetrics).toHaveBeenCalledWith("i1", "s1");
   });
 
+  it("«Обзор» с компонентом GPU — блок «GPU сервера»: без привязки подсказка", async () => {
+    vi.mocked(api.listInstallations).mockResolvedValue([{ ...PROFILE, gpu: { mode: "prometheus" } }]);
+    renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
+    const block = await screen.findByRole("region", { name: "GPU сервера" });
+    expect(within(block).getByText("Привяжите GPU в форме сервера")).toBeInTheDocument();
+  });
+
   it("«Конфигурация»: источник, расхождение источников и скрытый секрет", async () => {
     const user = userEvent.setup();
     renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
````

- [ ] **Шаг 11.2: Увидеть падение.**

Run: `pnpm exec vitest run src/components/gpu/GpuSummary.test.tsx src/components/installations/InstallationPanel.test.tsx src/components/servers/ServerPanel.test.tsx`
Expected: FAIL — нет `./GpuSummary`, блоков «GPU по хостам» и «GPU сервера» нет

- [ ] **Шаг 11.3: Реализация.** Блоки, подключение к панелям, стили, словари.

<!-- block: patch t11-src-app-css step=11.3 -->
````
diff --git a/src/app.css b/src/app.css
index befcc5e..711e14f 100644
--- a/src/app.css
+++ b/src/app.css
@@ -53,3 +53,8 @@ input::placeholder, textarea::placeholder { font-style: italic; color: var(--mut
 .gpu-bar { display: inline-block; width: 60px; height: 6px; border-radius: 3px; background: var(--bg-alt); vertical-align: middle; overflow: hidden; }
 .gpu-bar > span { display: block; height: 100%; background: var(--accent); }
 .gpu-alerts li { font-size: 12px; }
+.gpu-summary { border-collapse: collapse; width: 100%; }
+.gpu-summary th { text-align: left; color: var(--muted); font-weight: normal; padding: 2px 8px 2px 0; }
+.gpu-summary td { padding: 2px 8px 2px 0; font-variant-numeric: tabular-nums; }
+.gpu-server { display: flex; flex-direction: column; gap: 4px; margin-top: 8px; }
+.gpu-server h4 { margin: 0; font-size: 13px; }
````

<!-- block: file t11-src-components-gpu-gpusummary-tsx step=11.3 path=src/components/gpu/GpuSummary.tsx -->
````
import { useTranslation } from "react-i18next";
import type { InstallationProfile, ServerConfig } from "../../api/types";
import { tr } from "../../i18n/text";
import { useWorkspace } from "../../state/workspace";
import { fmtNumber } from "../servers/format";
import { cardAlerts } from "./alerts";
import { boundCards, hostSummary } from "./summary";
import { useGpuStatus } from "./useGpu";

/** Блок «GPU по хостам» «Обзора» инсталляции (спека I1d §5): строка на хост; щелчок открывает вкладку GPU. */
export function GpuHostsBlock({ profile }: { profile: InstallationProfile }) {
  const { t } = useTranslation();
  const open = useWorkspace((s) => s.open);
  const live = useGpuStatus(profile.id, true);
  const openGpu = () => open({ kind: "gpu", installationId: profile.id });
  const pct = (v: number | null) => (v == null ? "—" : t("gpu.units.percent", { value: fmtNumber(v) }));
  return (
    <section className="card" aria-label={t("gpu.overview.title")}>
      <h4>
        <button type="button" className="link" onClick={openGpu}>
          {t("gpu.overview.title")}
        </button>
      </h4>
      {live.error && <p className="srv-error">{tr(live.error)}</p>}
      {!live.status && !live.error && <p className="muted">{t("common.loading")}</p>}
      {live.status && live.status.hosts.length === 0 && <p className="muted">{t("gpu.panel.noCards")}</p>}
      {live.status && live.status.hosts.length > 0 && (
        <table className="gpu-summary">
          <thead>
            <tr>
              <th>{t("gpu.overview.host")}</th>
              <th>{t("gpu.overview.model")}</th>
              <th>{t("gpu.overview.cards")}</th>
              <th>{t("gpu.overview.util")}</th>
              <th>{t("gpu.overview.memory")}</th>
              <th>{t("gpu.overview.temp")}</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {live.status.hosts.map(hostSummary).map((s) => (
              <tr key={s.host}>
                <td>
                  <button type="button" className="link" onClick={openGpu}>
                    {s.host}
                  </button>
                </td>
                <td>{s.model ?? t("gpu.overview.mixed")}</td>
                <td>{s.count}</td>
                <td>{pct(s.avgUtil)}</td>
                <td>
                  {s.memUsed == null || s.memTotal == null
                    ? "—"
                    : t("gpu.card.memoryValue", { used: fmtNumber(s.memUsed / 1024), total: fmtNumber(s.memTotal / 1024) })}
                </td>
                <td>{s.maxTemp == null ? "—" : t("gpu.units.celsius", { value: fmtNumber(s.maxTemp) })}</td>
                <td>{s.troubled > 0 && <span className="warn">⚠ {t("gpu.overview.troubled", { count: s.troubled })}</span>}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}

/**
 * Блок «GPU сервера» «Обзора» сервера (спека I1d §5): строка на привязанную карту и ссылка на вкладку GPU. Без привязки —
 * одна подсказка, опроса нет.
 */
export function ServerGpuBlock({ profile, server }: { profile: InstallationProfile; server: ServerConfig }) {
  const { t } = useTranslation();
  const open = useWorkspace((s) => s.open);
  const binding = server.gpus;
  const live = useGpuStatus(profile.id, !!binding);
  const cards = binding ? boundCards(live.status, binding) : [];
  return (
    <section className="gpu-server" aria-label={t("gpu.overview.server")}>
      <h4>{t("gpu.overview.server")}</h4>
      {!binding && <p className="muted">{t("gpu.overview.bindHint")}</p>}
      {binding && live.error && <p className="srv-error">{tr(live.error)}</p>}
      {binding && live.status && cards.length === 0 && <p className="muted">{t("gpu.overview.noBoundCards", { host: binding.host })}</p>}
      {cards.length > 0 && (
        <ul className="steps">
          {cards.map((c) => {
            const v = c.values;
            const alerts = cardAlerts(c);
            return (
              <li key={c.index}>
                <strong>{t("gpu.card.title", { index: c.index })}</strong>{" "}
                <span>
                  {[
                    v.util == null ? null : t("gpu.overview.utilValue", { value: fmtNumber(v.util) }),
                    v.memUsed == null || v.memTotal == null
                      ? null
                      : t("gpu.card.memoryValue", { used: fmtNumber(v.memUsed / 1024, 1), total: fmtNumber(v.memTotal / 1024, 1) }),
                    v.temp == null ? null : t("gpu.units.celsius", { value: fmtNumber(v.temp) }),
                  ]
                    .filter(Boolean)
                    .join(" · ") || t("gpu.card.noData")}
                </span>
                {alerts.map((a) => (
                  <span key={a.kind} className={a.severity === "danger" ? "srv-error" : "warn"} title={t(`gpu.alerts.${a.kind}Hint`, a.params)}>
                    {" "}
                    ⚠ {t(`gpu.alerts.${a.kind}`, a.params)}
                  </span>
                ))}
              </li>
            );
          })}
        </ul>
      )}
      {binding && (
        <button type="button" className="link" onClick={() => open({ kind: "gpu", installationId: profile.id })}>
          {t("gpu.overview.openGpu")}
        </button>
      )}
    </section>
  );
}
````

<!-- block: file t11-src-components-gpu-summary-ts step=11.3 path=src/components/gpu/summary.ts -->
````
import type { GpuBinding, GpuCard, GpuHost, GpuStatus } from "../../api/types";
import { cardAlerts } from "./alerts";

/** Строка хоста в «Обзоре» инсталляции (спека I1d §5). */
export interface HostSummary {
  host: string;
  /** Модель карт; у разных моделей — `null`. */
  model: string | null;
  count: number;
  avgUtil: number | null;
  memUsed: number | null;
  memTotal: number | null;
  maxTemp: number | null;
  /** Карт со значками проблем. */
  troubled: number;
}

const values = (cards: GpuCard[], f: (c: GpuCard) => number | undefined) =>
  cards.map(f).filter((v): v is number => typeof v === "number");

export function hostSummary(h: GpuHost): HostSummary {
  const models = new Set(h.cards.map((c) => c.model ?? ""));
  const util = values(h.cards, (c) => c.values.util);
  const used = values(h.cards, (c) => c.values.memUsed);
  const total = values(h.cards, (c) => c.values.memTotal);
  const temp = values(h.cards, (c) => c.values.temp);
  return {
    host: h.host,
    model: models.size === 1 ? [...models][0] || null : null,
    count: h.cards.length,
    avgUtil: util.length ? util.reduce((a, b) => a + b, 0) / util.length : null,
    memUsed: used.length ? used.reduce((a, b) => a + b, 0) : null,
    memTotal: total.length ? total.reduce((a, b) => a + b, 0) : null,
    maxTemp: temp.length ? Math.max(...temp) : null,
    troubled: h.cards.filter((c) => cardAlerts(c).length > 0).length,
  };
}

/** Карты привязки сервера в снимке: хост без учёта регистра, все карты или перечисленные. */
export function boundCards(status: GpuStatus | undefined, b: GpuBinding): GpuCard[] {
  const h = status?.hosts.find((x) => x.host.toLowerCase() === b.host.toLowerCase());
  if (!h) return [];
  return b.indices ? h.cards.filter((c) => b.indices?.includes(c.index)) : h.cards;
}
````

<!-- block: patch t11-src-components-installations-installationpanel-tsx step=11.3 -->
````
diff --git a/src/components/installations/InstallationPanel.tsx b/src/components/installations/InstallationPanel.tsx
index 6dc15ca..fa02ed9 100644
--- a/src/components/installations/InstallationPanel.tsx
+++ b/src/components/installations/InstallationPanel.tsx
@@ -7,6 +7,7 @@ import { useWorkspace } from "../../state/workspace";
 import { EnvBadge } from "../common/EnvBadge";
 import { ErrorBanner } from "../common/ErrorBanner";
 import { Hint } from "../common/Hint";
+import { GpuHostsBlock } from "../gpu/GpuSummary";
 import { fmtNumber, fmtPercent } from "../servers/format";
 import { noMetricsHintKey, readsMetrics, useServerMetrics } from "../servers/useServerMetrics";
 import { selectorLabel } from "./HistoryFields";
@@ -69,6 +70,7 @@ export function InstallationPanel({ installationId }: { installationId: string }
           ))}
         </div>
       )}
+      {profile.gpu && <GpuHostsBlock profile={profile} />}
     </div>
   );
 }
````

<!-- block: patch t11-src-components-servers-serverpanel-tsx step=11.3 -->
````
diff --git a/src/components/servers/ServerPanel.tsx b/src/components/servers/ServerPanel.tsx
index 95332c1..89b3eeb 100644
--- a/src/components/servers/ServerPanel.tsx
+++ b/src/components/servers/ServerPanel.tsx
@@ -6,6 +6,7 @@ import { tr } from "../../i18n/text";
 import { useWorkspace } from "../../state/workspace";
 import { EnvBadge } from "../common/EnvBadge";
 import { ErrorBanner } from "../common/ErrorBanner";
+import { ServerGpuBlock } from "../gpu/GpuSummary";
 import { installationQueryKey } from "../installations/InstallationPanel";
 import { ServerConfigView } from "./ServerConfigView";
 import { ServerMetricsView } from "./ServerMetricsView";
@@ -91,7 +92,10 @@ export function ServerPanel({ installationId, serverId }: { installationId: stri
             ))}
           </div>
           {sub === "overview" && (
-            <ServerOverview facts={state.facts} latest={live.error ? undefined : live.latest} restarted={restarted} error={!!live.error} fromHistory={live.fromHistory} />
+            <>
+              <ServerOverview facts={state.facts} latest={live.error ? undefined : live.latest} restarted={restarted} error={!!live.error} fromHistory={live.fromHistory} />
+              {profile.gpu && <ServerGpuBlock profile={profile} server={server} />}
+            </>
           )}
           {sub === "config" && <ServerConfigView installationId={installationId} serverId={serverId} />}
           {sub === "metrics" && (
````

<!-- block: locales t11-locales-t11 step=11.3 -->
````
{
  "en": {
    "gpu": { "overview": {
      "title": "GPU by host",
      "host": "Host",
      "model": "Model",
      "cards": "GPUs",
      "util": "Utilization",
      "memory": "Memory",
      "temp": "Max temperature",
      "mixed": "mixed",
      "troubled": "problems: {{count}}",
      "server": "GPUs of the server",
      "bindHint": "Bind the GPUs in the server form",
      "noBoundCards": "Host {{host}} has none of the bound GPUs",
      "utilValue": "utilization {{value}} %",
      "openGpu": "Open GPU"
    } }
  },
  "ru": {
    "gpu": { "overview": {
      "title": "GPU по хостам",
      "host": "Хост",
      "model": "Модель",
      "cards": "Карт",
      "util": "Утилизация",
      "memory": "Память",
      "temp": "Макс. температура",
      "mixed": "разные",
      "troubled": "проблемы: {{count}}",
      "server": "GPU сервера",
      "bindHint": "Привяжите GPU в форме сервера",
      "noBoundCards": "На хосте {{host}} нет привязанных карт",
      "utilValue": "утилизация {{value}} %",
      "openGpu": "Открыть GPU"
    } }
  }
}
````

- [ ] **Шаг 11.4: Проверки.**

Run: `pnpm test`
Expected: `Test Files  40 passed`, `Tests  288 passed`
Run: `pnpm exec tsc --noEmit`
Expected: без вывода
Run: `pnpm i18n:check`
Expected: `✓ словари en, ru согласованы, вызовов из Rust: 187`

- [ ] **Шаг 11.5: Коммит.**

<!-- block: shell t11-commit step=11.5 -->
````
git add src/app.css \
  src/components/gpu/GpuSummary.test.tsx \
  src/components/gpu/GpuSummary.tsx \
  src/components/gpu/summary.ts \
  src/components/installations/InstallationPanel.test.tsx \
  src/components/installations/InstallationPanel.tsx \
  src/components/servers/ServerPanel.test.tsx \
  src/components/servers/ServerPanel.tsx \
  src/locales/en.json \
  src/locales/ru.json
git commit -s -q -F - <<'EOF'
feat(gpu): GPU blocks in the installation and server overviews

The installation overview gets GPU by host: model or mixed, card count,
average utilization, memory, maximum temperature and the number of GPUs
with problems; a click opens the GPU tab. The server overview gets the
GPUs of the server: bound cards with utilization, memory, temperature
and badges, and Open GPU; without a binding one hint and no polling.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
````

---

### Задача 12: Глава «GPU», тексты этапа, смоук

Глава руководства `gpu` (en, ru; спека I1d §5): источник карт, привязка серверов, вкладка и значки, какие
счётчики включить, что делать, если не работает, пример конфигурации Prometheus; на неё ссылаются «Серверы», «Начало
работы», F1 на вкладке GPU и подсказка о неполном наборе счётчиков. README, «Начало работы» и `about.summary` называют
этап I1d. Смоук `docs/smoke/i1d.md` — с моками DCGM и настоящим Prometheus. Спека I1d приводится к коду (метка
`Hostname` — до 4.1, привязку без поля `auto` помнит форма, шаблоны — в `gpu/promql.rs` с подзапросом ошибок NVLink,
совпадение коротких имён хостов, мок `dcgm-4.1` и `--pod`, новые ошибки), общая спека получает изменения §5.3, §5.7,
§5.8, §6.2–6.4, §7.2 и §20.

**Files:**
- Create: `docs/guide/en/gpu.md`
- Create: `docs/guide/ru/gpu.md`
- Create: `docs/smoke/i1d.md`
- Modify: `README.md`
- Modify: `docs/guide/en/getting-started.md`
- Modify: `docs/guide/en/servers.md`
- Modify: `docs/guide/ru/getting-started.md`
- Modify: `docs/guide/ru/servers.md`
- Modify: `docs/superpowers/specs/2026-10-29-glafira-inference-design.md`
- Modify: `docs/superpowers/specs/2026-11-01-glafira-inference-i1d-design.md`
- Modify: `scripts/i18n-check.mjs`
- Modify: `src/components/gpu/GpuPanel.test.tsx`
- Modify: `src/components/gpu/GpuPanel.tsx`
- Modify: `src/components/help/helpChapterFor.test.ts`
- Modify: `src/components/help/helpChapterFor.ts`
- Modify: `src/guide/chapters.test.ts`
- Modify: `src/guide/chapters.ts`
- Modify: `src/locales/en.json`
- Modify: `src/locales/ru.json`

**Interfaces:**
- Produces: `GUIDE_CHAPTERS` с `gpu` (`src/guide/chapters.ts` и `scripts/i18n-check.mjs`); `helpChapterFor({ kind:
  "gpu" })` → `gpu`; ключ `gpu.panel.missingLink`.

- [ ] **Шаг 12.1: Тесты.** Тесты: у ru есть глава «GPU», F1 на вкладке GPU — глава `gpu`, ссылка из подсказки о счётчиках.

<!-- block: patch t12-src-components-gpu-gpupanel-test-tsx step=12.1 -->
````
diff --git a/src/components/gpu/GpuPanel.test.tsx b/src/components/gpu/GpuPanel.test.tsx
index c19030c..0b5abd2 100644
--- a/src/components/gpu/GpuPanel.test.tsx
+++ b/src/components/gpu/GpuPanel.test.tsx
@@ -22,6 +22,7 @@ globalThis.ResizeObserver ??= class {
 } as unknown as typeof ResizeObserver;
 
 import { api } from "../../api/client";
+import { useWorkspace } from "../../state/workspace";
 import { boundServers, GpuPanel, gpuHistoryUnavailable } from "./GpuPanel";
 import { card, host, status } from "./testGpu";
 
@@ -80,6 +81,9 @@ describe("GpuPanel", () => {
     renderWithClient(<GpuPanel installationId="i1" />);
     expect(await screen.findByText(/Экспортёр http:\/\/gpu-2:9400\/metrics:/)).toBeInTheDocument();
     expect(screen.getByText(/Экспортёр не собирает: активность SM, ошибки ECC/)).toBeInTheDocument();
+    await userEvent.setup().click(screen.getByRole("button", { name: "Какие счётчики включить" }));
+    const tabs = useWorkspace.getState().tabs;
+    expect(tabs[tabs.length - 1]).toMatchObject({ kind: "help", chapter: "gpu", anchor: "counters" });
 
     vi.mocked(api.readGpuStatus).mockRejectedValue({ code: "CONNECTION_FAILED", key: "errors.server.serverError", params: { path: "/metrics", status: "503" }, message: "x", retryable: true });
     vi.mocked(api.listInstallations).mockResolvedValue([profile({ id: "i2" })]);
````

<!-- block: patch t12-src-components-help-helpchapterfor-test-ts step=12.1 -->
````
diff --git a/src/components/help/helpChapterFor.test.ts b/src/components/help/helpChapterFor.test.ts
index 5f22a20..1677aa2 100644
--- a/src/components/help/helpChapterFor.test.ts
+++ b/src/components/help/helpChapterFor.test.ts
@@ -6,6 +6,7 @@ const cases: [TabSpec | null, string][] = [
   [{ kind: "installation-form", installationId: null }, "installations"],
   [{ kind: "installation", installationId: "i" }, "installations"],
   [{ kind: "server", installationId: "i", serverId: "s" }, "servers"],
+  [{ kind: "gpu", installationId: "i" }, "gpu"],
   [{ kind: "about" }, "updates"],
   [{ kind: "settings" }, "getting-started"],
   [{ kind: "help", chapter: "installations" }, "getting-started"],
````

<!-- block: patch t12-src-guide-chapters-test-ts step=12.1 -->
````
diff --git a/src/guide/chapters.test.ts b/src/guide/chapters.test.ts
index cf4fae5..70222dc 100644
--- a/src/guide/chapters.test.ts
+++ b/src/guide/chapters.test.ts
@@ -7,6 +7,7 @@ describe("loadGuide", () => {
     expect(Object.keys(ru).sort()).toEqual([...GUIDE_CHAPTERS].sort());
     expect(ru.installations).toMatch(/^# Инсталляции/);
     expect(ru.history).toMatch(/^# История метрик/);
+    expect(ru.gpu).toMatch(/^# GPU/);
   });
 
   it("falls back to English for a language without chapters", async () => {
````

- [ ] **Шаг 12.2: Увидеть падение.**

Run: `pnpm exec vitest run src/guide src/components/help src/components/gpu/GpuPanel.test.tsx`
Expected: FAIL — `ru.gpu` нет, `helpChapterFor` даёт `getting-started`, ссылки «Какие счётчики включить» нет

- [ ] **Шаг 12.3: Реализация.** Глава на двух языках, список глав, ссылки, тексты этапа (`about.summary` и `gpu.panel.missing` заменяются — `replace=1`), смоук, спеки.

<!-- block: patch t12-readme-md step=12.3 -->
````
diff --git a/README.md b/README.md
index a3d30d7..6a23b2a 100644
--- a/README.md
+++ b/README.md
@@ -3,11 +3,12 @@
 Desktop-приложение (Tauri 2 + Rust + React) для подключения к корпоративному инференсу open-source LLM, его
 мониторинга, диагностики и аудита; в фокусе — vLLM и SGLang на GPU NVIDIA.
 
-Состояние: готовы этапы I1a, I1b и I1c — подключение, проверка, метрики сервера и их история. Это инсталляции из
-серверов, определение типа, версии и моделей сервера, пошаговая проверка сервера, папки, экспорт и импорт, вкладка
-сервера: плитки и графики метрик «сейчас» (опрос `/metrics` раз в 5 с) и аргументы запуска с источниками, — и история
-метрик из Prometheus или VictoriaMetrics: окна 1 ч / 24 ч / 7 д / 30 д и «Сейчас» с последних 15 минут. GPU,
-диагностика и аудит появятся в следующих этапах, I1d–I7 (спецификация, §20).
+Состояние: готовы этапы I1a, I1b, I1c и I1d — подключение, проверка, метрики сервера, их история и GPU. Это инсталляции
+из серверов, определение типа, версии и моделей сервера, пошаговая проверка сервера, папки, экспорт и импорт, вкладка
+сервера: плитки и графики метрик «сейчас» (опрос `/metrics` раз в 5 с) и аргументы запуска с источниками, — история
+метрик из Prometheus или VictoriaMetrics: окна 1 ч / 24 ч / 7 д / 30 д и «Сейчас» с последних 15 минут, — и карты NVIDIA
+по метрикам DCGM: экспортёры или источник истории, привязка серверов к картам, вкладка GPU и блоки GPU в обзорах.
+Диагностика и аудит появятся в следующих этапах, I2–I7 (спецификация, §20).
 
 vLLM, SGLang, LiteLLM, NVIDIA and other product names are trademarks of their respective owners. This project is not
 affiliated with them.
````

<!-- block: patch t12-docs-guide-en-getting-started-md step=12.3 -->
````
diff --git a/docs/guide/en/getting-started.md b/docs/guide/en/getting-started.md
index 5259b1f..47cbc30 100644
--- a/docs/guide/en/getting-started.md
+++ b/docs/guide/en/getting-started.md
@@ -1,10 +1,10 @@
 # Getting started
 
 Glafira for LLM Inference is a desktop app for connecting to corporate open-source LLM inference, monitoring,
-diagnosing and auditing it, with vLLM and SGLang in focus. This version (stage I1c) covers connecting and checking:
+diagnosing and auditing it, with vLLM and SGLang in focus. This version (stage I1d) covers connecting and checking:
 installations of servers, the kind, version and models of each server, a step-by-step server check, live server
-metrics (tiles, charts and launch arguments) and metric history from Prometheus or VictoriaMetrics. GPUs,
-diagnostics and audit come in the next stages.
+metrics (tiles, charts and launch arguments), metric history from Prometheus or VictoriaMetrics and NVIDIA GPUs from
+DCGM metrics (see [GPU](gpu.md)). Diagnostics and audit come in the next stages.
 Glafira changes nothing on the servers: it reads their answers with your API keys. Glafira never reads or shows the texts of your users' requests and answers.
 
 ## Installation {#install}
````

<!-- block: file t12-docs-guide-en-gpu-md step=12.3 path=docs/guide/en/gpu.md -->
````
# GPU

Glafira shows the NVIDIA GPUs of an installation from DCGM metrics: utilization, SM activity, memory, temperature,
power, clocks and signs of trouble — throttling, XID errors, ECC, NVLink errors and PCIe replays. The metrics come from
[dcgm-exporter](https://github.com/NVIDIA/dcgm-exporter); Glafira only reads them and writes nothing to disk.

## GPU source {#source}

In the installation form, section [[ui:installations.gpu.title]], choose the source:

- **[[ui:installations.gpu.modes.direct]]** — Glafira reads `/metrics` of the exporters itself. Enter the addresses,
  such as `http://gpu-node-1:9400/metrics` (without a path `/metrics` is used), up to 16 exporters. Sign-in — none or a
  user name and password (the exporter `web-config.yml`); the password is shared by all exporters and is kept only in the
  system secret store. CA and proxy — as for servers.
- **[[ui:installations.gpu.modes.prometheus]]** — the same DCGM series from the Prometheus or VictoriaMetrics of the
  installation. Handy when the exporters are not reachable from the workstation. The
  [[ui:installations.gpu.selector]] picks the GPUs of the installation, such as `{hostname=~"gpu-node-1|gpu-node-2"}`
  or `{cluster="llm"}`; the check fills an empty selector from the hosts of the servers. An installation can have at most
  64 GPUs.

Click [[ui:installations.gpu.check]]: the check goes through every exporter (DNS/TCP, `/metrics`, GPUs) or finds the
GPUs in the history source, then binds the servers and matches the exporters with the history. A GPU failure does not
stop the servers from connecting.

## Binding servers {#binding}

Every server in the form has the [[ui:installations.gpu.binding]] field: a GPU host and all or some of its cards. With
the binding the server overview shows its GPUs, and the rules of the next stages relate the server queue to the GPU load.

- The check binds a server itself when the host of its address matches a GPU host: names ignore case, a short node name
  (`gpu-node-1`) matches the full one (`gpu-node-1.corp.local`), `localhost` and `127.0.0.1` are one address. The check
  does not overwrite a choice made by a person; if the server address changes after the auto-match, the binding resets.
- Behind a load balancer or a Kubernetes Service the server address is not a node address: choose the host manually.
- If the server takes some of the cards (say 4 of 8), clear [[ui:installations.gpu.allCards]] and tick its cards.

The host of a GPU is the `hostname` label of the exporter (since dcgm-exporter 4.8; `Hostname` in 4.1 and earlier), and
without it the host of the exporter address or the `instance` label.

## GPU tab {#tab}

In the tree an installation with a GPU component has a “GPU” item. The tab shows the hosts (driver, GPU count, bound
servers) and GPU cards. A field without a series reads “no data”, not zero. Badges:

| Badge | Meaning |
|---|---|
| Throttling | Clocks are reduced by power or temperature (`SW_POWER_CAP`, `HW_SLOWDOWN`, `SW_THERMAL`, `HW_THERMAL`, `HW_POWER_BRAKE`) or by the throttle time counters |
| XID | The last XID error of the driver; stays until the driver reloads |
| ECC | Uncorrectable ECC errors or remapped memory rows — a reason to replace the card |
| ECC +N | Correctable ECC errors in the last minute |
| NVLink +N, PCIe +N | NVLink errors and PCIe replays in the last minute |

“Workload” is the pod and namespace from the Kubernetes labels of the exporter. GPUs in MIG mode list their instances;
Glafira does not show instance metrics yet.

A click on a card opens its charts (utilization and SM, memory, GPU and memory temperature, power, clocks), a click on a
host — the charts of all its cards, a series per card. The “Now” window polls every 5 s while the tab is visible and
keeps up to an hour of points in memory; the 1 h, 24 h, 7 d and 30 d windows come from the history source if it has the
DCGM series. In [[ui:installations.gpu.modes.direct]] mode every exporter in the form has the
[[ui:installations.gpu.exporterSeries]] field: the Prometheus target with the same `instance`, which the check picks
itself.

The installation overview shows GPU by host, the server overview — the GPUs of its binding.

## Counters to enable {#counters}

The default counter set of dcgm-exporter (`default-counters.csv`) does not collect some of what Glafira needs. Without
them the cards work, but some fields read “no data”, and the check tells what is missing. Add to the exporter counter
file (`-f`, `--collectors`):

```
DCGM_FI_PROF_SM_ACTIVE,          gauge,   The ratio of cycles an SM has at least 1 warp assigned.
DCGM_FI_DEV_ECC_SBE_VOL_TOTAL,   counter, Total number of single-bit volatile ECC errors.
DCGM_FI_DEV_ECC_DBE_VOL_TOTAL,   counter, Total number of double-bit volatile ECC errors.
DCGM_FI_DEV_CLOCKS_EVENT_REASONS, gauge,  Current clock event reasons.
DCGM_FI_DEV_POWER_VIOLATION,     counter, Throttling duration due to power constraints (in ns).
DCGM_FI_DEV_THERMAL_VIOLATION,   counter, Throttling duration due to thermal constraints (in ns).
```

NVLink errors are `DCGM_FI_DEV_NVLINK_CRC_ERROR_TOTAL`, `DCGM_FI_DEV_NVLINK_RECOVERY_TOTAL`,
`DCGM_FI_DEV_NVLINK_REPLAY_TOTAL` (Hopper) or `DCGM_FI_DEV_NVLINK_*_ERROR_TOTAL` (earlier). Without SM activity Glafira
shows the graphics engine activity (`DCGM_FI_PROF_GR_ENGINE_ACTIVE`) marked “by GR engine”.

## Troubleshooting {#troubleshooting}

- “There are no DCGM metrics” — the address answers, but it is not dcgm-exporter: check the port (9400 by default) and
  the path.
- “The DCGM exporter rejected the request” — wrong user name or password.
- “Found more than 64 GPUs” — narrow the card selector or remove exporters.
- “The hosts of the servers are not among the GPU hosts” — choose the GPU hosts in the form or bind the servers
  manually.
- History windows are disabled — the installation has no history source or the exporters are not matched with its
  targets.

## Prometheus configuration example {#prometheus-config}

```yaml
scrape_configs:
  - job_name: dcgm
    static_configs:
      - targets: ["gpu-node-1:9400", "gpu-node-2:9400"]
```

In Kubernetes dcgm-exporter usually comes with the GPU Operator and is scraped through a ServiceMonitor; the exporter
adds the `pod` and `namespace` labels itself.
````

<!-- block: patch t12-docs-guide-en-servers-md step=12.3 -->
````
diff --git a/docs/guide/en/servers.md b/docs/guide/en/servers.md
index b3a5522..05c863a 100644
--- a/docs/guide/en/servers.md
+++ b/docs/guide/en/servers.md
@@ -22,6 +22,10 @@ shows their number. The tiles show the current state:
 "No data" means the server does not expose the series (the tile's tooltip names it) or there were no requests in the
 interval. Glafira never shows zero in place of an unknown value.
 
+If the installation has a GPU component, the [[ui:gpu.overview.server]] block below the tiles shows the GPUs the server
+is bound to with utilization, memory, temperature and problem badges. The binding is set in the installation form — see
+[GPU](gpu.md#binding).
+
 ## Percentiles {#percentiles}
 
 Percentiles come from the server's histograms, the same way as `histogram_quantile` in Prometheus, with linear
````

<!-- block: patch t12-docs-guide-ru-getting-started-md step=12.3 -->
````
diff --git a/docs/guide/ru/getting-started.md b/docs/guide/ru/getting-started.md
index 3c2ab2b..065743b 100644
--- a/docs/guide/ru/getting-started.md
+++ b/docs/guide/ru/getting-started.md
@@ -1,10 +1,10 @@
 # Начало работы
 
 Glafira for LLM Inference — настольное приложение для подключения к корпоративному инференсу open-source LLM, его
-мониторинга, диагностики и аудита; в фокусе — vLLM и SGLang. В этой версии (этап I1c) готовы подключение и проверка:
+мониторинга, диагностики и аудита; в фокусе — vLLM и SGLang. В этой версии (этап I1d) готовы подключение и проверка:
 инсталляции из серверов, тип, версия и модели каждого сервера, пошаговая проверка сервера — метрики сервера «сейчас»:
-плитки, графики и аргументы запуска — и история метрик из Prometheus или VictoriaMetrics. GPU, диагностика и аудит
-появятся в следующих этапах. Glafira
+плитки, графики и аргументы запуска, — история метрик из Prometheus или VictoriaMetrics и карты NVIDIA по метрикам DCGM
+(глава [GPU](gpu.md)). Диагностика и аудит появятся в следующих этапах. Glafira
 ничего не меняет в серверах: она читает их ответы с вашими ключами API. Тексты запросов и ответов ваших пользователей Glafira не читает и не показывает.
 
 ## Установка {#install}
````

<!-- block: file t12-docs-guide-ru-gpu-md step=12.3 path=docs/guide/ru/gpu.md -->
````
# GPU

Glafira показывает карты NVIDIA инсталляции по метрикам DCGM: утилизацию, активность SM, память, температуру,
мощность, такты и признаки проблем — троттлинг, XID-ошибки, ECC, ошибки NVLink и повторы PCIe. Метрики отдаёт
[dcgm-exporter](https://github.com/NVIDIA/dcgm-exporter); Glafira только читает их и ничего не пишет на диск.

## Источник карт {#source}

В форме инсталляции, в разделе [[ui:installations.gpu.title]], выберите источник:

- **[[ui:installations.gpu.modes.direct]]** — Glafira сама читает `/metrics` экспортёров. Укажите адреса, например
  `http://gpu-node-1:9400/metrics` (без пути подставляется `/metrics`), до 16 экспортёров. Вход — без входа или имя
  пользователя и пароль (`web-config.yml` экспортёра); пароль общий для всех экспортёров и хранится только в системном
  хранилище секретов. CA и прокси — как у серверов.
- **[[ui:installations.gpu.modes.prometheus]]** — те же серии DCGM из Prometheus или VictoriaMetrics инсталляции.
  Удобно, если экспортёры недоступны с рабочего места. Карты инсталляции выбирает
  [[ui:installations.gpu.selector]], например `{hostname=~"gpu-node-1|gpu-node-2"}` или `{cluster="llm"}`; пустой
  селектор проверка подбирает сама — по хостам серверов. У инсталляции может быть не больше 64 карт.

Нажмите [[ui:installations.gpu.check]]: проверка пройдёт каждый экспортёр (DNS/TCP, `/metrics`, карты) или найдёт
карты в источнике истории, затем привяжет серверы и сопоставит экспортёры с историей. Сбой GPU не мешает подключению
серверов.

## Привязка серверов {#binding}

У каждого сервера в форме есть поле [[ui:installations.gpu.binding]]: хост карт и все или некоторые его карты. По
привязке «Обзор» сервера показывает его карты, а правила следующих этапов связывают очередь сервера с загрузкой карт.

- Проверка привязывает сервер сама, если хост его адреса совпал с хостом карт: имена — без учёта регистра, короткое имя
  узла (`gpu-node-1`) совпадает с полным (`gpu-node-1.corp.local`), `localhost` и `127.0.0.1` — один адрес. Выбор
  человека проверка не перезаписывает; если после автоподбора сменить адрес сервера, привязка сбрасывается.
- За балансировщиком или Service Kubernetes адрес сервера — не адрес узла: выберите хост вручную.
- Если сервер занимает часть карт (например, 4 из 8), снимите [[ui:installations.gpu.allCards]] и отметьте свои.

Хост карты — метка `hostname` экспортёра (с dcgm-exporter 4.8; в 4.1 и раньше — `Hostname`), а без неё — хост адреса
экспортёра или метка `instance`.

## Вкладка GPU {#tab}

В дереве у инсталляции с компонентом GPU есть пункт «GPU». Вкладка показывает хосты (драйвер, число карт, привязанные
серверы) и карточки карт. Поле без серии — «нет данных», а не ноль. Значки:

| Значок | Что значит |
|---|---|
| Троттлинг | Такты снижены по мощности или температуре (`SW_POWER_CAP`, `HW_SLOWDOWN`, `SW_THERMAL`, `HW_THERMAL`, `HW_POWER_BRAKE`) или по счётчикам времени троттлинга |
| XID | Последняя XID-ошибка драйвера; держится до перезагрузки драйвера |
| ECC | Некорректируемые ошибки ECC или переназначенные строки памяти — повод заменить карту |
| ECC +N | Корректируемые ошибки ECC за последнюю минуту |
| NVLink +N, PCIe +N | Ошибки NVLink и повторы PCIe за последнюю минуту |

«Нагрузка» — под и namespace из меток Kubernetes экспортёра. У карт в режиме MIG перечислены экземпляры; метрики
экземпляров Glafira пока не показывает.

Щелчок по карточке открывает графики карты (утилизация и SM, память, температура карты и памяти, мощность, такты),
щелчок по хосту — графики всех его карт, по ряду на карту. Окно «Сейчас» — опрос раз в 5 с, пока вкладка видна, и до
часа точек в памяти; окна 1 ч, 24 ч, 7 д и 30 д — из источника истории, если серии DCGM есть в нём. В режиме
[[ui:installations.gpu.modes.direct]] у каждого экспортёра в форме есть поле [[ui:installations.gpu.exporterSeries]]:
цель Prometheus с тем же `instance`, проверка подбирает её сама.

«Обзор» инсталляции показывает GPU по хостам, «Обзор» сервера — карты его привязки.

## Какие счётчики включить {#counters}

Набор счётчиков dcgm-exporter по умолчанию (`default-counters.csv`) не собирает часть того, что нужно Glafira. Без них
карточки работают, но часть полей — «нет данных», а проверка подсказывает, чего нет. Допишите в файл счётчиков
экспортёра (`-f`, `--collectors`):

```
DCGM_FI_PROF_SM_ACTIVE,          gauge,   The ratio of cycles an SM has at least 1 warp assigned.
DCGM_FI_DEV_ECC_SBE_VOL_TOTAL,   counter, Total number of single-bit volatile ECC errors.
DCGM_FI_DEV_ECC_DBE_VOL_TOTAL,   counter, Total number of double-bit volatile ECC errors.
DCGM_FI_DEV_CLOCKS_EVENT_REASONS, gauge,  Current clock event reasons.
DCGM_FI_DEV_POWER_VIOLATION,     counter, Throttling duration due to power constraints (in ns).
DCGM_FI_DEV_THERMAL_VIOLATION,   counter, Throttling duration due to thermal constraints (in ns).
```

Ошибки NVLink — `DCGM_FI_DEV_NVLINK_CRC_ERROR_TOTAL`, `DCGM_FI_DEV_NVLINK_RECOVERY_TOTAL`,
`DCGM_FI_DEV_NVLINK_REPLAY_TOTAL` (Hopper) или `DCGM_FI_DEV_NVLINK_*_ERROR_TOTAL` (раньше). Без активности SM Glafira
показывает активность graphics engine (`DCGM_FI_PROF_GR_ENGINE_ACTIVE`) с пометкой «по GR engine».

## Если не работает {#troubleshooting}

- «Метрик DCGM нет» — адрес отвечает, но это не dcgm-exporter: проверьте порт (по умолчанию 9400) и путь.
- «Экспортёр DCGM отклонил запрос» — неверные имя пользователя или пароль.
- «Найдено карт больше 64» — сузьте селектор карт или уберите экспортёры.
- «Хостов серверов нет среди хостов GPU» — выберите хосты карт в форме или привяжите серверы вручную.
- Окна истории неактивны — у инсталляции нет источника истории или экспортёры не сопоставлены с его целями.

## Пример конфигурации Prometheus {#prometheus-config}

```yaml
scrape_configs:
  - job_name: dcgm
    static_configs:
      - targets: ["gpu-node-1:9400", "gpu-node-2:9400"]
```

В Kubernetes dcgm-exporter обычно ставится с GPU Operator и собирается через ServiceMonitor; метки `pod` и `namespace`
экспортёр добавляет сам.
````

<!-- block: patch t12-docs-guide-ru-servers-md step=12.3 -->
````
diff --git a/docs/guide/ru/servers.md b/docs/guide/ru/servers.md
index c0183a4..41a8cb0 100644
--- a/docs/guide/ru/servers.md
+++ b/docs/guide/ru/servers.md
@@ -22,6 +22,10 @@
 «Нет данных» — сервер не отдаёт нужную серию (подсказка у плитки называет её) или за интервал не было запросов.
 Glafira не подставляет ноль вместо неизвестного значения.
 
+Если у инсталляции есть компонент GPU, под плитками — блок [[ui:gpu.overview.server]]: карты, к которым привязан
+сервер, с утилизацией, памятью, температурой и значками проблем. Привязка задаётся в форме инсталляции — подробнее в
+главе [GPU](gpu.md#binding).
+
 ## Перцентили {#percentiles}
 
 Перцентили считаются по гистограммам сервера так же, как `histogram_quantile` в Prometheus, с линейной
````

<!-- block: file t12-docs-smoke-i1d-md step=12.3 path=docs/smoke/i1d.md -->
````
# Смоук I1d (GPU)

Сборка: `pnpm tauri dev`. Карты — моки DCGM (формат dcgm-exporter 4.8.4, собран по его исходникам), история —
настоящий Prometheus (`brew install prometheus`) с конфигом `scripts/prometheus/smoke.yml`. Каждый процесс — в своём
терминале:

```
pnpm mock-server --kind vllm --port 8000
pnpm mock-server --kind dcgm-full --gpus 8 --hostname gpu-node-1 --port 9400
pnpm mock-server --kind dcgm --gpus 2 --port 9401
prometheus --config.file=scripts/prometheus/smoke.yml --storage.tsdb.path=/tmp/glafira-prom --web.listen-address=127.0.0.1:9090
```

Перед окнами истории дайте Prometheus собрать данные 15–20 минут. Мок на 9402 запускается только в пункте 6.

1. Экспортёры. Инсталляция `dev-gpu` с серверами `vllm-1` `http://127.0.0.1:8000` и `vllm-lb` `http://lb.local:8000`
   (адреса нет — это нормально, сервер просто не подключится). «Изменить» → раздел «GPU (DCGM)», источник «Экспортёры
   DCGM», экспортёр 1 `http://127.0.0.1:9400/metrics`, «Добавить экспортёр», экспортёр 2 `http://127.0.0.1:9401`
   (без пути). «Проверить GPU»: «Карты» — «http://127.0.0.1:9400/metrics: карт 8 на gpu-node-1, драйвер 570.124.06» и
   «http://127.0.0.1:9401/metrics: карт 2 на localhost…»; «Привязка» — у `vllm-1` «localhost, все карты (автоподбор)»,
   у `vllm-lb` ⚠ «хоста сервера нет среди хостов GPU». Подсказок «Счётчики» нет (полный набор у мока 9400). У
   `vllm-1` поле «GPU» — `localhost`; у `vllm-lb` выбрать `gpu-node-1`, снять «Все карты», отметить GPU 0–3.
   «Сохранить»; снова «Изменить»: адрес экспортёра 2 — `http://127.0.0.1:9401/metrics`, проверка запускается сама.
2. Вкладка GPU. В дереве у `dev-gpu` пункт «GPU · 10» (после первого опроса). Вкладка: хосты `gpu-node-1` («Карт: 8 ·
   драйвер 570.124.06 · Серверы: vllm-lb») и `localhost` («Серверы: vllm-1»). Карточки: утилизация, SM, память «X / 79,6
   ГиБ», температура, мощность меняются каждые 5 с. Значков нет. Щелчок по `gpu-node-1` — пять графиков с рядом на
   карту («GPU 0»…«GPU 7»); щелчок по карточке GPU 2 — графики карты: «Утилизация и SM», «Память» (занято и всего),
   «Температура» (карта и память), «Мощность», «Такты».
3. Набор по умолчанию. Остановить мок 9400 и запустить вместо него `pnpm mock-server --kind dcgm --gpus 8 --hostname
   gpu-node-1 --port 9400`. На вкладке: «Экспортёр не собирает: активность SM, ошибки ECC, причины троттлинга, ошибки
   NVLink» и ссылка «Какие счётчики включить» (открывает главу GPU, раздел о счётчиках); у SM пометка «по GR engine».
   «Проверить GPU» — четыре ⚠ «Счётчики». Вернуть `dcgm-full`.
4. Проблемы. Перезапустить мок 9400 с `--throttle --xid 79 --ecc`: у GPU 0 значки «Троттлинг: HW_SLOWDOWN, HW_THERMAL,
   POWER … %, THERMAL … %», «XID 79», «ECC: 2», а через 5–10 с ещё «ECC +N», «NVLink +N», «PCIe +N»; у остальных карт —
   «Троттлинг», «NVLink +N» и «PCIe +N», без XID и ECC. Подсказка у значка — расшифровка. На «Обзоре» инсталляции в
   строке `gpu-node-1` — «⚠ проблемы: 8».
5. Частичный сбой. Остановить мок 9401: над хостами «Экспортёр http://127.0.0.1:9401/metrics: …», карты `gpu-node-1`
   на месте; хост `localhost` пропадает. Остановить и 9400 — ошибка опроса с «Повторить». Запустить оба снова.
6. Из источника истории. В форме добавить источник истории `http://127.0.0.1:9090` («Проверить источник» проходит).
   Источник GPU — «Из источника истории», селектор пуст, «Проверить GPU»: «Карты» — «Селектор карт
   {hostname=~"localhost"} (автоподбор)» и «Карт 2 на localhost», в «Хостах» отмечен `localhost`. Отметить
   `gpu-node-1` — селектор `{hostname=~"gpu-node-1|localhost"}`. Предел: запустить `pnpm mock-server --kind dcgm
   --gpus 72 --hostname big-node --port 9402`, подождать 10 с, селектор `{job="dcgm"}`, «Проверить GPU» — ❌ «Найдено
   карт: 82…» (или 80 — по числу опрошенных целей). Остановить 9402, вернуть селектор хостов, «Сохранить». Вкладка GPU
   показывает те же карты, что в режиме экспортёров.
7. Окна истории. Вкладка GPU, щелчок по `gpu-node-1`: окно «Сейчас» сразу заполнено (до 15 минут), пунктир и «История
   до HH:MM». «1 ч» — «Шаг 1 мин · Prometheus», графики за время жизни Prometheus; «24 ч» — шаг 90 с. Точка «1 ч»
   сверяется с веб-интерфейсом Prometheus (Graph) по запросу
   `avg by (hostname, gpu) (avg_over_time(DCGM_FI_DEV_GPU_UTIL{hostname="gpu-node-1",GPU_I_ID=""}[1m]))`. Вернуть
   режим экспортёров: в форме у экспортёров поле «Серии в истории» подобрано («dcgm · 127.0.0.1:9400 (автоподбор)»),
   окна истории работают и в этом режиме.
8. Без источника истории. Убрать источник: окна «1 ч»–«30 д» неактивны с подсказкой «Добавьте источник истории метрик
   в инсталляцию»; в режиме «Из источника истории» форма предупреждает, что компонент не сохранится.
9. MIG и метка 4.1. Мок `pnpm mock-server --kind dcgm-mig --gpus 2 --hostname gpu-mig --port 9403` и экспортёр
   `http://127.0.0.1:9403/metrics`: у GPU 0 «MIG: 3g.40gb #1, 3g.40gb #2», утилизация и память — «нет данных»,
   температура есть. `--kind dcgm-4.1 --hostname gpu-old --port 9404` — хост `gpu-old` (метка `Hostname`).
10. Пароль экспортёров. Мок `pnpm mock-server --kind dcgm --api-key dcgm-pa55 --port 9405`, экспортёр
    `http://127.0.0.1:9405/metrics`: без входа — ❌ «Вход» «Экспортёр DCGM отклонил запрос (401)…»; вход «Имя
    пользователя и пароль», `glafira` / `dcgm-pa55` — проверка проходит. В `installations.json`, логах и выводе
    `pnpm tauri dev` строки `dcgm-pa55` нет. Экспорт инсталляции — `needsSecret.gpu: true`, пароля в файле нет.
11. «Обзор» сервера. Вкладка `vllm-lb` → «Обзор»: блок «GPU сервера» — GPU 0–3 с утилизацией, памятью и температурой,
    «Открыть GPU». У сервера без привязки — «Привяжите GPU в форме сервера».
12. Справка и английский: F1 на вкладке GPU открывает главу «GPU»; в английском интерфейсе тексты вкладки, формы и
    проверки — английские.

Отложено до стенда с GPU:

13. Настоящий dcgm-exporter: сверка формата меток (`hostname`/`Hostname`, `UUID`/`uuid`, `DCGM_FI_DRIVER_VERSION`) и
    единиц (МиБ, Вт, доли у `PROF_*`) с моком — первым делом.
14. Троттлинг и ошибки под нагрузкой; карты A100/H100 с NVLink; MIG на настоящей карте.
15. dcgm-exporter в Kubernetes (GPU Operator): метки `pod`/`namespace` и «Нагрузка» на карточке.
````

<!-- block: patch t12-docs-superpowers-specs-2026-10-29-glafira-inference-design-md step=12.3 -->
````
diff --git a/docs/superpowers/specs/2026-10-29-glafira-inference-design.md b/docs/superpowers/specs/2026-10-29-glafira-inference-design.md
index b9b0a56..62978c4 100644
--- a/docs/superpowers/specs/2026-10-29-glafira-inference-design.md
+++ b/docs/superpowers/specs/2026-10-29-glafira-inference-design.md
@@ -253,9 +253,10 @@ crates/glafira-mcp-inference — инструменты, типы ответов
 - **История:** Prometheus или VictoriaMetrics — URL, авторизация (без неё / bearer / basic), CA. Сопоставление серий
   с серверами — селектор на сервер (`{job="vllm", instance="10.0.0.5:8000"}`), автоподбор по совпадению `instance`
   с адресом сервера; без сопоставления история сервера недоступна.
-- **GPU:** экспортёр DCGM — `direct` (список URL `/metrics` экспортёров), `prometheus` (те же серии из источника
-  истории) или `ssh` (`nvidia-smi --query-gpu` на SSH-хостах). Привязка GPU к хосту — по меткам `Hostname`, `pod`,
-  `namespace` экспортёра или по SSH-хосту.
+- **GPU:** экспортёр DCGM — `direct` (до 16 URL `/metrics` экспортёров с общим входом basic), `prometheus` (те же серии
+  из источника истории под селектором карт с автоподбором, не больше 64 карт) или `ssh` (`nvidia-smi --query-gpu` на
+  SSH-хостах, с I3). Хост карты — метка `hostname` экспортёра (dcgm-exporter 4.8) или `Hostname` (4.1 и раньше), иначе
+  адрес экспортёра или SSH-хост; уточнение — спека I1d §2–3.
 
 ### 5.4 Kubernetes
 
@@ -326,6 +327,8 @@ gpus? [{ hostRef, index }] }]`. Автоопределение (`detect_bindings
 проброс порта контейнера) совпадает с портом URL; GPU — по `CUDA_VISIBLE_DEVICES`, `nvidia-smi
 --query-compute-apps` (pid → контейнер) или меткам DCGM `pod`/`namespace`. Найденные связи человек подтверждает или
 правит в форме; неподтверждённые помечены «предположительно», правила по ним работают, но находка говорит об этом.
+До I3 привязка сервера к картам — `ServerConfig.gpus { host, indices? }` с автоподбором по хосту URL (спека I1d §2);
+в I3 она становится частью `bindings`.
 
 ### 5.8 Проверка подключения
 
@@ -338,7 +341,8 @@ gpus? [{ hostRef, index }] }]`. Автоопределение (`detect_bindings
   конечный адрес»; прочие 4xx — ❌ неожиданный статус; «не готов» — только 5xx) → вход (`/v1/models` с ключом;
   401/403 — ❌ на шаге «Вход», без ключа — «Сервер требует ключ API») → тип и версия → модели (пустой список — ❌);
 - **история метрик:** DNS/TCP → TLS → вход → запрос `up` → найдены серии серверов;
-- **DCGM:** DNS/TCP → `/metrics` → есть `DCGM_FI_DEV_GPU_UTIL`;
+- **DCGM:** DNS/TCP → `/metrics` → есть `DCGM_FI_DEV_FB_USED` (в режиме MIG утилизации карты нет) → привязка серверов
+  → сопоставление экспортёров с историей → полнота набора счётчиков (спека I1d §3);
 - **Kubernetes:** шаги Kubernetes-версии → права на чтение подов в указанных namespace → найден инференс;
 - **SSH:** DNS/TCP → ключ хоста → вход → `uname` → какие утилиты из шаблонов есть;
 - **шлюз:** `/health` или чтение файла → admin API или разбор `config.yaml` → число маршрутов.
@@ -361,14 +365,16 @@ gpus? [{ hostRef, index }] }]`. Автоопределение (`detect_bindings
 
 ### 6.2 Обзор инсталляции
 
-Серверы (тип, версия, модели, статус, running/waiting, KV), GPU по хостам (модель, утилизация, память, температура),
+Серверы (тип, версия, модели, статус, running/waiting, KV), GPU по хостам (модель, утилизация, память, температура; с
+I1d),
 шлюз (число маршрутов, внешние маршруты), сводка последней проверки и отчёта о замкнутости, последние замеры; кнопки
 «Проверить», «Проверить замкнутость», «Замер», «Подбор конфигурации».
 
 ### 6.3 Сервер
 
 Подвкладки:
-- **Обзор:** статус, тип, версия, модели (`root`, `max_model_len`), связи, плитки ключевых метрик, блок находок;
+- **Обзор:** статус, тип, версия, модели (`root`, `max_model_len`), связи, плитки ключевых метрик, карты GPU привязки
+  (с I1d), блок находок;
 - **Конфигурация:** аргументы `{ name, value, source }` (§4.3), переменные окружения (§5.5), расхождения источников,
   кнопка «Артефакт» (§9.4);
 - **Метрики:** графики running/waiting, KV-кэш, вытеснения, токены в секунду (вход и выход), prefix-cache hit,
@@ -378,10 +384,11 @@ gpus? [{ hostRef, index }] }]`. Автоопределение (`detect_bindings
 
 ### 6.4 GPU
 
-Карточки по хостам и картам: модель, память (занято / всего), утилизация, активность SM (`DCGM_FI_PROF_SM_ACTIVE`, где
-есть), температура, мощность, причины троттлинга, XID, ECC (корректируемые и некорректируемые), NVLink и PCIe (ошибки
-и повторы), процессы на карте (по SSH: pid и имя команды без аргументов) и к какому серверу они относятся; история —
-из источника истории.
+Карточки по хостам и картам: модель, память (занято / всего), утилизация, активность SM (`DCGM_FI_PROF_SM_ACTIVE`, без
+неё — `DCGM_FI_PROF_GR_ENGINE_ACTIVE` с пометкой), температура, мощность, причины троттлинга, XID, ECC (корректируемые и
+некорректируемые), NVLink и PCIe (ошибки и повторы), процессы на карте (по SSH: pid и имя команды без аргументов, с I3;
+до I3 — «Нагрузка» по меткам Kubernetes экспортёра) и к какому серверу они относятся; у MIG — перечень экземпляров;
+история — из источника истории. Уточнение — спека I1d §4–5.
 
 ### 6.5 Логи и фильтр промптов
 
@@ -449,10 +456,10 @@ gpus? [{ hostRef, index }] }]`. Автоопределение (`detect_bindings
 | `config.prefixCachingOff` | server | prefix caching выключен | И | И | — |
 | `config.noApiKey` | server | `/v1/models` отвечает 200 без ключа API | К | П | И |
 | `gpu.xid` | gpu | XID-ошибка за окно | К | П | П |
-| `gpu.eccUncorrectable` | gpu | некорректируемые ECC-ошибки (volatile) > 0 | К | К | П |
-| `gpu.throttling` | gpu | тепловой или энергетический троттлинг (`HW_SLOWDOWN`, `HW_THERMAL`, `SW_THERMAL`, `HW_POWER_BRAKE`) в отсчётах окна | П | П | И |
+| `gpu.eccUncorrectable` | gpu | некорректируемые ECC-ошибки (volatile) > 0 или переназначенные строки по некорректируемым ошибкам; без счётчиков ECC — разрыв в `check.partial` | К | К | П |
+| `gpu.throttling` | gpu | тепловой или энергетический троттлинг (`HW_SLOWDOWN`, `HW_THERMAL`, `SW_THERMAL`, `HW_POWER_BRAKE`) в отсчётах окна; без маски и счётчиков троттлинга — разрыв в `check.partial` с именем счётчика | П | П | И |
 | `gpu.foreignProcess` | gpu | на GPU сервера есть процесс, не связанный с сервером (§5.7) | П | П | И |
-| `gpu.idleWithQueue` | server | `requests_waiting` > 0 и средняя активность SM (без неё — утилизация) GPU сервера < 50 % за окно | П | И | И |
+| `gpu.idleWithQueue` | server | `requests_waiting` > 0 и средняя активность SM (без неё — активность graphics engine, без неё — утилизация) GPU сервера < 50 % за окно | П | И | И |
 | `gpu.driverMismatch` | installation | у хостов серверов одной модели разные версии драйвера | И | И | — |
 | `privacy.promptLogging` | server / gateway | сервер пишет тексты запросов в лог (аргументы по таблице адаптера или `requestLoggingDetected`, §6.5) или шлюз хранит промпты (`store_prompts_in_spend_logs`) | П | П | И |
 | `egress.usageStats` | server | сбор статистики использования vLLM не выключен (нет `VLLM_NO_USAGE_STATS=1` и `DO_NOT_TRACK=1`) | П | П | И |
@@ -959,7 +966,7 @@ keychain: сервис glafira-inference, ключи <installation-id>/<componen
 | **I1a. Каркас и подключения** | Новый репозиторий, перенос основы (§3) и `PORTED.md`; хранилище, keychain, настройки, политика, журнал, словари en/ru, подсказки, справка (главы `getting-started`, `installations`), папки, экспорт и импорт; профиль инсталляции с серверами; HTTP-подключение, определение типа (§4.3), проверка подключения сервера (§5.8); дерево, вкладка инсталляции (состав, статус), «О программе»; CI |
 | **I1b. Сервер: чтение и метрики** | Адаптеры vLLM и SGLang: модели, аргументы с источниками `server_info`/`models`/`metrics`, канонические метрики (кандидаты имён серий, спека этапа I1b); поток метрик «сейчас» и экраны сервера; графики; нижние границы версий vLLM 0.10, SGLang 0.5 |
 | **I1c. История метрик** | Источник истории (Prometheus/VM) и сопоставление серий (найденные цели, автоподбор, селектор), окна 1 ч / 24 ч / 7 д / 30 д, «Сейчас» из истории (спека этапа I1c) |
-| **I1d. GPU** | DCGM (`direct`, `prometheus`), привязка карт к хостам, экран GPU |
+| **I1d. GPU** | DCGM (`direct`, `prometheus`), привязка карт к хостам и серверам, экран GPU, блоки GPU в обзорах инсталляции и сервера (спека этапа I1d) |
 | **I2. Проверка и MCP** | Проверка (`health/`), правила `server.*`, `kv.*`, `queue.*`, `latency.*`, `requests.*`, `config.*`, `gpu.*` (кроме `gpu.foreignProcess`), `check.partial`; вкладка «Замечания», значки и блоки; `glafira-mcp-core` (перенос), `glafira-mcp-inference`, уровень `metadata`, 11 инструментов; релизный процесс и первый релиз |
 | **I3. Kubernetes и SSH** | Перенос подключения к Kubernetes, поиск инференса в подах, источник `pod`; SSH (`russh`, `known_hosts`, `ProxyJump`, импорт `~/.ssh/config`, шаблоны команд), источники `cmdline` и `docker`, GPU по SSH, `gpu.foreignProcess`; связи компонентов (§5.7); логи подов, docker и journald с фильтром промптов, `privacy.promptLogging`; уровень `logs`, политика `mcpLogsEnabled`, `allowSsh`; 13 инструментов. Первая задача плана — проверка агента SSH на Windows |
 | **I4. Подбор и артефакты** | Каталоги моделей и GPU, импорт `config.json`, калькулятор, артефакты четырёх видов с diff, кнопки «Артефакт» у находок, `estimate_capacity`, `propose_config`, `list_proposals`, `withdraw_proposal` (17 инструментов); уточнение резервов калькулятора замерами на стенде |
````

<!-- block: patch t12-docs-superpowers-specs-2026-11-01-glafira-inference-i1d-design-md step=12.3 -->
````
diff --git a/docs/superpowers/specs/2026-11-01-glafira-inference-i1d-design.md b/docs/superpowers/specs/2026-11-01-glafira-inference-i1d-design.md
index 40c956f..a22aff7 100644
--- a/docs/superpowers/specs/2026-11-01-glafira-inference-i1d-design.md
+++ b/docs/superpowers/specs/2026-11-01-glafira-inference-i1d-design.md
@@ -23,7 +23,7 @@
   `get_metrics_history` для GPU (I2); метрики экземпляров MIG (после MVP).
 
 **Что умеет dcgm-exporter 4.8.4** (по исходникам, ветка `main` на 2026-11-01). Метки карты: `gpu` (номер), `UUID`
-(или `uuid` — по флагу экспортёра), `pci_bus_id`, `device`, `modelName`, `hostname` (в 3.x — `Hostname`), у MIG —
+(или `uuid` — по флагу экспортёра), `pci_bus_id`, `device`, `modelName`, `hostname` (до 4.1 включительно — `Hostname`), у MIG —
 `GPU_I_PROFILE`, `GPU_I_ID`; версия драйвера — меткой `DCGM_FI_DRIVER_VERSION`; в Kubernetes — `pod`, `namespace`,
 `container`. Набор счётчиков по умолчанию (`default-counters.csv`) **не содержит** `DCGM_FI_PROF_SM_ACTIVE`, ECC
 (`DCGM_FI_DEV_ECC_*`), причин троттлинга (`DCGM_FI_DEV_CLOCKS_EVENT_REASONS`), счётчиков `*_VIOLATION` и ошибок
@@ -47,8 +47,9 @@ remapped rows и `DCGM_FI_DEV_PCIE_REPLAY_COUNTER`. Поэтому у каждо
   ```
 
   Поля из более новой версии — `extra`, как у прочих частей профиля.
-- `direct`: адрес экспортёра — `http(s)://host:port[/путь]`, путь по умолчанию — `/metrics`; `user:pass@` в URL
-  отклоняется (`errors.installations.credentialsInUrl`); больше 16 экспортёров — `errors.gpu.tooManyExporters`;
+- `direct`: адрес экспортёра — `http(s)://host:port[/путь]`, путь по умолчанию — `/metrics` (нормализация дописывает
+  его к адресу без пути); `user:pass@` в URL отклоняется (`errors.installations.credentialsInUrl`); без экспортёров —
+  `errors.gpu.noExporters`, больше 16 — `errors.gpu.tooManyExporters`;
   повторяющийся адрес — `errors.gpu.duplicateExporter`. Без проверки сертификата — `CaConfig::Insecure` с постоянным
   предупреждением, запрещается политикой `allowInsecureTls = false` (при сохранении и импорте). Перенаправления не
   выполняются, как у серверов.
@@ -64,17 +65,19 @@ remapped rows и `DCGM_FI_DEV_PCIE_REPLAY_COUNTER`. Поэтому у каждо
 - **Привязка сервера:** `ServerConfig.gpus: Option<GpuBinding>`:
 
   ```
-  GpuBinding { host, indices?: [u32], auto: bool }   // indices = None — все карты хоста
+  GpuBinding { host, indices?: [u32] }   // indices = None — все карты хоста
   ```
 
-  `auto = true` — привязку подобрала проверка (раздел 3). Если после этого у сервера меняется адрес, подобранная
-  привязка сбрасывается, и следующая проверка подбирает заново; выбранная человеком остаётся.
+  Подобранную проверкой привязку помнит форма, как селекторы истории в I1c: если после автоподбора у сервера меняется
+  адрес, она сбрасывается, и следующая проверка подбирает заново; выбранная человеком остаётся. Нормализация:
+  хост без пробелов, номера по возрастанию без повторов, пустой список — все карты; без компонента GPU привязок нет.
+  Привязка с пустым хостом — `errors.gpu.badBinding`.
 - **Экспорт и импорт** — как в I1c: секрета в файле нет, у инсталляции флаг `needsSecret.gpu`; при обновлении импорт
   предупреждает, если у компонента с сохранённым секретом меняются адреса экспортёров, прокси или проверка
   сертификата; без компонента в файле или с другим видом входа сохранённый секрет удаляется. Селекторы и привязки
   экспортируются как есть.
 
-**Хост карты.** Метка `hostname`, затем `Hostname`; нет обеих — хост из URL экспортёра (`direct`) или из метки
+**Хост карты.** Метка `hostname` (dcgm-exporter 4.8), затем `Hostname` (4.1 и раньше); нет обеих — хост из URL экспортёра (`direct`) или из метки
 `instance` без порта (`prometheus`). Карта — пара «хост + `gpu`»; `UUID` показывается в подсказке. Две карты с
 одной парой (два экспортёра на одном хосте) — обе показываются, вторая с подписью экспортёра, проверка пишет ⚠️.
 
@@ -103,16 +106,21 @@ remapped rows и `DCGM_FI_DEV_PCIE_REPLAY_COUNTER`. Поэтому у каждо
 - **Сопоставление с историей** (только `direct`, при источнике истории) — шаг на экспортёр: ✅ «`dcgm · host:9400`
   (автоподбор)» или «селектор» (под селектором есть `DCGM_FI_DEV_FB_USED`); ⚠️ «Цели с адресом экспортёра нет»;
   ⚠️ «Под селектором нет серий»; ❌ ошибка разбора селектора.
-- **Полнота набора счётчиков** — шаг-подсказка: ⚠️ «Экспортёр не собирает: активность SM, ECC, причины троттлинга»
-  (перечень отсутствующих групп раздела 4) со ссылкой на главу справки. Подключению не мешает.
+- **Полнота набора счётчиков** — шаг-подсказка «Счётчики» ⚠️ на каждую группу раздела 4, которой нет ни у одной карты
+  (`messages.gpuTest.missingSm`, `missingEcc`, `missingThrottle`, `missingNvlink` — с именами серий). Подключению не
+  мешает. Имена новых шагов: `metrics`, `cards`, `bind`, `history`, `counters`.
 
 Сбой компонента GPU не мешает подключению серверов: инсталляция подключена, GPU — ❌ в проверке и ошибка на
 вкладке GPU.
 
-**Автоподбор селектора `prometheus`.** Хосты серверов — хосты их URL (`localhost`, `127.0.0.1`, `[::1]` — один
-адрес, как в I1c). Совпавшие с хостами карт (по имени без учёта регистра или по IP) дают селектор
-`{hostname=~"h1|h2"}` — по той метке, которой подписаны найденные карты (`Hostname` у 3.x; при картах с обеими
-метками — два селектора не строятся, берётся `hostname`). Значения экранируются для регулярного выражения.
+**Совпадение хостов** (`gpu/bind.rs`): имена без учёта регистра; короткое имя совпадает с полным, если это его первая
+часть (`gpu-node-1` и `gpu-node-1.corp.local` — метка `hostname` обычно короткое имя узла); адреса IP — только
+целиком; `localhost`, `127.0.0.1`, `[::1]` — один адрес, как в I1c. DNS проверка не опрашивает.
+
+**Автоподбор селектора `prometheus`.** Хосты серверов — хосты их URL. Совпавшие с хостами карт дают селектор
+`{hostname=~"h1|h2"}` — по той метке, которой подписаны найденные карты (`Hostname` у 4.1 и раньше; при картах с
+обеими метками берётся `hostname`; без метки хоста — `{instance=~"(h1|h2)(:[0-9]+)?"}`). Значения экранируются для
+регулярного выражения.
 Срабатывает при проверке, если селектор пуст; подобранный селектор отмечен «автоподбор» в форме.
 
 **Автоподбор привязки** — у серверов без привязки: хост URL сервера совпал с хостом карт → `GpuBinding { host,
@@ -122,8 +130,11 @@ indices: None, auto: true }`. За балансировщиком и в Kubernet
 **Автоподбор `historySelector`** — как селектор сервера в I1c: цель с `instance = host:port` экспортёра среди
 `count by (job, instance) (DCGM_FI_DEV_FB_USED)`; несколько `job` — первый по алфавиту, остальные — в списке выбора.
 
-**Ошибки** `errors.gpu.*`: `noHistorySource`, `noDcgm`, `tooManyGpus`, `tooManyExporters`, `duplicateExporter`,
-`badUrl`, `badUsername`, `notConfigured` (у инсталляции нет компонента), `notBound` (у сервера нет привязки). Сеть —
+**Ошибки** `errors.gpu.*`: `noHistorySource`, `noDcgm`, `tooManyGpus`, `noExporters`, `tooManyExporters`,
+`duplicateExporter`, `badUrl`, `badUsername`, `badBinding`, `unauthorized` (401/403 экспортёра — «проверьте имя
+пользователя и пароль», а не ключ API), `notConfigured` (у инсталляции нет компонента), `notMatched` (экспортёры без
+селекторов истории), `hostNotInHistory` (в истории нет серий хоста окна), `tooManySeries` (больше 16 карт в окне хоста —
+запись `partial`). Сеть —
 `errors.server.*`, ответы источника — `errors.history.*`, разбор селекторов — `errors.history.selector` и
 `errors.history.selectorName`.
 
@@ -183,28 +194,35 @@ indices: None, auto: true }`. За балансировщиком и в Kubernet
   ответам.
 - `gpu/bind.rs` — хост карты, автоподбор селектора, привязки и `historySelector`, проверка номеров (раздел 3).
 - `gpu/check.rs` — шаги раздела 3.
-- `history/promql.rs` — шаблоны GPU для окон, чистые функции. Для селектора карт `S` (селектор `prometheus` или
-  `historySelector` экспортёра плюс `gpu="<номер>"` у одной карты) и интервала `r` окна (I1c §4):
+- `gpu/promql.rs` — шаблоны GPU для окон, чистые функции (отдельно от шаблонов сервера в `history/promql.rs`). Для
+  условий `S` (база — селектор `prometheus` или `historySelector` экспортёра, плюс точное условие на метку хоста и
+  `gpu="<номер>"` у одной карты, плюс `GPU_I_ID=""` — без экземпляров MIG) и интервала `r` окна (I1c §4), группировка
+  `B` = `hostname, Hostname, instance, gpu`:
 
   | Поле | Шаблон |
   |---|---|
-  | Датчик (`util`, `memUsed`, `temp`, `power`, такты, PCIe, NVLink) | `avg by (<метка хоста>, gpu) (avg_over_time(a{S}[r]))` |
-  | Доля (`smActive`, `tensorActive`, `dramActive`) | то же, `a or b` по кандидатам, × 100 в Rust |
-  | Троттлинг | `max by (<метка хоста>, gpu) (max_over_time(a{S}[r]))`, причины — разбор маски в Rust |
-  | Счётчик (`eccDbe`, `pcieReplay`, `nvlinkErrors`) | `sum by (<метка хоста>, gpu) (increase(a{S}[r]))` |
-
-  Метка хоста — `hostname`, `Hostname` или `instance`, та же, что у найденных проверкой карт. Шаги, длины и
-  интервалы окон — `Window` и `range` из I1c. Точки без значения — `null`.
-- `gpu/read.rs` — окно истории: запросы по полям графиков, не больше 4 одновременно, срок окна — как у серверов
-  (I1c §4), частичный сбой — `partial`.
+  | Датчик (`util`, `memUsed`, `memFree`, `memReserved`, `temp`, `memTemp`, `power`, `smClock`, `memClock`) | `avg by (B) (avg_over_time(a{S}[r]))` |
+  | Доля (`smActive`) | то же, `… or …` по кандидатам, × 100 в Rust |
+  | Троттлинг | `max by (B) (max_over_time(a{S}[r])) or …`, причины — разбор маски |
+  | Счётчик (`eccDbe`, `pcieReplay`) | `sum by (B) (increase(a{S}[r]))` |
+  | Сумма счётчиков (`nvlinkErrors`) | `increase((sum by (B) ({__name__=~"a\|b\|…",S}))[r:r/4])` — подзапрос: `increase` по каждому счётчику убрал бы имя серии, и одинаковые наборы меток столкнулись бы (Prometheus отвечает 422) |
+
+  14 запросов на окно; всего памяти — сумма в Rust. Хост ряда — из меток ответа тем же порядком, что у снимка. Шаги,
+  длины и интервалы окон — `Window` и `range` из I1c (интервал опроса цели не запрашивается). Точки без значения — нет
+  в `values` точки.
+- `gpu/read.rs` — окно истории: сначала `count by (hostname, Hostname, instance) (DCGM_FI_DEV_FB_USED{база})` под
+  каждой базой по очереди — какой меткой подписан хост окна (первая база, где он нашёлся); затем шаблоны, не больше 4
+  запросов одновременно (общий предел источника), срок окна — как у серверов (I1c §4), частичный сбой — `partial`.
+- `gpu/series.rs` — ряды окна: ряд на карту (не больше 16), точка на каждый шаг окна.
 - Открытый компонент (клиенты экспортёров или ссылка на источник истории) живёт в пуле подключений
   (`ServerPool::gpu`) до `evict` инсталляции.
 
 **Команды:**
 
-- `check_gpu_source(input)` (профиль формы и секреты; секрет — по правилам сохранения, как
-  `check_history_source`) → `{ report, hosts: [{ host, cards: [index] }], suggestions: { selector?, bindings:
-  [{ serverId, binding? }], exporters: [{ url, historySelector?, targets }] } }`;
+- `check_gpu_source(input)` (профиль формы и секреты: пароль экспортёров и секрет источника истории — по правилам
+  сохранения) → `{ report, hosts: [{ host, cards }], available: [host], selector?, bindings: [{ serverId, binding? }],
+  exporters: [{ url, historySelector?, targets }] }`; `available` — все хосты карт источника истории (режим
+  `prometheus`, для выбора хостов в форме);
 - `read_gpu_status(installationId)` → `GpuStatus` — снимок с приростами; фронтенд вызывает раз в 5 с, пока
   вкладка GPU, «Обзор» инсталляции или «Обзор» сервера с привязкой видны;
 - `read_gpu_history(installationId, window, host, gpu?)` → `{ window, stepSec, series: [{ host, gpu, points:
@@ -267,10 +285,12 @@ NVLink), MIG, что значат значки. Подсказки к полям
 
 - **Мок** `mock-server.mjs`:
   - `--kind dcgm` — набор счётчиков по умолчанию dcgm-exporter 4.8.4, метка `hostname`; параметры `--gpus 1..72`
-    (по умолчанию 2; больше 8 — только для проверки предела 64), `--hostname` (по умолчанию `localhost`), `--model` (по умолчанию `NVIDIA H100 80GB HBM3`);
+    (по умолчанию 2; больше 8 — только для проверки предела 64), `--hostname` (по умолчанию `localhost`), `--model`
+    (по умолчанию `NVIDIA H100 80GB HBM3`);
   - `--kind dcgm-full` — плюс `PROF_SM_ACTIVE`, ECC, `CLOCKS_EVENT_REASONS`, `*_VIOLATION`, ошибки NVLink (Hopper) и
     флаги `--throttle`, `--xid <код>`, `--ecc`, включающие проблемы;
-  - `--kind dcgm-3x` — метка `Hostname`, `UUID`; `--kind dcgm-mig` — карта с двумя экземплярами MIG;
+  - `--kind dcgm-4.1` — метка `Hostname` (dcgm-exporter 4.1), A100; `--kind dcgm-mig` — карта с двумя экземплярами MIG;
+  - `--pod ns/имя` — метки Kubernetes `container`, `namespace`, `pod`;
   - значения меняются от опроса к опросу, счётчики растут; `--api-key` у DCGM — пароль basic (логин `glafira`).
   Снимки собраны вручную по исходникам экспортёра, а не сняты с карты: файл `README.md` рядом со снимками говорит,
   откуда взят формат.
@@ -289,7 +309,7 @@ NVLink), MIG, что значат значки. Подсказки к полям
 - **Смоук `docs/smoke/i1d.md`:** `direct` на двух моках — проверка, карточки, «нет данных» и подсказка при наборе по
   умолчанию, значки при `--throttle --xid 79 --ecc`; частичный сбой (один мок остановлен); `prometheus` — автоподбор
   селектора, выбор хостов, предел 64 (селектор `{job="dcgm"}` при запущенном моке на 9402); привязка сервера автоподбором
-  и вручную; окна истории в обоих режимах и сверка точки с веб-интерфейсом Prometheus; MIG; `dcgm-3x`; пароль
+  и вручную; окна истории в обоих режимах и сверка точки с веб-интерфейсом Prometheus; MIG; `dcgm-4.1`; пароль
   экспортёра — нет на диске и в логах; справка и английский. Отложено до стенда: настоящий dcgm-exporter (сверка
   формата меток и единиц первым пунктом), троттлинг под нагрузкой, карты A100/H100 с NVLink.
 
@@ -297,7 +317,7 @@ NVLink), MIG, что значат значки. Подсказки к полям
 
 | Риск | Мера |
 |---|---|
-| Формат мока отличается от настоящего экспортёра (метки, единицы, версии 3.x/4.x) | Снимки по исходникам 4.8.4, кандидаты меток `hostname`/`Hostname`, `UUID`/`uuid`; сверка формата — первый пункт смоука на стенде |
+| Формат мока отличается от настоящего экспортёра (метки, единицы, версии 4.1/4.8) | Снимки по исходникам 4.8.4, кандидаты меток `hostname`/`Hostname`, `UUID`/`uuid`; сверка формата — первый пункт смоука на стенде |
 | Набор по умолчанию без SM, ECC и троттлинга — экран и будущие правила `gpu.*` без данных | Запасные серии, «нет данных» вместо нуля, шаг «полнота набора» с перечнем счётчиков, глава справки |
 | Хост URL сервера не совпадает с хостом карт (балансировщик, Service Kubernetes) | Ручная привязка в форме сервера; ⚠️ в проверке; точные связи — I3 |
 | Корпоративный Prometheus собирает сотни карт | Селектор с автоподбором, предел 64 карты, ❌ «Сузьте селектор» |
@@ -306,9 +326,11 @@ NVLink), MIG, что значат значки. Подсказки к полям
 
 ## 8. Изменения общей спеки
 
+Внесены в общую спеку последней задачей плана I1d.
+
 - §5.3 «GPU»: компонент — раздел 2 этой спеки (`direct` — до 16 экспортёров с общим входом basic, `prometheus` —
-  селектор карт с автоподбором, не больше 64 карт); хост карты — метка `hostname` (dcgm-exporter 4.x) или
-  `Hostname` (3.x), иначе адрес экспортёра.
+  селектор карт с автоподбором, не больше 64 карт); хост карты — метка `hostname` (dcgm-exporter 4.8) или
+  `Hostname` (4.1 и раньше), иначе адрес экспортёра.
 - §5.7: до I3 привязка сервера к GPU — `ServerConfig.gpus` (раздел 2); в I3 она становится частью `bindings`.
 - §5.8: шаги проверки DCGM — раздел 3; признак экспортёра — `DCGM_FI_DEV_FB_USED` вместо `DCGM_FI_DEV_GPU_UTIL`
   (в режиме MIG утилизации карты нет).
````

<!-- block: patch t12-scripts-i18n-check-mjs step=12.3 -->
````
diff --git a/scripts/i18n-check.mjs b/scripts/i18n-check.mjs
index 6f255c6..b25ba0b 100644
--- a/scripts/i18n-check.mjs
+++ b/scripts/i18n-check.mjs
@@ -562,7 +562,7 @@ export function findRustCyrillic(files) {
 }
 
 /** Главы руководства: тот же список, что `GUIDE_CHAPTERS` в `src/guide/chapters.ts`. */
-export const GUIDE_CHAPTERS = ["getting-started", "installations", "servers", "history", "updates"];
+export const GUIDE_CHAPTERS = ["getting-started", "installations", "servers", "history", "gpu", "updates"];
 
 const KEBAB = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
 
````

<!-- block: patch t12-src-components-gpu-gpupanel-tsx step=12.3 -->
````
diff --git a/src/components/gpu/GpuPanel.tsx b/src/components/gpu/GpuPanel.tsx
index c02c734..e29c84f 100644
--- a/src/components/gpu/GpuPanel.tsx
+++ b/src/components/gpu/GpuPanel.tsx
@@ -86,7 +86,12 @@ export function GpuPanel({ installationId }: { installationId: string }) {
         </p>
       ))}
       {status && status.missing.length > 0 && (
-        <p className="muted">{t("gpu.panel.missing", { groups: status.missing.map((g) => t(`gpu.groups.${g}`)).join(", ") })}</p>
+        <p className="muted">
+          {t("gpu.panel.missing", { groups: status.missing.map((g) => t(`gpu.groups.${g}`)).join(", ") })}{" "}
+          <button type="button" className="link" onClick={() => open({ kind: "help", chapter: "gpu", anchor: "counters" })}>
+            {t("gpu.panel.missingLink")}
+          </button>
+        </p>
       )}
       {status && status.hosts.length === 0 && <p className="empty">{t("gpu.panel.noCards")}</p>}
       {status?.hosts.map((h) => {
````

<!-- block: patch t12-src-components-help-helpchapterfor-ts step=12.3 -->
````
diff --git a/src/components/help/helpChapterFor.ts b/src/components/help/helpChapterFor.ts
index c512cc1..05bcc74 100644
--- a/src/components/help/helpChapterFor.ts
+++ b/src/components/help/helpChapterFor.ts
@@ -9,6 +9,8 @@ export function helpChapterFor(tab: TabSpec | null): ChapterId {
       return "installations";
     case "server":
       return "servers";
+    case "gpu":
+      return "gpu";
     case "about":
       return "updates";
     default:
````

<!-- block: patch t12-src-guide-chapters-ts step=12.3 -->
````
diff --git a/src/guide/chapters.ts b/src/guide/chapters.ts
index d275656..02b7544 100644
--- a/src/guide/chapters.ts
+++ b/src/guide/chapters.ts
@@ -1,5 +1,5 @@
 /** Главы руководства (спец. §16); каждый этап добавляет свои. Тот же список — в `scripts/i18n-check.mjs`. */
-export const GUIDE_CHAPTERS = ["getting-started", "installations", "servers", "history", "updates"] as const;
+export const GUIDE_CHAPTERS = ["getting-started", "installations", "servers", "history", "gpu", "updates"] as const;
 
 export type ChapterId = (typeof GUIDE_CHAPTERS)[number];
 
````

<!-- block: locales t12-locales-t12 step=12.3 -->
````
{
  "en": { "gpu": { "panel": { "missingLink": "Counters to enable" } } },
  "ru": { "gpu": { "panel": { "missingLink": "Какие счётчики включить" } } }
}
````

<!-- block: locales t12-locales-t12-replace step=12.3 replace=1 -->
````
{
  "en": {
    "about": { "summary": "Glafira is a desktop app for connecting to corporate LLM inference, monitoring, diagnosing and auditing it, with vLLM and SGLang in focus. This version (stage I1d) covers connecting and checking: installations of several servers, API keys in the system secret store, step-by-step server checks, folders, export and import, live server metrics with launch arguments, metric history from Prometheus or VictoriaMetrics and NVIDIA GPUs from DCGM. Diagnostics and audit come in the next stages." },
    "gpu": { "panel": { "missing": "The exporter does not collect: {{groups}}." } }
  },
  "ru": {
    "about": { "summary": "Glafira — настольное приложение для подключения к корпоративному инференсу LLM, его мониторинга, диагностики и аудита; в фокусе — vLLM и SGLang. В этой версии (этап I1d) готовы подключение и проверка: инсталляции из нескольких серверов, ключи API в системном хранилище секретов, пошаговая проверка серверов, папки, экспорт и импорт, метрики сервера «сейчас» и аргументы запуска, история метрик из Prometheus или VictoriaMetrics и карты NVIDIA по метрикам DCGM. Диагностика и аудит появятся в следующих этапах." },
    "gpu": { "panel": { "missing": "Экспортёр не собирает: {{groups}}." } }
  }
}
````

- [ ] **Шаг 12.4: Проверки.**

Run: `pnpm test`
Expected: `Test Files  40 passed`, `Tests  289 passed`
Run: `pnpm exec tsc --noEmit`
Expected: без вывода
Run: `pnpm i18n:check`
Expected: `✓ словари en, ru согласованы, вызовов из Rust: 187`
Run: `pnpm test:scripts`
Expected: `# pass 115`
Run: `cd src-tauri && cargo test --lib -q`
Expected: `test result: ok. 345 passed`
Run: `pnpm build`
Expected: `✓ dist/assets: чанки совместимы с WebKit macOS 11+`

- [ ] **Шаг 12.5: Коммит.**

<!-- block: shell t12-commit step=12.5 -->
````
git add README.md \
  docs/guide/en/getting-started.md \
  docs/guide/en/gpu.md \
  docs/guide/en/servers.md \
  docs/guide/ru/getting-started.md \
  docs/guide/ru/gpu.md \
  docs/guide/ru/servers.md \
  docs/smoke/i1d.md \
  docs/superpowers/specs/2026-10-29-glafira-inference-design.md \
  docs/superpowers/specs/2026-11-01-glafira-inference-i1d-design.md \
  scripts/i18n-check.mjs \
  src/components/gpu/GpuPanel.test.tsx \
  src/components/gpu/GpuPanel.tsx \
  src/components/help/helpChapterFor.test.ts \
  src/components/help/helpChapterFor.ts \
  src/guide/chapters.test.ts \
  src/guide/chapters.ts \
  src/locales/en.json \
  src/locales/ru.json
git commit -s -q -F - <<'EOF'
docs(gpu): GPU chapter, stage texts, I1d smoke

The guide gets a GPU chapter in English and Russian (GPU source, binding
servers, the GPU tab and badges, counters to enable, troubleshooting and
a Prometheus configuration example) linked from Servers, Getting started,
the F1 key on the GPU tab and the missing counters hint. README, Getting
started and About name stage I1d. docs/smoke/i1d.md runs against the
DCGM mocks and a real Prometheus. The I1d spec follows the prototype,
the general spec takes its changes.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
````

---

## Самопроверка плана

- Покрытие спеки I1d: §1 — границы этапа соблюдены (режим `ssh`, процессы, правила и MCP — I2/I3), мок — задача 1; §2 — задача 2 (профиль, секрет, экспорт и импорт), хост карты — задача 3; §3 — задачи 4 (автоподбор, совпадение хостов) и 7 (шаги проверки); §4 — задачи 3 (поля), 5 (источник, приросты), 6 (шаблоны, ряды), 7 (окно, команды, пул); §5 — задачи 8 (типы, хуки), 9 (вкладка, дерево), 10 (форма, поле сервера), 11 (обзоры), глава — задача 12; §6 — мок (задача 1), снимки мока (задачи 3, 5), ответы Prometheus (задачи 5, 6), тесты всех задач, смоук (задача 12); §7 — меры в задачах 3, 4, 5 и в смоуке; §8 — общая спека и спека I1d поправлены задачей 12.
- Плейсхолдеров нет: код каждого шага — блоки, извлекаемые скриптом; ожидаемые числа — по прогонам прототипа.
- Имена между задачами: `dcgmMetrics` (задача 1) → `GpuSource`, `DirectGpu`, `GpuBinding`, `gpu_secret_key`, `gpu_op` (2) → `Field`, `SPECS`, `GpuCard`, `cards_from`, `missing`, `group` (3) → `same_host`, `suggest_binding`, `suggest_selector`, `FoundCard`, `DcgmTarget` (4) → `OpenGpu`, `GpuSnapshot`, `snapshot`, `GpuStatus`, `record` (5) → `queries`, `GpuQuery`, `assemble`, `GpuSeries` (6) → `check_gpu`, `GpuCheck`, `read_history`, `ServerPool::gpu`, команды (7) → TS-типы `GpuStatus`, `GpuCheck`, `GpuHistoryAnswer`, `useGpuStatus`, `useGpuHistory`, `nowSeries` (8) → `GpuPanel`, `cardAlerts`, `mergeCards`, `ChartPoint` (9) → `GpuFields`, `GpuBindingField` (10) → `GpuHostsBlock`, `ServerGpuBlock` (11) → глава `gpu` (12).
