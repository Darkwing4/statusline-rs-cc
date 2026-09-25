use serde_json::Value;

pub use crate::config_schema::PromptCacheMiss;
use crate::segments::{GitCache, Segment};
use crate::session_token_tallies::load_session_tallies;
use crate::token_count_format::format_tokens;
use crate::transcript_cache_miss::CacheMiss;

impl Segment for PromptCacheMiss {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let miss = load_session_tallies(json)?.turn_cache_miss()?;
        let text = self.format(miss)?;

        Some(self.color.paint(&text))
    }
}

impl PromptCacheMiss {
    fn format(&self, miss: CacheMiss) -> Option<String> {
        if miss.tokens == 0 || miss.tokens < self.min_tokens {
            return None;
        }

        let mut text = format!("{}{}", self.prefix, format_tokens(miss.tokens));
        let cents = (miss.overpay_usd * 100.0).round();

        if cents.is_finite() && cents >= 1.0 {
            text.push_str(&format!(" ${:.2}", cents / 100.0));
        }

        text.push_str(&self.suffix);

        Some(text)
    }
}

#[cfg(test)]
mod tests {
    use super::PromptCacheMiss;
    use crate::config_schema::Color;
    use crate::transcript_cache_miss::CacheMiss;

    fn segment() -> PromptCacheMiss {
        PromptCacheMiss {
            color: Color::Named(31),
            prefix: "(missing! ".to_string(),
            suffix: ")".to_string(),
            min_tokens: 10_000,
        }
    }

    fn miss(tokens: u64, overpay_usd: f64) -> CacheMiss {
        CacheMiss {
            tokens,
            overpay_usd,
        }
    }

    #[test]
    fn shows_the_rewritten_tokens_and_what_they_cost_over_a_hit() {
        assert_eq!(
            segment().format(miss(254_300, 1.2206)).as_deref(),
            Some("(missing! 254k $1.22)")
        );
    }

    #[test]
    fn leaves_out_a_cost_below_a_cent_or_for_an_unpriced_model() {
        assert_eq!(
            segment().format(miss(12_000, 0.004)).as_deref(),
            Some("(missing! 12k)")
        );
        assert_eq!(
            segment().format(miss(12_000, f64::NAN)).as_deref(),
            Some("(missing! 12k)")
        );
    }

    #[test]
    fn stays_hidden_without_a_miss_worth_the_warning() {
        assert_eq!(segment().format(miss(0, 0.0)), None);
        assert_eq!(segment().format(miss(9_999, 0.05)), None);

        let without_threshold = PromptCacheMiss {
            min_tokens: 0,
            ..segment()
        };
        assert_eq!(without_threshold.format(miss(0, 0.0)), None);
        assert_eq!(
            without_threshold.format(miss(500, 0.0)).as_deref(),
            Some("(missing! 500)")
        );
    }
}
