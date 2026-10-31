import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { toAppError } from "../../api/client";
import type { AppError, HistoryWindow, ServerMetrics } from "../../api/types";
import { currentLocale } from "../../i18n";
import { ErrorBanner } from "../common/ErrorBanner";
import type { ChartPick } from "./chart";
import { fmtAdaptive, fmtNumber, fmtPercent } from "./format";
import { MetricChart } from "./MetricChart";
import { type MetricPoint, useServerHistory } from "./useServerMetrics";

/** Окна метрик: «сейчас» — из буфера опроса (и истории до него), остальные — из источника истории. */
const WINDOWS: HistoryWindow[] = ["now", "1h", "24h", "7d", "30d"];

const lat = (k: "ttft" | "itl" | "e2e", q: "p50" | "p95" | "p99") => (m: ServerMetrics) => m.latency[k]?.[q];

const EMPTY: MetricPoint[] = [];

/** Время «HH:MM» по языку интерфейса. */
const hhmm = (at: number) => new Date(at).toLocaleTimeString(currentLocale(), { hour: "2-digit", minute: "2-digit" });

interface Props {
  installationId: string;
  serverId: string;
  /** Буфер «Сейчас». */
  points: MetricPoint[];
  /** До какого времени буфер заполнен из истории; `null` — не заполнялся. */
  historyUntil: number | null;
  /** Почему буфер не заполнен из истории. */
  historyError: AppError | null;
  /** `null` — история сервера доступна; иначе ключ подсказки, почему нет. */
  historyUnavailable: string | null;
}

/** «Метрики» (спека I1c §5): графики за окно «сейчас» или окно истории 1 ч…30 д. */
export function ServerMetricsView({ installationId, serverId, points, historyUntil, historyError, historyUnavailable }: Props) {
  const { t } = useTranslation();
  const [win, setWin] = useState<HistoryWindow>("now");
  const fromHistory = win !== "now";
  const hist = useServerHistory(installationId, serverId, fromHistory ? win : "1h", fromHistory && !historyUnavailable);
  const shown = fromHistory ? (hist.data?.points ?? EMPTY) : points;
  // Причины завершения — из точек; от точек зависит только их список, иначе графики пересоздавались бы каждый опрос.
  // Прерывания vLLM приходят и причиной `abort`, и отдельным счётчиком: если счётчик есть, причину не рисуем второй раз.
  const hasAborted = shown.some((x) => x.m?.rates?.aborted != null);
  const reasonsKey = [...new Set(shown.flatMap((x) => Object.keys(x.m?.rates?.finishedByReason ?? {})))]
    .filter((r) => !(hasAborted && r === "abort"))
    .sort()
    .join("\n");
  const charts = useMemo(() => {
    const p = (label: string, pick: ChartPick["pick"]): ChartPick => ({ label, pick });
    const latency = (k: "ttft" | "itl" | "e2e") => [p("p50", lat(k, "p50")), p("p95", lat(k, "p95")), p("p99", lat(k, "p99"))];
    const reasons = reasonsKey ? reasonsKey.split("\n") : [];
    return [
      { id: "requests", picks: [p(t("servers.charts.running"), (m) => m.gauges.requestsRunning), p(t("servers.charts.waiting"), (m) => m.gauges.requestsWaiting)], format: (v: number) => fmtNumber(v, 1), step: 1 },
      { id: "kv", picks: [p(t("servers.charts.max"), (m) => m.gauges.kvUsage), p(t("servers.charts.avg"), (m) => m.gauges.kvUsageAvg)], format: fmtPercent, step: 0.01 },
      { id: "preemptions", picks: [p(t("servers.charts.perSec"), (m) => m.rates?.preemptions)], format: fmtAdaptive, step: 0.1 },
      { id: "tokens", picks: [p(t("servers.charts.prompt"), (m) => m.rates?.promptTokens), p(t("servers.charts.generation"), (m) => m.rates?.generationTokens)], format: fmtAdaptive, step: 0.1 },
      { id: "prefixCache", picks: [p(t("servers.charts.hit"), (m) => m.prefixCacheHitRatio)], format: fmtPercent, step: 0.01 },
      { id: "ttft", picks: latency("ttft"), format: (v: number) => t("servers.units.ms", { value: fmtNumber(v) }), step: 1 },
      { id: "itl", picks: latency("itl"), format: (v: number) => t("servers.units.ms", { value: fmtNumber(v) }), step: 1 },
      { id: "e2e", picks: latency("e2e"), format: (v: number) => t("servers.units.ms", { value: fmtNumber(v) }), step: 1 },
      { id: "queue", picks: [p("p95", (m) => m.latency.queueTime?.p95)], format: (v: number) => t("servers.units.ms", { value: fmtNumber(v) }), step: 1 },
      {
        id: "finished",
        picks: [...reasons.map((r) => p(r, (m: ServerMetrics) => m.rates?.finishedByReason[r])), p(t("servers.charts.aborted"), (m) => m.rates?.aborted)],
        format: fmtAdaptive,
        step: 0.1,
      },
    ];
  }, [reasonsKey, t]);
  const title = (w: HistoryWindow) => {
    if (w === "now") return historyError ? t("servers.metrics.prefillFailed", { error: historyError.message }) : undefined;
    return historyUnavailable ? t(historyUnavailable) : undefined;
  };
  const step = (sec: number) => (sec % 60 === 0 ? t("servers.units.min", { value: sec / 60 }) : t("servers.units.sec", { value: sec }));
  return (
    <div className="server-metrics">
      <div className="toolbar" role="group" aria-label={t("servers.metrics.window")}>
        {WINDOWS.map((w) => (
          <button
            key={w}
            type="button"
            className={w === win ? "active" : undefined}
            aria-pressed={w === win}
            disabled={w !== "now" && !!historyUnavailable}
            title={title(w)}
            onClick={() => setWin(w)}
          >
            {t(`servers.metrics.windows.${w}`)}
          </button>
        ))}
        {fromHistory && (
          <button type="button" onClick={() => void hist.refetch()} disabled={hist.isFetching}>
            {t("common.refresh")}
          </button>
        )}
        {!fromHistory && historyUntil != null && <span className="muted">{t("servers.metrics.historyUntil", { time: hhmm(historyUntil) })}</span>}
        {fromHistory && hist.data && (
          <span className="muted">{t("servers.metrics.step", { step: step(hist.data.stepSec), source: t(`servers.metrics.sources.${hist.data.source}`) })}</span>
        )}
      </div>
      {fromHistory && hist.error && <ErrorBanner error={toAppError(hist.error)} onRetry={() => void hist.refetch()} />}
      {fromHistory && hist.data && hist.data.partial.length > 0 && (
        <p className="warn" role="status">
          {t("servers.metrics.partial", {
            charts: hist.data.partial.map((x) => `${t(`servers.charts.${x.chart}`)} — ${x.error.message}`).join("; "),
          })}
        </p>
      )}
      {!(fromHistory && hist.error) && (
        <div className={fromHistory && hist.isFetching ? "charts loading" : "charts"} aria-busy={fromHistory && hist.isFetching}>
          {charts.map((c) => (
            <MetricChart
              key={c.id}
              title={t(`servers.charts.${c.id}`)}
              points={shown}
              picks={c.picks}
              format={c.format}
              step={c.step}
              marker={fromHistory ? null : historyUntil}
            />
          ))}
        </div>
      )}
    </div>
  );
}
