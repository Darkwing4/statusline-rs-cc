pub mod context;
pub mod cwd;
pub mod git;

#[cfg(debug_assertions)]
pub mod debug;

pub use git::GitCache;

use serde_json::Value;

pub trait Item {
    fn render(&self, json: &Value, git: &mut GitCache) -> Option<String>;

    fn standalone(&self) -> bool {
        false
    }
}
