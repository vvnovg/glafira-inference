import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { api } from "../api/client";
import type { InstallationProfile } from "../api/types";
import { ENV_COLORS } from "../lib/env";
import { type Tab, tabInstallation, useWorkspace } from "../state/workspace";
import { EnvBadge } from "./common/EnvBadge";
import { Sidebar } from "./Sidebar";
import { i18n } from "../i18n";
import { TabContent } from "./TabContent";
import { UpdateNotice } from "./about/UpdateNotice";
import { TabActiveContext } from "../lib/tabActive";
import { useHelpShortcut } from "../lib/useHelpShortcut";

function tabTitle(tab: Tab, byId: Map<string, InstallationProfile>): string {
  const instId = tabInstallation(tab);
  const name = instId ? byId.get(instId)?.name ?? "?" : "";
  switch (tab.kind) {
    case "settings":
      return i18n.t("tabs.settings");
    case "about":
      return i18n.t("tabs.about");
    case "help":
      return i18n.t("tabs.help");
    case "installation-form":
      return tab.installationId ? i18n.t("tabs.installationForm", { name }) : i18n.t("tabs.newInstallation");
    case "installation":
      return name;
    case "server": {
      const server = byId.get(tab.installationId)?.servers.find((s) => s.id === tab.serverId)?.name ?? "?";
      return `${server} · ${name}`;
    }
  }
}

export function Layout() {
  const { t } = useTranslation(); // заодно перерисовывает заголовки вкладок при смене языка
  const { tabs, activeId, activate, close } = useWorkspace();
  useHelpShortcut();
  const { data: installations = [] } = useQuery({ queryKey: ["installations"], queryFn: api.listInstallations });
  const byId = new Map(installations.map((c) => [c.id, c]));
  const active = tabs.find((x) => x.id === activeId);
  const activeInstId = active ? tabInstallation(active) : null;
  const activeInst = activeInstId ? byId.get(activeInstId) : undefined;

  return (
    <div className="layout">
      <Sidebar />
      <main className="workspace">
        <div className="env-stripe" style={{ background: activeInst ? ENV_COLORS[activeInst.environment] : "transparent" }} />
        <UpdateNotice />
        <nav className="tabbar" role="tablist">
          {tabs.map((tab) => {
            const tInstId = tabInstallation(tab);
            const inst = tInstId ? byId.get(tInstId) : undefined;
            const isActive = tab.id === activeId;
            return (
              <div key={tab.id} className={isActive ? "tab active" : "tab"}>
                {inst && <EnvBadge environment={inst.environment} readOnly={inst.readOnly} />}
                <button type="button" role="tab" aria-selected={isActive} onClick={() => activate(tab.id)}>
                  {tabTitle(tab, byId)}
                </button>
                <button
                  type="button"
                  className="icon"
                  aria-label={t("layout.closeTab")}
                  onClick={(e) => {
                    e.stopPropagation();
                    close(tab.id);
                  }}
                >
                  ×
                </button>
              </div>
            );
          })}
        </nav>
        {tabs.length === 0 && <p className="empty">{t("layout.empty")}</p>}
        {/* Неактивные вкладки скрыты, но не размонтированы — подключения и состояние форм сохраняются. */}
        {/* Экраны узнают о скрытии из контекста: опрос серверов (с этапа I1b) останавливается. */}
        {tabs.map((tab) => (
          <section key={tab.id} className="tab-content" hidden={tab.id !== activeId}>
            <TabActiveContext.Provider value={tab.id === activeId}>
              <TabContent tab={tab} />
            </TabActiveContext.Provider>
          </section>
        ))}
      </main>
    </div>
  );
}
