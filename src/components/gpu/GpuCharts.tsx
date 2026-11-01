import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { toAppError } from "../../api/client";
import type { HistoryWindow } from "../../api/types";
import { currentLocale } from "../../i18n";
import { tr } from "../../i18n/text";
import { ErrorBanner } from "../common/ErrorBanner";
import { MetricChart } from "../servers/MetricChart";
import { type Cards, gpuCharts, historyCards, mergeCards } from "./gpuPoints";
import { type CardRef, type GpuBuffer, nowSeries, useGpuHistory } from "./useGpu";

/** Время «HH:MM» по языку интерфейса. */
const hhmm = (at: number) => new Date(at).toLocaleTimeString(currentLocale(), { hour: "2-digit", minute: "2-digit" });

interface Props {
  installationId: string;
  host: string;
  /** Карта; `null` — все карты хоста (не больше 16). */
  gpu: number | null;
  /** Экспортёр выбранной карты (если у хоста повторяется номер карты). */
  exporter?: string;
  /** Карты хоста. */
  cards: CardRef[];
  buffer: GpuBuffer;
  window: HistoryWindow;
  /** `null` — история доступна; иначе ключ подсказки, почему нет. */
  historyUnavailable: string | null;
}

/**
 * Графики карты или хоста (спека I1d §5): окно «Сейчас» — буфер опроса с историей за 15 минут до него, окна 1 ч…30 д —
 * из источника истории. Частичное окно и ошибка — как у графиков сервера.
 */
export function GpuCharts({ installationId, host, gpu, exporter, cards, buffer, window: win, historyUnavailable }: Props) {
  const { t } = useTranslation();
  const now = win === "now";
  const shown: CardRef[] = gpu == null ? cards.slice(0, 16) : [{ index: gpu, exporter }];
  const shownIndices = [...new Set(shown.map((c) => c.index))];
  const prefill = useGpuHistory(installationId, "now", host, null, now && !historyUnavailable);
  const hist = useGpuHistory(installationId, now ? "1h" : win, host, gpu, !now && !historyUnavailable);
  const series = now ? nowSeries(buffer, host, shown, prefill.data) : historyCards(hist.data, host);
  const points = mergeCards(Object.fromEntries(shownIndices.map((i) => [i, series[i] ?? []])));
  const indicesKey = shownIndices.join(",");
  const charts = useMemo(() => gpuCharts(t, indicesKey.split(",").map(Number), gpu != null), [t, indicesKey, gpu]);
  const until = now ? (prefill.data?.series.flatMap((s) => (s.host === host ? s.points.map((p) => p.at) : [])) ?? []) : [];
  const marker = until.length > 0 ? Math.max(...until) : null;
  const firstLoad = !now && !hist.error && !hist.data && hist.isFetching;
  const step = (sec: number) => (sec % 60 === 0 ? t("servers.units.min", { value: sec / 60 }) : t("servers.units.sec", { value: sec }));
  return (
    <div className="gpu-charts">
      {now && marker != null && <p className="muted">{t("servers.metrics.historyUntil", { time: hhmm(marker) })}</p>}
      {!now && hist.data && (
        <p className="muted">{t("servers.metrics.step", { step: step(hist.data.stepSec), source: t(`servers.metrics.sources.${hist.data.source}`) })}</p>
      )}
      {!now && hist.error && (
        <ErrorBanner error={toAppError(hist.error)} onRetry={toAppError(hist.error).retryable ? () => void hist.refetch() : undefined} />
      )}
      {!now && hist.data && hist.data.partial.length > 0 && (
        <p className="warn" role="status">
          {t("gpu.partial", {
            charts: hist.data.partial.map((x) => `${t(`gpu.charts.${x.chart}`)} — ${tr(x.error)}`).join("; "),
          })}
        </p>
      )}
      {now && prefill.error && <p className="muted">{t("servers.metrics.prefillFailed", { error: tr(toAppError(prefill.error)) })}</p>}
      {firstLoad && (
        <p className="muted" role="status">
          {t("common.loading")}
        </p>
      )}
      {!(!now && hist.error) && !firstLoad && (
        <div className={!now && hist.isFetching ? "charts loading" : "charts"} aria-busy={!now && hist.isFetching}>
          {charts.map((c) => (
            <MetricChart<Cards> key={c.id} title={t(gpu != null && c.id === "util" ? "gpu.charts.utilSm" : `gpu.charts.${c.id}`)} points={points} picks={c.picks} format={c.format} step={c.step} marker={now ? marker : null} />
          ))}
        </div>
      )}
    </div>
  );
}
