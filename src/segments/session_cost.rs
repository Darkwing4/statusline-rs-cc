use serde_json::Value;

pub use crate::config_schema::SessionCost;
use crate::segments::{GitCache, Segment};

impl Segment for SessionCost {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let dollars = json.pointer("/cost/total_cost_usd")?.as_f64()?;
        let text = self.format(dollars)?;

        Some(self.color.paint(&text))
    }
}

impl SessionCost {
    fn format(&self, dollars: f64) -> Option<String> {
        if !dollars.is_finite() || dollars <= 0.0 {
            return None;
        }

        Some(format!("{}${:.2}", self.prefix, dollars))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::SessionCost;
    use crate::config_schema::Color;
    use crate::segments::{GitCache, Segment};

    fn cost() -> SessionCost {
        SessionCost {
            color: Color::Named(90),
            prefix: String::new(),
        }
    }

    #[test]
    fn shows_the_session_cost_in_dollars_and_cents() {
        let json = json!({"cost": {"total_cost_usd": 4.2}});
        let mut git = GitCache::new(String::new());

        assert_eq!(
            cost().render(&json, &mut git),
            Some("\x1b[90m$4.20\x1b[0m".to_string())
        );
    }

    #[test]
    fn hides_until_the_first_response_is_priced() {
        let mut git = GitCache::new(String::new());

        assert_eq!(cost().render(&json!({}), &mut git), None);
        assert_eq!(
            cost().render(&json!({"cost": {"total_cost_usd": 0}}), &mut git),
            None
        );
    }
}
