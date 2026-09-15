use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::iso8601::parse_iso8601_utc;
use crate::transcript_forward_reader::read_records_forward;
use crate::transcript_spoken_text::{is_conversation_record, spoken_text};

const USER_ROW_TYPE: &str = "user";

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub(crate) struct TokenBuckets {
    pub(crate) input: u64,
    pub(crate) output: u64,
    pub(crate) thinking: u64,
    pub(crate) cache_write: u64,
    pub(crate) cache_read: u64,
}

impl TokenBuckets {
    pub(crate) fn total(&self) -> u64 {
        self.input
            .saturating_add(self.output)
            .saturating_add(self.cache_write)
            .saturating_add(self.cache_read)
    }

    pub(crate) fn add(&mut self, other: &TokenBuckets) {
        self.input = self.input.saturating_add(other.input);
        self.output = self.output.saturating_add(other.output);
        self.thinking = self.thinking.saturating_add(other.thinking);
        self.cache_write = self.cache_write.saturating_add(other.cache_write);
        self.cache_read = self.cache_read.saturating_add(other.cache_read);
    }

    fn remove(&mut self, other: &TokenBuckets) {
        self.input = self.input.saturating_sub(other.input);
        self.output = self.output.saturating_sub(other.output);
        self.thinking = self.thinking.saturating_sub(other.thinking);
        self.cache_write = self.cache_write.saturating_sub(other.cache_write);
        self.cache_read = self.cache_read.saturating_sub(other.cache_read);
    }
}

#[derive(Default, Deserialize, Serialize)]
pub(crate) struct TokenTally {
    by_model: BTreeMap<String, u64>,
    session: TokenBuckets,
    turn: TokenBuckets,
    turn_started_at: Option<i64>,
    last_response: Option<CountedResponse>,
}

#[derive(Deserialize, Serialize)]
struct CountedResponse {
    id: String,
    model: String,
    usage: TokenBuckets,
    in_turn: bool,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct TranscriptTally {
    pub(crate) path: String,
    pub(crate) scanned_bytes: u64,
    pub(crate) tally: TokenTally,
}

struct Response {
    id: Option<String>,
    model: String,
    usage: TokenBuckets,
    written_at: Option<i64>,
}

impl TokenTally {
    pub(crate) fn total(&self) -> u64 {
        self.session.total()
    }

    pub(crate) fn session(&self) -> &TokenBuckets {
        &self.session
    }

    pub(crate) fn turn(&self) -> &TokenBuckets {
        &self.turn
    }

    pub(crate) fn turn_started_at(&self) -> Option<i64> {
        self.turn_started_at
    }

    pub(crate) fn by_model(&self) -> impl Iterator<Item = (&str, u64)> {
        self.by_model
            .iter()
            .map(|(model, tokens)| (model.as_str(), *tokens))
    }

    fn start_turn(&mut self, started_at: i64) {
        if self.turn_started_at == Some(started_at) {
            return;
        }

        self.turn_started_at = Some(started_at);
        self.turn = TokenBuckets::default();

        if let Some(last) = self.last_response.as_mut() {
            last.in_turn = false;
        }
    }

    fn is_in_turn(&self, written_at: Option<i64>) -> bool {
        match (self.turn_started_at, written_at) {
            (Some(started_at), Some(written_at)) => written_at >= started_at,
            _ => false,
        }
    }

    fn count(&mut self, response: Response) {
        let repeated = self
            .last_response
            .take()
            .filter(|last| response.id.as_deref() == Some(last.id.as_str()));

        if let Some(last) = repeated {
            self.uncount(&last);
        }

        let in_turn = self.is_in_turn(response.written_at);

        *self.by_model.entry(response.model.clone()).or_insert(0) += response.usage.total();
        self.session.add(&response.usage);

        if in_turn {
            self.turn.add(&response.usage);
        }

        self.last_response = response.id.map(|id| CountedResponse {
            id,
            model: response.model,
            usage: response.usage,
            in_turn,
        });
    }

    fn uncount(&mut self, last: &CountedResponse) {
        if let Some(counted) = self.by_model.get_mut(&last.model) {
            *counted = counted.saturating_sub(last.usage.total());
        }

        self.session.remove(&last.usage);

        if last.in_turn {
            self.turn.remove(&last.usage);
        }
    }
}

pub(crate) fn refresh_tallies<'a>(
    transcripts: impl IntoIterator<Item = (&'a Path, u64)>,
    tallies: &mut Vec<TranscriptTally>,
    turn_started_at: Option<i64>,
) {
    let mut previous = std::mem::take(tallies);

    for (path, len) in transcripts {
        let key = path.to_string_lossy();
        let known = previous
            .iter()
            .position(|tally| tally.path == key)
            .map(|index| previous.swap_remove(index));

        tallies.push(refresh_tally(path, len, known, turn_started_at));
    }
}

pub(crate) fn refresh_tally(
    path: &Path,
    len: u64,
    known: Option<TranscriptTally>,
    turn_started_at: Option<i64>,
) -> TranscriptTally {
    let key = path.to_string_lossy().into_owned();
    let mut transcript = known
        .filter(|tally| tally.path == key && tally.scanned_bytes <= len)
        .unwrap_or_else(|| TranscriptTally {
            path: key,
            scanned_bytes: 0,
            tally: TokenTally::default(),
        });

    if let Some(started_at) = turn_started_at {
        transcript.tally.start_turn(started_at);
    }

    advance(path, transcript)
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
        let Ok(row) = serde_json::from_slice::<TranscriptRow>(record) else {
            return;
        };

        let written_at = row.timestamp.as_deref().and_then(parse_iso8601_utc);

        if let Some(response) = row
            .message
            .and_then(|message| message.into_response(written_at))
        {
            tally.count(response);
            return;
        }

        let Some(written_at) = written_at else {
            return;
        };

        if row.kind.as_deref() == Some(USER_ROW_TYPE) && is_prompt(record) {
            tally.start_turn(written_at);
        }
    })
}

