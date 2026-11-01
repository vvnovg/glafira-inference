#!/usr/bin/env node
// Мок-сервер инференса для разработки и смоука без GPU: отвечает как vLLM, SGLang или OpenAI-совместимый
// сервер снимками ответов из src-tauri/src/servers/testdata (те же снимки проверяют тесты Rust), а вид
// prometheus — как источник истории, ответами настоящего Prometheus из src-tauri/src/history/testdata, виды dcgm* —
// как экспортёр DCGM (формат dcgm-exporter 4.8.4, собран по его исходникам).
// Только модули node:. Использование:
//   node scripts/mock-server.mjs --kind vllm|vllm-0.11|vllm-0.10|vllm-0.8|vllm-old|sglang|sglang-no-metrics|sglang-0.5.4|sglang-0.4|llamacpp|ollama|prometheus|dcgm|dcgm-full|dcgm-4.1|dcgm-mig
//        [--port 8000] [--host 127.0.0.1] [--api-key sk-dev]
//        [--health 200..599] [--models full|empty] [--engines 1..8] [--reset-after N] [--slow N] [--huge]
//        [--gpus 1..72] [--hostname <имя>] [--model <модель>] [--pod <namespace>/<pod>] [--throttle] [--xid <код>] [--ecc]
//   --port    целое 0..65535 (0 — любой свободный порт).
//   --health  код ответа /health, любое целое 200..599 (например 200, 301, 401, 503);
//             у 3xx добавляется заголовок Location на https://<хост>:<порт>/health.
//   --models  full — снимок /v1/models (по умолчанию), empty — {"object":"list","data":[]}.
//   /metrics видов vllm, vllm-0.10 и sglang «живые»: счётчики и гистограммы растут на 10 % снимка за опрос,
//   KV-кэш колеблется. --engines N — N движков data parallel (vLLM — метка engine, SGLang — dp_rank;
//   снимок, где уже несколько движков, например vllm-0.10, не размножается);
//   --reset-after N — после N опросов /metrics счётчики начинаются заново, как при перезапуске сервера.
//   Вид prometheus (смоук отказов источника истории): --api-key — токен (без него 401), --slow N — ответ через
//   N секунд (таймаут), --huge — ответ query_range больше 8 МБ; выражение с __bad__ — ошибка bad_data.
//   query_range отдаёт снятые ряды шаблонов Glafira на сетке запроса (значения по кругу), чужие — пусто.
//   Виды dcgm (набор счётчиков по умолчанию), dcgm-full (плюс SM_ACTIVE, ECC, причины троттлинга, ошибки NVLink),
//   dcgm-4.1 (метка Hostname, A100), dcgm-mig (карта 0 с двумя экземплярами MIG): --gpus — число карт (2),
//   --hostname — хост в метках (localhost), --model — модель, --pod — метки Kubernetes, --throttle — троттлинг
//   и ошибки NVLink/PCIe, --xid — код XID у карты 0, --ecc — ошибки ECC у карты 0; --api-key — пароль basic
//   (логин glafira). Датчики колеблются, счётчики растут с каждым опросом.
import { Buffer } from "node:buffer";
import { readFileSync } from "node:fs";
import { createServer } from "node:http";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const DATA = join(ROOT, "src-tauri/src/servers/testdata");
const PROMETHEUS = join(ROOT, "src-tauri/src/history/testdata/prometheus-3.15.1");

