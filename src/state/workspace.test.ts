import { beforeEach, describe, expect, it } from "vitest";
import { tabId, tabInstallation, useWorkspace } from "./workspace";

beforeEach(() => {
  useWorkspace.setState({ tabs: [], activeId: null });
});

describe("workspace", () => {
  it("повторное открытие активирует существующую вкладку", () => {
    const { open } = useWorkspace.getState();
    open({ kind: "installation", installationId: "a" });
    open({ kind: "settings" });
    open({ kind: "installation", installationId: "a" });
    const s = useWorkspace.getState();
    expect(s.tabs).toHaveLength(2);
    expect(s.activeId).toBe("installation:a");
  });

  it("закрытие активной вкладки активирует соседнюю", () => {
    const { open, close, activate } = useWorkspace.getState();
    open({ kind: "installation", installationId: "a" });
    open({ kind: "installation", installationId: "b" });
    open({ kind: "installation", installationId: "c" });
    activate("installation:b");
    close("installation:b");
    expect(useWorkspace.getState().activeId).toBe("installation:c");
    close("installation:c");
    expect(useWorkspace.getState().activeId).toBe("installation:a");
    close("installation:a");
    expect(useWorkspace.getState().activeId).toBeNull();
  });

  it("id вкладок и инсталляция вкладки", () => {
    expect(tabId({ kind: "installation-form", installationId: null })).toBe("installation-form:new");
    expect(tabId({ kind: "installation-form", installationId: "a" })).toBe("installation-form:a");
    expect(tabId({ kind: "installation", installationId: "a" })).toBe("installation:a");
    expect(tabId({ kind: "help", chapter: "installations" })).toBe("help");
    expect(tabId({ kind: "about" })).toBe("about");
    expect(tabInstallation({ kind: "installation-form", installationId: "a" })).toBe("a");
    expect(tabInstallation({ kind: "settings" })).toBeNull();
  });

  it("справка: повторное открытие меняет главу, а не добавляет вкладку", () => {
    const { open } = useWorkspace.getState();
    open({ kind: "help", chapter: "getting-started" });
    open({ kind: "help", chapter: "installations", anchor: "servers" });
    const s = useWorkspace.getState();
    expect(s.tabs).toHaveLength(1);
    expect(s.tabs[0]).toMatchObject({ kind: "help", chapter: "installations", anchor: "servers" });
  });

  it("closeForInstallation закрывает все вкладки инсталляции", () => {
    const { open, closeForInstallation } = useWorkspace.getState();
    open({ kind: "installation", installationId: "a" });
    open({ kind: "installation-form", installationId: "a" });
    open({ kind: "installation", installationId: "b" });
    open({ kind: "settings" });
    useWorkspace.getState().activate("installation-form:a");
    closeForInstallation("a");
    const s = useWorkspace.getState();
    expect(s.tabs.map((t) => t.id)).toEqual(["installation:b", "settings"]);
    expect(s.activeId).toBe("installation:b");
  });
});
