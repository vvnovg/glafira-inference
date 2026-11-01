//! Окна истории (спека I1c §4): длина, шаг, интервал `rate` и границы запроса. Шаг — не меньше
//! 1 мин у окон 1 ч…30 д и не больше 1000 точек на ряд; окно `now` предзаполняет «Сейчас».

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Window {
    #[serde(rename = "now")]
    Now,
    #[serde(rename = "1h")]
    Hour,
    #[serde(rename = "24h")]
    Day,
    #[serde(rename = "7d")]
    Week,
    #[serde(rename = "30d")]
    Month,
}

impl Window {
    /// Длина окна, с.
    pub fn length_sec(self) -> u64 {
        match self {
            Self::Now => 15 * 60,
            Self::Hour => 3_600,
            Self::Day => 86_400,
            Self::Week => 7 * 86_400,
            Self::Month => 30 * 86_400,
        }
    }

    /// Шаг точек, с.
    pub fn step_sec(self) -> u64 {
        match self {
            Self::Now => 15,
            Self::Hour => 60,
            Self::Day => 90,
            Self::Week => 12 * 60,
            Self::Month => 45 * 60,
        }
    }
}

/// Границы запроса `query_range` и интервал `rate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    /// Начало и конец, с Unix; конец — `now`, округлённый вниз до шага.
    pub start: u64,
    pub end: u64,
    pub step: u64,
    /// Интервал `rate`, с: не меньше шага, минуты и четырёх интервалов опроса цели.
    pub rate: u64,
}

/// Окно, которое кончается сейчас. `scrape_sec` — интервал опроса цели источником (`None` —
/// неизвестен). Конец округляется до шага: повторные запросы окна попадают в кеш источника.
pub fn range(window: Window, now_sec: u64, scrape_sec: Option<u64>) -> Range {
    let step = window.step_sec();
    let end = now_sec - now_sec % step;
    Range {
        start: end.saturating_sub(window.length_sec()),
        end,
        step,
        rate: step.max(60).max(4 * scrape_sec.unwrap_or(0)),
    }
}

/// Длительность PromQL: `90s`, `1m`, `45m`.
pub fn duration(sec: u64) -> String {
    if sec.is_multiple_of(60) {
        format!("{}m", sec / 60)
    } else {
        format!("{sec}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_keep_points_per_series_within_limits() {
        for (w, points) in [
            (Window::Now, 60),
            (Window::Hour, 60),
            (Window::Day, 960),
            (Window::Week, 840),
            (Window::Month, 960),
        ] {
            assert_eq!(w.length_sec() / w.step_sec(), points, "{w:?}");
            assert!(points <= 1000);
            if w != Window::Now {
                assert!(w.step_sec() >= 60, "{w:?}");
            }
        }
    }

    #[test]
    fn range_is_aligned_to_the_step() {
        let r = range(Window::Day, 1_000_000_055, Some(15));
        assert_eq!(r.end, 999_999_990);
        assert_eq!(r.end - r.start, 86_400);
        assert_eq!((r.step, r.rate), (90, 90));
        // Четыре интервала опроса больше шага; без интервала — не меньше минуты.
        assert_eq!(range(Window::Hour, 3_600, Some(30)).rate, 120);
        assert_eq!(range(Window::Now, 3_600, None).rate, 60);
        assert_eq!(range(Window::Now, 3_600, Some(5)).rate, 60);
        assert_eq!(range(Window::Now, 100, None).start, 0);
    }

    #[test]
    fn window_names_and_durations() {
        let names: Vec<String> = [
            Window::Now,
            Window::Hour,
            Window::Day,
            Window::Week,
            Window::Month,
        ]
        .iter()
        .map(|w| serde_json::to_string(w).unwrap())
        .collect();
        assert_eq!(
            names,
            [r#""now""#, r#""1h""#, r#""24h""#, r#""7d""#, r#""30d""#]
        );
        assert_eq!(
            serde_json::from_str::<Window>(r#""7d""#).unwrap(),
            Window::Week
        );
        assert_eq!(duration(60), "1m");
        assert_eq!(duration(90), "90s");
        assert_eq!(duration(2_700), "45m");
    }
}