const USAGE =
  "Использование: node scripts/mock-server.mjs --kind <вид> [--port 0..65535] [--host 127.0.0.1] [--api-key <ключ>] [--health 200..599] [--models full|empty] [--engines 1..8] [--reset-after N] [--slow N] [--huge] [--gpus 1..72] [--hostname <имя>] [--model <модель>] [--pod <namespace>/<pod>] [--throttle] [--xid <код>] [--ecc]";

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
  // Источник истории: с --api-key закрыто всё (вход по токену); /api/v1/query и query_range — prometheusReply.
  prometheus: {
    dir: "../../history/testdata/prometheus-3.15.1",
    routes: { "/api/v1/status/buildinfo": "buildinfo.json", "/api/v1/targets": "targets.json" },
    open: () => false,
    noHealth: true,
    prometheus: true,
  },
  // Экспортёры DCGM: /metrics собирается на лету (dcgmMetrics), вход — basic с логином glafira.
  dcgm: { dcgm: "default", routes: {}, open: () => false, noHealth: true },
  "dcgm-full": { dcgm: "full", routes: {}, open: () => false, noHealth: true },
  "dcgm-4.1": { dcgm: "4.1", routes: {}, open: () => false, noHealth: true },
  "dcgm-mig": { dcgm: "mig", routes: {}, open: () => false, noHealth: true },
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
  const opts = { kind: "vllm", port: 8000, host: "127.0.0.1", apiKey: null, health: 200, models: "full", engines: 1, resetAfter: 0, slow: 0, huge: false };
  const dcgm = { gpus: 2, hostname: "localhost", model: null, pod: null, throttle: false, xid: 0, ecc: false };
  let dcgmSet = false;
  for (let i = 0; i < argv.length; i++) {
    const [name, value] = [argv[i], argv[i + 1]];
    if (name === "--huge") {
      opts.huge = true;
      continue;
    }
    if (name === "--throttle" || name === "--ecc") {
      dcgm[name.slice(2)] = true;
      dcgmSet = true;
      continue;
    }
    if (["--gpus", "--hostname", "--model", "--pod", "--xid"].includes(name)) {
      if (name === "--gpus") dcgm.gpus = intArg(name, value, 1, 72);
      else if (name === "--xid") dcgm.xid = intArg(name, value, 1, 1000);
      else if (name === "--pod" && !/^[^/\s]+\/[^/\s]+$/.test(value ?? "")) fail(`--pod: нужно <namespace>/<pod>, получено ${value === undefined ? "ничего" : `«${value}»`}`);
      else if (value === undefined || value === "") fail(`${name}: значение не указано`);
      else dcgm[name.slice(2)] = value;
      dcgmSet = true;
      i++;
      continue;
    }
    if (name === "--kind") opts.kind = value;
    else if (name === "--port") opts.port = intArg(name, value, 0, 65535);
    else if (name === "--host") opts.host = value;
    else if (name === "--api-key") opts.apiKey = value;
    else if (name === "--health") opts.health = intArg(name, value, 200, 599);
    else if (name === "--engines") opts.engines = intArg(name, value, 1, 8);
    else if (name === "--reset-after") opts.resetAfter = intArg(name, value, 1, 1_000_000);
    else if (name === "--slow") opts.slow = intArg(name, value, 1, 600);
    else if (name === "--models") {
      if (value !== "full" && value !== "empty") fail(`--models: нужно full или empty, получено ${value === undefined ? "ничего" : `«${value}»`}`);
      opts.models = value;
    } else fail(`неизвестный параметр ${name}`);
    i++;
  }
  if (!KINDS[opts.kind]) fail(`неизвестный вид ${opts.kind}: ${Object.keys(KINDS).join(", ")}`);
  if (KINDS[opts.kind].dcgm) opts.dcgm = dcgm;
  else if (dcgmSet) fail(`параметры --gpus, --hostname, --model, --pod, --throttle, --xid и --ecc — только у видов dcgm`);
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

