use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::statusline_cache_dir::cache_dir;

const FILE_NAME: &str = "reminders.json";

#[derive(Clone, Deserialize, Serialize)]
pub struct Reminder {
    pub text: String,
    pub show_at: i64,
    pub expires_at: i64,
}

impl Reminder {
    pub fn is_due(&self, now: i64) -> bool {
        now >= self.show_at
    }

    pub fn is_expired(&self, now: i64) -> bool {
        now >= self.expires_at
    }
}

#[derive(Default, Deserialize, Serialize)]
struct ReminderFile {
    #[serde(default)]
    reminders: Vec<Reminder>,
}

pub fn add(reminder: Reminder, now: i64) -> Result<(), String> {
    let mut kept = live_reminders(&load(), now);
    kept.push(reminder);

    store(&kept)
}

pub fn due(now: i64) -> Vec<Reminder> {
    let stored = load();
    let live = live_reminders(&stored, now);

    if live.len() != stored.len() {
        let _ = store(&live);
    }

    live.into_iter()
        .filter(|reminder| reminder.is_due(now))
        .collect()
}

pub fn clear(due_only: bool, now: i64) -> Result<usize, String> {
    let stored = load();
    let kept = keep_after_clear(live_reminders(&stored, now), due_only, now);
    let removed = stored.len() - kept.len();

    if removed > 0 {
        store(&kept)?;
    }

    Ok(removed)
}

fn keep_after_clear(live: Vec<Reminder>, due_only: bool, now: i64) -> Vec<Reminder> {
    if !due_only {
        return Vec::new();
    }

    live.into_iter()
        .filter(|reminder| !reminder.is_due(now))
        .collect()
}

fn live_reminders(reminders: &[Reminder], now: i64) -> Vec<Reminder> {
    let mut live: Vec<Reminder> = reminders
        .iter()
        .filter(|reminder| !reminder.is_expired(now))
        .cloned()
        .collect();

    live.sort_by_key(|reminder| reminder.show_at);

    live
}

fn load() -> Vec<Reminder> {
    let Some(path) = reminder_path() else {
        return Vec::new();
    };

    let Ok(body) = fs::read_to_string(path) else {
        return Vec::new();
    };

    serde_json::from_str::<ReminderFile>(&body)
        .map(|file| file.reminders)
        .unwrap_or_default()
}

fn store(reminders: &[Reminder]) -> Result<(), String> {
    let path = reminder_path().ok_or("no cache directory for reminders")?;
    let parent = path.parent().ok_or("no cache directory for reminders")?;

    fs::create_dir_all(parent).map_err(|error| format!("{}: {}", parent.display(), error))?;

    let file = ReminderFile {
        reminders: reminders.to_vec(),
    };
    let body = serde_json::to_string(&file).map_err(|error| error.to_string())?;
    let pending = path.with_extension("pending");

    fs::write(&pending, body).map_err(|error| format!("{}: {}", pending.display(), error))?;

    if let Err(error) = fs::rename(&pending, &path) {
        let _ = fs::remove_file(&pending);
        return Err(format!("{}: {}", path.display(), error));
    }

    Ok(())
}

fn reminder_path() -> Option<PathBuf> {
    Some(cache_dir()?.join(FILE_NAME))
}

#[cfg(test)]
mod tests {
    use super::{keep_after_clear, live_reminders, Reminder};

    fn reminder(text: &str, show_at: i64, expires_at: i64) -> Reminder {
        Reminder {
            text: text.to_string(),
            show_at,
            expires_at,
        }
    }

    #[test]
    fn drops_expired_reminders_and_orders_the_rest_by_their_time() {
        let stored = vec![
            reminder("later", 2_000, 5_000),
            reminder("gone", 100, 900),
            reminder("now", 1_000, 5_000),
        ];

        let live = live_reminders(&stored, 1_000);
        let texts: Vec<&str> = live.iter().map(|reminder| reminder.text.as_str()).collect();

        assert_eq!(texts, ["now", "later"]);
    }

    #[test]
    fn clearing_the_due_ones_leaves_the_reminders_that_have_not_fired_yet() {
        let live = vec![
            reminder("fired", 900, 5_000),
            reminder("waiting", 2_000, 5_000),
        ];

        let kept = keep_after_clear(live.clone(), true, 1_000);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].text, "waiting");

        assert!(keep_after_clear(live, false, 1_000).is_empty());
    }

    #[test]
    fn becomes_due_at_its_time_and_expires_at_its_deadline() {
        let reminder = reminder("созвон", 1_000, 2_000);

        assert!(!reminder.is_due(999));
        assert!(reminder.is_due(1_000));
        assert!(!reminder.is_expired(1_999));
        assert!(reminder.is_expired(2_000));
    }
}
