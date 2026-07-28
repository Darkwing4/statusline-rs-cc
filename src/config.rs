pub use crate::config_schema::{RootConfig, SegmentSpec};
use crate::segments::Segment;

const EMBEDDED: &str = include_str!(concat!(env!("OUT_DIR"), "/embedded_config.ron"));

impl SegmentSpec {
    pub fn into_segment(self) -> Box<dyn Segment> {
        match self {
            SegmentSpec::Context(s) => Box::new(s),
            SegmentSpec::CacheTtl(s) => Box::new(s),
            SegmentSpec::ClaudeResourceUsage(s) => Box::new(s),
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
    crate::config_schema::parse(EMBEDDED)
}
