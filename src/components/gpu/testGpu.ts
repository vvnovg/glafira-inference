import type { GpuCard, GpuHost, GpuStatus } from "../../api/types";

/** Карта для тестов: H100 с полным набором полей; `over` — свои значения. */
export function card(index: number, over: Partial<GpuCard> = {}): GpuCard {
  return {
    index,
    uuid: `GPU-${index}`,
    model: "NVIDIA H100 80GB HBM3",
    smSource: "sm",
    values: { util: 60 + index, smActive: 55, memUsed: 61440, memTotal: 81559, temp: 60, power: 450, smClock: 1980, memClock: 2619, throttle: 0, xid: 0, eccDbe: 0, nvlinkErrors: 0, pcieReplay: 0 },
    deltas: { eccDbe: 0, pcieReplay: 0, nvlinkErrors: 0 },
    ...over,
  };
}

export function host(name: string, cards: GpuCard[], driver = "570.124.06"): GpuHost {
  return { host: name, driver, cards };
}

export function status(at: number, hosts: GpuHost[], over: Partial<GpuStatus> = {}): GpuStatus {
  return { at, hosts, errors: [], missing: [], deltaSec: 5, ...over };
}
