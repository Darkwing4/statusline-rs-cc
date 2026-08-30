mod insight_cache;
mod turn_delta;

use std::fs::{self, File};
use std::io::{Seek, SeekFrom};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::SystemTime;

use serde_json::Value;

pub use crate::config_schema::LlmInsight;
use crate::segments::background_command::{BackgroundCommand, REQUEST_FLAG};
use crate::segments::single_line_text::sanitize;
use crate::segments::{GitCache, Segment};
use crate::statusline_cli::REFRESH_FLAG;
use crate::statusline_input::session_key;

use self::insight_cache::InsightState;
use self::turn_delta::{keep_tail, scan_delta};

const INITIAL_TAIL_BYTES: u64 = 256 * 1024;
const MIN_REFRESH_GAP_SECONDS: u64 = 30;
const NO_PREVIOUS_ANSWER: &str = "(none yet)";

impl Segment for LlmInsight {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let transcript = json.get("transcript_path")?.as_str()?;
        let session = session_key(json)?;
        let base = insight_cache::base_path(&self.background_command().fingerprint(), &session)?;

        self.advance(&base, transcript);

        let stored = fs::read_to_string(insight_cache::result_path(&base)).ok()?;
        let text = sanitize(&stored, self.max_chars);

        if text.is_empty() {
            return None;
        }

        Some(self.color.paint(&format!("{}{}", self.prefix, text)))
    }

    fn standalone(&self) -> bool {
        self.standalone
    }
}

impl LlmInsight {
    pub(super) fn background_command(&self) -> BackgroundCommand {
        BackgroundCommand {
            command: self.command.clone(),
            args: self.args.clone(),
            stdin_input: self.prompt.clone(),
            ttl_seconds: 0,
            max_chars: self.max_chars,
        }
    }

    fn advance(&self, base: &Path, transcript: &str) {
        let Ok(length) = fs::metadata(transcript).map(|meta| meta.len()) else {
            return;
        };

        let mut state = insight_cache::load(base);

        if state.scanned_bytes > length {
            state = InsightState::default();
        }

        if state.scanned_bytes == 0 {
            state.scanned_bytes = length.saturating_sub(INITIAL_TAIL_BYTES);
        }

        let Ok(mut file) = File::open(transcript) else {
            return;
        };

        if file.seek(SeekFrom::Start(state.scanned_bytes)).is_err() {
            return;
        }

        let scan = scan_delta(&mut file);
        state.scanned_bytes += scan.bytes;
        state.turns_since_run += scan.turns;

        if !scan.text.is_empty() {
            if !state.delta.is_empty() {
                state.delta.push('\n');
            }

            state.delta.push_str(&scan.text);
            state.delta = keep_tail(&state.delta, self.delta_chars);
        }

        if self.is_due(&state) && self.spawn_worker(base, &state.delta) {
            state.turns_since_run = 0;
            state.delta.clear();
        }

        insight_cache::store(base, &state);
    }

    fn is_due(&self, state: &InsightState) -> bool {
        self.every_turns > 0 && state.turns_since_run >= self.every_turns
    }

    fn spawn_worker(&self, base: &Path, delta: &str) -> bool {
        if delta.trim().is_empty() || is_recent(&insight_cache::attempt_path(base)) {
            return false;
        }

        let previous = fs::read_to_string(insight_cache::result_path(base)).unwrap_or_default();
        let request = self.request_body(&previous, delta);

        if fs::write(insight_cache::request_path(base), request).is_err() {
            return false;
        }

        if fs::write(insight_cache::attempt_path(base), b"").is_err() {
            return false;
        }

        let Ok(exe) = std::env::current_exe() else {
            return false;
        };

        Command::new(exe)
            .arg(REFRESH_FLAG)
            .arg(self.background_command().fingerprint())
            .arg(REQUEST_FLAG)
            .arg(base)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .is_ok()
    }

    fn request_body(&self, previous: &str, delta: &str) -> String {
        let previous = previous.trim();
        let previous = if previous.is_empty() {
            NO_PREVIOUS_ANSWER
        } else {
            previous
        };

        format!(
            "{}\n\n[your previous answer]\n{}\n\n[new conversation since then]\n{}\n",
            self.prompt.trim(),
            previous,
            delta.trim()
        )
    }
}

fn is_recent(path: &Path) -> bool {
    let Ok(modified) = fs::metadata(path).and_then(|meta| meta.modified()) else {
        return false;
    };

    SystemTime::now()
        .duration_since(modified)
        .map(|age| age.as_secs() < MIN_REFRESH_GAP_SECONDS)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::insight_cache::InsightState;
    use super::LlmInsight;
    use crate::types::Color;

    fn segment(every_turns: usize) -> LlmInsight {
        LlmInsight {
            color: Color::Named(90),
            prefix: "🎯 ".to_string(),
            command: "codex".to_string(),
            args: vec!["exec".to_string()],
            prompt: "One sentence: the user's goal and what is being done for it.".to_string(),
            every_turns,
            delta_chars: 4_000,
            max_chars: 128,
            standalone: true,
        }
    }

    fn state(turns_since_run: usize) -> InsightState {
        InsightState {
            scanned_bytes: 10,
            turns_since_run,
            delta: "user: почини сборку".to_string(),
        }
    }

    #[test]
    fn runs_once_the_configured_number_of_turns_passed() {
        let segment = segment(2);

        assert!(!segment.is_due(&state(1)));
        assert!(segment.is_due(&state(2)));
        assert!(segment.is_due(&state(5)));
    }

    #[test]
    fn never_runs_when_the_turn_interval_is_zero() {
        assert!(!segment(0).is_due(&state(99)));
    }

    #[test]
    fn feeds_the_model_its_own_previous_answer_and_the_new_turns() {
        let body = segment(2).request_body("цель: собрать релиз", "user: теперь тесты");

        assert_eq!(
            body,
            concat!(
                "One sentence: the user's goal and what is being done for it.\n\n",
                "[your previous answer]\nцель: собрать релиз\n\n",
                "[new conversation since then]\nuser: теперь тесты\n"
            )
        );
    }

    #[test]
    fn marks_the_first_run_as_having_no_previous_answer() {
        let body = segment(2).request_body("   ", "user: привет");

        assert!(body.contains("[your previous answer]\n(none yet)"));
    }
}
