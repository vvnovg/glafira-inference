import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { ServerMetrics } from "../../api/types";
import type { ChartPick } from "./chart";
import { fmtAdaptive, fmtNumber, fmtPercent } from "./format";
import { MetricChart } from "./MetricChart";
import type { MetricPoint } from "./useServerMetrics";

/** Окна метрик: «сейчас» — из буфера опроса; история — с этапа I1c. */
const WINDOWS = ["now", "1h", "24h", "7d", "30d"] as const;

const lat = (k: "ttft" | "itl" | "e2e", q: "p50" | "p95" | "p99") => (m: ServerMetrics) => m.latency[k]?.[q];

/** «Метрики»: графики за окно «сейчас» (до часа в памяти вкладки). */
export function ServerMetricsView({ points }: { points: MetricPoint[] }) {
  const { t } = useTranslation();
  // Причины завершения — из буфера; от точек зависит только их список, иначе графики пересоздавались бы каждый опрос.
  const reasonsKey = [...new Set(points.flatMap((x) => Object.keys(x.m?.rates?.finishedByReason ?? {})))].sort().join("\n");
  const charts = useMemo(() => {
    const p = (label: string, pick: ChartPick["pick"]): ChartPick => ({ label, pick });
    const latency = (k: "ttft" | "itl" | "e2e") => [p("p50", lat(k, "p50")), p("p95", lat(k, "p95")), p("p99", lat(k, "p99"))];
    const reasons = reasonsKey ? reasonsKey.split("\n") : [];
    return [
      { id: "requests", picks: [p(t("servers.charts.running"), (m) => m.gauges.requestsRunning), p(t("servers.charts.waiting"), (m) => m.gauges.requestsWaiting)], format: (v: number) => fmtNumber(v, 1) },
      { id: "kv", picks: [p(t("servers.charts.max"), (m) => m.gauges.kvUsage), p(t("servers.charts.avg"), (m) => m.gauges.kvUsageAvg)], format: fmtPercent },
      { id: "preemptions", picks: [p(t("servers.charts.perSec"), (m) => m.rates?.preemptions)], format: fmtAdaptive },
      { id: "tokens", picks: [p(t("servers.charts.prompt"), (m) => m.rates?.promptTokens), p(t("servers.charts.generation"), (m) => m.rates?.generationTokens)], format: fmtAdaptive },
      { id: "prefixCache", picks: [p(t("servers.charts.hit"), (m) => m.prefixCacheHitRatio)], format: fmtPercent },
      { id: "ttft", picks: latency("ttft"), format: (v: number) => t("servers.units.ms", { value: fmtNumber(v) }) },
      { id: "itl", picks: latency("itl"), format: (v: number) => t("servers.units.ms", { value: fmtNumber(v) }) },
      { id: "e2e", picks: latency("e2e"), format: (v: number) => t("servers.units.ms", { value: fmtNumber(v) }) },
      { id: "queue", picks: [p("p95", (m) => m.latency.queueTime?.p95)], format: (v: number) => t("servers.units.ms", { value: fmtNumber(v) }) },
      {
        id: "finished",
        picks: [...reasons.map((r) => p(r, (m: ServerMetrics) => m.rates?.finishedByReason[r])), p(t("servers.charts.aborted"), (m) => m.rates?.aborted)],
        format: fmtAdaptive,
      },
    ];
  }, [reasonsKey, t]);
  return (
    <div className="server-metrics">
      <div className="toolbar" role="group" aria-label={t("servers.metrics.window")}>
        {WINDOWS.map((w) => (
          <button key={w} type="button" className={w === "now" ? "active" : undefined} aria-pressed={w === "now"} disabled={w !== "now"} title={w === "now" ? undefined : t("servers.metrics.historyLater")}>
            {t(`servers.metrics.windows.${w}`)}
          </button>
        ))}
      </div>
      <div className="charts">
        {charts.map((c) => (
          <MetricChart key={c.id} title={t(`servers.charts.${c.id}`)} points={points} picks={c.picks} format={c.format} />
        ))}
      </div>
    </div>
  );
}
