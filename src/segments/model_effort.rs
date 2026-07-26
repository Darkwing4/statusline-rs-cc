use serde::Deserialize;
use serde_json::Value;

use crate::segments::{GitCache, Segment};
use crate::types::Color;

#[derive(Deserialize)]
pub struct ModelEffort {
    pub model_color: Color,
    pub effort_color: Color,
}

impl Segment for ModelEffort {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let model = json
            .pointer("/model/display_name")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .or_else(|| {
                json.pointer("/model/id")
                    .and_then(Value::as_str)
                    .filter(|value| !value.is_empty())
            })?;

        let mut rendered = self.model_color.paint(model);

        if let Some(effort) = json
            .pointer("/effort/level")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        {
            rendered.push(' ');
            rendered.push_str(&self.effort_color.paint(effort));
        }

        Some(rendered)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::ModelEffort;
    use crate::segments::{GitCache, Segment};
    use crate::types::Color;

    fn render(json: Value) -> Option<String> {
        let segment = ModelEffort {
            model_color: Color::Named(36),
            effort_color: Color::Named(33),
        };
        let mut git = GitCache::new(String::new());

        segment.render(&json, &mut git)
    }

    #[test]
    fn renders_model_and_effort() {
        let json = json!({
            "model": {
                "display_name": "Claude Opus 4.1",
                "id": "claude-opus-4-1"
            },
            "effort": {
                "level": "high"
            }
        });

        assert_eq!(
            render(json),
            Some("\u{1b}[36mClaude Opus 4.1\u{1b}[0m \u{1b}[33mhigh\u{1b}[0m".to_string())
        );
    }

    #[test]
    fn renders_model_without_effort() {
        let json = json!({
            "model": {
                "display_name": "Claude Sonnet 4"
            }
        });

        assert_eq!(
            render(json),
            Some("\u{1b}[36mClaude Sonnet 4\u{1b}[0m".to_string())
        );
    }

    #[test]
    fn falls_back_to_model_id() {
        let json = json!({
            "model": {
                "display_name": "",
                "id": "claude-sonnet-4"
            }
        });

        assert_eq!(
            render(json),
            Some("\u{1b}[36mclaude-sonnet-4\u{1b}[0m".to_string())
        );
    }

    #[test]
    fn hides_effort_when_model_is_missing_or_empty() {
        assert_eq!(render(json!({"effort": {"level": "high"}})), None);
        assert_eq!(
            render(json!({
                "model": {
                    "display_name": "",
                    "id": ""
                },
                "effort": {
                    "level": "high"
                }
            })),
            None
        );
    }
}
