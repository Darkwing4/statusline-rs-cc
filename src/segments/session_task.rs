use std::fs::File;
use std::io::{Read, Seek};
use std::ops::ControlFlow;

use serde_json::Value;

pub use crate::config_schema::SessionTask;
use crate::segments::single_line_text::sanitize;
use crate::segments::{GitCache, Overflow, Segment};
use crate::transcript_record_probe::{has_tool_result, has_type};
use crate::transcript_spoken_text::{is_conversation_record, spoken_text};
use crate::transcript_tail_reader::{scan_jsonl_records_from_end, JsonlRecord};

impl Segment for SessionTask {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let transcript = json.get("transcript_path")?.as_str()?;
        let mut file = File::open(transcript).ok()?;
        let prompt = read_last_user_prompt(&mut file)?;
        let text = sanitize(&prompt, self.max_chars);

        if text.is_empty() {
            return None;
        }

        Some(self.color.paint(&format!("{}{}", self.prefix, text)))
    }

    fn standalone(&self) -> bool {
        self.standalone
    }

    fn overflow(&self) -> Overflow {
        Overflow::Truncate
    }
}

fn read_last_user_prompt<R: Read + Seek>(reader: &mut R) -> Option<String> {
    let result = scan_jsonl_records_from_end(reader, |record| {
        if !has_type(record, "user") || record.rewind().is_err() {
            return ControlFlow::Continue(());
        }

        if has_tool_result(record) || record.rewind().is_err() {
            return ControlFlow::Continue(());
        }

        match parse_user_prompt(record) {
            Some(prompt) => ControlFlow::Break(prompt),
            None => ControlFlow::Continue(()),
        }
    })
    .ok()?;

    match result {
        ControlFlow::Break(prompt) => Some(prompt),
        ControlFlow::Continue(()) => None,
    }
}

fn parse_user_prompt(record: &mut dyn JsonlRecord) -> Option<String> {
    let row: Value = serde_json::from_reader(record).ok()?;

    if row.get("type").and_then(Value::as_str) != Some("user") {
        return None;
    }

    if !is_conversation_record(&row) {
        return None;
    }

    spoken_text(row.get("message")?.get("content")?)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::read_last_user_prompt;

    #[test]
    fn takes_the_latest_prompt_the_user_actually_typed() {
        let transcript = concat!(
            r#"{"type":"user","message":{"content":"первая задача"}}"#,
            "\n",
            r#"{"type":"user","message":{"content":[{"type":"text","text":"почини сборку"}]}}"#,
            "\n",
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"ок"}]}}"#,
            "\n",
            r#"{"type":"user","message":{"content":[{"type":"tool_result","content":"done"}]}}"#,
            "\n",
        );
        let mut reader = Cursor::new(transcript.as_bytes());

        assert_eq!(
            read_last_user_prompt(&mut reader).as_deref(),
            Some("почини сборку")
        );
    }

    #[test]
    fn walks_past_slash_commands_hooks_and_caveats() {
        let transcript = concat!(
            r#"{"type":"user","message":{"content":"собери релиз"}}"#,
            "\n",
            r#"{"type":"user","message":{"content":"<command-name>/compact</command-name>"}}"#,
            "\n",
            r#"{"type":"user","message":{"content":"Caveat: the messages below were generated"}}"#,
            "\n",
        );
        let mut reader = Cursor::new(transcript.as_bytes());

        assert_eq!(
            read_last_user_prompt(&mut reader).as_deref(),
            Some("собери релиз")
        );
    }

    #[test]
    fn ignores_slash_commands_and_skills_injected_as_user_records() {
        let transcript = concat!(
            r#"{"type":"user","message":{"content":"почини сборку"}}"#,
            "\n",
            r#"{"type":"user","isMeta":true,"message":{"content":[{"type":"text","text":"Workflow authoring reference"}]}}"#,
            "\n",
        );
        let mut reader = Cursor::new(transcript.as_bytes());

        assert_eq!(
            read_last_user_prompt(&mut reader).as_deref(),
            Some("почини сборку")
        );
    }

    #[test]
    fn ignores_prompts_that_belong_to_a_subagent() {
        let transcript = concat!(
            r#"{"type":"user","message":{"content":"главная задача"}}"#,
            "\n",
            r#"{"type":"user","isSidechain":true,"message":{"content":"задача субагента"}}"#,
            "\n",
        );
        let mut reader = Cursor::new(transcript.as_bytes());

        assert_eq!(
            read_last_user_prompt(&mut reader).as_deref(),
            Some("главная задача")
        );
    }

    #[test]
    fn walks_past_the_marker_left_by_interrupting_a_turn() {
        let transcript = concat!(
            r#"{"type":"user","isSidechain":false,"message":{"content":"собери билд"}}"#,
            "\n",
            r#"{"type":"user","isSidechain":false,"interruptedMessageId":"msg_1","message":{"content":[{"type":"text","text":"[Request interrupted by user for tool use]"}]}}"#,
            "\n",
            r#"{"type":"user","isSidechain":false,"message":{"content":[{"type":"text","text":"[Request interrupted by user]"}]}}"#,
            "\n",
        );
        let mut reader = Cursor::new(transcript.as_bytes());

        assert_eq!(
            read_last_user_prompt(&mut reader).as_deref(),
            Some("собери билд")
        );
    }

    #[test]
    fn walks_past_the_summary_a_compact_writes_as_a_user_record() {
        let transcript = concat!(
            r#"{"type":"user","isSidechain":false,"message":{"content":"собери билд"}}"#,
            "\n",
            r#"{"type":"user","isSidechain":false,"isCompactSummary":true,"isVisibleInTranscriptOnly":true,"message":{"content":"This session is being continued from a previous conversation that ran out of context."}}"#,
            "\n",
        );
        let mut reader = Cursor::new(transcript.as_bytes());

        assert_eq!(
            read_last_user_prompt(&mut reader).as_deref(),
            Some("собери билд")
        );
    }
}
