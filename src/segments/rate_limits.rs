use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

pub use crate::config_schema::{ColorMode, Fill, RateLimit, Style, Window};
use crate::segments::{GitCache, Segment};
use crate::types::{Color, RESET};

const COUNTDOWN_TOKEN: &str = "{t}";

impl RateLimit {
    fn resolve_prefix(&self, window_data: &Value) -> String {
        if !self.prefix.contains(COUNTDOWN_TOKEN) {
            return self.prefix.clone();
        }

        let remaining = self.remaining_units(window_data);
        self.prefix
            .replace(COUNTDOWN_TOKEN, &format!("{:.1}", remaining))
    }

    fn remaining_units(&self, window_data: &Value) -> f64 {
        let (unit_secs, nominal) = match self.window {
            Window::FiveHour => (3600.0, 5.0),
            Window::SevenDay => (86400.0, 7.0),
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs() as i64);
        let resets_at = window_data.get("resets_at").and_then(Value::as_i64);

        match (now, resets_at) {
            (Some(now), Some(resets_at)) => {
                let remaining = (resets_at - now).max(0) as f64;
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
        (x as f64 + (y as f64 - x as f64) * t).round().clamp(0.0, 255.0) as u8
    };
    (lerp(a.0, b.0, t), lerp(a.1, b.1, t), lerp(a.2, b.2, t))
}
