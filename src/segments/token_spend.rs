use serde_json::Value;

pub use crate::config_schema::TokenSpend;
use crate::segments::{GitCache, Segment};
use crate::session_token_tallies::load_session_tallies;
use crate::token_count_format::format_tokens;
use crate::transcript_token_tally::{TokenBuckets, TranscriptTally};

const TURN_LABEL: &str = "turn ";
const SESSION_LABEL: &str = "session ";
const DETAILS_GAP: &str = "  ";
const DETAIL_SEPARATOR: &str = " · ";
const SCOPE_SEPARATOR: &str = " │ ";

struct Spent {
    turn: TokenBuckets,
    session: TokenBuckets,
}

impl Segment for TokenSpend {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let tallies = load_session_tallies(json)?;

        self.format(&spent(tallies.transcripts()))
    }
}

impl TokenSpend {
    fn format(&self, spent: &Spent) -> Option<String> {
        if spent.session.total() == 0 {
            return None;
        }

        let session = self.block(SESSION_LABEL, &spent.session);

        if spent.turn.total() == 0 || spent.turn == spent.session {
            return Some(session);
        }

        Some(format!(
            "{}{}{}",
            self.block(TURN_LABEL, &spent.turn),
            self.label_color.paint(SCOPE_SEPARATOR),
            session
        ))
    }

    fn block(&self, label: &str, spent: &TokenBuckets) -> String {
        let mut details = vec![self.metric("out ", spent.output)];

        if spent.thinking > 0 {
            details.push(self.metric("think ", spent.thinking));
        }

        details.push(self.metric("in ", spent.input));

        if spent.cache_read > 0 || spent.cache_write > 0 {
            details.push(self.cache(spent));
        }

        format!(
            "{}{}{}",
            self.metric(label, spent.total()),
            DETAILS_GAP,
            details.join(&self.label_color.paint(DETAIL_SEPARATOR))
        )
    }

    fn cache(&self, spent: &TokenBuckets) -> String {
        let mut text = self.metric("cache ", spent.cache_read);

        if spent.cache_write > 0 {
            text.push(' ');
            text.push_str(
                &self
                    .color
                    .paint(&format!("+{}", format_tokens(spent.cache_write))),
            );
        }

        text
    }

    fn metric(&self, label: &str, tokens: u64) -> String {
        format!(
            "{}{}",
            self.label_color.paint(label),
            self.color.paint(&format_tokens(tokens))
        )
    }
}

fn spent<'a>(transcripts: impl IntoIterator<Item = &'a TranscriptTally>) -> Spent {
    let mut spent = Spent {
        turn: TokenBuckets::default(),
        session: TokenBuckets::default(),
    };

    for transcript in transcripts {
        spent.turn.add(transcript.tally.turn());
        spent.session.add(transcript.tally.session());
    }

    spent
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{spent, Spent, TokenSpend};
    use crate::config_schema::Color;
    use crate::transcript_token_tally::{tally_tokens, TokenBuckets, TokenTally, TranscriptTally};

    const FIRST_TURN: TokenBuckets = TokenBuckets {
        input: 34,
        output: 1_200,
        thinking: 408,
        cache_write: 26_000,
        cache_read: 51_000,
    };

    fn plain() -> TokenSpend {
        TokenSpend {
            color: Color::Gradient,
            label_color: Color::Gradient,
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
    fn shows_the_turn_then_the_session_with_every_bucket() {
        let spent = Spent {
            turn: FIRST_TURN,
            session: TokenBuckets {
                input: 2_100,
                output: 96_000,
                thinking: 41_000,
                cache_write: 110_000,
                cache_read: 1_200_000,
            },
        };

        assert_eq!(
            plain().format(&spent).as_deref(),
            Some("turn 78k  out 1.2k · think 408 · in 34 · cache 51k +26k │ session 1.4M  out 96k · think 41k · in 2.1k · cache 1.2M +110k")
        );
    }

    #[test]
    fn shows_one_block_while_the_turn_is_the_whole_session_or_has_not_spent_yet() {
        let first = Spent {
            turn: FIRST_TURN,
            session: FIRST_TURN,
        };
        let waiting = Spent {
            turn: TokenBuckets::default(),
            session: FIRST_TURN,
        };
        let expected = "session 78k  out 1.2k · think 408 · in 34 · cache 51k +26k";

        assert_eq!(plain().format(&first).as_deref(), Some(expected));
        assert_eq!(plain().format(&waiting).as_deref(), Some(expected));
    }

    #[test]
    fn leaves_out_reasoning_and_cache_writes_that_did_not_happen() {
        let spent = Spent {
            turn: TokenBuckets::default(),
            session: TokenBuckets {
                input: 2,
                output: 446,
                cache_read: 11_000,
                ..TokenBuckets::default()
            },
        };

        assert_eq!(
            plain().format(&spent).as_deref(),
            Some("session 11k  out 446 · in 2 · cache 11k")
        );
    }

    #[test]
    fn hides_itself_before_any_tokens_are_spent() {
        let spent = Spent {
            turn: TokenBuckets::default(),
            session: TokenBuckets::default(),
        };

        assert_eq!(plain().format(&spent), None);
    }

    #[test]
    fn paints_counts_and_labels_in_their_own_colours() {
        let segment = TokenSpend {
            color: Color::Named(97),
            label_color: Color::Named(90),
        };
        let spent = Spent {
            turn: TokenBuckets::default(),
            session: TokenBuckets {
                output: 5,
                ..TokenBuckets::default()
            },
        };

        assert_eq!(
            segment.format(&spent).as_deref(),
            Some("\x1b[90msession \x1b[0m\x1b[97m5\x1b[0m  \x1b[90mout \x1b[0m\x1b[97m5\x1b[0m\x1b[90m · \x1b[0m\x1b[90min \x1b[0m\x1b[97m0\x1b[0m")
        );
    }

    #[test]
    fn adds_the_main_thread_and_subagents_up() {
        let main = transcript(&[
            r#"{"type":"user","timestamp":"2026-09-15T08:00:00Z","message":{"role":"user","content":"first"}}"#,
            r#"{"type":"assistant","timestamp":"2026-09-15T08:00:05Z","message":{"id":"a","model":"claude-opus-5","usage":{"input_tokens":100}}}"#,
            r#"{"type":"user","timestamp":"2026-09-15T08:01:00Z","message":{"role":"user","content":"second"}}"#,
            r#"{"type":"assistant","timestamp":"2026-09-15T08:01:05Z","message":{"id":"b","model":"claude-opus-5","usage":{"output_tokens":20}}}"#,
        ]);
        let subagent = transcript(&[
            r#"{"type":"assistant","isSidechain":true,"timestamp":"2026-09-15T07:59:30Z","message":{"id":"s","model":"claude-haiku-4-5","usage":{"cache_read_input_tokens":7}}}"#,
        ]);

        let total = spent(&[main, subagent]);

        assert_eq!(
            total.turn,
            TokenBuckets {
                output: 20,
                ..TokenBuckets::default()
            }
        );
        assert_eq!(
            total.session,
            TokenBuckets {
                input: 100,
                output: 20,
                cache_read: 7,
                ..TokenBuckets::default()
            }
        );
    }
}
