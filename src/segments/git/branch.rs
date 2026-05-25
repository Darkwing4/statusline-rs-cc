use serde_json::Value;

use super::tools::{detect_state, is_worktree, GitCache};
use crate::types::Color;
use crate::segments::Segment;

pub struct GitBranch {
    pub color: Color,
    pub state_color: Color,
    pub show_worktree: bool,
    pub show_ahead_behind: bool,
    pub show_state: bool,
}

impl Segment for GitBranch {
    fn render(&self, _json: &Value, git: &mut GitCache) -> Option<String> {
        let dir = git.dir()?.to_path_buf();
        let status = git.status()?;
        if status.branch.is_empty() {
            return None;
        }

        let mut branch_text = String::new();
        if self.show_worktree && is_worktree(&dir) {
            branch_text.push('⑂');
        }
        branch_text.push_str(&status.branch);

        let mut out = self.color.paint(&branch_text);

        if self.show_state {
            if let Some(state) = detect_state(&dir) {
                out.push(' ');
                out.push_str(&self.state_color.paint(&format!("[{}]", state)));
            }
        }

        if self.show_ahead_behind {
            if let Some(ab) = format_ahead_behind(status.ahead, status.behind) {
                out.push_str(&self.color.paint(&ab));
            }
        }

        Some(out)
    }
}

fn format_ahead_behind(ahead: u32, behind: u32) -> Option<String> {
    let mut parts = Vec::with_capacity(2);
    if ahead > 0 {
        parts.push(format!("↑{}", ahead));
    }
    if behind > 0 {
        parts.push(format!("↓{}", behind));
    }
    if parts.is_empty() {
        None
    } else {
        Some(format!("({})", parts.join(" ")))
    }
}
