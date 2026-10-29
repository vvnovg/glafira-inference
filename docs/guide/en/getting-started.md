# Getting started

Glafira for LLM Inference is a desktop app for connecting to corporate open-source LLM inference, monitoring,
diagnosing and auditing it, with vLLM and SGLang in focus. Glafira changes nothing on the servers: it reads their
answers with your API keys. Glafira never reads or shows the texts of your users' requests and answers.

## Installation {#install}

Installers for macOS (`.dmg`), Windows (`.msi`) and Linux (`.deb`, `.rpm`, `.AppImage`) are published at
https://github.com/vvnovg/glafira-inference-releases/releases. Your organization can configure the app with a policy:
then some settings are set by the administrator.

## First installation {#first-installation}

An installation is the inference of one system: the vLLM, SGLang or other OpenAI-compatible servers that serve its
models.

1. Click [[ui:sidebar.newInstallation]] in the left panel.
2. Name the installation and choose its [[ui:installations.form.environment]].
3. For each server, give it a [[ui:installations.form.serverName]]: it is required, and the servers of one
   installation must have different names (letter case does not matter). Then enter the
   [[ui:installations.form.baseUrl]] — the address of the OpenAI-compatible API without `/v1`, such as
   `http://vllm.internal:8000` — and the [[ui:installations.form.apiKey]] if the server runs with `--api-key`. More in
   [Servers](installations.md#servers).
4. Click [[ui:installations.form.test]]: Glafira checks the address, TLS, `/health`, the key, the type and version of
   the server and its models step by step and shows the step that fails. Then click [[ui:common.save]].
5. The installation tab opens: which servers are connected, their type, version, models and whether they expose
   metrics.

## Interface language {#language}

Open [[ui:sidebar.settings]] and choose the [[ui:settings.language.label]]. By default it follows the system. If the
language is set by your organization, it cannot be changed.

## Help {#help}

F1 on any screen opens its chapter. The guide works offline; search is in the field above the contents.
