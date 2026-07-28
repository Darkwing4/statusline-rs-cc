pub mod cache_ttl;
pub mod claude_resource_usage;
pub mod context;
pub mod cwd;
pub mod effort;
pub mod git;
pub mod idle_time;
pub mod model;
pub mod rate_limits;

#[cfg(debug_assertions)]
pub mod debug;

pub use git::GitCache;

use serde_json::Value;

pub trait Segment {
    fn render(&self, json: &Value, git: &mut GitCache) -> Option<String>;

    fn standalone(&self) -> bool {
        false
    }
}
