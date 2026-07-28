use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

pub use crate::config_schema::{ColorMode, Fill, RateLimit, Style, Window};
use crate::gradient::{gradient, Quantization};
use crate::segments::{GitCache, Segment};
use crate::types::{Color, RESET};

const COUNTDOWN_TOKEN: &str = "{t}";
const UNKNOWN_COUNTDOWN: &str = "?";

impl RateLimit {
    fn resolve_prefix(&self, window_data: &Value, now: Option<i64>) -> String {
        if !self.prefix.contains(COUNTDOWN_TOKEN) {
            return self.prefix.clone();
        }

        let remaining = match self.remaining_units(window_data, now) {
            Some(value) => format!("{value:.1}"),
            None => UNKNOWN_COUNTDOWN.to_string(),
        };
        self.prefix.replace(COUNTDOWN_TOKEN, &remaining)
    }

    fn remaining_units(&self, window_data: &Value, now: Option<i64>) -> Option<f64> {
        let unit_secs = match self.window {
            Window::FiveHour => 3600.0,
            Window::SevenDay => 86400.0,
        };
        let now = now?;
        let resets_at = window_data.get("resets_at").and_then(Value::as_i64)?;
        let remaining = resets_at.saturating_sub(now).max(0) as f64;
        Some(remaining / unit_secs)
    }
}

const BAR_GLYPHS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
const RADIAL_GLYPHS: [char; 5] = ['○', '◔', '◑', '◕', '●'];

fn bar_glyph(pct: f64) -> char {
    let idx = (pct * 8.0 / 100.0) as isize;
    let clamped = idx.clamp(0, 7) as usize;
    BAR_GLYPHS[clamped]
}

fn radial_glyph(pct: f64) -> char {
    let idx = (pct * 5.0 / 100.0) as isize;
    let clamped = idx.clamp(0, 4) as usize;
    RADIAL_GLYPHS[clamped]
}

impl Segment for RateLimit {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let key = match self.window {
            Window::FiveHour => "five_hour",
            Window::SevenDay => "seven_day",
        };

        let window_data = json.get("rate_limits")?.get(key)?;

        let pct = window_data.get("used_percentage")?.as_f64()?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|duration| i64::try_from(duration.as_secs()).ok());
        let prefix = self.resolve_prefix(window_data, now);

        let rounded = pct.round() as i64;

        let glyph_pct = match self.fill {
            Fill::Used => pct,
            Fill::Remaining => 100.0 - pct,
        };

        let text = match self.style {
            Style::Percent => format!("{}{}%", prefix, rounded),
            Style::Bar => format!("{}{}", prefix, bar_glyph(glyph_pct)),
            Style::BarPercent => {
                format!("{}{} {}%", prefix, bar_glyph(glyph_pct), rounded)
            }
            Style::Radial => format!("{}{}", prefix, radial_glyph(glyph_pct)),
            Style::RadialPercent => {
                format!("{}{} {}%", prefix, radial_glyph(glyph_pct), rounded)
            }
        };

        let painted = match self.color_mode {
            ColorMode::Steps => {
                let color = if pct < 50.0 {
                    self.low_color
                } else if pct <= 80.0 {
                    self.mid_color
                } else {
                    self.high_color
                };
                color.paint(&text)
            }
            ColorMode::Gradient => {
                let low = color_to_rgb(self.low_color, (60, 200, 60));
                let mid = color_to_rgb(self.mid_color, (220, 200, 40));
                let high = color_to_rgb(self.high_color, (220, 60, 60));
                let stops = [
                    (0.0, low),
                    (self.gradient_midpoint_percentage, mid),
                    (100.0, high),
                ];
                let (r, g, b) = gradient(&stops, pct, Quantization::Nearest);
                format!("\x1b[38;2;{};{};{}m{}{}", r, g, b, text, RESET)
            }
        };

        Some(painted)
    }
}

