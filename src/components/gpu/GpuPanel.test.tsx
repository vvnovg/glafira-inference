import { act, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { InstallationProfile } from "../../api/types";
import { renderWithClient } from "../../test/render";
import { newInstallation, newServer } from "../installations/profile";

vi.mock("../../api/client", async (importOriginal) => {
  const orig = await importOriginal<typeof import("../../api/client")>();
  return { ...orig, api: { listInstallations: vi.fn(), readGpuStatus: vi.fn(), readGpuHistory: vi.fn() } };
});

vi.mock("uplot", () => ({
  default: vi.fn().mockImplementation(function () {
    return { destroy: vi.fn(), setSize: vi.fn(), setData: vi.fn(), redraw: vi.fn() };
  }),
}));
globalThis.ResizeObserver ??= class {
  observe() {}
  disconnect() {}
  unobserve() {}
} as unknown as typeof ResizeObserver;

import { api } from "../../api/client";
import { boundServers, GpuPanel, gpuHistoryUnavailable } from "./GpuPanel";
import { card, host, status } from "./testGpu";

const direct = (historySelector?: string): InstallationProfile["gpu"] => ({
  mode: "direct",
  exporters: [{ url: "http://gpu-1:9400/metrics", historySelector }],
  auth: { mode: "none" },
  ca: { mode: "system" },
  proxy: { mode: "system" },
});

function profile(over: Partial<InstallationProfile> = {}): InstallationProfile {
  const s = { ...newServer(), id: "s1", name: "vllm-1", gpus: { host: "GPU-1" } };
  return { ...newInstallation(), id: "i1", name: "prod LLM", servers: [s], gpu: direct(), ...over };
}

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.listInstallations).mockResolvedValue([profile()]);
  vi.mocked(api.readGpuHistory).mockResolvedValue({ window: "1h", stepSec: 60, source: "prometheus", series: [], partial: [] });
});