/** Серии набора счётчиков по умолчанию dcgm-exporter 4.8.4 (etc/default-counters.csv): имя, тип, справка. */
const DCGM_DEFAULT = [
  ["DCGM_FI_DEV_SM_CLOCK", "gauge", "SM clock frequency (in MHz)."],
  ["DCGM_FI_DEV_MEM_CLOCK", "gauge", "Memory clock frequency (in MHz)."],
  ["DCGM_FI_DEV_MEMORY_TEMP", "gauge", "Memory temperature (in C)."],
  ["DCGM_FI_DEV_GPU_TEMP", "gauge", "GPU temperature (in C)."],
  ["DCGM_FI_DEV_POWER_USAGE", "gauge", "Power draw (in W)."],
  ["DCGM_FI_DEV_TOTAL_ENERGY_CONSUMPTION", "counter", "Total energy consumption since the driver was last reloaded (in mJ)."],
  ["DCGM_FI_DEV_PCIE_REPLAY_COUNTER", "counter", "Total number of PCIe retries."],
  ["DCGM_FI_DEV_GPU_UTIL", "gauge", "GPU utilization (in %)."],
  ["DCGM_FI_DEV_MEM_COPY_UTIL", "gauge", "Memory utilization (in %)."],
  ["DCGM_FI_DEV_ENC_UTIL", "gauge", "Encoder utilization (in %)."],
  ["DCGM_FI_DEV_DEC_UTIL", "gauge", "Decoder utilization (in %)."],
  ["DCGM_FI_DEV_XID_ERRORS", "gauge", "Value of the last XID error encountered."],
  ["DCGM_FI_DEV_FB_FREE", "gauge", "Framebuffer memory free (in MiB)."],
  ["DCGM_FI_DEV_FB_USED", "gauge", "Framebuffer memory used (in MiB)."],
  ["DCGM_FI_DEV_FB_RESERVED", "gauge", "Framebuffer memory reserved (in MiB)."],
  ["DCGM_FI_DEV_NVLINK_BANDWIDTH_TOTAL", "gauge", "Aggregate NVLink throughput across all lanes (in MB/s)."],
  ["DCGM_FI_DEV_VGPU_LICENSE_STATUS", "gauge", "vGPU License status"],
  ["DCGM_FI_DEV_UNCORRECTABLE_REMAPPED_ROWS", "counter", "Number of remapped rows for uncorrectable errors"],
  ["DCGM_FI_DEV_CORRECTABLE_REMAPPED_ROWS", "counter", "Number of remapped rows for correctable errors"],
  ["DCGM_FI_DEV_ROW_REMAP_FAILURE", "gauge", "Whether remapping of rows has failed"],
  ["DCGM_FI_PROF_GR_ENGINE_ACTIVE", "gauge", "Ratio of time the graphics engine is active."],
  ["DCGM_FI_PROF_PIPE_TENSOR_ACTIVE", "gauge", "Ratio of cycles the tensor (HMMA) pipe is active."],
  ["DCGM_FI_PROF_DRAM_ACTIVE", "gauge", "Ratio of cycles the device memory interface is active sending or receiving data."],
  ["DCGM_FI_PROF_PCIE_TX_BYTES", "gauge", "The rate of data transmitted over the PCIe bus - including both protocol headers and data payloads - in bytes per second."],
  ["DCGM_FI_PROF_PCIE_RX_BYTES", "gauge", "The rate of data received over the PCIe bus - including both protocol headers and data payloads - in bytes per second."],
];

/** Серии, которые набор по умолчанию не собирает (закомментированы в default-counters.csv), — у вида dcgm-full. */
const DCGM_EXTRA = [
  ["DCGM_FI_PROF_SM_ACTIVE", "gauge", "The ratio of cycles an SM has at least 1 warp assigned."],
  ["DCGM_FI_DEV_ECC_SBE_VOL_TOTAL", "counter", "Total number of single-bit volatile ECC errors."],
  ["DCGM_FI_DEV_ECC_DBE_VOL_TOTAL", "counter", "Total number of double-bit volatile ECC errors."],
  ["DCGM_FI_DEV_CLOCKS_EVENT_REASONS", "gauge", "Current clock event reasons (bitmask of DCGM_CLOCKS_EVENT_REASON_*)."],
  ["DCGM_FI_DEV_POWER_VIOLATION", "counter", "Throttling duration due to power constraints (in ns)."],
  ["DCGM_FI_DEV_THERMAL_VIOLATION", "counter", "Throttling duration due to thermal constraints (in ns)."],
  ["DCGM_FI_DEV_NVLINK_CRC_ERROR_TOTAL", "counter", "Total number of NVLink data-link CRC errors."],
  ["DCGM_FI_DEV_NVLINK_RECOVERY_TOTAL", "counter", "Total number of NVLink data-link recovery errors."],
  ["DCGM_FI_DEV_NVLINK_REPLAY_TOTAL", "counter", "Total number of NVLink data-link replay errors."],
];

/** Серии, которые есть у экземпляра MIG (у карты в режиме MIG нет GPU_UTIL). */
const MIG_FIELDS = new Set(["DCGM_FI_DEV_FB_FREE", "DCGM_FI_DEV_FB_USED", "DCGM_FI_DEV_FB_RESERVED", "DCGM_FI_PROF_GR_ENGINE_ACTIVE", "DCGM_FI_PROF_PIPE_TENSOR_ACTIVE", "DCGM_FI_PROF_DRAM_ACTIVE"]);

