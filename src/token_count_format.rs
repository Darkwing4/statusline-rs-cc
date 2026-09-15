pub fn format_tokens(tokens: u64) -> String {
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
    use super::format_tokens;

    #[test]
    fn formats_token_magnitudes() {
        assert_eq!(format_tokens(999), "999");
        assert_eq!(format_tokens(1_500), "1.5k");
        assert_eq!(format_tokens(42_400), "42k");
        assert_eq!(format_tokens(1_240_000), "1.2M");
    }
}
