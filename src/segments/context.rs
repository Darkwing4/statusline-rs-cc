use serde_json::Value;

pub use crate::config_schema::Context;
use crate::gradient::{gradient, Quantization, Rgb};
use crate::segments::{GitCache, Segment};
use crate::types::{Color, RESET};

const CONTEXT_GRADIENT: &[(f64, Rgb)] = &[
    (0.0, (150, 150, 150)),
    (20.0, (180, 165, 100)),
    (30.0, (220, 60, 60)),
];

impl Segment for Context {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let p = json
            .get("context_window")?
            .get("used_percentage")?
            .as_f64()?;

        let pct = format!("{}%", p.round() as i64);

        let painted_pct = match self.color {
            Color::Gradient => {
                let (r, g, b) = gradient(CONTEXT_GRADIENT, p, Quantization::Truncate);
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

#[cfg(test)]
mod tests {
    use super::{gradient, Quantization, CONTEXT_GRADIENT};

    #[test]
    fn returns_colors_at_gradient_stops() {
        assert_eq!(
            gradient(CONTEXT_GRADIENT, 0.0, Quantization::Truncate),
            (150, 150, 150)
        );
        assert_eq!(
            gradient(CONTEXT_GRADIENT, 20.0, Quantization::Truncate),
            (180, 165, 100)
        );
        assert_eq!(
            gradient(CONTEXT_GRADIENT, 30.0, Quantization::Truncate),
            (220, 60, 60)
        );
    }

    #[test]
    fn interpolates_between_gradient_stops() {
        assert_eq!(
            gradient(CONTEXT_GRADIENT, 10.0, Quantization::Truncate),
            (165, 157, 125)
        );
        assert_eq!(
            gradient(CONTEXT_GRADIENT, 25.0, Quantization::Truncate),
            (200, 112, 80)
        );
    }

    #[test]
    fn clamps_values_outside_gradient_range() {
        assert_eq!(
            gradient(CONTEXT_GRADIENT, -1.0, Quantization::Truncate),
            (150, 150, 150)
        );
        assert_eq!(
            gradient(CONTEXT_GRADIENT, 100.0, Quantization::Truncate),
            (220, 60, 60)
        );
    }
}
