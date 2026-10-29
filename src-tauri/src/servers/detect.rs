//! Определение типа сервера по ответам (спец. §4.3) — чистые функции над снимком `Probe`.
//! Признаки: `/model_info` и `/server_info` SGLang (у старых версий — `/get_model_info` и
//! `/get_server_info`), `/version` vLLM, префиксы серий в `/metrics` (`vllm:`, `sglang:`) и поле
//! `owned_by` в `/v1/models`. Тип из профиля побеждает признаки, но расхождение — предупреждение.

use serde::{Deserialize, Serialize};

use crate::error::AppResult;
use crate::i18n::LocalizedText;
use crate::servers::client::parse_json;
use crate::storage::profile::ServerKind;

/// Тип сервера, с которым работает Glafira.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DetectedKind {
    Vllm,
    Sglang,
    Openai,
}

impl DetectedKind {
    pub fn label(self) -> &'static str {
        match self {
            DetectedKind::Vllm => "vLLM",
            DetectedKind::Sglang => "SGLang",
            DetectedKind::Openai => "OpenAI-compatible",
        }
    }

    /// Нижняя граница версии, с которой работают адаптеры (спец. §1); ниже — `openai`.
    pub fn min_version(self) -> Option<&'static str> {
        match self {
            DetectedKind::Vllm => Some("0.8"),
            DetectedKind::Sglang => Some("0.4"),
            DetectedKind::Openai => None,
        }
    }

    fn from_configured(k: ServerKind) -> Option<Self> {
        match k {
            ServerKind::Auto => None,
            ServerKind::Vllm => Some(DetectedKind::Vllm),
            ServerKind::Sglang => Some(DetectedKind::Sglang),
            ServerKind::Openai => Some(DetectedKind::Openai),
        }
    }
}

/// Ответ на один запрос снимка: код и тело (не больше лимита клиента).
#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    pub status: u16,
    pub body: Vec<u8>,
}

impl Answer {
    /// JSON-объект ответа 2xx; иначе `None` (404, HTML обратного прокси, обрыв).
    fn object(&self) -> Option<serde_json::Map<String, serde_json::Value>> {
        if !(200..300).contains(&self.status) {
            return None;
        }
        match serde_json::from_slice(&self.body).ok()? {
            serde_json::Value::Object(m) => Some(m),
            _ => None,
        }
    }
}

/// Снимок ответов сервера, по которому определяется тип. `None` — запрос не удался.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Probe {
    pub version: Option<Answer>,
    pub model_info: Option<Answer>,
    pub server_info: Option<Answer>,
    /// Начало `/metrics`, если он ответил 2xx.
    pub metrics: Option<String>,
    /// `/v1/models` — обязательный ответ: без него сервер не OpenAI-совместимый.
    pub models: Vec<u8>,
}

/// Модель из `/v1/models`. У адаптера LoRA `parent` — базовая модель.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_model_len: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owned_by: Option<String>,
}

#[derive(Deserialize)]
struct ModelList {
    data: Vec<RawModel>,
}

/// Модель как её отдают серверы: `max_model_len` у vLLM бывает `null`, у прочих его нет.
#[derive(Deserialize)]
struct RawModel {
    id: String,
    #[serde(default)]
    root: Option<String>,
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    max_model_len: Option<u64>,
    #[serde(default)]
    owned_by: Option<String>,
}

/// Модели из ответа `/v1/models`.
pub fn parse_models(body: &[u8]) -> AppResult<Vec<Model>> {
    let list: ModelList = parse_json(body, "/v1/models")?;
    Ok(list
        .data
        .into_iter()
        .map(|m| Model {
            id: m.id,
            root: m.root,
            parent: m.parent,
            max_model_len: m.max_model_len,
            owned_by: m.owned_by,
        })
        .collect())
}

/// Что известно о сервере после подключения.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerFacts {
    /// Тип, с которым работает Glafira: из профиля или по признакам; версия ниже границы — `openai`.
    pub kind: DetectedKind,
    /// Тип по признакам сервера.
    pub detected: DetectedKind,
    pub version: Option<String>,
    pub models: Vec<Model>,
    /// `/metrics` отвечает, и в нём есть серии этого типа.
    pub metrics: bool,
    pub warnings: Vec<LocalizedText>,
}

fn has_series(metrics: &Option<String>, prefix: &str) -> bool {
    metrics.as_deref().is_some_and(|m| {
        m.lines()
            .any(|l| l.starts_with(prefix) || l.starts_with(&format!("# TYPE {prefix}")))
    })
}

