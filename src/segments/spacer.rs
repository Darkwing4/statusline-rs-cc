use serde_json::Value;

pub use crate::config_schema::{Spacer, SpacerShape};
use crate::segments::{GitCache, Segment};

const BLANK: &str = "\u{2060}";

impl Segment for Spacer {
    fn render(&self, _json: &Value, _git: &mut GitCache) -> Option<String> {
        Some(BLANK.to_string())
    }

    fn standalone(&self) -> bool {
        self.shape == SpacerShape::BlankLine
    }

    fn breaks_line(&self) -> bool {
        self.shape == SpacerShape::LineBreak
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Spacer, SpacerShape, BLANK};
    use crate::ansi::visible_width;
    use crate::segments::{GitCache, Segment};

    fn render(shape: SpacerShape) -> (Option<String>, bool, bool) {
        let segment = Spacer { shape };
        let mut git = GitCache::new(String::new());

        (
            segment.render(&json!({}), &mut git),
            segment.standalone(),
            segment.breaks_line(),
        )
    }

    #[test]
    fn renders_text_that_takes_no_space_and_survives_trimming() {
        let (rendered, _, _) = render(SpacerShape::Gap);

        assert_eq!(rendered.as_deref(), Some(BLANK));
        assert_eq!(visible_width(BLANK), 0);
        assert!(!BLANK.trim().is_empty());
        assert!(!BLANK.split_whitespace().collect::<String>().is_empty());
    }

    #[test]
    fn follows_the_configured_shape() {
        assert_eq!(
            (render(SpacerShape::Gap).1, render(SpacerShape::Gap).2),
            (false, false)
        );
        assert_eq!(
            (
                render(SpacerShape::LineBreak).1,
                render(SpacerShape::LineBreak).2
            ),
            (false, true)
        );
        assert_eq!(
            (
                render(SpacerShape::BlankLine).1,
                render(SpacerShape::BlankLine).2
            ),
            (true, false)
        );
    }
}
