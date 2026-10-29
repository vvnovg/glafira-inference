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

  it("ошибка из Rust приходит как есть, прочие — как INTERNAL", async () => {
    const e: AppError = { code: "TIMEOUT", key: "errors.server.timeout", message: "late", retryable: true };
    invoke.mockRejectedValue(e);
    await expect(api.listInstallations()).rejects.toEqual(e);
    invoke.mockRejectedValue(new Error("boom"));
    await expect(api.listInstallations()).rejects.toEqual({ code: "INTERNAL", message: "boom", retryable: false });
    expect(toAppError("x")).toEqual({ code: "INTERNAL", message: "x", retryable: false });
  });
});
