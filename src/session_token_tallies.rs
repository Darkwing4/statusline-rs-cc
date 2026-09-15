mod tally_cache;

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::statusline_input::session_key;
use crate::subagent_transcript_files::list_subagent_transcripts;
use crate::transcript_token_tally::{refresh_tallies, refresh_tally, TranscriptTally};

#[derive(Default, Deserialize, Serialize)]
pub(crate) struct SessionTallies {
    main: Option<TranscriptTally>,
    subagents: Vec<TranscriptTally>,
}

impl SessionTallies {
    pub(crate) fn transcripts(&self) -> impl Iterator<Item = &TranscriptTally> {
        self.main.iter().chain(&self.subagents)
    }
}

pub(crate) fn load_session_tallies(json: &Value) -> Option<SessionTallies> {
    let transcript = json.get("transcript_path")?.as_str()?;
    let transcript_len = fs::metadata(transcript).ok()?.len();
    let subagents = list_subagent_transcripts(transcript);
    let (mut tallies, cache_path) = tally_cache::load(&session_key(json)?);

    let main = refresh_tally(
        Path::new(transcript),
        transcript_len,
        tallies.main.take(),
        None,
    );

    refresh_tallies(
        subagents.iter().map(|file| (file.path.as_path(), file.len)),
        &mut tallies.subagents,
        main.tally.turn_started_at(),
    );
    tallies.main = Some(main);

    if let Some(path) = cache_path.as_ref() {
        tally_cache::store(path, &tallies);
    }

    Some(tallies)
}
