use std::env;
use std::fs;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const C_GREEN: &str = "\x1b[0;32m";
const C_YELLOW: &str = "\x1b[0;33m";
const C_RED: &str = "\x1b[0;31m";
const C_BOLD_RED: &str = "\x1b[1;31m";
const C_RESET: &str = "\x1b[0m";

const SEPARATOR: &str = "  ";

struct Input {
    cwd: String,
    ctx_pct: Option<f64>,
}

#[derive(Default)]
struct GitStatus {
    branch: String,
    ahead: u32,
    behind: u32,
    modified: u32,
    untracked: u32,
    deleted: u32,
}

fn main() {
    let Some(input) = read_input() else { return };
    let segments = build_segments(&input);
    let mut line = segments.join(SEPARATOR);
    if let Some(cols) = terminal_width() {
        let max = cols.saturating_sub(4);
        if max > 0 {
            line = truncate_visible(&line, max);
        }
    }
    let _ = io::stdout().lock().write_all(line.as_bytes());
}

fn terminal_width() -> Option<usize> {
    let tty = File::open("/dev/tty").ok()?;
    let output = Command::new("stty")
        .arg("size")
        .stdin(Stdio::from(tty))
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.split_whitespace().nth(1)?.parse().ok()
}

fn truncate_visible(s: &str, max: usize) -> String {
    let mut out = String::with_capacity(s.len());
    let mut visible = 0usize;
    let mut chars = s.chars().peekable();
    let mut truncated = false;
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            out.push(c);
            while let Some(&nc) = chars.peek() {
                out.push(nc);
                chars.next();
                if nc.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }
        if visible >= max {
            truncated = true;
            break;
        }
        out.push(c);
        visible += 1;
    }
    if truncated {
        out.push_str(C_RESET);
    }
    out
}

fn read_input() -> Option<Input> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).ok()?;

    let json: serde_json::Value = serde_json::from_str(&buf).ok()?;

    let cwd = json
        .get("cwd")
        .and_then(|v| v.as_str())
        .or_else(|| {
            json.get("workspace")
                .and_then(|w| w.get("current_dir"))
                .and_then(|v| v.as_str())
        })
        .filter(|s| !s.is_empty())?
        .to_string();

    let ctx_pct = json
        .get("context_window")
        .and_then(|c| c.get("used_percentage"))
        .and_then(|v| v.as_f64());

    Some(Input { cwd, ctx_pct })
}

fn build_segments(input: &Input) -> Vec<String> {
    let mut segments = Vec::with_capacity(5);

    if let Some(pct) = input.ctx_pct {
        segments.push(ctx_segment(pct));
    }

    segments.push(shorten_home(&input.cwd));

    if let Some(git_dir) = find_git_repo(&input.cwd) {
        push_git_segments(&mut segments, &input.cwd, &git_dir);
    }

    segments
}

fn push_git_segments(segments: &mut Vec<String>, cwd: &str, git_dir: &Path) {
    let Some(status) = run_git_status(cwd) else { return };
    if status.branch.is_empty() {
        return;
    }

    segments.push(format_branch_segment(&status, git_dir));

    if let Some(stats) = format_diff_stats(&status) {
        segments.push(stats);
    }
}

fn shorten_home(path: &str) -> String {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_default();

    if !home.is_empty() && path.starts_with(&home) {
        format!("~{}", &path[home.len()..])
    } else {
        path.to_string()
    }
}

fn paint(color: &str, body: &str) -> String {
    format!("{color}{body}{C_RESET}")
}

