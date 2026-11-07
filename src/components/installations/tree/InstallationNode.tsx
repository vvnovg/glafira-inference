import { useQuery } from "@tanstack/react-query";
import type { HTMLAttributes } from "react";
import { useTranslation } from "react-i18next";
import { api } from "../../../api/client";
import type { InstallationProfile } from "../../../api/types";
import { useWorkspace } from "../../../state/workspace";
import { EnvBadge } from "../../common/EnvBadge";
import { Icon } from "../../common/Icon";
import { gpuStatusKey } from "../../gpu/useGpu";
import { installationQueryKey } from "../InstallationPanel";
import { TreeRow } from "./TreeRow";

/**
 * Узел дерева, которому принадлежит активная вкладка (спец. U1 §6): `installation`, `server:<id>` или `gpu`.
 * Строка, а не объект: селектор zustand должен возвращать одно и то же значение, пока состояние не изменилось.
 */
function useActiveNode(installationId: string): string | null {
  return useWorkspace((s) => {
    const tab = s.tabs.find((x) => x.id === s.activeId);
    if (!tab || !("installationId" in tab) || tab.installationId !== installationId) return null;
    switch (tab.kind) {
      case "installation":
      case "gpu":
        return tab.kind;
      case "server":
        return `server:${tab.serverId}`;
      default:
        return null;
    }
  });
}

/**
 * Узел инсталляции в дереве: точка цвета, имя, окружение, «Обзор», серверы (клик — вкладка сервера) и «GPU» у инсталляции
 * с компонентом GPU. Состояние серверов — из подключения вкладки инсталляции, число карт — из последнего снимка GPU;
 * само дерево не подключается и не опрашивает. rowProps — атрибуты строки (перетаскивание). Пункт активной вкладки
 * подсвечен (`aria-current`).
 */
export function InstallationNode({ installation: c, rowProps }: { installation: InstallationProfile; rowProps?: HTMLAttributes<HTMLDivElement> }) {
  const { t } = useTranslation();
  const open = useWorkspace((s) => s.open);
  const active = useActiveNode(c.id);
  const info = useQuery({ queryKey: installationQueryKey(c.id), queryFn: () => api.connectInstallation(c.id, false), enabled: false });
  const states = new Map((info.data?.servers ?? []).map((s) => [s.serverId, s]));
  const gpu = useQuery({ queryKey: gpuStatusKey(c.id), queryFn: () => api.readGpuStatus(c.id), enabled: false });
  const gpuCount = gpu.data?.hosts.reduce((n, h) => n + h.cards.length, 0);

  return (
    <li>
      <TreeRow
        chevron="leaf"
        icon={<span className="dot" style={{ background: c.color }} />}
        name={c.name}
        title={c.name}
        rowProps={{ ...rowProps, className: rowProps?.className ? `tree-conn ${rowProps.className}` : "tree-conn" }}
      >
        <EnvBadge environment={c.environment} readOnly={c.readOnly} />
        <button
          type="button"
          className="icon"
          title={t("sidebar.editInstallation")}
          aria-label={t("sidebar.editInstallation")}
          onClick={() => open({ kind: "installation-form", installationId: c.id })}
        >
          <Icon name="pencil" size={14} />
        </button>
      </TreeRow>
      <ul>
        <li>
          <TreeRow icon="grid" name={t("sidebar.overview")} current={active === "installation"} onActivate={() => open({ kind: "installation", installationId: c.id })} />
        </li>
        {c.servers.map((s) => {
          const state = states.get(s.id);
          const kind = !state ? "unknown" : state.facts ? "ok" : "error";
          const status = !state ? t("sidebar.serverUnknown") : state.facts ? t("sidebar.serverOk") : t("sidebar.serverError");
          return (
            <li key={s.id}>
              {/* Состояние — точка цветом справа от имени (вместо прежних символов текста), с подписью для экранного диктора. */}
              <TreeRow
                icon="server"
                name={s.name}
                title={s.baseUrl}
                extra={
                  <span className={`state-mark state-${kind}`} role="img" aria-label={status}>
                    <Icon name="dot" size={14} />
                  </span>
                }
                current={active === `server:${s.id}`}
                onActivate={() => open({ kind: "server", installationId: c.id, serverId: s.id })}
              />
            </li>
          );
        })}
        {c.gpu && (
          <li>
            <TreeRow
              icon="chip"
              name={gpuCount == null ? t("sidebar.gpu") : t("sidebar.gpuCount", { count: gpuCount })}
              current={active === "gpu"}
              onActivate={() => open({ kind: "gpu", installationId: c.id })}
            />
          </li>
        )}
      </ul>
    </li>
  );
}
