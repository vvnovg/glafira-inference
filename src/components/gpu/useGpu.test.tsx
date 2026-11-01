import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api/client", async (importOriginal) => {
  const orig = await importOriginal<typeof import("../../api/client")>();
  return { ...orig, api: { readGpuStatus: vi.fn(), readGpuHistory: vi.fn() } };
});

import { api } from "../../api/client";
import type { GpuHistoryAnswer } from "../../api/types";
import { TabActiveContext } from "../../lib/tabActive";
import { card, host, status } from "./testGpu";
import { appendError, appendStatus, cardKey, nowSeries, useGpuHistory, useGpuStatus } from "./useGpu";

describe("буфер «Сейчас» карт", () => {
  it("точка на каждую карту, старше окна отбрасываются, повтор снимка не дублируется", () => {
    let b = {};
    for (const at of [0, 5_000, 10_000, 10_000]) b = appendStatus(b, status(at, [host("gpu-1", [card(0), card(1)])]), 7_000);
    expect(Object.keys(b)).toEqual([cardKey("gpu-1", 0), cardKey("gpu-1", 1)]);
    expect((b as Record<string, { at: number }[]>)[cardKey("gpu-1", 1)].map((p) => p.at)).toEqual([5_000, 10_000]);
  });

  it("неудачный опрос — разрыв у всех карт", () => {
    const b = appendError(appendStatus({}, status(0, [host("gpu-1", [card(0)])])), 5_000);
    expect(b[cardKey("gpu-1", 0)]).toEqual([{ at: 0, values: card(0).values }, { at: 5_000, values: null }]);
  });

  it("ряды окна «Сейчас»: история раньше первой живой точки, затем буфер", () => {
    const b = appendStatus({}, status(30_000, [host("gpu-1", [card(0), card(1)])]));
    const history: GpuHistoryAnswer = {
      window: "now",
      stepSec: 15,
      source: "prometheus",
      series: [{ host: "gpu-1", gpu: 0, points: [{ at: 15_000, values: { util: 1 } }, { at: 30_000, values: { util: 2 } }] }],
      partial: [],
    };
    const s = nowSeries(b, "gpu-1", [{ index: 0 }, { index: 1 }, { index: 2 }], history);
    expect(s[0].map((p) => p.at)).toEqual([15_000, 30_000]);
    expect(s[0][0].values).toEqual({ util: 1 });
    expect(s[1].map((p) => p.at)).toEqual([30_000]);
    expect(s[2]).toEqual([]);
    expect(nowSeries(b, "gpu-1", [{ index: 0 }])[0]).toHaveLength(1);
  });

  it("карты с одним хостом и номером от разных экспортёров не затирают буфер друг друга", () => {
    const a = card(0, { exporter: "http://10.0.0.5:9400/metrics", values: { util: 10 } });
    const b = card(0, { exporter: "http://gpu-1:9400/metrics", values: { util: 20 } });
    let buf = {};
    for (const at of [0, 5_000]) buf = appendStatus(buf, status(at, [host("gpu-1", [a, b])]));
    expect(Object.keys(buf)).toEqual([cardKey("gpu-1", 0, a.exporter), cardKey("gpu-1", 0, b.exporter)]);
    const buffer = buf as Record<string, { at: number; values: { util?: number } | null }[]>;
    expect(buffer[cardKey("gpu-1", 0, a.exporter)].map((p) => p.values?.util)).toEqual([10, 10]);
    expect(buffer[cardKey("gpu-1", 0, b.exporter)].map((p) => p.values?.util)).toEqual([20, 20]);
    expect(nowSeries(buf, "gpu-1", [{ index: 0, exporter: b.exporter }])[0][0].values).toEqual({ util: 20 });
  });
});

function wrapper(active: boolean) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>
      <TabActiveContext.Provider value={active}>{children}</TabActiveContext.Provider>
    </QueryClientProvider>
  );
}

describe("useGpuStatus и useGpuHistory", () => {
  beforeEach(() => {
    vi.mocked(api.readGpuStatus).mockReset();
    vi.mocked(api.readGpuHistory).mockReset();
  });

  it("снимок попадает в буфер; ошибка опроса — в error", async () => {
    vi.mocked(api.readGpuStatus).mockResolvedValue(status(1_000, [host("gpu-1", [card(0)])]));
    const { result } = renderHook(() => useGpuStatus("i1"), { wrapper: wrapper(true) });
    await waitFor(() => expect(result.current.status?.at).toBe(1_000));
    await waitFor(() => expect(result.current.buffer[cardKey("gpu-1", 0)]).toHaveLength(1));
    expect(api.readGpuStatus).toHaveBeenCalledWith("i1");

    vi.mocked(api.readGpuStatus).mockRejectedValue({ code: "CONNECTION_FAILED", message: "down", retryable: true });
    const failed = renderHook(() => useGpuStatus("i2"), { wrapper: wrapper(true) });
    await waitFor(() => expect(failed.result.current.error?.code).toBe("CONNECTION_FAILED"));
  });

  it("выключенный хук не опрашивает; окно истории — по хосту и карте", async () => {
    renderHook(() => useGpuStatus("i1", false), { wrapper: wrapper(true) });
    vi.mocked(api.readGpuHistory).mockResolvedValue({ window: "1h", stepSec: 60, source: "prometheus", series: [], partial: [] });
    const { result } = renderHook(() => useGpuHistory("i1", "1h", "gpu-1", 2, true), { wrapper: wrapper(true) });
    await waitFor(() => expect(result.current.data?.window).toBe("1h"));
    expect(api.readGpuStatus).not.toHaveBeenCalled();
    expect(api.readGpuHistory).toHaveBeenCalledWith("i1", "1h", "gpu-1", 2);
  });
});
