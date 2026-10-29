import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { renderWithClient } from "../../test/render";

vi.mock("../../api/client", async (importOriginal) => {
  const orig = await importOriginal<typeof import("../../api/client")>();
  return {
    ...orig,
    api: {
      testServer: vi.fn(),
      saveInstallation: vi.fn(),
      deleteInstallation: vi.fn(),
      listInstallations: vi.fn().mockResolvedValue([]),
      listFolders: vi.fn().mockResolvedValue([]),
      moveItems: vi.fn(),
      getPolicyStatus: vi.fn(),
    },
  };
});

import { api } from "../../api/client";
import type { InstallationProfile, PolicyStatus } from "../../api/types";
import { useWorkspace } from "../../state/workspace";
import { InstallationForm } from "./InstallationForm";
import { newInstallation } from "./profile";

const POLICY: PolicyStatus = {
  managed: false,
  source: null,
  updatesEnabled: true,
  updateUrl: null,
  language: null,
  mcpEnabled: true,
  allowInsecureTls: true,
  errors: [],
};

const msg = (key: string, text: string, params: Record<string, string> = {}) => ({ key, params, text });

function saved(): InstallationProfile {
  return {
    ...newInstallation(),
    id: "i1",
    name: "prod-llm",
    servers: [
      { id: "s1", name: "gpu-1", baseUrl: "https://gpu-1:8000", kind: "auto", metricsPath: "/metrics", ca: { mode: "system" }, proxy: { mode: "system" } },
      { id: "s2", name: "gpu-2", baseUrl: "http://gpu-2:8000", kind: "vllm", metricsPath: "/metrics", ca: { mode: "system" }, proxy: { mode: "system" } },
    ],
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.listFolders).mockResolvedValue([]);
  vi.mocked(api.getPolicyStatus).mockResolvedValue(POLICY);
  useWorkspace.setState({ tabs: [], activeId: null });
});

