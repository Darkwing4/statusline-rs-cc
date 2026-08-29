use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::statusline_cache_dir::cache_dir;

#[derive(Deserialize, Serialize)]
pub struct Notice {
    pub text: String,
    #[serde(default)]
    pub expires_at: Option<i64>,
}

impl Notice {
    pub fn is_expired(&self, now: i64) -> bool {
        match self.expires_at {
            Some(expires_at) => now >= expires_at,
            None => false,
        }
    }

    pub fn remaining_seconds(&self, now: i64) -> Option<i64> {
        self.expires_at.map(|expires_at| (expires_at - now).max(0))
    }
}

pub fn load(session_key: &str) -> Option<Notice> {
    let body = fs::read_to_string(notice_path(session_key)?).ok()?;

    serde_json::from_str(&body).ok()
}

pub fn store(session_key: &str, notice: &Notice) -> Result<PathBuf, String> {
    let path = notice_path(session_key).ok_or("no cache directory for the notice")?;
    let parent = path.parent().ok_or("no cache directory for the notice")?;

    fs::create_dir_all(parent).map_err(|error| format!("{}: {}", parent.display(), error))?;

    let body = serde_json::to_string(notice).map_err(|error| error.to_string())?;
    let pending = path.with_extension("pending");

    fs::write(&pending, body).map_err(|error| format!("{}: {}", pending.display(), error))?;

    if let Err(error) = fs::rename(&pending, &path) {
        let _ = fs::remove_file(&pending);
        return Err(format!("{}: {}", path.display(), error));
    }

    Ok(path)
}

pub fn clear(session_key: &str) -> Result<(), String> {
    let path = notice_path(session_key).ok_or("no cache directory for the notice")?;

    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("{}: {}", path.display(), error)),
    }
}

fn notice_path(session_key: &str) -> Option<PathBuf> {
    Some(cache_dir()?.join(format!("notice-{}.json", file_key(session_key))))
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
    use super::{file_key, Notice};

    fn notice(expires_at: Option<i64>) -> Notice {
        Notice {
            text: "созвон 15:00".to_string(),
            expires_at,
        }
    }

    #[test]
    fn expires_once_the_deadline_is_reached() {
        assert!(!notice(Some(1_000)).is_expired(999));
        assert!(notice(Some(1_000)).is_expired(1_000));
        assert!(!notice(None).is_expired(i64::MAX));
    }

    #[test]
    fn reports_the_remaining_time_without_going_negative() {
        assert_eq!(notice(Some(1_000)).remaining_seconds(940), Some(60));
        assert_eq!(notice(Some(1_000)).remaining_seconds(1_200), Some(0));
        assert_eq!(notice(None).remaining_seconds(940), None);
    }

    #[test]
    fn keeps_the_file_name_free_of_path_separators() {
        assert_eq!(file_key("58eb9b9b-bdeb-42eb"), "58eb9b9b-bdeb-42eb");
        assert_eq!(file_key("../../etc/passwd"), "______etc_passwd");
    }
}
