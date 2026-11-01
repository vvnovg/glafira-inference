import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { api, toAppError } from "../../api/client";
import type { GpuHistoryAnswer, GpuStatus, GpuValues, HistoryWindow } from "../../api/types";
import { useRefetchInterval } from "../../lib/tabActive";
import { POLL_MS, WINDOW_MS } from "../servers/useServerMetrics";

/** Точка буфера «Сейчас» карты: значения полей или `null` — опрос не удался (на графике разрыв). */
export interface GpuNowPoint {
  at: number;
  values: GpuValues | null;
}

/** Буфер «Сейчас»: карта (`cardKey`) → точки за последний час. */
export type GpuBuffer = Record<string, GpuNowPoint[]>;

export const cardKey = (host: string, index: number) => `${host}\u0000${index}`;

function append(points: GpuNowPoint[] | undefined, p: GpuNowPoint, windowMs: number): GpuNowPoint[] {
  const list = points ?? [];
  if (list.length > 0 && list[list.length - 1].at === p.at) return list;
  const from = p.at - windowMs;
  const kept = list[0] && list[0].at < from ? list.filter((x) => x.at >= from) : list;
  return [...kept, p];
}

/** Снимок карт в буфер: точка каждой карты, старше окна отбрасываются, повтор того же снимка не добавляется. */
export function appendStatus(buf: GpuBuffer, status: GpuStatus, windowMs = WINDOW_MS): GpuBuffer {
  const next: GpuBuffer = { ...buf };
  for (const h of status.hosts)
    for (const c of h.cards) {
      const key = cardKey(h.host, c.index);
      next[key] = append(buf[key], { at: status.at, values: c.values }, windowMs);
    }
  return next;
}

/** Неудачный опрос: разрыв у всех карт буфера. */
export function appendError(buf: GpuBuffer, at: number, windowMs = WINDOW_MS): GpuBuffer {
  const next: GpuBuffer = {};
  for (const [key, points] of Object.entries(buf)) next[key] = append(points, { at, values: null }, windowMs);
  return next;
}

export function gpuStatusKey(installationId: string) {
  return ["gpu-status", installationId] as const;
}

export function gpuHistoryKey(installationId: string, window: HistoryWindow, host: string, gpu: number | null) {
  return ["gpu-history", installationId, window, host, gpu] as const;
}

/**
 * Карты инсталляции «сейчас» (спека I1d §5): опрос раз в 5 с, только пока вкладка видна. Вкладка GPU, «Обзор»
 * инсталляции и «Обзор» сервера делят запрос (общий ключ react-query); буфер точек за час — у каждого хука свой.
 */
export function useGpuStatus(installationId: string, enabled = true) {
  const refetchInterval = useRefetchInterval(POLL_MS);
  const q = useQuery({
    queryKey: gpuStatusKey(installationId),
    queryFn: () => api.readGpuStatus(installationId),
    enabled,
    refetchInterval: enabled ? refetchInterval : false,
    retry: false,
    refetchOnWindowFocus: false,
  });
  const [buf, setBuf] = useState<{ id: string; points: GpuBuffer }>({ id: installationId, points: {} });
  useEffect(() => {
    if (q.data) {
      const status = q.data;
      setBuf((b) => ({ id: installationId, points: appendStatus(b.id === installationId ? b.points : {}, status) }));
    }
  }, [installationId, q.data, q.dataUpdatedAt]);
  useEffect(() => {
    if (q.error && q.errorUpdatedAt) {
      const at = q.errorUpdatedAt;
      setBuf((b) => ({ id: installationId, points: appendError(b.id === installationId ? b.points : {}, at) }));
    }
  }, [installationId, q.error, q.errorUpdatedAt]);
  return {
    status: q.data,
    error: q.error ? toAppError(q.error) : null,
    isPending: q.isPending,
    buffer: buf.id === installationId ? buf.points : {},
    refetch: q.refetch,
  };
}

/**
 * Окно истории карт хоста (одной карты `gpu` или всех): запрашивается при выборе, ответ живёт минуту, как окно
 * истории сервера; свежее — по «Обновить».
 */
export function useGpuHistory(installationId: string, window: HistoryWindow, host: string, gpu: number | null, enabled: boolean) {
  return useQuery({
    queryKey: gpuHistoryKey(installationId, window, host, gpu),
    queryFn: () => api.readGpuHistory(installationId, window, host, gpu),
    enabled,
    staleTime: 60_000,
    gcTime: 60_000,
    placeholderData: keepPreviousData,
    retry: false,
    refetchOnWindowFocus: false,
  });
}

/**
 * Ряды окна «Сейчас» карт `indices` хоста `host`: точки истории (окно `now`) раньше первой живой точки карты, затем
 * буфер опроса.
 */
export function nowSeries(buffer: GpuBuffer, host: string, indices: number[], history?: GpuHistoryAnswer): Record<number, GpuNowPoint[]> {
  const out: Record<number, GpuNowPoint[]> = {};
  for (const index of indices) {
    const live = buffer[cardKey(host, index)] ?? [];
    const first = live.length > 0 ? live[0].at : Number.POSITIVE_INFINITY;
    const past = history?.series.find((s) => s.host === host && s.gpu === index)?.points.filter((p) => p.at < first) ?? [];
    out[index] = [...past, ...live];
  }
  return out;
}
