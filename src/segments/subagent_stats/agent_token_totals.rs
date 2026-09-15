use crate::subagent_transcript_files::list_subagent_transcripts;
use crate::transcript_token_tally::{refresh_tallies, TranscriptTally};

pub(super) struct SubagentFiles {
    pub(super) count: usize,
    pub(super) tokens: u64,
    pub(super) last_written_at: Option<i64>,
}

pub(super) fn collect(
    transcript_path: &str,
    count_tokens: bool,
    cached: &mut Vec<TranscriptTally>,
) -> SubagentFiles {
    let transcripts = list_subagent_transcripts(transcript_path);

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

    refresh_tallies(
        transcripts
            .iter()
            .map(|entry| (entry.path.as_path(), entry.len)),
        cached,
    );
    let tokens = cached
        .iter()
        .map(|transcript| transcript.tally.total())
        .sum();

    SubagentFiles {
        count,
        tokens,
        last_written_at,
    }
}
