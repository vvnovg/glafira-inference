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
}

export interface InstallationProfile {
  id: string;
  name: string;
  color: string;
  environment: Environment;
  readOnly: boolean;
  servers: ServerConfig[];
  requestTimeoutMs: number;
  connectTimeoutMs: number;
  folderId: string | null;
  /** Место в папке; у записей без него — в конце уровня. */
  order?: number;
}

/** Ключи API из формы: id сервера → значение; сервера нет — оставить сохранённый, "" — удалить. */
export interface SecretsInput {
  apiKeys?: Record<string, string>;
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

export type TestStepName = "config" | "tcp" | "tls" | "health" | "auth" | "kind" | "models";

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
