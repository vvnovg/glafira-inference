//! OpenAI-совместимый сервер (llama.cpp, Ollama, vLLM и SGLang ниже границы версий):
//! канонических серий нет, экран показывает «нет данных».

use super::{MetricSpec, ServerAdapter};

pub struct Openai;

impl ServerAdapter for Openai {
    fn metric_map(&self) -> &'static [MetricSpec] {
        &[]
    }
}
