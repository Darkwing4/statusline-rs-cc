use serde_json::Value;

use crate::types::Color;
use crate::segments::{GitCache, Segment};

pub struct InputFromClaudeToStatusline {
    pub color: Color,
}

impl Segment for InputFromClaudeToStatusline {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let pretty = serde_json::to_string_pretty(json).ok()?;
        Some(self.color.paint(&pretty))
    }

    fn standalone(&self) -> bool {
        true
    }
}
