use std::borrow::Cow;

use unicode_width::UnicodeWidthStr;

pub(crate) fn visible_width(s: &str) -> usize {
    without_ansi(s).width()
}

#[cfg(any(debug_assertions, test))]
pub(crate) fn strip_ansi(s: &str) -> String {
    without_ansi(s).into_owned()
}

fn without_ansi(s: &str) -> Cow<'_, str> {
    if !s.as_bytes().contains(&0x1b) {
        return Cow::Borrowed(s);
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

    Cow::Owned(visible)
}

#[cfg(test)]
mod tests {
    use super::{strip_ansi, visible_width};

    #[test]
    fn preserves_plain_unicode_text() {
        assert_eq!(strip_ansi("a界🙂"), "a界🙂");
        assert_eq!(visible_width("a界🙂"), 5);
        assert_eq!(visible_width("👩‍💻"), 2);
    }

    #[test]
    fn removes_project_ansi_sequences() {
        let text = "\x1b[31mred\x1b[0m \x1b[38;2;1;2;3mrgb\x1b[0m";

        assert_eq!(strip_ansi(text), "red rgb");
        assert_eq!(visible_width(text), 7);
    }

    #[test]
    fn treats_alphabetic_terminators_as_escape_end() {
        let text = "a\x1b[2Kb";

        assert_eq!(strip_ansi(text), "ab");
        assert_eq!(visible_width(text), 2);
    }

    #[test]
    fn ignores_incomplete_escape_tail() {
        let text = "a\x1b[31";

        assert_eq!(strip_ansi(text), "a");
        assert_eq!(visible_width(text), 1);
    }
}