fn is_prompt(record: &[u8]) -> bool {
    let Ok(row) = serde_json::from_slice::<Value>(record) else {
        return false;
    };

    if !is_conversation_record(&row) {
        return false;
    }

    row.get("message")
        .and_then(|message| message.get("content"))
        .and_then(spoken_text)
        .is_some()
}

#[derive(Deserialize)]
struct TranscriptRow {
    #[serde(rename = "type")]
    kind: Option<String>,
    timestamp: Option<String>,
    message: Option<UsageMessage>,
}

#[derive(Deserialize)]
struct UsageMessage {
    id: Option<String>,
    model: Option<String>,
    usage: Option<Usage>,
}

impl UsageMessage {
    fn into_response(self, written_at: Option<i64>) -> Option<Response> {
        let (Some(model), Some(usage)) = (self.model, self.usage) else {
            return None;
        };

        Some(Response {
            id: self.id,
            model,
            usage: usage.buckets(),
            written_at,
        })
    }
}

#[derive(Deserialize)]
struct Usage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
    output_tokens_details: Option<OutputTokensDetails>,
}

#[derive(Deserialize)]
struct OutputTokensDetails {
    thinking_tokens: Option<u64>,
}

impl Usage {
    fn buckets(&self) -> TokenBuckets {
        let thinking = self
            .output_tokens_details
            .as_ref()
            .and_then(|details| details.thinking_tokens);

        TokenBuckets {
            input: self.input_tokens.unwrap_or(0),
            output: self.output_tokens.unwrap_or(0),
            thinking: thinking.unwrap_or(0),
            cache_write: self.cache_creation_input_tokens.unwrap_or(0),
            cache_read: self.cache_read_input_tokens.unwrap_or(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{tally_tokens, TokenBuckets, TokenTally};
    use crate::iso8601::parse_iso8601_utc;

    const FIRST_TURN: &str = r#"{"type":"assistant","message":{"id":"msg_1","model":"claude-opus-5","usage":{"input_tokens":3,"output_tokens":5,"cache_creation_input_tokens":7,"cache_read_input_tokens":11}}}"#;
    const SECOND_TURN: &str = r#"{"type":"assistant","message":{"id":"msg_2","model":"claude-opus-5","usage":{"output_tokens":4}}}"#;

    fn tally_of(transcript: &str) -> (TokenTally, u64) {
        let mut tally = TokenTally::default();
        let scanned = tally_tokens(Cursor::new(transcript.as_bytes()), &mut tally);

        (tally, scanned)
    }

    fn rows(rows: &[&str]) -> String {
        rows.iter().map(|row| format!("{row}\n")).collect()
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
    fn a_prompt_starts_a_turn_that_tool_results_and_injected_rows_keep_going() {
        let transcript = rows(&[
            r#"{"type":"user","timestamp":"2026-09-15T08:00:00Z","message":{"role":"user","content":"first"}}"#,
            r#"{"type":"assistant","timestamp":"2026-09-15T08:00:05Z","message":{"id":"a","model":"claude-opus-5","usage":{"input_tokens":100}}}"#,
            r#"{"type":"user","timestamp":"2026-09-15T08:01:00Z","message":{"role":"user","content":"second"}}"#,
            r#"{"type":"assistant","timestamp":"2026-09-15T08:01:05Z","message":{"id":"b","model":"claude-opus-5","usage":{"input_tokens":10,"output_tokens":20,"output_tokens_details":{"thinking_tokens":15}}}}"#,
            r#"{"type":"user","timestamp":"2026-09-15T08:01:10Z","message":{"role":"user","content":[{"type":"tool_result","content":"ok"}]}}"#,
            r#"{"type":"user","timestamp":"2026-09-15T08:01:11Z","isMeta":true,"message":{"role":"user","content":"skill body"}}"#,
            r#"{"type":"user","timestamp":"2026-09-15T08:01:12Z","message":{"role":"user","content":"<task-notification>done</task-notification>"}}"#,
            r#"{"type":"assistant","timestamp":"2026-09-15T08:01:20Z","message":{"id":"c","model":"claude-opus-5","usage":{"cache_read_input_tokens":300,"cache_creation_input_tokens":40}}}"#,
        ]);

        let (tally, _) = tally_of(&transcript);

        assert_eq!(
            *tally.turn(),
            TokenBuckets {
                input: 10,
                output: 20,
                thinking: 15,
                cache_write: 40,
                cache_read: 300,
            }
        );
        assert_eq!(tally.session().total(), 470);
    }

    #[test]
    fn a_subagent_counts_toward_the_turn_only_from_the_moment_it_started() {
        let transcript = rows(&[
            r#"{"type":"user","isSidechain":true,"timestamp":"2026-09-15T08:00:50Z","message":{"role":"user","content":"explore the repo"}}"#,
            r#"{"type":"assistant","isSidechain":true,"timestamp":"2026-09-15T08:00:59Z","message":{"id":"old","model":"claude-haiku-4-5","usage":{"output_tokens":7}}}"#,
            r#"{"type":"assistant","isSidechain":true,"timestamp":"2026-09-15T08:01:30Z","message":{"id":"new","model":"claude-haiku-4-5","usage":{"output_tokens":9}}}"#,
        ]);
        let mut tally = TokenTally::default();
        tally.start_turn(parse_iso8601_utc("2026-09-15T08:01:00Z").unwrap());

        tally_tokens(Cursor::new(transcript.as_bytes()), &mut tally);

        assert_eq!(tally.turn().output, 9);
        assert_eq!(tally.session().output, 16);
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
