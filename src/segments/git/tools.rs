use std::fs;
use std::path::{Component, Path, PathBuf};
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
    status: Option<Result<GitStatus, String>>,
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
        self.ensure_status()?.as_ref().ok()
    }

    pub fn error(&mut self) -> Option<&str> {
        self.ensure_status()?.as_ref().err().map(String::as_str)
    }

    fn ensure_status(&mut self) -> Option<&Result<GitStatus, String>> {
        if self.cwd.is_empty() {
            return None;
        }
        if self.status.is_none() {
            self.status = Some(run_git_status(&self.cwd));
        }
        self.status.as_ref()
    }
}

fn find_git_repo(cwd: &str) -> Option<PathBuf> {
    for dir in Path::new(cwd).ancestors() {
        let dot_git = dir.join(".git");
        match fs::metadata(&dot_git) {
            Ok(m) if m.is_dir() => return Some(dot_git),
            Ok(m) if m.is_file() => return read_gitfile(&dot_git, dir),
            _ => {}
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

fn run_git_status(cwd: &str) -> Result<GitStatus, String> {
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
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let msg = stderr
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .unwrap_or("git status failed")
            .to_string();
        return Err(msg);
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut status = GitStatus::default();
    for line in text.lines() {
        parse_status_line(line, &mut status);
    }
    Ok(status)
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

    let mut fields = line.splitn(3, ' ');
    if matches!(fields.next(), Some("1" | "2")) {
        let (Some(xy), Some(_)) = (fields.next(), fields.next()) else {
            return;
        };
        let mut states = xy.chars();
        let (Some(index), Some(worktree)) = (states.next(), states.next()) else {
            return;
        };
        if states.next().is_some() {
            return;
        }

        if matches!(index, 'D') || matches!(worktree, 'D') {
            s.deleted += 1;
        } else if [index, worktree]
            .into_iter()
            .any(|state| matches!(state, 'M' | 'A' | 'T' | 'R' | 'C'))
        {
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
    if git_dir.join("commondir").is_file() {
        return true;
    }

    let comps: Vec<_> = git_dir.components().collect();
    comps.windows(3).any(|window| {
        window[0].as_os_str() == ".git"
            && window[1].as_os_str() == "worktrees"
            && matches!(window[2], Component::Normal(_))
    })
}

#[cfg(test)]
mod tests {
    use super::{is_worktree, parse_status_line, GitStatus};
    use std::fs;
    use std::path::Path;

    fn parse(lines: &[&str]) -> GitStatus {
        let mut status = GitStatus::default();
        for line in lines {
            parse_status_line(line, &mut status);
        }
        status
    }

    #[test]
    fn parses_branch_and_ahead_behind() {
        let status = parse(&["# branch.head feature/git", "# branch.ab +12 -3"]);

        assert_eq!(status.branch, "feature/git");
        assert_eq!(status.ahead, 12);
        assert_eq!(status.behind, 3);
    }

    #[test]
    fn classifies_modified_tracked_entries() {
        let status = parse(&[
            "1 M. N... 100644 100644 100644 abc def modified",
            "1 .M N... 100644 100644 100644 abc def modified-in-worktree",
            "1 A. N... 100644 100644 100644 abc def added",
            "1 T. N... 100644 100644 100644 abc def type-changed",
            "1 .T N... 100644 100644 100644 abc def type-changed-in-worktree",
            "2 R. N... 100644 100644 100644 abc def R100 renamed\toriginal",
            "2 C. N... 100644 100644 100644 abc def C100 copied\toriginal",
        ]);

        assert_eq!(status.modified, 7);
        assert_eq!(status.deleted, 0);
    }

    #[test]
    fn prioritizes_deleted_tracked_entries() {
        let status = parse(&[
            "1 D. N... 100644 000000 000000 abc def deleted",
            "1 .D N... 100644 100644 000000 abc def deleted-in-worktree",
            "1 MD N... 100644 100644 000000 abc def modified-and-deleted",
        ]);

        assert_eq!(status.deleted, 3);
        assert_eq!(status.modified, 0);
    }

    #[test]
    fn classifies_untracked_and_unmerged_entries() {
        let status = parse(&[
            "? untracked",
            "u UU N... 100644 100644 100644 100644 abc def ghi conflicted",
        ]);

        assert_eq!(status.untracked, 1);
        assert_eq!(status.modified, 1);
    }

    #[test]
    fn ignores_malformed_and_ignored_entries() {
        let status = parse(&[
            "",
            "1",
            "1 M",
            "1 MOD rest",
            "2",
            "2 T",
            "! ignored",
            "# branch.oid abc",
        ]);

        assert!(status.branch.is_empty());
        assert_eq!(status.ahead, 0);
        assert_eq!(status.behind, 0);
        assert_eq!(status.modified, 0);
        assert_eq!(status.untracked, 0);
        assert_eq!(status.deleted, 0);
    }

    #[test]
    fn detects_linked_worktree_git_dirs() {
        assert!(is_worktree(Path::new("/repo/.git/worktrees/feature")));
        assert!(is_worktree(Path::new("/repo/.git/worktrees/feature/logs")));
        assert!(is_worktree(Path::new(
            "/external/worktrees/repo/.git/worktrees/feature"
        )));
    }

    #[test]
    fn detects_linked_worktree_with_separate_git_dir() {
        let root =
            std::env::temp_dir().join(format!("statusline-worktree-test-{}", std::process::id()));
        let git_dir = root.join("repo.git/worktrees/feature");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(git_dir.join("commondir"), "../..").unwrap();

        assert!(is_worktree(&git_dir));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_non_worktree_git_dirs() {
        assert!(!is_worktree(Path::new("/repo/.git")));
        assert!(!is_worktree(Path::new("/repo/.git/worktrees")));
        assert!(!is_worktree(Path::new("/repo/worktrees/feature")));
        assert!(!is_worktree(Path::new(
            "/repo/.git/objects/worktrees/feature"
        )));
        assert!(!is_worktree(Path::new("/repo/.git/worktrees/../feature")));
    }
}
