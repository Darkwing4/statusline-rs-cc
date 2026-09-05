pub mod background_command;
pub mod claude_resource_usage;
pub mod context_usage;
pub mod cwd;
pub mod effort;
pub mod git;
pub mod llm_answer;
pub mod llm_insight;
pub mod model;
pub mod my_last_prompt;
pub mod prompt_cache_ttl;
pub mod rate_limits;
pub mod reminder;
pub mod session_notice;
pub mod single_line_text;
pub mod spacer;
pub mod subagent_stats;
pub mod user_idle_time;
pub mod weather;

pub use git::GitCache;

use serde_json::Value;

pub(crate) fn json_field<'a>(json: &'a Value, pointer: &str) -> Option<&'a str> {
    json.pointer(pointer)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

#[derive(Clone, Copy)]
pub enum Overflow {
    Wrap,
    Truncate,
}

pub trait Segment {
    fn render(&self, json: &Value, git: &mut GitCache) -> Option<String>;

    fn standalone(&self) -> bool {
        false
    }

    fn overflow(&self) -> Overflow {
        Overflow::Wrap
    }
}
