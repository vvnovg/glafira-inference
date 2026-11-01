import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { api, toAppError } from "../../api/client";
import type { HistoryWindow, InstallationProfile } from "../../api/types";
import { tr } from "../../i18n/text";
import { useWorkspace } from "../../state/workspace";
import { EnvBadge } from "../common/EnvBadge";
import { ErrorBanner } from "../common/ErrorBanner";
import { GpuCardView } from "./GpuCardView";
import { GpuCharts } from "./GpuCharts";
import { useGpuStatus } from "./useGpu";

const WINDOWS: HistoryWindow[] = ["now", "1h", "24h", "7d", "30d"];

/** Почему окна истории GPU недоступны (`null` — доступны). */
export function gpuHistoryUnavailable(p: InstallationProfile): string | null {
  if (!p.history) return "gpu.noSource";
  if (p.gpu?.mode === "direct" && !p.gpu.exporters.some((e) => e.historySelector)) return "gpu.notMatched";
  return null;
}

/** Серверы, привязанные к хосту карт (без учёта регистра). */
export function boundServers(p: InstallationProfile, host: string): string[] {
  return p.servers.filter((s) => s.gpus && s.gpus.host.toLowerCase() === host.toLowerCase()).map((s) => s.name);
}

/**
 * Вкладка GPU (спека I1d §5): окна «Сейчас / 1 ч / 24 ч / 7 д / 30 д», сбои экспортёров, подсказка о неполном наборе
 * счётчиков, группы по хостам с карточками карт. Щелчок по карточке или заголовку хоста раскрывает графики под группой.
 */
export function GpuPanel({ installationId }: { installationId: string }) {
  const { t } = useTranslation();
  const open = useWorkspace((s) => s.open);
  const queryClient = useQueryClient();
  const installations = useQuery({ queryKey: ["installations"], queryFn: api.listInstallations });
  const profile = installations.data?.find((c) => c.id === installationId);
  const live = useGpuStatus(installationId, !!profile?.gpu);
  const [win, setWin] = useState<HistoryWindow>("now");
  const [selected, setSelected] = useState<{ host: string; gpu: number | null } | null>(null);

  if (!profile) {
    if (installations.error) return <ErrorBanner error={toAppError(installations.error)} onRetry={() => void installations.refetch()} />;
    return <p className="empty">{installations.isSuccess ? t("installations.form.deleted") : t("common.loading")}</p>;
  }
  if (!profile.gpu) return <p className="empty">{t("gpu.panel.noComponent")}</p>;
  const unavailable = gpuHistoryUnavailable(profile);
  const status = live.status;
  const pick = (host: string, gpu: number | null) =>
    setSelected((s) => (s && s.host === host && s.gpu === gpu ? null : { host, gpu }));

  return (
    <div className="page">
      <div className="toolbar">
        <h2>{t("gpu.panel.title")}</h2>
        <EnvBadge environment={profile.environment} readOnly={profile.readOnly} />
        <button type="button" className="link" onClick={() => open({ kind: "installation", installationId })}>
          {profile.name}
        </button>
      </div>
      <div className="toolbar server-metrics" role="group" aria-label={t("servers.metrics.window")}>
        {WINDOWS.map((w) => (
          <button
            key={w}
            type="button"
            className={w === win ? "active" : undefined}
            aria-pressed={w === win}
            disabled={w !== "now" && !!unavailable}
            title={w !== "now" && unavailable ? t(unavailable) : undefined}
            onClick={() => setWin(w)}
          >
            {t(`servers.metrics.windows.${w}`)}
          </button>
        ))}
        {win !== "now" && (
          <button type="button" onClick={() => void queryClient.invalidateQueries({ queryKey: ["gpu-history", installationId] })}>
            {t("common.refresh")}
          </button>
        )}
      </div>
      {live.isPending && !live.error && <p className="muted">{t("common.loading")}</p>}
      {live.error && <ErrorBanner error={live.error} onRetry={() => void live.refetch()} />}
      {status?.errors.map((e, i) => (
        <p key={`${e.exporter ?? ""}-${i}`} className="warn" role="status">
          {e.exporter ? t("gpu.panel.exporterError", { exporter: e.exporter, error: tr(e.error) }) : tr(e.error)}
        </p>
      ))}
      {status && status.missing.length > 0 && (
        <p className="muted">{t("gpu.panel.missing", { groups: status.missing.map((g) => t(`gpu.groups.${g}`)).join(", ") })}</p>
      )}
      {status && status.hosts.length === 0 && <p className="empty">{t("gpu.panel.noCards")}</p>}
      {status?.hosts.map((h) => {
        const servers = boundServers(profile, h.host);
        const indices = h.cards.map((c) => c.index);
        return (
          <section key={h.host} className="gpu-host" aria-label={h.host}>
            <button type="button" className="link gpu-host-title" aria-pressed={selected?.host === h.host && selected.gpu == null} onClick={() => pick(h.host, null)}>
              <strong>{h.host}</strong>
            </button>
            <span className="muted">
              {" "}
              {[
                t("gpu.panel.cards", { count: h.cards.length }),
                h.driver ? t("gpu.panel.driver", { driver: h.driver }) : null,
                servers.length > 0 ? t("gpu.panel.servers", { servers: servers.join(", ") }) : null,
              ]
                .filter(Boolean)
                .join(" · ")}
            </span>
            <div className="gpu-cards">
              {h.cards.map((c) => (
                <GpuCardView
                  key={`${c.index}-${c.exporter ?? ""}`}
                  card={c}
                  selected={selected?.host === h.host && selected.gpu === c.index}
                  onSelect={() => pick(h.host, c.index)}
                />
              ))}
            </div>
            {selected?.host === h.host && (
              <GpuCharts
                installationId={installationId}
                host={h.host}
                gpu={selected.gpu}
                indices={indices}
                buffer={live.buffer}
                window={win}
                historyUnavailable={unavailable}
              />
            )}
          </section>
        );
      })}
      {status && status.hosts.length > 0 && !selected && <p className="muted">{t("gpu.panel.selectHint")}</p>}
    </div>
  );
}
