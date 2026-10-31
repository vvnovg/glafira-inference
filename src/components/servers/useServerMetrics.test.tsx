import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api/client", async (importOriginal) => {
  const orig = await importOriginal<typeof import("../../api/client")>();
  return { ...orig, api: { readServerMetrics: vi.fn() } };
});

import { api } from "../../api/client";
import { TabActiveContext } from "../../lib/tabActive";
import { metrics } from "./testMetrics";
import { appendPoint, type MetricPoint, useServerMetrics, WINDOW_MS } from "./useServerMetrics";

const point = (at: number): MetricPoint => ({ at, m: metrics(at) });

describe("appendPoint", () => {
  it("добавляет точки и отбрасывает старше окна", () => {
    let b: MetricPoint[] = [];
    for (const at of [0, 5_000, 10_000]) b = appendPoint(b, point(at), 7_000);
    expect(b.map((p) => p.at)).toEqual([5_000, 10_000]);
  });

  it("повтор того же снимка не дублируется", () => {
    const b = appendPoint([point(1)], point(1));
    expect(b).toHaveLength(1);
  });

  it("окно по умолчанию — час: 720 точек раз в 5 с", () => {
    let b: MetricPoint[] = [];
    for (let i = 0; i <= 800; i++) b = appendPoint(b, point(i * 5_000));
    expect(b[b.length - 1].at - b[0].at).toBeLessThanOrEqual(WINDOW_MS);
    expect(b).toHaveLength(721);
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

describe("useServerMetrics", () => {
  beforeEach(() => vi.mocked(api.readServerMetrics).mockReset());

  it("копит точки опросов, ошибка — точка-разрыв", async () => {
    // Время снимков — настоящее: точка ошибки ставится по часам окна, а окно отсчитывается от последней точки.
    const t0 = Date.now();
    vi.mocked(api.readServerMetrics)
      .mockResolvedValueOnce(metrics(t0))
      .mockRejectedValueOnce({ code: "TIMEOUT", key: "errors.server.timeout", message: "late", retryable: true })
      .mockResolvedValueOnce(metrics(t0 + 10_000));
    const { result } = renderHook(() => useServerMetrics("i1", "s1"), { wrapper: wrapper(true) });
    await waitFor(() => expect(result.current.points).toHaveLength(1));
    expect(api.readServerMetrics).toHaveBeenCalledWith("i1", "s1");
    await act(() => result.current.refetch());
    await waitFor(() => expect(result.current.points).toHaveLength(2));
    expect(result.current.points[1]).toMatchObject({ m: null, error: { code: "TIMEOUT" } });
    expect(result.current.error?.code).toBe("TIMEOUT");
    await act(() => result.current.refetch());
    await waitFor(() => expect(result.current.points).toHaveLength(3));
    expect(result.current.latest?.at).toBe(t0 + 10_000);
    expect(result.current.error).toBeNull();
  });

  it("смена сервера начинает новый буфер: точки двух серверов не смешиваются", async () => {
    vi.mocked(api.readServerMetrics).mockImplementation(async (_i, s) => metrics(s === "s1" ? 1_000 : 2_000));
    const { result, rerender } = renderHook(({ s }) => useServerMetrics("i1", s), {
      wrapper: wrapper(true),
      initialProps: { s: "s1" },
    });
    await waitFor(() => expect(result.current.points.map((p) => p.at)).toEqual([1_000]));
    rerender({ s: "s2" });
    expect(result.current.points).toEqual([]);
    await waitFor(() => expect(result.current.points.map((p) => p.at)).toEqual([2_000]));
  });

  it("disabled — сервер не опрашивается", async () => {
    vi.useFakeTimers();
    try {
      renderHook(() => useServerMetrics("i1", "s1", false), { wrapper: wrapper(true) });
      await act(async () => {
        await vi.advanceTimersByTimeAsync(20_000);
      });
      expect(api.readServerMetrics).not.toHaveBeenCalled();
    } finally {
      vi.useRealTimers();
    }
  });

  it("на скрытой вкладке интервального опроса нет", async () => {
    vi.useFakeTimers();
    try {
      vi.mocked(api.readServerMetrics).mockResolvedValue(metrics(1));
      renderHook(() => useServerMetrics("i1", "s1"), { wrapper: wrapper(false) });
      await act(async () => {
        await vi.advanceTimersByTimeAsync(0);
      });
      expect(api.readServerMetrics).toHaveBeenCalledTimes(1);
      await act(async () => {
        await vi.advanceTimersByTimeAsync(20_000);
      });
      expect(api.readServerMetrics).toHaveBeenCalledTimes(1);
    } finally {
      vi.useRealTimers();
    }
  });

  it("на видимой вкладке — раз в 5 с", async () => {
    vi.useFakeTimers();
    try {
      let at = 0;
      vi.mocked(api.readServerMetrics).mockImplementation(async () => metrics((at += 5_000)));
      renderHook(() => useServerMetrics("i1", "s1"), { wrapper: wrapper(true) });
      await act(async () => {
        await vi.advanceTimersByTimeAsync(0);
      });
      expect(api.readServerMetrics).toHaveBeenCalledTimes(1);
      await act(async () => {
        await vi.advanceTimersByTimeAsync(10_500);
      });
      expect(vi.mocked(api.readServerMetrics).mock.calls.length).toBeGreaterThanOrEqual(3);
    } finally {
      vi.useRealTimers();
    }
  });
});
