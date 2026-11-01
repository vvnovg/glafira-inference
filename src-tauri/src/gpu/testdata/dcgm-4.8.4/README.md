# Снимки `/metrics` экспортёра DCGM

Сняты с мок-сервера (`scripts/mock-server.mjs`, виды `dcgm*`), а не с карты: стенда с GPU у разработки нет. Формат —
dcgm-exporter 4.8.4, собран по его исходникам (`etc/default-counters.csv`, `internal/pkg/rendermetrics/render_metrics.go`):
метки карты `gpu`, `UUID`, `pci_bus_id`, `device`, `modelName`, хост — `hostname` (у 4.1 — `Hostname`), версия драйвера —
меткой `DCGM_FI_DRIVER_VERSION`, у MIG — `GPU_I_PROFILE` и `GPU_I_ID`, с `--pod` — `container`, `namespace`, `pod`.

| Файл | Вид мока и параметры | Опрос |
|---|---|---|
| `default.txt` | `dcgm --gpus 2` (набор по умолчанию: без активности SM, ECC и причин троттлинга) | 0 |
| `full.txt` | `dcgm-full --gpus 8 --hostname gpu-node-1 --pod llm/vllm-0 --throttle --xid 79 --ecc` | 4 |
| `full-next.txt` | то же | 5 |
| `4.1.txt` | `dcgm-4.1 --gpus 2 --hostname gpu-old` (метка `Hostname`, A100) | 0 |
| `mig.txt` | `dcgm-mig --gpus 2 --hostname gpu-mig` (карта 0 — два экземпляра MIG) | 0 |

Пересоздать (из корня репозитория):

```bash
node --input-type=module -e '
import { dcgmMetrics, parseArgs } from "./scripts/mock-server.mjs";
import { writeFileSync } from "node:fs";
const kinds = { dcgm: "default", "dcgm-full": "full", "dcgm-4.1": "4.1", "dcgm-mig": "mig" };
const make = (file, args, poll) => {
  const o = parseArgs(args);
  writeFileSync(`src-tauri/src/gpu/testdata/dcgm-4.8.4/${file}`, dcgmMetrics(kinds[o.kind], o.dcgm, poll));
};
const full = ["--kind", "dcgm-full", "--gpus", "8", "--hostname", "gpu-node-1", "--pod", "llm/vllm-0", "--throttle", "--xid", "79", "--ecc"];
make("default.txt", ["--kind", "dcgm", "--gpus", "2"], 0);
make("full.txt", full, 4);
make("full-next.txt", full, 5);
make("4.1.txt", ["--kind", "dcgm-4.1", "--gpus", "2", "--hostname", "gpu-old"], 0);
make("mig.txt", ["--kind", "dcgm-mig", "--gpus", "2", "--hostname", "gpu-mig"], 0);
'
```
