use serde_json::Value;

pub use crate::config_schema::Model;
use crate::segments::{json_field, GitCache, Segment};

impl Segment for Model {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let raw =
            json_field(json, "/model/display_name").or_else(|| json_field(json, "/model/id"))?;

        let name = self
            .replacements
            .iter()
            .fold(raw.to_string(), |name, (from, to)| name.replace(from, to));

        if name.is_empty() {
            return None;
        }

        Some(self.color.paint(&format!("{}{}", self.prefix, name)))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::Model;
    use crate::config_schema::Color;
    use crate::segments::{GitCache, Segment};

    fn render(json: Value, prefix: &str) -> Option<String> {
        render_with(json, prefix, Vec::new())
    }

    fn render_with(
        json: Value,
        prefix: &str,
        replacements: Vec<(String, String)>,
    ) -> Option<String> {
        let segment = Model {
            color: Color::Named(36),
            prefix: prefix.to_string(),
            replacements,
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
    fn applies_replacements_in_order() {
        let json = json!({"model": {"display_name": "Opus 5 (1M context)"}});
        let replacements = vec![
            (" (1M context)".to_string(), String::new()),
            ("Opus 5".to_string(), "Opus".to_string()),
        ];

        assert_eq!(
            render_with(json, "", replacements),
            Some("\u{1b}[36mOpus\u{1b}[0m".to_string())
        );
    }

    #[test]
    fn hides_when_replacements_empty_the_name() {
        let json = json!({"model": {"display_name": "Opus 5"}});
        let replacements = vec![("Opus 5".to_string(), String::new())];

        assert_eq!(render_with(json, "", replacements), None);
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
