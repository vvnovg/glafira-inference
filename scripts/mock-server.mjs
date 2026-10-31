#!/usr/bin/env node
// Мок-сервер инференса для разработки и смоука без GPU: отвечает как vLLM, SGLang или OpenAI-совместимый
// сервер снимками ответов из src-tauri/src/servers/testdata (те же снимки проверяют тесты Rust).
// Только модули node:. Использование:
//   node scripts/mock-server.mjs --kind vllm|vllm-0.11|vllm-0.10|vllm-0.8|vllm-old|sglang|sglang-no-metrics|sglang-0.5.4|sglang-0.4|llamacpp|ollama
//        [--port 8000] [--host 127.0.0.1] [--api-key sk-dev]
//        [--health 200..599] [--models full|empty] [--engines 1..8] [--reset-after N]
//   --port    целое 0..65535 (0 — любой свободный порт).
//   --health  код ответа /health, любое целое 200..599 (например 200, 301, 401, 503);
//             у 3xx добавляется заголовок Location на https://<хост>:<порт>/health.
//   --models  full — снимок /v1/models (по умолчанию), empty — {"object":"list","data":[]}.
//   /metrics видов vllm, vllm-0.10 и sglang «живые»: счётчики и гистограммы растут на 10 % снимка за опрос,
//   KV-кэш колеблется. --engines N — N движков data parallel (vLLM — метка engine, SGLang — dp_rank;
//   снимок, где уже несколько движков, например vllm-0.10, не размножается);
//   --reset-after N — после N опросов /metrics счётчики начинаются заново, как при перезапуске сервера.
import { readFileSync } from "node:fs";
import { createServer } from "node:http";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const DATA = join(fileURLToPath(new URL("..", import.meta.url)), "src-tauri/src/servers/testdata");

const USAGE =
  "Использование: node scripts/mock-server.mjs --kind <вид> [--port 0..65535] [--host 127.0.0.1] [--api-key <ключ>] [--health 200..599] [--models full|empty] [--engines 1..8] [--reset-after N]";

/** vLLM (AuthenticationMiddleware): ключ нужен только путям с префиксом /v1, остальное (/version, /metrics, /health) открыто.
 *  Так же отвечают llama.cpp, Ollama и прочие OpenAI-совместимые виды: ключ защищает только /v1. */
const openOutsideV1 = (path) => !path.startsWith("/v1");
/** SGLang (add_api_key_middleware): без ключа открыты только пути с префиксами /health и /metrics;
 *  /server_info, /model_info, /get_* и /v1/* отвечают 401. */
const openHealthAndMetrics = (path) => path.startsWith("/health") || path.startsWith("/metrics");

