use std::fmt;
#[cfg(debug_assertions)]
use std::fs;
use std::fs::File;
use std::io::{Read, Seek};
use std::ops::ControlFlow;
#[cfg(debug_assertions)]
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::de::{IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

pub use crate::config_schema::IdleTime;
use crate::segments::{GitCache, Segment};
use crate::transcript_record_probe::{has_tool_result, has_type};
use crate::transcript_tail_reader::{scan_jsonl_records_from_end, JsonlRecord};

#[derive(Deserialize)]
struct RawUserLine {
    #[serde(rename = "type")]
    kind: Option<String>,
    timestamp: Option<String>,
    message: Option<RawUserMessage>,
}

#[derive(Deserialize)]
struct RawUserMessage {
    content: Option<UserContent>,
}

struct UserContent {
    is_input: bool,
}

impl<'de> Deserialize<'de> for UserContent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(UserContentVisitor)
    }
}

struct UserContentVisitor;

impl<'de> Visitor<'de> for UserContentVisitor {
    type Value = UserContent;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Claude message content")
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E> {
        Ok(UserContent { is_input: true })
    }

    fn visit_string<E>(self, _value: String) -> Result<Self::Value, E> {
        Ok(UserContent { is_input: true })
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut has_tool_result = false;
        while let Some(block) = sequence.next_element::<ContentBlock>()? {
            has_tool_result |= block.kind.as_deref() == Some("tool_result");
        }

        Ok(UserContent {
            is_input: !has_tool_result,
        })
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        while map.next_entry::<String, IgnoredAny>()?.is_some() {}
        Ok(UserContent { is_input: false })
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(UserContent { is_input: false })
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(UserContent { is_input: false })
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(UserContent { is_input: false })
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(UserContent { is_input: false })
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(UserContent { is_input: false })
    }
}

struct ContentBlock {
    kind: Option<String>,
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
        let mut kind = None;
        while let Some(key) = map.next_key::<String>()? {
            if key == "type" {
                let value = map.next_value::<Value>()?;
                kind = value.as_str().map(str::to_owned);
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }

        Ok(ContentBlock { kind })
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence.next_element::<IgnoredAny>()?.is_some() {}
        Ok(ContentBlock { kind: None })
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E> {
        Ok(ContentBlock { kind: None })
    }

    fn visit_string<E>(self, _value: String) -> Result<Self::Value, E> {
        Ok(ContentBlock { kind: None })
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(ContentBlock { kind: None })
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(ContentBlock { kind: None })
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(ContentBlock { kind: None })
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(ContentBlock { kind: None })
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(ContentBlock { kind: None })
    }
}

impl IdleTime {
    #[cfg(debug_assertions)]
    pub fn validate_debug(&self) {
        debug_assert!(
            statusline_refresh_interval_enabled(),
            "IdleTime requires statusLine.refreshInterval > 0 in Claude Code settings"
        );
    }
}

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

    if row.kind.as_deref() != Some("user") {
        return None;
    }

    let content = row.message?.content?;

    if !content.is_input {
        return None;
    }

    row.timestamp.as_deref().and_then(parse_iso8601_utc)
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

fn parse_iso8601_utc(s: &str) -> Option<i64> {
    let s = s.strip_suffix('Z').unwrap_or(s);
    let (date, time) = s.split_once('T')?;

    let mut dp = date.split('-');
    let y: i64 = dp.next()?.parse().ok()?;
    let mo: u32 = dp.next()?.parse().ok()?;
    let d: u32 = dp.next()?.parse().ok()?;

    let time = time.split('.').next()?;
    let mut tp = time.split(':');
    let h: i64 = tp.next()?.parse().ok()?;
    let mi: i64 = tp.next()?.parse().ok()?;
    let se: i64 = tp.next()?.parse().ok()?;

    let days = days_from_civil(y, mo, d);
    Some(days * 86400 + h * 3600 + mi * 60 + se)
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = m as i64;
    let d = d as i64;
    let shifted_m = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * shifted_m + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

#[cfg(debug_assertions)]
fn statusline_refresh_interval_enabled() -> bool {
    let Some(settings_path) = claude_settings_path() else {
        return false;
    };
    let Ok(body) = fs::read_to_string(settings_path) else {
        return false;
    };
    let Ok(json) = serde_json::from_str::<Value>(&body) else {
        return false;
    };

    json.pointer("/statusLine/refreshInterval")
        .and_then(Value::as_f64)
        .is_some_and(|n| n > 0.0)
}

#[cfg(debug_assertions)]
fn claude_settings_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("STATUSLINE_SETTINGS") {
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }

    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".claude").join("settings.json"))
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
}
