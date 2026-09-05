use serde::{Deserialize, Deserializer};

#[derive(Clone, Copy, Deserialize)]
pub enum Color {
    Named(u8),
    Rgb(u8, u8, u8),
    Gradient,
}

pub(crate) const RESET: &str = "\x1b[0m";

impl Color {
    pub fn paint(&self, body: &str) -> String {
        match self {
            Color::Gradient => body.to_string(),
            Color::Named(code) => format!("\x1b[{}m{}{}", code, body, RESET),
            Color::Rgb(r, g, b) => format!("\x1b[38;2;{};{};{}m{}{}", r, g, b, body, RESET),
        }
    }
}

/// How much of the context window is already filled.
#[derive(Deserialize)]
pub struct ContextUsage {
    pub color: Color,
    pub prefix: String,
    pub prefix_color: Color,
    pub suffix: String,
    pub suffix_color: Color,
}

/// How long the prompt cache stays warm before the next request pays to build it again.
#[derive(Deserialize)]
pub struct PromptCacheTtl {
    pub color: Color,
    pub prefix: String,
}

/// How much CPU and memory the Claude Code process tree is eating right now (Linux only).
#[derive(Deserialize)]
pub struct ClaudeResourceUsage {
    pub color: Color,
    pub cpu_prefix: String,
    pub memory_prefix: String,
}

/// The directory Claude Code is working in, with your home folder shortened to a tilde.
#[derive(Deserialize)]
pub struct Cwd {
    pub color: Color,
}

/// Nothing at all: a blank line of its own, or a gap between two neighbours.
#[derive(Deserialize)]
pub struct Spacer {
    pub standalone: bool,
}

/// How hard the model is currently set to think.
#[derive(Deserialize)]
pub struct Effort {
    pub color: Color,
    pub prefix: String,
}

/// The model Claude Code is answering with.
#[derive(Deserialize)]
pub struct Model {
    pub color: Color,
    pub prefix: String,
    #[serde(default, deserialize_with = "deserialize_replacements")]
    pub replacements: Vec<(String, String)>,
}

fn deserialize_replacements<'de, D>(deserializer: D) -> Result<Vec<(String, String)>, D::Error>
where
    D: Deserializer<'de>,
{
    let pairs = Vec::<(String, String)>::deserialize(deserializer)?;

    if pairs.iter().any(|(from, _)| from.is_empty()) {
        return Err(serde::de::Error::custom(
            "model replacements must not search for an empty string",
        ));
    }

    Ok(pairs)
}

/// The branch you are on, what git is in the middle of, and how far you drifted from upstream.
#[derive(Deserialize)]
pub struct GitBranch {
    pub color: Color,
    pub state_color: Color,
    pub show_worktree: bool,
    pub show_ahead_behind: bool,
    pub show_state: bool,
}

/// How many files you changed, added, and deleted since the last commit.
#[derive(Deserialize)]
pub struct GitDiff {
    pub modified_color: Color,
    pub untracked_color: Color,
    pub deleted_color: Color,
}

/// A short marker for when the current directory is not a git repository at all.
#[derive(Deserialize)]
pub struct GitError {
    pub color: Color,
    pub text: String,
}

/// How long it has been since you last typed something to Claude Code.
#[derive(Deserialize)]
pub struct UserIdleTime {
    pub color: Color,
    pub prefix: String,
    pub threshold_seconds: u64,
}

/// How many subagents are running, how long the oldest one has been at it, and the tokens they burned.
#[derive(Deserialize)]
pub struct SubagentStats {
    pub color: Color,
    pub active_color: Color,
    pub stall_color: Color,
    pub prefix: String,
    pub stall_marker: String,
    pub stall_seconds: u64,
    pub show_tokens: bool,
}

/// The last line an external command prints for a fixed prompt, re-asked once its TTL runs out.
#[derive(Deserialize)]
pub struct LlmAnswer {
    pub color: Color,
    pub prefix: String,
    pub command: String,
    pub args: Vec<String>,
    pub prompt: String,
    pub ttl_seconds: u64,
    pub max_chars: usize,
}

/// A message pinned to this session by `statusline --notice` or the failed-command hook, until its TTL expires.
#[derive(Deserialize)]
pub struct SessionNotice {
    pub color: Color,
    pub prefix: String,
    pub max_chars: usize,
    pub show_remaining: bool,
    pub standalone: bool,
}

/// The last thing you actually typed, so you can see what the model is working on.
#[derive(Deserialize)]
pub struct MyLastPrompt {
    pub color: Color,
    pub prefix: String,
    pub max_chars: usize,
    pub standalone: bool,
}

/// Reminders whose time has come, written earlier by `statusline --remind` and shared by every session.
#[derive(Deserialize)]
pub struct Reminder {
    pub color: Color,
    pub prefix: String,
    pub separator: String,
    pub max_chars: usize,
    pub standalone: bool,
}

/// What an external model makes of the chat since it last looked, re-asked every few turns.
#[derive(Deserialize)]
pub struct LlmInsight {
    pub color: Color,
    pub prefix: String,
    pub command: String,
    pub args: Vec<String>,
    pub prompt: String,
    pub every_turns: usize,
    pub scan_whole_session: bool,
    pub initial_scan_bytes: u64,
    pub context_chars: usize,
    pub max_chars: usize,
    pub standalone: bool,
}

/// The weather outside, from wttr.in, for the city your system timezone points at.
#[derive(Deserialize)]
pub struct Weather {
    pub color: Color,
    pub prefix: String,
    pub location: String,
    pub format: String,
    pub ttl_seconds: u64,
    pub max_chars: usize,
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

/// How much of a usage window you have left before the limit resets.
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
    ClaudeResourceUsage(ClaudeResourceUsage),
    ContextUsage(ContextUsage),
    Cwd(Cwd),
    Effort(Effort),
    GitBranch(GitBranch),
    GitDiff(GitDiff),
    GitError(GitError),
    LlmAnswer(LlmAnswer),
    LlmInsight(LlmInsight),
    Model(Model),
    MyLastPrompt(MyLastPrompt),
    PromptCacheTtl(PromptCacheTtl),
    RateLimit(RateLimit),
    Reminder(Reminder),
    SessionNotice(SessionNotice),
    Spacer(Spacer),
    SubagentStats(SubagentStats),
    UserIdleTime(UserIdleTime),
    Weather(Weather),
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
    fn parses_model_replacements() {
        let body = r#"(
            separator: " ",
            separator_color: Named(90),
            segments: [Model(
                color: Named(36),
                prefix: "",
                replacements: [("Opus 5 (1M context)", "Opus")],
            )],
        )"#;

        assert!(parse(body).is_ok());
    }

    #[test]
    fn rejects_model_replacement_with_empty_search() {
        let body = r#"(
            separator: " ",
            separator_color: Named(90),
            segments: [Model(
                color: Named(36),
                prefix: "",
                replacements: [("", "Opus")],
            )],
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
