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
