import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { renderWithClient } from "../../test/render";

vi.mock("../../api/client", async (importOriginal) => {
  const orig = await importOriginal<typeof import("../../api/client")>();
  return { ...orig, api: { listInstallations: vi.fn(), connectInstallation: vi.fn() } };
});

import { api } from "../../api/client";
import type { InstallationInfo, InstallationProfile } from "../../api/types";
import { useWorkspace } from "../../state/workspace";
import { InstallationPanel, modelLabel } from "./InstallationPanel";
import { newInstallation } from "./profile";

const msg = (key: string, text: string, params: Record<string, string> = {}) => ({ key, params, text });

const PROFILE: InstallationProfile = {
  ...newInstallation(),
  id: "i1",
  name: "prod-llm",
  environment: "prod",
  readOnly: true,
  servers: [
    { id: "s1", name: "gpu-1", baseUrl: "http://gpu-1:8000", kind: "auto", metricsPath: "/metrics", ca: { mode: "system" }, proxy: { mode: "system" } },
    { id: "s2", name: "gpu-2", baseUrl: "http://gpu-2:8000", kind: "auto", metricsPath: "/metrics", ca: { mode: "system" }, proxy: { mode: "system" } },
  ],
};

const INFO: InstallationInfo = {
  installationId: "i1",
  servers: [
    {
      serverId: "s1",
      facts: {
        kind: "vllm",
        detected: "vllm",
        version: "0.11.0",
        models: [
          { id: "llama-3.1-8b", maxModelLen: 131072 },
          { id: "sql-lora", parent: "llama-3.1-8b" },
        ],
        metrics: true,
        warnings: [],
      },
    },
    {
      serverId: "s2",
      error: { code: "AUTH_FAILED", key: "errors.server.unauthorized", params: { status: "401" }, message: "rejected", retryable: false },
    },
  ],
};

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.listInstallations).mockResolvedValue([PROFILE]);
  vi.mocked(api.connectInstallation).mockResolvedValue(INFO);
  useWorkspace.setState({ tabs: [], activeId: null });
});

describe("InstallationPanel", () => {
  it("показывает серверы: тип, версию, модели и метрики или ошибку сервера", async () => {
    renderWithClient(<InstallationPanel installationId="i1" />);
    const first = await screen.findByRole("region", { name: "gpu-1" });
    expect(within(first).getByText("vLLM 0.11.0")).toBeInTheDocument();
    expect(within(first).getByText("llama-3.1-8b · 131072")).toBeInTheDocument();
    expect(within(first).getByText("sql-lora → llama-3.1-8b")).toBeInTheDocument();
    expect(within(first).getByText("отдаёт")).toBeInTheDocument();
    const second = screen.getByRole("region", { name: "gpu-2" });
    expect(within(second).getByText("Сервер отклонил запрос (401) — проверьте ключ API")).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("Подключено серверов: 1 из 2");
    expect(api.connectInstallation).toHaveBeenCalledWith("i1", false);
  });

  it("предупреждения и тип, выбранный вручную", async () => {
    vi.mocked(api.connectInstallation).mockResolvedValue({
      installationId: "i1",
      servers: [
        {
          serverId: "s1",
          facts: {
            kind: "openai",
            detected: "vllm",
            version: null,
            models: [],
            metrics: false,
            warnings: [msg("messages.servers.unsupportedVersion", "old", { kind: "vLLM", version: "0.7.3", min: "0.8" })],
          },
        },
      ],
    });
    renderWithClient(<InstallationPanel installationId="i1" />);
    const first = await screen.findByRole("region", { name: "gpu-1" });
    expect(within(first).getByText("OpenAI")).toBeInTheDocument();
    expect(within(first).getByText("по признакам — vLLM")).toBeInTheDocument();
    expect(within(first).getByText("моделей нет")).toBeInTheDocument();
    expect(within(first).getByText(/vLLM 0\.7\.3 старше 0\.8/)).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "gpu-2" })).getByText("Не подключался")).toBeInTheDocument();
  });

  it("«Подключиться заново» — свежее подключение; «Изменить» открывает форму", async () => {
    const user = userEvent.setup();
    renderWithClient(<InstallationPanel installationId="i1" />);
    await screen.findByRole("region", { name: "gpu-1" });
    await user.click(screen.getByRole("button", { name: "Подключиться заново" }));
    await waitFor(() => expect(api.connectInstallation).toHaveBeenLastCalledWith("i1", true));
    await user.click(screen.getByRole("button", { name: "Изменить" }));
    expect(useWorkspace.getState().activeId).toBe("installation-form:i1");
  });

  it("ошибка подключения целиком — баннер с повтором", async () => {
    vi.mocked(api.connectInstallation).mockRejectedValueOnce({ code: "NOT_FOUND", key: "errors.installations.notFound", params: { id: "i1" }, message: "nf", retryable: false });
    const user = userEvent.setup();
    renderWithClient(<InstallationPanel installationId="i1" />);
    expect(await screen.findByText("Инсталляция i1 не найдена")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Повторить" }));
    expect(await screen.findByRole("region", { name: "gpu-1" })).toBeInTheDocument();
  });

  it("удалённая инсталляция", async () => {
    vi.mocked(api.listInstallations).mockResolvedValue([]);
    renderWithClient(<InstallationPanel installationId="i1" />);
    expect(await screen.findByText("Инсталляция удалена")).toBeInTheDocument();
  });

  it("подпись модели", () => {
    expect(modelLabel({ id: "m" })).toBe("m");
    expect(modelLabel({ id: "m", maxModelLen: 4096 })).toBe("m · 4096");
    expect(modelLabel({ id: "a", parent: "b", maxModelLen: 8 })).toBe("a → b · 8");
  });
});
