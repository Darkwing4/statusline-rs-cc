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
    /// Colour of the percentage; Gradient shifts it from green to red as the window fills up.
    pub color: Color,
    /// Text before the percentage.
    pub prefix: String,
    /// Colour of the prefix.
    pub prefix_color: Color,
    /// Text after the percentage.
    pub suffix: String,
    /// Colour of the suffix.
    pub suffix_color: Color,
}

/// How long the prompt cache stays warm before the next request pays to build it again.
#[derive(Deserialize)]
pub struct PromptCacheTtl {
    /// Colour of the countdown; Gradient shifts it as the cache gets closer to going cold.
    pub color: Color,
    /// Text before the countdown.
    pub prefix: String,
}

/// How much CPU and memory the Claude Code process tree is eating right now (Linux only).
#[derive(Deserialize)]
pub struct ClaudeResourceUsage {
    /// Colour of both readings.
    pub color: Color,
    /// Text before the CPU reading, which is in cores, so 1.00c is one fully busy core.
    pub cpu_prefix: String,
    /// Text before the memory reading, which is in MiB.
    pub memory_prefix: String,
}

/// The directory Claude Code is working in, with your home folder shortened to a tilde.
#[derive(Deserialize)]
pub struct Cwd {
    /// Colour of the path.
    pub color: Color,
}

/// Nothing at all: a gap between two neighbours, a line break, or a blank line of its own.
#[derive(Deserialize)]
pub struct Spacer {
    /// Gap sits between two neighbours, LineBreak starts the next piece on a new line, BlankLine leaves an empty line.
    pub shape: SpacerShape,
}

#[derive(Clone, Copy, Deserialize, PartialEq)]
pub enum SpacerShape {
    Gap,
    LineBreak,
    BlankLine,
}

/// How hard the model is currently set to think.
#[derive(Deserialize)]
pub struct Effort {
    /// Colour of the effort level.
    pub color: Color,
    /// Text before the effort level.
    pub prefix: String,
}

/// The model Claude Code is answering with.
#[derive(Deserialize)]
pub struct Model {
    /// Colour of the model name.
    pub color: Color,
    /// Text before the model name.
    pub prefix: String,
    /// Pairs of text to find in the model name and what to show instead, such as Opus 5 (1M context) and Opus.
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
    /// Colour of the branch name and of the ahead and behind counts.
    pub color: Color,
    /// Colour of what git is in the middle of, such as rebase.
    pub state_color: Color,
    /// Put a ⑂ before the branch name while you are in a linked git worktree.
    pub show_worktree: bool,
    /// Show how many commits you are ahead of and behind upstream, as ↑2 ↓1.
    pub show_ahead_behind: bool,
    /// Show what git is in the middle of, such as [rebase], after the branch name.
    pub show_state: bool,
}

/// How many files you changed, added, and deleted since the last commit.
#[derive(Deserialize)]
pub struct GitDiff {
    /// Colour of the ~N count of changed files.
    pub modified_color: Color,
    /// Colour of the +N count of new files.
    pub untracked_color: Color,
    /// Colour of the -N count of deleted files.
    pub deleted_color: Color,
}

/// A short marker for when the current directory is not a git repository at all.
#[derive(Deserialize)]
pub struct GitError {
    /// Colour of the marker.
    pub color: Color,
    /// What to show when the directory is not inside a git repository.
    pub text: String,
}

/// How long it has been since you last typed something to Claude Code.
#[derive(Deserialize)]
pub struct UserIdleTime {
    /// Colour of the idle time.
    pub color: Color,
    /// Text before the idle time.
    pub prefix: String,
    /// Stay hidden until you have been idle for this many seconds.
    pub threshold_seconds: u64,
}

