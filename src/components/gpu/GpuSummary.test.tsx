import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { InstallationProfile } from "../../api/types";
import { useWorkspace } from "../../state/workspace";
import { renderWithClient } from "../../test/render";
import { newInstallation, newServer } from "../installations/profile";

vi.mock("../../api/client", async (importOriginal) => {
  const orig = await importOriginal<typeof import("../../api/client")>();
  return { ...orig, api: { readGpuStatus: vi.fn() } };
});

import { api } from "../../api/client";
import { GpuHostsBlock, ServerGpuBlock } from "./GpuSummary";
import { boundCards, hostSummary } from "./summary";
import { card, host, status } from "./testGpu";

const profile = (): InstallationProfile => ({ ...newInstallation(), id: "i1", servers: [{ ...newServer(), id: "s1", name: "vllm-1" }], gpu: { mode: "prometheus" } });

beforeEach(() => {
  vi.clearAllMocks();
  useWorkspace.setState({ tabs: [], activeId: null });
});

describe("сводка GPU", () => {
  it("строка хоста: модель или «разные», средняя утилизация, сумма памяти, максимум температуры, карты с проблемами", () => {
    const h = host("gpu-1", [
      card(0, { values: { util: 80, memUsed: 1024, memTotal: 2048, temp: 70 } }),
      card(1, { values: { util: 40, memUsed: 1024, memTotal: 2048, temp: 85, xid: 79 } }),
    ]);
    expect(hostSummary(h)).toEqual({ host: "gpu-1", model: "NVIDIA H100 80GB HBM3", count: 2, avgUtil: 60, memUsed: 2048, memTotal: 4096, maxTemp: 85, troubled: 1 });
    const mixed = hostSummary(host("x", [card(0), card(1, { model: "NVIDIA A100", values: {} })]));
    expect(mixed.model).toBeNull();
    expect(hostSummary(host("y", [card(0, { values: {} })]))).toMatchObject({ avgUtil: null, memUsed: null, maxTemp: null });
  });

  it("карты привязки — по хосту без учёта регистра и номерам", () => {
    const s = status(1, [host("gpu-1", [card(0), card(1), card(2)])]);
    expect(boundCards(s, { host: "GPU-1" }).map((c) => c.index)).toEqual([0, 1, 2]);
    expect(boundCards(s, { host: "gpu-1", indices: [2, 5] }).map((c) => c.index)).toEqual([2]);
    expect(boundCards(s, { host: "gpu-9" })).toEqual([]);
    expect(boundCards(undefined, { host: "gpu-1" })).toEqual([]);
  });

  it("«GPU по хостам» в обзоре инсталляции открывает вкладку GPU", async () => {
    vi.mocked(api.readGpuStatus).mockResolvedValue(status(1, [host("gpu-1", [card(0, { values: { util: 50, memUsed: 40960, memTotal: 81920, temp: 61 } })])]));
    const user = userEvent.setup();
    renderWithClient(<GpuHostsBlock profile={profile()} />);
    const block = screen.getByRole("region", { name: "GPU по хостам" });
    const row = (await within(block).findByRole("button", { name: "gpu-1" })).closest("tr") as HTMLElement;
    expect(within(row).getByText("50 %")).toBeInTheDocument();
    expect(within(row).getByText("40 / 80 ГиБ")).toBeInTheDocument();
    expect(within(row).getByText("61 °C")).toBeInTheDocument();
    await user.click(within(row).getByRole("button", { name: "gpu-1" }));
    expect(useWorkspace.getState().activeId).toBe("gpu:i1");
  });

  it("«GPU сервера»: привязанные карты и значки; без привязки — подсказка без опроса", async () => {
    vi.mocked(api.readGpuStatus).mockResolvedValue(status(1, [host("gpu-1", [card(0), card(1, { values: { util: 12, xid: 79 } })])]));
    const p = profile();
    renderWithClient(<ServerGpuBlock profile={p} server={{ ...p.servers[0], gpus: { host: "gpu-1", indices: [1] } }} />);
    const block = screen.getByRole("region", { name: "GPU сервера" });
    expect(await within(block).findByText("GPU 1")).toBeInTheDocument();
    expect(within(block).queryByText("GPU 0")).toBeNull();
    expect(within(block).getByText(/утилизация 12 %/)).toBeInTheDocument();
    expect(within(block).getByText(/XID 79/)).toBeInTheDocument();
    expect(within(block).getByRole("button", { name: "Открыть GPU" })).toBeInTheDocument();

    vi.mocked(api.readGpuStatus).mockClear();
    renderWithClient(<ServerGpuBlock profile={p} server={p.servers[0]} />);
    expect(screen.getByText("Привяжите GPU в форме сервера")).toBeInTheDocument();
    expect(api.readGpuStatus).not.toHaveBeenCalled();
  });
});
