import { useEffect, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";
import uPlot from "uplot";
import "uplot/dist/uPlot.min.css";
import { type ChartPick, axisIncrs, axisSize, chartData, hasValues, themeColor } from "./chart";
import type { MetricPoint } from "./useServerMetrics";

const PALETTE: [string, string][] = [
  ["--chart-1", "#3b82f6"],
  ["--chart-2", "#10b981"],
  ["--chart-3", "#8b5cf6"],
  ["--chart-4", "#f97316"],
  ["--chart-5", "#ec4899"],
  ["--chart-6", "#14b8a6"],
];

const HEIGHT = 200;
/** Шрифт подписей осей uPlot по умолчанию — по нему считается ширина оси Y. */
const AXIS_FONT = '12px system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, "Noto Sans", sans-serif';

interface Props {
  title: string;
  points: MetricPoint[];
  picks: ChartPick[];
  format: (v: number) => string;
  /** Наименьший шаг делений оси Y — точность `format` (0,1 у десятых, 1 у целых). */
  step: number;
}

/**
 * График метрики сервера за окно «сейчас» (спец. I1b §6): ряды по `picks`, разрывы на ошибках опроса, «нет данных»,
 * если значений нет. По образцу `UsageChart` Kubernetes-версии: цвета из CSS-переменных темы, ширина — по контейнеру.
 */
export function MetricChart({ title, points, picks, format, step }: Props) {
  const { t } = useTranslation();
  const ref = useRef<HTMLDivElement>(null);
  const data = useMemo(() => chartData(points, picks), [points, picks]);
  const empty = !hasValues(data);
  const labels = picks.map((p) => p.label).join("\n");
  const plotRef = useRef<uPlot | null>(null);
  // Последние данные и format — в ref: пересоздание графика (язык, заголовок) не теряет данные, а новая функция format
  // от родителя график не пересоздаёт.
  const dataRef = useRef(data);
  dataRef.current = data;
  const formatRef = useRef(format);
  formatRef.current = format;

  useEffect(() => {
    const el = ref.current;
    if (!el || empty) return;
    const muted = themeColor("--muted", "#6b7280");
    const axis = { stroke: muted, grid: { stroke: themeColor("--border", "#e3e5e8") }, ticks: { stroke: muted } };
    const value = (_u: uPlot, v: number | null) => (v == null ? "—" : formatRef.current(v));
    const plot = new uPlot(
      {
        width: el.clientWidth || 600,
        height: HEIGHT,
        title,
        axes: [
          axis,
          {
            ...axis,
            incrs: axisIncrs(step),
            size: (_u: uPlot, labels: string[] | null) => axisSize(labels, measureText),
            values: (_u: uPlot, vals: number[]) => vals.map((v) => formatRef.current(v)),
          },
        ],
        series: [
          {},
          ...labels.split("\n").map((label, i) => ({
            label,
            stroke: themeColor(...PALETTE[i % PALETTE.length]),
            width: 1.5,
            spanGaps: false,
            value,
          })),
        ],
      },
      dataRef.current as uPlot.AlignedData,
      el,
    );
    plotRef.current = plot;
    const resize = new ResizeObserver(() => plot.setSize({ width: el.clientWidth || 600, height: HEIGHT }));
    resize.observe(el);
    return () => {
      resize.disconnect();
      plot.destroy();
      plotRef.current = null;
    };
  }, [title, labels, empty, step]);

  // Новые точки — без пересоздания графика: setData сохраняет выделение и легенду.
  useEffect(() => {
    plotRef.current?.setData(data as uPlot.AlignedData);
  }, [data]);

  return (
    <section className="metric-chart" aria-label={title}>
      {empty ? (
        <>
          <h4>{title}</h4>
          <p className="empty">{t("servers.metrics.noData")}</p>
        </>
      ) : (
        <div ref={ref} />
      )}
    </section>
  );
}

let measureCtx: CanvasRenderingContext2D | null | undefined;
/** Ширина подписи шрифтом осей; без canvas (тесты) — по числу знаков. */
function measureText(s: string): number {
  if (measureCtx === undefined) {
    try {
      measureCtx = document.createElement("canvas").getContext("2d");
    } catch {
      measureCtx = null;
    }
    if (measureCtx) measureCtx.font = AXIS_FONT;
  }
  return measureCtx ? measureCtx.measureText(s).width : s.length * 7;
}