const DCGM_MODELS = { default: "NVIDIA H100 80GB HBM3", full: "NVIDIA H100 80GB HBM3", "4.1": "NVIDIA A100-SXM4-80GB", mig: "NVIDIA A100-SXM4-80GB" };

/** Значение поля карты `gpu` на опросе `poll`: датчики колеблются, счётчики растут с каждым опросом. */
function dcgmValue(name, gpu, poll, o) {
  const wave = (period, phase = 0) => Math.abs(Math.sin((poll + gpu * 1.7 + phase) / period));
  const total = 81559;
  const used = Math.round(52000 + 20000 * wave(5));
  switch (name) {
    case "DCGM_FI_DEV_SM_CLOCK": return o.throttle ? 1095 : 1980;
    case "DCGM_FI_DEV_MEM_CLOCK": return 2619;
    case "DCGM_FI_DEV_MEMORY_TEMP": return Math.round(48 + 10 * wave(6));
    case "DCGM_FI_DEV_GPU_TEMP": return o.throttle ? 89 : Math.round(45 + 25 * wave(6));
    case "DCGM_FI_DEV_POWER_USAGE": return Number((180 + 450 * wave(4)).toFixed(3));
    case "DCGM_FI_DEV_TOTAL_ENERGY_CONSUMPTION": return 9_000_000_000 + gpu * 1_000_000 + poll * 2_500_000;
    case "DCGM_FI_DEV_PCIE_REPLAY_COUNTER": return o.throttle ? poll * 3 : 0;
    case "DCGM_FI_DEV_GPU_UTIL": return Math.round(40 + 55 * wave(4));
    case "DCGM_FI_DEV_MEM_COPY_UTIL": return Math.round(20 + 50 * wave(4, 1));
    case "DCGM_FI_DEV_UNCORRECTABLE_REMAPPED_ROWS": return o.ecc && gpu === 0 ? 1 : 0;
    case "DCGM_FI_DEV_XID_ERRORS": return o.xid && gpu === 0 ? o.xid : 0;
    case "DCGM_FI_DEV_FB_USED": return used;
    case "DCGM_FI_DEV_FB_RESERVED": return 541;
    case "DCGM_FI_DEV_FB_FREE": return total - used - 541;
    case "DCGM_FI_DEV_NVLINK_BANDWIDTH_TOTAL": return Math.round(30000 + 90000 * wave(5, 2));
    case "DCGM_FI_PROF_GR_ENGINE_ACTIVE": return Number((0.4 + 0.55 * wave(4)).toFixed(6));
    case "DCGM_FI_PROF_SM_ACTIVE": return Number((0.35 + 0.55 * wave(4)).toFixed(6));
    case "DCGM_FI_PROF_PIPE_TENSOR_ACTIVE": return Number((0.1 + 0.4 * wave(4)).toFixed(6));
    case "DCGM_FI_PROF_DRAM_ACTIVE": return Number((0.2 + 0.5 * wave(4, 1)).toFixed(6));
    case "DCGM_FI_PROF_PCIE_TX_BYTES": return Math.round(2e8 + 6e8 * wave(3));
    case "DCGM_FI_PROF_PCIE_RX_BYTES": return Math.round(5e8 + 9e8 * wave(3, 1));
    case "DCGM_FI_DEV_ECC_SBE_VOL_TOTAL": return o.ecc && gpu === 0 ? 12 + poll : 0;
    case "DCGM_FI_DEV_ECC_DBE_VOL_TOTAL": return o.ecc && gpu === 0 ? 2 : 0;
    // HW_SLOWDOWN (0x8) | HW_THERMAL (0x40).
    case "DCGM_FI_DEV_CLOCKS_EVENT_REASONS": return o.throttle ? 0x48 : 0;
    case "DCGM_FI_DEV_POWER_VIOLATION": return o.throttle ? poll * 2_000_000_000 : 0;
    case "DCGM_FI_DEV_THERMAL_VIOLATION": return o.throttle ? poll * 3_000_000_000 : 0;
    case "DCGM_FI_DEV_NVLINK_CRC_ERROR_TOTAL":
    case "DCGM_FI_DEV_NVLINK_RECOVERY_TOTAL":
    case "DCGM_FI_DEV_NVLINK_REPLAY_TOTAL":
      return o.throttle ? poll : 0;
    default: return 0;
  }
}

