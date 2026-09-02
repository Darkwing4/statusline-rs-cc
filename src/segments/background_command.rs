use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::SystemTime;

use crate::config::SegmentSpec;
use crate::private_file;
use crate::segments::single_line_text::sanitize;
use crate::statusline_cache_dir::cache_dir;
use crate::statusline_cli::REFRESH_FLAG;

pub const REQUEST_FLAG: &str = "--request";

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

    fn run(&self, workdir: &Path) -> Option<String> {
        let mut child = Command::new(&self.command)
            .args(&self.args)
            .current_dir(workdir)
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

pub fn refresh(fingerprint: &str, request_base: Option<PathBuf>) {
    let Ok(config) = crate::config::load_embedded() else {
        return;
    };

    let Some(mut command) = config
        .segments
        .into_iter()
        .filter_map(background_command)
        .find(|command| command.fingerprint() == fingerprint)
    else {
        return;
    };

    let Some(paths) = (match request_base.as_deref() {
        Some(base) => CachePaths::for_request(base, &mut command),
        None => CachePaths::for_command(&command),
    }) else {
        return;
    };

    let Some(workdir) = command_workdir() else {
        return;
    };

    let Some(text) = command.run(&workdir) else {
        return;
    };

    paths.store(&text);

    if let Some(base) = request_base.as_deref() {
        crate::segments::llm_insight::log_run(base, &text);
    }
}

fn command_workdir() -> Option<PathBuf> {
    let dir = cache_dir()?.join("workdir");
    fs::create_dir_all(&dir).ok()?;

    Some(dir)
}

fn background_command(spec: SegmentSpec) -> Option<BackgroundCommand> {
    match spec {
        SegmentSpec::LlmInsight(segment) => Some(segment.background_command()),
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

    fn for_request(base: &Path, command: &mut BackgroundCommand) -> Option<Self> {
        let request = suffixed(base, "request");
        command.stdin_input = fs::read_to_string(&request).ok()?;

        Some(CachePaths {
            result: suffixed(base, "txt"),
            attempt: suffixed(base, "attempt"),
            pending: suffixed(base, "pending"),
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

        if private_file::write(&self.pending, text).is_err() {
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

fn suffixed(base: &Path, suffix: &str) -> PathBuf {
    let mut name = base.as_os_str().to_os_string();
    name.push(".");
    name.push(suffix);

    PathBuf::from(name)
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

#[cfg(test)]
mod tests {
    use super::{is_expired, last_non_empty_line, BackgroundCommand};

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
    fn expires_only_after_the_ttl() {
        assert!(is_expired(None, 900));
        assert!(is_expired(Some(900), 900));
        assert!(!is_expired(Some(899), 900));
    }

    #[cfg(unix)]
    #[test]
    fn runs_the_command_inside_the_given_directory() {
        let workdir = std::env::temp_dir();
        let mut command = command(&[], "");
        command.command = "pwd".to_string();
        command.max_chars = 0;

        assert_eq!(
            command.run(&workdir).map(std::path::PathBuf::from),
            std::fs::canonicalize(&workdir).ok()
        );
    }

    #[test]
    fn fingerprint_tracks_command_args_and_stdin() {
        let base = command(&["exec"], "one quote");

        assert_eq!(
            base.fingerprint(),
            command(&["exec"], "one quote").fingerprint()
        );
        assert_ne!(
            base.fingerprint(),
            command(&["exec"], "another").fingerprint()
        );
        assert_ne!(
            base.fingerprint(),
            command(&["exec", "--json"], "one quote").fingerprint()
        );
    }
}
