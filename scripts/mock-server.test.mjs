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
  assert.deepEqual(parseArgs([]), { kind: "vllm", port: 8000, host: "127.0.0.1", apiKey: null, health: 200, models: "full" });
  assert.deepEqual(
    parseArgs(["--kind", "sglang", "--port", "30000", "--api-key", "sk", "--health", "503", "--models", "empty"]),
    { kind: "sglang", port: 30000, host: "127.0.0.1", apiKey: "sk", health: 503, models: "empty" },
  );
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
    assert.deepEqual(await (await fetch(`${s.base}/version`)).json(), { version: "0.11.0" });
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
