//! OpenAI-совместимый сервер (llama.cpp, Ollama, vLLM и SGLang ниже границы версий):
//! канонических серий нет, экран показывает «нет данных»; аргументы — только из `/v1/models`.

use super::{MetricSpec, ServerAdapter};
use crate::servers::config::{ConfigEntry, ConfigInput, merge, model_fields};

pub struct Openai;

impl ServerAdapter for Openai {
    fn metric_map(&self) -> &'static [MetricSpec] {
        &[]
    }

    /// Только то, что видно в `/v1/models`.
    fn config(&self, input: &ConfigInput) -> Vec<ConfigEntry> {
        merge(
            model_fields(&input.models, "served_model_name", "model", "max_model_len"),
            &["model", "served_model_name", "max_model_len"],
        )
    }
}
