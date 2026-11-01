import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createServer } from "node:http";
import { test } from "node:test";
import { dcgmMetrics, handler, KINDS, liveMetrics, parseArgs, prometheusReply } from "./mock-server.mjs";

async function serve(opts) {
  const server = createServer(handler({ health: 200, apiKey: null, ...opts }));
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const base = `http://127.0.0.1:${server.address().port}`;
  return { base, close: () => new Promise((resolve) => server.close(resolve)) };
}

test("параметры: значения по умолчанию, свои и ошибки", () => {
  const defaults = { kind: "vllm", port: 8000, host: "127.0.0.1", apiKey: null, health: 200, models: "full", engines: 1, resetAfter: 0, slow: 0, huge: false };
  assert.deepEqual(parseArgs([]), defaults);
  assert.deepEqual(
    parseArgs(["--kind", "sglang", "--port", "30000", "--api-key", "sk", "--health", "503", "--models", "empty", "--engines", "2", "--reset-after", "3"]),
    { ...defaults, kind: "sglang", port: 30000, apiKey: "sk", health: 503, models: "empty", engines: 2, resetAfter: 3 },
  );
  assert.throws(() => parseArgs(["--engines", "9"]), /--engines.*Использование/s);
  assert.throws(() => parseArgs(["--reset-after", "0"]), /--reset-after/);
  assert.deepEqual(parseArgs(["--kind", "prometheus", "--huge", "--slow", "40"]), { ...defaults, kind: "prometheus", huge: true, slow: 40 });
  assert.throws(() => parseArgs(["--slow", "0"]), /--slow/);
  assert.throws(() => parseArgs(["--kind", "tgi"]), /неизвестный вид/);
  assert.throws(() => parseArgs(["--verbose", "1"]), /неизвестный параметр/);
});

