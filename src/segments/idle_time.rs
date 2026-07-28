use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use serde_json::Value;

use crate::segments::{GitCache, Segment};
use crate::types::Color;

#[derive(Deserialize)]
pub struct IdleTime {
    pub color: Color,
    pub prefix: String,
    pub threshold_seconds: u64,
}

impl Segment for IdleTime {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let transcript = json.get("transcript_path")?.as_str()?;
        let body = fs::read_to_string(transcript).ok()?;

        let last_ts = body
            .lines()
            .rev()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter(is_user_input)
            .find_map(|v| {
                v.get("timestamp")
                    .and_then(|t| t.as_str())
                    .and_then(parse_iso8601_utc)
            })?;

        let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;

        let diff = now - last_ts;
        if diff < self.threshold_seconds as i64 {
            return None;
        }

        let text = format!("{}{}", self.prefix, format_duration(diff));
        Some(self.color.paint(&text))
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
