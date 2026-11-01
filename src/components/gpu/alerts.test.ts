import { describe, expect, it } from "vitest";
import { cardAlerts, reasons } from "./alerts";
import { card } from "./testGpu";

describe("значки карты", () => {
  it("маска причин — по разрядам", () => {
    expect(reasons(0x48)).toEqual(["HW_SLOWDOWN", "HW_THERMAL"]);
    expect(reasons(0)).toEqual([]);
  });

  it("здоровая карта и простой без троттлинга — без значков", () => {
    expect(cardAlerts(card(0))).toEqual([]);
    expect(cardAlerts(card(0, { values: { throttle: 0x1 | 0x2 } }))).toEqual([]);
    expect(cardAlerts(card(0, { values: {} }))).toEqual([]);
  });

  it("троттлинг, XID, ECC, NVLink и PCIe", () => {
    const bad = card(0, {
      values: { throttle: 0x48, xid: 79, xidCount: 1, eccDbe: 2, remapUncorrectable: 1 },
      deltas: { powerViolation: 40, eccSbe: 3, nvlinkErrors: 3, pcieReplay: 6 },
    });
    expect(cardAlerts(bad)).toEqual([
      { kind: "throttle", severity: "warn", params: { reasons: "HW_SLOWDOWN, HW_THERMAL, POWER 40 %" } },
      { kind: "xid", severity: "danger", params: { code: "79", count: "1" } },
      { kind: "ecc", severity: "danger", params: { dbe: "2", remap: "1", failed: "0" } },
      { kind: "nvlink", severity: "warn", params: { count: "3" } },
      { kind: "pcie", severity: "warn", params: { count: "6" } },
    ]);
    expect(cardAlerts(card(0, { values: {}, deltas: { eccSbe: 2 } }))).toEqual([{ kind: "eccCorrectable", severity: "warn", params: { count: "2" } }]);
    expect(cardAlerts(card(0, { values: {}, deltas: { thermalViolation: 12.345 } }))[0].params.reasons).toBe("THERMAL 12.3 %");
  });

  it("сбой переназначения строк без ошибок ECC — значок называет причину", () => {
    expect(cardAlerts(card(0, { values: { eccDbe: 0, remapFailure: 1 } }))).toEqual([
      { kind: "eccRemapFailure", severity: "danger", params: { dbe: "0", remap: "0", failed: "1" } },
    ]);
    // Некорректируемые ошибки главнее: бейдж остаётся «ECC: N».
    expect(cardAlerts(card(0, { values: { eccDbe: 2, remapFailure: 1 } }))[0].kind).toBe("ecc");
  });

  it("переназначенные строки без ошибок и сбоев (после перезагрузки драйвера) — свой значок, не «ECC: 0»", () => {
    expect(cardAlerts(card(0, { values: { eccDbe: 0, remapUncorrectable: 3 } }))).toEqual([
      { kind: "eccRemapped", severity: "danger", params: { dbe: "0", remap: "3", failed: "0", count: "3" } },
    ]);
    // Порядок: ошибки → сбой переназначения → переназначенные строки → рост корректируемых.
    expect(cardAlerts(card(0, { values: { eccDbe: 1, remapUncorrectable: 3 } }))[0].kind).toBe("ecc");
    expect(cardAlerts(card(0, { values: { remapFailure: 1, remapUncorrectable: 3 } }))[0].kind).toBe("eccRemapFailure");
    expect(cardAlerts(card(0, { values: { remapUncorrectable: 3 }, deltas: { eccSbe: 2 } })).map((a) => a.kind)).toEqual(["eccRemapped"]);
  });
});
