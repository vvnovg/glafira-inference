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
