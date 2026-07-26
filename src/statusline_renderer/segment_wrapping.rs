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
    let mut width = 0usize;
    let bytes = s.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == 0x1b {
            i += 1;
            while i < bytes.len() && !bytes[i].is_ascii_alphabetic() {
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
        } else {
            let ch = s[i..].chars().next().unwrap();
            width += 1;
            i += ch.len_utf8();
        }
    }

    width
}

#[cfg(test)]
mod tests {
    use super::{visible_width, wrap_segments};

    #[test]
    fn ignores_ansi_sequences_when_measuring_width() {
        assert_eq!(visible_width("\u{1b}[31mred\u{1b}[0m"), 3);
    }

    #[test]
    fn preserves_existing_unicode_scalar_width() {
        assert_eq!(visible_width("a界🙂"), 3);
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
