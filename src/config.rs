use serde::Deserialize;

use crate::segments::{
    cache_ttl::CacheTtl,
    context::Context,
    cwd::Cwd,
    git::{GitBranch, GitDiff, GitError},
    idle_time::IdleTime,
    rate_limits::{RateLimit, Window},
    Segment,
};
use crate::types::Color;

const EMBEDDED: &str = include_str!(concat!(env!("OUT_DIR"), "/embedded_config.ron"));

#[derive(Deserialize)]
pub struct RootConfig {
    pub separator: String,
    pub separator_color: Color,
    pub segments: Vec<SegmentSpec>,
}

#[derive(Deserialize)]
pub enum SegmentSpec {
    Context {
        color: Color,
        prefix: String,
        prefix_color: Color,
        suffix: String,
        suffix_color: Color,
    },
    CacheTtl {
        color: Color,
        prefix: String,
    },
    Cwd {
        color: Color,
    },
    GitBranch {
        color: Color,
        state_color: Color,
        show_worktree: bool,
        show_ahead_behind: bool,
        show_state: bool,
    },
    GitDiff {
        modified_color: Color,
        untracked_color: Color,
        deleted_color: Color,
    },
    GitError {
        color: Color,
        prefix: String,
    },
    IdleTime {
        color: Color,
        prefix: String,
        threshold_seconds: u64,
    },
    RateLimit {
        window: Window,
        prefix: String,
        low_color: Color,
        mid_color: Color,
        high_color: Color,
    },
}

impl SegmentSpec {
    pub fn into_segment(self) -> Box<dyn Segment> {
        match self {
            SegmentSpec::Context {
                color,
                prefix,
                prefix_color,
                suffix,
                suffix_color,
            } => Box::new(Context {
                color,
                prefix,
                prefix_color,
                suffix,
                suffix_color,
            }),
            SegmentSpec::CacheTtl { color, prefix } => Box::new(CacheTtl { color, prefix }),
            SegmentSpec::Cwd { color } => Box::new(Cwd { color }),
            SegmentSpec::GitBranch {
                color,
                state_color,
                show_worktree,
                show_ahead_behind,
                show_state,
            } => Box::new(GitBranch {
                color,
                state_color,
                show_worktree,
                show_ahead_behind,
                show_state,
            }),
            SegmentSpec::GitDiff {
                modified_color,
                untracked_color,
                deleted_color,
            } => Box::new(GitDiff {
                modified_color,
                untracked_color,
                deleted_color,
            }),
            SegmentSpec::GitError { color, prefix } => Box::new(GitError { color, prefix }),
            SegmentSpec::IdleTime {
                color,
                prefix,
                threshold_seconds,
            } => Box::new(IdleTime::new(color, prefix, threshold_seconds)),
            SegmentSpec::RateLimit {
                window,
                prefix,
                low_color,
                mid_color,
                high_color,
            } => Box::new(RateLimit {
                window,
                prefix,
                low_color,
                mid_color,
                high_color,
            }),
        }
    }
}

pub fn load_embedded() -> Result<RootConfig, ron::de::SpannedError> {
    ron::from_str(EMBEDDED)
}
