//! Цели инференса в источнике истории и сопоставление их с серверами (спека I1c §3): разбор и
//! сборка селектора серий, автоподбор цели по `host:port` адреса сервера.

use serde::Serialize;

use crate::error::{AppError, AppResult, ErrorCode};

/// Тип цели — по префиксу её серий (`vllm:`, `sglang:`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetKind {
    Vllm,
    Sglang,
}

/// Цель инференса, найденная в источнике: пара `job`, `instance`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub job: String,
    pub instance: String,
    pub kind: TargetKind,
}

impl Target {
    /// Селектор серий этой цели: `{job="…", instance="…"}`.
    pub fn selector(&self) -> String {
        format!(
            "{{job=\"{}\", instance=\"{}\"}}",
            escape(&self.job),
            escape(&self.instance)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchOp {
    Eq,
    Ne,
    Re,
    Nre,
}

impl MatchOp {
    fn as_str(self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Ne => "!=",
            Self::Re => "=~",
            Self::Nre => "!~",
        }
    }
}

/// Условие на метку: `job="vllm"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Matcher {
    pub label: String,
    pub op: MatchOp,
    pub value: String,
}

/// Значение метки в двойных кавычках PromQL: `\` и `"` экранируются, перевод строки — `\n`.
pub fn escape(v: &str) -> String {
    v.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Условия селектора без фигурных скобок, для подстановки в шаблон: `job="vllm",instance="h:8000"`.
pub fn render(matchers: &[Matcher]) -> String {
    matchers
        .iter()
        .map(|m| format!("{}{}\"{}\"", m.label, m.op.as_str(), escape(&m.value)))
        .collect::<Vec<_>>()
        .join(",")
}

fn bad_selector(pos: usize) -> AppError {
    app_err!(
        ErrorCode::InvalidInput,
        "errors.history.selector",
        "The series selector is invalid at character {{pos}}: use {job=\"vllm\", instance=\"host:port\"}",
        pos = pos + 1
    )
}

/// Строгий разбор селектора (спека I1c §3): `{` условия через запятую `}`, операции `=`, `!=`,
/// `=~`, `!~`, значения в двойных кавычках (`\"`, `\\`, `\n`), хотя бы одно условие, без
/// `__name__` — имя серии подставляет шаблон. Ошибка называет позицию (с 1) в символах.
pub fn parse_selector(text: &str) -> AppResult<Vec<Matcher>> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let skip_ws = |i: &mut usize| {
        while *i < chars.len() && chars[*i].is_whitespace() {
            *i += 1;
        }
    };
    skip_ws(&mut i);
    if chars.get(i) != Some(&'{') {
        return Err(bad_selector(i));
    }
    i += 1;
    let mut out = Vec::new();
    loop {
        skip_ws(&mut i);
        if chars.get(i) == Some(&'}') && !out.is_empty() {
            i += 1;
            break;
        }
        let start = i;
        while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
            i += 1;
        }
        let label: String = chars[start..i].iter().collect();
        if label.is_empty() || label.starts_with(|c: char| c.is_ascii_digit()) {
            return Err(bad_selector(start));
        }
        if label == "__name__" {
            return Err(app_err!(
                ErrorCode::InvalidInput,
                "errors.history.selectorName",
                "The series selector must not set __name__: Glafira adds the series names itself"
            ));
        }
        skip_ws(&mut i);
        let op = match (chars.get(i), chars.get(i + 1)) {
            (Some('='), Some('~')) => MatchOp::Re,
            (Some('!'), Some('=')) => MatchOp::Ne,
            (Some('!'), Some('~')) => MatchOp::Nre,
            (Some('='), _) => MatchOp::Eq,
            _ => return Err(bad_selector(i)),
        };
        i += op.as_str().len();
        skip_ws(&mut i);
        if chars.get(i) != Some(&'"') {
            return Err(bad_selector(i));
        }
        i += 1;
        let mut value = String::new();
        loop {
            match chars.get(i) {
                None => return Err(bad_selector(i)),
                Some('"') => break,
                Some('\\') => {
                    match chars.get(i + 1) {
                        Some('"') => value.push('"'),
                        Some('\\') => value.push('\\'),
                        Some('n') => value.push('\n'),
                        _ => return Err(bad_selector(i)),
                    }
                    i += 2;
                }
                Some(&c) => {
                    value.push(c);
                    i += 1;
                }
            }
        }
        i += 1;
        out.push(Matcher { label, op, value });
        skip_ws(&mut i);
        match chars.get(i) {
            Some(',') => i += 1,
            Some('}') => {
                i += 1;
                break;
            }
            _ => return Err(bad_selector(i)),
        }
    }
    skip_ws(&mut i);
    if i != chars.len() {
        return Err(bad_selector(i));
    }
    Ok(out)
}

