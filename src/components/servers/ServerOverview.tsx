import { useTranslation } from "react-i18next";
import type { Canonical, ServerFacts, ServerMetrics } from "../../api/types";
import { modelLabel } from "../installations/InstallationPanel";
import { noMetricsHintKey, readsMetrics } from "./useServerMetrics";
import { fmtAdaptive, fmtNumber, fmtPercent } from "./format";

interface Tile {
  id: string;
  /** Каноническая метрика, которая питает плитку: подсказка «какая серия нужна». */
  canonical: Canonical;
  value: string | null;
}

/** Плитки ключевых метрик (спец. I1b §6): перцентили — за последнюю минуту, скорости — за последний интервал. */
export function tiles(m: ServerMetrics | undefined, t: (k: string, o?: Record<string, unknown>) => string): Tile[] {
  const g = m?.gauges;
  const r = m?.rates;
  const ms = (v: number | null | undefined) => (v == null ? null : t("servers.units.ms", { value: fmtNumber(v) }));
  const perSec = (v: number | null | undefined) => (v == null ? null : t("servers.units.perSec", { value: fmtAdaptive(v) }));
  const pair = (a: number | null | undefined, b: number | null | undefined) =>
    a == null && b == null ? null : `${a == null ? "—" : fmtNumber(a)} · ${b == null ? "—" : fmtNumber(b)}`;
  return [
    { id: "requests", canonical: "requestsRunning", value: pair(g?.requestsRunning, g?.requestsWaiting) },
    { id: "kv", canonical: "kvUsage", value: g?.kvUsage == null ? null : fmtPercent(g.kvUsage) },
    { id: "promptTokens", canonical: "promptTokensTotal", value: perSec(r?.promptTokens) },
    { id: "generationTokens", canonical: "generationTokensTotal", value: perSec(r?.generationTokens) },
    { id: "ttft", canonical: "ttft", value: ms(m?.latency1m.ttft?.p95) },
    { id: "itl", canonical: "itl", value: ms(m?.latency1m.itl?.p95) },
    { id: "prefixCache", canonical: "prefixCacheHitsTotal", value: m?.prefixCacheHitRatio == null ? null : fmtPercent(m.prefixCacheHitRatio) },
    {
      id: "preemptions",
      canonical: "preemptionsTotal",
      value: m?.preemptions1m == null ? null : t("servers.units.perMin", { value: fmtAdaptive(m.preemptions1m) }),
    },
  ];
}

/** «Обзор» сервера: тип и версия, модели, число движков, плитки метрик. `restarted` — сервер перезапущен меньше минуты назад. */
export function ServerOverview({
  facts,
  latest,
  restarted = false,
  error = false,
}: {
  facts: ServerFacts;
  latest?: ServerMetrics;
  restarted?: boolean;
  /** Опрос метрик не прошёл: плитки «—» и подсказка «нет ответа», а не «…» и не «серии нет». */
  error?: boolean;
}) {
  const { t } = useTranslation();
  const kind = t(`installations.form.kinds.${facts.kind}`);
  const readable = readsMetrics(facts);
  const noSeriesHint = (c: Canonical) => {
    const key = noMetricsHintKey(facts);
    if (key) return t(key);
    if (error) return t("servers.tiles.noAnswer");
    const series = latest?.series[c];
    return series ? t("servers.tiles.series", { series }) : t("servers.tiles.noSeries");
  };
  return (
    <>
      <dl className="facts">
        <dt>{t("installations.panel.kind")}</dt>
        <dd>{facts.version ? `${kind} ${facts.version}` : kind}</dd>
        <dt>{t("installations.panel.models")}</dt>
        <dd>
          <ul className="kv">
            {facts.models.map((m) => (
              <li key={m.id}>
                <code>{modelLabel(m)}</code>
                {m.root && m.root !== m.id && <span className="muted"> {m.root}</span>}
              </li>
            ))}
          </ul>
        </dd>
        {latest && latest.engines > 1 && (
          <>
            <dt>{t("servers.overview.engines")}</dt>
            <dd>{latest.engines}</dd>
          </>
        )}
      </dl>
      {restarted && <p className="warn">{t("servers.overview.reset")}</p>}
      <div className="tiles">
        {tiles(latest, t).map((tile) => (
          <section key={tile.id} className="tile" aria-label={t(`servers.tiles.${tile.id}`)} title={noSeriesHint(tile.canonical)}>
            <span className="tile-label">{t(`servers.tiles.${tile.id}`)}</span>
            <span className={tile.value == null ? "tile-value muted" : "tile-value"}>
              {tile.value ?? (error ? "—" : latest || !readable ? t("servers.metrics.noData") : "…")}
            </span>
          </section>
        ))}
      </div>
    </>
  );
}
