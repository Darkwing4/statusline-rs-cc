use serde_json::Value;

const CAVEAT_PREFIX: &str = "Caveat:";
const INTERRUPTION_PREFIX: &str = "[Request interrupted";

pub(crate) fn is_conversation_record(row: &Value) -> bool {
    !flag(row, "isSidechain") && !flag(row, "isMeta") && !flag(row, "isCompactSummary")
}

pub(crate) fn spoken_text(content: &Value) -> Option<String> {
    let joined = match content {
        Value::String(text) => text.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .filter(|block| block.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|block| block.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(" "),
        _ => return None,
    };

    let trimmed = joined.trim();

    if trimmed.is_empty() || is_injected(trimmed) {
        return None;
    }

    Some(trimmed.to_string())
}

fn is_injected(text: &str) -> bool {
    text.starts_with('<')
        || text.starts_with(CAVEAT_PREFIX)
        || text.starts_with(INTERRUPTION_PREFIX)
}

fn flag(row: &Value, name: &str) -> bool {
    row.get(name).and_then(Value::as_bool) == Some(true)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{is_conversation_record, spoken_text};

    #[test]
    fn keeps_records_the_user_and_the_assistant_produced() {
        assert!(is_conversation_record(&json!({"type": "user"})));
        assert!(is_conversation_record(
            &json!({"type": "user", "isSidechain": false})
        ));
    }

    #[test]
    fn drops_subagent_meta_and_compact_summary_records() {
        assert!(!is_conversation_record(
            &json!({"type": "user", "isSidechain": true})
        ));
        assert!(!is_conversation_record(
            &json!({"type": "user", "isMeta": true})
        ));
        assert!(!is_conversation_record(&json!({
            "type": "user",
            "isCompactSummary": true,
            "isVisibleInTranscriptOnly": true
        })));
    }

    #[test]
    fn reads_plain_and_block_content() {
        assert_eq!(
            spoken_text(&json!("  собери релиз  ")).as_deref(),
            Some("собери релиз")
        );
        assert_eq!(
            spoken_text(&json!([
                {"type": "text", "text": "почини"},
                {"type": "image", "source": {}},
                {"type": "text", "text": "сборку"}
            ]))
            .as_deref(),
            Some("почини сборку")
        );
    }

    #[test]
    fn drops_text_the_harness_injected() {
        assert_eq!(
            spoken_text(&json!("<system-reminder>x</system-reminder>")),
            None
        );
        assert_eq!(
            spoken_text(&json!("Caveat: the messages below were generated")),
            None
        );
        assert_eq!(spoken_text(&json!("   ")), None);
        assert_eq!(spoken_text(&json!({"text": "не тот тип"})), None);
    }

    #[test]
    fn drops_the_interruption_marker_claude_code_writes_as_a_user_message() {
        assert_eq!(
            spoken_text(&json!([{"type": "text", "text": "[Request interrupted by user]"}])),
            None
        );
        assert_eq!(
            spoken_text(&json!([
                {"type": "text", "text": "[Request interrupted by user for tool use]"}
            ])),
            None
        );
    }
}
