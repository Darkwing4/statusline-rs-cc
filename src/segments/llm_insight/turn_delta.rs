use std::io::Read;

use serde_json::Value;

use crate::transcript_forward_reader::read_records_forward;

const USER_MARKER: &str = "user:";
const ASSISTANT_MARKER: &str = "assistant:";
const CAVEAT_PREFIX: &str = "Caveat:";
const MAX_MESSAGE_CHARS: usize = 800;

pub(super) struct Scan {
    pub(super) bytes: u64,
    pub(super) turns: usize,
    pub(super) text: String,
}

pub(super) fn scan_delta<R: Read>(reader: R) -> Scan {
    let mut turns = 0;
    let mut lines: Vec<String> = Vec::new();

    let bytes = read_records_forward(reader, |record| {
        let Ok(row) = serde_json::from_slice::<Value>(record) else {
            return;
        };

        if row.get("isSidechain").and_then(Value::as_bool) == Some(true) {
            return;
        }

        if row.get("isMeta").and_then(Value::as_bool) == Some(true) {
            return;
        }

        let Some(kind) = row.get("type").and_then(Value::as_str) else {
            return;
        };

        let Some(content) = row
            .get("message")
            .and_then(|message| message.get("content"))
        else {
            return;
        };

        match kind {
            "user" => {
                if holds_tool_result(content) {
                    return;
                }

                let Some(text) = spoken_text(content) else {
                    return;
                };

                turns += 1;
                lines.push(format!("{} {}", USER_MARKER, text));
            }
            "assistant" => {
                if let Some(text) = spoken_text(content) {
                    lines.push(format!("{} {}", ASSISTANT_MARKER, text));
                }
            }
            _ => {}
        }
    });

    Scan {
        bytes,
        turns,
        text: lines.join("\n"),
    }
}

pub(super) fn keep_tail(text: &str, max_chars: usize) -> String {
    let total = text.chars().count();

    if max_chars == 0 || total <= max_chars {
        return text.to_string();
    }

    text.chars().skip(total - max_chars).collect()
}

fn holds_tool_result(content: &Value) -> bool {
    let Some(blocks) = content.as_array() else {
        return false;
    };

    blocks
        .iter()
        .any(|block| block.get("type").and_then(Value::as_str) == Some("tool_result"))
}

fn spoken_text(content: &Value) -> Option<String> {
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

    if trimmed.is_empty() || trimmed.starts_with('<') || trimmed.starts_with(CAVEAT_PREFIX) {
        return None;
    }

    let collapsed = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");

    Some(keep_head(&collapsed, MAX_MESSAGE_CHARS))
}

fn keep_head(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let kept: String = text.chars().take(max_chars).collect();

    format!("{}…", kept)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{keep_tail, scan_delta};

    #[test]
    fn counts_real_prompts_and_keeps_both_sides_of_the_conversation() {
        let transcript = concat!(
            r#"{"type":"user","message":{"content":"почини сборку"}}"#,
            "\n",
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"смотрю логи"}]}}"#,
            "\n",
            r#"{"type":"user","message":{"content":[{"type":"tool_result","content":"ok"}]}}"#,
            "\n",
            r#"{"type":"user","message":{"content":"теперь тесты"}}"#,
            "\n",
        );

        let scan = scan_delta(Cursor::new(transcript.as_bytes()));

        assert_eq!(scan.turns, 2);
        assert_eq!(
            scan.text,
            "user: почини сборку\nassistant: смотрю логи\nuser: теперь тесты"
        );
        assert_eq!(scan.bytes, transcript.len() as u64);
    }

    #[test]
    fn skips_subagent_traffic_and_injected_blocks() {
        let transcript = concat!(
            r#"{"type":"user","isSidechain":true,"message":{"content":"задача субагента"}}"#,
            "\n",
            r#"{"type":"user","isMeta":true,"message":{"content":[{"type":"text","text":"Workflow authoring reference"}]}}"#,
            "\n",
            r#"{"type":"user","message":{"content":"<system-reminder>x</system-reminder>"}}"#,
            "\n",
            r#"{"type":"user","message":{"content":"настоящий вопрос"}}"#,
            "\n",
        );

        let scan = scan_delta(Cursor::new(transcript.as_bytes()));

        assert_eq!(scan.turns, 1);
        assert_eq!(scan.text, "user: настоящий вопрос");
    }

    #[test]
    fn stops_before_a_record_that_is_still_being_written() {
        let transcript = concat!(
            r#"{"type":"user","message":{"content":"готовая строка"}}"#,
            "\n",
            r#"{"type":"user","message":{"content":"недописанная"#,
        );

        let scan = scan_delta(Cursor::new(transcript.as_bytes()));

        assert_eq!(scan.turns, 1);
        assert_eq!(scan.text, "user: готовая строка");
    }

    #[test]
    fn cuts_one_oversized_message_so_it_cannot_flood_the_window() {
        let long = "с".repeat(super::MAX_MESSAGE_CHARS + 50);
        let transcript = format!(
            "{{\"type\":\"user\",\"message\":{{\"content\":\"{}\"}}}}\n",
            long
        );

        let scan = scan_delta(Cursor::new(transcript.as_bytes()));

        assert_eq!(
            scan.text.chars().count(),
            "user: ".chars().count() + super::MAX_MESSAGE_CHARS + 1
        );
        assert!(scan.text.ends_with('…'));
    }

    #[test]
    fn keeps_only_the_newest_characters_of_a_long_delta() {
        assert_eq!(keep_tail("абвгде", 3), "где");
        assert_eq!(keep_tail("абвгде", 6), "абвгде");
        assert_eq!(keep_tail("абвгде", 0), "абвгде");
    }
}