test("параметры: порт и код здоровья проверяются, ошибка с текстом использования", () => {
  assert.equal(parseArgs(["--port", "0"]).port, 0, "порт 0 — любой свободный");
  assert.equal(parseArgs(["--port", "65535"]).port, 65535);
  assert.equal(parseArgs(["--health", "200"]).health, 200);
  assert.equal(parseArgs(["--health", "599"]).health, 599);
  assert.throws(() => parseArgs(["--health", "99"]), /--health.*Использование/s);
  assert.throws(() => parseArgs(["--health", "abc"]), /--health.*Использование/s);
  assert.throws(() => parseArgs(["--health", "600"]), /--health/);
  assert.throws(() => parseArgs(["--health", "200.5"]), /--health/);
  assert.throws(() => parseArgs(["--port", "abc"]), /--port.*Использование/s);
  assert.throws(() => parseArgs(["--port", "70000"]), /--port.*Использование/s);
  assert.throws(() => parseArgs(["--port", "-1"]), /--port/);
  assert.throws(() => parseArgs(["--port", ""]), /--port/);
  assert.throws(() => parseArgs(["--port"]), /--port/, "значение не указано");
  assert.throws(() => parseArgs(["--models", "many"]), /--models.*Использование/s);
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
    assert.deepEqual(await (await fetch(`${s.base}/version`)).json(), { version: "0.32.0" });
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

test("sglang-no-metrics: пути sglang 0.5.22 без /metrics", async () => {
  const s = await serve({ kind: "sglang-no-metrics" });
  try {
    assert.equal((await (await fetch(`${s.base}/server_info`)).json()).version, "0.5.22");
    assert.equal((await fetch(`${s.base}/model_info`)).status, 200);
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

test("ключ API у vLLM: ключ нужен только путям с префиксом /v1", async () => {
  const s = await serve({ kind: "vllm", apiKey: "sk-dev" });
  try {
    assert.equal((await fetch(`${s.base}/version`)).status, 200);
    assert.equal((await fetch(`${s.base}/metrics`)).status, 200);
    assert.equal((await fetch(`${s.base}/health`)).status, 200);
    const denied = await fetch(`${s.base}/v1/models`);
    assert.equal(denied.status, 401);
    assert.deepEqual(await denied.json(), { error: "Unauthorized" });
    const headers = { authorization: "Bearer sk-dev" };
    assert.equal((await fetch(`${s.base}/v1/models`, { headers })).status, 200);
    assert.equal((await fetch(`${s.base}/v1/models`, { headers: { authorization: "Bearer other" } })).status, 401);
  } finally {
    await s.close();
  }
});

test("ключ API у SGLang: без ключа открыты только /health и /metrics", async () => {
  const s = await serve({ kind: "sglang", apiKey: "sk-dev" });
  try {
    assert.equal((await fetch(`${s.base}/metrics`)).status, 200);
    assert.equal((await fetch(`${s.base}/health`)).status, 200);
    for (const path of ["/server_info", "/get_server_info", "/model_info", "/get_model_info", "/v1/models"]) {
      const denied = await fetch(`${s.base}${path}`);
      assert.equal(denied.status, 401, path);
      assert.deepEqual(await denied.json(), { error: "Unauthorized" });
    }
    const headers = { authorization: "Bearer sk-dev" };
    assert.equal((await fetch(`${s.base}/server_info`, { headers })).status, 200);
  } finally {
    await s.close();
  }
});

test("ключ API у llama.cpp и Ollama — как у vLLM: только /v1", async () => {
  for (const kind of ["llamacpp", "ollama"]) {
    const s = await serve({ kind, apiKey: "sk-dev" });
    try {
      assert.equal((await fetch(`${s.base}/v1/models`)).status, 401, kind);
      assert.equal((await fetch(`${s.base}/v1/models`, { headers: { authorization: "Bearer sk-dev" } })).status, 200, kind);
      assert.equal((await fetch(`${s.base}/nope`)).status, 404, `${kind}: вне /v1 ключ не нужен`);
    } finally {
      await s.close();
    }
  }
});

test("--models empty: /v1/models отдаёт пустой список, остальное без изменений", async () => {
  for (const kind of Object.keys(KINDS).filter((k) => !KINDS[k].prometheus && !KINDS[k].dcgm)) {
    const s = await serve({ kind, models: "empty" });
    try {
      const r = await fetch(`${s.base}/v1/models`);
      assert.equal(r.status, 200, kind);
      assert.match(r.headers.get("content-type"), /application\/json/);
      assert.deepEqual(await r.json(), { object: "list", data: [] }, kind);
    } finally {
      await s.close();
    }
  }
  const s = await serve({ kind: "vllm", models: "empty" });
  try {
    assert.deepEqual(await (await fetch(`${s.base}/version`)).json(), { version: "0.32.0" });
  } finally {
    await s.close();
  }
  const full = await serve({ kind: "vllm" });
  try {
    assert.ok((await (await fetch(`${full.base}/v1/models`)).json()).data.length > 0, "по умолчанию — снимок");
  } finally {
    await full.close();
  }
});

test("--health 3xx: перенаправление с заголовком Location на https", async () => {
  for (const code of [301, 302, 307, 308]) {
    const s = await serve({ kind: "vllm", health: code });
    try {
      const r = await fetch(`${s.base}/health`, { redirect: "manual" });
      assert.equal(r.status, code);
      assert.equal(r.headers.get("location"), `https://${new URL(s.base).host}/health`);
    } finally {
      await s.close();
    }
  }
  const s = await serve({ kind: "vllm", health: 503 });
  try {
    const r = await fetch(`${s.base}/health`, { redirect: "manual" });
    assert.equal(r.status, 503);
    assert.equal(r.headers.get("location"), null, "Location только у 3xx");
  } finally {
    await s.close();
  }
});

/** Значение первой серии `name` (с метками `labels`, если заданы) в тексте /metrics. */
function value(text, name, labels = "") {
  const line = text.split("\n").find((l) => l.startsWith(name) && l.includes(labels) && /[ {]/.test(l[name.length]));
  return line === undefined ? undefined : Number(line.slice(line.lastIndexOf(" ") + 1));
}

test("живые метрики: счётчики и гистограммы растут от опроса к опросу, KV колеблется в 0..1", async () => {
  const s = await serve({ kind: "vllm" });
  try {
    const a = await (await fetch(`${s.base}/metrics`)).text();
    const b = await (await fetch(`${s.base}/metrics`)).text();
    assert.equal(value(a, "vllm:prompt_tokens_total"), 1041);
    assert.ok(value(b, "vllm:prompt_tokens_total") > value(a, "vllm:prompt_tokens_total"));
    assert.ok(value(b, "vllm:time_to_first_token_seconds_count") > value(a, "vllm:time_to_first_token_seconds_count"));
    assert.equal(value(b, "vllm:num_requests_running"), value(a, "vllm:num_requests_running"), "gauge не растёт");
    for (const t of [a, b]) {
      const kv = value(t, "vllm:kv_cache_usage_perc");
      assert.ok(kv >= 0 && kv <= 1, String(kv));
    }
  } finally {
    await s.close();
  }
});

test("--reset-after: после N опросов счётчики начинаются заново", async () => {
  const s = await serve({ kind: "sglang", resetAfter: 2 });
  try {
    const read = async () => value(await (await fetch(`${s.base}/metrics`)).text(), "sglang:prompt_tokens_total");
    const [a, b, c] = [await read(), await read(), await read()];
    assert.ok(b > a);
    assert.equal(c, a, "третий опрос — как после перезапуска");
  } finally {
    await s.close();
  }
});

test("--engines: серии по движкам — engine у vLLM, dp_rank у SGLang", () => {
  const vllm = liveMetrics('# TYPE vllm:num_requests_running gauge\nvllm:num_requests_running{engine="0"} 3.0\nprocess_open_fds 7.0', 0, 3, "vllm");
  assert.deepEqual(vllm.split("\n").slice(1), [
    'vllm:num_requests_running{engine="0"} 3.0',
    'vllm:num_requests_running{engine="1"} 3.0',
    'vllm:num_requests_running{engine="2"} 3.0',
    "process_open_fds 7.0",
  ]);
  const sglang = liveMetrics('sglang:num_running_reqs{tp_rank="0"} 4.0\nsglang:x 1', 0, 2, "sglang");
  assert.deepEqual(sglang.split("\n"), [
    'sglang:num_running_reqs{dp_rank="0",tp_rank="0"} 4.0',
    'sglang:num_running_reqs{dp_rank="1",tp_rank="0"} 4.0',
    'sglang:x{dp_rank="0"} 1.0',
    'sglang:x{dp_rank="1"} 1.0',
  ]);
});

test("--engines не размножает снимок, где уже несколько движков (vllm-0.10): без дублей серий", () => {
  const text = readFileSync("src-tauri/src/servers/testdata/vllm-0.10.0/metrics.txt", "utf8");
  for (const engines of [2, 3]) {
    const samples = liveMetrics(text, 1, engines, "vllm-0.10")
      .split("\n")
      .filter((l) => l && !l.startsWith("#"));
    const keys = samples.map((l) => l.slice(0, l.lastIndexOf(" ")));
    assert.equal(new Set(keys).size, keys.length, `дубли серий при engines=${engines}`);
    const ids = new Set(samples.map((l) => /engine="(\d+)"/.exec(l)?.[1]).filter((x) => x !== undefined));
    assert.deepEqual([...ids].sort(), ["0", "1"]);
  }
});

test("vllm-0.10: старые и новые имена рядом, два движка; неживые виды отдают снимок как есть", async () => {
  let s = await serve({ kind: "vllm-0.10" });
  try {
    const t = await (await fetch(`${s.base}/metrics`)).text();
    assert.match(t, /^vllm:gpu_cache_usage_perc\{engine="1"/m);
    assert.match(t, /^vllm:kv_cache_usage_perc\{engine="0"/m);
    assert.deepEqual(await (await fetch(`${s.base}/version`)).json(), { version: "0.11.0" });
  } finally {
    await s.close();
  }
  s = await serve({ kind: "vllm-0.11" });
  try {
    const [a, b] = [await (await fetch(`${s.base}/metrics`)).text(), await (await fetch(`${s.base}/metrics`)).text()];
    assert.equal(a, b);
  } finally {
    await s.close();
  }
});


const promql = (path, params) => new URL(`http://x${path}?${new URLSearchParams(params)}`);

test("prometheus: служебные ответы, цели и сопоставление — ответами настоящего Prometheus", () => {
  const body = (url) => JSON.parse(prometheusReply(url, {})[1]);
  assert.equal(body(promql("/api/v1/query", { query: "up" })).data.result.length, 4);
  const all = body(promql("/api/v1/query", { query: 'count by (job, instance) ({__name__=~"vllm:.+|sglang:.+"})' }));
  assert.equal(all.data.result.length, 4);
  const sglang = body(promql("/api/v1/query", { query: 'count by (job, instance) ({__name__=~"sglang:.+"})' }));
  assert.deepEqual(sglang.data.result.map((r) => r.metric.job), ["sglang"]);
  const one = body(promql("/api/v1/query", { query: 'count({job="vllm",__name__=~"vllm:.+|sglang:.+"})' }));
  assert.equal(one.data.result[0].value[1], "57");
  assert.deepEqual(body(promql("/api/v1/query", { query: "foo" })).data.result, []);
  const [status, bad] = prometheusReply(promql("/api/v1/query_range", { query: 'sum(x{__bad__="1"})', start: 0, end: 60, step: 15 }), {});
  assert.equal(status, 400);
  assert.equal(JSON.parse(bad).errorType, "bad_data");
});

test("prometheus: query_range — снятый ряд шаблона на сетке запроса, для любого селектора", () => {
  const expr = 'sum(rate(vllm:prompt_tokens_total{job="other",instance="10.0.0.5:8000"}[2m]))';
  const [status, text] = prometheusReply(promql("/api/v1/query_range", { query: expr, start: 1000, end: 1180, step: 60 }), {});
  assert.equal(status, 200);
  const result = JSON.parse(text).data.result;
  assert.equal(result.length, 1);
  assert.deepEqual(result[0].values.map(([t]) => t), [1000, 1060, 1120, 1180]);
  assert.ok(result[0].values.every(([, v]) => Number(v) > 0));
  const reasons = JSON.parse(prometheusReply(promql("/api/v1/query_range", {
    query: 'sum by (finished_reason) (rate(vllm:request_success_total{job="vllm"}[1m]))', start: 0, end: 30, step: 15,
  }), {})[1]).data.result;
  assert.ok(reasons.map((r) => r.metric.finished_reason).includes("stop"));
  const unknown = JSON.parse(prometheusReply(promql("/api/v1/query_range", { query: "sum(nothing)", start: 0, end: 30, step: 15 }), {})[1]);
  assert.deepEqual(unknown.data.result, []);
  // Окно «1 ч» Glafira — шаг 60 с, 61 точка: ответ всё равно больше 8 МБ.
  for (const step of [60, 15]) {
    const huge = prometheusReply(promql("/api/v1/query_range", { query: expr, start: 0, end: 3600, step }), { huge: true })[1];
    assert.ok(huge.length > 8 * 1024 * 1024, `step ${step}: ${huge.length}`);
  }
});

test("prometheus: --api-key закрывает всё, --slow задерживает ответ", async () => {
  const s = await serve({ kind: "prometheus", apiKey: "prom-t0ken", slow: 1 });
  try {
    const started = Date.now();
    assert.equal((await fetch(`${s.base}/api/v1/status/buildinfo`)).status, 401);
    assert.ok(Date.now() - started >= 900, "ответ задержан");
    const ok = await fetch(`${s.base}/api/v1/query?query=up`, { headers: { authorization: "Bearer prom-t0ken" } });
    assert.equal(ok.status, 200);
    assert.equal((await ok.json()).data.result.length, 4);
  } finally {
    await s.close();
  }
});

const DCGM = { gpus: 2, hostname: "localhost", model: null, pod: null, throttle: false, xid: 0, ecc: false };

/** Серии /metrics экспортёра DCGM: [{ name, labels, value }]. */
function series(text) {
  return text
    .split("\n")
    .filter((l) => l && !l.startsWith("#"))
    .map((l) => {
      const [, name, labels, value] = /^(\w+)\{(.*)\} (\S+)$/.exec(l);
      return { name, labels: Object.fromEntries([...labels.matchAll(/(\w+)="([^"]*)"/g)].map((m) => [m[1], m[2]])), value: Number(value) };
    });
}

test("параметры DCGM: значения по умолчанию, свои и ошибки", () => {
  assert.deepEqual(parseArgs(["--kind", "dcgm"]).dcgm, DCGM);
  assert.deepEqual(
    parseArgs(["--kind", "dcgm-full", "--gpus", "8", "--hostname", "gpu-node-1", "--model", "NVIDIA H200", "--pod", "llm/vllm-0", "--throttle", "--xid", "79", "--ecc"]).dcgm,
    { gpus: 8, hostname: "gpu-node-1", model: "NVIDIA H200", pod: "llm/vllm-0", throttle: true, xid: 79, ecc: true },
  );
  assert.equal(parseArgs(["--kind", "vllm"]).dcgm, undefined);
  assert.throws(() => parseArgs(["--kind", "vllm", "--gpus", "2"]), /только у видов dcgm/);
  assert.throws(() => parseArgs(["--kind", "dcgm", "--gpus", "73"]), /--gpus.*Использование/s);
  assert.throws(() => parseArgs(["--kind", "dcgm", "--pod", "vllm-0"]), /--pod/);
  assert.throws(() => parseArgs(["--kind", "dcgm", "--hostname"]), /--hostname/);
  assert.throws(() => parseArgs(["--kind", "dcgm", "--xid", "0"]), /--xid/);
});

test("dcgm: набор по умолчанию dcgm-exporter 4.8.4 — метки карты, без SM_ACTIVE, ECC и причин троттлинга", () => {
  const all = series(dcgmMetrics("default", DCGM, 0));
  const names = new Set(all.map((s) => s.name));
  for (const name of ["DCGM_FI_DEV_GPU_UTIL", "DCGM_FI_DEV_FB_USED", "DCGM_FI_PROF_GR_ENGINE_ACTIVE", "DCGM_FI_DEV_XID_ERRORS", "DCGM_FI_DEV_PCIE_REPLAY_COUNTER"])
    assert.ok(names.has(name), name);
  for (const name of ["DCGM_FI_PROF_SM_ACTIVE", "DCGM_FI_DEV_ECC_DBE_VOL_TOTAL", "DCGM_FI_DEV_CLOCKS_EVENT_REASONS", "DCGM_EXP_XID_ERRORS_COUNT"])
    assert.ok(!names.has(name), name);
  const util = all.filter((s) => s.name === "DCGM_FI_DEV_GPU_UTIL");
  assert.deepEqual(util.map((s) => s.labels.gpu), ["0", "1"]);
  assert.deepEqual(Object.keys(util[0].labels), ["gpu", "UUID", "pci_bus_id", "device", "modelName", "hostname", "DCGM_FI_DRIVER_VERSION"]);
  assert.equal(util[0].labels.modelName, "NVIDIA H100 80GB HBM3");
  const mem = (name) => all.find((s) => s.name === name && s.labels.gpu === "0").value;
  assert.equal(mem("DCGM_FI_DEV_FB_USED") + mem("DCGM_FI_DEV_FB_FREE") + mem("DCGM_FI_DEV_FB_RESERVED"), 81559);
});

test("dcgm-full, dcgm-4.1, dcgm-mig и --pod: дополнительные серии, метка Hostname, экземпляры MIG, метки Kubernetes", () => {
  const full = new Set(series(dcgmMetrics("full", DCGM, 0)).map((s) => s.name));
  for (const name of ["DCGM_FI_PROF_SM_ACTIVE", "DCGM_FI_DEV_ECC_SBE_VOL_TOTAL", "DCGM_FI_DEV_ECC_DBE_VOL_TOTAL", "DCGM_FI_DEV_CLOCKS_EVENT_REASONS", "DCGM_FI_DEV_POWER_VIOLATION", "DCGM_FI_DEV_NVLINK_CRC_ERROR_TOTAL"])
    assert.ok(full.has(name), name);
  const old = series(dcgmMetrics("4.1", { ...DCGM, hostname: "gpu-old" }, 0))[0];
  assert.equal(old.labels.Hostname, "gpu-old");
  assert.equal(old.labels.hostname, undefined);
  assert.equal(old.labels.modelName, "NVIDIA A100-SXM4-80GB");
  const mig = series(dcgmMetrics("mig", DCGM, 0));
  assert.deepEqual(mig.filter((s) => s.name === "DCGM_FI_DEV_GPU_UTIL").map((s) => s.labels.gpu), ["1"], "у карты MIG нет утилизации");
  const used = mig.filter((s) => s.name === "DCGM_FI_DEV_FB_USED" && s.labels.gpu === "0");
  assert.deepEqual(used.map((s) => [s.labels.GPU_I_PROFILE, s.labels.GPU_I_ID]), [["3g.40gb", "1"], ["3g.40gb", "2"]]);
  assert.ok(mig.some((s) => s.name === "DCGM_FI_DEV_GPU_TEMP" && s.labels.gpu === "0" && !s.labels.GPU_I_ID), "температура — у карты");
  const pod = series(dcgmMetrics("default", { ...DCGM, pod: "llm/vllm-0" }, 0))[0].labels;
  assert.deepEqual([pod.container, pod.namespace, pod.pod], ["vllm", "llm", "vllm-0"]);
});

test("dcgm: датчики меняются, счётчики растут; --throttle, --xid и --ecc включают проблемы", () => {
  const at = (kind, o, poll, name, gpu = "0") => series(dcgmMetrics(kind, o, poll)).find((s) => s.name === name && s.labels.gpu === gpu)?.value;
  assert.notEqual(at("default", DCGM, 0, "DCGM_FI_DEV_GPU_UTIL"), at("default", DCGM, 2, "DCGM_FI_DEV_GPU_UTIL"));
  assert.ok(at("default", DCGM, 3, "DCGM_FI_DEV_TOTAL_ENERGY_CONSUMPTION") > at("default", DCGM, 1, "DCGM_FI_DEV_TOTAL_ENERGY_CONSUMPTION"));
  assert.equal(at("full", DCGM, 5, "DCGM_FI_DEV_CLOCKS_EVENT_REASONS"), 0);
  assert.equal(at("full", DCGM, 5, "DCGM_FI_DEV_XID_ERRORS"), 0);
  const bad = { ...DCGM, throttle: true, xid: 79, ecc: true };
  assert.equal(at("full", bad, 5, "DCGM_FI_DEV_CLOCKS_EVENT_REASONS"), 0x48, "HW_SLOWDOWN | HW_THERMAL");
  assert.ok(at("full", bad, 5, "DCGM_FI_DEV_PCIE_REPLAY_COUNTER") > at("full", bad, 2, "DCGM_FI_DEV_PCIE_REPLAY_COUNTER"));
  assert.ok(at("full", bad, 5, "DCGM_FI_DEV_NVLINK_CRC_ERROR_TOTAL") > 0);
  assert.equal(at("full", bad, 5, "DCGM_FI_DEV_XID_ERRORS"), 79);
  assert.equal(at("full", bad, 5, "DCGM_FI_DEV_XID_ERRORS", "1"), 0, "проблемы — только у карты 0");
  assert.equal(at("full", bad, 5, "DCGM_FI_DEV_ECC_DBE_VOL_TOTAL"), 2);
  assert.equal(at("full", bad, 5, "DCGM_EXP_XID_ERRORS_COUNT"), 1);
  assert.equal(at("default", bad, 5, "DCGM_EXP_XID_ERRORS_COUNT"), undefined, "в наборе по умолчанию счётчика XID нет");
});

test("dcgm: /metrics по опросам; --api-key — пароль basic с логином glafira", async () => {
  const s = await serve({ kind: "dcgm", dcgm: { ...DCGM, gpus: 3 }, apiKey: "dcgm-pa55" });
  try {
    assert.equal((await fetch(`${s.base}/metrics`)).status, 401);
    assert.equal((await fetch(`${s.base}/metrics`, { headers: { authorization: "Bearer dcgm-pa55" } })).status, 401);
    const auth = { authorization: `Basic ${Buffer.from("glafira:dcgm-pa55").toString("base64")}` };
    const r = await fetch(`${s.base}/metrics`, { headers: auth });
    assert.equal(r.status, 200);
    assert.match(r.headers.get("content-type"), /text\/plain/);
    const util = series(await r.text()).filter((x) => x.name === "DCGM_FI_DEV_GPU_UTIL");
    assert.equal(util.length, 3);
    assert.equal((await fetch(`${s.base}/nope`, { headers: auth })).status, 404);
  } finally {
    await s.close();
  }
});
