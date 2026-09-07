use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub fn claude_config_dir() -> Option<PathBuf> {
    resolve(
        env::var_os("CLAUDE_CONFIG_DIR"),
        env::var_os("HOME").or_else(|| env::var_os("USERPROFILE")),
    )
}

pub fn claude_config_key() -> Option<String> {
    claude_config_dir().map(|dir| file_key(&dir))
}

fn resolve(configured: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    if let Some(dir) = configured.filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(dir));
    }

    let home = home.filter(|value| !value.is_empty())?;

    Some(PathBuf::from(home).join(".claude"))
}

fn file_key(dir: &Path) -> String {
    dir.to_string_lossy()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    use super::{file_key, resolve};

    #[test]
    fn prefers_claude_config_dir_over_home() {
        assert_eq!(
            resolve(Some(OsString::from("/home/ivan/.claude-2")), Some(OsString::from("/home/ivan"))),
            Some(PathBuf::from("/home/ivan/.claude-2"))
        );
        assert_eq!(
            resolve(Some(OsString::new()), Some(OsString::from("/home/ivan"))),
            Some(PathBuf::from("/home/ivan/.claude"))
        );
        assert_eq!(resolve(None, Some(OsString::from("/home/ivan"))), Some(PathBuf::from("/home/ivan/.claude")));
        assert_eq!(resolve(None, None), None);
    }

    #[test]
    fn keys_differ_per_config_dir_and_stay_file_safe() {
        let main = file_key(Path::new("/home/ivan/.claude"));
        let second = file_key(Path::new("/home/ivan/.claude-2"));

        assert_ne!(main, second);
        assert_eq!(second, "_home_ivan__claude-2");
        assert!(second.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }
}
