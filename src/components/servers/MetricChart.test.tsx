import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

interface FakePlot {
  opts: { title: string; series: { label?: string }[] };
  data: unknown;
  destroy: () => void;
  setSize: () => void;
  setData: (d: unknown) => void;
}
const plots: FakePlot[] = [];
vi.mock("uplot", () => ({
  default: vi.fn().mockImplementation(function (this: object, opts: FakePlot["opts"], data: unknown) {
    const p: FakePlot = { opts, data, destroy: vi.fn(), setSize: vi.fn(), setData: vi.fn((d) => (p.data = d)) };
    plots.push(p);
    return p;
  }),
}));
globalThis.ResizeObserver ??= class {
  observe() {}
  disconnect() {}
  unobserve() {}
} as unknown as typeof ResizeObserver;

import { axisIncrs, axisSize, chartData, hasValues } from "./chart";
import { MetricChart } from "./MetricChart";
import { ServerMetricsView } from "./ServerMetricsView";
import { metrics } from "./testMetrics";
import type { MetricPoint } from "./useServerMetrics";

const picks = [
  { label: "running", pick: (m: ReturnType<typeof metrics>) => m.gauges.requestsRunning },
  { label: "waiting", pick: (m: ReturnType<typeof metrics>) => m.gauges.requestsWaiting },
];

describe("ось Y", () => {
  it("шаг делений не мельче точности формата", () => {
    const incrs = axisIncrs(0.1);
    expect(incrs[0]).toBe(0.1);
    expect(incrs).toContain(0.25);
    expect(incrs).toContain(1000);
    expect(incrs.every((v) => v >= 0.1)).toBe(true);
    expect(axisIncrs(1)[0]).toBe(1);
  });

  it("ширина оси — по самой длинной подписи", () => {
    const measure = (s: string) => s.length * 7;
    expect(axisSize(["0 мс", "10 000 мс"], measure)).toBe(9 * 7 + 16);
    expect(axisSize(["0"], measure)).toBe(40);
    expect(axisSize(null, measure)).toBe(40);
  });
});

describe("chartData", () => {
  it("время в секундах, ошибка опроса и пропуск значения — null", () => {
    const pts: MetricPoint[] = [
      { at: 5_000, m: metrics(5_000) },
      { at: 10_000, m: null },
      { at: 15_000, m: metrics(15_000, { gauges: { requestsRunning: null, requestsWaiting: 2, kvUsage: null, kvUsageAvg: null } }) },
    ];
    expect(chartData(pts, picks)).toEqual([
      [5, 10, 15],
      [3, null, null],
      [1, null, 2],
    ]);
    expect(hasValues(chartData([{ at: 1, m: null }], picks))).toBe(false);
  });
});

describe("MetricChart", () => {
  it("ряды по подписям, новые точки — setData; при размонтировании график уничтожается", () => {
    const pts: MetricPoint[] = [{ at: 1_000, m: metrics(1_000) }];
    const { rerender, unmount } = render(<MetricChart title="Запросы" points={pts} picks={picks} format={String} step={1} />);
    expect(plots).toHaveLength(1);
    expect(plots[0].opts.series.map((s) => s.label)).toEqual([undefined, "running", "waiting"]);
    expect(plots[0].data).toEqual([[1], [3], [1]]);
    rerender(<MetricChart title="Запросы" points={[...pts, { at: 6_000, m: metrics(6_000) }]} picks={picks} format={String} step={1} />);
    expect(plots).toHaveLength(1);
    expect(plots[0].data).toEqual([
      [1, 6],
      [3, 3],
      [1, 1],
    ]);
    unmount();
    expect(plots[0].destroy).toHaveBeenCalled();
  });

  it("пересоздание при смене заголовка не теряет данные", () => {
    plots.length = 0;
    const pts: MetricPoint[] = [{ at: 1_000, m: metrics(1_000) }];
    const { rerender } = render(<MetricChart title="Запросы" points={pts} picks={picks} format={String} step={1} />);
    expect(plots).toHaveLength(1);
    rerender(<MetricChart title="Requests" points={pts} picks={picks} format={String} step={1} />);
    expect(plots).toHaveLength(2);
    expect(plots[1].data).toEqual([[1], [3], [1]]);
  });

  it("новая функция format график не пересоздаёт", () => {
    plots.length = 0;
    const pts: MetricPoint[] = [{ at: 1_000, m: metrics(1_000) }];
    const { rerender } = render(<MetricChart title="Запросы" points={pts} picks={picks} format={String} step={1} />);
    rerender(<MetricChart title="Запросы" points={pts} picks={picks} format={(v) => `${v}`} step={1} />);
    expect(plots).toHaveLength(1);
  });

  it("без значений — «нет данных», график не строится", () => {
    plots.length = 0;
    render(<MetricChart title="KV" points={[{ at: 1, m: null }]} picks={picks} format={String} step={1} />);
    expect(screen.getByText("KV")).toBeTruthy();
    expect(screen.getByText(/no data|нет данных/i)).toBeTruthy();
    expect(plots).toHaveLength(0);
  });
});

describe("ServerMetricsView", () => {
  const finished = (reasons: Record<string, number>, aborted: number | null) =>
    metrics(1_000, { rates: { promptTokens: 1, generationTokens: 1, requests: 1, preemptions: 0, aborted, finishedByReason: reasons } });
  const finishedLabels = () => plots.filter((p) => p.opts.title === "Завершения в секунду").pop()?.opts.series.map((s) => s.label);

  it("«Завершения»: прерванные запросы vLLM — одна линия, причина abort не дублирует «прерваны»", () => {
    plots.length = 0;
    render(<ServerMetricsView points={[{ at: 1_000, m: finished({ stop: 2, abort: 1 }, 1) }]} />);
    expect(finishedLabels()).toEqual([undefined, "stop", "прерваны"]);
  });

  it("«Завершения»: без отдельного счётчика прерываний причина abort остаётся", () => {
    plots.length = 0;
    render(<ServerMetricsView points={[{ at: 1_000, m: finished({ stop: 2, abort: 1 }, null) }]} />);
    expect(finishedLabels()).toEqual([undefined, "abort", "stop", "прерваны"]);
  });
});
