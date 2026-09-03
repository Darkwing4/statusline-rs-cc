use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

pub use crate::config_schema::Notice as NoticeSegment;
use crate::duration_format::format_duration_padded;
use crate::segments::single_line_text::sanitize;
use crate::segments::{GitCache, Segment};
use crate::statusline_input::session_key;
use crate::statusline_notice_store::{self, Notice};

impl Segment for NoticeSegment {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let session = session_key(json)?;
        let notice = statusline_notice_store::load(&session)?;
        let now = now_seconds();

        if notice.is_expired(now) {
            let _ = statusline_notice_store::clear(&session);
            return None;
        }

        Some(self.color.paint(&self.body(&notice, now)?))
    }

    fn standalone(&self) -> bool {
        self.standalone
    }
}

impl NoticeSegment {
    fn body(&self, notice: &Notice, now: i64) -> Option<String> {
        let text = sanitize(&notice.text, self.max_chars);

        if text.is_empty() {
            return None;
        }

        let mut body = format!("{}{}", self.prefix, text);

        if self.show_remaining {
            if let Some(remaining) = notice.remaining_seconds(now) {
                body.push_str(&format!(" ({})", format_duration_padded(remaining)));
            }
        }

        Some(body)
    }
}

fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::NoticeSegment;
    use crate::config_schema::Color;
    use crate::statusline_notice_store::Notice;

    fn segment() -> NoticeSegment {
        NoticeSegment {
            color: Color::Named(93),
            prefix: "📌 ".to_string(),
            max_chars: 60,
            show_remaining: true,
            standalone: true,
        }
    }

    fn notice(text: &str, expires_at: Option<i64>) -> Notice {
        Notice {
            text: text.to_string(),
            expires_at,
            source: None,
        }
    }

    #[test]
    fn shows_the_text_with_the_time_left() {
        assert_eq!(
            segment().body(&notice("созвон 15:00", Some(1_252)), 1_000),
            Some("📌 созвон 15:00 (4m12s)".to_string())
        );
    }

    #[test]
    fn shows_a_notice_without_a_deadline_as_plain_text() {
        assert_eq!(
            segment().body(&notice("не забыть про бэкап", None), 1_000),
            Some("📌 не забыть про бэкап".to_string())
        );
    }

    #[test]
    fn hides_the_time_left_when_disabled() {
        let mut segment = segment();
        segment.show_remaining = false;

        assert_eq!(
            segment.body(&notice("созвон", Some(1_252)), 1_000),
            Some("📌 созвон".to_string())
        );
    }

    #[test]
    fn sanitizes_and_truncates_whatever_was_written_into_the_notice() {
        let mut segment = segment();
        segment.max_chars = 7;
        segment.show_remaining = false;

        assert_eq!(
            segment.body(&notice("\u{1b}[31mсбилди\nи запусти\u{7}", None), 1_000),
            Some("📌 сбилди…".to_string())
        );
    }

    #[test]
    fn skips_a_notice_that_holds_no_visible_text() {
        assert_eq!(segment().body(&notice("\u{1b}[0m", None), 1_000), None);
    }
}
