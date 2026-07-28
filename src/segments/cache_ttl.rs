use std::fs::File;
use std::io::{Read, Seek};
use std::ops::ControlFlow;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use serde_json::Value;

pub use crate::config_schema::CacheTtl;
use crate::gradient::{gradient, Quantization, Rgb};
use crate::iso8601::parse_iso8601_utc;
use crate::segments::{GitCache, Segment};
use crate::transcript_record_probe::has_type;
use crate::transcript_tail_reader::{scan_jsonl_records_from_end, JsonlRecord};
use crate::types::Color;

const TTL_5M_SECS: i64 = 5 * 60;
const TTL_1H_SECS: i64 = 60 * 60;
const TTL_GRADIENT: &[(f64, Rgb)] = &[
    (0.0, (120, 120, 120)),
    (70.0, (200, 180, 80)),
    (90.0, (230, 100, 60)),
    (100.0, (255, 50, 50)),
];
const COLD_GRADIENT: &[(f64, Rgb)] = &[
    (0.0, (120, 120, 120)),
    (40.0, (200, 180, 80)),
    (75.0, (230, 100, 60)),
    (100.0, (255, 50, 50)),
];

pub(super) struct CacheSnapshot {
    pub(super) last_activity: i64,
    pub(super) ttl_secs: i64,
}

#[derive(Deserialize)]
struct RawLine {
    #[serde(rename = "type")]
    kind: Option<String>,
    #[serde(rename = "isSidechain")]
    is_sidechain: Option<bool>,
    timestamp: Option<String>,
    message: Option<RawMessage>,
}

#[derive(Deserialize)]
struct RawMessage {
    usage: Option<RawUsage>,
}

#[derive(Deserialize)]
struct RawUsage {
    cache_creation_input_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
    cache_creation: Option<RawCacheCreation>,
}

#[derive(Deserialize)]
struct RawCacheCreation {
    ephemeral_1h_input_tokens: Option<u64>,
    ephemeral_5m_input_tokens: Option<u64>,
}

struct UsageRow {
    timestamp: Option<i64>,
    e1h: u64,
    e5m: u64,
}

impl UsageRow {
    fn ttl_hint(&self) -> Option<i64> {
        if self.e1h > 0 {
            return Some(TTL_1H_SECS);
        }

        if self.e5m > 0 {
            return Some(TTL_5M_SECS);
        }

        None
    }
}

impl Segment for CacheTtl {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;
        let snapshot = read_cache_snapshot(json);

        let painted = snapshot.as_ref().map(|s| {
            let remaining = s.ttl_secs - (now - s.last_activity);

            let (text, rgb) = if remaining > 0 {
                active_view(&self.prefix, remaining, s.ttl_secs)
            } else {
                cold_view(&self.prefix, json)
            };

            match self.color {
                Color::Gradient => {
                    let (r, g, b) = rgb;
                    Color::Rgb(r, g, b).paint(&text)
                }
                _ => self.color.paint(&text),
            }
        });

        #[cfg(debug_assertions)]
        crate::segments::debug::cache_ttl_dump::append(
            json,
            now,
            snapshot.as_ref(),
            painted.as_deref(),
        );

        painted
    }
}

fn read_cache_snapshot(json: &Value) -> Option<CacheSnapshot> {
    let transcript = json.get("transcript_path")?.as_str()?;
    let mut file = File::open(transcript).ok()?;
    read_cache_snapshot_from(&mut file)
}

fn read_cache_snapshot_from<R: Read + Seek>(reader: &mut R) -> Option<CacheSnapshot> {
    let mut last_activity: Option<i64> = None;
    let result = scan_jsonl_records_from_end(reader, |record| {
        if !has_type(record, "assistant") || record.rewind().is_err() {
            return ControlFlow::Continue(());
        }

        let Some(row) = parse_usage_row(record) else {
            return ControlFlow::Continue(());
        };

        if last_activity.is_none() {
            if let Some(ts) = row.timestamp {
                last_activity = Some(ts);
            }
        }

        if let Some(ttl) = row.ttl_hint() {
            return ControlFlow::Break(ttl);
        }

        ControlFlow::Continue(())
    })
    .ok()?;

    let ControlFlow::Break(ttl_secs) = result else {
        return None;
    };

    Some(CacheSnapshot {
        last_activity: last_activity?,
        ttl_secs,
    })
}

