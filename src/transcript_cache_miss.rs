use serde::{Deserialize, Serialize};

use crate::claude_model_pricing::{cache_rewrite_overpay, CacheLifetime};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub(crate) struct CacheMiss {
    pub(crate) tokens: u64,
    pub(crate) overpay_usd: f64,
}

pub(crate) struct CachedPrompt<'a> {
    pub(crate) model: &'a str,
    pub(crate) cache_read: u64,
    pub(crate) cache_write: u64,
    pub(crate) lifetime: CacheLifetime,
}

#[derive(Default, Deserialize, Serialize)]
pub(crate) struct CacheMissTracker {
    cached_prefix: u64,
    turn: CacheMiss,
}

impl CacheMissTracker {
    pub(crate) fn turn(&self) -> CacheMiss {
        self.turn
    }

    pub(crate) fn start_turn(&mut self) {
        self.turn = CacheMiss::default();
    }

    pub(crate) fn forget_prefix(&mut self) {
        self.cached_prefix = 0;
    }

    pub(crate) fn observe(&mut self, prompt: &CachedPrompt, in_turn: bool) {
        let cached_now = prompt.cache_read.saturating_add(prompt.cache_write);

        if cached_now == 0 {
            return;
        }

        let lost = self.cached_prefix.saturating_sub(prompt.cache_read);
        let missed = lost.min(prompt.cache_write);
        self.cached_prefix = cached_now;

        if !in_turn || missed == 0 {
            return;
        }

        let overpay = cache_rewrite_overpay(prompt.model, missed, prompt.lifetime).unwrap_or(0.0);
        self.turn.tokens = self.turn.tokens.saturating_add(missed);
        self.turn.overpay_usd += overpay;
    }
}

#[cfg(test)]
mod tests {
    use super::{CacheMissTracker, CachedPrompt};
    use crate::claude_model_pricing::CacheLifetime;

    fn prompt(cache_read: u64, cache_write: u64) -> CachedPrompt<'static> {
        CachedPrompt {
            model: "claude-opus-5-5",
            cache_read,
            cache_write,
            lifetime: CacheLifetime::FiveMinutes,
        }
    }

    #[test]
    fn a_warm_prefix_that_only_grows_is_not_a_miss() {
        let mut tracker = CacheMissTracker::default();

        tracker.observe(&prompt(11_764, 26_350), true);
        tracker.observe(&prompt(38_114, 1_097), true);
        tracker.observe(&prompt(39_211, 1_380), true);

        assert_eq!(tracker.turn().tokens, 0);
    }

    #[test]
    fn rewriting_the_prefix_a_cold_cache_lost_is_a_miss_priced_over_a_hit() {
        let mut tracker = CacheMissTracker::default();

        tracker.observe(&prompt(200_000, 1_000), false);
        tracker.observe(&prompt(11_000, 192_000), true);

        assert_eq!(tracker.turn().tokens, 190_000);
        assert!((tracker.turn().overpay_usd - 0.912).abs() < 1e-9);
    }

    #[test]
    fn a_compacted_conversation_starts_a_fresh_prefix() {
        let mut tracker = CacheMissTracker::default();

        tracker.observe(&prompt(150_000, 2_000), true);
        tracker.forget_prefix();
        tracker.observe(&prompt(11_764, 25_015), true);

        assert_eq!(tracker.turn().tokens, 0);
    }

    #[test]
    fn a_response_without_cache_usage_keeps_the_known_prefix() {
        let mut tracker = CacheMissTracker::default();

        tracker.observe(&prompt(100_000, 500), true);
        tracker.observe(&prompt(0, 0), true);
        tracker.observe(&prompt(0, 100_600), true);

        assert_eq!(tracker.turn().tokens, 100_500);
    }

    #[test]
    fn a_new_turn_clears_the_misses_of_the_last_one() {
        let mut tracker = CacheMissTracker::default();

        tracker.observe(&prompt(100_000, 500), true);
        tracker.observe(&prompt(0, 100_600), true);
        tracker.start_turn();

        assert_eq!(tracker.turn().tokens, 0);
    }
}
