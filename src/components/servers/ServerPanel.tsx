import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { api, toAppError } from "../../api/client";
import { tr } from "../../i18n/text";
import { useWorkspace } from "../../state/workspace";
import { EnvBadge } from "../common/EnvBadge";
import { ErrorBanner } from "../common/ErrorBanner";
import { ServerGpuBlock } from "../gpu/GpuSummary";
import { installationQueryKey } from "../installations/InstallationPanel";
import { ServerConfigView } from "./ServerConfigView";
import { ServerMetricsView } from "./ServerMetricsView";
import { ServerOverview } from "./ServerOverview";
import { readsMetrics, useServerMetrics } from "./useServerMetrics";
import { SubTabs } from "../common/SubTabs";
import type { IconName } from "../common/Icon";

/** Сколько предупреждение о перезапуске остаётся на экране после сброса счётчиков. */
const RESTART_NOTICE_MS = 60_000;

type Sub = "overview" | "config" | "metrics";
const SUBS: Sub[] = ["overview", "config", "metrics"];
/** Значки подвкладок (спец. переноса U1 §1). */
const SUB_ICONS: Record<Sub, IconName> = { overview: "grid", config: "config", metrics: "chart" };

/**
 * Вкладка сервера (спец. I1b §6): «Обзор», «Конфигурация», «Метрики». Метрики опрашиваются здесь, а не в подвкладке:
 * буфер точек за час переживает переключение подвкладок.
 */
export function ServerPanel({ installationId, serverId }: { installationId: string; serverId: string }) {
  const { t } = useTranslation();
  const open = useWorkspace((s) => s.open);
  const [sub, setSub] = useState<Sub>("overview");
  const installations = useQuery({ queryKey: ["installations"], queryFn: api.listInstallations });
  const profile = installations.data?.find((c) => c.id === installationId);
  const server = profile?.servers.find((s) => s.id === serverId);
  const info = useQuery({
    queryKey: installationQueryKey(installationId),
    queryFn: () => api.connectInstallation(installationId, false),
    enabled: !!server,
  });
  const state = info.data?.servers.find((s) => s.serverId === serverId);
  // История — если у инсталляции есть источник, а серии сервера сопоставлены (спека I1c §5).
  const historyUnavailable = !profile?.history
    ? "servers.metrics.noSource"
    : !server?.historySelector
      ? "servers.metrics.notMatched"
      : null;
  const reads = readsMetrics(state?.facts);
  const live = useServerMetrics(installationId, serverId, reads, reads && !historyUnavailable);
  // Предупреждение о перезапуске живёт минуту с момента сброса, а не один опрос (ответ после сброса уже `reset: false`).
  const [resetAt, setResetAt] = useState<number | null>(null);
  const [, redraw] = useState(0);
  const latestReset = live.latest?.reset ? live.latest.at : null;
  useEffect(() => {
    if (latestReset != null) setResetAt(latestReset);
  }, [latestReset]);
  useEffect(() => {
    if (resetAt == null) return;
    const left = resetAt + RESTART_NOTICE_MS - Date.now();
    if (left <= 0) return;
    const id = setTimeout(() => redraw((n) => n + 1), left);
    return () => clearTimeout(id);
  }, [resetAt]);
  const restarted = resetAt != null && Date.now() - resetAt < RESTART_NOTICE_MS;

  if (!profile || !server) {
    if (installations.error) return <ErrorBanner error={toAppError(installations.error)} onRetry={() => void installations.refetch()} />;
    return <p className="empty">{installations.isSuccess ? t("servers.panel.deleted") : t("common.loading")}</p>;
  }

  return (
    <div className="page">
      <div className="toolbar">
        <h2>{server.name}</h2>
        <EnvBadge environment={profile.environment} readOnly={profile.readOnly} />
        <button type="button" className="link" onClick={() => open({ kind: "installation", installationId })}>
          {profile.name}
        </button>
        {live.error && state?.facts && (
          <span className="srv-error" role="status">
            {t("servers.panel.noAnswer", { error: tr(live.error) })}
          </span>
        )}
      </div>
      {info.isPending && <p className="muted">{t("installations.panel.connecting")}</p>}
      {info.error && <ErrorBanner error={toAppError(info.error)} onRetry={() => void info.refetch()} />}
      {state?.error && <ErrorBanner error={state.error} />}
      {state?.facts && (
        <>
          <SubTabs tabs={SUBS.map((s) => ({ id: s, label: t(`servers.tabs.${s}`), icon: SUB_ICONS[s] }))} active={sub} onChange={setSub} />
          {sub === "overview" && (
            <>
              <ServerOverview facts={state.facts} latest={live.error ? undefined : live.latest} restarted={restarted} error={!!live.error} fromHistory={live.fromHistory} />
              {profile.gpu && <ServerGpuBlock profile={profile} server={server} />}
            </>
          )}
          {sub === "config" && <ServerConfigView installationId={installationId} serverId={serverId} />}
          {sub === "metrics" && (
            <ServerMetricsView
              installationId={installationId}
              serverId={serverId}
              points={live.points}
              historyUntil={live.historyUntil}
              historyError={live.historyError}
              historyUnavailable={historyUnavailable}
            />
          )}
        </>
      )}
    </div>
  );
}