/// Хост и порт для сравнения: имя без регистра, `localhost`, `127.0.0.1` и `::1` — один адрес.
fn endpoint(host: &str, port: u16) -> (String, u16) {
    let host = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_lowercase();
    let host = match host.as_str() {
        "localhost" | "127.0.0.1" | "::1" => "loopback".to_string(),
        _ => host,
    };
    (host, port)
}

/// `instance` цели как хост и порт; без порта — `default_port` (порт по умолчанию схемы).
fn instance_endpoint(instance: &str, default_port: u16) -> Option<(String, u16)> {
    let (host, port) = if let Some(rest) = instance.strip_prefix('[') {
        let (host, tail) = rest.split_once(']')?;
        match tail.strip_prefix(':') {
            Some(p) => (host, p.parse().ok()?),
            None if tail.is_empty() => (host, default_port),
            None => return None,
        }
    } else {
        match instance.rsplit_once(':') {
            Some((h, p)) if !h.contains(':') => (h, p.parse().ok()?),
            Some(_) => (instance, default_port),
            None => (instance, default_port),
        }
    };
    Some(endpoint(host, port))
}

/// Совпадает ли `instance` цели с хостом и портом адреса сервера (схема и путь не учитываются).
pub fn instance_matches(instance: &str, base: &url::Url) -> bool {
    let (Some(host), Some(port)) = (base.host_str(), base.port_or_known_default()) else {
        return false;
    };
    let scheme_port = if base.scheme() == "https" { 443 } else { 80 };
    instance_endpoint(instance, scheme_port).is_some_and(|e| e == endpoint(host, port))
}

/// Автоподбор (спека I1c §3): цели, у которых `instance` — хост и порт адреса сервера, по
/// порядку `job`, затем `instance`; первая — подобранная.
pub fn matching_targets<'a>(base: &url::Url, targets: &'a [Target]) -> Vec<&'a Target> {
    let mut found: Vec<&Target> = targets
        .iter()
        .filter(|t| instance_matches(&t.instance, base))
        .collect();
    found.sort();
    found
}

