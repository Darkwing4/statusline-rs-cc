use std::fs::File;
use std::io::{Read, Seek};
use std::ops::ControlFlow;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

pub use crate::config_schema::CacheTtl;
use crate::segments::{GitCache, Segment};
use crate::transcript_tail_reader::scan_jsonl_lines_from_end;
use crate::types::Color;

const TTL_5M_SECS: i64 = 5 * 60;
const TTL_1H_SECS: i64 = 60 * 60;

pub(super) struct CacheSnapshot {
    pub(super) last_activity: i64,
    pub(super) ttl_secs: i64,
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
    let result = scan_jsonl_lines_from_end(reader, |line| {
        let Some(row) = parse_usage_row(line) else {
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

fn parse_usage_row(line: &str) -> Option<UsageRow> {
    let v: Value = serde_json::from_str(line).ok()?;

    if v.get("type").and_then(Value::as_str) != Some("assistant") {
        return None;
    }

    if v.get("isSidechain").and_then(Value::as_bool) == Some(true) {
        return None;
    }

    let usage = v.pointer("/message/usage")?;

    let creation = usage
        .get("cache_creation_input_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let read = usage
        .get("cache_read_input_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    if creation == 0 && read == 0 {
        return None;
    }

    let timestamp = v
        .get("timestamp")
        .and_then(Value::as_str)
        .and_then(parse_iso8601_utc);

    let e1h = usage
        .pointer("/cache_creation/ephemeral_1h_input_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let e5m = usage
        .pointer("/cache_creation/ephemeral_5m_input_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    Some(UsageRow {
        timestamp,
        e1h,
        e5m,
    })
}

fn active_view(prefix: &str, remaining: i64, ttl: i64) -> (String, (u8, u8, u8)) {
    let text = format!("{}{}", prefix, format_duration(remaining));
    let burned = ((ttl - remaining) as f64 / ttl as f64) * 100.0;

    (text, gradient_rgb_ttl(burned))
}

fn cold_view(prefix: &str, json: &Value) -> (String, (u8, u8, u8)) {
    let text = format!("{}cold", prefix);
    let ctx_pct = json
        .get("context_window")
        .and_then(|v| v.get("used_percentage"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);

    (text, gradient_rgb_context_cold(ctx_pct))
}

fn gradient_rgb_ttl(p: f64) -> (u8, u8, u8) {
    let lerp = |a: i32, b: i32, t: f64| (a as f64 + (b - a) as f64 * t.clamp(0.0, 1.0)) as u8;

    if p <= 70.0 {
        let t = p / 70.0;
        (lerp(120, 200, t), lerp(120, 180, t), lerp(120, 80, t))
    } else if p <= 90.0 {
        let t = (p - 70.0) / 20.0;
        (lerp(200, 230, t), lerp(180, 100, t), lerp(80, 60, t))
    } else {
        let t = (p - 90.0) / 10.0;
        (lerp(230, 255, t), lerp(100, 50, t), lerp(60, 50, t))
    }
}

fn gradient_rgb_context_cold(p: f64) -> (u8, u8, u8) {
    let lerp = |a: i32, b: i32, t: f64| (a as f64 + (b - a) as f64 * t.clamp(0.0, 1.0)) as u8;

    if p <= 40.0 {
        let t = p / 40.0;
        (lerp(120, 200, t), lerp(120, 180, t), lerp(120, 80, t))
    } else if p <= 75.0 {
        let t = (p - 40.0) / 35.0;
        (lerp(200, 230, t), lerp(180, 100, t), lerp(80, 60, t))
    } else {
        let t = (p - 75.0) / 25.0;
        (lerp(230, 255, t), lerp(100, 50, t), lerp(60, 50, t))
    }
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

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{parse_iso8601_utc, read_cache_snapshot_from, TTL_1H_SECS};

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
