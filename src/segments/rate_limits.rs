use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::segments::{GitCache, Segment};
use crate::types::{Color, RESET};

use serde::{Deserialize, Deserializer};

#[derive(Clone, Copy, Deserialize)]
pub enum Window {
    FiveHour,
    SevenDay,
}

#[derive(Clone, Copy, Deserialize)]
pub enum Style {
    Percent,
    Bar,
    BarPercent,
    Radial,
    RadialPercent,
}

#[derive(Clone, Copy, Deserialize)]
pub enum Fill {
    Used,
    Remaining,
}

#[derive(Clone, Copy, Deserialize)]
pub enum ColorMode {
    Steps,
    Gradient,
}

#[derive(Deserialize)]
pub struct RateLimit {
    pub window: Window,
    pub style: Style,
    pub fill: Fill,
    pub color_mode: ColorMode,
    #[serde(
        default = "default_gradient_midpoint_percentage",
        deserialize_with = "deserialize_gradient_midpoint_percentage"
    )]
    pub gradient_midpoint_percentage: f64,
    pub prefix: String,
    pub low_color: Color,
    pub mid_color: Color,
    pub high_color: Color,
}

const COUNTDOWN_TOKEN: &str = "{t}";
const UNKNOWN_COUNTDOWN: &str = "?";

fn default_gradient_midpoint_percentage() -> f64 {
    50.0
}

fn deserialize_gradient_midpoint_percentage<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = f64::deserialize(deserializer)?;
    if value.is_finite() && value > 0.0 && value < 100.0 {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(
            "gradient_midpoint_percentage must be finite, greater than 0, and less than 100",
        ))
    }
}

impl RateLimit {
    fn resolve_prefix(&self, window_data: &Value) -> String {
        if !self.prefix.contains(COUNTDOWN_TOKEN) {
            return self.prefix.clone();
        }

        let remaining = match self.remaining_units(window_data) {
            Some(value) => format!("{value:.1}"),
            None => UNKNOWN_COUNTDOWN.to_string(),
        };
        self.prefix.replace(COUNTDOWN_TOKEN, &remaining)
    }

    fn remaining_units(&self, window_data: &Value) -> Option<f64> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|duration| duration.as_secs() as i64)?;
        self.remaining_units_at(window_data, now)
    }

    fn remaining_units_at(&self, window_data: &Value, now: i64) -> Option<f64> {
        let unit_secs = match self.window {
            Window::FiveHour => 3600.0,
            Window::SevenDay => 86400.0,
        };
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

        let prefix = self.resolve_prefix(window_data);

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
                let (r, g, b) =
                    gradient_rgb(pct, self.gradient_midpoint_percentage, low, mid, high);
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
    midpoint_percentage: f64,
    low: (u8, u8, u8),
    mid: (u8, u8, u8),
    high: (u8, u8, u8),
) -> (u8, u8, u8) {
    let p = pct.clamp(0.0, 100.0);
    let (a, b, t) = if p <= midpoint_percentage {
        (low, mid, p / midpoint_percentage)
    } else {
        (
            mid,
            high,
            (p - midpoint_percentage) / (100.0 - midpoint_percentage),
        )
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
    use ron::extensions::Extensions;
    use serde_json::json;

    use super::{gradient_rgb, ColorMode, Fill, RateLimit, Style, Window};
    use crate::types::Color;

    fn rate_limit(window: Window) -> RateLimit {
        RateLimit {
            window,
            style: Style::Bar,
            fill: Fill::Remaining,
            color_mode: ColorMode::Gradient,
            gradient_midpoint_percentage: 50.0,
            prefix: "{t}h ".to_string(),
            low_color: Color::Rgb(0, 0, 0),
            mid_color: Color::Rgb(100, 100, 100),
            high_color: Color::Rgb(200, 200, 200),
        }
    }

    fn parse_rate_limit(body: &str) -> Result<RateLimit, ron::error::SpannedError> {
        ron::Options::default()
            .with_default_extension(Extensions::UNWRAP_VARIANT_NEWTYPES)
            .from_str(body)
    }

    #[test]
    fn marks_missing_reset_time_as_unknown() {
        let rate_limit = rate_limit(Window::FiveHour);
        let window_data = json!({});

        assert_eq!(rate_limit.remaining_units_at(&window_data, 0), None);
        assert_eq!(rate_limit.resolve_prefix(&window_data), "?h ");
    }

    #[test]
    fn calculates_remaining_units_from_reset_time() {
        let rate_limit = rate_limit(Window::FiveHour);
        let window_data = json!({ "resets_at": 9_000 });

        assert_eq!(rate_limit.remaining_units_at(&window_data, 0), Some(2.5));
    }

    #[test]
    fn places_mid_color_at_configured_gradient_midpoint() {
        let low = (0, 0, 0);
        let mid = (100, 100, 100);
        let high = (200, 200, 200);

        assert_eq!(gradient_rgb(25.0, 25.0, low, mid, high), mid);
        assert_eq!(gradient_rgb(62.5, 25.0, low, mid, high), (150, 150, 150));
    }

    #[test]
    fn default_gradient_midpoint_preserves_existing_curve() {
        let low = (0, 0, 0);
        let mid = (100, 100, 100);
        let high = (200, 200, 200);

        assert_eq!(gradient_rgb(-1.0, 50.0, low, mid, high), low);
        assert_eq!(gradient_rgb(25.0, 50.0, low, mid, high), (50, 50, 50));
        assert_eq!(gradient_rgb(50.0, 50.0, low, mid, high), mid);
        assert_eq!(gradient_rgb(75.0, 50.0, low, mid, high), (150, 150, 150));
        assert_eq!(gradient_rgb(101.0, 50.0, low, mid, high), high);
    }

    #[test]
    fn defaults_gradient_midpoint_for_existing_configs() {
        let rate_limit = parse_rate_limit(
            r#"(
                window: FiveHour,
                style: Bar,
                fill: Remaining,
                color_mode: Gradient,
                prefix: "{t}h ",
                low_color: Rgb(0, 0, 0),
                mid_color: Rgb(100, 100, 100),
                high_color: Rgb(200, 200, 200),
            )"#,
        )
        .expect("legacy RateLimit config should parse");

        assert_eq!(rate_limit.gradient_midpoint_percentage, 50.0);
    }

    #[test]
    fn rejects_invalid_gradient_midpoint() {
        let body = r#"(
            window: SevenDay,
            style: Percent,
            fill: Used,
            color_mode: Gradient,
            gradient_midpoint_percentage: 100.0,
            prefix: "{t}d ",
            low_color: Rgb(0, 0, 0),
            mid_color: Rgb(100, 100, 100),
            high_color: Rgb(200, 200, 200),
        )"#;

        assert!(parse_rate_limit(body).is_err());
    }
}
