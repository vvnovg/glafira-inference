import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

import { api, toAppError } from "./client";
import type { AppError, InstallationProfile } from "./types";

beforeEach(() => {
  invoke.mockReset();
});

describe("api client", () => {
  it("saveInstallation оборачивает профиль и ключи в input, ключи по умолчанию пустые", async () => {
    invoke.mockResolvedValue({});
    const profile = { id: "a" } as InstallationProfile;
    await api.saveInstallation(profile, { apiKeys: { s1: "sk" } });
    expect(invoke).toHaveBeenCalledWith("save_installation", {
      input: { profile, secrets: { apiKeys: { s1: "sk" } } },
    });
    await api.saveInstallation(profile);
    expect(invoke).toHaveBeenLastCalledWith("save_installation", { input: { profile, secrets: {} } });
  });

  it("checkHistorySource передаёт профиль формы и секрет источника", async () => {
    invoke.mockResolvedValue({ report: { ok: true, steps: [], warnings: [] }, targets: [], suggestions: [] });
    const profile = { id: "a" } as InstallationProfile;
    await api.checkHistorySource(profile, { historySecret: "t" });
    expect(invoke).toHaveBeenCalledWith("check_history_source", { input: { profile, secrets: { historySecret: "t" } } });
  });

  it("checkGpuSource передаёт профиль формы и пароль экспортёров", async () => {
    invoke.mockResolvedValue({ report: { ok: true, steps: [], warnings: [] }, hosts: [], available: [], bindings: [], exporters: [] });
    const profile = { id: "a" } as InstallationProfile;
    await api.checkGpuSource(profile, { gpuSecret: "p" });
    expect(invoke).toHaveBeenCalledWith("check_gpu_source", { input: { profile, secrets: { gpuSecret: "p" } } });
  });

  it("readGpuStatus и readGpuHistory: инсталляция, окно, хост и карта (без карты — null)", async () => {
    invoke.mockResolvedValue({});
    await api.readGpuStatus("i1");
    expect(invoke).toHaveBeenLastCalledWith("read_gpu_status", { installationId: "i1" });
    await api.readGpuHistory("i1", "1h", "gpu-node-1", 3);
    expect(invoke).toHaveBeenLastCalledWith("read_gpu_history", { installationId: "i1", window: "1h", host: "gpu-node-1", gpu: 3 });
    await api.readGpuHistory("i1", "now", "gpu-node-1");
    expect(invoke).toHaveBeenLastCalledWith("read_gpu_history", { installationId: "i1", window: "now", host: "gpu-node-1", gpu: null });
  });

  it("testServer передаёт профиль формы, ключи и id сервера", async () => {
    invoke.mockResolvedValue({ ok: true, steps: [], warnings: [] });
    const profile = { id: "", name: "x" } as InstallationProfile;
    await api.testServer(profile, "s1", { apiKeys: { s1: "sk" } });
    expect(invoke).toHaveBeenCalledWith("test_server_connection", {
      input: { profile, secrets: { apiKeys: { s1: "sk" } } },
      serverId: "s1",
    });
  });

  it("connectInstallation передаёт аргументы в camelCase, fresh по умолчанию false", async () => {
    invoke.mockResolvedValue({});
    await api.connectInstallation("i1");
    expect(invoke).toHaveBeenCalledWith("connect_installation", { installationId: "i1", fresh: false });
    await api.connectInstallation("i1", true);
    expect(invoke).toHaveBeenLastCalledWith("connect_installation", { installationId: "i1", fresh: true });
  });

  it("метрики и аргументы сервера — по инсталляции и серверу", async () => {
    invoke.mockResolvedValue({});
    await api.readServerMetrics("i1", "s1");
    expect(invoke).toHaveBeenCalledWith("read_server_metrics", { installationId: "i1", serverId: "s1" });
    await api.readServerConfig("i1", "s2");
    expect(invoke).toHaveBeenLastCalledWith("read_server_config", { installationId: "i1", serverId: "s2" });
    await api.readServerHistory("i1", "s2", "24h");
    expect(invoke).toHaveBeenLastCalledWith("read_server_history", { installationId: "i1", serverId: "s2", window: "24h" });
  });

  it("ошибка из Rust приходит как есть, прочие — как INTERNAL", async () => {
    const e: AppError = { code: "TIMEOUT", key: "errors.server.timeout", message: "late", retryable: true };
    invoke.mockRejectedValue(e);
    await expect(api.listInstallations()).rejects.toEqual(e);
    invoke.mockRejectedValue(new Error("boom"));
    await expect(api.listInstallations()).rejects.toEqual({ code: "INTERNAL", message: "boom", retryable: false });
    expect(toAppError("x")).toEqual({ code: "INTERNAL", message: "x", retryable: false });
  });
});
