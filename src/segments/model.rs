use serde_json::Value;

pub use crate::config_schema::Model;
use crate::segments::{json_field, GitCache, Segment};

impl Segment for Model {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let name = json_field(json, "/model/display_name")
            .or_else(|| json_field(json, "/model/id"))?;

        Some(self.color.paint(&format!("{}{}", self.prefix, name)))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::Model;
    use crate::segments::{GitCache, Segment};
    use crate::config_schema::Color;

    fn render(json: Value, prefix: &str) -> Option<String> {
        let segment = Model {
            color: Color::Named(36),
            prefix: prefix.to_string(),
        };
        let mut git = GitCache::new(String::new());

        segment.render(&json, &mut git)
    }

    #[test]
    fn renders_display_name() {
        let json = json!({
            "model": {
                "display_name": "Claude Opus 4.1",
                "id": "claude-opus-4-1"
            }
        });

        assert_eq!(
            render(json, ""),
            Some("\u{1b}[36mClaude Opus 4.1\u{1b}[0m".to_string())
        );
    }

    #[test]
    fn falls_back_to_id() {
        let json = json!({
            "model": {
                "display_name": "",
                "id": "claude-sonnet-4"
            }
        });

        assert_eq!(
            render(json, ""),
            Some("\u{1b}[36mclaude-sonnet-4\u{1b}[0m".to_string())
        );
    }

    #[test]
    fn renders_prefix() {
        let json = json!({"model": {"display_name": "Opus"}});

        assert_eq!(
            render(json, "model "),
            Some("\u{1b}[36mmodel Opus\u{1b}[0m".to_string())
        );
    }

    #[test]
    fn hides_when_model_is_missing_or_empty() {
        assert_eq!(render(json!({}), ""), None);
        assert_eq!(
            render(json!({"model": {"display_name": "", "id": ""}}), ""),
            None
        );
    }
}
