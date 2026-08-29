use crate::ansi::strip_ansi;

const ELLIPSIS: char = '…';

pub(super) fn sanitize(text: &str, max_chars: usize) -> String {
    let plain = strip_ansi(text);
    let words = plain
        .split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|character| !character.is_control())
                .collect::<String>()
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    truncate(&words, max_chars)
}

fn truncate(text: &str, max_chars: usize) -> String {
    if max_chars == 0 || text.chars().count() <= max_chars {
        return text.to_string();
    }

    let kept = max_chars.saturating_sub(1);
    let mut shortened = text.chars().take(kept).collect::<String>();
    shortened.push(ELLIPSIS);

    shortened
}

#[cfg(test)]
mod tests {
    use super::sanitize;

    #[test]
    fn strips_ansi_and_control_characters_and_collapses_whitespace() {
        assert_eq!(
            sanitize("\u{1b}[31mбей\tкрепче\u{7}  и молча\u{1b}[0m", 80),
            "бей крепче и молча"
        );
    }

    #[test]
    fn truncates_to_max_chars_with_an_ellipsis() {
        assert_eq!(sanitize("абвгде", 4), "абв…");
        assert_eq!(sanitize("абвгде", 6), "абвгде");
        assert_eq!(sanitize("абвгде", 0), "абвгде");
    }
}
