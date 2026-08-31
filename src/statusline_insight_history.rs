use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::statusline_cache_dir::cache_dir;

const FILE_NAME: &str = "insight-history.jsonl";
const RETENTION_SECONDS: i64 = 90 * 24 * 60 * 60;
const PRUNE_ABOVE_BYTES: u64 = 256 * 1024;

#[derive(Deserialize, Serialize)]
pub struct Entry {
    pub at: i64,
    pub session: String,
    pub cwd: String,
    pub prompt: String,
    pub input: String,
    pub answer: String,
}

pub fn append(entry: Entry) {
    let Some(path) = history_path() else {
        return;
    };

    let Some(parent) = path.parent() else {
        return;
    };

    if fs::create_dir_all(parent).is_err() {
        return;
    }

    prune_if_large(&path, entry.at);

    let Ok(line) = serde_json::to_string(&entry) else {
        return;
    };

    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) else {
        return;
    };

    let _ = writeln!(file, "{}", line);
}

pub fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0)
}

fn prune_if_large(path: &Path, now: i64) {
    let Ok(size) = fs::metadata(path).map(|meta| meta.len()) else {
        return;
    };

    if size < PRUNE_ABOVE_BYTES {
        return;
    }

    let Ok(body) = fs::read_to_string(path) else {
        return;
    };

    let kept = retained_lines(&body, now);
    let pending = path.with_extension("pending");

    if fs::write(&pending, kept).is_err() {
        let _ = fs::remove_file(&pending);
        return;
    }

    if fs::rename(&pending, path).is_err() {
        let _ = fs::remove_file(&pending);
    }
}

fn retained_lines(body: &str, now: i64) -> String {
    let mut kept = String::new();

    for line in body.lines() {
        if !is_expired(line, now) {
            kept.push_str(line);
            kept.push('\n');
        }
    }

    kept
}

fn is_expired(line: &str, now: i64) -> bool {
    match serde_json::from_str::<Entry>(line) {
        Ok(entry) => now - entry.at > RETENTION_SECONDS,
        Err(_) => true,
    }
}

fn history_path() -> Option<PathBuf> {
    Some(cache_dir()?.join(FILE_NAME))
}

#[cfg(test)]
mod tests {
    use super::{retained_lines, RETENTION_SECONDS};

    fn line(at: i64) -> String {
        format!(
            r#"{{"at":{},"session":"s","cwd":"/tmp","prompt":"p","input":"i","answer":"a"}}"#,
            at
        )
    }

    #[test]
    fn keeps_entries_from_the_last_three_months_and_drops_older_ones() {
        let now = 1_800_000_000;
        let body = format!(
            "{}\n{}\n{}\n",
            line(now - RETENTION_SECONDS - 1),
            line(now - RETENTION_SECONDS),
            line(now - 60)
        );

        let kept = retained_lines(&body, now);

        assert_eq!(kept.lines().count(), 2);
        assert!(kept.contains(&line(now - 60)));
        assert!(!kept.contains(&line(now - RETENTION_SECONDS - 1)));
    }

    #[test]
    fn drops_lines_it_cannot_read() {
        assert_eq!(retained_lines("not json\n", 1_800_000_000), "");
    }
}
