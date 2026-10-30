import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createServer } from "node:http";
import { test } from "node:test";
import { handler, KINDS, liveMetrics, parseArgs } from "./mock-server.mjs";

async function serve(opts) {
  const server = createServer(handler({ health: 200, apiKey: null, ...opts }));
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const base = `http://127.0.0.1:${server.address().port}`;
  return { base, close: () => new Promise((resolve) => server.close(resolve)) };
}

test("параметры: значения по умолчанию, свои и ошибки", () => {
  const defaults = { kind: "vllm", port: 8000, host: "127.0.0.1", apiKey: null, health: 200, models: "full", engines: 1, resetAfter: 0 };
  assert.deepEqual(parseArgs([]), defaults);
  assert.deepEqual(
    parseArgs(["--kind", "sglang", "--port", "30000", "--api-key", "sk", "--health", "503", "--models", "empty", "--engines", "2", "--reset-after", "3"]),
    { ...defaults, kind: "sglang", port: 30000, apiKey: "sk", health: 503, models: "empty", engines: 2, resetAfter: 3 },
  );
  assert.throws(() => parseArgs(["--engines", "9"]), /--engines.*Использование/s);
  assert.throws(() => parseArgs(["--reset-after", "0"]), /--reset-after/);
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
  for (const kind of Object.keys(KINDS)) {
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
