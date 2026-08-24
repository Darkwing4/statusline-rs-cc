use serde_json::Value;

pub use crate::config_schema::Effort;
use crate::segments::{json_field, GitCache, Segment};

impl Segment for Effort {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let level = json_field(json, "/effort/level")?;

        Some(self.color.paint(&format!("{}{}", self.prefix, level)))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::Effort;
    use crate::config_schema::Color;
    use crate::segments::{GitCache, Segment};

    fn render(json: Value, prefix: &str) -> Option<String> {
        let segment = Effort {
            color: Color::Named(33),
            prefix: prefix.to_string(),
        };
        let mut git = GitCache::new(String::new());

        segment.render(&json, &mut git)
    }

    #[test]
    fn renders_level() {
        let json = json!({"effort": {"level": "high"}});

        assert_eq!(
            render(json, ""),
            Some("\u{1b}[33mhigh\u{1b}[0m".to_string())
        );
    }

    #[test]
    fn renders_prefix() {
        let json = json!({"effort": {"level": "high"}});

        assert_eq!(
            render(json, "effort "),
            Some("\u{1b}[33meffort high\u{1b}[0m".to_string())
        );
    }

    #[test]
    fn hides_when_effort_is_missing_or_empty() {
        assert_eq!(render(json!({}), ""), None);
        assert_eq!(render(json!({"effort": {"level": ""}}), ""), None);
    }
}
