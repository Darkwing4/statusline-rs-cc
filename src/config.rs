use serde::Deserialize;

use crate::segments::{
    cache_ttl::CacheTtl,
    context::Context,
    cwd::Cwd,
    effort::Effort,
    git::{GitBranch, GitDiff, GitError},
    idle_time::IdleTime,
    model::Model,
    rate_limits::RateLimit,
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
    Context(Context),
    CacheTtl(CacheTtl),
    Cwd(Cwd),
    Effort(Effort),
    GitBranch(GitBranch),
    GitDiff(GitDiff),
    GitError(GitError),
    IdleTime(IdleTime),
    Model(Model),
    RateLimit(RateLimit),
}

impl SegmentSpec {
    pub fn into_segment(self) -> Box<dyn Segment> {
        match self {
            SegmentSpec::Context(s) => Box::new(s),
            SegmentSpec::CacheTtl(s) => Box::new(s),
            SegmentSpec::Cwd(s) => Box::new(s),
            SegmentSpec::Effort(s) => Box::new(s),
            SegmentSpec::GitBranch(s) => Box::new(s),
            SegmentSpec::GitDiff(s) => Box::new(s),
            SegmentSpec::GitError(s) => Box::new(s),
            SegmentSpec::IdleTime(s) => {
                #[cfg(debug_assertions)]
                s.validate_debug();
                Box::new(s)
            }
            SegmentSpec::Model(s) => Box::new(s),
            SegmentSpec::RateLimit(s) => Box::new(s),
        }
    }
}

pub fn load_embedded() -> Result<RootConfig, ron::error::SpannedError> {
    ron::Options::default()
        .with_default_extension(ron::extensions::Extensions::UNWRAP_VARIANT_NEWTYPES)
        .from_str(EMBEDDED)
}
