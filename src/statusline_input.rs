use std::io::{self, Read};
use std::path::Path;

use serde_json::Value;

pub fn read() -> Option<Value> {
    let mut body = Vec::new();
    io::stdin().lock().read_to_end(&mut body).ok()?;

    let parsed: Value = serde_json::from_slice(&body).ok()?;

    Some(parsed)
}

pub fn cwd(json: &Value) -> Option<&str> {
    json.get("cwd")
        .and_then(|v| v.as_str())
        .or_else(|| {
            json.get("workspace")
                .and_then(|w| w.get("current_dir"))
                .and_then(|v| v.as_str())
        })
        .filter(|s| !s.is_empty())
}

pub fn session_key(json: &Value) -> Option<String> {
    if let Some(session_id) = json
        .get("session_id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    {
        return Some(session_id.to_string());
    }

    let transcript = json.get("transcript_path").and_then(Value::as_str)?;

    Path::new(transcript)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{cwd, session_key};

    #[test]
    fn uses_top_level_cwd() {
        let json = json!({
            "cwd": "/top-level",
            "workspace": {
                "current_dir": "/workspace"
            }
        });

        assert_eq!(cwd(&json), Some("/top-level"));
    }

    #[test]
    fn falls_back_to_workspace_current_dir() {
        for json in [
            json!({
                "workspace": {
                    "current_dir": "/workspace"
                }
            }),
            json!({
                "cwd": null,
                "workspace": {
                    "current_dir": "/workspace"
                }
            }),
            json!({
                "cwd": 42,
                "workspace": {
                    "current_dir": "/workspace"
                }
            }),
        ] {
            assert_eq!(cwd(&json), Some("/workspace"));
        }
    }

    #[test]
    fn returns_none_when_cwd_is_unavailable() {
        assert_eq!(cwd(&json!({})), None);
        assert_eq!(cwd(&json!({"workspace": {"current_dir": ""}})), None);
    }

    #[test]
    fn prefers_the_session_id_over_the_transcript_name() {
        let json = json!({
            "session_id": "58eb9b9b",
            "transcript_path": "/home/me/.claude/projects/p/other.jsonl"
        });

        assert_eq!(session_key(&json).as_deref(), Some("58eb9b9b"));
    }

    #[test]
    fn falls_back_to_the_transcript_file_name() {
        let json = json!({
            "session_id": "",
            "transcript_path": "/home/me/.claude/projects/p/58eb9b9b.jsonl"
        });

        assert_eq!(session_key(&json).as_deref(), Some("58eb9b9b"));
    }

    #[test]
    fn returns_none_without_a_session_or_transcript() {
        assert_eq!(session_key(&json!({})), None);
    }
}
