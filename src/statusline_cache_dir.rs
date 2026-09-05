use std::env;
use std::ffi::OsString;
use std::path::PathBuf;

pub fn cache_dir() -> Option<PathBuf> {
    if let Some(base) = non_empty_var("XDG_CACHE_HOME") {
        return Some(PathBuf::from(base).join("statusline"));
    }

    if let Some(base) = non_empty_var("LOCALAPPDATA") {
        return Some(PathBuf::from(base).join("statusline"));
    }

    let home = non_empty_var("HOME").or_else(|| non_empty_var("USERPROFILE"))?;

    Some(PathBuf::from(home).join(".cache").join("statusline"))
}

fn non_empty_var(name: &str) -> Option<OsString> {
    env::var_os(name).filter(|value| !value.is_empty())
}
