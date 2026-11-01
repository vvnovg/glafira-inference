import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { renderWithClient } from "../../test/render";

vi.mock("../../api/client", async (importOriginal) => {
  const orig = await importOriginal<typeof import("../../api/client")>();
  return {
    ...orig,
    api: { listInstallations: vi.fn(), connectInstallation: vi.fn(), readServerMetrics: vi.fn(), readServerConfig: vi.fn(), readServerHistory: vi.fn() },
  };
});
vi.mock("uplot", () => ({
  default: vi.fn().mockImplementation(function () {
    return { destroy: vi.fn(), setSize: vi.fn(), setData: vi.fn() };
  }),
}));
globalThis.ResizeObserver ??= class {
  observe() {}
  disconnect() {}
  unobserve() {}
} as unknown as typeof ResizeObserver;

import { api } from "../../api/client";
import type { InstallationInfo, InstallationProfile } from "../../api/types";
import { useWorkspace } from "../../state/workspace";
import { newInstallation } from "../installations/profile";
import { ServerPanel } from "./ServerPanel";
import { metrics } from "./testMetrics";

const PROFILE: InstallationProfile = {
  ...newInstallation(),
  id: "i1",
  name: "prod-llm",
  servers: [
    { id: "s1", name: "gpu-1", baseUrl: "http://gpu-1:8000", kind: "auto", metricsPath: "/metrics", ca: { mode: "system" }, proxy: { mode: "system" } },
    { id: "s2", name: "gpu-2", baseUrl: "http://gpu-2:8000", kind: "auto", metricsPath: "/metrics", ca: { mode: "system" }, proxy: { mode: "system" } },
  ],
};

const HISTORY = { url: "http://prometheus:9090", auth: { mode: "none" }, ca: { mode: "system" }, proxy: { mode: "system" } } as const;

/** Профиль с источником истории и сопоставленным сервером s1. */
const withHistory = (): InstallationProfile => ({
  ...PROFILE,
  history: HISTORY,
  servers: PROFILE.servers.map((s) => (s.id === "s1" ? { ...s, historySelector: '{job="vllm", instance="gpu-1:8000"}' } : s)),
});

const INFO: InstallationInfo = {
  installationId: "i1",
  servers: [
    {
      serverId: "s1",
      facts: { kind: "vllm", detected: "vllm", version: "0.32.0", models: [{ id: "qwen", root: "/models/Qwen", maxModelLen: 32768 }], metrics: true, warnings: [] },
    },
    { serverId: "s2", error: { code: "AUTH_FAILED", key: "errors.server.unauthorized", params: { status: "401" }, message: "rejected", retryable: false } },
  ],
};

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.readServerHistory).mockReset();
  vi.mocked(api.listInstallations).mockResolvedValue([PROFILE]);
  vi.mocked(api.connectInstallation).mockResolvedValue(INFO);
  vi.mocked(api.readServerMetrics).mockResolvedValue(
    metrics(Date.now(), { missing: ["itl"], latency1m: { ttft: { p50: 90, p95: 250, p99: 410, count: 120 }, itl: null, e2e: null, queueTime: null } }),
  );
  vi.mocked(api.readServerConfig).mockResolvedValue([
    { name: "model", value: "/models/Qwen", source: "models" },
    { name: "served_model_name", value: "qwen", source: "server_info", alternatives: [{ value: "qwen-old", source: "models" }] },
    { name: "api_key", value: "", source: "server_info", secret: true },
    { name: "block_size", value: "16", source: "metrics" },
  ]);
  useWorkspace.setState({ tabs: [], activeId: null });
});