/** /metrics экспортёра DCGM в формате dcgm-exporter (internal/pkg/rendermetrics): метки карты gpu, UUID,
 *  pci_bus_id, device, modelName, у MIG — GPU_I_PROFILE, GPU_I_ID; хост — hostname (4.8) или Hostname (до 4.1);
 *  версия драйвера — меткой DCGM_FI_DRIVER_VERSION; --pod добавляет метки Kubernetes. */
export function dcgmMetrics(kind, o, poll) {
  const fields = kind === "full" ? [...DCGM_DEFAULT, ...DCGM_EXTRA] : DCGM_DEFAULT;
  const model = o.model ?? DCGM_MODELS[kind];
  const hostLabel = kind === "4.1" || kind === "mig" ? "Hostname" : "hostname";
  const [namespace, pod] = o.pod ? o.pod.split("/") : [];
  const labels = (gpu, mig) => {
    const hex = gpu.toString(16).padStart(2, "0");
    const parts = [
      `gpu="${gpu}"`,
      `UUID="GPU-6c2d1a3e-${hex}00-4b1c-9f3e-0a1b2c3d4e${hex}"`,
      `pci_bus_id="00000000:${(0x18 + gpu * 0x10).toString(16).toUpperCase().padStart(2, "0")}:00.0"`,
      `device="nvidia${gpu}"`,
      `modelName="${model}"`,
    ];
    if (mig) parts.push(`GPU_I_PROFILE="3g.40gb"`, `GPU_I_ID="${mig}"`);
    parts.push(`${hostLabel}="${o.hostname}"`, `DCGM_FI_DRIVER_VERSION="570.124.06"`);
    if (pod) parts.push(`container="vllm"`, `namespace="${namespace}"`, `pod="${pod}"`);
    return `{${parts.join(",")}}`;
  };
  const out = [];
  for (const [name, type, help] of fields) {
    out.push(`# HELP ${name} ${help}`, `# TYPE ${name} ${type}`);
    for (let gpu = 0; gpu < o.gpus; gpu++) {
      if (kind === "mig" && gpu === 0) {
        // Карта в режиме MIG: память и профилировочные серии — по экземплярам, утилизации карты нет.
        if (MIG_FIELDS.has(name))
          for (const instance of [1, 2]) {
            const v = name === "DCGM_FI_DEV_FB_USED" ? 15000 + instance * 4000 : name === "DCGM_FI_DEV_FB_FREE" ? 25000 : dcgmValue(name, gpu, poll, o);
            out.push(`${name}${labels(gpu, String(instance))} ${v}`);
          }
        if (name === "DCGM_FI_DEV_GPU_UTIL" || MIG_FIELDS.has(name)) continue;
      }
      out.push(`${name}${labels(gpu)} ${dcgmValue(name, gpu, poll, o)}`);
    }
  }
  if (kind === "full" && o.xid) {
    out.push("# HELP DCGM_EXP_XID_ERRORS_COUNT XID errors observed during the configured time window.", "# TYPE DCGM_EXP_XID_ERRORS_COUNT gauge");
    out.push(`DCGM_EXP_XID_ERRORS_COUNT${labels(0).replace(/}$/, `,window_size_in_ms="300000",xid="${o.xid}"}`)} 1`);
  }
  return out.join("\n") + "\n";
}

/** Обработчик запросов: ключ проверяется по правилу вида (kind.open); /health отвечает заданным кодом,
 *  у 3xx — с Location на https, как у настоящего перенаправления. */
/** Сетка `query_range`: точки от start до end с шагом step, секунды. */
function grid(start, end, step) {
  const out = [];
  for (let t = start; t <= end; t += step) out.push(t);
  return out;
}

/** Выражение без селекторов и интервалов: так шаблон для любого сервера находит снятый ряд. */
const shape = (expr) => expr.replace(/\{[^}]*\}/g, "{}").replace(/\[[^\]]*\]/g, "[]");

