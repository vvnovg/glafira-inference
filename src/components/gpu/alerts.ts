import type { GpuCard } from "../../api/types";

/** Разряды маски причин снижения тактов (`DCGM_CLOCKS_EVENT_REASON_*`), как в `gpu/fields.rs`. */
export const REASONS: [number, string][] = [
  [0x1, "GPU_IDLE"],
  [0x2, "CLOCKS_SETTING"],
  [0x4, "SW_POWER_CAP"],
  [0x8, "HW_SLOWDOWN"],
  [0x10, "SYNC_BOOST"],
  [0x20, "SW_THERMAL"],
  [0x40, "HW_THERMAL"],
  [0x80, "HW_POWER_BRAKE"],
  [0x100, "DISPLAY_CLOCKS"],
];

/** Причины, которые отмечаются значком «троттлинг»; остальные — только в подсказке. */
export const ALERT_REASONS = new Set(["SW_POWER_CAP", "HW_SLOWDOWN", "SW_THERMAL", "HW_THERMAL", "HW_POWER_BRAKE"]);

/** Причины из маски по порядку разрядов. */
export function reasons(mask: number): string[] {
  return REASONS.filter(([bit]) => (mask & bit) !== 0).map(([, name]) => name);
}

export type AlertKind = "throttle" | "xid" | "ecc" | "eccCorrectable" | "nvlink" | "pcie";

/** Значок проблемы карты: вид, серьёзность и параметры подсказки (`gpu.alerts.<вид>`). */
export interface CardAlert {
  kind: AlertKind;
  severity: "warn" | "danger";
  params: Record<string, string>;
}

const fmt = (v: number) => String(Math.round(v * 10) / 10);

/**
 * Значки карты (спека I1d §5): троттлинг (причины из маски или доля времени по счётчикам), последняя XID-ошибка,
 * некорректируемые ECC и сбои переназначения строк, корректируемые ECC, ошибки NVLink и повторы PCIe за минуту.
 */
export function cardAlerts(c: GpuCard): CardAlert[] {
  const v = c.values;
  const d = c.deltas ?? {};
  const out: CardAlert[] = [];
  const throttling = v.throttle != null ? reasons(v.throttle).filter((r) => ALERT_REASONS.has(r)) : [];
  const violations = [
    ["POWER", d.powerViolation],
    ["THERMAL", d.thermalViolation],
  ].filter((x): x is [string, number] => typeof x[1] === "number" && x[1] > 0);
  if (throttling.length > 0 || violations.length > 0)
    out.push({
      kind: "throttle",
      severity: "warn",
      params: { reasons: [...throttling, ...violations.map(([name, share]) => `${name} ${fmt(share)} %`)].join(", ") },
    });
  if (v.xid != null && v.xid > 0) out.push({ kind: "xid", severity: "danger", params: { code: String(v.xid), count: String(v.xidCount ?? "—") } });
  const dbe = v.eccDbe ?? 0;
  const remapUncorrectable = v.remapUncorrectable ?? 0;
  const remapFailure = v.remapFailure ?? 0;
  if (dbe > 0 || remapUncorrectable + remapFailure > 0)
    out.push({ kind: "ecc", severity: "danger", params: { dbe: String(dbe), remap: String(remapUncorrectable), failed: String(remapFailure) } });
  else if ((d.eccSbe ?? 0) > 0) out.push({ kind: "eccCorrectable", severity: "warn", params: { count: String(d.eccSbe) } });
  if ((d.nvlinkErrors ?? 0) > 0) out.push({ kind: "nvlink", severity: "warn", params: { count: String(d.nvlinkErrors) } });
  if ((d.pcieReplay ?? 0) > 0) out.push({ kind: "pcie", severity: "warn", params: { count: String(d.pcieReplay) } });
  return out;
}
