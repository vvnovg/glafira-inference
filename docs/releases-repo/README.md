# Glafira for LLM Inference — releases

Desktop app for corporate open-source LLM inference inside your network, with vLLM and SGLang in focus: connecting to
the servers, checking them, and later monitoring, diagnostics and audit. Download the installer for your OS from
[Releases](../../releases/latest).

**macOS** — open the `.dmg` and drag the app to Applications (or run the `.pkg`). The builds are signed by Apple and notarized, so the app opens right away. If macOS says the app is damaged or cannot be verified, the download is corrupted: download it again.

User guide: [English](guide/en/getting-started.md) · [Русский](guide/ru/getting-started.md)

## Windows builds (unsigned)
macOS builds are signed by Apple and notarized. Windows builds are not signed yet, so SmartScreen may warn on first launch:

**Windows** — in the SmartScreen dialog click "More info" → "Run anyway", or unblock the installer in PowerShell:
```powershell
Unblock-File -Path "$env:USERPROFILE\Downloads\Glafira_for_LLM_Inference_*_x64_en-US.msi"
```

## Source code and support
Glafira is free and open source under the Apache License 2.0: https://github.com/vvnovg/glafira-inference.
All features are available to everyone. If Glafira is useful to you, see [SUPPORT.md](SUPPORT.md).

vLLM, SGLang, LiteLLM, NVIDIA and other product names are trademarks of their respective owners. This project is not
affiliated with them.
