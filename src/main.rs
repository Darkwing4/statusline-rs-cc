mod config;
mod segments;
mod statusline_renderer;
mod statusline_input;
mod transcript_tail_reader;
mod types;

use std::io::{self, Write};

use statusline_renderer::Renderer;

fn main() {
    let Some(json) = statusline_input::read() else {
        return;
    };

    let cfg = match config::load_embedded() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("statusline: config parse error: {}", e);
            return;
        }
    };

    let renderer = Renderer {
        separator: cfg.separator,
        separator_color: cfg.separator_color,
        segments: cfg.segments.into_iter().map(|s| s.into_segment()).collect(),
    };

    let line = renderer.render(&json);
    let _ = io::stdout().lock().write_all(line.as_bytes());
}