/// How many subagents are running, how long the oldest one has been at it, and the tokens they burned.
#[derive(Deserialize)]
pub struct SubagentStats {
    /// Colour while no subagent is running.
    pub color: Color,
    /// Colour while at least one subagent is running.
    pub active_color: Color,
    /// Colour once a running subagent has gone quiet for longer than the stall limit.
    pub stall_color: Color,
    /// Text before the counts.
    pub prefix: String,
    /// Text appended after the counts once a subagent has stalled.
    pub stall_marker: String,
    /// How many seconds a subagent may stay quiet before it counts as stalled.
    pub stall_seconds: u64,
    /// Show the tokens all subagents have burned, as 1.2M.
    pub show_tokens: bool,
}

/// The last line an external command prints for a fixed prompt, re-asked once its TTL runs out.
#[derive(Deserialize)]
pub struct LlmAnswer {
    /// Colour of the answer.
    pub color: Color,
    /// Text before the answer.
    pub prefix: String,
    /// The program to run and its arguments, such as codex exec; the prompt goes to its stdin.
    pub command: String,
    /// Arguments the command is started with.
    pub args: Vec<String>,
    /// What to ask; the last line the command prints is what gets shown.
    pub prompt: String,
    /// How many seconds the answer is kept before the command is run again.
    pub ttl_seconds: u64,
    /// Cut the answer after this many characters.
    pub max_chars: usize,
}

/// A message pinned to this session by `statusline --notice` or the failed-command hook, until its TTL expires.
#[derive(Deserialize)]
pub struct SessionNotice {
    /// Colour of the notice.
    pub color: Color,
    /// Text before the notice.
    pub prefix: String,
    /// Cut the notice after this many characters.
    pub max_chars: usize,
    /// Show how long the notice stays pinned, in brackets after it.
    pub show_remaining: bool,
    /// Put the notice on a line of its own instead of inline with the rest.
    pub standalone: bool,
}

/// The last thing you actually typed, so you can see what the model is working on.
#[derive(Deserialize)]
pub struct MyLastPrompt {
    /// Colour of the prompt.
    pub color: Color,
    /// Text before the prompt.
    pub prefix: String,
    /// Cut the prompt after this many characters; 0 lets it run to the edge of the terminal.
    pub max_chars: usize,
    /// Put the prompt on a line of its own instead of inline with the rest.
    pub standalone: bool,
}

/// Reminders whose time has come, written earlier by `statusline --remind` and shared by every session.
#[derive(Deserialize)]
pub struct Reminder {
    /// Colour of the reminders.
    pub color: Color,
    /// Text before the reminders.
    pub prefix: String,
    /// Text between two reminders that are due at the same time.
    pub separator: String,
    /// Cut the joined reminders after this many characters.
    pub max_chars: usize,
    /// Put the reminders on a line of their own instead of inline with the rest.
    pub standalone: bool,
}

/// What an external model makes of the chat since it last looked, re-asked every few turns.
#[derive(Deserialize)]
pub struct LlmInsight {
    /// Colour of the answer.
    pub color: Color,
    /// Text before the answer.
    pub prefix: String,
    /// The program to run and its arguments, such as codex exec; the prompt and the conversation go to its stdin.
    pub command: String,
    /// Arguments the command is started with.
    pub args: Vec<String>,
    /// What you want the model to tell you about the conversation, in one line.
    pub prompt: String,
    /// Run the command again after this many of your prompts.
    pub every_turns: usize,
    /// On the first render of a session read the transcript from its very beginning instead of only its tail.
    pub scan_whole_session: bool,
    /// How many KiB from the end of the transcript to read on the first render when the whole session is not scanned.
    pub initial_scan_kib: u64,
    /// How many of the newest characters of the conversation the model gets to see each time.
    pub context_chars: usize,
    /// The model is asked to fit its answer into this many characters, and anything longer is cut.
    pub max_chars: usize,
    /// Put the answer on a line of its own instead of inline with the rest.
    pub standalone: bool,
}