/// Тип по признакам, без учёта профиля.
pub fn detect(p: &Probe, models: &[Model]) -> DetectedKind {
    let model_info = p.model_info.as_ref().and_then(Answer::object);
    let server_info = p.server_info.as_ref().and_then(Answer::object);
    let owned = |who: &str| models.iter().any(|m| m.owned_by.as_deref() == Some(who));
    if model_info.is_some_and(|m| m.contains_key("model_path"))
        || server_info.is_some_and(|m| m.contains_key("model_path") && m.contains_key("version"))
        || has_series(&p.metrics, "sglang:")
        || owned("sglang")
    {
        return DetectedKind::Sglang;
    }
    let version_json = p
        .version
        .as_ref()
        .and_then(Answer::object)
        .is_some_and(|m| m.get("version").is_some_and(|v| v.is_string()));
    if has_series(&p.metrics, "vllm:") || owned("vllm") || (version_json && owned_none(models)) {
        return DetectedKind::Vllm;
    }
    DetectedKind::Openai
}

/// У моделей нет `owned_by` вовсе — обратный прокси мог его вырезать; `/version` тогда решает.
fn owned_none(models: &[Model]) -> bool {
    models.iter().all(|m| m.owned_by.is_none())
}

/// Версия сервера: vLLM — `/version`, SGLang — `version` в сведениях о сервере.
fn version_of(kind: DetectedKind, p: &Probe) -> Option<String> {
    let from = |a: &Option<Answer>| {
        a.as_ref()
            .and_then(Answer::object)
            .and_then(|m| m.get("version")?.as_str().map(String::from))
            .filter(|v| !v.trim().is_empty())
    };
    match kind {
        DetectedKind::Vllm => from(&p.version),
        DetectedKind::Sglang => from(&p.server_info).or_else(|| from(&p.version)),
        DetectedKind::Openai => None,
    }
}

/// Числа начала версии: `0.10.1.1`, `0.6.3.post1`, `0.5.0rc1`, `0.11.0+cu128` → `[0, 10, 1]`.
/// Берутся первые три числа; суффиксы сборок не учитываются.
pub fn version_numbers(v: &str) -> Vec<u64> {
    v.trim()
        .trim_start_matches('v')
        .split('.')
        .take(3)
        .map_while(|part| {
            let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
            digits.parse().ok()
        })
        .collect()
}

/// Версия `v` не ниже `min`. Неразборчивая версия считается поддерживаемой: Glafira не
/// отказывается от сервера из-за необычной строки.
pub fn version_at_least(v: &str, min: &str) -> bool {
    let (mut a, mut b) = (version_numbers(v), version_numbers(min));
    if a.is_empty() {
        return true;
    }
    let n = a.len().max(b.len());
    a.resize(n, 0);
    b.resize(n, 0);
    a >= b
}

/// Сведения о сервере по снимку и типу из профиля.
pub fn analyze(configured: ServerKind, p: &Probe) -> AppResult<ServerFacts> {
    let models = parse_models(&p.models)?;
    let detected = detect(p, &models);
    let mut warnings = Vec::new();
    let mut kind = detected;
    if let Some(chosen) = DetectedKind::from_configured(configured) {
        if chosen != detected {
            warnings.push(ltext!(
                "messages.servers.kindMismatch",
                "The server looks like {{detected}}, but {{configured}} is selected in the installation",
                detected = detected.label(),
                configured = chosen.label()
            ));
        }
        kind = chosen;
    }
    let version = version_of(kind, p);
    if let (Some(v), Some(min)) = (&version, kind.min_version())
        && !version_at_least(v, min)
    {
        warnings.push(ltext!(
            "messages.servers.unsupportedVersion",
            "{{kind}} {{version}} is older than {{min}}: Glafira works with it as with an OpenAI-compatible server",
            kind = kind.label(),
            version = v,
            min = min
        ));
        kind = DetectedKind::Openai;
    }
    let metrics = match kind {
        DetectedKind::Vllm => has_series(&p.metrics, "vllm:"),
        DetectedKind::Sglang => has_series(&p.metrics, "sglang:"),
        DetectedKind::Openai => false,
    };
    if !metrics && kind == DetectedKind::Sglang {
        warnings.push(ltext!(
            "messages.servers.noMetricsSglang",
            "SGLang does not expose metrics: start it with --enable-metrics"
        ));
    } else if !metrics && kind == DetectedKind::Vllm {
        warnings.push(ltext!(
            "messages.servers.noMetrics",
            "The server does not expose its metrics at the metrics path of the installation"
        ));
    }
    Ok(ServerFacts {
        kind,
        detected,
        version,
        models,
        metrics,
        warnings,
    })
}

/// Снимки ответов настоящих серверов для тестов (`servers/testdata/<сервер>-<версия>/`).
#[cfg(test)]
pub mod fixtures {
    use super::{Answer, Probe};

    fn ok(body: &str) -> Option<Answer> {
        Some(Answer {
            status: 200,
            body: body.as_bytes().to_vec(),
        })
    }