describe("GpuPanel", () => {
  it("хосты и карточки карт; «нет данных», запасная SM, значки и нагрузка", async () => {
    vi.mocked(api.readGpuStatus).mockResolvedValue(
      status(1, [
        host("gpu-1", [
          card(0, { workload: { namespace: "llm", pod: "vllm-0" }, values: { util: 91, smActive: 40, memUsed: 40960, memTotal: 81920, temp: 89, power: 650, throttle: 0x48, xid: 79 } }),
          card(1, { smSource: "gr", values: { util: 20, smActive: 18 }, deltas: {} }),
        ]),
      ]),
    );
    renderWithClient(<GpuPanel installationId="i1" />);
    const hostSection = await screen.findByRole("region", { name: "gpu-1" });
    expect(within(hostSection).getByText(/Карт: 2/)).toBeInTheDocument();
    expect(within(hostSection).getByText(/драйвер 570\.124\.06/)).toBeInTheDocument();
    expect(within(hostSection).getByText(/Серверы: vllm-1/)).toBeInTheDocument();
    const c0 = screen.getByRole("region", { name: "GPU 0" });
    expect(within(c0).getByText(/Троттлинг: HW_SLOWDOWN, HW_THERMAL/)).toBeInTheDocument();
    expect(within(c0).getByText(/XID 79/)).toBeInTheDocument();
    expect(within(c0).getByText("Нагрузка: llm/vllm-0")).toBeInTheDocument();
    expect(within(c0).getByText(/40 \/ 80 ГиБ/)).toBeInTheDocument();
    const c1 = screen.getByRole("region", { name: "GPU 1" });
    expect(within(c1).getByText("по GR engine")).toBeInTheDocument();
    expect(within(c1).getAllByText("нет данных")).toHaveLength(3);
    expect(screen.getByText("Щёлкните карту или хост, чтобы открыть графики")).toBeInTheDocument();
  });

  it("сбой экспортёра, неполный набор счётчиков и сбой всего опроса", async () => {
    vi.mocked(api.readGpuStatus).mockResolvedValue(
      status(1, [host("gpu-1", [card(0)])], {
        errors: [{ exporter: "http://gpu-2:9400/metrics", error: { code: "CONNECTION_FAILED", key: "errors.server.serverError", params: { path: "/metrics", status: "503" }, message: "x", retryable: true } }],
        missing: ["sm", "ecc"],
      }),
    );
    renderWithClient(<GpuPanel installationId="i1" />);
    expect(await screen.findByText(/Экспортёр http:\/\/gpu-2:9400\/metrics:/)).toBeInTheDocument();
    expect(screen.getByText(/Экспортёр не собирает: активность SM, ошибки ECC/)).toBeInTheDocument();

    vi.mocked(api.readGpuStatus).mockRejectedValue({ code: "CONNECTION_FAILED", key: "errors.server.serverError", params: { path: "/metrics", status: "503" }, message: "x", retryable: true });
    vi.mocked(api.listInstallations).mockResolvedValue([profile({ id: "i2" })]);
    renderWithClient(<GpuPanel installationId="i2" />);
    expect(await screen.findByRole("alert")).toBeInTheDocument();
  });

  it("щелчок по хосту и карте раскрывает графики; окна истории — по профилю", async () => {
    vi.mocked(api.readGpuStatus).mockResolvedValue(status(1, [host("gpu-1", [card(0), card(1)])]));
    const user = userEvent.setup();
    renderWithClient(<GpuPanel installationId="i1" />);
    await screen.findByRole("region", { name: "gpu-1" });
    expect(screen.getByRole("button", { name: "1 ч" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "gpu-1" }));
    expect(screen.getByRole("region", { name: "Утилизация" })).toBeInTheDocument();
    await user.click(within(screen.getByRole("region", { name: "GPU 1" })).getByRole("button", { name: /GPU 1/ }));
    expect(screen.getByRole("region", { name: "Утилизация и SM" })).toBeInTheDocument();
    await user.click(within(screen.getByRole("region", { name: "GPU 1" })).getByRole("button", { name: /GPU 1/ }));
    expect(screen.queryByRole("region", { name: "Утилизация и SM" })).not.toBeInTheDocument();
    expect(api.readGpuHistory).not.toHaveBeenCalled();
  });

  it("с историей: окно 1 ч запрашивается по хосту и карте; «Сейчас» предзаполняется", async () => {
    vi.mocked(api.listInstallations).mockResolvedValue([
      profile({ history: { url: "http://prom:9090", auth: { mode: "none" }, ca: { mode: "system" }, proxy: { mode: "system" } }, gpu: direct('{job="dcgm"}') }),
    ]);
    vi.mocked(api.readGpuStatus).mockResolvedValue(status(1, [host("gpu-1", [card(0)])]));
    const user = userEvent.setup();
    renderWithClient(<GpuPanel installationId="i1" />);
    await screen.findByRole("region", { name: "gpu-1" });
    await user.click(screen.getByRole("button", { name: "gpu-1" }));
    expect(api.readGpuHistory).toHaveBeenCalledWith("i1", "now", "gpu-1", null);
    await user.click(screen.getByRole("button", { name: "1 ч" }));
    await act(async () => {});
    expect(api.readGpuHistory).toHaveBeenCalledWith("i1", "1h", "gpu-1", null);
    expect(await screen.findByText(/Шаг 1 мин · Prometheus/)).toBeInTheDocument();
  });

  it("без компонента GPU — подсказка; помощники доступности истории и привязанных серверов", async () => {
    vi.mocked(api.listInstallations).mockResolvedValue([profile({ gpu: undefined })]);
    renderWithClient(<GpuPanel installationId="i1" />);
    expect(await screen.findByText("У инсталляции нет компонента GPU — добавьте его в форме инсталляции")).toBeInTheDocument();
    expect(gpuHistoryUnavailable(profile())).toBe("gpu.noSource");
    const history = { url: "http://p:9090", auth: { mode: "none" as const }, ca: { mode: "system" as const }, proxy: { mode: "system" as const } };
    expect(gpuHistoryUnavailable(profile({ history }))).toBe("gpu.notMatched");
    expect(gpuHistoryUnavailable(profile({ history, gpu: { mode: "prometheus" } }))).toBeNull();
    expect(boundServers(profile(), "gpu-1")).toEqual(["vllm-1"]);
    expect(boundServers(profile(), "gpu-2")).toEqual([]);
  });
});