let captured;
/** Снятые ответы query_range (range-*.json) по форме выражения; первый снимок формы — главный. */
function capturedRanges() {
  if (captured) return captured;
  captured = new Map();
  for (const name of ["range-vllm-8000.json", "range-sglang-30000.json", "range-vllm-8003.json"]) {
    for (const { expr, response } of JSON.parse(readFileSync(join(PROMETHEUS, name), "utf8")).queries)
      if (!captured.has(shape(expr))) captured.set(shape(expr), response.data.result);
  }
  return captured;
}

const vector = (result) => JSON.stringify({ status: "success", data: { resultType: "vector", result } });

/** Ответ источника истории на /api/v1/query и /api/v1/query_range (вид prometheus). */
export function prometheusReply(url, opts) {
  const query = url.searchParams.get("query") ?? "";
  if (query.includes("__bad__"))
    return [400, JSON.stringify({ status: "error", errorType: "bad_data", error: `invalid parameter "query": 1:1: parse error: unexpected identifier "__bad__"` })];
  if (url.pathname === "/api/v1/query") {
    const file = (name) => readFileSync(join(PROMETHEUS, name), "utf8");
    if (query === "up") return [200, file("up.json")];
    if (query.startsWith("count by (job, instance)")) return [200, file(query.includes("vllm:") ? "discover.json" : "discover-sglang.json")];
    if (query.startsWith("count({")) return [200, vector([{ metric: {}, value: [Date.now() / 1000, "57"] }])];
    return [200, file("empty.json")];
  }
  const [start, end, step] = ["start", "end", "step"].map((k) => Number(url.searchParams.get(k)));
  const times = grid(start, end, step);
  if (opts.huge) {
    // Число рядов — по размеру одного ряда в JSON: около 10 МБ при любом шаге окна.
    const one = (i) => ({ metric: { pod: `vllm-${i}` }, values: times.map((t) => [t, "1"]) });
    const series = Math.ceil((10 * 1024 * 1024) / JSON.stringify(one(0)).length);
    const big = Array.from({ length: series }, (_, i) => one(i));
    return [200, JSON.stringify({ status: "success", data: { resultType: "matrix", result: big } })];
  }
  const result = (capturedRanges().get(shape(query)) ?? []).map((s) => {
    const values = s.values.filter(([, v]) => v !== "NaN");
    return { metric: s.metric, values: values.length ? times.map((t, i) => [t, values[i % values.length][1]]) : [] };
  });
  return [200, JSON.stringify({ status: "success", data: { resultType: "matrix", result } })];
}

export function handler(opts) {
  const kind = KINDS[opts.kind];
  let polls = 0;
  return (req, res) => {
    const url = new URL(req.url ?? "/", "http://x");
    const path = url.pathname;
    const reply = (status, type, body, headers = {}) => {
      const send = () => {
        res.writeHead(status, { "content-type": type, ...headers });
        res.end(body);
      };
      if (opts.slow) setTimeout(send, opts.slow * 1000);
      else send();
    };
    // Экспортёр DCGM (web-config.yml dcgm-exporter) закрывается basic с логином glafira, остальные виды — токеном.
    const expected = kind.dcgm ? `Basic ${Buffer.from(`glafira:${opts.apiKey}`).toString("base64")}` : `Bearer ${opts.apiKey}`;
    if (opts.apiKey && !kind.open(path) && req.headers.authorization !== expected)
      return reply(401, "application/json", JSON.stringify({ error: "Unauthorized" }));
    if (kind.dcgm && path === "/metrics") return reply(200, "text/plain; version=0.0.4", dcgmMetrics(kind.dcgm, opts.dcgm, polls++));
    if (path === "/health" && !kind.noHealth) {
      const redirect = opts.health >= 300 && opts.health < 400;
      const authority = req.headers.host ?? `${opts.host}:${opts.port}`;
      return reply(opts.health, "text/plain", "", redirect ? { location: `https://${authority}/health` } : {});
    }
    if (kind.prometheus && (path === "/api/v1/query" || path === "/api/v1/query_range")) {
      const [status, body] = prometheusReply(url, opts);
      return reply(status, "application/json", body);
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
    const key = !opts.apiKey ? "" : KINDS[opts.kind].dcgm ? `, вход glafira / ${opts.apiKey}` : `, ключ API ${opts.apiKey}`;
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
