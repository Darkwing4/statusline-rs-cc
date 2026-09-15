use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::transcript_forward_reader::read_records_forward;

#[derive(Default, Deserialize, Serialize)]
pub(crate) struct TokenTally {
    by_model: BTreeMap<String, u64>,
    last_response: Option<CountedResponse>,
}

#[derive(Deserialize, Serialize)]
struct CountedResponse {
    id: String,
    model: String,
    tokens: u64,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct TranscriptTally {
    pub(crate) path: String,
    pub(crate) scanned_bytes: u64,
    pub(crate) tally: TokenTally,
}

impl TokenTally {
    pub(crate) fn total(&self) -> u64 {
        self.by_model.values().sum()
    }

    pub(crate) fn by_model(&self) -> impl Iterator<Item = (&str, u64)> {
        self.by_model
            .iter()
            .map(|(model, tokens)| (model.as_str(), *tokens))
    }

    fn count(&mut self, id: Option<String>, model: String, tokens: u64) {
        let repeated = self
            .last_response
            .as_ref()
            .filter(|last| id.as_deref() == Some(last.id.as_str()));

        if let Some(last) = repeated {
            if let Some(counted) = self.by_model.get_mut(&last.model) {
                *counted = counted.saturating_sub(last.tokens);
            }
        }

        *self.by_model.entry(model.clone()).or_insert(0) += tokens;
        self.last_response = id.map(|id| CountedResponse { id, model, tokens });
    }
}

pub(crate) fn refresh_tallies<'a>(
    transcripts: impl IntoIterator<Item = (&'a Path, u64)>,
    tallies: &mut Vec<TranscriptTally>,
) {
    let mut previous = std::mem::take(tallies);

    for (path, len) in transcripts {
        let key = path.to_string_lossy().into_owned();
        let known = previous
            .iter()
            .position(|tally| tally.path == key)
            .map(|index| previous.swap_remove(index))
            .filter(|tally| tally.scanned_bytes <= len);

        let start = known.unwrap_or_else(|| TranscriptTally {
            path: key,
            scanned_bytes: 0,
            tally: TokenTally::default(),
        });

        tallies.push(advance(path, start));
    }
}

fn advance(path: &Path, mut transcript: TranscriptTally) -> TranscriptTally {
    let Ok(mut file) = File::open(path) else {
        return transcript;
    };

    if transcript.scanned_bytes > 0
        && file
            .seek(SeekFrom::Start(transcript.scanned_bytes))
            .is_err()
    {
        return transcript;
    }

    transcript.scanned_bytes += tally_tokens(file, &mut transcript.tally);

    transcript
}

pub(crate) fn tally_tokens<R: Read>(reader: R, tally: &mut TokenTally) -> u64 {
    read_records_forward(reader, |record| {
        let Ok(row) = serde_json::from_slice::<UsageRow>(record) else {
            return;
        };

        let Some(message) = row.message else {
            return;
        };

        let (Some(model), Some(usage)) = (message.model, message.usage) else {
            return;
        };

        tally.count(message.id, model, usage.total());
    })
}

#[derive(Deserialize)]
struct UsageRow {
    message: Option<UsageMessage>,
}

#[derive(Deserialize)]
struct UsageMessage {
    id: Option<String>,
    model: Option<String>,
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Usage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
}

impl Usage {
    fn total(&self) -> u64 {
        self.input_tokens.unwrap_or(0)
            + self.output_tokens.unwrap_or(0)
            + self.cache_creation_input_tokens.unwrap_or(0)
            + self.cache_read_input_tokens.unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{tally_tokens, TokenTally};

    const FIRST_TURN: &str = r#"{"type":"assistant","message":{"id":"msg_1","model":"claude-opus-5","usage":{"input_tokens":3,"output_tokens":5,"cache_creation_input_tokens":7,"cache_read_input_tokens":11}}}"#;
    const SECOND_TURN: &str = r#"{"type":"assistant","message":{"id":"msg_2","model":"claude-opus-5","usage":{"output_tokens":4}}}"#;

    fn tally_of(transcript: &str) -> (TokenTally, u64) {
        let mut tally = TokenTally::default();
        let scanned = tally_tokens(Cursor::new(transcript.as_bytes()), &mut tally);

        (tally, scanned)
    }

    #[test]
    fn sums_every_usage_bucket_and_skips_rows_without_usage() {
        let transcript = format!(
            "{}\n{FIRST_TURN}\n{SECOND_TURN}\n",
            r#"{"type":"user","message":{"role":"user","content":"start"}}"#
        );

        assert_eq!(tally_of(&transcript).0.total(), 30);
    }

    #[test]
    fn counts_a_response_once_although_every_content_block_repeats_its_usage() {
        let transcript = concat!(
            r#"{"type":"assistant","message":{"id":"msg_1","model":"claude-opus-5","content":[{"type":"thinking"}],"usage":{"input_tokens":2,"output_tokens":1,"cache_read_input_tokens":100}}}"#,
            "\n",
            r#"{"type":"assistant","message":{"id":"msg_1","model":"claude-opus-5","content":[{"type":"tool_use"}],"usage":{"input_tokens":2,"output_tokens":40,"cache_read_input_tokens":100}}}"#,
            "\n",
            r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result"}]}}"#,
            "\n",
            r#"{"type":"assistant","message":{"id":"msg_1","model":"claude-opus-5","content":[{"type":"text"}],"usage":{"input_tokens":2,"output_tokens":55,"cache_read_input_tokens":100}}}"#,
            "\n",
        );

        assert_eq!(tally_of(transcript).0.total(), 157);
    }

    #[test]
    fn splits_tokens_by_model() {
        let transcript = format!(
            "{FIRST_TURN}\n{}\n",
            r#"{"type":"assistant","message":{"id":"msg_9","model":"claude-haiku-4-5-20251001","usage":{"input_tokens":10,"output_tokens":2}}}"#
        );
        let (tally, _) = tally_of(&transcript);

        assert_eq!(
            tally.by_model().collect::<Vec<_>>(),
            vec![("claude-haiku-4-5-20251001", 12), ("claude-opus-5", 26)]
        );
    }

    #[test]
    fn reports_the_offset_of_the_last_complete_row() {
        let complete = format!("{FIRST_TURN}\n");
        let transcript = format!("{complete}{}", &SECOND_TURN[..30]);

        let (tally, scanned) = tally_of(&transcript);

        assert_eq!(tally.total(), 26);
        assert!(scanned <= complete.len() as u64);
    }

    #[test]
    fn resuming_from_an_offset_still_counts_a_split_response_once() {
        let head = format!("{FIRST_TURN}\n");
        let repeated = FIRST_TURN.replace(r#""output_tokens":5"#, r#""output_tokens":9"#);
        let transcript = format!("{head}{repeated}\n{SECOND_TURN}\n");

        let mut resumed = TokenTally::default();
        let scanned = tally_tokens(Cursor::new(head.as_bytes()), &mut resumed);
        let mut tail = Cursor::new(transcript.as_bytes());
        tail.set_position(scanned);
        tally_tokens(&mut tail, &mut resumed);

        assert_eq!(resumed.total(), 34);
        assert_eq!(resumed.total(), tally_of(&transcript).0.total());
    }
}
