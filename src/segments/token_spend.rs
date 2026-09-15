use serde_json::Value;

use crate::config_schema::TokenScope;
pub use crate::config_schema::TokenSpend;
use crate::segments::{GitCache, Segment};
use crate::session_token_tallies::load_session_tallies;
use crate::token_count_format::format_tokens;
use crate::transcript_token_tally::{TokenBuckets, TranscriptTally};

impl Segment for TokenSpend {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let tallies = load_session_tallies(json)?;
        let text = self.format(&self.spent(tallies.transcripts()))?;

        Some(self.color.paint(&text))
    }
}

impl TokenSpend {
    fn spent<'a>(
        &self,
        transcripts: impl IntoIterator<Item = &'a TranscriptTally>,
    ) -> TokenBuckets {
        let mut spent = TokenBuckets::default();

        for transcript in transcripts {
            let buckets = match self.scope {
                TokenScope::LastTurn => transcript.tally.turn(),
                TokenScope::Session => transcript.tally.session(),
            };
            spent.add(buckets);
        }

        spent
    }

    fn format(&self, spent: &TokenBuckets) -> Option<String> {
        if spent.total() == 0 {
            return None;
        }

        Some(format!(
            "{}{}: in {} out {} think {} cache read {} write {}",
            self.prefix,
            format_tokens(spent.total()),
            format_tokens(spent.input),
            format_tokens(spent.output),
            format_tokens(spent.thinking),
            format_tokens(spent.cache_read),
            format_tokens(spent.cache_write),
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::TokenSpend;
    use crate::config_schema::{Color, TokenScope};
    use crate::transcript_token_tally::{tally_tokens, TokenBuckets, TokenTally, TranscriptTally};

    fn spend(scope: TokenScope) -> TokenSpend {
        TokenSpend {
            scope,
            color: Color::Named(90),
            prefix: "turn ".to_string(),
        }
    }

    fn transcript(rows: &[&str]) -> TranscriptTally {
        let body: String = rows.iter().map(|row| format!("{row}\n")).collect();
        let mut tally = TokenTally::default();
        tally_tokens(Cursor::new(body.as_bytes()), &mut tally);

        TranscriptTally {
            path: String::new(),
            scanned_bytes: 0,
            tally,
        }
    }

    #[test]
    fn spells_out_every_bucket_after_the_total() {
        let spent = TokenBuckets {
            input: 1_200,
            output: 3_400,
            thinking: 1_100,
            cache_write: 1_900,
            cache_read: 51_600,
        };

        assert_eq!(
            spend(TokenScope::LastTurn).format(&spent).as_deref(),
            Some("turn 58k: in 1.2k out 3.4k think 1.1k cache read 52k write 1.9k")
        );
    }

    #[test]
    fn hides_itself_before_any_tokens_are_spent() {
        assert_eq!(
            spend(TokenScope::Session).format(&TokenBuckets::default()),
            None
        );
    }

    #[test]
    fn adds_the_main_thread_and_subagents_up_within_the_scope() {
        let main = transcript(&[
            r#"{"type":"user","timestamp":"2026-09-15T08:00:00Z","message":{"role":"user","content":"first"}}"#,
            r#"{"type":"assistant","timestamp":"2026-09-15T08:00:05Z","message":{"id":"a","model":"claude-opus-5","usage":{"input_tokens":100}}}"#,
            r#"{"type":"user","timestamp":"2026-09-15T08:01:00Z","message":{"role":"user","content":"second"}}"#,
            r#"{"type":"assistant","timestamp":"2026-09-15T08:01:05Z","message":{"id":"b","model":"claude-opus-5","usage":{"output_tokens":20}}}"#,
        ]);
        let subagent = transcript(&[
            r#"{"type":"assistant","isSidechain":true,"timestamp":"2026-09-15T07:59:30Z","message":{"id":"s","model":"claude-haiku-4-5","usage":{"cache_read_input_tokens":7}}}"#,
        ]);
        let transcripts = [main, subagent];

        assert_eq!(
            spend(TokenScope::LastTurn).spent(&transcripts),
            TokenBuckets {
                output: 20,
                ..TokenBuckets::default()
            }
        );
        assert_eq!(
            spend(TokenScope::Session).spent(&transcripts),
            TokenBuckets {
                input: 100,
                output: 20,
                cache_read: 7,
                ..TokenBuckets::default()
            }
        );
    }
}