/** Пути сервера каждого вида → файл снимка (каталог testdata/<вид>); open(путь) — путь открыт без ключа. */
export const KINDS = {
  vllm: {
    dir: "vllm-0.32.0",
    routes: { "/version": "version.json", "/v1/models": "models.json", "/load": "load.json", "/metrics": "metrics-after.txt" },
    open: openOutsideV1,
    live: true,
  },
  "vllm-0.11": {
    dir: "vllm-0.11.0",
    routes: { "/version": "version.json", "/v1/models": "models.json", "/metrics": "metrics.txt" },
    open: openOutsideV1,
  },
  // Два имени рядом (gpu_cache_usage_perc и kv_cache_usage_perc), два движка; /version и /v1/models — от 0.11.
  "vllm-0.10": {
    dir: "vllm-0.10.0",
    routes: { "/version": "../vllm-0.11.0/version.json", "/v1/models": "../vllm-0.11.0/models.json", "/metrics": "metrics.txt" },
    open: openOutsideV1,
    live: true,
  },
  "vllm-0.8": {
    dir: "vllm-0.8.5",
    routes: { "/version": "version.json", "/v1/models": "models.json", "/metrics": "metrics.txt" },
    open: openOutsideV1,
  },
  "vllm-old": {
    dir: "vllm-0.7.3",
    routes: { "/version": "version.json", "/v1/models": "models.json", "/metrics": "metrics.txt" },
    open: openOutsideV1,
  },
  sglang: {
    dir: "sglang-0.5.22",
    routes: {
      "/model_info": "model_info.json",
      "/server_info": "server_info.json",
      "/v1/models": "models.json",
      "/metrics": "metrics.txt",
    },
    open: openHealthAndMetrics,
    live: true,
  },
  // SGLang 0.5.22 без --enable-metrics: пути сведений как у `sglang`, а /metrics нет (404).
  "sglang-no-metrics": {
    dir: "sglang-0.5.22",
    routes: { "/model_info": "model_info.json", "/server_info": "server_info.json", "/v1/models": "models.json" },
    open: openHealthAndMetrics,
  },
  "sglang-0.5.4": {
    dir: "sglang-0.5.4",
    routes: {
      "/model_info": "model_info.json",
      "/get_model_info": "model_info.json",
      "/server_info": "server_info.json",
      "/get_server_info": "server_info.json",
      "/v1/models": "models.json",
      "/metrics": "metrics.txt",
    },
    open: openHealthAndMetrics,
  },
  // SGLang 0.4 без --enable-metrics: только устаревшие пути сведений, /metrics нет.
  "sglang-0.4": {
    dir: "sglang-0.4.6",
    routes: { "/get_model_info": "model_info.json", "/get_server_info": "server_info.json", "/v1/models": "models.json" },
    open: openHealthAndMetrics,
  },
  llamacpp: { dir: "llamacpp", routes: { "/v1/models": "models.json" }, open: openOutsideV1 },
  // Ollama: /health нет.
  ollama: { dir: "ollama", routes: { "/v1/models": "models.json" }, noHealth: true, open: openOutsideV1 },
};

/** Целое из значения параметра в пределах [min, max]; иначе ошибка с текстом использования. */
function intArg(name, value, min, max) {
  if (typeof value === "string" && /^\d+$/.test(value) && Number(value) >= min && Number(value) <= max) return Number(value);
  throw new Error(`${name}: нужно целое от ${min} до ${max}, получено ${value === undefined ? "ничего" : `«${value}»`}\n${USAGE}`);
}

export function parseArgs(argv) {
  const fail = (message) => {
    throw new Error(`${message}\n${USAGE}`);
  };
  const opts = { kind: "vllm", port: 8000, host: "127.0.0.1", apiKey: null, health: 200, models: "full", engines: 1, resetAfter: 0 };
  for (let i = 0; i < argv.length; i++) {
    const [name, value] = [argv[i], argv[i + 1]];
    if (name === "--kind") opts.kind = value;
    else if (name === "--port") opts.port = intArg(name, value, 0, 65535);
    else if (name === "--host") opts.host = value;
    else if (name === "--api-key") opts.apiKey = value;
    else if (name === "--health") opts.health = intArg(name, value, 200, 599);
    else if (name === "--engines") opts.engines = intArg(name, value, 1, 8);
    else if (name === "--reset-after") opts.resetAfter = intArg(name, value, 1, 1_000_000);
    else if (name === "--models") {
      if (value !== "full" && value !== "empty") fail(`--models: нужно full или empty, получено ${value === undefined ? "ничего" : `«${value}»`}`);
      opts.models = value;
    } else fail(`неизвестный параметр ${name}`);
    i++;
  }
  if (!KINDS[opts.kind]) fail(`неизвестный вид ${opts.kind}: ${Object.keys(KINDS).join(", ")}`);
  return opts;
}

/** «Живой» /metrics: опрос `poll` (с 0) растит счётчики и гистограммы на 10 % снимка за опрос,
 *  KV-кэш колеблется; `engines` > 1 размножает серии по движкам. Если в снимке уже несколько движков
 *  (vLLM — engine, отличный от "0"; SGLang — dp_rank), `engines` игнорируется. Целые значения пишутся
 *  с `.0`, как у Python prometheus_client. */
