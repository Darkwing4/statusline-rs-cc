use std::fs::File;
use std::io::{Read, Seek};
use std::ops::ControlFlow;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

pub use crate::config_schema::IdleTime;
use crate::iso8601::parse_iso8601_utc;
use crate::segments::{GitCache, Segment};
use crate::transcript_tail_reader::scan_jsonl_lines_from_end;

impl Segment for IdleTime {
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
    let result = scan_jsonl_lines_from_end(reader, |line| {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return ControlFlow::Continue(());
        };
        if !is_user_input(&value) {
            return ControlFlow::Continue(());
        }

        let Some(timestamp) = value
            .get("timestamp")
            .and_then(Value::as_str)
            .and_then(parse_iso8601_utc)
        else {
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

fn is_user_input(v: &Value) -> bool {
    if v.get("type").and_then(|x| x.as_str()) != Some("user") {
        return false;
    }
    let Some(content) = v.pointer("/message/content") else {
        return false;
    };
    match content {
        Value::String(_) => true,
        Value::Array(arr) => !arr
            .iter()
            .any(|c| c.get("type").and_then(|t| t.as_str()) == Some("tool_result")),
        _ => false,
    }
}

fn format_duration(seconds: i64) -> String {
    let s = seconds.max(0);
    let h = s / 3600;
    let m = (s % 3600) / 60;
    let sec = s % 60;
    if h > 0 {
        format!("{}h{}m", h, m)
    } else if m > 0 {
        format!("{}m{}s", m, sec)
    } else {
        format!("{}s", sec)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{parse_iso8601_utc, read_last_user_input_timestamp};

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
}
