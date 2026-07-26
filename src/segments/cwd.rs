use std::{env, path::Path};

use serde_json::Value;

pub use crate::config_schema::Cwd;
use crate::segments::{GitCache, Segment};
use crate::statusline_input;

impl Segment for Cwd {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let cwd = statusline_input::cwd(json)?;
        Some(self.color.paint(&shorten_home(cwd)))
    }
}

fn shorten_home(path: &str) -> String {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_default();

    shorten_home_path(path, &home)
}

fn shorten_home_path(path: &str, home: &str) -> String {
    if !home.is_empty() && path.starts_with(home) && Path::new(path).starts_with(Path::new(home)) {
        format!("~{}", &path[home.len()..])
    } else {
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::shorten_home_path;

    #[test]
    fn shortens_home_and_descendant_paths() {
        assert_eq!(shorten_home_path("/home/ivan", "/home/ivan"), "~");
        assert_eq!(
            shorten_home_path("/home/ivan/project", "/home/ivan"),
            "~/project"
        );
    }

    #[test]
    fn keeps_path_with_matching_text_prefix() {
        assert_eq!(
            shorten_home_path("/home/ivan-project", "/home/ivan"),
            "/home/ivan-project"
        );
    }
}
