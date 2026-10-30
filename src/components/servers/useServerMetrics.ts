import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { api, toAppError } from "../../api/client";
import type { AppError, ServerFacts, ServerMetrics } from "../../api/types";
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
  if (facts.kind === "openai") return "servers.tiles.openaiNoMetrics";
  if (!facts.metrics) return facts.kind === "sglang" ? "servers.tiles.sglangEnableMetrics" : "servers.tiles.noSeries";
  return undefined;
}

export function metricsQueryKey(installationId: string, serverId: string) {
  return ["server-metrics", installationId, serverId] as const;
}

/**
 * Метрики сервера «сейчас» и буфер точек за час. Один запрос на сервер: карточка инсталляции и вкладка сервера
 * делят его (общий ключ react-query), сервер опрашивается один раз за интервал.
 */
export function useServerMetrics(installationId: string, serverId: string, enabled = true) {
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
  const points = buf.key === key ? buf.points : EMPTY;
  return { latest: q.data, points, error: q.error ? toAppError(q.error) : null, isPending: q.isPending, refetch: q.refetch };
}
