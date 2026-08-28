mod ansi;
mod config;
mod config_schema;
mod duration_format;
mod gradient;
mod iso8601;
#[cfg(target_os = "linux")]
mod process_stat;
mod segments;
mod statusline_cache_dir;
mod statusline_input;
mod statusline_renderer;
mod transcript_forward_reader;
mod transcript_record_probe;
mod transcript_tail_reader;

use std::io::{self, Write};
use std::process::ExitCode;

use statusline_renderer::Renderer;

fn main() -> ExitCode {
    if let Some(fingerprint) = refresh_request() {
        segments::background_command::refresh(&fingerprint);
        return ExitCode::SUCCESS;
    }

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

fn refresh_request() -> Option<String> {
    let mut args = std::env::args().skip(1);

    if args.next()? != segments::background_command::REFRESH_FLAG {
        return None;
    }

    args.next()
}
