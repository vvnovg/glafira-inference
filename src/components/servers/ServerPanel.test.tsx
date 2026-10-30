import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { renderWithClient } from "../../test/render";

vi.mock("../../api/client", async (importOriginal) => {
  const orig = await importOriginal<typeof import("../../api/client")>();
  return {
    ...orig,
    api: { listInstallations: vi.fn(), connectInstallation: vi.fn(), readServerMetrics: vi.fn(), readServerConfig: vi.fn() },
  };
});
vi.mock("uplot", () => ({
  default: vi.fn().mockImplementation(function () {
    return { destroy: vi.fn(), setSize: vi.fn(), setData: vi.fn() };
  }),
}));
globalThis.ResizeObserver ??= class {
  observe() {}
  disconnect() {}
  unobserve() {}
} as unknown as typeof ResizeObserver;

import { api } from "../../api/client";
import type { InstallationInfo, InstallationProfile } from "../../api/types";
import { useWorkspace } from "../../state/workspace";
import { newInstallation } from "../installations/profile";
import { ServerPanel } from "./ServerPanel";
import { metrics } from "./testMetrics";

const PROFILE: InstallationProfile = {
  ...newInstallation(),
  id: "i1",
  name: "prod-llm",
  servers: [
    { id: "s1", name: "gpu-1", baseUrl: "http://gpu-1:8000", kind: "auto", metricsPath: "/metrics", ca: { mode: "system" }, proxy: { mode: "system" } },
    { id: "s2", name: "gpu-2", baseUrl: "http://gpu-2:8000", kind: "auto", metricsPath: "/metrics", ca: { mode: "system" }, proxy: { mode: "system" } },
  ],
};

const INFO: InstallationInfo = {
  installationId: "i1",
  servers: [
    {
      serverId: "s1",
      facts: { kind: "vllm", detected: "vllm", version: "0.32.0", models: [{ id: "qwen", root: "/models/Qwen", maxModelLen: 32768 }], metrics: true, warnings: [] },
    },
    { serverId: "s2", error: { code: "AUTH_FAILED", key: "errors.server.unauthorized", params: { status: "401" }, message: "rejected", retryable: false } },
  ],
};

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.listInstallations).mockResolvedValue([PROFILE]);
  vi.mocked(api.connectInstallation).mockResolvedValue(INFO);
  vi.mocked(api.readServerMetrics).mockResolvedValue(
    metrics(Date.now(), { missing: ["itl"], latency1m: { ttft: { p50: 90, p95: 250, p99: 410, count: 120 }, itl: null, e2e: null, queueTime: null } }),
  );
  vi.mocked(api.readServerConfig).mockResolvedValue([
    { name: "model", value: "/models/Qwen", source: "models" },
    { name: "served_model_name", value: "qwen", source: "server_info", alternatives: [{ value: "qwen-old", source: "models" }] },
    { name: "api_key", value: "", source: "server_info", secret: true },
    { name: "block_size", value: "16", source: "metrics" },
  ]);
  useWorkspace.setState({ tabs: [], activeId: null });
});

describe("ServerPanel", () => {
  it("«Обзор»: тип и версия, модели, плитки метрик; нет серии — «нет данных»", async () => {
    renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
    expect(await screen.findByText("vLLM 0.32.0")).toBeInTheDocument();
    expect(screen.getByText("qwen · 32768")).toBeInTheDocument();
    const kv = await screen.findByRole("region", { name: "KV-кэш" });
    await waitFor(() => expect(within(kv).getByText(/42\s?%/)).toBeInTheDocument());
    expect(within(screen.getByRole("region", { name: "Выполняются · ждут" })).getByText("3 · 1")).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "TTFT p95 за минуту" })).getByText("250 мс")).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "Токены на входе" })).getByText("120/с")).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "ITL p95 за минуту" })).getByText("Нет данных")).toBeInTheDocument();
    expect(screen.queryByText("Движки (data parallel)")).toBeNull();
    expect(api.readServerMetrics).toHaveBeenCalledWith("i1", "s1");
  });

  it("«Конфигурация»: источник, расхождение источников и скрытый секрет", async () => {
    const user = userEvent.setup();
    renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
    await user.click(await screen.findByRole("tab", { name: "Конфигурация" }));
    const row = (await screen.findByText("served_model_name")).closest("tr") as HTMLElement;
    expect(within(row).getByText("сведения о сервере")).toBeInTheDocument();
    expect(within(row).getByText("⚠ модели: qwen-old")).toBeInTheDocument();
    const secret = screen.getByText("api_key").closest("tr") as HTMLElement;
    expect(within(secret).getByText("задан (скрыт)")).toBeInTheDocument();
    expect(within(screen.getByText("block_size").closest("tr") as HTMLElement).getByText("метрики")).toBeInTheDocument();
    expect(api.readServerConfig).toHaveBeenCalledWith("i1", "s1");
  });

  it("«Метрики»: окно «Сейчас», история недоступна до I1c, графики строятся", async () => {
    const user = userEvent.setup();
    renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
    await user.click(await screen.findByRole("tab", { name: "Метрики" }));
    expect(screen.getByRole("button", { name: "Сейчас" })).toHaveAttribute("aria-pressed", "true");
    const day = screen.getByRole("button", { name: "24 ч" });
    expect(day).toBeDisabled();
    expect(day).toHaveAttribute("title", "История — с этапа I1c");
    expect(await screen.findByRole("region", { name: "Время до первого токена (TTFT)" })).toBeInTheDocument();
  });

  it("сервер, который не подключился: его ошибка, без подвкладок и без опроса", async () => {
    renderWithClient(<ServerPanel installationId="i1" serverId="s2" />);
    expect(await screen.findByText("Сервер отклонил запрос (401) — проверьте ключ API")).toBeInTheDocument();
    expect(screen.queryByRole("tab")).toBeNull();
    expect(api.readServerMetrics).not.toHaveBeenCalled();
  });

  it("ошибка опроса — «Нет ответа сервера» в шапке", async () => {
    vi.mocked(api.readServerMetrics).mockRejectedValue({ code: "TIMEOUT", key: "errors.server.timeout", message: "timed out", retryable: true });
    renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
    expect(await screen.findByText("Нет ответа сервера: timed out")).toBeInTheDocument();
  });

  it("ссылка на инсталляцию открывает её вкладку; удалённый сервер", async () => {
    const user = userEvent.setup();
    const { unmount } = renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
    await user.click(await screen.findByRole("button", { name: "prod-llm" }));
    expect(useWorkspace.getState().activeId).toBe("installation:i1");
    unmount();
    renderWithClient(<ServerPanel installationId="i1" serverId="gone" />);
    expect(await screen.findByText("Сервер или его инсталляция удалены")).toBeInTheDocument();
  });
});
