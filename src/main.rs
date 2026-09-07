mod ansi;
mod claude_config_dir;
mod config;
mod config_schema;
mod duration_format;
mod gradient;
mod iso8601;
#[cfg(target_os = "linux")]
mod private_file;
mod process_stat;
mod segment_catalog;
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
mod transcript_spoken_text;
mod transcript_tail_reader;

use std::io::{self, Write};
use std::path::Path;
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
        Command::Render { config_path } => render(config_path.as_deref()),
        Command::CheckConfig { path } => check_config(&path),
        Command::Schema => print_schema(),
        Command::Refresh {
            fingerprint,
            request_base,
        } => {
            segments::background_command::refresh(&fingerprint, request_base);
            ExitCode::SUCCESS
        }
        Command::RefreshUsage => {
            segments::rate_limits::refresh_usage();
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

fn render(config_path: Option<&Path>) -> ExitCode {
    let loaded = match config_path {
        Some(path) => config::load_path(path),
        None => config::load_default(),
    };

    let cfg = match loaded {
        Ok(c) => c,
        Err(e) => {
            eprintln!("statusline: {}", e);
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

fn check_config(path: &Path) -> ExitCode {
    match config::load_path(path) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("statusline: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn print_schema() -> ExitCode {
    match segment_catalog::to_json() {
        Ok(json) => {
            let _ = writeln!(io::stdout().lock(), "{}", json);
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("statusline: {}", message);
            ExitCode::FAILURE
        }
    }
}
