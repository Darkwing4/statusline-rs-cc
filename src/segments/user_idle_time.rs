use std::fs::File;
use std::io::{Read, Seek};
use std::ops::ControlFlow;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use serde_json::Value;

pub use crate::config_schema::UserIdleTime;
use crate::duration_format::format_duration;
use crate::iso8601::parse_iso8601_utc;
use crate::segments::{GitCache, Segment};
use crate::transcript_record_probe::{has_tool_result, has_type};
use crate::transcript_tail_reader::{scan_jsonl_records_from_end, JsonlRecord};

#[derive(Deserialize)]
struct RawUserLine {
    timestamp: Option<String>,
}

impl Segment for UserIdleTime {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let transcript = json.get("transcript_path")?.as_str()?;
        let mut file = File::open(transcript).ok()?;
        let last_ts = read_last_user_input_timestamp(&mut file)?;

        let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;

        let diff = now - last_ts;
        if diff < self.threshold_seconds as i64 {
            return None;
        }

        let text = format!("{}{}", self.prefix, format_duration(diff));
        Some(self.color.paint(&text))
    }
}

fn read_last_user_input_timestamp<R: Read + Seek>(reader: &mut R) -> Option<i64> {
    let result = scan_jsonl_records_from_end(reader, |record| {
        if !has_type(record, "user") || record.rewind().is_err() {
            return ControlFlow::Continue(());
        }

        if has_tool_result(record) || record.rewind().is_err() {
            return ControlFlow::Continue(());
        }

        let Some(timestamp) = parse_user_input_timestamp(record) else {
            return ControlFlow::Continue(());
        };

        ControlFlow::Break(timestamp)
    })
    .ok()?;

    match result {
        ControlFlow::Break(timestamp) => Some(timestamp),
        ControlFlow::Continue(()) => None,
    }
}

fn parse_user_input_timestamp(record: &mut dyn JsonlRecord) -> Option<i64> {
    let row: RawUserLine = serde_json::from_reader(record).ok()?;

    row.timestamp.as_deref().and_then(parse_iso8601_utc)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{parse_iso8601_utc, read_last_user_input_timestamp};
    use crate::transcript_tail_reader::BLOCK_SIZE_BYTES;

    #[test]
    fn finds_latest_real_user_message_with_a_timestamp() {
        let transcript = concat!(
            r#"{"type":"user","timestamp":"2026-01-01T00:00:01Z","message":{"content":"first"}}"#,
            "\n",
            r#"{"type":"user","timestamp":"2026-01-01T00:00:02Z","message":{"content":[{"type":"text","text":"second"}]}}"#,
            "\n",
            r#"{"type":"user","timestamp":"2026-01-01T00:00:03Z","message":{"content":[{"type":"tool_result","content":"done"}]}}"#,
            "\n",
            r#"{"type":"user","message":{"content":"missing timestamp"}}"#,
            "\n",
            r#"{"type":"user""#,
        );
        let mut reader = Cursor::new(transcript.as_bytes());

        assert_eq!(
            read_last_user_input_timestamp(&mut reader),
            parse_iso8601_utc("2026-01-01T00:00:02Z")
        );
    }

    #[test]
    fn skips_large_tool_result_content() {
        let content = "x".repeat(BLOCK_SIZE_BYTES * 4);
        let user_input =
            r#"{"type":"user","timestamp":"2026-01-01T00:00:02Z","message":{"content":"input"}}"#;
        let tool_result = format!(
            r#"{{"type":"user","timestamp":"2026-01-01T00:00:03Z","message":{{"content":[null,{{"type":"tool_result","content":"{content}"}}]}}}}"#
        );
        let transcript = format!("{user_input}\n{tool_result}");
        let mut reader = Cursor::new(transcript.as_bytes());

        assert_eq!(
            read_last_user_input_timestamp(&mut reader),
            parse_iso8601_utc("2026-01-01T00:00:02Z")
        );
    }

    #[test]
    fn skips_assistant_records() {
        let transcript = concat!(
            r#"{"type":"user","timestamp":"2026-01-01T00:00:01Z","message":{"content":"hi"}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-01-01T00:00:02Z","message":{"content":[{"type":"text","text":"reply"}]}}"#,
        );
        let mut reader = Cursor::new(transcript.as_bytes());

        assert_eq!(
            read_last_user_input_timestamp(&mut reader),
            parse_iso8601_utc("2026-01-01T00:00:01Z")
        );
    }

    #[test]
    fn returns_none_without_user_input() {
        let transcript = r#"{"type":"user","timestamp":"2026-01-01T00:00:03Z","message":{"content":[{"type":"tool_result","content":"done"}]}}"#;
        let mut reader = Cursor::new(transcript.as_bytes());

        assert_eq!(read_last_user_input_timestamp(&mut reader), None);
    }
}