/// Подпись селектора для отчёта: ровно `job="…"` и `instance="…"` — `job · instance`, как у
/// цели в форме; любой другой — сам селектор.
pub fn selector_label(selector: &str) -> String {
    match parse_selector(selector).as_deref() {
        Ok([job, instance])
            if job.label == "job"
                && job.op == MatchOp::Eq
                && instance.label == "instance"
                && instance.op == MatchOp::Eq =>
        {
            format!("{} · {}", job.value, instance.value)
        }
        _ => selector.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selector_of_a_target_is_shown_as_its_label() {
        assert_eq!(
            selector_label(r#"{job="vllm", instance="127.0.0.1:8000"}"#),
            "vllm · 127.0.0.1:8000"
        );
        assert_eq!(
            selector_label(r#"{ job = "vllm" , instance="h:1" }"#),
            "vllm · h:1"
        );
        for other in [
            r#"{job="nothing"}"#,
            r#"{instance="h:1", job="vllm"}"#,
            r#"{job="vllm", instance=~"h:.*"}"#,
            r#"{job="vllm", instance="h:1", pod="p"}"#,
            "job=x",
        ] {
            assert_eq!(selector_label(other), other);
        }
    }

    fn m(label: &str, op: MatchOp, value: &str) -> Matcher {
        Matcher {
            label: label.into(),
            op,
            value: value.into(),
        }
    }

    #[test]
    fn selector_is_parsed_strictly() {
        assert_eq!(
            parse_selector(r#" { job="vllm", instance = "10.0.0.5:8000" , } "#).unwrap(),
            [
                m("job", MatchOp::Eq, "vllm"),
                m("instance", MatchOp::Eq, "10.0.0.5:8000")
            ]
        );
        assert_eq!(
            parse_selector(r#"{pod=~"vllm-.*",namespace!="dev",zone!~"b|c"}"#).unwrap(),
            [
                m("pod", MatchOp::Re, "vllm-.*"),
                m("namespace", MatchOp::Ne, "dev"),
                m("zone", MatchOp::Nre, "b|c"),
            ]
        );
        assert_eq!(
            parse_selector(r#"{a="q\"x\\y\nz"}"#).unwrap(),
            [m("a", MatchOp::Eq, "q\"x\\y\nz")]
        );
        for (bad, pos) in [
            ("", 1),
            ("job=\"x\"", 1),
            ("{}", 2),
            ("{job}", 5),
            ("{job=x}", 6),
            ("{job='x'}", 6),
            ("{job=\"x\"", 9),
            ("{job=\"x\"} extra", 11),
            ("{1job=\"x\"}", 2),
            ("{job=\"x\\t\"}", 8),
            ("{job=\"x\" instance=\"y\"}", 10),
        ] {
            let e = parse_selector(bad).unwrap_err();
            assert_eq!(e.key, "errors.history.selector", "{bad}");
            assert_eq!(e.params["pos"], pos.to_string(), "{bad}");
        }
        let e = parse_selector(r#"{__name__="up"}"#).unwrap_err();
        assert_eq!(e.key, "errors.history.selectorName");
    }

    #[test]
    fn selector_round_trips_with_escaping() {
        let parsed = parse_selector(r#"{job="v\"llm", instance=~"a\\b"}"#).unwrap();
        assert_eq!(render(&parsed), r#"job="v\"llm",instance=~"a\\b""#);
        let t = Target {
            job: "vllm".into(),
            instance: "10.0.0.5:8000".into(),
            kind: TargetKind::Vllm,
        };
        assert_eq!(t.selector(), r#"{job="vllm", instance="10.0.0.5:8000"}"#);
        assert_eq!(parse_selector(&t.selector()).unwrap().len(), 2);
    }

    fn target(job: &str, instance: &str) -> Target {
        Target {
            job: job.into(),
            instance: instance.into(),
            kind: TargetKind::Vllm,
        }
    }

    fn url(s: &str) -> url::Url {
        url::Url::parse(s).unwrap()
    }

    #[test]
    fn instance_matches_host_and_port() {
        let base = url("http://localhost:8000/v1");
        for ok in [
            "127.0.0.1:8000",
            "localhost:8000",
            "[::1]:8000",
            "LOCALHOST:8000",
        ] {
            assert!(instance_matches(ok, &base), "{ok}");
        }
        for no in ["127.0.0.1:8001", "10.0.0.5:8000", "localhost", "x:y"] {
            assert!(!instance_matches(no, &base), "{no}");
        }
        // Порт по умолчанию — из схемы адреса сервера.
        assert!(instance_matches("vllm.ai", &url("https://vllm.ai")));
        assert!(instance_matches("vllm.ai:443", &url("https://vllm.ai")));
        assert!(!instance_matches("vllm.ai:80", &url("https://vllm.ai")));
        assert!(instance_matches(
            "[fd00::5]:8000",
            &url("http://[fd00::5]:8000")
        ));
    }

    #[test]
    fn matching_targets_are_ordered_by_job() {
        let targets = [
            target("vllm", "10.0.0.5:8000"),
            target("inference", "10.0.0.5:8000"),
            target("vllm", "10.0.0.6:8000"),
        ];
        let found = matching_targets(&url("http://10.0.0.5:8000"), &targets);
        assert_eq!(
            found.iter().map(|t| t.job.as_str()).collect::<Vec<_>>(),
            ["inference", "vllm"]
        );
        assert!(matching_targets(&url("http://svc:8000"), &targets).is_empty());
    }
}
