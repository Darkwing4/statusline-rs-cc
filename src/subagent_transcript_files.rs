use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const SUBAGENTS_DIR: &str = "subagents";
const MAX_SCAN_DEPTH: usize = 3;

pub(crate) struct TranscriptFile {
    pub(crate) path: PathBuf,
    pub(crate) len: u64,
    pub(crate) modified_at: Option<i64>,
}

pub(crate) fn list_subagent_transcripts(transcript_path: &str) -> Vec<TranscriptFile> {
    let mut transcripts = Vec::new();

    if let Some(root) = subagents_dir(transcript_path) {
        collect_transcripts(&root, MAX_SCAN_DEPTH, &mut transcripts);
    }

    transcripts
}

fn subagents_dir(transcript_path: &str) -> Option<PathBuf> {
    let transcript = Path::new(transcript_path);
    let session_dir = transcript.parent()?.join(transcript.file_stem()?);
    let subagents = session_dir.join(SUBAGENTS_DIR);

    subagents.is_dir().then_some(subagents)
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
