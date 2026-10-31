import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { api, toAppError } from "../../api/client";
import type { Model, ServerConfig, ServerFacts, ServerState } from "../../api/types";
import { tr } from "../../i18n/text";
import { useWorkspace } from "../../state/workspace";
import { EnvBadge } from "../common/EnvBadge";
import { ErrorBanner } from "../common/ErrorBanner";
import { Hint } from "../common/Hint";
import { fmtNumber, fmtPercent } from "../servers/format";
import { noMetricsHintKey, readsMetrics, useServerMetrics } from "../servers/useServerMetrics";

/** Ключ запроса сведений о подключённой инсталляции. */
export const installationQueryKey = (id: string) => ["installation", id] as const;

/** Вкладка инсталляции: подключается при открытии и показывает серверы — тип, версию, модели, метрики, ошибки. */
export function InstallationPanel({ installationId }: { installationId: string }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const open = useWorkspace((s) => s.open);
  const installations = useQuery({ queryKey: ["installations"], queryFn: api.listInstallations });
  const profile = installations.data?.find((c) => c.id === installationId);
  // Подключаемся, только когда профиль найден: для удалённой или ещё не загруженной инсталляции вызов лишний.
  const q = useQuery({
    queryKey: installationQueryKey(installationId),
    queryFn: () => api.connectInstallation(installationId, false),
    enabled: !!profile,
  });

  if (!profile) {
    // Упавший список — не «удалена»: показываем ошибку с повтором, а не проглатываем её.
    if (installations.error) return <ErrorBanner error={toAppError(installations.error)} onRetry={() => void installations.refetch()} />;
    return <p className="empty">{installations.isSuccess ? t("installations.form.deleted") : t("common.loading")}</p>;
  }

  // Подключение заново: свежие сведения о серверах (после перезапуска сервера, смены модели).
  const reconnect = () =>
    void queryClient
      .fetchQuery({ queryKey: installationQueryKey(installationId), queryFn: () => api.connectInstallation(installationId, true), staleTime: 0 })
      .catch(() => undefined);

  const states = new Map((q.data?.servers ?? []).map((s) => [s.serverId, s]));
  const connected = q.data?.servers.filter((s) => s.facts).length ?? 0;

  return (
    <div className="page">
      <div className="toolbar">
        <h2>{profile.name}</h2>
        <EnvBadge environment={profile.environment} readOnly={profile.readOnly} />
        {q.data && (
          <span className="muted" role="status">
            {t("installations.panel.summary", { connected, total: profile.servers.length })}
          </span>
        )}
        <button type="button" disabled={q.isFetching} onClick={reconnect}>
          {t("installations.panel.reconnect")}
        </button>
        <button type="button" onClick={() => open({ kind: "installation-form", installationId })}>
          {t("installations.panel.edit")}
        </button>
      </div>
      {q.isPending && <p className="muted">{t("installations.panel.connecting")}</p>}
      {q.error && <ErrorBanner error={toAppError(q.error)} onRetry={() => void q.refetch()} />}
      {q.data && (
        <div className="cards">
          {profile.servers.map((s) => (
            <ServerCard key={s.id} installationId={installationId} server={s} state={states.get(s.id)} />
          ))}
        </div>
      )}
    </div>
  );
}

function ServerCard({ installationId, server, state }: { installationId: string; server: ServerConfig; state?: ServerState }) {
  const { t } = useTranslation();
  const open = useWorkspace((s) => s.open);
  const facts = state?.facts;
  return (
    <section className="card" aria-label={server.name}>
      <h4>
        {facts ? "✅" : "❌"} {server.name}{" "}
        <button type="button" className="link" onClick={() => open({ kind: "server", installationId, serverId: server.id })}>
          {t("servers.panel.open")}
        </button>
      </h4>
      <dl className="facts">
        <dt>{t("installations.panel.address")}</dt>
        <dd>{server.baseUrl}</dd>
        {facts && <ServerFactsRows facts={facts} />}
        {facts && <LiveRow installationId={installationId} serverId={server.id} facts={facts} />}
      </dl>
      {state?.error && <ErrorBanner error={state.error} />}
      {!state && <p className="muted">{t("installations.panel.notConnected")}</p>}
      {facts && facts.warnings.length > 0 && (
        <ul className="steps">
          {facts.warnings.map((w) => (
            <li key={w.key} className="warn">
              ⚠ {tr(w)}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function ServerFactsRows({ facts }: { facts: ServerFacts }) {
  const { t } = useTranslation();
  // Название типа — из словаря формы («OpenAI-compatible» / «OpenAI-совместимый»), без литералов в коде.
  const kind = t(`installations.form.kinds.${facts.kind}`);
  return (
    <>
      <dt>
        <span className="field-label">
          {t("installations.panel.kind")}
          <Hint k="hints.installations.detectedKind" label={t("installations.panel.kind")} />
        </span>
      </dt>
      <dd>
        {facts.version ? `${kind} ${facts.version}` : kind}
        {facts.kind !== facts.detected && <span className="muted"> {t("installations.panel.detectedAs", { kind: t(`installations.form.kinds.${facts.detected}`) })}</span>}
      </dd>
      <dt>{t("installations.panel.models")}</dt>
      <dd>
        {facts.models.length === 0 ? (
          <span className="warn">{t("installations.panel.noModels")}</span>
        ) : (
          <ul className="kv">
            {facts.models.map((m) => (
              <li key={m.id}>
                <code>{modelLabel(m)}</code>
              </li>
            ))}
          </ul>
        )}
      </dd>
      <dt>{t("installations.panel.metrics")}</dt>
      <dd>{facts.metrics ? t("installations.panel.metricsYes") : t("installations.panel.metricsNo")}</dd>
    </>
  );
}

/** Строка «выполняются · ждут · KV» — тот же опрос, что у вкладки сервера (раз в 5 с, пока вкладка видна). */
function LiveRow({ installationId, serverId, facts }: { installationId: string; serverId: string; facts: ServerFacts }) {
  const { t } = useTranslation();
  const polled = readsMetrics(facts);
  const { latest, error } = useServerMetrics(installationId, serverId, polled);
  const g = latest?.gauges;
  const part = (v: number | null | undefined, f: (v: number) => string) => (v == null ? "—" : f(v));
  // react-query хранит последние успешные данные и после неудачного опроса: при ошибке показываем «нет ответа», а не устаревшие числа.
  const text = error
    ? null
    : latest
      ? `${part(g?.requestsRunning, fmtNumber)} · ${part(g?.requestsWaiting, fmtNumber)} · ${part(g?.kvUsage, fmtPercent)}`
      : "…";
  const noMetricsKey = polled ? undefined : noMetricsHintKey(facts);
  return (
    <>
      <dt>{t("installations.panel.live")}</dt>
      <dd className="live" title={noMetricsKey ? t(noMetricsKey) : t("installations.panel.liveHint")}>
        {noMetricsKey ? "—" : (text ?? <span className="muted">{t("installations.panel.liveError")}</span>)}
      </dd>
    </>
  );
}

/** Модель с контекстом и базовой моделью адаптера LoRA: `qwen3-32b · 32768`, `sql-lora → llama-3.1-8b`. */
export function modelLabel(m: Model): string {
  const parts = [m.parent ? `${m.id} → ${m.parent}` : m.id];
  if (m.maxModelLen) parts.push(String(m.maxModelLen));
  return parts.join(" · ");
}

