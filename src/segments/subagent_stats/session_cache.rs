use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::statusline_cache_dir::cache_dir;
use crate::transcript_token_tally::TranscriptTally;

#[derive(Default, Deserialize, Serialize)]
pub(super) struct SessionCache {
    #[serde(default)]
    pub(super) transcript: TranscriptState,
    #[serde(default)]
    pub(super) agent_files: Vec<TranscriptTally>,
}

#[derive(Clone, Default, Deserialize, Serialize)]
pub(super) struct TranscriptState {
    pub(super) scanned_bytes: u64,
    pub(super) launched: usize,
    pub(super) pending: Vec<PendingAgent>,
}

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct PendingAgent {
    pub(super) tool_use_id: String,
    pub(super) started_at: Option<i64>,
}

pub(super) fn load(session_key: &str) -> (SessionCache, Option<PathBuf>) {
    let Some(path) = cache_path(session_key) else {
        return (SessionCache::default(), None);
    };

    let cache = fs::read_to_string(&path)
        .ok()
        .and_then(|body| serde_json::from_str(&body).ok())
        .unwrap_or_default();

    (cache, Some(path))
}

pub(super) fn store(path: &PathBuf, cache: &SessionCache) {
    let Some(parent) = path.parent() else {
        return;
    };

    if fs::create_dir_all(parent).is_err() {
        return;
    }

    let Ok(body) = serde_json::to_string(cache) else {
        return;
    };

    let _ = fs::write(path, body);
}

fn cache_path(session_key: &str) -> Option<PathBuf> {
    let safe_key: String = session_key
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' {
                character
            } else {
                '_'
            }
        })
        .collect();

    Some(cache_dir()?.join(format!("subagents-{safe_key}.json")))
}

#[cfg(test)]
mod tests {
    use super::SessionCache;

    #[test]
    fn rejects_a_cache_whose_agent_totals_counted_repeated_usage_rows() {
        let old = r#"{"transcript":{"scanned_bytes":10,"launched":1,"pending":[]},"agent_files":[{"path":"a.jsonl","scanned_bytes":10,"tokens":42}]}"#;

        assert!(serde_json::from_str::<SessionCache>(old).is_err());
    }
}
