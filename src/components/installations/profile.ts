import type { CaConfig, InstallationProfile, ProxyConfig, ServerConfig } from "../../api/types";

export type CaMode = CaConfig["mode"];
export type ProxyMode = ProxyConfig["mode"];

/** Больше серверов в инсталляции нельзя: тот же лимит держит бэкенд (MAX_SERVERS в storage/profile.rs), форма показывает его до «Сохранить». */
export const MAX_SERVERS = 32;

/**
 * Идентификатор нового сервера: часть ключа keychain, поэтому только латиница, цифры и «-».
 * `crypto.getRandomValues`, а не `randomUUID`: тот появился только в Safari 15.4, а нужен WebKit macOS 11.
 */
export function newServerId(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(6));
  return `s-${Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("")}`;
}

export function newServer(): ServerConfig {
  return {
    id: newServerId(),
    name: "",
    baseUrl: "",
    kind: "auto",
    metricsPath: "/metrics",
    ca: { mode: "system" },
    proxy: { mode: "system" },
  };
}

export function newInstallation(): InstallationProfile {
  return {
    id: "",
    name: "",
    color: "#3b82f6",
    environment: "dev",
    readOnly: false,
    servers: [newServer()],
    requestTimeoutMs: 30000,
    connectTimeoutMs: 10000,
    folderId: null,
    order: 0,
  };
}

/** Режим CA; введённый PEM сохраняется при переключении туда и обратно. */
export function withCaMode(ca: CaConfig, mode: CaMode, lastPem = ""): CaConfig {
  if (mode === "pem") return { mode, pem: ca.mode === "pem" ? ca.pem : lastPem };
  return { mode };
}

/** Режим прокси; введённый адрес сохраняется при переключении туда и обратно. */
export function withProxyMode(proxy: ProxyConfig, mode: ProxyMode, lastUrl = ""): ProxyConfig {
  if (mode === "url") return { mode, url: proxy.mode === "url" ? proxy.url : lastUrl };
  return { mode };
}

/** Адрес без TLS: ключ API уходит по нему открытым текстом. */
export function isPlainHttp(baseUrl: string): boolean {
  return /^\s*http:\/\//i.test(baseUrl);
}
