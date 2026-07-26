mod config;
mod config_schema;
mod segments;
mod statusline_input;
mod statusline_renderer;
mod transcript_tail_reader;
mod types;

use std::io::{self, Write};
use std::process::ExitCode;

use statusline_renderer::Renderer;

fn main() -> ExitCode {
    let cfg = match config::load_embedded() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("statusline: config parse error: {}", e);
            return ExitCode::FAILURE;
        }
    };

    let Some(json) = statusline_input::read() else {
        return ExitCode::SUCCESS;
    };

    let renderer = Renderer {
        separator: cfg.separator,
        separator_color: cfg.separator_color,
        segments: cfg.segments.into_iter().map(|s| s.into_segment()).collect(),
    };

    let line = renderer.render(&json);
    let _ = io::stdout().lock().write_all(line.as_bytes());
    ExitCode::SUCCESS
}
