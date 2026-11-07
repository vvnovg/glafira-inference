# Changelog

Versions in the other interface languages are in [docs/changelog](docs/changelog/). The section of a version is shown in About as the update notes, in the interface language.

## 0.1.0
First version of Glafira for LLM Inference: a desktop app for corporate open-source LLM inference inside your network, with vLLM and SGLang in focus. This version covers connecting, checking and monitoring; diagnostics and audit come in the next versions.
- Installations:
  - an installation groups the inference servers of one system: vLLM, SGLang or any OpenAI-compatible server;
  - `http://` and `https://` addresses, the system CA store, listed certificates or no verification (if the policy allows it), the system proxy or an explicit one;
  - API keys are kept only in the system keychain;
  - folders, drag and drop, export and import without keys.
- Server check, step by step: configuration, DNS/TCP, TLS, `/health`, API key, kind and version, models.
- The installation tab: for each server, the kind and version, models with context length and LoRA adapters, metrics availability and errors.
- The server tab: live metrics from `/metrics` every 5 seconds while the tab is open (requests running and waiting, KV cache, tokens per second, TTFT and ITL percentiles, preemptions), the launch arguments with their source, and charts for the last hour.
- Metric history from Prometheus or VictoriaMetrics that already scrapes the servers: windows of 1 hour to 30 days, server matching checked step by step; Glafira stores nothing itself.
- NVIDIA GPUs from DCGM metrics (dcgm-exporter directly or through Prometheus): utilization, SM activity, memory, temperature, power, clocks and trouble signs (throttling, XID, ECC, NVLink, PCIe replays), per host and per card, with charts.
- A macOS look: the title bar is merged with the sidebar and the tab bar, the system accent and the macOS colors in both themes, one connection tree style with icons and guide lines, the item of the active tab highlighted, icons instead of text symbols. Keyboard: ⌘N new installation, ⌘W close tab, ⌘R refresh, ⌘F find, ⌘1…⌘9 and ⌃Tab switch tabs, ⌘, settings; on macOS in the application menu, on Windows and Linux the same with Ctrl. The window remembers its size and position.
- Glafira never reads or shows the texts of your users' requests and answers.
- Updates from About → Updates, with a signature check. Installers for macOS (the app is signed and notarized by Apple), Windows and Linux.
- Guide in English and Russian.
