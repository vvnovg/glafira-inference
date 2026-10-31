import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api/client", async (importOriginal) => {
  const orig = await importOriginal<typeof import("../../api/client")>();
  return { ...orig, api: { readServerMetrics: vi.fn(), readServerHistory: vi.fn() } };
});

import { api } from "../../api/client";
import { TabActiveContext } from "../../lib/tabActive";
import { metrics } from "./testMetrics";
import { appendPoint, historyQueryKey, type MetricPoint, prependHistory, useServerMetrics, WINDOW_MS, withHistory } from "./useServerMetrics";

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

describe("история в буфере и плитках", () => {
  it("точки истории — только раньше первой живой", () => {
    const b = prependHistory([point(30_000), point(35_000)], [point(15_000), point(30_000)]);
    expect(b.map((p) => p.at)).toEqual([15_000, 30_000, 35_000]);
    expect(prependHistory([], [point(1)]).map((p) => p.at)).toEqual([1]);
  });

  it("первый живой снимок берёт скорости и перцентили за минуту из истории", () => {
    const hist = metrics(10_000);
    const first = metrics(20_000, { intervalSec: null, rates: null, latency1m: { ttft: null, itl: null, e2e: null, queueTime: null }, preemptions1m: null });
    const { latest, fromHistory } = withHistory(first, hist);
    expect(fromHistory).toBe(true);
    expect(latest?.at).toBe(20_000);
    expect(latest?.gauges).toEqual(first.gauges);
    expect(latest?.rates).toEqual(hist.rates);
    expect(latest?.latency1m).toEqual(hist.latency1m);
    expect(withHistory(metrics(25_000), hist)).toEqual({ latest: metrics(25_000), fromHistory: false });
    expect(withHistory(undefined, hist)).toEqual({ latest: hist, fromHistory: true });
    expect(withHistory(first, undefined)).toEqual({ latest: first, fromHistory: false });
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
  beforeEach(() => {
    vi.mocked(api.readServerMetrics).mockReset();
    vi.mocked(api.readServerHistory).mockReset();
  });

  it("с историей буфер начинается последними 15 минутами из неё", async () => {
    const t0 = Date.now();
    vi.mocked(api.readServerHistory).mockResolvedValue({
      window: "now",
      stepSec: 15,
      source: "prometheus",
      points: [{ at: t0 - 30_000, m: metrics(t0 - 30_000) }, { at: t0 - 15_000, m: metrics(t0 - 15_000) }],
      partial: [],
    });
    vi.mocked(api.readServerMetrics).mockResolvedValue(metrics(t0, { intervalSec: null, rates: null }));
    const { result } = renderHook(() => useServerMetrics("i1", "s1", true, true), { wrapper: wrapper(true) });
    await waitFor(() => expect(result.current.points).toHaveLength(3));
    expect(api.readServerHistory).toHaveBeenCalledWith("i1", "s1", "now");
    expect(result.current.points.map((p) => p.at)).toEqual([t0 - 30_000, t0 - 15_000, t0]);
    expect(result.current.historyUntil).toBe(t0 - 15_000);
    expect(result.current.fromHistory).toBe(true);
    expect(result.current.latest?.rates?.promptTokens).toBe(120);
  });

  it("ответ истории старше минуты из кеша не берётся: при новом монтировании запрос уходит снова", async () => {
    const t0 = Date.now();
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const stale = { window: "now" as const, stepSec: 15, source: "prometheus" as const, points: [{ at: t0 - 400_000, m: metrics(t0 - 400_000) }], partial: [] };
    client.setQueryData(historyQueryKey("i1", "s1", "now"), stale, { updatedAt: t0 - 120_000 });
    vi.mocked(api.readServerHistory).mockResolvedValue({ ...stale, points: [{ at: t0 - 15_000, m: metrics(t0 - 15_000) }] });
    vi.mocked(api.readServerMetrics).mockResolvedValue(metrics(t0));
    const w = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={client}>
        <TabActiveContext.Provider value={true}>{children}</TabActiveContext.Provider>
      </QueryClientProvider>
    );
    const { result } = renderHook(() => useServerMetrics("i1", "s1", true, true), { wrapper: w });
    await waitFor(() => expect(api.readServerHistory).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(result.current.historyUntil).toBe(t0 - 15_000));
  });

  it("при disabled ошибка истории не показывается", async () => {
    vi.mocked(api.readServerHistory).mockRejectedValue({ code: "TIMEOUT", key: "errors.server.timeout", message: "late", retryable: true });
    vi.mocked(api.readServerMetrics).mockResolvedValue(metrics(Date.now()));
    const { result, rerender } = renderHook(({ e }) => useServerMetrics("i1", "s1", e, true), {
      wrapper: wrapper(true),
      initialProps: { e: true },
    });
    await waitFor(() => expect(result.current.historyError?.code).toBe("TIMEOUT"));
    rerender({ e: false });
    expect(result.current.historyError).toBeNull();
  });

  it("без истории она не запрашивается", async () => {
    vi.mocked(api.readServerMetrics).mockResolvedValue(metrics(Date.now()));
    const { result } = renderHook(() => useServerMetrics("i1", "s1"), { wrapper: wrapper(true) });
    await waitFor(() => expect(result.current.points).toHaveLength(1));
    expect(api.readServerHistory).not.toHaveBeenCalled();
    expect(result.current.historyUntil).toBeNull();
  });

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
