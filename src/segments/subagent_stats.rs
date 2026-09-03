mod agent_lifecycle;
mod agent_token_totals;
mod session_cache;

use std::fs::{self, File};
use std::io::{Seek, SeekFrom};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

pub use crate::config_schema::SubagentStats;
use crate::duration_format::format_duration_padded;
use crate::segments::{GitCache, Segment};
use crate::statusline_input::session_key;

use self::agent_lifecycle::scan_agent_activity;
use self::agent_token_totals::collect;
use self::session_cache::TranscriptState;

struct Stats {
    active: usize,
    total: usize,
    longest_active_seconds: Option<i64>,
    tokens: u64,
    stalled: bool,
}

impl Segment for SubagentStats {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let transcript = json.get("transcript_path")?.as_str()?;
        let stats = self.collect_stats(transcript, &session_key(json)?)?;

        let color = if stats.stalled {
            self.stall_color
        } else if stats.active > 0 {
            self.active_color
        } else {
            self.color
        };

        Some(color.paint(&self.format(&stats)))
    }
}

impl SubagentStats {
    fn collect_stats(&self, transcript: &str, session_key: &str) -> Option<Stats> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;
        let (mut cache, cache_path) = session_cache::load(session_key);

        let mut file = File::open(transcript).ok()?;
        rewind_stale_cache(&mut cache.transcript, transcript);
        if file
            .seek(SeekFrom::Start(cache.transcript.scanned_bytes))
            .is_err()
        {
            cache.transcript = TranscriptState::default();
        }

        let scanned = scan_agent_activity(&mut file, &mut cache.transcript);
        cache.transcript.scanned_bytes += scanned;

        let files = collect(transcript, self.show_tokens, &mut cache.agent_files);

        if let Some(path) = cache_path.as_ref() {
            session_cache::store(path, &cache);
        }

        let total = cache.transcript.launched.max(files.count);
        if total == 0 {
            return None;
        }

        let oldest_start = cache
            .transcript
            .pending
            .iter()
            .filter_map(|agent| agent.started_at)
            .min();
        let last_signal = files.last_written_at.or(oldest_start);
        let active = cache.transcript.pending.len();

        Some(Stats {
            active,
            total,
            longest_active_seconds: oldest_start.map(|started| (now - started).max(0)),
            tokens: files.tokens,
            stalled: active > 0 && is_stalled(now, last_signal, self.stall_seconds),
        })
    }

    fn format(&self, stats: &Stats) -> String {
        let mut text = if stats.active > 0 {
            format!("{}{}/{}", self.prefix, stats.active, stats.total)
        } else {
            format!("{}{}", self.prefix, stats.total)
        };

        if stats.stalled {
            text.push_str(&self.stall_marker);
        }

        if let Some(seconds) = stats.longest_active_seconds {
            text.push(' ');
            text.push_str(&format_duration_padded(seconds));
        }

        if self.show_tokens && stats.tokens > 0 {
            text.push(' ');
            text.push_str(&format_tokens(stats.tokens));
        }

        text
    }
}

fn rewind_stale_cache(state: &mut TranscriptState, transcript: &str) {
    let current_len = fs::metadata(transcript).map(|meta| meta.len()).unwrap_or(0);

    if state.scanned_bytes > current_len {
        *state = TranscriptState::default();
    }
}

fn is_stalled(now: i64, last_signal: Option<i64>, stall_seconds: u64) -> bool {
    let Some(last_signal) = last_signal else {
        return false;
    };

    now - last_signal > stall_seconds as i64
}

fn format_tokens(tokens: u64) -> String {
    if tokens < 1_000 {
        return tokens.to_string();
    }

    if tokens < 1_000_000 {
        let thousands = tokens as f64 / 1_000.0;
        if thousands < 10.0 {
            return format!("{:.1}k", thousands);
        }

        return format!("{}k", thousands.round() as u64);
    }

    format!("{:.1}M", tokens as f64 / 1_000_000.0)
}

#[cfg(test)]
mod tests {
    use super::{format_tokens, is_stalled, Stats, SubagentStats};
    use crate::config_schema::Color;

    fn sample() -> SubagentStats {
        SubagentStats {
            color: Color::Named(90),
            active_color: Color::Named(32),
            stall_color: Color::Named(91),
            prefix: "agents ".to_string(),
            stall_marker: "!".to_string(),
            stall_seconds: 120,
            show_tokens: true,
        }
    }

    fn stats(active: usize, total: usize, seconds: Option<i64>, tokens: u64, stalled: bool) -> Stats {
        Stats {
            active,
            total,
            longest_active_seconds: seconds,
            tokens,
            stalled,
        }
    }

    #[test]
    fn shows_active_over_total_with_age_and_tokens() {
        let segment = sample();

        assert_eq!(
            segment.format(&stats(2, 7, Some(252), 1_240_000, false)),
            "agents 2/7 4m12s 1.2M"
        );
    }

    #[test]
    fn drops_the_active_counter_and_age_once_every_agent_finished() {
        let segment = sample();

        assert_eq!(segment.format(&stats(0, 7, None, 42_000, false)), "agents 7 42k");
    }

    #[test]
    fn marks_stalled_agents() {
        let segment = sample();

        assert_eq!(
            segment.format(&stats(1, 3, Some(3_720), 0, true)),
            "agents 1/3! 1h02m"
        );
    }

    #[test]
    fn hides_tokens_when_disabled() {
        let mut segment = sample();
        segment.show_tokens = false;

        assert_eq!(segment.format(&stats(1, 1, Some(5), 900_000, false)), "agents 1/1 5s");
    }

    #[test]
    fn stalls_only_after_the_configured_quiet_window() {
        assert!(!is_stalled(1_000, Some(900), 120));
        assert!(is_stalled(1_000, Some(870), 120));
        assert!(!is_stalled(1_000, None, 120));
    }

    #[test]
    fn formats_token_magnitudes() {
        assert_eq!(format_tokens(999), "999");
        assert_eq!(format_tokens(1_500), "1.5k");
        assert_eq!(format_tokens(42_400), "42k");
        assert_eq!(format_tokens(1_240_000), "1.2M");
    }
}
