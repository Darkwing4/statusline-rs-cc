use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Default)]
pub struct GitStatus {
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    pub modified: u32,
    pub untracked: u32,
    pub deleted: u32,
}

pub struct GitCache {
    cwd: String,
    dir: Option<Option<PathBuf>>,
    status: Option<Option<GitStatus>>,
}

impl GitCache {
    pub fn new(cwd: String) -> Self {
        Self {
            cwd,
            dir: None,
            status: None,
        }
    }

    pub fn dir(&mut self) -> Option<&Path> {
        if self.cwd.is_empty() {
            return None;
        }
        if self.dir.is_none() {
            self.dir = Some(find_git_repo(&self.cwd));
        }
        self.dir.as_ref().unwrap().as_deref()
    }

    pub fn status(&mut self) -> Option<&GitStatus> {
        if self.cwd.is_empty() {
            return None;
        }
        if self.status.is_none() {
            self.status = Some(run_git_status(&self.cwd));
        }
        self.status.as_ref().unwrap().as_ref()
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

pub fn detect_state(git_dir: &Path) -> Option<String> {
    if let Some(s) = rebase_merge_state(git_dir) {
        return Some(s);
    }
    if let Some(s) = rebase_apply_state(git_dir) {
        return Some(s);
    }

    const SIMPLE: &[(&str, &str)] = &[
        ("MERGE_HEAD", "MERGE"),
        ("CHERRY_PICK_HEAD", "CHERRY-PICK"),
        ("REVERT_HEAD", "REVERT"),
        ("BISECT_LOG", "BISECT"),
    ];

    SIMPLE
        .iter()
        .find(|(f, _)| git_dir.join(f).is_file())
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

fn progress_label(dir: &Path, num: &str, total: &str, label: &str) -> String {
    let n = read_trim(&dir.join(num));
    let t = read_trim(&dir.join(total));
    match (n, t) {
        (Some(n), Some(t)) => format!("{} {}/{}", label, n, t),
        _ => label.to_string(),
    }
}

fn read_trim(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

pub fn is_worktree(git_dir: &Path) -> bool {
    let comps: Vec<_> = git_dir.components().collect();
    let Some(pos) = comps.iter().position(|c| c.as_os_str() == "worktrees") else {
        return false;
    };
    pos > 0 && comps[pos - 1].as_os_str() == ".git"
}
