use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

use serde_json::{json, Value};

use crate::segments::cache_ttl::CacheSnapshot;

pub fn append(
    json_input: &Value,
    now: i64,
    snapshot: Option<&CacheSnapshot>,
    rendered: Option<&str>,
) {
    let Ok(home) = std::env::var("HOME") else {
        return;
    };

    let path = PathBuf::from(home)
        .join(".claude")
        .join("statusline-cache-ttl-debug.jsonl");

    let transcript_path = json_input.get("transcript_path").and_then(Value::as_str);

    let mut transcript_size: Option<u64> = None;
    let mut transcript_mtime: Option<i64> = None;
    if let Some(p) = transcript_path {
        if let Ok(meta) = fs::metadata(p) {
            transcript_size = Some(meta.len());
            transcript_mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64);
        }
    }

    let snapshot_value = snapshot.map(|s| {
        json!({
            "last_activity": s.last_activity,
            "ttl_secs": s.ttl_secs,
            "remaining": s.ttl_secs - (now - s.last_activity),
        })
    });

    let recent_rows = transcript_path.map(recent_assistant_rows).unwrap_or_default();

    let rendered_text = rendered.map(strip_ansi);

    let session_id = json_input.get("session_id").and_then(Value::as_str);
    let cwd = json_input.get("cwd").and_then(Value::as_str);

    let entry = json!({
        "now": now,
        "pid": std::process::id(),
        "ppid": read_ppid(),
        "session_id": session_id,
        "cwd": cwd,
        "transcript_path": transcript_path,
        "transcript_size": transcript_size,
        "transcript_mtime": transcript_mtime,
        "snapshot": snapshot_value,
        "recent_rows": recent_rows,
        "rendered": rendered_text,
    });

    let line = format!("{}\n", entry);

    let _ = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| f.write_all(line.as_bytes()));
}

fn recent_assistant_rows(transcript: &str) -> Vec<Value> {
    let Ok(body) = fs::read_to_string(transcript) else {
        return Vec::new();
    };

    let mut out: Vec<Value> = Vec::new();
    for line in body.lines().rev() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if v.get("type").and_then(Value::as_str) != Some("assistant") {
            continue;
        }

        let usage = v.pointer("/message/usage");
        let creation = usage
            .and_then(|u| u.get("cache_creation_input_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let read = usage
            .and_then(|u| u.get("cache_read_input_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let e1h = usage
            .and_then(|u| u.pointer("/cache_creation/ephemeral_1h_input_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let e5m = usage
            .and_then(|u| u.pointer("/cache_creation/ephemeral_5m_input_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or(0);

        out.push(json!({
            "ts": v.get("timestamp").and_then(Value::as_str),
            "side": v.get("isSidechain").and_then(Value::as_bool),
            "model": v.pointer("/message/model").and_then(Value::as_str),
            "creation": creation,
            "read": read,
            "e1h": e1h,
            "e5m": e5m,
        }));

        if out.len() >= 5 {
            break;
        }
    }
    out
}

fn read_ppid() -> Option<u32> {
    let body = fs::read_to_string("/proc/self/status").ok()?;
    for line in body.lines() {
        if let Some(rest) = line.strip_prefix("PPid:") {
            return rest.trim().parse().ok();
        }
    }
    None
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_escape = false;
    for c in s.chars() {
        if in_escape {
            if c == 'm' {
                in_escape = false;
            }
            continue;
        }
        if c == '\x1b' {
            in_escape = true;
            continue;
        }
        out.push(c);
    }
    out
}
