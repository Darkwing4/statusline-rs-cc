use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;

use crate::transcript_forward_reader::read_records_forward;

use super::session_cache::AgentFileState;

const SUBAGENTS_DIR: &str = "subagents";
const MAX_SCAN_DEPTH: usize = 3;

pub(super) struct SubagentFiles {
    pub(super) count: usize,
    pub(super) tokens: u64,
    pub(super) last_written_at: Option<i64>,
}

pub(super) fn collect(
    transcript_path: &str,
    count_tokens: bool,
    cached: &mut Vec<AgentFileState>,
) -> SubagentFiles {
    let mut transcripts = Vec::new();
    if let Some(root) = subagents_dir(transcript_path) {
        collect_transcripts(&root, MAX_SCAN_DEPTH, &mut transcripts);
    }

    let last_written_at = transcripts
        .iter()
        .filter_map(|entry| entry.modified_at)
        .max();
    let count = transcripts.len();

    if !count_tokens {
        return SubagentFiles {
            count,
            tokens: 0,
            last_written_at,
        };
    }

    let previous = std::mem::take(cached);
    let mut tokens = 0;

    for entry in &transcripts {
        let path = entry.path.to_string_lossy().into_owned();
        let known = previous
            .iter()
            .find(|state| state.path == path)
            .filter(|state| state.scanned_bytes <= entry.len);

        let (start, counted_so_far) = match known {
            Some(state) => (state.scanned_bytes, state.tokens),
            None => (0, 0),
        };

        let (added, scanned_bytes) = count_new_tokens(&entry.path, start);
        let total = counted_so_far + added;

        tokens += total;
        cached.push(AgentFileState {
            path,
            scanned_bytes,
            tokens: total,
        });
    }

    SubagentFiles {
        count,
        tokens,
        last_written_at,
    }
}

fn subagents_dir(transcript_path: &str) -> Option<PathBuf> {
    let transcript = Path::new(transcript_path);
    let session_dir = transcript.parent()?.join(transcript.file_stem()?);
    let subagents = session_dir.join(SUBAGENTS_DIR);

    subagents.is_dir().then_some(subagents)
}

struct TranscriptFile {
    path: PathBuf,
    len: u64,
    modified_at: Option<i64>,
}

fn collect_transcripts(dir: &Path, depth: usize, found: &mut Vec<TranscriptFile>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };

        if metadata.is_dir() {
            if depth > 1 {
                collect_transcripts(&path, depth - 1, found);
            }

            continue;
        }

        if path.extension().and_then(|extension| extension.to_str()) != Some("jsonl") {
            continue;
        }

        found.push(TranscriptFile {
            path,
            len: metadata.len(),
            modified_at: metadata.modified().ok().and_then(unix_seconds),
        });
    }
}

fn unix_seconds(time: SystemTime) -> Option<i64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok())
}

fn count_new_tokens(path: &Path, start: u64) -> (u64, u64) {
    let Ok(mut file) = File::open(path) else {
        return (0, start);
    };

    if start > 0 && file.seek(SeekFrom::Start(start)).is_err() {
        return (0, start);
    }

    let (tokens, scanned) = count_tokens_from(file);

    (tokens, start + scanned)
}

fn count_tokens_from<R: Read>(reader: R) -> (u64, u64) {
    let mut total = 0;

    let consumed = read_records_forward(reader, |record| {
        let Ok(row) = serde_json::from_slice::<UsageRow>(record) else {
            return;
        };

        let Some(usage) = row.message.and_then(|message| message.usage) else {
            return;
        };

        total += usage.input_tokens.unwrap_or(0)
            + usage.output_tokens.unwrap_or(0)
            + usage.cache_creation_input_tokens.unwrap_or(0)
            + usage.cache_read_input_tokens.unwrap_or(0);
    });

    (total, consumed)
}

#[derive(Deserialize)]
struct UsageRow {
    message: Option<UsageMessage>,
}

#[derive(Deserialize)]
struct UsageMessage {
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Usage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::count_tokens_from;

    const FIRST_TURN: &str = r#"{"type":"assistant","message":{"usage":{"input_tokens":3,"output_tokens":5,"cache_creation_input_tokens":7,"cache_read_input_tokens":11}}}"#;
    const SECOND_TURN: &str = r#"{"type":"assistant","message":{"usage":{"output_tokens":4}}}"#;

    #[test]
    fn sums_every_usage_bucket_and_skips_rows_without_usage() {
        let transcript = format!(
            "{}\n{FIRST_TURN}\n{SECOND_TURN}\n",
            r#"{"type":"user","message":{"role":"user","content":"start"}}"#
        );

        assert_eq!(count_tokens_from(Cursor::new(transcript.as_bytes())).0, 30);
    }

    #[test]
    fn reports_the_offset_of_the_last_complete_row() {
        let complete = format!("{FIRST_TURN}\n");
        let transcript = format!("{complete}{}", &SECOND_TURN[..30]);

        let (tokens, scanned) = count_tokens_from(Cursor::new(transcript.as_bytes()));

        assert_eq!(tokens, 26);
        assert!(scanned <= complete.len() as u64);
    }

    #[test]
    fn resuming_from_an_offset_counts_only_the_new_rows() {
        let head = format!("{FIRST_TURN}\n");
        let transcript = format!("{head}{SECOND_TURN}\n");

        let (head_tokens, scanned) = count_tokens_from(Cursor::new(head.as_bytes()));
        let mut tail = Cursor::new(transcript.as_bytes());
        tail.set_position(scanned);
        let (tail_tokens, _) = count_tokens_from(&mut tail);

        assert_eq!(
            head_tokens + tail_tokens,
            count_tokens_from(Cursor::new(transcript.as_bytes())).0
        );
    }
}