describe("ServerPanel", () => {
  it("«Обзор»: тип и версия, модели, плитки метрик; нет серии — «нет данных»", async () => {
    renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
    expect(await screen.findByText("vLLM 0.32.0")).toBeInTheDocument();
    expect(screen.getByText("qwen · 32768")).toBeInTheDocument();
    const kv = await screen.findByRole("region", { name: "KV-кэш" });
    await waitFor(() => expect(within(kv).getByText(/42\s?%/)).toBeInTheDocument());
    expect(within(screen.getByRole("region", { name: "Выполняются · ждут" })).getByText("3 · 1")).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "TTFT p95 за минуту" })).getByText("250 мс")).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "Токены на входе" })).getByText("120/с")).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "ITL p95 за минуту" })).getByText("Нет данных")).toBeInTheDocument();
    expect(screen.queryByText("Движки (data parallel)")).toBeNull();
    expect(api.readServerMetrics).toHaveBeenCalledWith("i1", "s1");
  });

  it("«Конфигурация»: источник, расхождение источников и скрытый секрет", async () => {
    const user = userEvent.setup();
    renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
    await user.click(await screen.findByRole("tab", { name: "Конфигурация" }));
    const row = (await screen.findByText("served_model_name")).closest("tr") as HTMLElement;
    expect(within(row).getByText("сведения о сервере")).toBeInTheDocument();
    expect(within(row).getByText("⚠ модели: qwen-old")).toBeInTheDocument();
    const secret = screen.getByText("api_key").closest("tr") as HTMLElement;
    expect(within(secret).getByText("задан (скрыт)")).toBeInTheDocument();
    expect(within(screen.getByText("block_size").closest("tr") as HTMLElement).getByText("метрики")).toBeInTheDocument();
    expect(api.readServerConfig).toHaveBeenCalledWith("i1", "s1");
  });

  it("«Метрики»: окно «Сейчас»; без источника истории окна недоступны, графики строятся", async () => {
    const user = userEvent.setup();
    renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
    await user.click(await screen.findByRole("tab", { name: "Метрики" }));
    expect(screen.getByRole("button", { name: "Сейчас" })).toHaveAttribute("aria-pressed", "true");
    const day = screen.getByRole("button", { name: "24 ч" });
    expect(day).toBeDisabled();
    expect(day).toHaveAttribute("title", "Добавьте источник истории в инсталляцию");
    expect(await screen.findByRole("region", { name: "Время до первого токена (TTFT)" })).toBeInTheDocument();
    expect(api.readServerHistory).not.toHaveBeenCalled();
  });

  it("источник есть, а серии сервера не сопоставлены — окна недоступны с подсказкой", async () => {
    vi.mocked(api.listInstallations).mockResolvedValue([{ ...PROFILE, history: HISTORY }]);
    const user = userEvent.setup();
    renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
    await user.click(await screen.findByRole("tab", { name: "Метрики" }));
    expect(screen.getByRole("button", { name: "7 д" })).toHaveAttribute("title", "Серии сервера не сопоставлены с историей");
    expect(api.readServerHistory).not.toHaveBeenCalled();
  });

  it("окно истории: запрос при выборе, шаг и источник, частичное окно; «Сейчас» начинается из истории", async () => {
    vi.mocked(api.listInstallations).mockResolvedValue([withHistory()]);
    const now = Date.now();
    vi.mocked(api.readServerHistory).mockImplementation(async (_i, _s, window) => ({
      window,
      stepSec: window === "now" ? 15 : 90,
      source: "prometheus",
      points: [{ at: now - 30_000, m: metrics(now - 30_000) }],
      partial: window === "24h" ? [{ chart: "ttft", error: { code: "TIMEOUT", key: "errors.history.timeout", message: "late", retryable: true } }] : [],
    }));
    const user = userEvent.setup();
    renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
    await user.click(await screen.findByRole("tab", { name: "Метрики" }));
    expect(api.readServerHistory).toHaveBeenCalledWith("i1", "s1", "now");
    expect(await screen.findByText(/^История до \d\d:\d\d$/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "24 ч" }));
    expect(api.readServerHistory).toHaveBeenLastCalledWith("i1", "s1", "24h");
    expect(await screen.findByText("Шаг 90 с · Prometheus")).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("Часть рядов не прочитана: Время до первого токена (TTFT) — Источник истории метрик не успел выполнить запрос: выберите окно короче");
    expect(screen.getByRole("button", { name: "24 ч" })).toHaveAttribute("aria-pressed", "true");
    await user.click(screen.getByRole("button", { name: "Обновить" }));
    await waitFor(() => expect(vi.mocked(api.readServerHistory).mock.calls.filter((c) => c[2] === "24h")).toHaveLength(2));
  });

  it("«Обзор» до второго опроса: скорости и перцентили — из истории, с пометкой", async () => {
    vi.mocked(api.listInstallations).mockResolvedValue([withHistory()]);
    const now = Date.now();
    vi.mocked(api.readServerHistory).mockResolvedValue({
      window: "now",
      stepSec: 15,
      source: "prometheus",
      points: [{ at: now - 15_000, m: metrics(now - 15_000) }],
      partial: [],
    });
    vi.mocked(api.readServerMetrics).mockResolvedValue(
      metrics(now, { intervalSec: null, rates: null, prefixCacheHitRatio: null, latency1m: { ttft: null, itl: null, e2e: null, queueTime: null }, preemptions1m: null }),
    );
    renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
    const ttft = await screen.findByRole("region", { name: "TTFT p95 за минуту" });
    await waitFor(() => expect(within(ttft).getByText("250 мс")).toBeInTheDocument());
    expect(within(ttft).getByText("по истории")).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "KV-кэш" })).queryByText("по истории")).toBeNull();
  });

  it("ошибка окна истории — вместо графиков", async () => {
    vi.mocked(api.listInstallations).mockResolvedValue([withHistory()]);
    vi.mocked(api.readServerHistory).mockImplementation(async (_i, _s, window) => {
      if (window === "now") throw { code: "CONNECTION_FAILED", key: "errors.history.timeout", message: "down", retryable: true };
      if (window === "24h") throw { code: "TIMEOUT", key: "errors.history.deadline", params: { seconds: "30" }, message: "late", retryable: true };
      throw { code: "AUTH_FAILED", key: "errors.history.unauthorized", message: "401", retryable: false };
    });
    const user = userEvent.setup();
    renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
    await user.click(await screen.findByRole("tab", { name: "Метрики" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Сейчас" })).toHaveAttribute("title", "История не загружена: Источник истории метрик не успел выполнить запрос: выберите окно короче"));
    await user.click(screen.getByRole("button", { name: "1 ч" }));
    expect(await screen.findByText(/Источник истории метрик отклонил вход \(401\)/)).toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "Время до первого токена (TTFT)" })).toBeNull();
    // Повтор не поможет — кнопки нет; таймаут — есть.
    expect(screen.queryByRole("button", { name: "Повторить" })).toBeNull();
    await user.click(screen.getByRole("button", { name: "24 ч" }));
    expect(await screen.findByText(/не отдал окно за 30 с/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Повторить" })).toBeInTheDocument();
  });

  it("сервер, который не подключился: его ошибка, без подвкладок и без опроса", async () => {
    renderWithClient(<ServerPanel installationId="i1" serverId="s2" />);
    expect(await screen.findByText("Сервер отклонил запрос (401) — проверьте ключ API")).toBeInTheDocument();
    expect(screen.queryByRole("tab")).toBeNull();
    expect(api.readServerMetrics).not.toHaveBeenCalled();
  });

  it("ошибка опроса — «Нет ответа сервера» в шапке", async () => {
    vi.mocked(api.readServerMetrics).mockRejectedValue({ code: "TIMEOUT", key: "errors.server.timeout", message: "timed out", retryable: true });
    renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
    expect(await screen.findByText("Нет ответа сервера: Сервер не ответил вовремя")).toBeInTheDocument();
  });

  it("OpenAI-совместимый сервер (Ollama): метрики не опрашиваются, «Нет ответа сервера» нет, у плиток подсказка", async () => {
    vi.mocked(api.connectInstallation).mockResolvedValue({
      installationId: "i1",
      servers: [{ serverId: "s1", facts: { kind: "openai", detected: "openai", version: null, models: [{ id: "llama3" }], metrics: false, warnings: [] } }],
    });
    renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
    const kv = await screen.findByRole("region", { name: "KV-кэш" });
    expect(within(kv).getByText("Нет данных")).toBeInTheDocument();
    expect(kv).toHaveAttribute("title", "Glafira читает метрики только vLLM и SGLang");
    expect(api.readServerMetrics).not.toHaveBeenCalled();
    expect(screen.queryByText(/Нет ответа сервера/)).toBeNull();
  });

  it("SGLang без --enable-metrics: метрики не опрашиваются, подсказка про флаг", async () => {
    vi.mocked(api.connectInstallation).mockResolvedValue({
      installationId: "i1",
      servers: [{ serverId: "s1", facts: { kind: "sglang", detected: "sglang", version: "0.5.4", models: [{ id: "m" }], metrics: false, warnings: [] } }],
    });
    renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
    const kv = await screen.findByRole("region", { name: "KV-кэш" });
    expect(kv).toHaveAttribute("title", "SGLang отдаёт метрики, только если запущен с --enable-metrics");
    expect(within(kv).getByText("Нет данных")).toBeInTheDocument();
    expect(api.readServerMetrics).not.toHaveBeenCalled();
    expect(screen.queryByText(/Нет ответа сервера/)).toBeNull();
  });

  it("плитка «Вытеснения» — число за минуту, а не скорость × 60", async () => {
    vi.mocked(api.readServerMetrics).mockResolvedValue(
      metrics(Date.now(), { preemptions1m: 7, rates: { promptTokens: 1, generationTokens: 1, requests: 1, preemptions: 5, aborted: 0, finishedByReason: {} } }),
    );
    renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
    const tile = await screen.findByRole("region", { name: "Вытеснения" });
    await waitFor(() => expect(within(tile).getByText("7/мин")).toBeInTheDocument());
  });

  it("после ошибки опроса плитки не выглядят свежими", async () => {
    const { client } = renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
    const kv = await screen.findByRole("region", { name: "KV-кэш" });
    await waitFor(() => expect(within(kv).getByText(/42\s?%/)).toBeInTheDocument());
    vi.mocked(api.readServerMetrics).mockRejectedValue({ code: "TIMEOUT", key: "errors.server.timeout", message: "timed out", retryable: true });
    await act(async () => {
      await client.refetchQueries({ queryKey: ["server-metrics", "i1", "s1"] });
    });
    expect(await screen.findByText("Нет ответа сервера: Сервер не ответил вовремя")).toBeInTheDocument();
    expect(within(kv).queryByText(/42\s?%/)).toBeNull();
    expect(within(kv).getByText("—")).toBeInTheDocument();
    expect(within(kv).queryByText("…")).toBeNull();
    expect(kv).toHaveAttribute("title", "Нет ответа сервера");
  });

  it("предупреждение о перезапуске остаётся минуту после сброса, а не на один опрос", async () => {
    vi.mocked(api.readServerMetrics).mockResolvedValue(metrics(Date.now(), { reset: true, rates: null }));
    const { client } = renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
    expect(await screen.findByText(/Счётчики уменьшились/)).toBeInTheDocument();
    vi.mocked(api.readServerMetrics).mockResolvedValue(metrics(Date.now() + 5_000));
    await act(async () => {
      await client.refetchQueries({ queryKey: ["server-metrics", "i1", "s1"] });
    });
    await waitFor(() => expect(within(screen.getByRole("region", { name: "Токены на входе" })).getByText("120/с")).toBeInTheDocument());
    expect(screen.getByText(/Счётчики уменьшились/)).toBeInTheDocument();
  });

  it("сброс старше минуты предупреждения не даёт", async () => {
    vi.mocked(api.readServerMetrics).mockResolvedValue(metrics(Date.now() - 61_000, { reset: true, rates: null }));
    renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
    const kv = await screen.findByRole("region", { name: "KV-кэш" });
    await waitFor(() => expect(within(kv).getByText(/42\s?%/)).toBeInTheDocument());
    expect(api.readServerMetrics).toHaveBeenCalled();
    expect(screen.queryByText(/Счётчики уменьшились/)).toBeNull();
  });

  it("версия ниже границы (тип openai, определён как vLLM): подсказка про версию, опроса нет", async () => {
    vi.mocked(api.connectInstallation).mockResolvedValue({
      installationId: "i1",
      servers: [{ serverId: "s1", facts: { kind: "openai", detected: "vllm", version: "0.7.3", models: [{ id: "m" }], metrics: false, warnings: [] } }],
    });
    renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
    const kv = await screen.findByRole("region", { name: "KV-кэш" });
    expect(kv).toHaveAttribute("title", "Версия ниже поддерживаемой: метрики читаются у vLLM с 0.10 и SGLang с 0.5");
    expect(api.readServerMetrics).not.toHaveBeenCalled();
  });

  it("ссылка на инсталляцию открывает её вкладку; удалённый сервер", async () => {
    const user = userEvent.setup();
    const { unmount } = renderWithClient(<ServerPanel installationId="i1" serverId="s1" />);
    await user.click(await screen.findByRole("button", { name: "prod-llm" }));
    expect(useWorkspace.getState().activeId).toBe("installation:i1");
    unmount();
    renderWithClient(<ServerPanel installationId="i1" serverId="gone" />);
    expect(await screen.findByText("Сервер или его инсталляция удалены")).toBeInTheDocument();
  });
});
