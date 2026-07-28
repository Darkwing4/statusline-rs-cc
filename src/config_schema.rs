use serde::Deserialize;

#[derive(Clone, Copy, Deserialize)]
pub enum Color {
    Named(u8),
    Rgb(u8, u8, u8),
    Gradient,
}

#[derive(Deserialize)]
pub struct Context {
    pub color: Color,
    pub prefix: String,
    pub prefix_color: Color,
    pub suffix: String,
    pub suffix_color: Color,
}

#[derive(Deserialize)]
pub struct CacheTtl {
    pub color: Color,
    pub prefix: String,
}

#[derive(Deserialize)]
pub struct ClaudeResourceUsage {
    pub color: Color,
    pub cpu_prefix: String,
    pub memory_prefix: String,
}

#[derive(Deserialize)]
pub struct Cwd {
    pub color: Color,
}

#[derive(Deserialize)]
pub struct Effort {
    pub color: Color,
    pub prefix: String,
}

#[derive(Deserialize)]
pub struct Model {
    pub color: Color,
    pub prefix: String,
}

#[derive(Deserialize)]
pub struct GitBranch {
    pub color: Color,
    pub state_color: Color,
    pub show_worktree: bool,
    pub show_ahead_behind: bool,
    pub show_state: bool,
}

#[derive(Deserialize)]
pub struct GitDiff {
    pub modified_color: Color,
    pub untracked_color: Color,
    pub deleted_color: Color,
}

#[derive(Deserialize)]
pub struct GitError {
    pub color: Color,
    pub text: String,
}

#[derive(Deserialize)]
pub struct IdleTime {
    pub color: Color,
    pub prefix: String,
    pub threshold_seconds: u64,
}

#[derive(Clone, Copy, Deserialize)]
pub enum Window {
    FiveHour,
    SevenDay,
}

#[derive(Clone, Copy, Deserialize)]
pub enum Style {
    Percent,
    Bar,
    BarPercent,
    Radial,
    RadialPercent,
}

#[derive(Clone, Copy, Deserialize)]
pub enum Fill {
    Used,
    Remaining,
}

#[derive(Clone, Copy, Deserialize)]
pub enum ColorMode {
    Steps,
    Gradient,
}

#[derive(Deserialize)]
pub struct RateLimit {
    pub window: Window,
    pub style: Style,
    pub fill: Fill,
    pub color_mode: ColorMode,
    pub prefix: String,
    pub low_color: Color,
    pub mid_color: Color,
    pub high_color: Color,
}

#[derive(Deserialize)]
pub enum SegmentSpec {
    Context(Context),
    CacheTtl(CacheTtl),
    ClaudeResourceUsage(ClaudeResourceUsage),
    Cwd(Cwd),
    Effort(Effort),
    GitBranch(GitBranch),
    GitDiff(GitDiff),
    GitError(GitError),
    IdleTime(IdleTime),
    Model(Model),
    RateLimit(RateLimit),
}

#[derive(Deserialize)]
pub struct RootConfig {
    pub separator: String,
    pub separator_color: Color,
    pub segments: Vec<SegmentSpec>,
}

pub fn parse(body: &str) -> Result<RootConfig, ron::error::SpannedError> {
    ron::Options::default()
        .with_default_extension(ron::extensions::Extensions::UNWRAP_VARIANT_NEWTYPES)
        .from_str(body)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_default_config() {
        assert!(parse(include_str!("../config/default.ron")).is_ok());
    }

    #[test]
    fn rejects_broken_syntax() {
        assert!(parse("(").is_err());
    }

    #[test]
    fn rejects_missing_required_field() {
        let body = r#"(
            separator_color: Named(90),
            segments: [],
        )"#;

        assert!(parse(body).is_err());
    }

    #[test]
    fn rejects_unknown_segment_spec() {
        let body = r#"(
            separator: " ",
            separator_color: Named(90),
            segments: [Unknown()],
        )"#;

        assert!(parse(body).is_err());
    }
}
