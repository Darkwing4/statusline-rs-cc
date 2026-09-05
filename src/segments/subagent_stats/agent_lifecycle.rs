use std::fmt;
use std::io::Read;

use serde::de::{IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::iso8601::parse_iso8601_utc;
use crate::transcript_forward_reader::read_records_forward;

use super::session_cache::{PendingAgent, TranscriptState};

const AGENT_TOOL_NAME: &str = "Agent";
const ASYNC_LAUNCH_STATUS: &str = "async_launched";
const NOTIFICATION_MARKER: &str = "<task-notification>";
const NOTIFICATION_ID_OPEN: &str = "<tool-use-id>";
const NOTIFICATION_ID_CLOSE: &str = "</tool-use-id>";

pub(super) fn scan_agent_activity<R: Read>(reader: R, state: &mut TranscriptState) -> u64 {
    read_records_forward(reader, |record| {
        let Ok(row) = serde_json::from_slice::<TranscriptRow>(record) else {
            return;
        };

        if row.is_sidechain == Some(true) {
            return;
        }

        if let Some(id) = queued_notification_id(&row) {
            finish(&mut state.pending, id);
            return;
        }

        let timestamp = row.timestamp.as_deref().and_then(parse_iso8601_utc);
        let Some(content) = row.message.and_then(|message| message.content) else {
            return;
        };

        match content {
            RowContent::Blocks(blocks) => {
                for block in blocks {
                    apply_block(&block, &row.tool_use_result, timestamp, state);
                }
            }
            RowContent::Notification(text) => {
                if let Some(id) = notification_tool_use_id(&text) {
                    finish(&mut state.pending, id);
                }
            }
            RowContent::Unrelated => {}
        }
    })
}

fn apply_block(
    block: &ContentBlock,
    outcome: &Option<ToolUseOutcome>,
    timestamp: Option<i64>,
    state: &mut TranscriptState,
) {
    let Some(kind) = block.kind.as_deref() else {
        return;
    };

    let Some(id) = block.id.as_deref() else {
        return;
    };

    if kind == "tool_use" {
        if block.name.as_deref() != Some(AGENT_TOOL_NAME) {
            return;
        }

        state.launched += 1;
        state.pending.push(PendingAgent {
            tool_use_id: id.to_string(),
            started_at: timestamp,
        });
        return;
    }

    if kind != "tool_result" {
        return;
    }

    let still_running = outcome
        .as_ref()
        .and_then(|outcome| outcome.status.as_deref())
        .is_some_and(|status| status == ASYNC_LAUNCH_STATUS);

    if !still_running {
        finish(&mut state.pending, id);
    }
}

fn finish(pending: &mut Vec<PendingAgent>, tool_use_id: &str) {
    pending.retain(|agent| agent.tool_use_id != tool_use_id);
}

fn queued_notification_id(row: &TranscriptRow) -> Option<&str> {
    let queued = row
        .attachment
        .as_ref()
        .and_then(|queued| queued.prompt.as_ref());
    let text = notification_text(row.content.as_ref()).or_else(|| notification_text(queued))?;

    notification_tool_use_id(text)
}

fn notification_text(content: Option<&RowContent>) -> Option<&str> {
    match content? {
        RowContent::Notification(text) => Some(text),
        _ => None,
    }
}

fn notification_tool_use_id(text: &str) -> Option<&str> {
    if !text.contains(NOTIFICATION_MARKER) {
        return None;
    }

    let start = text.find(NOTIFICATION_ID_OPEN)? + NOTIFICATION_ID_OPEN.len();
    let rest = &text[start..];
    let end = rest.find(NOTIFICATION_ID_CLOSE)?;

    Some(rest[..end].trim())
}

#[derive(Deserialize)]
struct TranscriptRow {
    #[serde(rename = "isSidechain")]
    is_sidechain: Option<bool>,
    timestamp: Option<String>,
    message: Option<RowMessage>,
    #[serde(rename = "toolUseResult")]
    tool_use_result: Option<ToolUseOutcome>,
    content: Option<RowContent>,
    attachment: Option<RowAttachment>,
}

#[derive(Deserialize)]
struct RowAttachment {
    prompt: Option<RowContent>,
}

#[derive(Deserialize)]
struct RowMessage {
    content: Option<RowContent>,
}

enum RowContent {
    Blocks(Vec<ContentBlock>),
    Notification(String),
    Unrelated,
}

impl<'de> Deserialize<'de> for RowContent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(RowContentVisitor)
    }
}

