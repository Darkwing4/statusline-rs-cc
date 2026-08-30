use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::statusline_cache_dir::cache_dir;

#[derive(Default, Deserialize, Serialize)]
pub(super) struct InsightState {
    #[serde(default)]
    pub(super) scanned_bytes: u64,
    #[serde(default)]
    pub(super) turns_since_run: usize,
    #[serde(default)]
    pub(super) context: String,
    #[serde(default)]
    pub(super) fresh: String,
}

pub(super) fn load(base: &Path) -> InsightState {
    fs::read_to_string(state_path(base))
        .ok()
        .and_then(|body| serde_json::from_str(&body).ok())
        .unwrap_or_default()
}

pub(super) fn store(base: &Path, state: &InsightState) {
    let path = state_path(base);
    let Some(parent) = path.parent() else {
        return;
    };

    if fs::create_dir_all(parent).is_err() {
        return;
    }

    let Ok(body) = serde_json::to_string(state) else {
        return;
    };

    let _ = fs::write(path, body);
}

pub(super) fn base_path(fingerprint: &str, session_key: &str) -> Option<PathBuf> {
    Some(cache_dir()?.join(format!("insight-{}-{}", fingerprint, file_key(session_key))))
}

pub(super) fn state_path(base: &Path) -> PathBuf {
    with_suffix(base, "state.json")
}

pub(super) fn request_path(base: &Path) -> PathBuf {
    with_suffix(base, "request")
}

pub(super) fn result_path(base: &Path) -> PathBuf {
    with_suffix(base, "txt")
}

pub(super) fn attempt_path(base: &Path) -> PathBuf {
    with_suffix(base, "attempt")
}

fn with_suffix(base: &Path, suffix: &str) -> PathBuf {
    let mut name = base.as_os_str().to_os_string();
    name.push(".");
    name.push(suffix);

    PathBuf::from(name)
}

fn file_key(session_key: &str) -> String {
    session_key
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{file_key, request_path, result_path, state_path};

    #[test]
    fn derives_every_file_from_one_base_path() {
        let base = PathBuf::from("/cache/insight-abc-s1");

        assert_eq!(
            state_path(&base),
            PathBuf::from("/cache/insight-abc-s1.state.json")
        );
        assert_eq!(
            request_path(&base),
            PathBuf::from("/cache/insight-abc-s1.request")
        );
        assert_eq!(
            result_path(&base),
            PathBuf::from("/cache/insight-abc-s1.txt")
        );
    }

    #[test]
    fn keeps_the_session_out_of_the_directory_tree() {
        assert_eq!(file_key("../../etc"), "______etc");
    }
}