describe("InstallationForm", () => {
  it("сохраняет новую инсталляцию с ключом сервера и открывает её вкладку", async () => {
    vi.mocked(api.saveInstallation).mockImplementation(async (p) => ({ ...p, id: "new-id" }));
    const user = userEvent.setup();
    useWorkspace.getState().open({ kind: "installation-form", installationId: null });
    renderWithClient(<InstallationForm initial={newInstallation()} tabId="installation-form:new" />);

    await user.type(screen.getByLabelText("Имя"), "dev-llm");
    await user.type(screen.getByLabelText("Имя сервера"), "vllm-1");
    await user.type(screen.getByLabelText("Адрес"), "https://vllm.dev:8000");
    await user.selectOptions(screen.getByLabelText("Тип сервера"), "vllm");
    await user.type(screen.getByLabelText("Ключ API"), "sk-dev");
    await user.click(screen.getByRole("button", { name: "Сохранить" }));

    await waitFor(() => expect(api.saveInstallation).toHaveBeenCalled());
    const [profile, secrets] = vi.mocked(api.saveInstallation).mock.calls[0];
    expect(profile).toMatchObject({ id: "", name: "dev-llm", folderId: null });
    expect(profile.servers[0]).toMatchObject({ name: "vllm-1", baseUrl: "https://vllm.dev:8000", kind: "vllm" });
    expect(secrets).toEqual({ apiKeys: { [profile.servers[0].id]: "sk-dev" } });
    await waitFor(() => expect(useWorkspace.getState().activeId).toBe("installation:new-id"));
    expect(useWorkspace.getState().tabs.map((t) => t.id)).toEqual(["installation:new-id"]);
  });

  it("новая prod-инсталляция — только для чтения", async () => {
    const user = userEvent.setup();
    renderWithClient(<InstallationForm initial={newInstallation()} tabId="t" />);
    await user.selectOptions(screen.getByLabelText("Окружение"), "prod");
    expect(screen.getByLabelText("Только чтение")).toBeChecked();
  });

  it("проверяет сервер формы и показывает шаги и предупреждения", async () => {
    vi.mocked(api.testServer).mockResolvedValue({
      ok: true,
      steps: [
        { name: "config", ok: true, message: msg("messages.connectionTest.configOk", "Configuration is valid") },
        { name: "tcp", ok: true, message: msg("messages.connectionTest.tcpOk", "TCP", { address: "10.0.0.5:8000" }) },
        { name: "kind", ok: true, message: msg("messages.connectionTest.kindVersion", "vLLM 0.7.3", { kind: "vLLM", version: "0.7.3" }) },
      ],
      warnings: [msg("messages.servers.unsupportedVersion", "old", { kind: "vLLM", version: "0.7.3", min: "0.8" })],
    });
    const user = userEvent.setup();
    renderWithClient(<InstallationForm initial={saved()} tabId="t" />);
    const second = screen.getByRole("group", { name: "Сервер 2" });
    await user.type(within(second).getByLabelText("Ключ API"), "sk-2");
    await user.click(within(second).getByRole("button", { name: "Проверить сервер" }));

    await waitFor(() => expect(api.testServer).toHaveBeenCalled());
    const [profile, serverId, secrets] = vi.mocked(api.testServer).mock.calls[0];
    expect(serverId).toBe("s2");
    expect(profile.servers).toHaveLength(2);
    expect(secrets).toEqual({ apiKeys: { s2: "sk-2" } });
    const report = await within(second).findByRole("list", { name: "Результат проверки" });
    expect(within(report).getByText("TCP-соединение с 10.0.0.5:8000 установлено")).toBeInTheDocument();
    expect(within(report).getByText(/vLLM 0\.7\.3 старше 0\.8/)).toBeInTheDocument();
    expect(within(screen.getByRole("group", { name: "Сервер 1" })).queryByRole("list")).toBeNull();
  });

  it("сохранённый ключ: пустое поле оставляет его, флажок удаляет", async () => {
    vi.mocked(api.saveInstallation).mockImplementation(async (p) => p);
    const user = userEvent.setup();
    renderWithClient(<InstallationForm initial={saved()} tabId="t" />);
    const first = screen.getByRole("group", { name: "Сервер 1" });
    expect(within(first).getByLabelText("Ключ API")).toHaveAttribute("placeholder", "оставьте пустым, чтобы не менять сохранённый");
    await user.click(within(first).getByLabelText("Удалить сохранённый ключ"));
    expect(within(first).getByLabelText("Ключ API")).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Сохранить" }));
    await waitFor(() => expect(api.saveInstallation).toHaveBeenCalled());
    expect(vi.mocked(api.saveInstallation).mock.calls[0][1]).toEqual({ apiKeys: { s1: "" } });
  });

  it("ключ по http — предупреждение", async () => {
    const user = userEvent.setup();
    renderWithClient(<InstallationForm initial={newInstallation()} tabId="t" />);
    await user.type(screen.getByLabelText("Адрес"), "http://10.0.0.5:8000");
    expect(screen.queryByText(/ключ API уйдёт открытым текстом/)).toBeNull();
    await user.type(screen.getByLabelText("Ключ API"), "sk");
    expect(screen.getByText("Адрес — http://: ключ API уйдёт открытым текстом")).toBeInTheDocument();
    // У сохранённого сервера с http ключ может уже быть: предупреждение и без ввода.
    renderWithClient(<InstallationForm initial={saved()} tabId="t2" />);
    expect(within(screen.getAllByRole("group", { name: "Сервер 2" })[0]).getByRole("alert")).toBeInTheDocument();
  });

  it("серверы добавляются и убираются; последний убрать нельзя", async () => {
    vi.mocked(api.saveInstallation).mockImplementation(async (p) => p);
    const user = userEvent.setup();
    renderWithClient(<InstallationForm initial={newInstallation()} tabId="t" />);
    expect(screen.queryByRole("button", { name: "Убрать сервер" })).toBeNull();
    await user.click(screen.getByRole("button", { name: "Добавить сервер" }));
    expect(screen.getAllByRole("group", { name: /Сервер \d/ })).toHaveLength(2);
    await user.click(screen.getAllByRole("button", { name: "Убрать сервер" })[0]);
    expect(screen.getAllByRole("group", { name: /Сервер \d/ })).toHaveLength(1);
  });

  it("не больше 32 серверов: на 32-м кнопка «Добавить сервер» отключается", async () => {
    const user = userEvent.setup();
    renderWithClient(<InstallationForm initial={newInstallation()} tabId="t" />);
    const add = screen.getByRole("button", { name: "Добавить сервер" });
    // Лимит бэкенда (MAX_SERVERS в storage/profile.rs): до 31 серверов кнопка доступна.
    for (let n = 1; n < 32; n++) {
      expect(add).toBeEnabled();
      await user.click(add);
    }
    expect(screen.getAllByRole("group", { name: /Сервер \d/ })).toHaveLength(32);
    expect(add).toBeDisabled();
    // Щелчок по отключённой кнопке 33-й сервер не добавляет.
    await user.click(add);
    expect(screen.getAllByRole("group", { name: /Сервер \d/ })).toHaveLength(32);
  });

  it("две формы на странице: подпись «Только чтение» переключает чекбокс своей формы", async () => {
    const user = userEvent.setup();
    // Вкладки приложения остаются смонтированными: «правка» и «новая» живут в одном документе.
    const edit = renderWithClient(<InstallationForm initial={saved()} tabId="installation-form:i1" />);
    const created = renderWithClient(<InstallationForm initial={newInstallation()} tabId="installation-form:new" />);
    const editBox = within(edit.container).getByLabelText("Только чтение");
    const createdBox = within(created.container).getByRole("checkbox");

    await user.click(within(created.container).getByText("Только чтение"));
    expect(createdBox).toBeChecked();
    expect(editBox).not.toBeChecked();

    // Так же с подписями полей серверов: каждая указывает на поле внутри своей формы.
    for (const form of [edit, created]) {
      for (const label of Array.from(form.container.querySelectorAll("label"))) {
        expect(label.control, label.textContent ?? "").not.toBeNull();
        expect(form.container).toContainElement(label.control);
      }
    }
  });

  it("CA: введённый PEM не теряется при переключении режима; без проверки — запрет политикой", async () => {
    vi.mocked(api.getPolicyStatus).mockResolvedValue({ ...POLICY, allowInsecureTls: false });
    const user = userEvent.setup();
    renderWithClient(<InstallationForm initial={newInstallation()} tabId="t" />);
    await user.selectOptions(screen.getByLabelText("Проверка сертификата"), "pem");
    await user.type(screen.getByLabelText("CA-сертификаты (PEM)"), "PEM-TEXT");
    await user.selectOptions(screen.getByLabelText("Проверка сертификата"), "system");
    await user.selectOptions(screen.getByLabelText("Проверка сертификата"), "pem");
    expect(screen.getByLabelText("CA-сертификаты (PEM)")).toHaveValue("PEM-TEXT");
    await waitFor(() => expect(screen.getByRole("option", { name: "Без проверки (небезопасно)" })).toBeDisabled());
  });

  it("ошибка сохранения показывается, вкладка остаётся", async () => {
    vi.mocked(api.saveInstallation).mockRejectedValue({
      code: "INVALID_INPUT",
      key: "errors.installations.noServers",
      message: "Add at least one inference server",
      retryable: false,
    });
    const user = userEvent.setup();
    useWorkspace.getState().open({ kind: "installation-form", installationId: null });
    renderWithClient(<InstallationForm initial={newInstallation()} tabId="installation-form:new" />);
    await user.click(screen.getByRole("button", { name: "Сохранить" }));
    expect(await screen.findByText("Добавьте хотя бы один сервер инференса")).toBeInTheDocument();
    expect(useWorkspace.getState().activeId).toBe("installation-form:new");
  });

  it("удаление — после подтверждения в форме; вкладки инсталляции закрываются", async () => {
    vi.mocked(api.deleteInstallation).mockResolvedValue(undefined);
    const user = userEvent.setup();
    const ws = useWorkspace.getState();
    ws.open({ kind: "installation", installationId: "i1" });
    ws.open({ kind: "installation-form", installationId: "i1" });
    renderWithClient(<InstallationForm initial={saved()} tabId="installation-form:i1" />);
    await user.click(screen.getByRole("button", { name: "Удалить" }));
    expect(api.deleteInstallation).not.toHaveBeenCalled();
    const confirm = screen.getByRole("group", { name: "Подтверждение удаления" });
    expect(within(confirm).getByText("Удалить «prod-llm» и ключи API её серверов?")).toBeInTheDocument();
    await user.click(within(confirm).getByRole("button", { name: "Удалить" }));
    await waitFor(() => expect(api.deleteInstallation).toHaveBeenCalledWith("i1"));
    await waitFor(() => expect(useWorkspace.getState().tabs).toEqual([]));
  });
});
