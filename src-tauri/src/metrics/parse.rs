//! Разбор текстового формата Prometheus 0.0.4 и OpenMetrics: имя, метки, значение.
//! `# HELP`, `# TYPE`, `# EOF` и серии `*_created` пропускаются; метка времени после значения
//! не нужна. Нераспознанная строка пропускается и считается: формат могут расширить, а одна
//! странная строка не должна лишать экрана всех метрик.

use crate::error::{AppResult, ErrorCode};

/// Строка длиннее — ошибка: так выглядит не `/metrics`, а что-то другое.
pub const MAX_LINE: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub name: String,
    pub labels: Vec<(String, String)>,
    pub value: f64,
}

impl Sample {
    pub fn label(&self, name: &str) -> Option<&str> {
        self.labels
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct Parsed {
    pub samples: Vec<Sample>,
    /// Пропущенные нераспознанные строки.
    pub skipped: usize,
}

pub fn too_large() -> crate::error::AppError {
    app_err!(
        ErrorCode::InvalidInput,
        "errors.metrics.tooLarge",
        "The metrics response is too large or is not in the Prometheus format"
    )
}

pub fn parse(text: &str) -> AppResult<Parsed> {
    let mut out = Parsed::default();
    for line in text.lines() {
        if line.len() > MAX_LINE {
            return Err(too_large());
        }
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match sample(line) {
            Some(s) if s.name.ends_with("_created") => {}
            Some(s) => out.samples.push(s),
            None => out.skipped += 1,
        }
    }
    Ok(out)
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == ':'
}

fn sample(line: &str) -> Option<Sample> {
    let name_end = line.find(|c: char| !is_name_char(c)).unwrap_or(line.len());
    let name = &line[..name_end];
    if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let mut rest = &line[name_end..];
    let mut labels = Vec::new();
    if let Some(r) = rest.strip_prefix('{') {
        let (l, after) = parse_labels(r)?;
        labels = l;
        rest = after;
    }
    let value = rest.split_whitespace().next()?;
    Some(Sample {
        name: name.to_string(),
        labels,
        value: parse_value(value)?,
    })
}

/// Метки до закрывающей `}`; в значениях — экранирование `\\`, `\"`, `\n`.
fn parse_labels(mut s: &str) -> Option<(Vec<(String, String)>, &str)> {
    let mut labels = Vec::new();
    loop {
        s = s.trim_start_matches([' ', ',']);
        if let Some(rest) = s.strip_prefix('}') {
            return Some((labels, rest));
        }
        let eq = s.find('=')?;
        let key = s[..eq].trim();
        if key.is_empty() || !key.chars().all(is_name_char) {
            return None;
        }
        let body = s[eq + 1..].trim_start().strip_prefix('"')?;
        let mut chars = body.char_indices();
        let mut value = String::new();
        let end = loop {
            let (i, c) = chars.next()?;
            match c {
                '"' => break i,
                '\\' => match chars.next()?.1 {
                    'n' => value.push('\n'),
                    other => value.push(other),
                },
                c => value.push(c),
            }
        };
        labels.push((key.to_string(), value));
        s = &body[end + 1..];
    }
}

fn parse_value(v: &str) -> Option<f64> {
    match v {
        "NaN" => Some(f64::NAN),
        "+Inf" | "Inf" => Some(f64::INFINITY),
        "-Inf" => Some(f64::NEG_INFINITY),
        _ => v.parse().ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(line: &str) -> Sample {
        let p = parse(line).unwrap();
        assert_eq!(p.skipped, 0, "{line}");
        p.samples.into_iter().next().unwrap()
    }

    #[test]
    fn names_labels_and_values() {
        let s = one(r#"vllm:num_requests_running{engine="0",model_name="qwen"} 3.0"#);
        assert_eq!(s.name, "vllm:num_requests_running");
        assert_eq!(s.label("engine"), Some("0"));
        assert_eq!(s.label("model_name"), Some("qwen"));
        assert_eq!(s.value, 3.0);
        assert_eq!(one("process_open_fds 12").labels, []);
        assert_eq!(one("x{} 1e-3 1700000000000").value, 0.001);
    }

    #[test]
    fn escaped_label_values() {
        let s = one(r#"x{a="say \"hi\"",b="c:\\dir",c="two\nlines",d="{,}"} 1"#);
        assert_eq!(s.label("a"), Some(r#"say "hi""#));
        assert_eq!(s.label("b"), Some(r"c:\dir"));
        assert_eq!(s.label("c"), Some("two\nlines"));
        assert_eq!(s.label("d"), Some("{,}"));
    }

    #[test]
    fn special_values() {
        assert!(one("x NaN").value.is_nan());
        assert_eq!(one(r#"h_bucket{le="+Inf"} +Inf"#).value, f64::INFINITY);
        assert_eq!(one("x -Inf").value, f64::NEG_INFINITY);
        assert_eq!(one(r#"h_bucket{le="+Inf"} 5"#).label("le"), Some("+Inf"));
    }

    #[test]
    fn comments_created_series_and_openmetrics_eof_are_skipped() {
        let p = parse(
            "# HELP x_total help\n# TYPE x_total counter\nx_total 5\nx_created 1.7e9\n\n# EOF\n",
        )
        .unwrap();
        assert_eq!(p.samples.len(), 1);
        assert_eq!(p.samples[0].name, "x_total");
    }

    #[test]
    fn malformed_lines_are_skipped_and_counted() {
        let p =
            parse("ok 1\n{a=\"b\"} 2\n9bad 3\nx{a=\"unterminated} 4\nnovalue\nx abc\n").unwrap();
        assert_eq!(p.samples.len(), 1);
        assert_eq!(p.skipped, 5);
    }

    #[test]
    fn overlong_line_is_an_error() {
        let line = format!("x{{a=\"{}\"}} 1", "y".repeat(MAX_LINE));
        assert_eq!(parse(&line).unwrap_err().key, "errors.metrics.tooLarge");
    }

    #[test]
    fn real_vllm_snapshot() {
        let p = parse(include_str!("../servers/testdata/vllm-0.11.0/metrics.txt")).unwrap();
        assert_eq!(p.skipped, 0);
        assert!(
            p.samples
                .iter()
                .any(|s| s.name == "vllm:kv_cache_usage_perc")
        );
        assert!(!p.samples.iter().any(|s| s.name.ends_with("_created")));
    }
}
