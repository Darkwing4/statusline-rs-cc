use serde_json::Value;

pub use crate::config_schema::Spacer;
use crate::segments::{GitCache, Segment};

const BLANK: &str = "\u{2060}";

impl Segment for Spacer {
    fn render(&self, _json: &Value, _git: &mut GitCache) -> Option<String> {
        Some(BLANK.to_string())
    }

    fn standalone(&self) -> bool {
        self.standalone
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Spacer, BLANK};
    use crate::ansi::visible_width;
    use crate::segments::{GitCache, Segment};

    fn render(standalone: bool) -> (Option<String>, bool) {
        let segment = Spacer { standalone };
        let mut git = GitCache::new(String::new());

        (segment.render(&json!({}), &mut git), segment.standalone())
    }

    #[test]
    fn renders_text_that_takes_no_space_and_survives_trimming() {
        let (rendered, _) = render(false);

        assert_eq!(rendered.as_deref(), Some(BLANK));
        assert_eq!(visible_width(BLANK), 0);
        assert!(!BLANK.trim().is_empty());
        assert!(!BLANK.split_whitespace().collect::<String>().is_empty());
    }

    #[test]
    fn follows_the_configured_standalone_flag() {
        assert!(!render(false).1);
        assert!(render(true).1);
    }
}
