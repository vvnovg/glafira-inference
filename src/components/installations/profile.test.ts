import { describe, expect, it } from "vitest";
import { isPlainHttp, newInstallation, newServer, newServerId, withCaMode, withProxyMode } from "./profile";

describe("профиль инсталляции", () => {
  it("новая инсталляция — с одним пустым сервером, тип определяется автоматически", () => {
    const p = newInstallation();
    expect(p.id).toBe("");
    expect(p.servers).toHaveLength(1);
    expect(p.servers[0]).toMatchObject({ kind: "auto", metricsPath: "/metrics", ca: { mode: "system" }, proxy: { mode: "system" } });
  });

  it("id сервера — короткий, латиница и цифры, у каждого свой", () => {
    const a = newServerId();
    expect(a).toMatch(/^s-[0-9a-f]{12}$/);
    expect(newServer().id).not.toBe(newServer().id);
  });

  it("CA и прокси: введённое не теряется при переключении режима", () => {
    expect(withCaMode({ mode: "pem", pem: "X" }, "pem")).toEqual({ mode: "pem", pem: "X" });
    expect(withCaMode({ mode: "system" }, "pem", "LAST")).toEqual({ mode: "pem", pem: "LAST" });
    expect(withCaMode({ mode: "pem", pem: "X" }, "insecure")).toEqual({ mode: "insecure" });
    expect(withProxyMode({ mode: "none" }, "url", "http://p:3128")).toEqual({ mode: "url", url: "http://p:3128" });
    expect(withProxyMode({ mode: "url", url: "u" }, "system")).toEqual({ mode: "system" });
  });

  it("http — без TLS", () => {
    expect(isPlainHttp("http://10.0.0.5:8000")).toBe(true);
    expect(isPlainHttp(" HTTP://h")).toBe(true);
    expect(isPlainHttp("https://h")).toBe(false);
    expect(isPlainHttp("")).toBe(false);
  });
});
