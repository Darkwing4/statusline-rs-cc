use std::fs;
use std::path::{Path, PathBuf};

use crate::private_file;
use crate::statusline_cache_dir::cache_dir;
use crate::transcript_token_tally::TranscriptTally;

pub(super) fn load(session_key: &str) -> (Vec<TranscriptTally>, Option<PathBuf>) {
    let Some(path) = cache_path(session_key) else {
        return (Vec::new(), None);
    };

    let tallies = fs::read_to_string(&path)
        .ok()
        .and_then(|body| serde_json::from_str(&body).ok())
        .unwrap_or_default();

    (tallies, Some(path))
}

pub(super) fn store(path: &Path, tallies: &[TranscriptTally]) {
    let Some(parent) = path.parent() else {
        return;
    };

    if fs::create_dir_all(parent).is_err() {
        return;
    }

    let Ok(body) = serde_json::to_string(tallies) else {
        return;
    };

    let _ = private_file::write(path, body);
}

fn cache_path(session_key: &str) -> Option<PathBuf> {
    Some(cache_dir()?.join(format!("tokens-by-model-{}.json", file_key(session_key))))
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
