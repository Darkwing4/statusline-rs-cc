use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::Value;

use crate::claude_config_dir::claude_config_key;
use crate::private_file;
use crate::segments::background_command::{age_seconds, is_expired};
use crate::statusline_cache_dir::cache_dir;
use crate::statusline_cli::USAGE_REFRESH_FLAG;

const FILE_STEM: &str = "claude-usage";

pub(super) fn snapshot(ttl_seconds: u64) -> Option<Value> {
    let paths = CachePaths::resolve()?;
    let cached = fs::read_to_string(&paths.result).ok();

    if paths.needs_refresh(ttl_seconds) {
        paths.mark_attempt();
        spawn_refresh();
    }

    serde_json::from_str(&cached?).ok()
}

pub(super) fn store(body: &str) {
    let Some(paths) = CachePaths::resolve() else {
        return;
    };

    if !paths.ensure_dir() {
        return;
    }

    if private_file::write(&paths.pending, body).is_err() {
        return;
    }

    if fs::rename(&paths.pending, &paths.result).is_err() {
        let _ = fs::remove_file(&paths.pending);
    }
}

struct CachePaths {
    result: PathBuf,
    attempt: PathBuf,
    pending: PathBuf,
}

impl CachePaths {
    fn resolve() -> Option<Self> {
        let dir = cache_dir()?;
        let stem = format!("{FILE_STEM}-{}", claude_config_key()?);

        Some(CachePaths {
            result: dir.join(format!("{stem}.json")),
            attempt: dir.join(format!("{stem}.attempt")),
            pending: dir.join(format!("{stem}.pending")),
        })
    }

    fn needs_refresh(&self, ttl_seconds: u64) -> bool {
        is_expired(age_seconds(&self.result), ttl_seconds)
            && is_expired(age_seconds(&self.attempt), ttl_seconds)
    }

    fn mark_attempt(&self) {
        if !self.ensure_dir() {
            return;
        }

        let _ = fs::write(&self.attempt, b"");
    }

    fn ensure_dir(&self) -> bool {
        let Some(parent) = self.result.parent() else {
            return false;
        };

        fs::create_dir_all(parent).is_ok()
    }
}

fn spawn_refresh() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };

    let _ = Command::new(exe)
        .arg(USAGE_REFRESH_FLAG)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}
