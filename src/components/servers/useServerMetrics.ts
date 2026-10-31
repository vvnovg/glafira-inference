import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { api, toAppError } from "../../api/client";
import type { AppError, HistoryWindow, ServerFacts, ServerMetrics } from "../../api/types";
import { useRefetchInterval } from "../../lib/tabActive";

/** Опрос `/metrics` сервера — раз в 5 с, только пока вкладка видна (спец. §6.6). */
export const POLL_MS = 5_000;
/** Окно «сейчас»: точки за последний час (до 720 при опросе раз в 5 с). */
export const WINDOW_MS = 60 * 60 * 1000;

/** Точка буфера: метрики или ошибка опроса (на графике — разрыв, а не ноль). */
export interface MetricPoint {
  at: number;
  m: ServerMetrics | null;
  error?: AppError;
}

/** Новая точка в буфер; точки старше окна отбрасываются, повтор того же снимка не добавляется. */
export function appendPoint(buf: MetricPoint[], p: MetricPoint, windowMs = WINDOW_MS): MetricPoint[] {
  if (buf.length > 0 && buf[buf.length - 1].at === p.at) return buf;
  const from = p.at - windowMs;
  const kept = buf[0] && buf[0].at < from ? buf.filter((x) => x.at >= from) : buf;
  return [...kept, p];
}

const EMPTY: MetricPoint[] = [];

/** Сервер отдаёт метрики, которые Glafira читает: не OpenAI-совместимый тип и `/metrics` с нужными сериями (иначе опрос только ошибается). */
export function readsMetrics(facts: ServerFacts | undefined): boolean {
  return !!facts && facts.kind !== "openai" && facts.metrics;
}

/** Подсказка, почему метрик нет (`undefined` — они есть). */
export function noMetricsHintKey(facts: ServerFacts): string | undefined {
  if (facts.kind === "openai") return facts.detected === "openai" ? "servers.tiles.openaiNoMetrics" : "servers.tiles.belowFloor";
  if (!facts.metrics) return facts.kind === "sglang" ? "servers.tiles.sglangEnableMetrics" : "servers.tiles.noSeries";
  return undefined;
}

export function metricsQueryKey(installationId: string, serverId: string) {
  return ["server-metrics", installationId, serverId] as const;
}

export function historyQueryKey(installationId: string, serverId: string, window: HistoryWindow) {
  return ["server-history", installationId, serverId, window] as const;
}

/**
 * Окно истории сервера (спека I1c §5): запрашивается при выборе окна, ответ живёт минуту — повторный выбор окна в
 * пределах минуты источник не нагружает. Автоматического перезапроса нет, свежее окно — по «Обновить».
 */
export function useServerHistory(installationId: string, serverId: string, window: HistoryWindow, enabled: boolean) {
  return useQuery({
    queryKey: historyQueryKey(installationId, serverId, window),
    queryFn: () => api.readServerHistory(installationId, serverId, window),
    enabled,
    staleTime: 60_000,
    // Пока грузится другое окно, видны прежние графики (под слоем загрузки).
    placeholderData: keepPreviousData,
    retry: false,
    refetchOnWindowFocus: false,
  });
}

/** Точки истории в начало буфера: только те, что раньше первой живой точки. */
export function prependHistory(buf: MetricPoint[], history: MetricPoint[]): MetricPoint[] {
  const first = buf.length > 0 ? buf[0].at : Number.POSITIVE_INFINITY;
  return [...history.filter((p) => p.at < first), ...buf];
}

/**
 * Метрики для плиток: у первого живого снимка ещё нет скоростей и перцентилей за минуту — до второго опроса они
 * берутся из последней точки истории (`fromHistory`). Без живого снимка — точка истории целиком.
 */
export function withHistory(live: ServerMetrics | undefined, last: ServerMetrics | undefined): { latest: ServerMetrics | undefined; fromHistory: boolean } {
  if (!last) return { latest: live, fromHistory: false };
  if (!live) return { latest: last, fromHistory: true };
  if (live.intervalSec != null) return { latest: live, fromHistory: false };
  return {
    latest: {
      ...live,
      rates: last.rates,
      prefixCacheHitRatio: last.prefixCacheHitRatio,
      latency: last.latency,
      latency1m: last.latency1m,
      preemptions1m: last.preemptions1m,
    },
    fromHistory: true,
  };
}

/**
 * Метрики сервера «сейчас» и буфер точек за час. Один запрос на сервер: карточка инсталляции и вкладка сервера
 * делят его (общий ключ react-query), сервер опрашивается один раз за интервал. `prefill` — у сервера есть история:
 * буфер один раз заполняется последними 15 минутами из неё (`historyUntil` — время последней точки истории).
 */
export function useServerMetrics(installationId: string, serverId: string, enabled = true, prefill = false) {
  const refetchInterval = useRefetchInterval(POLL_MS);
  const q = useQuery({
    queryKey: metricsQueryKey(installationId, serverId),
    queryFn: () => api.readServerMetrics(installationId, serverId),
    enabled,
    refetchInterval: enabled ? refetchInterval : false,
    // Ошибка опроса не повторяется сразу: следующая попытка — по интервалу.
    retry: false,
    refetchOnWindowFocus: false,
  });
  // Буфер привязан к ключу запроса: при смене сервера он начинается заново, точки двух серверов не смешиваются.
  const key = metricsQueryKey(installationId, serverId).join("\u0000");
  const [buf, setBuf] = useState<{ key: string; points: MetricPoint[] }>({ key, points: [] });
  const push = (p: MetricPoint) =>
    setBuf((b) => ({ key, points: appendPoint(b.key === key ? b.points : [], p) }));
  useEffect(() => {
    if (q.data) push({ at: q.data.at, m: q.data });
  }, [key, q.data, q.dataUpdatedAt]);
  useEffect(() => {
    if (q.error && q.errorUpdatedAt) push({ at: q.errorUpdatedAt, m: null, error: toAppError(q.error) });
  }, [key, q.error, q.errorUpdatedAt]);
  // История за 15 минут запрашивается при открытии вкладки, дальше буфер пополняет опрос. Ответ живёт не дольше
  // минуты, как у useServerHistory: при возврате на вкладку старая история из кеша не берётся, между её концом
  // и первой живой точкой не остаётся провала.
  const history = useQuery({
    queryKey: historyQueryKey(installationId, serverId, "now"),
    queryFn: () => api.readServerHistory(installationId, serverId, "now"),
    enabled: enabled && prefill,
    staleTime: 60_000,
    gcTime: 60_000,
    retry: false,
    refetchOnWindowFocus: false,
  });
  const historyPoints = enabled && prefill ? history.data?.points : undefined;
  useEffect(() => {
    if (historyPoints?.length)
      setBuf((b) => ({ key, points: prependHistory(b.key === key ? b.points : [], historyPoints) }));
  }, [key, historyPoints]);
  const points = buf.key === key ? buf.points : EMPTY;
  const last = historyPoints?.length ? historyPoints[historyPoints.length - 1] : undefined;
  const { latest, fromHistory } = withHistory(q.data, last?.m);
  return {
    latest,
    fromHistory,
    historyUntil: last?.at ?? null,
    historyError: enabled && prefill && history.error ? toAppError(history.error) : null,
    points,
    error: q.error ? toAppError(q.error) : null,
    isPending: q.isPending,
    refetch: q.refetch,
  };
}
