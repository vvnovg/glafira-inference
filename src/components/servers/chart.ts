import type { ServerMetrics } from "../../api/types";
import type { MetricPoint } from "./useServerMetrics";

/** Ряд графика: подпись и значение из метрик точки (`null` — нет данных). */
export interface ChartPick {
  label: string;
  pick: (m: ServerMetrics) => number | null | undefined;
}

/**
 * Данные uPlot из буфера точек: ось времени в секундах и ряды по `picks`. Точка с ошибкой опроса или без значения —
 * `null`: на графике разрыв, а не ноль.
 */
export function chartData(points: MetricPoint[], picks: ChartPick[]): (number | null)[][] {
  const xs = points.map((p) => p.at / 1000);
  return [xs, ...picks.map((c) => points.map((p) => (p.m ? (c.pick(p.m) ?? null) : null)))];
}

/** Есть ли в рядах хоть одно значение. */
export function hasValues(data: (number | null)[][]): boolean {
  return data.slice(1).some((row) => row.some((v) => v != null));
}

/** Цвет из CSS-переменной темы (`--chart-1`…); вне браузера — запасной. По образцу Kubernetes-версии. */
export function themeColor(name: string, fallback: string): string {
  const v = typeof document === "undefined" ? "" : getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return v || fallback;
}
