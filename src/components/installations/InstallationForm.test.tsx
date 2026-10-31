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
      checkHistorySource: vi.fn(),
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

const AUTO = '{job="vllm", instance="gpu-1:8000"}';

const CHECK = {
  report: {
    ok: true,
    steps: [
      { name: "auth" as const, ok: true, message: msg("messages.historyTest.source", "Prometheus 3.15.1", { kind: "Prometheus", version: "3.15.1" }) },
      { name: "targets" as const, ok: true, message: msg("messages.historyTest.targets", "Inference targets found: 2", { count: "2" }) },
    ],
    warnings: [msg("messages.historyTest.noTarget", "no target", { server: "gpu-2" })],
  },
  source: { kind: "prometheus" as const, version: "3.15.1" },
  targets: [
    { job: "sglang", instance: "gpu-3:30000", kind: "sglang" as const },
    { job: "vllm", instance: "gpu-1:8000", kind: "vllm" as const },
  ],
  suggestions: [
    { serverId: "s1", selector: AUTO, targets: [{ job: "vllm", instance: "gpu-1:8000", kind: "vllm" as const }] },
    { serverId: "s2", selector: null, targets: [] },
  ],
};

describe("InstallationForm: история метрик", () => {
  it("источник, проверка с автоподбором, свой селектор — всё уходит в сохранение, токен — секретом", async () => {
    vi.mocked(api.checkHistorySource).mockResolvedValue(CHECK);
    vi.mocked(api.saveInstallation).mockImplementation(async (p) => p);
    const user = userEvent.setup();
    renderWithClient(<InstallationForm initial={saved()} tabId="form:i1" />);
    const section = screen.getByRole("region", { name: "История метрик (Prometheus / VictoriaMetrics)" });
    await user.click(within(section).getByRole("button", { name: "Добавить источник истории" }));
    await user.type(within(section).getByLabelText("Адрес"), "http://prometheus:9090");
    await user.selectOptions(within(section).getByLabelText("Вход"), "bearer");
    await user.type(within(section).getByLabelText("Токен"), "prom-t0ken");
    await user.click(within(section).getByRole("button", { name: "Проверить источник" }));
    expect(api.checkHistorySource).toHaveBeenCalledWith(
      expect.objectContaining({ history: { url: "http://prometheus:9090", auth: { mode: "bearer" }, ca: { mode: "system" }, proxy: { mode: "system" } } }),
      { apiKeys: {}, historySecret: "prom-t0ken" },
    );
    const report = await within(section).findByRole("list", { name: "Проверка источника истории" });
    expect(within(report).getByText("Цели:")).toBeInTheDocument();
    expect(within(report).getByText(/«gpu-2»: цели с адресом сервера нет — выберите цель вручную/)).toBeInTheDocument();
    const gpu1 = within(section).getByLabelText("gpu-1") as HTMLSelectElement;
    expect(gpu1.value).toBe(AUTO);
    expect(within(section).getByText("автоподбор")).toBeInTheDocument();
    expect(within(gpu1).getByRole("option", { name: "sglang · gpu-3:30000 (SGLang)" })).toBeInTheDocument();
    await user.selectOptions(within(section).getByLabelText("gpu-2"), "Свой селектор…");
    await user.type(within(section).getByLabelText("Селектор серий «gpu-2»"), '{{job="x"}');
    await user.click(screen.getByRole("button", { name: "Сохранить" }));
    await waitFor(() => expect(api.saveInstallation).toHaveBeenCalled());
    const [profile, secrets] = vi.mocked(api.saveInstallation).mock.calls[0];
    expect(profile.servers.map((s) => s.historySelector)).toEqual([AUTO, '{job="x"}']);
    expect(profile.history?.auth).toEqual({ mode: "bearer" });
    expect(secrets).toEqual({ apiKeys: {}, historySecret: "prom-t0ken" });
    expect(JSON.stringify(profile)).not.toContain("prom-t0ken");
  });

  it("выбор человека автоподбор не перезаписывает; «Не сопоставлять» убирает селектор", async () => {
    vi.mocked(api.checkHistorySource).mockResolvedValue(CHECK);
    vi.mocked(api.saveInstallation).mockImplementation(async (p) => p);
    const initial = saved();
    initial.history = { url: "http://prometheus:9090", auth: { mode: "basic", username: "reader" }, ca: { mode: "system" }, proxy: { mode: "system" } };
    initial.servers[0].historySelector = '{job="mine"}';
    const user = userEvent.setup();
    renderWithClient(<InstallationForm initial={initial} tabId="form:i1" />);
    const section = screen.getByRole("region", { name: "История метрик (Prometheus / VictoriaMetrics)" });
    expect(within(section).getByLabelText("Пароль")).toHaveAttribute("placeholder", "оставьте пустым, чтобы не менять сохранённый");
    await user.click(within(section).getByRole("button", { name: "Проверить источник" }));
    await within(section).findByRole("list", { name: "Проверка источника истории" });
    expect(within(section).getByLabelText("Селектор серий «gpu-1»")).toHaveValue('{job="mine"}');
    expect(within(section).queryByText("автоподбор")).toBeNull();
    await user.selectOptions(within(section).getByLabelText("gpu-1"), "Не сопоставлять");
    await user.click(screen.getByRole("button", { name: "Сохранить" }));
    await waitFor(() => expect(api.saveInstallation).toHaveBeenCalledTimes(1));
    const [first, secrets] = vi.mocked(api.saveInstallation).mock.calls[0];
    expect(first.servers[0].historySelector).toBeUndefined();
    expect(secrets?.historySecret).toBeUndefined();
  });

  it("смена адреса сервера сбрасывает автоподобранный селектор, но не выбранный вручную", async () => {
    vi.mocked(api.checkHistorySource).mockResolvedValue(CHECK);
    const initial = saved();
    initial.history = { url: "http://prometheus:9090", auth: { mode: "none" }, ca: { mode: "system" }, proxy: { mode: "system" } };
    const user = userEvent.setup();
    renderWithClient(<InstallationForm initial={initial} tabId="form:i1" />);
    const section = screen.getByRole("region", { name: "История метрик (Prometheus / VictoriaMetrics)" });
    await user.click(within(section).getByRole("button", { name: "Проверить источник" }));
    await within(section).findByRole("list", { name: "Проверка источника истории" });
    expect(within(section).getByLabelText("gpu-1")).toHaveValue(AUTO);
    await user.selectOptions(within(section).getByLabelText("gpu-2"), "sglang · gpu-3:30000 (SGLang)");
    const manual = (within(section).getByLabelText("gpu-2") as HTMLSelectElement).value;
    await user.type(within(screen.getByRole("group", { name: "Сервер 1" })).getByLabelText("Адрес"), "1");
    expect(within(section).getByLabelText("gpu-1")).toHaveValue("");
    expect(within(section).queryByText("автоподбор")).toBeNull();
    await user.type(within(screen.getByRole("group", { name: "Сервер 2" })).getByLabelText("Адрес"), "1");
    expect(within(section).getByLabelText("gpu-2")).toHaveValue(manual);
  });

  it("«Убрать источник» — без него и секрета", async () => {
    vi.mocked(api.saveInstallation).mockImplementation(async (p) => p);
    const initial = saved();
    initial.history = { url: "http://prometheus:9090", auth: { mode: "bearer" }, ca: { mode: "system" }, proxy: { mode: "system" } };
    const user = userEvent.setup();
    renderWithClient(<InstallationForm initial={initial} tabId="form:i1" />);
    await user.click(screen.getByRole("button", { name: "Убрать источник" }));
    expect(screen.getByRole("button", { name: "Добавить источник истории" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Сохранить" }));
    await waitFor(() => expect(api.saveInstallation).toHaveBeenCalled());
    const [profile, secrets] = vi.mocked(api.saveInstallation).mock.calls[0];
    expect(profile.history).toBeUndefined();
    expect(secrets).toEqual({ apiKeys: {} });
  });

  it("сохранённый секрет: подсказка и флажок только пока вид входа тот же", async () => {
    const initial = saved();
    initial.history = { url: "http://prometheus:9090", auth: { mode: "bearer" }, ca: { mode: "system" }, proxy: { mode: "system" } };
    const user = userEvent.setup();
    renderWithClient(<InstallationForm initial={initial} tabId="form:i1" />);
    const section = screen.getByRole("region", { name: "История метрик (Prometheus / VictoriaMetrics)" });
    expect(within(section).getByLabelText("Токен")).toHaveAttribute("placeholder", "оставьте пустым, чтобы не менять сохранённый");
    expect(within(section).getByLabelText("Удалить сохранённый токен или пароль")).toBeInTheDocument();
    await user.selectOptions(within(section).getByLabelText("Вход"), "basic");
    expect(within(section).getByLabelText("Пароль")).not.toHaveAttribute("placeholder", "оставьте пустым, чтобы не менять сохранённый");
    expect(within(section).queryByLabelText("Удалить сохранённый токен или пароль")).toBeNull();
    await user.selectOptions(within(section).getByLabelText("Вход"), "bearer");
    expect(within(section).getByLabelText("Токен")).toHaveAttribute("placeholder", "оставьте пустым, чтобы не менять сохранённый");
    expect(within(section).getByLabelText("Удалить сохранённый токен или пароль")).toBeInTheDocument();
  });

  it("флажок «удалить сохранённый» не блокирует поле после смены вида входа и не уходит секретом", async () => {
    vi.mocked(api.saveInstallation).mockImplementation(async (p) => p);
    const initial = saved();
    initial.history = { url: "http://prometheus:9090", auth: { mode: "bearer" }, ca: { mode: "system" }, proxy: { mode: "system" } };
    const user = userEvent.setup();
    renderWithClient(<InstallationForm initial={initial} tabId="form:i1" />);
    const section = screen.getByRole("region", { name: "История метрик (Prometheus / VictoriaMetrics)" });
    await user.click(within(section).getByLabelText("Удалить сохранённый токен или пароль"));
    expect(within(section).getByLabelText("Токен")).toBeDisabled();
    await user.selectOptions(within(section).getByLabelText("Вход"), "basic");
    const password = within(section).getByLabelText("Пароль");
    expect(password).toBeEnabled();
    await user.type(password, "n3w-pass");
    await user.click(screen.getByRole("button", { name: "Сохранить" }));
    await waitFor(() => expect(api.saveInstallation).toHaveBeenCalled());
    expect(vi.mocked(api.saveInstallation).mock.calls[0][1]).toEqual({ apiKeys: {}, historySecret: "n3w-pass" });
  });

  it("«Убрать источник» сбрасывает введённый секрет и результат проверки", async () => {
    vi.mocked(api.checkHistorySource).mockResolvedValue(CHECK);
    vi.mocked(api.saveInstallation).mockImplementation(async (p) => p);
    const user = userEvent.setup();
    renderWithClient(<InstallationForm initial={saved()} tabId="form:i1" />);
    const section = () => screen.getByRole("region", { name: "История метрик (Prometheus / VictoriaMetrics)" });
    await user.click(within(section()).getByRole("button", { name: "Добавить источник истории" }));
    await user.selectOptions(within(section()).getByLabelText("Вход"), "bearer");
    await user.type(within(section()).getByLabelText("Токен"), "old-t0ken");
    await user.click(within(section()).getByRole("button", { name: "Проверить источник" }));
    await within(section()).findByRole("list", { name: "Проверка источника истории" });
    await user.click(within(section()).getByRole("button", { name: "Убрать источник" }));
    await user.click(within(section()).getByRole("button", { name: "Добавить источник истории" }));
    await user.selectOptions(within(section()).getByLabelText("Вход"), "bearer");
    expect(within(section()).getByLabelText("Токен")).toHaveValue("");
    expect(within(section()).queryByRole("list", { name: "Проверка источника истории" })).toBeNull();
    await user.click(screen.getByRole("button", { name: "Сохранить" }));
    await waitFor(() => expect(api.saveInstallation).toHaveBeenCalled());
    expect(vi.mocked(api.saveInstallation).mock.calls[0][1]).toEqual({ apiKeys: {} });
  });
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
      warnings: [msg("messages.servers.unsupportedVersion", "old", { kind: "vLLM", version: "0.7.3", min: "0.10" })],
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
    expect(within(report).getByText(/vLLM 0\.7\.3 старше 0\.10/)).toBeInTheDocument();
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
