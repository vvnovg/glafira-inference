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
  /** Селектор серий сервера в источнике истории (`{job="vllm", instance="10.0.0.5:8000"}`); нет — истории нет. */
  historySelector?: string;
}

/** Вход в источник истории: токен или пароль — только в keychain (спека I1c §2). */
export type HistoryAuth = { mode: "none" } | { mode: "bearer" } | { mode: "basic"; username: string };

/** Источник истории метрик — Prometheus или VictoriaMetrics. */
export interface HistorySource {
  url: string;
  auth: HistoryAuth;
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
  /** Источник истории метрик; нет — окна истории недоступны. */
  history?: HistorySource;
  requestTimeoutMs: number;
  connectTimeoutMs: number;
  folderId: string | null;
  /** Место в папке; у записей без него — в конце уровня. */
  order?: number;
}

/**
 * Ключи API из формы: id сервера → значение; сервера нет — оставить сохранённый, "" — удалить. Секрет источника
 * истории (токен или пароль) — по тем же правилам: нет поля — оставить, "" — удалить.
 */
export interface SecretsInput {
  apiKeys?: Record<string, string>;
  historySecret?: string;
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

export type TestStepName = "config" | "tcp" | "tls" | "health" | "auth" | "kind" | "models" | "up" | "targets" | "match";

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

/** Каноническая метрика Glafira (спец. I1b §3). */
export type Canonical =
  | "requestsRunning"
  | "requestsWaiting"
  | "kvUsage"
  | "preemptionsTotal"
  | "promptTokensTotal"
  | "generationTokensTotal"
  | "prefixCacheQueriesTotal"
  | "prefixCacheHitsTotal"
  | "requestSuccessTotal"
  | "requestAbortedTotal"
  | "ttft"
  | "itl"
  | "e2e"
  | "queueTime"
  | "promptLen"
  | "genLen";

/** Перцентили задержки в миллисекундах; `null` — наблюдений не было. */
export interface Latency {
  p50: number | null;
  p95: number | null;
  p99: number | null;
  count: number;
}

export interface LatencySet {
  ttft: Latency | null;
  itl: Latency | null;
  e2e: Latency | null;
  queueTime: Latency | null;
}

/** Метрики сервера «сейчас»: снимок и производные за интервал с предыдущего (спец. I1b §4). */
export interface ServerMetrics {
  /** Время снимка, мс Unix. */
  at: number;
  intervalSec: number | null;
  /** Счётчики уменьшились — сервер перезапущен, производных за интервал нет. */
  reset: boolean;
  engines: number;
  gauges: { requestsRunning: number | null; requestsWaiting: number | null; kvUsage: number | null; kvUsageAvg: number | null };
  rates: {
    promptTokens: number | null;
    generationTokens: number | null;
    requests: number | null;
    preemptions: number | null;
    aborted: number | null;
    finishedByReason: Record<string, number>;
  } | null;
  prefixCacheHitRatio: number | null;
  latency: LatencySet;
  /** Перцентили за последнюю минуту — для плиток. */
  latency1m: LatencySet;
  /** Вытеснений за ту же минуту; `null` — счётчика нет или минуты ещё нет. */
  preemptions1m: number | null;
  totals: Partial<Record<Canonical, number>>;
  missing: Canonical[];
  series: Partial<Record<Canonical, string>>;
}

/** Окно истории: `now` — 15 минут для «Сейчас», остальные — окна вкладки «Метрики». */
export type HistoryWindow = "now" | "1h" | "24h" | "7d" | "30d";
export type HistorySourceKind = "prometheus" | "victoriametrics";

/** Цель инференса в источнике истории. */
export interface HistoryTarget {
  job: string;
  instance: string;
  kind: "vllm" | "sglang";
}

/** Проверка источника истории: шаги, найденные цели и автоподбор серверов без селектора. */
export interface HistoryCheck {
  report: TestReport;
  source?: { kind: HistorySourceKind; version: string | null };
  targets: HistoryTarget[];
  suggestions: { serverId: string; selector: string | null; targets: HistoryTarget[] }[];
}

/** Окно истории сервера: точки той же формы, что у «Сейчас»; `partial` — ряды, которые не прочитались. */
export interface HistoryAnswer {
  window: HistoryWindow;
  stepSec: number;
  source: HistorySourceKind;
  points: { at: number; m: ServerMetrics }[];
  partial: { chart: string; error: AppError }[];
}

export type ConfigSource = "server_info" | "models" | "metrics";

/** Аргумент запуска сервера с источником (спец. I1b §5). */
export interface ConfigEntry {
  name: string;
  value: string;
  source: ConfigSource;
  alternatives?: { value: string; source: ConfigSource }[];
  /** Секрет: значение не передаётся, видно только «задан». */
  secret?: boolean;
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

export type UpdateCheck =
  | { status: "disabled"; reason: LocalizedText }
  | { status: "upToDate" }
  | { status: "available"; version: string; notes: string | null; notesI18n: Record<string, string>; date: string | null };

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
