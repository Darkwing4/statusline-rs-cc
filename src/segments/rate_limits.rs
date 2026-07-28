use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

pub use crate::config_schema::{ColorMode, Fill, RateLimit, Style, Window};
use crate::segments::{GitCache, Segment};
use crate::types::{Color, RESET};

const COUNTDOWN_TOKEN: &str = "{t}";

impl RateLimit {
    fn resolve_prefix(&self, window_data: &Value, now: Option<i64>) -> String {
        if !self.prefix.contains(COUNTDOWN_TOKEN) {
            return self.prefix.clone();
        }

        let remaining = self.remaining_units(window_data, now);
        self.prefix
            .replace(COUNTDOWN_TOKEN, &format!("{:.1}", remaining))
    }

    fn remaining_units(&self, window_data: &Value, now: Option<i64>) -> f64 {
        let (unit_secs, nominal) = match self.window {
            Window::FiveHour => (3600.0, 5.0),
            Window::SevenDay => (86400.0, 7.0),
        };

        let resets_at = window_data.get("resets_at").and_then(Value::as_i64);

        match (now, resets_at) {
            (Some(now), Some(resets_at)) => {
                let remaining = resets_at.saturating_sub(now).max(0) as f64;
                remaining / unit_secs
            }
            _ => nominal,
        }
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
                let (r, g, b) = gradient_rgb(pct, low, mid, high);
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

fn gradient_rgb(
    pct: f64,
    low: (u8, u8, u8),
    mid: (u8, u8, u8),
    high: (u8, u8, u8),
) -> (u8, u8, u8) {
    let p = pct.clamp(0.0, 100.0);
    let (a, b, t) = if p <= 50.0 {
        (low, mid, p / 50.0)
    } else {
        (mid, high, (p - 50.0) / 50.0)
    };
    lerp_rgb(a, b, t)
}

fn lerp_rgb(a: (u8, u8, u8), b: (u8, u8, u8), t: f64) -> (u8, u8, u8) {
    let lerp = |x: u8, y: u8, t: f64| {
        (x as f64 + (y as f64 - x as f64) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    (lerp(a.0, b.0, t), lerp(a.1, b.1, t), lerp(a.2, b.2, t))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{bar_glyph, gradient_rgb, radial_glyph, ColorMode, Fill, RateLimit, Style, Window};
    use crate::types::Color;

    fn rate_limit(window: Window, prefix: &str) -> RateLimit {
        RateLimit {
            window,
            style: Style::Percent,
            fill: Fill::Used,
            color_mode: ColorMode::Steps,
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
    fn gradient_rgb_uses_endpoints_and_interpolates_both_halves() {
        let low = (10, 20, 30);
        let mid = (110, 120, 130);
        let high = (210, 220, 230);

        assert_eq!(gradient_rgb(0.0, low, mid, high), low);
        assert_eq!(gradient_rgb(25.0, low, mid, high), (60, 70, 80));
        assert_eq!(gradient_rgb(50.0, low, mid, high), mid);
        assert_eq!(gradient_rgb(75.0, low, mid, high), (160, 170, 180));
        assert_eq!(gradient_rgb(100.0, low, mid, high), high);
    }

    #[test]
    fn gradient_rgb_rounds_and_clamps_percentage() {
        let low = (0, 2, 4);
        let mid = (1, 3, 5);
        let high = (2, 4, 6);

        assert_eq!(gradient_rgb(25.0, low, mid, high), (1, 3, 5));
        assert_eq!(gradient_rgb(75.0, low, mid, high), (2, 4, 6));
        assert_eq!(gradient_rgb(-1.0, low, mid, high), low);
        assert_eq!(gradient_rgb(101.0, low, mid, high), high);
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
    fn remaining_units_falls_back_to_nominal_window_lengths() {
        let data = json!({});

        assert_eq!(
            rate_limit(Window::FiveHour, "").remaining_units(&data, Some(1_000)),
            5.0
        );
        assert_eq!(
            rate_limit(Window::SevenDay, "").remaining_units(&data, Some(1_000)),
            7.0
        );
        assert_eq!(
            rate_limit(Window::FiveHour, "").remaining_units(&json!({"resets_at": 10_000}), None),
            5.0
        );
        assert_eq!(
            rate_limit(Window::SevenDay, "").remaining_units(&json!({"resets_at": 10_000}), None),
            7.0
        );
    }

    #[test]
    fn resolve_prefix_replaces_every_countdown_token() {
        let segment = rate_limit(Window::SevenDay, "{t} days, then {t} days");

        assert_eq!(
            segment.resolve_prefix(&json!({}), Some(1_000)),
            "7.0 days, then 7.0 days"
        );
        assert_eq!(
            segment.resolve_prefix(&json!({"resets_at": "invalid"}), Some(1_000)),
            "7.0 days, then 7.0 days"
        );
    }

    #[test]
    fn remaining_units_calculates_exact_future_countdown_for_each_window() {
        let five_hour = rate_limit(Window::FiveHour, "{t}");
        let seven_day = rate_limit(Window::SevenDay, "{t}");
        let five_hour_data = json!({"resets_at": 6_400});
        let seven_day_data = json!({"resets_at": 130_600});

        assert_eq!(five_hour.remaining_units(&five_hour_data, Some(1_000)), 1.5);
        assert_eq!(
            five_hour.resolve_prefix(&five_hour_data, Some(1_000)),
            "1.5"
        );
        assert_eq!(seven_day.remaining_units(&seven_day_data, Some(1_000)), 1.5);
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
            0.0
        );
        assert_eq!(
            segment.resolve_prefix(&json!({"resets_at": 999}), Some(1_000)),
            "0.0h"
        );
        assert_eq!(
            segment.remaining_units(&json!({"resets_at": i64::MIN}), Some(i64::MAX)),
            0.0
        );
        assert_eq!(
            segment.resolve_prefix(&json!({"resets_at": i64::MIN}), Some(i64::MAX)),
            "0.0h"
        );
    }
}
