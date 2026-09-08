use std::env;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub use crate::config_schema::{RootConfig, SegmentSpec};
use crate::segments::Segment;

const EMBEDDED: &str = include_str!(concat!(env!("OUT_DIR"), "/embedded_config.ron"));

#[derive(Debug)]
pub enum ConfigError {
    Read {
        path: PathBuf,
        source: io::Error,
    },
    ParseFile {
        path: PathBuf,
        source: ron::error::SpannedError,
    },
    ParseEmbedded {
        source: ron::error::SpannedError,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(
                    formatter,
                    "cannot read config '{}': {}",
                    path.display(),
                    source
                )
            }
            Self::ParseFile { path, source } => {
                write!(formatter, "invalid config '{}': {}", path.display(), source)
            }
            Self::ParseEmbedded { source } => {
                write!(formatter, "invalid embedded config: {}", source)
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::ParseFile { source, .. } | Self::ParseEmbedded { source } => Some(source),
        }
    }
}

impl SegmentSpec {
    pub fn into_segment(self) -> Box<dyn Segment> {
        match self {
            SegmentSpec::ClaudeResourceUsage(s) => Box::new(s),
            SegmentSpec::ContextUsage(s) => Box::new(s),
            SegmentSpec::Cwd(s) => Box::new(s),
            SegmentSpec::Effort(s) => Box::new(s),
            SegmentSpec::GitBranch(s) => Box::new(s),
            SegmentSpec::GitDiff(s) => Box::new(s),
            SegmentSpec::GitError(s) => Box::new(s),
            SegmentSpec::CommandOutput(s) => Box::new(s),
            SegmentSpec::LlmInsight(s) => Box::new(s),
            SegmentSpec::Model(s) => Box::new(s),
            SegmentSpec::MyLastPrompt(s) => Box::new(s),
            SegmentSpec::PromptCacheTtl(s) => Box::new(s),
            SegmentSpec::RateLimit(s) => Box::new(s),
            SegmentSpec::Reminder(s) => Box::new(s),
            SegmentSpec::SessionNotice(s) => Box::new(s),
            SegmentSpec::Spacer(s) => Box::new(s),
            SegmentSpec::SubagentStats(s) => Box::new(s),
            SegmentSpec::UserIdleTime(s) => Box::new(s),
            SegmentSpec::Weather(s) => Box::new(s),
        }
    }
}

pub fn load_embedded() -> Result<RootConfig, ron::error::SpannedError> {
    crate::config_schema::parse(EMBEDDED)
}

pub fn load_path(path: &Path) -> Result<RootConfig, ConfigError> {
    let body = fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;

    crate::config_schema::parse(&body).map_err(|source| ConfigError::ParseFile {
        path: path.to_path_buf(),
        source,
    })
}

pub fn load_default() -> Result<RootConfig, ConfigError> {
    let Some(path) = runtime_config_path() else {
        return load_embedded().map_err(|source| ConfigError::ParseEmbedded { source });
    };

    match fs::read_to_string(&path) {
        Ok(body) => crate::config_schema::parse(&body)
            .map_err(|source| ConfigError::ParseFile { path, source }),
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            load_embedded().map_err(|source| ConfigError::ParseEmbedded { source })
        }
        Err(source) => Err(ConfigError::Read { path, source }),
    }
}

fn runtime_config_path() -> Option<PathBuf> {
    let home = non_empty_var("HOME").or_else(|| non_empty_var("USERPROFILE"))?;

    Some(runtime_config_below(Path::new(&home)))
}

fn runtime_config_below(home: &Path) -> PathBuf {
    home.join(".claude").join("statusline").join("config.ron")
}

fn non_empty_var(name: &str) -> Option<OsString> {
    env::var_os(name).filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::runtime_config_below;

    #[test]
    fn resolves_the_runtime_config_below_the_home_directory() {
        assert_eq!(
            runtime_config_below(Path::new("/home/tester")),
            Path::new("/home/tester/.claude/statusline/config.ron")
        );
    }
}
