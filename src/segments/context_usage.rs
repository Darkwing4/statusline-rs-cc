use serde_json::Value;

use crate::config_schema::Color;
pub use crate::config_schema::ContextUsage;
use crate::gradient::{gradient, Rgb};
use crate::segments::{GitCache, Segment};

const CONTEXT_GRADIENT: &[(f64, Rgb)] = &[
    (0.0, (150, 150, 150)),
    (20.0, (180, 165, 100)),
    (30.0, (220, 60, 60)),
];

impl Segment for ContextUsage {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let p = json
            .get("context_window")?
            .get("used_percentage")?
            .as_f64()?;

        let pct = format!("{}%", p.round() as i64);

        let painted_pct = match self.color {
            Color::Gradient => {
                let (r, g, b) = gradient(CONTEXT_GRADIENT, p);
                Color::Rgb(r, g, b).paint(&pct)
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
    use serde_json::json;

    use super::{gradient, ContextUsage, CONTEXT_GRADIENT};
    use crate::config_schema::Color;
    use crate::segments::{GitCache, Segment};

    fn context(color: Color) -> ContextUsage {
        ContextUsage {
            color,
            prefix: String::new(),
            prefix_color: Color::Named(90),
            suffix: String::new(),
            suffix_color: Color::Named(90),
        }
    }

    #[test]
    fn paints_with_named_color() {
        let json = json!({"context_window": {"used_percentage": 10.4}});
        let mut git = GitCache::new(String::new());

        assert_eq!(
            context(Color::Named(33)).render(&json, &mut git),
            Some("\x1b[33m10%\x1b[0m".to_string())
        );
    }

    #[test]
    fn hides_without_context_window() {
        let mut git = GitCache::new(String::new());

        assert_eq!(context(Color::Gradient).render(&json!({}), &mut git), None);
        assert_eq!(
            context(Color::Gradient).render(&json!({"context_window": {}}), &mut git),
            None
        );
    }

    #[test]
    fn returns_colors_at_gradient_stops() {
        assert_eq!(gradient(CONTEXT_GRADIENT, 0.0), (150, 150, 150));
        assert_eq!(gradient(CONTEXT_GRADIENT, 20.0), (180, 165, 100));
        assert_eq!(gradient(CONTEXT_GRADIENT, 30.0), (220, 60, 60));
    }

    #[test]
    fn interpolates_between_gradient_stops() {
        assert_eq!(gradient(CONTEXT_GRADIENT, 10.0), (165, 158, 125));
        assert_eq!(gradient(CONTEXT_GRADIENT, 25.0), (200, 113, 80));
    }
}
