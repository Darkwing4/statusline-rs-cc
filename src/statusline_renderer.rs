mod segment_wrapping;
mod terminal_width;

use serde_json::Value;

use self::segment_wrapping::wrap_segments;
use self::terminal_width::terminal_width;

use crate::segments::{GitCache, Segment};
use crate::statusline_input;
use crate::types::Color;

pub struct Renderer {
    pub separator: String,
    pub separator_color: Color,
    pub segments: Vec<Box<dyn Segment>>,
}

impl Renderer {
    pub fn render(&self, json: &Value) -> String {
        let cwd = statusline_input::cwd(json).unwrap_or("").to_string();
        let mut git = GitCache::new(cwd);

        let mut main_parts: Vec<String> = Vec::new();
        let mut tail_lines: Vec<String> = Vec::new();

        for segment in &self.segments {
            let Some(rendered) = segment.render(json, &mut git) else {
                continue;
            };

            if rendered.is_empty() {
                continue;
            }

            if segment.standalone() {
                tail_lines.push(rendered);
            } else {
                main_parts.push(rendered);
            }
        }

        let sep = self.separator_color.paint(&self.separator);

        let main_block = match terminal_width() {
            Some(cols) => {
                let max = cols.saturating_sub(4);
                if max == 0 {
                    main_parts.join(&sep)
                } else {
                    wrap_segments(&main_parts, &sep, max)
                }
            }
            None => main_parts.join(&sep),
        };

        let mut lines = vec![main_block];
        lines.extend(tail_lines);
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::Renderer;
    use crate::segments::{GitCache, Segment};
    use crate::types::Color;

    struct FixedSegment {
        output: Option<&'static str>,
        standalone: bool,
    }

    impl Segment for FixedSegment {
        fn render(&self, _json: &Value, _git: &mut GitCache) -> Option<String> {
            self.output.map(str::to_owned)
        }

        fn standalone(&self) -> bool {
            self.standalone
        }
    }

    fn segment(output: &'static str, standalone: bool) -> Box<dyn Segment> {
        Box::new(FixedSegment {
            output: Some(output),
            standalone,
        })
    }

    fn omitted_segment(standalone: bool) -> Box<dyn Segment> {
        Box::new(FixedSegment {
            output: None,
            standalone,
        })
    }

    #[test]
    fn renders_standalone_segments_on_separate_lines_after_main_line() {
        let renderer = Renderer {
            separator: " ".to_string(),
            separator_color: Color::Gradient,
            segments: vec![
                segment("first standalone", true),
                segment("main", false),
                segment("second standalone", true),
            ],
        };

        assert_eq!(
            renderer.render(&serde_json::json!({})),
            "main\nfirst standalone\nsecond standalone"
        );
    }

    #[test]
    fn renders_only_standalone_segments_on_separate_lines() {
        let renderer = Renderer {
            separator: " ".to_string(),
            separator_color: Color::Gradient,
            segments: vec![
                segment("first standalone", true),
                segment("second standalone", true),
            ],
        };

        assert_eq!(
            renderer.render(&serde_json::json!({})),
            "\nfirst standalone\nsecond standalone"
        );
    }

    #[test]
    fn skips_absent_and_empty_segments_without_extra_lines() {
        let renderer = Renderer {
            separator: " ".to_string(),
            separator_color: Color::Gradient,
            segments: vec![
                omitted_segment(false),
                segment("", false),
                omitted_segment(true),
                segment("", true),
            ],
        };

        assert_eq!(renderer.render(&serde_json::json!({})), "");
    }
}
