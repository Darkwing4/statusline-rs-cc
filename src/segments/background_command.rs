use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::SystemTime;

use crate::ansi::strip_ansi;
use crate::config::SegmentSpec;
use crate::statusline_cache_dir::cache_dir;

pub const REFRESH_FLAG: &str = "--refresh";

const ELLIPSIS: char = '…';

pub(super) struct BackgroundCommand {
    pub(super) command: String,
    pub(super) args: Vec<String>,
    pub(super) stdin_input: String,
    pub(super) ttl_seconds: u64,
    pub(super) max_chars: usize,
}

impl BackgroundCommand {
    pub(super) fn cached_line(&self) -> Option<String> {
        if self.command.is_empty() {
            return None;
        }

        let paths = CachePaths::for_command(self)?;
        let cached = fs::read_to_string(&paths.result).ok();

        if self.needs_refresh(&paths) {
            paths.mark_attempt();
            self.spawn_refresh();
        }

        let text = sanitize(cached.as_deref()?, self.max_chars);

        (!text.is_empty()).then_some(text)
    }

    fn needs_refresh(&self, paths: &CachePaths) -> bool {
        is_expired(age_seconds(&paths.result), self.ttl_seconds)
            && is_expired(age_seconds(&paths.attempt), self.ttl_seconds)
    }

    fn spawn_refresh(&self) {
        let Ok(exe) = std::env::current_exe() else {
            return;
        };

        let _ = Command::new(exe)
            .arg(REFRESH_FLAG)
            .arg(self.fingerprint())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }

    pub(super) fn fingerprint(&self) -> String {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;

        for part in std::iter::once(self.command.as_str())
            .chain(self.args.iter().map(String::as_str))
            .chain(std::iter::once(self.stdin_input.as_str()))
        {
            for byte in part.as_bytes().iter().chain(std::iter::once(&0u8)) {
                hash ^= *byte as u64;
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }

        format!("{hash:016x}")
    }

    fn run(&self) -> Option<String> {
        let mut child = Command::new(&self.command)
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(self.stdin_input.as_bytes());
        }

        let output = child.wait_with_output().ok()?;
        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let line = last_non_empty_line(&stdout)?;
        let text = sanitize(line, self.max_chars);

        (!text.is_empty()).then_some(text)
    }
}

pub fn refresh(fingerprint: &str) {
    let Ok(config) = crate::config::load_embedded() else {
        return;
    };

    let Some(command) = config
        .segments
        .into_iter()
        .filter_map(background_command)
        .find(|command| command.fingerprint() == fingerprint)
    else {
        return;
    };

    let Some(paths) = CachePaths::for_command(&command) else {
        return;
    };

    let Some(text) = command.run() else {
        return;
    };

    paths.store(&text);
}

fn background_command(spec: SegmentSpec) -> Option<BackgroundCommand> {
    match spec {
        SegmentSpec::LlmMessage(segment) => Some(segment.background_command()),
        SegmentSpec::Weather(segment) => Some(segment.background_command()),
        _ => None,
    }
}

struct CachePaths {
    result: PathBuf,
    attempt: PathBuf,
    pending: PathBuf,
}

impl CachePaths {
    fn for_command(command: &BackgroundCommand) -> Option<Self> {
        let dir = cache_dir()?;
        let fingerprint = command.fingerprint();

        Some(CachePaths {
            result: dir.join(format!("command-{fingerprint}.txt")),
            attempt: dir.join(format!("command-{fingerprint}.attempt")),
            pending: dir.join(format!("command-{fingerprint}.pending")),
        })
    }

    fn mark_attempt(&self) {
        if !self.ensure_dir() {
            return;
        }

        let _ = fs::write(&self.attempt, b"");
    }

    fn store(&self, text: &str) {
        if !self.ensure_dir() {
            return;
        }

        if fs::write(&self.pending, text).is_err() {
            return;
        }

        if fs::rename(&self.pending, &self.result).is_err() {
            let _ = fs::remove_file(&self.pending);
        }
    }

    fn ensure_dir(&self) -> bool {
        let Some(parent) = self.result.parent() else {
            return false;
        };

        fs::create_dir_all(parent).is_ok()
    }
}

fn age_seconds(path: &Path) -> Option<u64> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;

    SystemTime::now()
        .duration_since(modified)
        .ok()
        .map(|elapsed| elapsed.as_secs())
}

fn is_expired(age: Option<u64>, ttl_seconds: u64) -> bool {
    match age {
        Some(age) => age >= ttl_seconds,
        None => true,
    }
}

fn last_non_empty_line(output: &str) -> Option<&str> {
    output.lines().map(str::trim).rfind(|line| !line.is_empty())
}

fn sanitize(text: &str, max_chars: usize) -> String {
    let plain = strip_ansi(text);
    let words = plain
        .split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|character| !character.is_control())
                .collect::<String>()
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    truncate(&words, max_chars)
}

fn truncate(text: &str, max_chars: usize) -> String {
    if max_chars == 0 || text.chars().count() <= max_chars {
        return text.to_string();
    }

    let kept = max_chars.saturating_sub(1);
    let mut shortened = text.chars().take(kept).collect::<String>();
    shortened.push(ELLIPSIS);

    shortened
}

#[cfg(test)]
mod tests {
    use super::{is_expired, last_non_empty_line, sanitize, BackgroundCommand};

    fn command(args: &[&str], stdin_input: &str) -> BackgroundCommand {
        BackgroundCommand {
            command: "codex".to_string(),
            args: args.iter().map(|arg| arg.to_string()).collect(),
            stdin_input: stdin_input.to_string(),
            ttl_seconds: 900,
            max_chars: 80,
        }
    }

    #[test]
    fn takes_the_final_line_of_command_output() {
        let output = "codex\nthinking...\n\nДержи слово, бейся до конца.\n\n";

        assert_eq!(
            last_non_empty_line(output),
            Some("Держи слово, бейся до конца.")
        );
        assert_eq!(last_non_empty_line("   \n\n"), None);
    }

    #[test]
    fn strips_ansi_and_control_characters_and_collapses_whitespace() {
        assert_eq!(
            sanitize("\u{1b}[31mбей\tкрепче\u{7}  и молча\u{1b}[0m", 80),
            "бей крепче и молча"
        );
    }

    #[test]
    fn truncates_to_max_chars_with_an_ellipsis() {
        assert_eq!(sanitize("абвгде", 4), "абв…");
        assert_eq!(sanitize("абвгде", 6), "абвгде");
        assert_eq!(sanitize("абвгде", 0), "абвгде");
    }

    #[test]
    fn expires_only_after_the_ttl() {
        assert!(is_expired(None, 900));
        assert!(is_expired(Some(900), 900));
        assert!(!is_expired(Some(899), 900));
    }

    #[test]
    fn fingerprint_tracks_command_args_and_stdin() {
        let base = command(&["exec"], "one quote");

        assert_eq!(base.fingerprint(), command(&["exec"], "one quote").fingerprint());
        assert_ne!(base.fingerprint(), command(&["exec"], "another").fingerprint());
        assert_ne!(
            base.fingerprint(),
            command(&["exec", "--json"], "one quote").fingerprint()
        );
    }
}
