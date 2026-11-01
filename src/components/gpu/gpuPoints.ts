import type { TFunction } from "i18next";
import type { GpuHistoryAnswer, GpuValues } from "../../api/types";
import type { ChartPick, ChartPoint } from "../servers/chart";
import { fmtNumber } from "../servers/format";
import type { GpuNowPoint } from "./useGpu";

/** Значения карт в момент времени: номер карты → поля. */
export type Cards = Record<number, GpuValues>;

/**
 * Точки графика из рядов карт: объединение моментов времени по порядку; карта без точки в момент — без значения.
 * Момент, где у всех карт ошибка опроса, — точка без данных (разрыв).
 */
export function mergeCards(series: Record<number, GpuNowPoint[]>): ChartPoint<Cards>[] {
  const at = new Map<number, Cards | null>();
  for (const [index, points] of Object.entries(series))
    for (const p of points) {
      if (!at.has(p.at)) at.set(p.at, null);
      if (p.values) at.set(p.at, { ...(at.get(p.at) ?? {}), [Number(index)]: p.values });
    }
  return [...at.entries()].sort((a, b) => a[0] - b[0]).map(([t, m]) => ({ at: t, m }));
}

/** Ряды окна истории хоста — в форме рядов «Сейчас». */
export function historyCards(answer: GpuHistoryAnswer | undefined, host: string): Record<number, GpuNowPoint[]> {
  const out: Record<number, GpuNowPoint[]> = {};
  for (const s of answer?.series ?? []) if (s.host === host) out[s.gpu] = s.points;
  return out;
}

export interface GpuChart {
  id: "util" | "memory" | "temp" | "power" | "clocks";
  picks: ChartPick<Cards>[];
  format: (v: number) => string;
  step: number;
}

/**
 * Графики вкладки GPU (спека I1d §5). У одной карты — поля рядом (утилизация и SM, занято и всего, температура карты
 * и памяти, такты SM и памяти); у хоста — ряд на карту по главному полю графика.
 */
export function gpuCharts(t: TFunction, indices: number[], single: boolean): GpuChart[] {
  const pct = (v: number) => t("gpu.units.percent", { value: fmtNumber(v) });
  const gib = (v: number) => t("gpu.units.gib", { value: fmtNumber(v / 1024, 1) });
  const deg = (v: number) => t("gpu.units.celsius", { value: fmtNumber(v) });
  const watt = (v: number) => t("gpu.units.watt", { value: fmtNumber(v) });
  const mhz = (v: number) => t("gpu.units.mhz", { value: fmtNumber(v) });
  const one = indices[0];
  const field = (label: string, f: keyof GpuValues): ChartPick<Cards> => ({ label, pick: (m) => m[one]?.[f] });
  const perCard = (f: keyof GpuValues): ChartPick<Cards>[] => indices.map((i) => ({ label: `GPU ${i}`, pick: (m) => m[i]?.[f] }));
  const charts: GpuChart[] = single
    ? [
        { id: "util", picks: [field(t("gpu.charts.util"), "util"), field(t("gpu.charts.sm"), "smActive")], format: pct, step: 1 },
        { id: "memory", picks: [field(t("gpu.charts.used"), "memUsed"), field(t("gpu.charts.total"), "memTotal")], format: gib, step: 102.4 },
        { id: "temp", picks: [field(t("gpu.charts.gpuTemp"), "temp"), field(t("gpu.charts.memTemp"), "memTemp")], format: deg, step: 1 },
        { id: "power", picks: [field(t("gpu.charts.power"), "power")], format: watt, step: 1 },
        { id: "clocks", picks: [field(t("gpu.charts.smClock"), "smClock"), field(t("gpu.charts.memClock"), "memClock")], format: mhz, step: 1 },
      ]
    : [
        { id: "util", picks: perCard("util"), format: pct, step: 1 },
        { id: "memory", picks: perCard("memUsed"), format: gib, step: 102.4 },
        { id: "temp", picks: perCard("temp"), format: deg, step: 1 },
        { id: "power", picks: perCard("power"), format: watt, step: 1 },
        { id: "clocks", picks: perCard("smClock"), format: mhz, step: 1 },
      ];
  return charts;
}
