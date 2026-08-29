pub mod background_command;
pub mod cache_ttl;
pub mod claude_resource_usage;
pub mod context;
pub mod cwd;
pub mod duration_format;
pub mod effort;
pub mod git;
pub mod idle_time;
pub mod llm_message;
pub mod model;
pub mod notice;
pub mod rate_limits;
pub mod single_line_text;
pub mod subagent_stats;
pub mod weather;

pub use git::GitCache;

use serde_json::Value;

pub(crate) fn json_field<'a>(json: &'a Value, pointer: &str) -> Option<&'a str> {
    json.pointer(pointer)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

pub trait Segment {
    fn render(&self, json: &Value, git: &mut GitCache) -> Option<String>;

    fn standalone(&self) -> bool {
        false
    }
}
