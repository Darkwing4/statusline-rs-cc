use std::io::{self, Read};

use serde_json::Value;

pub fn read() -> Option<Value> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).ok()?;
    serde_json::from_str(&buf).ok()
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
