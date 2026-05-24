mod items;
mod statusline_renderer;
mod statusline_input;
mod types;

use std::io::{self, Write};

use statusline_renderer::Renderer;
use types::Color;
use items::{
    context::Context,
    cwd::Cwd,
    git::{GitBranch, GitDiff},
};

#[cfg(debug_assertions)]
use items::debug::InputFromClaudeToStatusline;

fn main() {
    let Some(json) = statusline_input::read() else {
        return;
    };

    let renderer = Renderer {
        separator: " · ",
        separator_color: Color::Named(90),
        items: vec![
            Box::new(Context {
                color: Color::Gradient,
                prefix: "",
                prefix_color: Color::Rgb(180, 142, 173),
                suffix: "",
                suffix_color: Color::Rgb(180, 142, 173),
            }),
            Box::new(Cwd {
                color: Color::Rgb(95, 175, 175),
            }),
            Box::new(GitBranch {
                color: Color::Named(32),
                state_color: Color::Named(91),
                show_worktree: true,
                show_ahead_behind: true,
                show_state: true,
            }),
            Box::new(GitDiff {
                modified_color: Color::Named(33),
                untracked_color: Color::Named(32),
                deleted_color: Color::Named(31),
            }),
            #[cfg(debug_assertions)]
            Box::new(InputFromClaudeToStatusline {
                color: Color::Named(90),
            }),
        ],
    };

    let line = renderer.render(&json);
    let _ = io::stdout().lock().write_all(line.as_bytes());
}
