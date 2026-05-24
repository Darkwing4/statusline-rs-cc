use std::env;

use serde_json::Value;

use crate::types::Color;
use crate::items::{GitCache, Item};
use crate::statusline_input;

pub struct Cwd {
    pub color: Color,
}

impl Item for Cwd {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let cwd = statusline_input::cwd(json)?;
        Some(self.color.paint(&shorten_home(cwd)))
    }
}

fn shorten_home(path: &str) -> String {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_default();

    if !home.is_empty() && path.starts_with(&home) {
        format!("~{}", &path[home.len()..])
    } else {
        path.to_string()
    }
}
