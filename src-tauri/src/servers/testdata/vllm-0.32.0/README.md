# vLLM 0.32.0 — настоящий снимок

Снят 2026-10-30 на Apple Silicon (M4 Pro): vLLM 0.32.0 с плагином `vllm-metal` 0.32.0 (MLX),
модель `mlx-community/Qwen2.5-0.5B-Instruct-4bit`, `vllm serve … --api-key <ключ>`.

- `version.json`, `load.json`, `models.json` — ответы `/version`, `/load`, `/v1/models`;
- `metrics-before.txt` — `/metrics` сразу после запуска;
- `metrics-after.txt` — после 30 коротких запросов (10 без потока, 20 потоком, по 5 параллельно).

Ключ API и тексты запросов в файлы не попали. Отличия от сервера на CUDA — в метках
`vllm:cache_config_info` (устройство, размер кэша); имена канонических серий те же.
