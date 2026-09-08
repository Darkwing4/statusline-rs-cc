mod line_overflow;
mod segment_wrapping;
mod terminal_width;

use serde_json::Value;

use self::line_overflow::{truncate_line, wrap_words};
use self::segment_wrapping::wrap_segments;
use self::terminal_width::terminal_width;

use crate::config_schema::Color;
use crate::segments::{GitCache, Overflow, Segment};
use crate::statusline_input;

pub struct Renderer {
    pub separator: String,
    pub separator_color: Color,
    pub segments: Vec<Box<dyn Segment>>,
}

impl Renderer {
    pub fn render(&self, json: &Value) -> String {
        let cwd = statusline_input::cwd(json).unwrap_or("").to_string();
        let mut git = GitCache::new(cwd);

        let sep = self.separator_color.paint(&self.separator);
        let width = terminal_width()
            .map(|cols| cols.saturating_sub(4))
            .filter(|max| *max > 0);

        let mut lines: Vec<String> = Vec::new();
        let mut main_parts: Vec<String> = Vec::new();

        for segment in &self.segments {
            if segment.breaks_line() {
                if !main_parts.is_empty() {
                    lines.push(main_block(&main_parts, &sep, width));
                    main_parts.clear();
                }

                continue;
            }

            let Some(rendered) = segment.render(json, &mut git) else {
                continue;
            };

            if rendered.is_empty() {
                continue;
            }

            if !segment.standalone() {
                main_parts.push(rendered);
                continue;
            }

            if lines.is_empty() || !main_parts.is_empty() {
                lines.push(main_block(&main_parts, &sep, width));
                main_parts.clear();
            }

            lines.push(match (width, segment.overflow()) {
                (Some(max), Overflow::Wrap) => wrap_words(&rendered, max),
                (Some(max), Overflow::Truncate) => truncate_line(&rendered, max),
                (None, _) => rendered,
            });
        }

        if lines.is_empty() || !main_parts.is_empty() {
            lines.push(main_block(&main_parts, &sep, width));
        }

        lines.join("\n")
    }
}

fn main_block(parts: &[String], sep: &str, width: Option<usize>) -> String {
    match width {
        Some(max) => wrap_segments(parts, sep, max),
        None => parts.join(sep),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::Renderer;
    use crate::ansi::visible_width;
    use crate::config_schema::Color;
    use crate::segments::spacer::{Spacer, SpacerShape};
    use crate::segments::{GitCache, Segment};

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
    fn renders_standalone_segments_on_their_own_lines_in_config_order() {
        let renderer = Renderer {
            separator: " ".to_string(),
            separator_color: Color::Gradient,
            segments: vec![
                segment("first", false),
                segment("first standalone", true),
                segment("second", false),
                segment("third", false),
                segment("second standalone", true),
            ],
        };

        assert_eq!(
            renderer.render(&serde_json::json!({})),
            "first\nfirst standalone\nsecond third\nsecond standalone"
        );
    }

    #[test]
    fn keeps_the_first_line_for_the_main_block_even_when_a_standalone_leads() {
        let renderer = Renderer {
            separator: " ".to_string(),
            separator_color: Color::Gradient,
            segments: vec![
                segment("first standalone", true),
                segment("main", false),
            ],
        };

        assert_eq!(
            renderer.render(&serde_json::json!({})),
            "\nfirst standalone\nmain"
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
    fn starts_a_new_line_after_a_line_break_without_leaving_a_blank_one() {
        let renderer = Renderer {
            separator: " ".to_string(),
            separator_color: Color::Gradient,
            segments: vec![
                Box::new(Spacer {
                    shape: SpacerShape::LineBreak,
                }),
                segment("first", false),
                Box::new(Spacer {
                    shape: SpacerShape::LineBreak,
                }),
                Box::new(Spacer {
                    shape: SpacerShape::LineBreak,
                }),
                segment("second", false),
                segment("third", false),
            ],
        };

        assert_eq!(
            renderer.render(&serde_json::json!({})),
            "first\nsecond third"
        );
    }

    #[test]
    fn keeps_a_blank_line_for_every_standalone_spacer() {
        let renderer = Renderer {
            separator: " ".to_string(),
            separator_color: Color::Gradient,
            segments: vec![
                segment("main", false),
                Box::new(Spacer {
                    shape: SpacerShape::BlankLine,
                }),
                Box::new(Spacer {
                    shape: SpacerShape::BlankLine,
                }),
                segment("below", true),
            ],
        };

        let output = renderer.render(&serde_json::json!({}));
        let lines: Vec<&str> = output.split('\n').collect();

        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0], "main");
        assert_eq!(visible_width(lines[1]), 0);
        assert_eq!(visible_width(lines[2]), 0);
        assert!(!lines[1].is_empty());
        assert!(!lines[2].is_empty());
        assert_eq!(lines[3], "below");
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
