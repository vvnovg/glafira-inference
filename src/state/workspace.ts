import { create } from "zustand";

export type TabSpec =
  /** Инсталляция: серверы, их тип, версия, модели и состояние. */
  | { kind: "installation"; installationId: string }
  /** folderId — папка новой инсталляции («Новая инсталляция здесь»). */
  | { kind: "installation-form"; installationId: string | null; folderId?: string | null }
  | { kind: "about" }
  /** Справка: глава и раздел руководства (F1, ссылки). */
  | { kind: "help"; chapter?: string; anchor?: string }
  | { kind: "settings" };

export type Tab = TabSpec & { id: string };

export function tabId(spec: TabSpec): string {
  switch (spec.kind) {
    case "installation-form":
      return `installation-form:${spec.installationId ?? "new"}`;
    case "installation":
      return `installation:${spec.installationId}`;
    default:
      return spec.kind;
  }
}

/** Инсталляция вкладки или `null`, если вкладка не про инсталляцию. */
export function tabInstallation(tab: TabSpec): string | null {
  return "installationId" in tab ? tab.installationId : null;
}

interface WorkspaceState {
  tabs: Tab[];
  activeId: string | null;
  open: (spec: TabSpec) => void;
  close: (id: string) => void;
  /** Закрывает все вкладки инсталляции (например, после её удаления). */
  closeForInstallation: (installationId: string) => void;
  activate: (id: string) => void;
}

export const useWorkspace = create<WorkspaceState>((set) => ({
  tabs: [],
  activeId: null,
  open: (spec) =>
    set((s) => {
      const id = tabId(spec);
      const tab = { ...spec, id } as Tab;
      // Уже открытая вкладка получает новый spec (например, главу справки).
      if (s.tabs.some((t) => t.id === id)) return { tabs: s.tabs.map((t) => (t.id === id ? tab : t)), activeId: id };
      return { tabs: [...s.tabs, tab], activeId: id };
    }),
  close: (id) =>
    set((s) => {
      const idx = s.tabs.findIndex((t) => t.id === id);
      if (idx < 0) return s;
      const tabs = s.tabs.filter((t) => t.id !== id);
      const activeId = s.activeId !== id ? s.activeId : (tabs[idx] ?? tabs[idx - 1])?.id ?? null;
      return { tabs, activeId };
    }),
  closeForInstallation: (installationId) =>
    set((s) => {
      const firstIdx = s.tabs.findIndex((t) => tabInstallation(t) === installationId);
      if (firstIdx < 0) return s;
      const tabs = s.tabs.filter((t) => tabInstallation(t) !== installationId);
      const activeId = tabs.some((t) => t.id === s.activeId)
        ? s.activeId
        : (tabs[firstIdx] ?? tabs[tabs.length - 1])?.id ?? null;
      return { tabs, activeId };
    }),
  activate: (id) => set({ activeId: id }),
}));
