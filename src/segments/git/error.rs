use serde_json::Value;

use super::tools::GitCache;
pub use crate::config_schema::GitError;
use crate::segments::Segment;

impl Segment for GitError {
    fn render(&self, _json: &Value, git: &mut GitCache) -> Option<String> {
        git.error()?;
        Some(self.color.paint(&self.text))
    }
}
