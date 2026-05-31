mod palette;
mod segments;
mod statusline_renderer;
mod statusline_input;
mod types;

use std::io::{self, Write};

use palette::{BRIGHT_RED, GRAY, GREEN, MAUVE, RED, TEAL, YELLOW};
use statusline_renderer::Renderer;
use types::Color;
use segments::{
    context::Context,
    cwd::Cwd,
    git::{GitBranch, GitDiff, GitError},
};

#[cfg(debug_assertions)]
use segments::debug::InputFromClaudeToStatusline;

fn main() {
    let Some(json) = statusline_input::read() else {
        return;
    };

    let renderer = Renderer {
        separator: " · ",
        separator_color: GRAY,
        segments: vec![
            Box::new(Context {
                color: Color::Gradient,
                prefix: "",
                prefix_color: MAUVE,
                suffix: "",
                suffix_color: MAUVE,
            }),
            Box::new(Cwd {
                color: TEAL,
            }),
            Box::new(GitBranch {
                color: GREEN,
                state_color: BRIGHT_RED,
                show_worktree: true,
                show_ahead_behind: true,
                show_state: true,
            }),
            Box::new(GitDiff {
                modified_color: YELLOW,
                untracked_color: GREEN,
                deleted_color: RED,
            }),
            Box::new(GitError {
                color: BRIGHT_RED,
                prefix: "git: ",
            }),
            #[cfg(debug_assertions)]
            Box::new(InputFromClaudeToStatusline {
                color: GRAY,
            }),
        ],
    };

    let line = renderer.render(&json);
    let _ = io::stdout().lock().write_all(line.as_bytes());
}
