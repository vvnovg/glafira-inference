import { currentLocale } from "../../i18n";

/** Число с разделителями локали; `digits` знаков после запятой (не больше). */
export function fmtNumber(v: number, digits = 0): string {
  return v.toLocaleString(currentLocale(), { maximumFractionDigits: digits });
}

/** Доля 0…1 как процент локали: `42%`, `42 %`. */
export function fmtPercent(ratio: number): string {
  return ratio.toLocaleString(currentLocale(), { style: "percent", maximumFractionDigits: ratio < 0.1 ? 1 : 0 });
}

/** Мелкие значения — с десятыми, крупные — целыми: «0,4», «12», «1 250». */
export function fmtAdaptive(v: number): string {
  return fmtNumber(v, Math.abs(v) < 10 ? 1 : 0);
}
