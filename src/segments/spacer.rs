use serde_json::Value;

pub use crate::config_schema::Spacer;
use crate::segments::{GitCache, Segment};

const BLANK: &str = " ";

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

    use super::Spacer;
    use crate::segments::{GitCache, Segment};

    fn render(standalone: bool) -> (Option<String>, bool) {
        let segment = Spacer { standalone };
        let mut git = GitCache::new(String::new());

        (segment.render(&json!({}), &mut git), segment.standalone())
    }

    #[test]
    fn renders_blank_text_the_renderer_keeps() {
        let (rendered, _) = render(false);

        assert_eq!(rendered.as_deref(), Some(" "));
    }

    #[test]
    fn follows_the_configured_standalone_flag() {
        assert!(!render(false).1);
        assert!(render(true).1);
    }
}