    fn missing() -> Option<Answer> {
        Some(Answer {
            status: 404,
            body: br#"{"detail":"Not Found"}"#.to_vec(),
        })
    }

    pub fn vllm_0_11() -> Probe {
        Probe {
            version: ok(include_str!("testdata/vllm-0.11.0/version.json")),
            model_info: missing(),
            server_info: missing(),
            metrics: Some(include_str!("testdata/vllm-0.11.0/metrics.txt").into()),
            models: include_bytes!("testdata/vllm-0.11.0/models.json").to_vec(),
        }
    }

    pub fn vllm_0_8() -> Probe {
        Probe {
            version: ok(include_str!("testdata/vllm-0.8.5/version.json")),
            model_info: missing(),
            server_info: missing(),
            metrics: Some(include_str!("testdata/vllm-0.8.5/metrics.txt").into()),
            models: include_bytes!("testdata/vllm-0.8.5/models.json").to_vec(),
        }
    }

    pub fn vllm_0_7() -> Probe {
        Probe {
            version: ok(include_str!("testdata/vllm-0.7.3/version.json")),
            model_info: missing(),
            server_info: missing(),
            metrics: Some(include_str!("testdata/vllm-0.7.3/metrics.txt").into()),
            models: include_bytes!("testdata/vllm-0.7.3/models.json").to_vec(),
        }
    }

    pub fn sglang_0_5() -> Probe {
        Probe {
            version: missing(),
            model_info: ok(include_str!("testdata/sglang-0.5.4/model_info.json")),
            server_info: ok(include_str!("testdata/sglang-0.5.4/server_info.json")),
            metrics: Some(include_str!("testdata/sglang-0.5.4/metrics.txt").into()),
            models: include_bytes!("testdata/sglang-0.5.4/models.json").to_vec(),
        }
    }

    /// SGLang 0.4 без `--enable-metrics`: `/metrics` нет.
    pub fn sglang_0_4() -> Probe {
        Probe {
            version: missing(),
            model_info: ok(include_str!("testdata/sglang-0.4.6/model_info.json")),
            server_info: ok(include_str!("testdata/sglang-0.4.6/server_info.json")),
            metrics: None,
            models: include_bytes!("testdata/sglang-0.4.6/models.json").to_vec(),
        }
    }

    pub fn llamacpp() -> Probe {
        Probe {
            version: missing(),
            model_info: missing(),
            server_info: missing(),
            metrics: None,
            models: include_bytes!("testdata/llamacpp/models.json").to_vec(),
        }
    }