fn color_to_rgb(c: Color, fallback: (u8, u8, u8)) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => fallback,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{bar_glyph, radial_glyph, ColorMode, Fill, RateLimit, Style, Window};
    use crate::types::Color;

    fn rate_limit(window: Window, prefix: &str) -> RateLimit {
        RateLimit {
            window,
            style: Style::Percent,
            fill: Fill::Used,
            color_mode: ColorMode::Steps,
            gradient_midpoint_percentage: 50.0,
            prefix: prefix.to_string(),
            low_color: Color::Named(32),
            mid_color: Color::Named(33),
            high_color: Color::Named(31),
        }
    }

    #[test]
    fn bar_glyph_uses_all_thresholds_and_clamps() {
        let cases = [
            (-1.0, '▁'),
            (0.0, '▁'),
            (12.499, '▁'),
            (12.5, '▂'),
            (25.0, '▃'),
            (37.5, '▄'),
            (50.0, '▅'),
            (62.5, '▆'),
            (75.0, '▇'),
            (87.5, '█'),
            (100.0, '█'),
            (101.0, '█'),
        ];

        for (pct, expected) in cases {
            assert_eq!(bar_glyph(pct), expected, "{pct}");
        }
    }

    #[test]
    fn radial_glyph_uses_all_thresholds_and_clamps() {
        let cases = [
            (-1.0, '○'),
            (0.0, '○'),
            (19.999, '○'),
            (20.0, '◔'),
            (40.0, '◑'),
            (60.0, '◕'),
            (80.0, '●'),
            (100.0, '●'),
            (101.0, '●'),
        ];

        for (pct, expected) in cases {
            assert_eq!(radial_glyph(pct), expected, "{pct}");
        }
    }

    #[test]
    fn resolve_prefix_returns_unchanged_text_without_countdown_token() {
        let segment = rate_limit(Window::FiveHour, "limit ");

        assert_eq!(
            segment.resolve_prefix(&json!({"resets_at": i64::MIN}), Some(i64::MAX)),
            "limit "
        );
    }

    #[test]
    fn remaining_units_returns_none_without_real_reset_data() {
        let data = json!({});

        assert_eq!(
            rate_limit(Window::FiveHour, "").remaining_units(&data, Some(1_000)),
            None
        );
        assert_eq!(
            rate_limit(Window::SevenDay, "").remaining_units(&data, Some(1_000)),
            None
        );
        assert_eq!(
            rate_limit(Window::FiveHour, "").remaining_units(&json!({"resets_at": 10_000}), None),
            None
        );
        assert_eq!(
            rate_limit(Window::SevenDay, "").remaining_units(&json!({"resets_at": 10_000}), None),
            None
        );
    }

    #[test]
    fn resolve_prefix_replaces_every_countdown_token() {
        let segment = rate_limit(Window::SevenDay, "{t} days, then {t} days");

        assert_eq!(
            segment.resolve_prefix(&json!({}), Some(1_000)),
            "? days, then ? days"
        );
        assert_eq!(
            segment.resolve_prefix(&json!({"resets_at": "invalid"}), Some(1_000)),
            "? days, then ? days"
        );
    }

    #[test]
    fn remaining_units_calculates_exact_future_countdown_for_each_window() {
        let five_hour = rate_limit(Window::FiveHour, "{t}");
        let seven_day = rate_limit(Window::SevenDay, "{t}");
        let five_hour_data = json!({"resets_at": 6_400});
        let seven_day_data = json!({"resets_at": 130_600});

        assert_eq!(
            five_hour.remaining_units(&five_hour_data, Some(1_000)),
            Some(1.5)
        );
        assert_eq!(
            five_hour.resolve_prefix(&five_hour_data, Some(1_000)),
            "1.5"
        );
        assert_eq!(
            seven_day.remaining_units(&seven_day_data, Some(1_000)),
            Some(1.5)
        );
        assert_eq!(
            seven_day.resolve_prefix(&seven_day_data, Some(1_000)),
            "1.5"
        );
    }

    #[test]
    fn remaining_units_returns_zero_for_expired_and_extreme_resets() {
        let segment = rate_limit(Window::FiveHour, "{t}h");

        assert_eq!(
            segment.remaining_units(&json!({"resets_at": 999}), Some(1_000)),
            Some(0.0)
        );
        assert_eq!(
            segment.resolve_prefix(&json!({"resets_at": 999}), Some(1_000)),
            "0.0h"
        );
        assert_eq!(
            segment.remaining_units(&json!({"resets_at": i64::MIN}), Some(i64::MAX)),
            Some(0.0)
        );
        assert_eq!(
            segment.resolve_prefix(&json!({"resets_at": i64::MIN}), Some(i64::MAX)),
            "0.0h"
        );
    }
}
