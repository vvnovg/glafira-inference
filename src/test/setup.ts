import "@testing-library/jest-dom/vitest";
import { i18n } from "../i18n";

// uPlot при импорте читает window.matchMedia (плотность пикселей), а в jsdom её нет.
window.matchMedia ??= ((query: string) => ({
  matches: false,
  media: query,
  onchange: null,
  addListener: () => {},
  removeListener: () => {},
  addEventListener: () => {},
  removeEventListener: () => {},
  dispatchEvent: () => false,
})) as typeof window.matchMedia;

await i18n.changeLanguage("ru");
