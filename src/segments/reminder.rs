use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

pub use crate::config_schema::Reminder as ReminderSegment;
use crate::segments::single_line_text::sanitize;
use crate::segments::{GitCache, Segment};
use crate::statusline_reminder_store::{self, Reminder};

impl Segment for ReminderSegment {
    fn render(&self, _json: &Value, _git: &mut GitCache) -> Option<String> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;

        Some(
            self.color
                .paint(&self.body(&statusline_reminder_store::due(now))?),
        )
    }

    fn standalone(&self) -> bool {
        self.standalone
    }
}

impl ReminderSegment {
    fn body(&self, due: &[Reminder]) -> Option<String> {
        let joined = due
            .iter()
            .map(|reminder| reminder.text.as_str())
            .collect::<Vec<_>>()
            .join(&self.separator);

        let text = sanitize(&joined, self.max_chars);

        (!text.is_empty()).then(|| format!("{}{}", self.prefix, text))
    }
}

#[cfg(test)]
mod tests {
    use super::ReminderSegment;
    use crate::config_schema::Color;
    use crate::statusline_reminder_store::Reminder;

    fn segment() -> ReminderSegment {
        ReminderSegment {
            color: Color::Named(33),
            prefix: "⏰ ".to_string(),
            separator: " · ".to_string(),
            max_chars: 60,
            standalone: false,
        }
    }

    fn reminder(text: &str) -> Reminder {
        Reminder {
            text: text.to_string(),
            show_at: 0,
            expires_at: i64::MAX,
        }
    }

    #[test]
    fn joins_every_due_reminder_behind_one_prefix() {
        assert_eq!(
            segment().body(&[reminder("созвон"), reminder("забрать посылку")]),
            Some("⏰ созвон · забрать посылку".to_string())
        );
    }

    #[test]
    fn renders_nothing_when_no_reminder_is_due() {
        assert_eq!(segment().body(&[]), None);
        assert_eq!(segment().body(&[reminder(" ")]), None);
    }

    #[test]
    fn cuts_the_joined_text_to_max_chars() {
        let mut segment = segment();
        segment.max_chars = 7;

        assert_eq!(
            segment.body(&[reminder("созвон"), reminder("посылка")]),
            Some("⏰ созвон…".to_string())
        );
    }
}
