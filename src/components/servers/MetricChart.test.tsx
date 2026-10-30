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

import { chartData, hasValues } from "./chart";
import { MetricChart } from "./MetricChart";
import { metrics } from "./testMetrics";
import type { MetricPoint } from "./useServerMetrics";

const picks = [
  { label: "running", pick: (m: ReturnType<typeof metrics>) => m.gauges.requestsRunning },
  { label: "waiting", pick: (m: ReturnType<typeof metrics>) => m.gauges.requestsWaiting },
];

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
    const { rerender, unmount } = render(<MetricChart title="Запросы" points={pts} picks={picks} format={String} />);
    expect(plots).toHaveLength(1);
    expect(plots[0].opts.series.map((s) => s.label)).toEqual([undefined, "running", "waiting"]);
    expect(plots[0].data).toEqual([[1], [3], [1]]);
    rerender(<MetricChart title="Запросы" points={[...pts, { at: 6_000, m: metrics(6_000) }]} picks={picks} format={String} />);
    expect(plots).toHaveLength(1);
    expect(plots[0].data).toEqual([
      [1, 6],
      [3, 3],
      [1, 1],
    ]);
    unmount();
    expect(plots[0].destroy).toHaveBeenCalled();
  });

  it("без значений — «нет данных», график не строится", () => {
    plots.length = 0;
    render(<MetricChart title="KV" points={[{ at: 1, m: null }]} picks={picks} format={String} />);
    expect(screen.getByText("KV")).toBeTruthy();
    expect(screen.getByText(/no data|нет данных/i)).toBeTruthy();
    expect(plots).toHaveLength(0);
  });
});
