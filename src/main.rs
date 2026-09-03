mod ansi;
mod config;
mod config_schema;
mod duration_format;
mod gradient;
mod iso8601;
#[cfg(target_os = "linux")]
mod private_file;
mod process_stat;
mod segments;
mod statusline_cache_dir;
mod statusline_cli;
mod statusline_hook;
mod statusline_input;
mod statusline_notice_store;
mod statusline_reminder_store;
mod statusline_renderer;
mod transcript_forward_reader;
mod transcript_record_probe;
mod transcript_tail_reader;

use std::io::{self, Write};
use std::process::ExitCode;

use statusline_cli::Command;
use statusline_renderer::Renderer;

fn main() -> ExitCode {
    let command = match statusline_cli::parse(std::env::args().skip(1)) {
        Ok(command) => command,
        Err(message) => {
            eprintln!("statusline: {}", message);
            return ExitCode::FAILURE;
        }
    };

    match command {
        Command::Render => render(),
        Command::Refresh {
            fingerprint,
            request_base,
        } => {
            segments::background_command::refresh(&fingerprint, request_base);
            ExitCode::SUCCESS
        }
        written => match statusline_cli::apply(written) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("statusline: {}", message);
                ExitCode::FAILURE
            }
        },
    }
}

fn render() -> ExitCode {
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