export function liveMetrics(text, poll, engines, kind) {
  if (/engine="(?!0")[^"]*"|dp_rank="/.test(text)) engines = 1;
  const types = new Map();
  for (const m of text.matchAll(/^# TYPE (\S+) (\S+)$/gm)) types.set(m[1], m[2]);
  const typeOf = (name) =>
    types.get(name) ?? types.get(name.replace(/_(bucket|sum|count)$/, "")) ?? types.get(name.replace(/_total$/, ""));
  const grow = 1 + 0.1 * poll;
  const fmt = (v) => (Number.isInteger(v) ? v.toFixed(1) : String(Number(v.toFixed(6))));
  const out = [];
  for (const line of text.split("\n")) {
    const m = /^([a-zA-Z_:][\w:]*)(\{[^}]*\})? (\S+)$/.exec(line);
    if (!m) {
      out.push(line);
      continue;
    }
    const [, name, labels = "", raw] = m;
    let v = Number(raw);
    const type = typeOf(name);
    if (Number.isFinite(v) && (type === "counter" || type === "histogram")) v *= grow;
    if (/(kv_cache_usage_perc|gpu_cache_usage_perc|token_usage)$/.test(name)) v = Number((0.2 + 0.6 * Math.abs(Math.sin(poll / 3))).toFixed(4));
    const value = Number.isFinite(v) ? fmt(v) : raw;
    for (let e = 0; e < engines; e++) {
      if (engines > 1 && labels.includes('engine="0"')) out.push(`${name}${labels.replace('engine="0"', `engine="${e}"`)} ${value}`);
      else if (engines > 1 && kind.startsWith("sglang") && name.startsWith("sglang:"))
        out.push(`${name}${labels ? labels.replace("{", `{dp_rank="${e}",`) : `{dp_rank="${e}"}`} ${value}`);
      else if (e === 0) out.push(`${name}${labels} ${value}`);
    }
  }
  return out.join("\n");
}

/** Обработчик запросов: ключ проверяется по правилу вида (kind.open); /health отвечает заданным кодом,
 *  у 3xx — с Location на https, как у настоящего перенаправления. */
export function handler(opts) {
  const kind = KINDS[opts.kind];
  let polls = 0;
  return (req, res) => {
    const path = new URL(req.url ?? "/", "http://x").pathname;
    const reply = (status, type, body, headers = {}) => {
      res.writeHead(status, { "content-type": type, ...headers });
      res.end(body);
    };
    if (opts.apiKey && !kind.open(path) && req.headers.authorization !== `Bearer ${opts.apiKey}`)
      return reply(401, "application/json", JSON.stringify({ error: "Unauthorized" }));
    if (path === "/health" && !kind.noHealth) {
      const redirect = opts.health >= 300 && opts.health < 400;
      const authority = req.headers.host ?? `${opts.host}:${opts.port}`;
      return reply(opts.health, "text/plain", "", redirect ? { location: `https://${authority}/health` } : {});
    }
    const file = kind.routes[path];
    if (!file) return reply(404, "application/json", JSON.stringify({ detail: "Not Found" }));
    // --models empty: сервер жив и отвечает, но модель не загружена — список пуст.
    if (path === "/v1/models" && opts.models === "empty")
      return reply(200, "application/json", JSON.stringify({ object: "list", data: [] }));
    let body = readFileSync(join(DATA, kind.dir, file));
    if (path === "/metrics" && kind.live) {
      if (opts.resetAfter && polls >= opts.resetAfter) polls = 0;
      body = liveMetrics(body.toString("utf8"), polls++, opts.engines ?? 1, opts.kind);
    }
    reply(200, file.endsWith(".txt") ? "text/plain; version=0.0.4" : "application/json", body);
  };
}

function main() {
  const opts = parseArgs(process.argv.slice(2));
  const server = createServer(handler(opts));
  // Занятый порт, неизвестный хост и прочие сбои listen — сообщением и кодом 1, без стека.
  server.on("error", (e) => {
    console.error(`✗ мок ${opts.kind}: не удалось занять ${opts.host}:${opts.port}: ${e.message}`);
    process.exit(1);
  });
  server.listen(opts.port, opts.host, () => {
    const key = opts.apiKey ? `, ключ API ${opts.apiKey}` : "";
    console.log(`✓ мок ${opts.kind}: http://${opts.host}:${opts.port}${key}`);
  });
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try {
    main();
  } catch (e) {
    console.error(`✗ ${e.message}`);
    process.exit(1);
  }
}
