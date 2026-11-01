import type { GpuBinding, GpuCard, GpuHost, GpuStatus } from "../../api/types";
import { cardAlerts } from "./alerts";

/** Строка хоста в «Обзоре» инсталляции (спека I1d §5). */
export interface HostSummary {
  host: string;
  /** Модель карт; у разных моделей — `null`. */
  model: string | null;
  count: number;
  avgUtil: number | null;
  memUsed: number | null;
  memTotal: number | null;
  maxTemp: number | null;
  /** Карт со значками проблем. */
  troubled: number;
}

const values = (cards: GpuCard[], f: (c: GpuCard) => number | undefined) =>
  cards.map(f).filter((v): v is number => typeof v === "number");

export function hostSummary(h: GpuHost): HostSummary {
  const models = new Set(h.cards.map((c) => c.model ?? ""));
  const util = values(h.cards, (c) => c.values.util);
  const used = values(h.cards, (c) => c.values.memUsed);
  const total = values(h.cards, (c) => c.values.memTotal);
  const temp = values(h.cards, (c) => c.values.temp);
  return {
    host: h.host,
    model: models.size === 1 ? [...models][0] || null : null,
    count: h.cards.length,
    avgUtil: util.length ? util.reduce((a, b) => a + b, 0) / util.length : null,
    memUsed: used.length ? used.reduce((a, b) => a + b, 0) : null,
    memTotal: total.length ? total.reduce((a, b) => a + b, 0) : null,
    maxTemp: temp.length ? Math.max(...temp) : null,
    troubled: h.cards.filter((c) => cardAlerts(c).length > 0).length,
  };
}

/** Карты привязки сервера в снимке: хост без учёта регистра, все карты или перечисленные. */
export function boundCards(status: GpuStatus | undefined, b: GpuBinding): GpuCard[] {
  const h = status?.hosts.find((x) => x.host.toLowerCase() === b.host.toLowerCase());
  if (!h) return [];
  return b.indices ? h.cards.filter((c) => b.indices?.includes(c.index)) : h.cards;
}
