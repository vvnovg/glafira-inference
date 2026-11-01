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

/** Шаги делений оси Y — 1, 2, 2,5, 5 × 10ⁿ, не мельче `minStep`: формат графика не склеивает соседние подписи. */
export function axisIncrs(minStep: number): number[] {
  const out: number[] = [];
  for (let e = -3; e <= 12; e++) {
    for (const m of [1, 2, 2.5, 5]) {
      const v = Number((m * 10 ** e).toPrecision(2));
      if (v >= minStep) out.push(v);
    }
  }
  return out;
}

/**
 * Наименьшая ширина оси Y: в неё входят обычные подписи («2 000 мс», «80 %», «1,5»), поэтому графики в колонке
 * начинаются с одного места, а не каждый по ширине своих подписей.
 */
export const AXIS_MIN = 72;

/** Ширина оси Y в пикселях — по самой длинной подписи (`measure` — ширина текста), не меньше `AXIS_MIN`. */
export function axisSize(labels: string[] | null, measure: (s: string) => number): number {
  const widest = Math.max(0, ...(labels ?? []).map(measure));
  return Math.max(AXIS_MIN, Math.ceil(widest) + 16);
}

/** Часть uPlot, которой рисуется граница: холст, область графика и перевод времени в пиксели. */
export interface MarkerTarget {
  ctx: CanvasRenderingContext2D;
  bbox: { left: number; top: number; width: number; height: number };
  valToPos: (v: number, scale: string, canvasPixels?: boolean) => number;
}

/** Пунктирная вертикаль на времени `at` (мс) внутри области графика; вне её и без `at` — ничего. */
export function drawMarker(u: MarkerTarget, at: number | null, color: string) {
  if (at == null) return;
  const x = u.valToPos(at / 1000, "x", true);
  const { left, top, width, height } = u.bbox;
  if (!(x >= left && x <= left + width)) return;
  const ctx = u.ctx;
  ctx.save();
  ctx.strokeStyle = color;
  ctx.setLineDash([4, 4]);
  ctx.beginPath();
  ctx.moveTo(x, top);
  ctx.lineTo(x, top + height);
  ctx.stroke();
  ctx.restore();
}
