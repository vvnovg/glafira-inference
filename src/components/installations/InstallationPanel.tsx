import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { api, toAppError } from "../../api/client";
import type { Model, ServerConfig, ServerFacts, ServerState } from "../../api/types";
import { tr } from "../../i18n/text";
import { useWorkspace } from "../../state/workspace";
import { EnvBadge } from "../common/EnvBadge";
import { ErrorBanner } from "../common/ErrorBanner";
import { Hint } from "../common/Hint";

const KIND_LABELS: Record<ServerFacts["kind"], string> = { vllm: "vLLM", sglang: "SGLang", openai: "OpenAI" };

/** Ключ запроса сведений о подключённой инсталляции. */
export const installationQueryKey = (id: string) => ["installation", id] as const;

/** Вкладка инсталляции: подключается при открытии и показывает серверы — тип, версию, модели, метрики, ошибки. */
export function InstallationPanel({ installationId }: { installationId: string }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const open = useWorkspace((s) => s.open);
  const installations = useQuery({ queryKey: ["installations"], queryFn: api.listInstallations });
  const q = useQuery({ queryKey: installationQueryKey(installationId), queryFn: () => api.connectInstallation(installationId, false) });
  const profile = installations.data?.find((c) => c.id === installationId);

  if (!profile) return <p className="empty">{installations.isPending ? t("common.loading") : t("installations.form.deleted")}</p>;

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
            <ServerCard key={s.id} server={s} state={states.get(s.id)} />
          ))}
        </div>
      )}
    </div>
  );
}

function ServerCard({ server, state }: { server: ServerConfig; state?: ServerState }) {
  const { t } = useTranslation();
  const facts = state?.facts;
  return (
    <section className="card" aria-label={server.name}>
      <h4>
        {facts ? "✅" : "❌"} {server.name}
      </h4>
      <dl className="facts">
        <dt>{t("installations.panel.address")}</dt>
        <dd>{server.baseUrl}</dd>
        {facts && <ServerFactsRows facts={facts} />}
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
  const kind = KIND_LABELS[facts.kind];
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
        {facts.kind !== facts.detected && <span className="muted"> {t("installations.panel.detectedAs", { kind: KIND_LABELS[facts.detected] })}</span>}
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

/** Модель с контекстом и базовой моделью адаптера LoRA: `qwen3-32b · 32768`, `sql-lora → llama-3.1-8b`. */
export function modelLabel(m: Model): string {
  const parts = [m.parent ? `${m.id} → ${m.parent}` : m.id];
  if (m.maxModelLen) parts.push(String(m.maxModelLen));
  return parts.join(" · ");
}

