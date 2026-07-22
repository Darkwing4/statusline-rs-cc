use serde::Deserialize;
use serde_json::Value;

use super::tools::GitCache;
use crate::types::Color;
use crate::segments::Segment;

#[derive(Deserialize)]
pub struct GitError {
    pub color: Color,
    pub text: String,
}

impl Segment for GitError {
    fn render(&self, _json: &Value, git: &mut GitCache) -> Option<String> {
        git.error()?;
        Some(self.color.paint(&self.text))
    }
}
