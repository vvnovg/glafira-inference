# Installations

An installation groups the inference servers of one system. Its profile is kept on your computer, the API keys in the
system secret store (macOS Keychain, Windows Credential Manager, Secret Service on Linux).

## Servers {#servers}

Add each server separately, replicas of one model behind a load balancer too: this shows which one answers
differently. The address is the OpenAI-compatible API without `/v1`: `http://host:port` or `https://host:port`. A path
is allowed for a server behind a reverse proxy (`https://gw.corp/llm/qwen`). A user name and password in the address
are not allowed.

The [[ui:installations.form.metricsPath]] is where the server exposes Prometheus metrics, usually `/metrics`. SGLang
exposes them only when started with `--enable-metrics`.

## Server type {#kind}

By default the type is detected from the answers: vLLM by `/version` and `vllm:` metrics, SGLang by `/model_info` and
`/server_info`. Other servers with `/v1/models` (Ollama, llama.cpp, Triton, TGI) work as OpenAI-compatible. vLLM older
than 0.10 and SGLang older than 0.5 work as OpenAI-compatible too, and Glafira warns about it.

Choose the type by hand only if detection is wrong: Glafira works with the chosen type and shows that the server
looks different.

## API key {#api-key}

The key is the same value as the `--api-key` of the server; Glafira sends it as `Authorization: Bearer`. The key is
kept in the system secret store; it is neither in the profile file nor in an export. An empty field of a saved
server keeps the previous key, the [[ui:installations.form.apiKeyClear]] checkbox deletes it.

Over an `http://` address the key travels unencrypted, and the form warns about it. Glafira does not follow redirects
to another address, so the key never goes where you did not send it.

## Server check {#check}

[[ui:installations.form.test]] checks the server with the values of the form, before saving: configuration, DNS and
TCP, TLS (for `https://`), `/health`, sign-in with the key, type and version, models. The check stops at the first
failed step and names it. Warnings (an old version, another type in the installation, no metrics) do not fail it.

The [[ui:installations.form.step.tcp]] step depends on the proxy (see [Certificates and proxy](installations.md#tls)).
If the system proxy applies to the server address, the step connects to nothing: it shows
[[ui:messages.connectionTest.tcpSystemProxy]], and the steps after it go through the proxy. With an explicit proxy
address, the step checks the TCP connection to the proxy itself, not to the server.

## Certificates and proxy {#tls}

[[ui:installations.form.ca]]: the system store is the CAs this computer trusts; listed certificates are only them,
without the system ones. Mode without certificate verification is insecure, and your organization can forbid it.

[[ui:installations.form.proxy]]: with [[ui:installations.form.proxyMode.system]], the proxy comes from the `HTTP_PROXY`,
`HTTPS_PROXY`, `ALL_PROXY` and `NO_PROXY` environment variables (`HTTP_PROXY` serves an `http://` server address,
`HTTPS_PROXY` an `https://` one), and then from the network settings of the OS: the system network settings on macOS,
the proxy settings on Windows. `NO_PROXY` lists the addresses that go without the proxy. You can also enter a proxy
address or turn the proxy off.

## Installation tab {#installation-tab}

When opened, the tab connects to all servers at once. For each one it shows whether it is connected, its type and
version, the models with their context length (the base model for LoRA adapters) and whether it exposes metrics. An
error of one server does not affect the others. [[ui:installations.panel.reconnect]] reads everything again, for
example after a server restart.

## Environments and read-only {#environments}

The environment sets the color of the stripe above the tabs and the badge. A new `prod` installation is created
read-only: Glafira will not run anything that loads or changes its servers.

## Folders, export and import {#folders}

Drag installations into folders. [[ui:sidebar.exportAll]] and the export of a folder write a file without API keys:
the recipient enters their own. Import shows what will be added and what updated. If an update changes the address,
the proxy or the certificate verification of a server with a saved key, the preview warns that the key will go to the
new address. A server that is missing from the file disappears from the updated installation together with its saved
key, and the preview warns about that too.
