use unicode_width::UnicodeWidthStr;

pub(super) fn wrap_segments(parts: &[String], sep: &str, max: usize) -> String {
    if parts.is_empty() {
        return String::new();
    }

    let sep_w = visible_width(sep);
    let widths: Vec<usize> = parts.iter().map(|p| visible_width(p)).collect();

    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_w = 0usize;

    for (i, part) in parts.iter().enumerate() {
        let pw = widths[i];

        if current.is_empty() {
            current.push_str(part);
            current_w = pw;
            continue;
        }

        let projected = current_w + sep_w + pw;
        if projected > max {
            lines.push(std::mem::take(&mut current));
            current.push_str(part);
            current_w = pw;
        } else {
            current.push_str(sep);
            current.push_str(part);
            current_w = projected;
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines.join("\n")
}

fn visible_width(s: &str) -> usize {
    if !s.as_bytes().contains(&0x1b) {
        return s.width();
    }

    let mut visible = String::with_capacity(s.len());
    let mut chars = s.chars();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            for escape_char in chars.by_ref() {
                if escape_char.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            visible.push(ch);
        }
    }

    visible.width()
}

#[cfg(test)]
mod tests {
    use super::{visible_width, wrap_segments};

    #[test]
    fn ignores_ansi_sequences_when_measuring_width() {
        assert_eq!(visible_width("\u{1b}[31mred\u{1b}[0m"), 3);
        assert_eq!(visible_width("\u{1b}[31ma界🙂\u{1b}[0m"), 5);
        assert_eq!(visible_width("\u{1b}[31m👩‍💻\u{1b}[0m"), 2);
    }

    #[test]
    fn measures_unicode_display_width() {
        assert_eq!(visible_width("a界🙂"), 5);
        assert_eq!(visible_width("👩‍💻"), 2);
    }

    #[test]
    fn preserves_unicode_around_ansi_sequences() {
        assert_eq!(visible_width("界\u{1b}[31m🙂\u{1b}[0mé"), 5);
    }

    #[test]
    fn handles_incomplete_ansi_sequence() {
        assert_eq!(visible_width("界\u{1b}[31"), 2);
    }

    #[test]
    fn joins_segments_that_fit() {
        let parts = vec!["one".to_string(), "two".to_string()];

        assert_eq!(wrap_segments(&parts, " | ", 9), "one | two");
    }

    #[test]
    fn wraps_before_a_segment_that_exceeds_the_limit() {
        let parts = vec!["one".to_string(), "two".to_string()];

        assert_eq!(wrap_segments(&parts, " | ", 8), "one\ntwo");
    }

    #[test]
    fn wraps_using_unicode_display_width() {
        let parts = vec!["a界🙂".to_string(), "x".to_string()];

        assert_eq!(wrap_segments(&parts, " | ", 8), "a界🙂\nx");
    }

    #[test]
    fn measures_ansi_separator_by_visible_width() {
        let parts = vec!["one".to_string(), "two".to_string()];
        let separator = "\u{1b}[90m | \u{1b}[0m";

        assert_eq!(
            wrap_segments(&parts, separator, 9),
            format!("one{separator}two")
        );
    }

    #[test]
    fn returns_empty_output_for_empty_segments() {
        assert_eq!(wrap_segments(&[], " | ", 10), "");
    }
}