struct RowContentVisitor;

impl<'de> Visitor<'de> for RowContentVisitor {
    type Value = RowContent;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Claude message content")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        if value.contains(NOTIFICATION_MARKER) {
            return Ok(RowContent::Notification(value.to_string()));
        }

        Ok(RowContent::Unrelated)
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        if value.contains(NOTIFICATION_MARKER) {
            return Ok(RowContent::Notification(value));
        }

        Ok(RowContent::Unrelated)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut blocks = Vec::new();
        while let Some(block) = sequence.next_element::<ContentBlock>()? {
            if block.kind.is_some() {
                blocks.push(block);
            }
        }

        Ok(RowContent::Blocks(blocks))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        Ok(RowContent::Unrelated)
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(RowContent::Unrelated)
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(RowContent::Unrelated)
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(RowContent::Unrelated)
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(RowContent::Unrelated)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(RowContent::Unrelated)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(RowContent::Unrelated)
    }
}

struct ContentBlock {
    kind: Option<String>,
    id: Option<String>,
    name: Option<String>,
}

impl<'de> Deserialize<'de> for ContentBlock {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ContentBlockVisitor)
    }
}

struct ContentBlockVisitor;

impl<'de> Visitor<'de> for ContentBlockVisitor {
    type Value = ContentBlock;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a Claude content block")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut block = ContentBlock {
            kind: None,
            id: None,
            name: None,
        };

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "type" => block.kind = string_value(map.next_value::<Value>()?),
                "name" => block.name = string_value(map.next_value::<Value>()?),
                "id" | "tool_use_id" => block.id = string_value(map.next_value::<Value>()?),
                _ => {
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }

        Ok(block)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence.next_element::<IgnoredAny>()?.is_some() {}
        Ok(unrelated_block())
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E> {
        Ok(unrelated_block())
    }

    fn visit_string<E>(self, _value: String) -> Result<Self::Value, E> {
        Ok(unrelated_block())
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(unrelated_block())
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(unrelated_block())
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(unrelated_block())
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(unrelated_block())
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(unrelated_block())
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(unrelated_block())
    }
}

fn unrelated_block() -> ContentBlock {
    ContentBlock {
        kind: None,
        id: None,
        name: None,
    }
}

fn string_value(value: Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

struct ToolUseOutcome {
    status: Option<String>,
}

impl<'de> Deserialize<'de> for ToolUseOutcome {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ToolUseOutcomeVisitor)
    }
}

struct ToolUseOutcomeVisitor;

impl<'de> Visitor<'de> for ToolUseOutcomeVisitor {
    type Value = ToolUseOutcome;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a tool use result")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut status = None;

