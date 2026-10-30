import { useEffect, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";
import uPlot from "uplot";
import "uplot/dist/uPlot.min.css";
import { type ChartPick, chartData, hasValues, themeColor } from "./chart";
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

interface Props {
  title: string;
  points: MetricPoint[];
  picks: ChartPick[];
  format: (v: number) => string;
}

/**
 * График метрики сервера за окно «сейчас» (спец. I1b §6): ряды по `picks`, разрывы на ошибках опроса, «нет данных»,
 * если значений нет. По образцу `UsageChart` Kubernetes-версии: цвета из CSS-переменных темы, ширина — по контейнеру.
 */
export function MetricChart({ title, points, picks, format }: Props) {
  const { t } = useTranslation();
  const ref = useRef<HTMLDivElement>(null);
  const data = useMemo(() => chartData(points, picks), [points, picks]);
  const empty = !hasValues(data);
  const labels = picks.map((p) => p.label).join("\n");
  const plotRef = useRef<uPlot | null>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el || empty) return;
    const muted = themeColor("--muted", "#6b7280");
    const axis = { stroke: muted, grid: { stroke: themeColor("--border", "#e3e5e8") }, ticks: { stroke: muted } };
    const value = (_u: uPlot, v: number | null) => (v == null ? "—" : format(v));
    const plot = new uPlot(
      {
        width: el.clientWidth || 600,
        height: HEIGHT,
        title,
        axes: [axis, { ...axis, size: 60, values: (_u: uPlot, vals: number[]) => vals.map(format) }],
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
      [[], ...labels.split("\n").map(() => [])] as unknown as uPlot.AlignedData,
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
  }, [title, labels, format, empty]);

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
