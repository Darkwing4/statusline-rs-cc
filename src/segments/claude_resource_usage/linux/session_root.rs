use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Read;
use std::os::raw::c_int;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::process_stat::{self, ProcessStat};

const O_NOFOLLOW: c_int = 0o400000;
const MAX_REGISTRY_BYTES: u64 = 64 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ResolvedRoot {
    pub(super) pid: u32,
    pub(super) start_time: u64,
}

#[derive(Debug, Eq, PartialEq)]
struct SessionRecord {
    pid: u32,
    session_id: String,
    proc_start: u64,
}

pub(super) fn resolve(session_id: &str) -> Option<ResolvedRoot> {
    let sessions = session_directory()?;
    resolve_from_ancestry(session_id, &sessions)
        .or_else(|| resolve_from_registry(session_id, &sessions))
}

fn session_directory() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("CLAUDE_CONFIG_DIR").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(path).join("sessions"));
    }

    let home = std::env::var_os("HOME").filter(|value| !value.is_empty())?;
    Some(PathBuf::from(home).join(".claude").join("sessions"))
}

fn resolve_from_ancestry(session_id: &str, sessions: &Path) -> Option<ResolvedRoot> {
    let mut pid = std::process::id();
    let mut visited = HashSet::new();

    while pid != 0 && visited.insert(pid) {
        let stat = match process_stat::read(pid) {
            Some(stat) => stat,
            None => break,
        };
        let path = sessions.join(format!("{pid}.json"));

        if let Some(record) = read_session_record(&path) {
            if record_matches(&record, pid, session_id, &stat) {
                return Some(ResolvedRoot {
                    pid,
                    start_time: stat.start_time,
                });
            }
        }

        pid = stat.ppid;
    }

    None
}

fn resolve_from_registry(session_id: &str, sessions: &Path) -> Option<ResolvedRoot> {
    let entries = fs::read_dir(sessions).ok()?;
    let mut candidates = Vec::new();

    for entry in entries.flatten() {
        let Some(file_pid) = registry_file_pid(&entry.file_name()) else {
            continue;
        };
        let Some(record) = read_session_record(&entry.path()) else {
            continue;
        };
        let Some(stat) = process_stat::read(file_pid) else {
            continue;
        };

        if record_matches(&record, file_pid, session_id, &stat) {
            candidates.push(ResolvedRoot {
                pid: file_pid,
                start_time: stat.start_time,
            });
        }
    }

    unique_root(candidates)
}

fn registry_file_pid(name: &OsStr) -> Option<u32> {
    let name = name.to_str()?;
    let stem = name.strip_suffix(".json")?;
    stem.parse().ok()
}

fn read_session_record(path: &Path) -> Option<SessionRecord> {
    let body = read_regular_file(path, MAX_REGISTRY_BYTES)?;
    parse_session_record(&body)
}

fn parse_session_record(body: &str) -> Option<SessionRecord> {
    let value: Value = serde_json::from_str(body).ok()?;
    let pid = u32::try_from(value.get("pid")?.as_u64()?).ok()?;
    let session_id = value.get("sessionId")?.as_str()?.to_string();
    let proc_start_value = value.get("procStart")?;
    let proc_start = match proc_start_value {
        Value::String(value) => value.parse().ok()?,
        Value::Number(value) => value.as_u64()?,
        _ => return None,
    };

    Some(SessionRecord {
        pid,
        session_id,
        proc_start,
    })
}

fn record_matches(
    record: &SessionRecord,
    file_pid: u32,
    session_id: &str,
    stat: &ProcessStat,
) -> bool {
    record.pid == file_pid
        && stat.pid == file_pid
        && record.session_id == session_id
        && record.proc_start == stat.start_time
}

fn unique_root(candidates: Vec<ResolvedRoot>) -> Option<ResolvedRoot> {
    let mut unique = None;

    for candidate in candidates {
        match unique {
            None => unique = Some(candidate),
            Some(existing) if existing == candidate => {}
            Some(_) => return None,
        }
    }

    unique
}

fn read_regular_file(path: &Path, max_bytes: u64) -> Option<String> {
    let mut options = OpenOptions::new();
    options.read(true).custom_flags(O_NOFOLLOW);
    let file = options.open(path).ok()?;
    let metadata = file.metadata().ok()?;

    if !metadata.file_type().is_file() {
        return None;
    }

    let mut bytes = Vec::new();
    file.take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > max_bytes {
        return None;
    }

    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::{parse_session_record, record_matches, unique_root, ResolvedRoot, SessionRecord};
    use crate::segments::claude_resource_usage::linux::process_stat::ProcessStat;

    fn process(pid: u32, start_time: u64) -> ProcessStat {
        ProcessStat {
            pid,
            ppid: 42,
            start_time,
            cpu_ticks: 26,
            rss_pages: 1234,
        }
    }

    #[test]
    fn parses_registry_proc_start_as_string_or_number() {
        let string_record =
            parse_session_record(r#"{"pid":77,"sessionId":"abc","procStart":"98765"}"#);
        let number_record =
            parse_session_record(r#"{"pid":77,"sessionId":"abc","procStart":98765}"#);
        let expected = Some(SessionRecord {
            pid: 77,
            session_id: "abc".to_string(),
            proc_start: 98765,
        });

        assert_eq!(string_record, expected);
        assert_eq!(
            number_record,
            Some(SessionRecord {
                pid: 77,
                session_id: "abc".to_string(),
                proc_start: 98765,
            })
        );
    }

    #[test]
    fn validates_all_registry_identity_fields() {
        let record = SessionRecord {
            pid: 77,
            session_id: "abc".to_string(),
            proc_start: 98765,
        };
        let stat = process(77, 98765);

        assert!(record_matches(&record, 77, "abc", &stat));
        assert!(!record_matches(&record, 78, "abc", &stat));
        assert!(!record_matches(&record, 77, "other", &stat));
        assert!(!record_matches(&record, 77, "abc", &process(77, 98766)));
    }

    #[test]
    fn registry_fallback_requires_one_live_identity() {
        let root = ResolvedRoot {
            pid: 77,
            start_time: 98765,
        };
        let other = ResolvedRoot {
            pid: 78,
            start_time: 98766,
        };

        assert_eq!(unique_root(vec![root]), Some(root));
        assert_eq!(unique_root(vec![root, root]), Some(root));
        assert_eq!(unique_root(vec![root, other]), None);
        assert_eq!(unique_root(Vec::new()), None);
    }
}
