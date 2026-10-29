import { useQuery } from "@tanstack/react-query";
import type { HTMLAttributes } from "react";
import { useTranslation } from "react-i18next";
import { api } from "../../../api/client";
import type { InstallationProfile } from "../../../api/types";
import { useWorkspace } from "../../../state/workspace";
import { EnvBadge } from "../../common/EnvBadge";
import { installationQueryKey } from "../InstallationPanel";

/**
 * Узел инсталляции в дереве: точка цвета, имя, окружение, «Обзор» и серверы. Состояние серверов — из
 * подключения вкладки инсталляции; само дерево не подключается. rowProps — атрибуты строки (перетаскивание).
 */
export function InstallationNode({ installation: c, rowProps }: { installation: InstallationProfile; rowProps?: HTMLAttributes<HTMLDivElement> }) {
  const { t } = useTranslation();
  const open = useWorkspace((s) => s.open);
  const info = useQuery({ queryKey: installationQueryKey(c.id), queryFn: () => api.connectInstallation(c.id, false), enabled: false });
  const states = new Map((info.data?.servers ?? []).map((s) => [s.serverId, s]));
  const openTab = () => open({ kind: "installation", installationId: c.id });

  return (
    <li>
      <div {...rowProps} className={rowProps?.className ? `tree-conn ${rowProps.className}` : "tree-conn"}>
        <span className="dot" style={{ background: c.color }} />
        <span className="tree-name">{c.name}</span>
        <EnvBadge environment={c.environment} readOnly={c.readOnly} />
        <button type="button" className="icon" title={t("sidebar.editInstallation")} onClick={() => open({ kind: "installation-form", installationId: c.id })}>
          ✎
        </button>
      </div>
      <ul>
        <li>
          <button type="button" className="link" onClick={openTab}>
            {t("sidebar.overview")}
          </button>
        </li>
        {c.servers.map((s) => {
          const state = states.get(s.id);
          const mark = !state ? "○" : state.facts ? "●" : "✕";
          const status = !state ? t("sidebar.serverUnknown") : state.facts ? t("sidebar.serverOk") : t("sidebar.serverError");
          return (
            <li key={s.id}>
              <button type="button" className="link" title={s.baseUrl} onClick={openTab}>
                <span className={state ? (state.facts ? "srv-ok" : "srv-error") : "muted"} role="img" aria-label={status}>
                  {mark}
                </span>{" "}
                {s.name}
              </button>
            </li>
          );
        })}
      </ul>
    </li>
  );
}
