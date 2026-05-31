use serde_json::Value;

use super::tools::GitCache;
use crate::types::Color;
use crate::segments::Segment;

pub struct GitError {
    pub color: Color,
    pub prefix: &'static str,
}

impl Segment for GitError {
    fn render(&self, _json: &Value, git: &mut GitCache) -> Option<String> {
        git.dir()?;
        let err = git.error()?;
        let text = strip_git_prefix(err);
        Some(self.color.paint(&format!("{}{}", self.prefix, text)))
    }
}

fn strip_git_prefix(msg: &str) -> &str {
    for prefix in ["fatal: ", "error: ", "warning: "] {
        if let Some(rest) = msg.strip_prefix(prefix) {
            return rest;
        }
    }
    msg
}
