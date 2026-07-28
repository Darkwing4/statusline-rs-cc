use serde::{Deserialize, Deserializer};

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
    #[serde(
        default = "default_gradient_midpoint_percentage",
        deserialize_with = "deserialize_gradient_midpoint_percentage"
    )]
    pub gradient_midpoint_percentage: f64,
    pub prefix: String,
    pub low_color: Color,
    pub mid_color: Color,
    pub high_color: Color,
}

fn default_gradient_midpoint_percentage() -> f64 {
    50.0
}

fn deserialize_gradient_midpoint_percentage<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = f64::deserialize(deserializer)?;
    if value.is_finite() && value > 0.0 && value < 100.0 {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(
            "gradient_midpoint_percentage must be finite, greater than 0, and less than 100",
        ))
    }
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
    use ron::extensions::Extensions;

    use super::{parse, RateLimit};

    fn parse_rate_limit(body: &str) -> Result<RateLimit, ron::error::SpannedError> {
        ron::Options::default()
            .with_default_extension(Extensions::UNWRAP_VARIANT_NEWTYPES)
            .from_str(body)
    }

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

    #[test]
    fn defaults_rate_limit_gradient_midpoint_for_existing_configs() {
        let rate_limit = parse_rate_limit(
            r#"(
                window: FiveHour,
                style: Bar,
                fill: Remaining,
                color_mode: Gradient,
                prefix: "{t}h ",
                low_color: Rgb(0, 0, 0),
                mid_color: Rgb(100, 100, 100),
                high_color: Rgb(200, 200, 200),
            )"#,
        )
        .expect("legacy RateLimit config should parse");

        assert_eq!(rate_limit.gradient_midpoint_percentage, 50.0);
    }

    #[test]
    fn rejects_invalid_rate_limit_gradient_midpoint() {
        let body = r#"(
            window: SevenDay,
            style: Percent,
            fill: Used,
            color_mode: Gradient,
            gradient_midpoint_percentage: 100.0,
            prefix: "{t}d ",
            low_color: Rgb(0, 0, 0),
            mid_color: Rgb(100, 100, 100),
            high_color: Rgb(200, 200, 200),
        )"#;

        assert!(parse_rate_limit(body).is_err());
    }
}
