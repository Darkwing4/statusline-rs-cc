use std::io::{self, Read};

use serde_json::Value;

pub fn read() -> Option<Value> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).ok()?;

    let parsed: Value = serde_json::from_str(&buf).ok()?;

    #[cfg(debug_assertions)]
    {
        if let Ok(pretty) = serde_json::to_string_pretty(&parsed) {
            let _ = std::fs::write("/tmp/statusline-stdin.json", pretty);
        }
    }

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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::cwd;

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
}
