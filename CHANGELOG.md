# Changelog

Versions in the other interface languages are in [docs/changelog](docs/changelog/). The section of a version is shown in About as the update notes, in the interface language.

## 0.1.0
First version of Glafira for LLM Inference: a desktop app for corporate open-source LLM inference inside your network, with vLLM and SGLang in focus. This version covers connecting and checking; monitoring, diagnostics and audit come in the next versions.
- Installations:
  - an installation groups the inference servers of one system: vLLM, SGLang or any OpenAI-compatible server;
  - `http://` and `https://` addresses, the system CA store, listed certificates or no verification (if the policy allows it), the system proxy or an explicit one;
  - API keys are kept only in the system keychain;
  - folders, drag and drop, export and import without keys.
- Server check, step by step: configuration, DNS/TCP, TLS, `/health`, API key, kind and version, models.
- The installation tab: for each server, the kind and version, models with context length and LoRA adapters, metrics availability and errors.
- Glafira never reads or shows the texts of your users' requests and answers.
- Updates from About → Updates, with a signature check. Installers for macOS (the app is signed and notarized by Apple), Windows and Linux.
- Guide in English and Russian.
