use crate::ansi::visible_width;
use crate::types::RESET;

pub(super) fn wrap_words(text: &str, max: usize) -> String {
    text.lines()
        .map(|line| wrap_line(line, max))
        .collect::<Vec<_>>()
        .join("\n")
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

fn track_styles(styles: &mut String, word: &str) {
    let mut rest = word;

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
    use super::wrap_words;

    #[test]
    fn keeps_a_line_that_fits() {
        assert_eq!(wrap_words("one two", 7), "one two");
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
}
