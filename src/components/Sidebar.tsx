import { useTranslation } from "react-i18next";
import { isMacOS } from "../lib/platform";
import { openSupport } from "../lib/links";
import { useResizable } from "../lib/useResizable";
import { useWorkspace } from "../state/workspace";
import { InstallationTree } from "./installations/tree/InstallationTree";
import { TreeRow } from "./installations/tree/TreeRow";

const WIDTH_KEY = "glafira.sidebar.width";

/** Боковая панель (спец. U1 §6–7): дерево инсталляций, нижние пункты, разделитель с изменяемой шириной. */
export function Sidebar() {
  const { t } = useTranslation();
  const open = useWorkspace((s) => s.open);
  const activeKind = useWorkspace((s) => s.tabs.find((x) => x.id === s.activeId)?.kind ?? null);
  const { width, dragging, splitterProps } = useResizable(WIDTH_KEY, { min: 200, max: 400, initial: 248 });

  return (
    <aside className="sidebar" style={{ width }}>
      {/* macOS: место светофора; за эту полосу окно перетаскивается. */}
      {isMacOS() && <div className="sidebar-top" data-tauri-drag-region="" />}
      <div className="sidebar-scroll">
        <InstallationTree />
      </div>
      <div className="sidebar-footer">
        <TreeRow chevron="none" icon="settings" name={t("sidebar.settings")} current={activeKind === "settings"} onActivate={() => open({ kind: "settings" })} />
        <TreeRow chevron="none" icon="heart" name={t("support.title")} onActivate={openSupport} />
        <TreeRow chevron="none" icon="book" name={t("sidebar.help")} current={activeKind === "help"} onActivate={() => open({ kind: "help" })} />
        <TreeRow chevron="none" icon="info" name={t("sidebar.about")} current={activeKind === "about"} onActivate={() => open({ kind: "about" })} />
      </div>
      <div className={dragging ? "sidebar-splitter active" : "sidebar-splitter"} aria-label={t("sidebar.installations")} {...splitterProps} />
    </aside>
  );
}