fn parse_usage_row(record: &mut dyn JsonlRecord) -> Option<UsageRow> {
    let row: RawLine = serde_json::from_reader(record).ok()?;

    if row.kind.as_deref() != Some("assistant") {
        return None;
    }

    if row.is_sidechain == Some(true) {
        return None;
    }

    let usage = row.message?.usage?;

    let creation = usage.cache_creation_input_tokens.unwrap_or(0);
    let read = usage.cache_read_input_tokens.unwrap_or(0);

    if creation == 0 && read == 0 {
        return None;
    }

    let timestamp = row.timestamp.as_deref().and_then(parse_iso8601_utc);

    let (e1h, e5m) = match usage.cache_creation {
        Some(ephemeral) => (
            ephemeral.ephemeral_1h_input_tokens.unwrap_or(0),
            ephemeral.ephemeral_5m_input_tokens.unwrap_or(0),
        ),
        None => (0, 0),
    };

    Some(UsageRow {
        timestamp,
        e1h,
        e5m,
    })
}

fn active_view(prefix: &str, remaining: i64, ttl: i64) -> (String, (u8, u8, u8)) {
    let text = format!("{}{}", prefix, format_duration(remaining));
    let burned = ((ttl - remaining) as f64 / ttl as f64) * 100.0;

    (text, gradient(TTL_GRADIENT, burned, Quantization::Truncate))
}

fn cold_view(prefix: &str, json: &Value) -> (String, (u8, u8, u8)) {
    let text = format!("{}cold", prefix);
    let ctx_pct = json
        .get("context_window")
        .and_then(|v| v.get("used_percentage"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);

    (
        text,
        gradient(COLD_GRADIENT, ctx_pct, Quantization::Truncate),
    )
}

fn format_duration(seconds: i64) -> String {
    let s = seconds.max(0);
    let h = s / 3600;
    let m = (s % 3600) / 60;
    let sec = s % 60;

    if h > 0 {
        format!("{}h{:02}m", h, m)
    } else if m > 0 {
        format!("{}m{:02}s", m, sec)
    } else {
        format!("{}s", sec)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{
        gradient, parse_iso8601_utc, read_cache_snapshot_from, Quantization, COLD_GRADIENT,
        TTL_1H_SECS, TTL_GRADIENT,
    };

    #[test]
    fn preserves_ttl_gradient() {
        assert_eq!(
            gradient(TTL_GRADIENT, 80.0, Quantization::Truncate),
            (215, 140, 70)
        );
        assert_eq!(
            gradient(TTL_GRADIENT, 95.0, Quantization::Truncate),
            (242, 75, 55)
        );
    }

    #[test]
    fn preserves_cold_gradient() {
        assert_eq!(
            gradient(COLD_GRADIENT, 57.5, Quantization::Truncate),
            (215, 140, 70)
        );
        assert_eq!(
            gradient(COLD_GRADIENT, 87.5, Quantization::Truncate),
            (242, 75, 55)
        );
    }

    #[test]
    fn uses_newest_activity_and_nearest_older_ttl_hint() {
        let transcript = concat!(
            r#"{"type":"assistant","timestamp":"2026-01-01T00:00:01Z","message":{"usage":{"cache_creation_input_tokens":1,"cache_creation":{"ephemeral_5m_input_tokens":1}}}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-01-01T00:00:02Z","message":{"usage":{"cache_creation_input_tokens":1,"cache_creation":{"ephemeral_1h_input_tokens":1}}}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-01-01T00:00:03Z","message":{"usage":{"cache_read_input_tokens":1}}}"#,
            "\n",
            r#"{"type":"assistant","isSidechain":true,"timestamp":"2026-01-01T00:00:04Z","message":{"usage":{"cache_creation_input_tokens":1,"cache_creation":{"ephemeral_5m_input_tokens":1}}}}"#,
            "\n",
            r#"{"type":"assistant""#,
        );
        let mut reader = Cursor::new(transcript.as_bytes());

        let snapshot = read_cache_snapshot_from(&mut reader).unwrap();

        assert_eq!(
            snapshot.last_activity,
            parse_iso8601_utc("2026-01-01T00:00:03Z").unwrap()
        );
        assert_eq!(snapshot.ttl_secs, TTL_1H_SECS);
    }

    #[test]
    fn stops_at_ttl_hint_without_searching_for_an_older_timestamp() {
        let transcript = concat!(
            r#"{"type":"assistant","timestamp":"2026-01-01T00:00:01Z","message":{"usage":{"cache_read_input_tokens":1}}}"#,
            "\n",
            r#"{"type":"assistant","message":{"usage":{"cache_creation_input_tokens":1,"cache_creation":{"ephemeral_5m_input_tokens":1}}}}"#,
        );
        let mut reader = Cursor::new(transcript.as_bytes());

        assert!(read_cache_snapshot_from(&mut reader).is_none());
    }
}
