import { render, screen } from "@testing-library/react";
import { vi } from "vitest";

vi.mock("./api/client", async (importOriginal) => {
  const orig = await importOriginal<typeof import("./api/client")>();
  return {
    ...orig,
    api: {
      getLanguageSettings: vi.fn(() => new Promise(() => {})),
      listInstallations: vi.fn().mockResolvedValue([]),
      listFolders: vi.fn().mockResolvedValue([]),
    },
  };
});

import App from "./App";

test("первый запуск: пустое дерево с подсказкой и пустая рабочая область", async () => {
  render(<App />);
  expect(await screen.findByText(/Добавьте первую инсталляцию/)).toBeInTheDocument();
  expect(screen.getByText("Выберите инсталляцию слева или добавьте новую.")).toBeInTheDocument();
});
