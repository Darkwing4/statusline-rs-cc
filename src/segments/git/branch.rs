use serde_json::Value;

use super::tools::{detect_state, is_worktree, GitCache};
pub use crate::config_schema::GitBranch;
use crate::segments::Segment;

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

#[cfg(test)]
mod tests {
    use super::format_ahead_behind;

    #[test]
    fn omits_ahead_behind_when_counts_are_zero() {
        assert_eq!(format_ahead_behind(0, 0), None);
    }

    #[test]
    fn formats_ahead_count() {
        assert_eq!(format_ahead_behind(2, 0).as_deref(), Some("(↑2)"));
    }

    #[test]
    fn formats_behind_count() {
        assert_eq!(format_ahead_behind(0, 3).as_deref(), Some("(↓3)"));
    }

    #[test]
    fn formats_ahead_and_behind_counts() {
        assert_eq!(format_ahead_behind(2, 3).as_deref(), Some("(↑2 ↓3)"));
    }
}
