use unicode_width::UnicodeWidthChar;

use crate::ansi::visible_width;
use crate::segments::single_line_text::ELLIPSIS;
use crate::types::RESET;

pub(super) fn wrap_words(text: &str, max: usize) -> String {
    text.lines()
        .map(|line| wrap_line(line, max))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn truncate_line(line: &str, max: usize) -> String {
    if visible_width(line) <= max {
        return line.to_string();
    }

    let room = max.saturating_sub(1);
    let mut kept = String::new();
    let mut kept_w = 0usize;
    let mut styles = String::new();
    let mut chars = line.chars();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            let escape = read_escape(ch, &mut chars);
            track_styles(&mut styles, &escape);
            kept.push_str(&escape);
            continue;
        }

        let ch_w = ch.width().unwrap_or(0);

        if kept_w + ch_w > room {
            break;
        }

        kept.push(ch);
        kept_w += ch_w;
    }

    kept.push(ELLIPSIS);

    if !styles.is_empty() {
        kept.push_str(RESET);
    }

    kept
}

fn read_escape(first: char, chars: &mut std::str::Chars<'_>) -> String {
    let mut escape = String::from(first);

    for ch in chars.by_ref() {
        escape.push(ch);

        if ch.is_ascii_alphabetic() {
            break;
        }
    }

    escape
}

fn wrap_line(line: &str, max: usize) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_w = 0usize;
    let mut styles = String::new();

    for word in line.split_whitespace() {
        let word_w = visible_width(word);

        if current.is_empty() {
            current.push_str(word);
            current_w = word_w;
        } else if current_w + 1 + word_w > max {
            if !styles.is_empty() {
                current.push_str(RESET);
            }

            lines.push(std::mem::take(&mut current));
            current.push_str(&styles);
            current.push_str(word);
            current_w = word_w;
        } else {
            current.push(' ');
            current.push_str(word);
            current_w += 1 + word_w;
        }

        track_styles(&mut styles, word);
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines.join("\n")
}

fn track_styles(styles: &mut String, text: &str) {
    let mut rest = text;

    while let Some(start) = rest.find("\u{1b}[") {
        let sequence = &rest[start..];
        let Some(end) = sequence.find(|c: char| c.is_ascii_alphabetic()) else {
            return;
        };

        let (escape, tail) = sequence.split_at(end + 1);

        if escape.ends_with('m') {
            if is_reset(escape) {
                styles.clear();
            } else {
                styles.push_str(escape);
            }
        }

        rest = tail;
    }
}

fn is_reset(escape: &str) -> bool {
    matches!(escape, "\u{1b}[m" | "\u{1b}[0m")
}

#[cfg(test)]
mod tests {
    use super::{truncate_line, wrap_words};

    #[test]
    fn keeps_a_line_that_fits() {
        assert_eq!(wrap_words("one two", 7), "one two");
        assert_eq!(truncate_line("one two", 7), "one two");
    }

    #[test]
    fn breaks_between_words_at_the_limit() {
        assert_eq!(wrap_words("one two three", 7), "one two\nthree");
    }

    #[test]
    fn reopens_the_colour_on_every_wrapped_line() {
        assert_eq!(
            wrap_words("\u{1b}[31mone two three\u{1b}[0m", 7),
            "\u{1b}[31mone two\u{1b}[0m\n\u{1b}[31mthree\u{1b}[0m"
        );
    }

    #[test]
    fn forgets_a_colour_closed_before_the_break() {
        assert_eq!(
            wrap_words("\u{1b}[31mred\u{1b}[0m plain more", 9),
            "\u{1b}[31mred\u{1b}[0m plain\nmore"
        );
    }

    #[test]
    fn measures_words_by_visible_width() {
        assert_eq!(wrap_words("界界 x", 4), "界界\nx");
        assert_eq!(
            wrap_words("\u{1b}[31m界界\u{1b}[0m x", 6),
            "\u{1b}[31m界界\u{1b}[0m x"
        );
    }

    #[test]
    fn keeps_a_word_longer_than_the_limit_whole() {
        assert_eq!(wrap_words("abcdefgh ij", 4), "abcdefgh\nij");
    }

    #[test]
    fn wraps_each_existing_line_on_its_own() {
        assert_eq!(wrap_words("one two\nthree four", 7), "one two\nthree\nfour");
    }

    #[test]
    fn cuts_at_the_limit_and_ends_with_an_ellipsis() {
        assert_eq!(truncate_line("one two three", 7), "one tw…");
        assert_eq!(truncate_line("界界界", 5), "界界…");
        assert_eq!(truncate_line("界界界", 4), "界…");
    }

    #[test]
    fn closes_the_colour_after_the_ellipsis() {
        assert_eq!(
            truncate_line("\u{1b}[31mone two three\u{1b}[0m", 7),
            "\u{1b}[31mone tw…\u{1b}[0m"
        );
        assert_eq!(
            truncate_line("\u{1b}[31mred\u{1b}[0m plain text", 6),
            "\u{1b}[31mred\u{1b}[0m p…"
        );
    }
}
