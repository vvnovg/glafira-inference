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
