import { render, screen } from "@testing-library/react";
import { vi } from "vitest";

vi.mock("./api/client", () => ({
  api: { getLanguageSettings: vi.fn(() => new Promise(() => {})) },
  toAppError: (e: unknown) => e,
}));

import App from "./App";

test("показывает имя продукта", () => {
  render(<App />);
  expect(screen.getByText("Glafira for LLM Inference")).toBeInTheDocument();
});