/// The weather outside, from wttr.in, for the city your system timezone points at.
#[derive(Deserialize)]
pub struct Weather {
    /// Colour of the report.
    pub color: Color,
    /// Text before the report.
    pub prefix: String,
    /// City or place to ask wttr.in about; leave it empty to use the city your system timezone points at.
    pub location: String,
    /// wttr.in format string, such as %c+%t for the condition icon and the temperature.
    pub format: String,
    /// How many seconds the report is kept before it is fetched again.
    pub ttl_seconds: u64,
    /// Cut the report after this many characters.
    pub max_chars: usize,
}

#[derive(Clone, Copy, Deserialize)]
pub enum Window {
    FiveHour,
    SevenDay,
    Fable,
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
    /// Which limit to watch: the 5-hour window, the 7-day window, or the weekly Fable window fetched in the background.
    pub window: Window,
    /// Show the window as a percentage, a bar, a radial dial, or a bar or dial with the percentage next to it.
    pub style: Style,
    /// Whether the bar, the dial, and the percentage count what is used up or what is still left.
    pub fill: Fill,
    /// Steps jump between the three colours at fixed thresholds; Gradient blends them as usage grows.
    pub color_mode: ColorMode,
    /// Usage percentage at which a gradient shows exactly the middle colour.
    #[serde(
        default = "default_gradient_midpoint_percentage",
        deserialize_with = "deserialize_gradient_midpoint_percentage"
    )]
    pub gradient_midpoint_percentage: f64,
    /// Text before the reading; {t} becomes the time left until the window resets, in hours for 5h and days for 7d.
    pub prefix: String,
    /// How many seconds the Fable usage fetched from the API is reused before it is fetched again.
    #[serde(default = "default_usage_ttl_seconds")]
    pub usage_ttl_seconds: u64,
    /// Text appended while this window is the one currently limiting you.
    #[serde(default)]
    pub active_marker: String,
    /// Pairs of a severity the API reports, such as warning, and the text to append while it is active.
    #[serde(default, deserialize_with = "deserialize_severity_markers")]
    pub severity_markers: Vec<(String, String)>,
    /// Colour while usage is low.
    pub low_color: Color,
    /// Colour around the middle of the window.
    pub mid_color: Color,
    /// Colour once the window is nearly used up.
    pub high_color: Color,
}

fn default_gradient_midpoint_percentage() -> f64 {
    50.0
}

fn default_usage_ttl_seconds() -> u64 {
    300
}

fn deserialize_severity_markers<'de, D>(deserializer: D) -> Result<Vec<(String, String)>, D::Error>
where
    D: Deserializer<'de>,
{
    let pairs = Vec::<(String, String)>::deserialize(deserializer)?;

    if pairs.iter().any(|(severity, _)| severity.is_empty()) {
        return Err(serde::de::Error::custom(
            "rate limit severity markers must not match an empty severity",
        ));
    }

    Ok(pairs)
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
    fn defaults_the_fable_window_extras_for_existing_configs() {
        let rate_limit = parse_rate_limit(
            r#"(
                window: Fable,
                style: BarPercent,
                fill: Used,
                color_mode: Gradient,
                prefix: "{t}d ",
                low_color: Rgb(0, 0, 0),
                mid_color: Rgb(100, 100, 100),
                high_color: Rgb(200, 200, 200),
            )"#,
        )
        .expect("a Fable RateLimit without extras should parse");

        assert_eq!(rate_limit.usage_ttl_seconds, 300);
        assert_eq!(rate_limit.active_marker, "");
        assert!(rate_limit.severity_markers.is_empty());
    }

    #[test]
    fn rejects_a_severity_marker_without_a_severity() {
        let body = r#"(
            window: Fable,
            style: Percent,
            fill: Used,
            color_mode: Steps,
            prefix: "",
            severity_markers: [("", "!")],
            low_color: Rgb(0, 0, 0),
            mid_color: Rgb(100, 100, 100),
            high_color: Rgb(200, 200, 200),
        )"#;

        assert!(parse_rate_limit(body).is_err());
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
