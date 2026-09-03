use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::process_stat::{self, ProcessStat};

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
    resolve_from_ancestry(std::process::id(), session_id, process_stat::read, |pid| {
        read_session_record(&sessions.join(format!("{pid}.json")))
    })
    .or_else(|| resolve_from_registry(session_id, &sessions))
}

fn session_directory() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("CLAUDE_CONFIG_DIR").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(path).join("sessions"));
    }

    let home = std::env::var_os("HOME").filter(|value| !value.is_empty())?;
    Some(PathBuf::from(home).join(".claude").join("sessions"))
}

fn resolve_from_ancestry<ReadStat, ReadRecord>(
    start_pid: u32,
    session_id: &str,
    mut read_stat: ReadStat,
    mut read_record: ReadRecord,
) -> Option<ResolvedRoot>
where
    ReadStat: FnMut(u32) -> Option<ProcessStat>,
    ReadRecord: FnMut(u32) -> Option<SessionRecord>,
{
    let mut pid = start_pid;
    let mut visited = HashSet::new();

    while pid != 0 && visited.insert(pid) {
        let stat = match read_stat(pid) {
            Some(stat) => stat,
            None => break,
        };

        if let Some(record) = read_record(pid) {
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
    let body = super::read_regular_file(path, MAX_REGISTRY_BYTES, None)?;
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
    let mut candidates = candidates.into_iter();
    let first = candidates.next()?;

    candidates
        .all(|candidate| candidate == first)
        .then_some(first)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        parse_session_record, record_matches, resolve_from_ancestry, unique_root, ResolvedRoot,
        SessionRecord,
    };
    use crate::process_stat::ProcessStat;

    fn process(pid: u32, start_time: u64) -> ProcessStat {
        ProcessStat {
            pid,
            ppid: 42,
            start_time,
            cpu_ticks: 26,
            rss_pages: 1234,
        }
    }

    fn ancestor(pid: u32, ppid: u32, start_time: u64) -> ProcessStat {
        ProcessStat {
            pid,
            ppid,
            start_time,
            cpu_ticks: 26,
            rss_pages: 1234,
        }
    }

    fn record(session_id: &str) -> SessionRecord {
        SessionRecord {
            pid: 100,
            session_id: session_id.to_string(),
            proc_start: 98765,
        }
    }

    #[test]
    fn resolves_root_from_ancestor_with_matching_record() {
        let stats = HashMap::from([
            (300, ancestor(300, 200, 3)),
            (200, ancestor(200, 100, 2)),
            (100, ancestor(100, 1, 98765)),
            (1, ancestor(1, 0, 1)),
        ]);
        let read_stat = |pid: u32| stats.get(&pid).cloned();

        assert_eq!(
            resolve_from_ancestry(300, "abc", read_stat, |pid| {
                (pid == 100).then(|| record("abc"))
            }),
            Some(ResolvedRoot {
                pid: 100,
                start_time: 98765,
            })
        );
        assert_eq!(
            resolve_from_ancestry(300, "abc", read_stat, |pid| {
                (pid == 100).then(|| record("other"))
            }),
            None
        );
        assert_eq!(resolve_from_ancestry(300, "abc", read_stat, |_| None), None);
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
