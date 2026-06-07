use serde::Deserialize;
use serde_json::Value;

use crate::segments::{GitCache, Segment};
use crate::types::{Color, RESET};

#[derive(Deserialize)]
pub struct Context {
    pub color: Color,
    pub prefix: String,
    pub prefix_color: Color,
    pub suffix: String,
    pub suffix_color: Color,
}

impl Segment for Context {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let p = json
            .get("context_window")?
            .get("used_percentage")?
            .as_f64()?;

        let pct = format!("{}%", p.round() as i64);

        let painted_pct = match self.color {
            Color::Gradient => {
                let (r, g, b) = gradient_rgb(p);
                format!("\x1b[38;2;{};{};{}m{}{}", r, g, b, pct, RESET)
            }
            _ => self.color.paint(&pct),
        };

        let mut out = String::new();

        if !self.prefix.is_empty() {
            out.push_str(&self.prefix_color.paint(&self.prefix));
        }

        out.push_str(&painted_pct);

        if !self.suffix.is_empty() {
            out.push_str(&self.suffix_color.paint(&self.suffix));
        }

        Some(out)
    }
}

fn gradient_rgb(p: f64) -> (u8, u8, u8) {
    let lerp = |a: i32, b: i32, t: f64| (a as f64 + (b - a) as f64 * t.clamp(0.0, 1.0)) as u8;
    if p <= 20.0 {
        let t = p / 20.0;
        (lerp(150, 180, t), lerp(150, 165, t), lerp(150, 100, t))
    } else {
        let t = (p - 20.0) / 10.0;
        (lerp(180, 220, t), lerp(165, 60, t), lerp(100, 60, t))
    }
}