fn run_git_status(cwd: &str) -> Option<GitStatus> {
    let output = Command::new("git")
        .args([
            "-C",
            cwd,
            "--no-optional-locks",
            "status",
            "--branch",
            "--porcelain=v2",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut status = GitStatus::default();
    for line in text.lines() {
        parse_status_line(line, &mut status);
    }
    Some(status)
}

fn parse_status_line(line: &str, s: &mut GitStatus) {
    if let Some(rest) = line.strip_prefix("# branch.head ") {
        s.branch = rest.to_string();
        return;
    }

    if let Some(rest) = line.strip_prefix("# branch.ab ") {
        let mut parts = rest.split_whitespace();
        if let Some(a) = parts.next() {
            s.ahead = a.trim_start_matches('+').parse().unwrap_or(0);
        }
        if let Some(b) = parts.next() {
            s.behind = b.trim_start_matches('-').parse().unwrap_or(0);
        }
        return;
    }

    if line.starts_with("1 ") || line.starts_with("2 ") {
        let xy = line.get(2..4).unwrap_or("");
        if xy.contains('D') {
            s.deleted += 1;
        }
        if xy.chars().any(|c| matches!(c, 'M' | 'A' | 'R' | 'C' | 'U')) {
            s.modified += 1;
        }
        return;
    }

    if line.starts_with("? ") {
        s.untracked += 1;
    } else if line.starts_with("u ") {
        s.modified += 1;
    }
}

fn find_git_repo(cwd: &str) -> Option<PathBuf> {
    for dir in Path::new(cwd).ancestors() {
        let dot_git = dir.join(".git");
        if dot_git.is_dir() {
            return Some(dot_git);
        }
        if dot_git.is_file() {
            return read_gitfile(&dot_git, dir);
        }
    }
    None
}

fn read_gitfile(gitfile: &Path, base_dir: &Path) -> Option<PathBuf> {
    let content = fs::read_to_string(gitfile).ok()?;
    let path = content.trim().strip_prefix("gitdir: ")?;
    let resolved = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        base_dir.join(path)
    };
    Some(resolved)
}

fn detect_git_state(git_dir: &Path) -> Option<String> {
    if let Some(s) = rebase_merge_state(git_dir) {
        return Some(s);
    }
    if let Some(s) = rebase_apply_state(git_dir) {
        return Some(s);
    }

    const SIMPLE_STATES: &[(&str, &str)] = &[
        ("MERGE_HEAD", "MERGE"),
        ("CHERRY_PICK_HEAD", "CHERRY-PICK"),
        ("REVERT_HEAD", "REVERT"),
        ("BISECT_LOG", "BISECT"),
    ];

    SIMPLE_STATES
        .iter()
        .find(|(file, _)| git_dir.join(file).is_file())
        .map(|(_, label)| label.to_string())
}

fn rebase_merge_state(git_dir: &Path) -> Option<String> {
    let dir = git_dir.join("rebase-merge");
    if !dir.is_dir() {
        return None;
    }
    Some(progress_label(&dir, "msgnum", "end", "REBASE"))
}

fn rebase_apply_state(git_dir: &Path) -> Option<String> {
    let dir = git_dir.join("rebase-apply");
    if !dir.is_dir() {
        return None;
    }
    let label = if dir.join("applying").is_file() {
        "AM"
    } else {
        "REBASE"
    };
    Some(progress_label(&dir, "next", "last", label))
}

fn progress_label(dir: &Path, num_file: &str, total_file: &str, label: &str) -> String {
    let num = read_trim(&dir.join(num_file));
    let total = read_trim(&dir.join(total_file));
    match (num, total) {
        (Some(n), Some(t)) => format!("{label} {n}/{t}"),
        _ => label.to_string(),
    }
}

fn read_trim(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn format_branch_segment(status: &GitStatus, git_dir: &Path) -> String {
    let mut segment = String::new();
    if is_worktree(git_dir) {
        segment.push('⑂');
    }
    segment.push_str(&status.branch);

    if let Some(state) = detect_git_state(git_dir) {
        segment.push(' ');
        segment.push_str(&paint(C_BOLD_RED, &format!("[{state}]")));
    }

    if let Some(ab) = format_ahead_behind(status.ahead, status.behind) {
        segment.push_str(&ab);
    }

    segment
}

fn format_ahead_behind(ahead: u32, behind: u32) -> Option<String> {
    let mut parts = Vec::with_capacity(2);
    if ahead > 0 {
        parts.push(format!("↑{ahead}"));
    }
    if behind > 0 {
        parts.push(format!("↓{behind}"));
    }
    if parts.is_empty() {
        None
    } else {
        Some(format!("({})", parts.join(" ")))
    }
}

fn format_diff_stats(status: &GitStatus) -> Option<String> {
    let mut parts = Vec::with_capacity(3);

    if status.modified > 0 {
        parts.push(paint(C_YELLOW, &format!("~{}", status.modified)));
    }
    if status.untracked > 0 {
        parts.push(paint(C_GREEN, &format!("+{}", status.untracked)));
    }
    if status.deleted > 0 {
        parts.push(paint(C_RED, &format!("-{}", status.deleted)));
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn ctx_segment(p: f64) -> String {
    let lerp = |a: i32, b: i32, t: f64| (a as f64 + (b - a) as f64 * t.clamp(0.0, 1.0)) as u8;
    let (r, g, b) = if p <= 20.0 {
        let t = p / 20.0;
        (lerp(150, 180, t), lerp(150, 165, t), lerp(150, 100, t))
    } else {
        let t = (p - 20.0) / 10.0;
        (lerp(180, 220, t), lerp(165, 60, t), lerp(100, 60, t))
    };
    format!("\x1b[38;2;{r};{g};{b}m{}%\x1b[0m", p.round() as i64)
}

fn is_worktree(git_dir: &Path) -> bool {
    let comps: Vec<_> = git_dir.components().collect();
    let Some(pos) = comps.iter().position(|c| c.as_os_str() == "worktrees") else {
        return false;
    };
    pos > 0 && comps[pos - 1].as_os_str() == ".git"
}
