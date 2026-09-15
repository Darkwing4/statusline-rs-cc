mod tally_cache;

use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::fs;
use std::iter;
use std::path::Path;

use serde_json::Value;

pub use crate::config_schema::TokensByModel;
use crate::segments::{GitCache, Segment};
use crate::statusline_input::session_key;
use crate::subagent_transcript_files::list_subagent_transcripts;
use crate::token_count_format::format_tokens;
use crate::transcript_token_tally::{refresh_tallies, TranscriptTally};

const MODEL_ID_PREFIX: &str = "claude-";
const RELEASE_DATE_DIGITS: usize = 8;

impl Segment for TokensByModel {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let transcript = json.get("transcript_path")?.as_str()?;
        let tallies = session_tallies(transcript, &session_key(json)?)?;
        let text = self.format(&tokens_per_model(&tallies))?;

        Some(self.color.paint(&text))
    }
}

impl TokensByModel {
    fn format(&self, ranked: &[(String, u64)]) -> Option<String> {
        if ranked.is_empty() {
            return None;
        }

        let parts: Vec<String> = ranked
            .iter()
            .map(|(model, tokens)| format!("{} {}", model, format_tokens(*tokens)))
            .collect();

        Some(format!("{}{}", self.prefix, parts.join(&self.separator)))
    }
}

fn session_tallies(transcript: &str, session_key: &str) -> Option<Vec<TranscriptTally>> {
    let transcript_len = fs::metadata(transcript).ok()?.len();
    let subagents = list_subagent_transcripts(transcript);
    let (mut tallies, cache_path) = tally_cache::load(session_key);

    let transcripts = iter::once((Path::new(transcript), transcript_len))
        .chain(subagents.iter().map(|file| (file.path.as_path(), file.len)));
    refresh_tallies(transcripts, &mut tallies);

    if let Some(path) = cache_path.as_ref() {
        tally_cache::store(path, &tallies);
    }

    Some(tallies)
}

fn tokens_per_model(tallies: &[TranscriptTally]) -> Vec<(String, u64)> {
    let mut totals: BTreeMap<&str, u64> = BTreeMap::new();

    for transcript in tallies {
        for (model, tokens) in transcript.tally.by_model() {
            *totals.entry(short_model_name(model)).or_insert(0) += tokens;
        }
    }

    let mut ranked: Vec<(String, u64)> = totals
        .into_iter()
        .filter(|(_, tokens)| *tokens > 0)
        .map(|(model, tokens)| (model.to_string(), tokens))
        .collect();
    ranked.sort_by_key(|(_, tokens)| Reverse(*tokens));

    ranked
}

fn short_model_name(model: &str) -> &str {
    let name = model.strip_prefix(MODEL_ID_PREFIX).unwrap_or(model);

    match name.rsplit_once('-') {
        Some((family, date)) if is_release_date(date) => family,
        _ => name,
    }
}

fn is_release_date(part: &str) -> bool {
    part.len() == RELEASE_DATE_DIGITS && part.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{short_model_name, tokens_per_model, TokensByModel};
    use crate::config_schema::Color;
    use crate::transcript_token_tally::{tally_tokens, TokenTally, TranscriptTally};

    fn sample() -> TokensByModel {
        TokensByModel {
            color: Color::Named(90),
            prefix: "tokens ".to_string(),
            separator: " · ".to_string(),
        }
    }

    fn transcript(rows: &str) -> TranscriptTally {
        let mut tally = TokenTally::default();
        tally_tokens(Cursor::new(rows.as_bytes()), &mut tally);

        TranscriptTally {
            path: String::new(),
            scanned_bytes: 0,
            tally,
        }
    }

    #[test]
    fn lists_models_from_the_most_tokens_spent() {
        let ranked = vec![
            ("opus-5".to_string(), 3_400_000),
            ("haiku-4-5".to_string(), 45_000),
        ];

        assert_eq!(
            sample().format(&ranked).as_deref(),
            Some("tokens opus-5 3.4M · haiku-4-5 45k")
        );
    }

    #[test]
    fn hides_itself_before_any_tokens_are_spent() {
        assert_eq!(sample().format(&[]), None);
    }

    #[test]
    fn merges_dated_model_ids_ranks_by_tokens_and_drops_empty_models() {
        let main = transcript(concat!(
            r#"{"message":{"id":"m1","model":"claude-opus-5","usage":{"input_tokens":900}}}"#,
            "\n",
            r#"{"message":{"id":"m2","model":"<synthetic>","usage":{"input_tokens":0}}}"#,
            "\n",
        ));
        let subagent = transcript(concat!(
            r#"{"message":{"id":"m3","model":"claude-haiku-4-5-20251001","usage":{"output_tokens":700}}}"#,
            "\n",
            r#"{"message":{"id":"m4","model":"claude-haiku-4-5","usage":{"output_tokens":500}}}"#,
            "\n",
        ));

        assert_eq!(
            tokens_per_model(&[main, subagent]),
            vec![
                ("haiku-4-5".to_string(), 1_200),
                ("opus-5".to_string(), 900)
            ]
        );
    }

    #[test]
    fn shortens_model_ids_to_family_and_version() {
        assert_eq!(short_model_name("claude-opus-5"), "opus-5");
        assert_eq!(short_model_name("claude-haiku-4-5-20251001"), "haiku-4-5");
        assert_eq!(short_model_name("claude-fable-5-1"), "fable-5-1");
        assert_eq!(short_model_name("gpt-5.6-sol"), "gpt-5.6-sol");
    }
}
