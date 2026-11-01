import type { TFunction } from "i18next";
import { describe, expect, it } from "vitest";
import { fmtNumber } from "../servers/format";
import { gpuCharts, historyCards, mergeCards } from "./gpuPoints";

const t = ((key: string, o?: { value?: string }) => (o?.value != null ? `${o.value} ${key}` : key)) as unknown as TFunction;

describe("графики GPU", () => {
  it("точки карт объединяются по времени; ошибка у всех карт — разрыв", () => {
    const pts = mergeCards({
      0: [{ at: 10, values: { util: 1 } }, { at: 20, values: null }, { at: 30, values: { util: 3 } }],
      1: [{ at: 20, values: null }, { at: 30, values: { util: 4 } }],
    });
    expect(pts).toEqual([
      { at: 10, m: { 0: { util: 1 } } },
      { at: 20, m: null },
      { at: 30, m: { 0: { util: 3 }, 1: { util: 4 } } },
    ]);
  });

  it("ряды истории хоста — по номерам карт", () => {
    const h = historyCards(
      { window: "1h", stepSec: 60, source: "prometheus", partial: [], series: [{ host: "a", gpu: 2, points: [{ at: 1, values: {} }] }, { host: "b", gpu: 0, points: [] }] },
      "a",
    );
    expect(Object.keys(h)).toEqual(["2"]);
    expect(historyCards(undefined, "a")).toEqual({});
  });

  it("у карты — поля рядом, у хоста — ряд на карту; память в ГиБ", () => {
    const single = gpuCharts(t, [3], true);
    expect(single.map((c) => c.id)).toEqual(["util", "memory", "temp", "power", "clocks"]);
    expect(single[0].picks.map((p) => p.label)).toEqual(["gpu.charts.util", "gpu.charts.sm"]);
    expect(single[0].picks[1].pick({ 3: { smActive: 55 } })).toBe(55);
    expect(single[1].format(81920)).toBe(`${fmtNumber(80, 1)} gpu.units.gib`);
    const host = gpuCharts(t, [0, 1, 2], false);
    expect(host[0].picks.map((p) => p.label)).toEqual(["GPU 0", "GPU 1", "GPU 2"]);
    expect(host[3].picks[2].pick({ 2: { power: 400 } })).toBe(400);
    expect(host[3].picks[1].pick({ 2: { power: 400 } })).toBeUndefined();
  });
});
