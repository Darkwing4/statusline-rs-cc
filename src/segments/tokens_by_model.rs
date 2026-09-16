use std::cmp::Reverse;
use std::collections::BTreeMap;

use serde_json::Value;

pub use crate::config_schema::TokensByModel;
use crate::segments::{GitCache, Segment};
use crate::session_token_tallies::load_session_tallies;
use crate::token_count_format::format_tokens;
use crate::transcript_token_tally::TranscriptTally;

const MODEL_ID_PREFIX: &str = "claude-";

impl Segment for TokensByModel {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let tallies = load_session_tallies(json)?;
        let text = self.format(&tokens_per_model(tallies.transcripts()))?;

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

fn tokens_per_model<'a>(
    transcripts: impl IntoIterator<Item = &'a TranscriptTally>,
) -> Vec<(String, u64)> {
    let mut totals: BTreeMap<String, u64> = BTreeMap::new();

    for transcript in transcripts {
        for (model, tokens) in transcript.tally.by_model() {
            *totals.entry(model_family(model)).or_insert(0) += tokens;
        }
    }

    let mut ranked: Vec<(String, u64)> = totals
        .into_iter()
        .filter(|(_, tokens)| *tokens > 0)
        .collect();
    ranked.sort_by_key(|(_, tokens)| Reverse(*tokens));

    ranked
}

fn model_family(model: &str) -> String {
    let Some(name) = model.strip_prefix(MODEL_ID_PREFIX) else {
        return model.to_string();
    };

    let family = name.split('-').next().unwrap_or(name);
    let mut letters = family.chars();

    match letters.next() {
        Some(first) => first.to_uppercase().chain(letters).collect(),
        None => model.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{model_family, tokens_per_model, TokensByModel};
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
            ("Opus".to_string(), 3_400_000),
            ("Haiku".to_string(), 45_000),
        ];

        assert_eq!(
            sample().format(&ranked).as_deref(),
            Some("tokens Opus 3.4M · Haiku 45k")
        );
    }

    #[test]
    fn hides_itself_before_any_tokens_are_spent() {
        assert_eq!(sample().format(&[]), None);
    }

    #[test]
    fn merges_model_versions_ranks_by_tokens_and_drops_empty_models() {
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
            vec![("Haiku".to_string(), 1_200), ("Opus".to_string(), 900)]
        );
    }

    #[test]
    fn keeps_only_the_model_family_and_leaves_other_ids_alone() {
        assert_eq!(model_family("claude-opus-5"), "Opus");
        assert_eq!(model_family("claude-haiku-4-5-20251001"), "Haiku");
        assert_eq!(model_family("claude-fable-5-1"), "Fable");
        assert_eq!(model_family("gpt-5.6-sol"), "gpt-5.6-sol");
    }
}