        while let Some(key) = map.next_key::<String>()? {
            if key == "status" {
                status = string_value(map.next_value::<Value>()?);
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }

        Ok(ToolUseOutcome { status })
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence.next_element::<IgnoredAny>()?.is_some() {}
        Ok(ToolUseOutcome { status: None })
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E> {
        Ok(ToolUseOutcome { status: None })
    }

    fn visit_string<E>(self, _value: String) -> Result<Self::Value, E> {
        Ok(ToolUseOutcome { status: None })
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(ToolUseOutcome { status: None })
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(ToolUseOutcome { status: None })
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(ToolUseOutcome { status: None })
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(ToolUseOutcome { status: None })
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(ToolUseOutcome { status: None })
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(ToolUseOutcome { status: None })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{parse_iso8601_utc, scan_agent_activity, TranscriptState};

    const LAUNCH_FIRST: &str = r#"{"type":"assistant","timestamp":"2026-01-01T00:00:01Z","message":{"content":[{"type":"tool_use","id":"toolu_1","name":"Agent","input":{"prompt":"go"}}]}}"#;
    const LAUNCH_SECOND: &str = r#"{"type":"assistant","timestamp":"2026-01-01T00:00:02Z","message":{"content":[{"type":"tool_use","id":"toolu_2","name":"Agent","input":{"prompt":"go"}}]}}"#;
    const FINISH_FIRST: &str = r#"{"type":"user","timestamp":"2026-01-01T00:00:03Z","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_1","content":"done"}]},"toolUseResult":{"status":"completed","agentId":"a1"}}"#;

    fn scan(transcript: &str) -> TranscriptState {
        let mut state = TranscriptState::default();
        scan_agent_activity(Cursor::new(transcript.as_bytes()), &mut state);

        state
    }

    fn started_at(state: &TranscriptState) -> Vec<i64> {
        state
            .pending
            .iter()
            .filter_map(|agent| agent.started_at)
            .collect()
    }

    #[test]
    fn counts_launched_agents_and_keeps_unfinished_ones_active() {
        let transcript = format!("{LAUNCH_FIRST}\n{LAUNCH_SECOND}\n{FINISH_FIRST}\n");
        let state = scan(&transcript);

        assert_eq!(state.launched, 2);
        assert_eq!(
            started_at(&state),
            vec![parse_iso8601_utc("2026-01-01T00:00:02Z").unwrap()]
        );
    }

    #[test]
    fn resumes_from_the_previous_offset_without_double_counting() {
        let head = format!("{LAUNCH_FIRST}\n{LAUNCH_SECOND}\n");
        let transcript = format!("{head}{FINISH_FIRST}\n");

        let mut state = TranscriptState::default();
        let scanned = scan_agent_activity(Cursor::new(head.as_bytes()), &mut state);
        assert_eq!(state.launched, 2);
        assert_eq!(state.pending.len(), 2);

        let mut tail = Cursor::new(transcript.as_bytes());
        tail.set_position(scanned);
        scan_agent_activity(&mut tail, &mut state);

        let one_pass = scan(&transcript);
        assert_eq!(state.launched, one_pass.launched);
        assert_eq!(started_at(&state), started_at(&one_pass));
    }

    #[test]
    fn stops_at_a_partially_written_row_and_reports_its_offset() {
        let complete = format!("{LAUNCH_FIRST}\n");
        let transcript = format!("{complete}{}", &LAUNCH_SECOND[..40]);

        let mut state = TranscriptState::default();
        let scanned = scan_agent_activity(Cursor::new(transcript.as_bytes()), &mut state);

        assert_eq!(state.launched, 1);
        assert!(scanned <= complete.len() as u64);
    }

    #[test]
    fn keeps_async_agents_active_until_their_task_notification() {
        let launch = concat!(
            r#"{"type":"assistant","timestamp":"2026-01-01T00:00:01Z","message":{"content":[{"type":"tool_use","id":"toolu_1","name":"Agent","input":{}}]}}"#,
            "\n",
            r#"{"type":"user","timestamp":"2026-01-01T00:00:02Z","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_1","content":"launched"}]},"toolUseResult":{"status":"async_launched","agentId":"a1","isAsync":true}}"#,
            "\n",
        );

        assert_eq!(scan(launch).pending.len(), 1);

        let finished = format!(
            "{launch}{}\n",
            r#"{"type":"user","timestamp":"2026-01-01T00:00:09Z","message":{"content":"<task-notification>\n<task-id>a1</task-id>\n<tool-use-id>toolu_1</tool-use-id>\n<status>completed</status>\n</task-notification>"}}"#
        );
        let state = scan(&finished);

        assert_eq!(state.launched, 1);
        assert!(state.pending.is_empty());
    }

    #[test]
    fn finishes_async_agents_whose_notification_was_absorbed_mid_turn() {
        let transcript = concat!(
            r#"{"type":"assistant","timestamp":"2026-01-01T00:00:01Z","message":{"content":[{"type":"tool_use","id":"toolu_1","name":"Agent","input":{}}]}}"#,
            "\n",
            r#"{"type":"user","timestamp":"2026-01-01T00:00:02Z","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_1","content":"launched"}]},"toolUseResult":{"status":"async_launched","agentId":"a1","isAsync":true}}"#,
            "\n",
            r#"{"type":"queue-operation","operation":"enqueue","timestamp":"2026-01-01T00:00:08Z","content":"<task-notification>\n<task-id>a1</task-id>\n<tool-use-id>toolu_1</tool-use-id>\n<status>killed</status>\n</task-notification>"}"#,
            "\n",
        );
        let state = scan(transcript);

        assert_eq!(state.launched, 1);
        assert!(state.pending.is_empty());
    }

    #[test]
    fn finishes_async_agents_from_a_queued_command_attachment() {
        let transcript = concat!(
            r#"{"type":"assistant","timestamp":"2026-01-01T00:00:01Z","message":{"content":[{"type":"tool_use","id":"toolu_1","name":"Agent","input":{}}]}}"#,
            "\n",
            r#"{"type":"user","timestamp":"2026-01-01T00:00:02Z","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_1","content":"launched"}]},"toolUseResult":{"status":"async_launched","agentId":"a1","isAsync":true}}"#,
            "\n",
            r#"{"type":"attachment","isSidechain":false,"timestamp":"2026-01-01T00:00:08Z","attachment":{"type":"queued_command","commandMode":"task-notification","prompt":"<task-notification>\n<task-id>a1</task-id>\n<tool-use-id>toolu_1</tool-use-id>\n<status>completed</status>\n</task-notification>"}}"#,
            "\n",
        );
        let state = scan(transcript);

        assert_eq!(state.launched, 1);
        assert!(state.pending.is_empty());
    }

    #[test]
    fn ignores_sidechain_rows_and_other_tools() {
        let transcript = concat!(
            r#"{"type":"assistant","isSidechain":true,"timestamp":"2026-01-01T00:00:01Z","message":{"content":[{"type":"tool_use","id":"toolu_1","name":"Agent","input":{}}]}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-01-01T00:00:02Z","message":{"content":[{"type":"tool_use","id":"toolu_2","name":"Bash","input":{"command":"ls"}}]}}"#,
            "\n",
            r#"{"type":"user","timestamp":"2026-01-01T00:00:03Z","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_2","content":[{"type":"text","text":"files"}]}]},"toolUseResult":{"stdout":"files","stderr":""}}"#,
            "\n",
        );
        let state = scan(transcript);

        assert_eq!(state.launched, 0);
        assert!(state.pending.is_empty());
    }

    #[test]
    fn survives_rows_where_tool_use_result_is_not_an_object() {
        let transcript = concat!(
            r#"{"type":"assistant","timestamp":"2026-01-01T00:00:01Z","message":{"content":[{"type":"tool_use","id":"toolu_1","name":"Agent","input":{}}]}}"#,
            "\n",
            r#"{"type":"user","timestamp":"2026-01-01T00:00:02Z","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_9","content":"x"}]},"toolUseResult":"plain string"}"#,
            "\n",
            r#"{"type":"user","timestamp":"2026-01-01T00:00:03Z","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_1","content":"x"}]},"toolUseResult":["a","b"]}"#,
            "\n",
        );
        let state = scan(transcript);

        assert_eq!(state.launched, 1);
        assert!(state.pending.is_empty());
    }
}
