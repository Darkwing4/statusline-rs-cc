use std::fs::File;
use std::process::{Command, Stdio};

use serde_json::Value;

use crate::segments::{GitCache, Segment};
use crate::statusline_input;
use crate::types::{Color, RESET};

pub struct Renderer {
    pub separator: &'static str,
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

        let sep = self.separator_color.paint(self.separator);
        let mut main_line = main_parts.join(&sep);

        if let Some(cols) = terminal_width() {
            let max = cols.saturating_sub(4);
            if max > 0 {
                main_line = truncate_visible(&main_line, max);
            }
        }

        let mut lines = vec![main_line];
        lines.extend(tail_lines);
        lines.join("\n")
    }
}

fn terminal_width() -> Option<usize> {
    let tty = File::open("/dev/tty").ok()?;
    let output = Command::new("stty")
        .arg("size")
        .stdin(Stdio::from(tty))
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    text.split_whitespace().nth(1)?.parse().ok()
}

enum AnsiPart<'a> {
    Ansi(&'a str),
    Visible(&'a str),
}

fn parse_ansi_parts(s: &str) -> Vec<AnsiPart<'_>> {
    let mut parts = Vec::new();
    let mut rest = s;

    while !rest.is_empty() {
        if rest.as_bytes()[0] == 0x1b {
            let end = rest
                .char_indices()
                .skip(1)
                .find(|(_, c)| c.is_ascii_alphabetic())
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(rest.len());
            let (ansi, tail) = rest.split_at(end);
            parts.push(AnsiPart::Ansi(ansi));
            rest = tail;
        } else {
            let char_len = rest.chars().next().unwrap().len_utf8();
            let (visible, tail) = rest.split_at(char_len);
            parts.push(AnsiPart::Visible(visible));
            rest = tail;
        }
    }

    parts
}

fn truncate_visible(s: &str, max: usize) -> String {
    let mut out = String::with_capacity(s.len());
    let mut visible_count = 0usize;
    let mut truncated = false;

    for part in parse_ansi_parts(s) {
        match part {
            AnsiPart::Ansi(esc) => out.push_str(esc),
            AnsiPart::Visible(c) => {
                if visible_count >= max {
                    truncated = true;
                    break;
                }
                out.push_str(c);
                visible_count += 1;
            }
        }
    }

    if truncated {
        out.push_str(RESET);
    }

    out
}
