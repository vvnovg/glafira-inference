import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { api, toAppError } from "../../api/client";
import type { ConfigEntry } from "../../api/types";
import { ErrorBanner } from "../common/ErrorBanner";

/** «Конфигурация»: аргументы запуска с источником; расхождение источников — ⚠ и все значения (спец. I1b §5). */
export function ServerConfigView({ installationId, serverId }: { installationId: string; serverId: string }) {
  const { t } = useTranslation();
  const q = useQuery({
    queryKey: ["server-config", installationId, serverId],
    queryFn: () => api.readServerConfig(installationId, serverId),
    staleTime: 0,
  });
  return (
    <div className="server-config">
      <div className="toolbar">
        <p className="muted">{t("servers.config.partial")}</p>
        <button type="button" disabled={q.isFetching} onClick={() => void q.refetch()}>
          {t("servers.config.refresh")}
        </button>
      </div>
      {q.isPending && <p className="muted">{t("common.loading")}</p>}
      {q.error && <ErrorBanner error={toAppError(q.error)} onRetry={() => void q.refetch()} />}
      {q.data && q.data.length === 0 && <p className="empty">{t("servers.config.empty")}</p>}
      {q.data && q.data.length > 0 && (
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>{t("servers.config.name")}</th>
                <th>{t("servers.config.value")}</th>
                <th>{t("servers.config.source")}</th>
              </tr>
            </thead>
            <tbody>
              {q.data.map((e) => (
                <ConfigRow key={e.name} entry={e} />
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

function ConfigRow({ entry: e }: { entry: ConfigEntry }) {
  const { t } = useTranslation();
  const differs = (e.alternatives?.length ?? 0) > 0;
  return (
    <tr>
      <td>
        <code>{e.name}</code>
      </td>
      <td>
        {e.secret ? <span className="muted">{t("servers.config.secretSet")}</span> : <code>{e.value}</code>}
        {differs && (
          <ul className="kv warn">
            {e.alternatives?.map((a) => (
              <li key={`${a.source}:${a.value}`}>
                ⚠ {t("servers.config.differs", { value: a.value, source: t(`servers.sources.${a.source}`) })}
              </li>
            ))}
          </ul>
        )}
      </td>
      <td>
        <span className="badge" title={t(`servers.sourceHints.${e.source}`)}>
          {t(`servers.sources.${e.source}`)}
        </span>
      </td>
    </tr>
  );
}
