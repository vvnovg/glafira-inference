import { act, fireEvent, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { InstallationProfile } from "../api/types";
import { useHelpContext } from "../state/helpContext";
import { useWorkspace } from "../state/workspace";
import { renderWithClient } from "../test/render";
import { Layout } from "./Layout";

vi.mock("../api/client", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../api/client")>();
  const inst: InstallationProfile = {
    id: "c1",
    name: "prod-1",
    color: "#888",
    environment: "prod",
    readOnly: true,
    servers: [{ id: "s1", name: "gpu-1", baseUrl: "http://gpu-1:8000", kind: "auto", metricsPath: "/metrics", ca: { mode: "system" }, proxy: { mode: "system" } }],
    requestTimeoutMs: 5000,
    connectTimeoutMs: 5000,
    folderId: null,
    order: 0,
  };
  return { ...actual, api: { ...actual.api, listInstallations: vi.fn().mockResolvedValue([inst]), listFolders: vi.fn().mockResolvedValue([]) } };
});

// Вместо экрана — метка, показана ли его вкладка (контекст `Layout` для опросов и watch).
vi.mock("./TabContent", async () => {
  const { useTabActive } = await import("../lib/tabActive");
  return { TabContent: ({ tab }: { tab: { id: string } }) => <div data-testid={tab.id} data-tab-active={String(useTabActive())} /> };
});

describe("Layout tabs", () => {
  afterEach(() => useHelpContext.setState({ byTab: {} }));

  beforeEach(() => {
    useHelpContext.setState({ byTab: {} });
    useWorkspace.setState({
      tabs: [
        { kind: "installation", installationId: "c1", id: "installation:c1" },
        { kind: "installation-form", installationId: "c1", id: "installation-form:c1" },
      ],
      activeId: "installation:c1",
    });
  });

  it("shows env badge and lock in the tab, with accessible tab roles", async () => {
    renderWithClient(<Layout />);
    const list = screen.getByRole("tablist");
    const tabs = within(list).getAllByRole("tab");
    expect(tabs[0].getAttribute("aria-selected")).toBe("true");
    expect(tabs[1].getAttribute("aria-selected")).toBe("false");
    expect((await within(list).findAllByText("PROD")).length).toBe(2);
    expect(within(list).getAllByRole("img", { name: "Только чтение" }).length).toBe(2);
  });

  it("server tab is titled «server · installation»", async () => {
    useWorkspace.getState().open({ kind: "server", installationId: "c1", serverId: "s1" });
    renderWithClient(<Layout />);
    expect(await within(screen.getByRole("tablist")).findByRole("tab", { name: "gpu-1 · prod-1" })).toBeInTheDocument();
  });

  it("activates a tab via its button", () => {
    renderWithClient(<Layout />);
    const tabs = within(screen.getByRole("tablist")).getAllByRole("tab");
    fireEvent.click(tabs[1]);
    expect(tabs[1].getAttribute("aria-selected")).toBe("true");
    expect(tabs[0].getAttribute("aria-selected")).toBe("false");
  });

  it("tells each screen whether its tab is shown: hidden tabs stay mounted but must stop polling", () => {
    renderWithClient(<Layout />);
    const shown = (id: string) => screen.getByTestId(id).getAttribute("data-tab-active");
    expect([shown("installation:c1"), shown("installation-form:c1")]).toEqual(["true", "false"]);
    expect(screen.getByTestId("installation-form:c1").closest("section")).toHaveAttribute("hidden");
    fireEvent.click(within(screen.getByRole("tablist")).getAllByRole("tab")[1]);
    expect([shown("installation:c1"), shown("installation-form:c1")]).toEqual(["false", "true"]);
  });

  it("F1 opens the guide chapter of the active screen", () => {
    useWorkspace.setState({
      tabs: [{ kind: "installation-form", installationId: "c1", id: "installation-form:c1" }],
      activeId: "installation-form:c1",
    });
    renderWithClient(<Layout />);
    const ev = new KeyboardEvent("keydown", { key: "F1", bubbles: true, cancelable: true });
    act(() => {
      window.dispatchEvent(ev);
    });
    expect(ev.defaultPrevented).toBe(true);
    const { tabs, activeId } = useWorkspace.getState();
    expect(activeId).toBe("help");
    expect(tabs.find((t) => t.id === "help")).toMatchObject({ kind: "help", chapter: "installations" });
    expect(within(screen.getByRole("tablist")).getByRole("tab", { name: "Справка" })).toBeInTheDocument();
  });

  it("F1 prefers the chapter the active screen reports", () => {
    useWorkspace.setState({
      tabs: [{ kind: "settings", id: "settings" }],
      activeId: "settings",
    });
    useHelpContext.setState({ byTab: { settings: "installations" } });
    renderWithClient(<Layout />);
    fireEvent.keyDown(window, { key: "F1" });
    expect(useWorkspace.getState().tabs.find((t) => t.id === "help")).toMatchObject({ chapter: "installations" });
  });

  it("opens the guide from the sidebar", () => {
    renderWithClient(<Layout />);
    fireEvent.click(screen.getByRole("button", { name: "Справка" }));
    expect(useWorkspace.getState().activeId).toBe("help");
  });
});
