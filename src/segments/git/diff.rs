use serde_json::Value;

use super::tools::GitCache;
use crate::types::Color;
use crate::segments::Segment;

pub struct GitDiff {
    pub modified_color: Color,
    pub untracked_color: Color,
    pub deleted_color: Color,
}

impl Segment for GitDiff {
    fn render(&self, _json: &Value, git: &mut GitCache) -> Option<String> {
        git.dir()?;
        let status = git.status()?;

        let mut parts = Vec::with_capacity(3);
        if status.modified > 0 {
            parts.push(self.modified_color.paint(&format!("~{}", status.modified)));
        }
        if status.untracked > 0 {
            parts.push(self.untracked_color.paint(&format!("+{}", status.untracked)));
        }
        if status.deleted > 0 {
            parts.push(self.deleted_color.paint(&format!("-{}", status.deleted)));
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" "))
        }
    }
}