    pub fn ollama() -> Probe {
        Probe {
            version: missing(),
            model_info: missing(),
            server_info: missing(),
            metrics: None,
            models: include_bytes!("testdata/ollama/models.json").to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    fn keys(f: &ServerFacts) -> Vec<&str> {
        f.warnings.iter().map(|w| w.key.as_str()).collect()
    }

    #[test]
    fn snapshots_of_real_servers() {
        for (name, probe, kind, version, metrics) in [
            (
                "vllm 0.11",
                vllm_0_11(),
                DetectedKind::Vllm,
                Some("0.11.0"),
                true,
            ),
            (
                "vllm 0.8",
                vllm_0_8(),
                DetectedKind::Vllm,
                Some("0.8.5.post1"),
                true,
            ),
            (
                "sglang 0.5",
                sglang_0_5(),
                DetectedKind::Sglang,
                Some("0.5.4"),
                true,
            ),
            (
                "sglang 0.4",
                sglang_0_4(),
                DetectedKind::Sglang,
                Some("0.4.6.post1"),
                false,
            ),
            ("llama.cpp", llamacpp(), DetectedKind::Openai, None, false),
            ("ollama", ollama(), DetectedKind::Openai, None, false),
        ] {
            let f = analyze(ServerKind::Auto, &probe).unwrap();
            assert_eq!(
                (f.kind, f.detected, f.version.as_deref(), f.metrics),
                (kind, kind, version, metrics),
                "{name}"
            );
        }
    }

    #[test]
    fn models_with_lora_adapters_and_context_length() {
        let f = analyze(ServerKind::Auto, &vllm_0_8()).unwrap();
        assert_eq!(f.models.len(), 2);
        assert_eq!(f.models[0].max_model_len, Some(131072));
        assert_eq!(
            f.models[0].root.as_deref(),
            Some("/models/Meta-Llama-3.1-8B-Instruct")
        );
        assert_eq!(f.models[1].parent.as_deref(), Some("llama-3.1-8b"));
        assert_eq!(f.models[1].max_model_len, None);
        let f = analyze(ServerKind::Auto, &ollama()).unwrap();
        assert_eq!(
            f.models.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            ["llama3.1:8b", "qwen3:32b"]
        );
        let json = serde_json::to_value(analyze(ServerKind::Auto, &sglang_0_5()).unwrap()).unwrap();
        assert_eq!(json["kind"], "sglang");
        assert_eq!(json["models"][0]["maxModelLen"], 40960);
        assert!(json["models"][0].get("parent").is_none());
    }

    #[test]
    fn server_info_secrets_never_reach_the_facts() {
        let f = analyze(ServerKind::Auto, &sglang_0_5()).unwrap();
        let json = serde_json::to_string(&f).unwrap();
        assert!(!json.contains("sk-sglang-s3cr3t"), "{json}");
    }

    #[test]
    fn old_version_is_worked_with_as_openai_compatible() {
        let f = analyze(ServerKind::Auto, &vllm_0_7()).unwrap();
        assert_eq!(
            (f.detected, f.kind),
            (DetectedKind::Vllm, DetectedKind::Openai)
        );
        assert_eq!(keys(&f), ["messages.servers.unsupportedVersion"]);
        assert_eq!(f.warnings[0].params["min"], "0.8");
        assert!(!f.metrics, "метрики openai не читаются");
    }

    #[test]
    fn sglang_without_metrics_is_told_how_to_enable_them() {
        let f = analyze(ServerKind::Auto, &sglang_0_4()).unwrap();
        assert_eq!(keys(&f), ["messages.servers.noMetricsSglang"]);
        let mut p = vllm_0_11();
        p.metrics = None;
        let f = analyze(ServerKind::Auto, &p).unwrap();
        assert_eq!(keys(&f), ["messages.servers.noMetrics"]);
    }

    #[test]
    fn configured_kind_wins_with_a_warning_on_mismatch() {
        let f = analyze(ServerKind::Sglang, &vllm_0_11()).unwrap();
        assert_eq!(
            (f.kind, f.detected),
            (DetectedKind::Sglang, DetectedKind::Vllm)
        );
        assert_eq!(f.warnings[0].key, "messages.servers.kindMismatch");
        assert_eq!(f.warnings[0].params["detected"], "vLLM");
        let f = analyze(ServerKind::Vllm, &vllm_0_11()).unwrap();
        assert!(f.warnings.is_empty());
        let f = analyze(ServerKind::Openai, &vllm_0_11()).unwrap();
        assert_eq!((f.kind, f.version.as_deref()), (DetectedKind::Openai, None));
    }

    #[test]
    fn signals_each_point_to_a_kind() {
        // Обратный прокси вырезал `owned_by`, метрик нет: решает `/version`.
        let mut p = vllm_0_11();
        p.metrics = None;
        p.models = br#"{"data":[{"id":"m"}]}"#.to_vec();
        assert_eq!(
            detect(&p, &parse_models(&p.models).unwrap()),
            DetectedKind::Vllm
        );
        // HTML-заглушка прокси на любом пути — не признак SGLang.
        let mut p = llamacpp();
        p.model_info = Some(Answer {
            status: 200,
            body: b"<html>ok</html>".to_vec(),
        });
        p.version = p.model_info.clone();
        assert_eq!(
            detect(&p, &parse_models(&p.models).unwrap()),
            DetectedKind::Openai
        );
        // `/version` с JSON, но модели принадлежат llama.cpp — не vLLM.
        let mut p = llamacpp();
        p.version = Some(Answer {
            status: 200,
            body: br#"{"version":"b4500"}"#.to_vec(),
        });
        assert_eq!(
            detect(&p, &parse_models(&p.models).unwrap()),
            DetectedKind::Openai
        );
    }

    #[test]
    fn models_must_be_a_list() {
        let mut p = llamacpp();
        p.models = b"<html>".to_vec();
        let e = analyze(ServerKind::Auto, &p).unwrap_err();
        assert_eq!(e.key, "errors.server.badJson");
        p.models = br#"{"object":"list","data":[]}"#.to_vec();
        assert!(analyze(ServerKind::Auto, &p).unwrap().models.is_empty());
    }

    #[test]
    fn versions() {
        assert_eq!(version_numbers("0.10.1.1"), [0, 10, 1]);
        assert_eq!(version_numbers("0.6.3.post1"), [0, 6, 3]);
        assert_eq!(version_numbers("0.5.0rc1"), [0, 5, 0]);
        assert_eq!(version_numbers("v0.11.0+cu128"), [0, 11, 0]);
        assert!(version_numbers("main").is_empty());
        for (v, min, ok) in [
            ("0.11.0", "0.8", true),
            ("0.8.0", "0.8", true),
            ("0.7.3", "0.8", false),
            ("0.4.6.post1", "0.4", true),
            ("0.3.6", "0.4", false),
            ("1.0", "0.8", true),
            ("dev", "0.8", true),
        ] {
            assert_eq!(version_at_least(v, min), ok, "{v} >= {min}");
        }
    }
}
