# GPU

Glafira shows the NVIDIA GPUs of an installation from DCGM metrics: utilization, SM activity, memory, temperature,
power, clocks and signs of trouble — throttling, XID errors, ECC, NVLink errors and PCIe replays. The metrics come from
[dcgm-exporter](https://github.com/NVIDIA/dcgm-exporter); Glafira only reads them and writes nothing to disk.

## GPU source {#source}

In the installation form, section [[ui:installations.gpu.title]], choose the source:

- **[[ui:installations.gpu.modes.direct]]** — Glafira reads `/metrics` of the exporters itself. Enter the addresses,
  such as `http://gpu-node-1:9400/metrics` (without a path `/metrics` is used), up to 16 exporters. Sign-in — none or a
  user name and password (the exporter `web-config.yml`); the password is shared by all exporters and is kept only in the
  system secret store. CA and proxy — as for servers.
- **[[ui:installations.gpu.modes.prometheus]]** — the same DCGM series from the Prometheus or VictoriaMetrics of the
  installation. Handy when the exporters are not reachable from the workstation. The
  [[ui:installations.gpu.selector]] picks the GPUs of the installation, such as `{hostname=~"gpu-node-1|gpu-node-2"}`
  or `{cluster="llm"}`; the check fills an empty selector from the hosts of the servers. An installation can have at most
  64 GPUs.

Click [[ui:installations.gpu.check]]: the check goes through every exporter (DNS/TCP, `/metrics`, GPUs) or finds the
GPUs in the history source, then binds the servers and matches the exporters with the history. A GPU failure does not
stop the servers from connecting.

## Binding servers {#binding}

Every server in the form has the [[ui:installations.gpu.binding]] field: a GPU host and all or some of its cards. With
the binding the server overview shows its GPUs, and the rules of the next stages relate the server queue to the GPU load.

- The check binds a server itself when the host of its address matches a GPU host: names ignore case, a short node name
  (`gpu-node-1`) matches the full one (`gpu-node-1.corp.local`), `localhost` and `127.0.0.1` are one address. The check
  does not overwrite a choice made by a person; if the server address changes after the auto-match, the binding resets.
- Behind a load balancer or a Kubernetes Service the server address is not a node address: choose the host manually.
- If the server takes some of the cards (say 4 of 8), clear [[ui:installations.gpu.allCards]] and tick its cards.

The host of a GPU is the `hostname` label of the exporter (since dcgm-exporter 4.8; `Hostname` in 4.1 and earlier), and
without it the host of the exporter address or the `instance` label.

## GPU tab {#tab}

In the tree an installation with a GPU component has a “GPU” item. The tab shows the hosts (driver, GPU count, bound
servers) and GPU cards. A field without a series reads “no data”, not zero. Badges:

| Badge | Meaning |
|---|---|
| Throttling | Clocks are reduced by power or temperature (`SW_POWER_CAP`, `HW_SLOWDOWN`, `SW_THERMAL`, `HW_THERMAL`, `HW_POWER_BRAKE`) or by the throttle time counters |
| XID | The last XID error of the driver; stays until the driver reloads |
| ECC | Uncorrectable ECC errors or remapped memory rows — a reason to replace the card |
| ECC +N | Correctable ECC errors in the last minute |
| NVLink +N, PCIe +N | NVLink errors and PCIe replays in the last minute |

“Workload” is the pod and namespace from the Kubernetes labels of the exporter. GPUs in MIG mode list their instances;
Glafira does not show instance metrics yet.

A click on a card opens its charts (utilization and SM, memory, GPU and memory temperature, power, clocks), a click on a
host — the charts of all its cards, a series per card. The “Now” window polls every 5 s while the tab is visible and
keeps up to an hour of points in memory; the 1 h, 24 h, 7 d and 30 d windows come from the history source if it has the
DCGM series. In [[ui:installations.gpu.modes.direct]] mode every exporter in the form has the
[[ui:installations.gpu.exporterSeries]] field: the Prometheus target with the same `instance`, which the check picks
itself.

The installation overview shows GPU by host, the server overview — the GPUs of its binding.

## Counters to enable {#counters}

The default counter set of dcgm-exporter (`default-counters.csv`) does not collect some of what Glafira needs. Without
them the cards work, but some fields read “no data”, and the check tells what is missing. Add to the exporter counter
file (`-f`, `--collectors`):

```
DCGM_FI_PROF_SM_ACTIVE,          gauge,   The ratio of cycles an SM has at least 1 warp assigned.
DCGM_FI_DEV_ECC_SBE_VOL_TOTAL,   counter, Total number of single-bit volatile ECC errors.
DCGM_FI_DEV_ECC_DBE_VOL_TOTAL,   counter, Total number of double-bit volatile ECC errors.
DCGM_FI_DEV_CLOCKS_EVENT_REASONS, gauge,  Current clock event reasons.
DCGM_FI_DEV_POWER_VIOLATION,     counter, Throttling duration due to power constraints (in ns).
DCGM_FI_DEV_THERMAL_VIOLATION,   counter, Throttling duration due to thermal constraints (in ns).
```

NVLink errors are `DCGM_FI_DEV_NVLINK_CRC_ERROR_TOTAL`, `DCGM_FI_DEV_NVLINK_RECOVERY_TOTAL`,
`DCGM_FI_DEV_NVLINK_REPLAY_TOTAL` (Hopper) or `DCGM_FI_DEV_NVLINK_*_ERROR_TOTAL` (earlier). Without SM activity Glafira
shows the graphics engine activity (`DCGM_FI_PROF_GR_ENGINE_ACTIVE`) marked “by GR engine”.

## Troubleshooting {#troubleshooting}

- “There are no DCGM metrics” — the address answers, but it is not dcgm-exporter: check the port (9400 by default) and
  the path.
- “The DCGM exporter rejected the request” — wrong user name or password.
- “Found more than 64 GPUs” — narrow the card selector or remove exporters.
- “The hosts of the servers are not among the GPU hosts” — choose the GPU hosts in the form or bind the servers
  manually.
- History windows are disabled — the installation has no history source or the exporters are not matched with its
  targets.

## Prometheus configuration example {#prometheus-config}

```yaml
scrape_configs:
  - job_name: dcgm
    static_configs:
      - targets: ["gpu-node-1:9400", "gpu-node-2:9400"]
```

In Kubernetes dcgm-exporter usually comes with the GPU Operator and is scraped through a ServiceMonitor; the exporter
adds the `pod` and `namespace` labels itself.
