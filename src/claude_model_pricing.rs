const DOLLARS_PER_MILLION: &[(&str, InputPrices)] = &[
    ("claude-fable-5-1", InputPrices::new(10.0, 0.25)),
    ("claude-mythos-5-1", InputPrices::new(10.0, 0.25)),
    ("claude-fable-5", InputPrices::new(10.0, 1.0)),
    ("claude-mythos-5", InputPrices::new(10.0, 1.0)),
    ("claude-opus-5-5", InputPrices::new(4.0, 0.2)),
    ("claude-opus-5", InputPrices::new(5.0, 0.5)),
    ("claude-opus-4-8", InputPrices::new(5.0, 0.5)),
    ("claude-opus-4-7", InputPrices::new(5.0, 0.5)),
    ("claude-opus-4-6", InputPrices::new(5.0, 0.5)),
    ("claude-opus-4-5", InputPrices::new(5.0, 0.5)),
    ("claude-opus-4", InputPrices::new(15.0, 1.5)),
    ("claude-sonnet-5", InputPrices::new(2.0, 0.2)),
    ("claude-sonnet-4", InputPrices::new(3.0, 0.3)),
    ("claude-haiku-4-5", InputPrices::new(1.0, 0.1)),
];

const SHORT_CACHE_WRITE_MULTIPLIER: f64 = 1.25;
const LONG_CACHE_WRITE_MULTIPLIER: f64 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum CacheLifetime {
    FiveMinutes,
    OneHour,
}

#[derive(Clone, Copy)]
struct InputPrices {
    base: f64,
    cache_read: f64,
}

impl InputPrices {
    const fn new(base: f64, cache_read: f64) -> Self {
        InputPrices { base, cache_read }
    }

    fn cache_write(&self, lifetime: CacheLifetime) -> f64 {
        let multiplier = match lifetime {
            CacheLifetime::FiveMinutes => SHORT_CACHE_WRITE_MULTIPLIER,
            CacheLifetime::OneHour => LONG_CACHE_WRITE_MULTIPLIER,
        };

        self.base * multiplier
    }
}

pub(crate) fn cache_rewrite_overpay(
    model: &str,
    tokens: u64,
    lifetime: CacheLifetime,
) -> Option<f64> {
    let prices = DOLLARS_PER_MILLION
        .iter()
        .find(|(prefix, _)| model.starts_with(prefix))
        .map(|(_, prices)| prices)?;
    let per_million = prices.cache_write(lifetime) - prices.cache_read;

    Some(tokens as f64 * per_million / 1_000_000.0)
}

#[cfg(test)]
mod tests {
    use super::{cache_rewrite_overpay, CacheLifetime};

    fn overpay(model: &str, lifetime: CacheLifetime) -> Option<f64> {
        cache_rewrite_overpay(model, 1_000_000, lifetime)
            .map(|dollars| (dollars * 1000.0).round() / 1000.0)
    }

    #[test]
    fn charges_the_write_price_minus_the_read_that_a_hit_would_have_cost() {
        assert_eq!(
            overpay("claude-opus-5-5", CacheLifetime::FiveMinutes),
            Some(4.8)
        );
        assert_eq!(
            overpay("claude-opus-5", CacheLifetime::FiveMinutes),
            Some(5.75)
        );
        assert_eq!(
            overpay("claude-fable-5-1", CacheLifetime::OneHour),
            Some(19.75)
        );
        assert_eq!(
            overpay("claude-fable-5", CacheLifetime::OneHour),
            Some(19.0)
        );
    }

    #[test]
    fn tells_older_opus_from_the_cheaper_ones_by_version() {
        assert_eq!(
            overpay("claude-opus-4-5-20251101", CacheLifetime::FiveMinutes),
            Some(5.75)
        );
        assert_eq!(
            overpay("claude-opus-4-1-20250805", CacheLifetime::FiveMinutes),
            Some(17.25)
        );
        assert_eq!(
            overpay("claude-haiku-4-5-20251001", CacheLifetime::FiveMinutes),
            Some(1.15)
        );
    }

    #[test]
    fn leaves_unknown_models_unpriced() {
        assert_eq!(overpay("<synthetic>", CacheLifetime::FiveMinutes), None);
        assert_eq!(overpay("gpt-5", CacheLifetime::OneHour), None);
    }
}
