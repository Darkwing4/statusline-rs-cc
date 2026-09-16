pub fn format_dollars(dollars: f64) -> Option<String> {
    if !dollars.is_finite() || dollars <= 0.0 {
        return None;
    }

    Some(format!("${:.2}", dollars))
}

#[cfg(test)]
mod tests {
    use super::format_dollars;

    #[test]
    fn shows_dollars_and_cents_for_a_positive_amount() {
        assert_eq!(format_dollars(4.2).as_deref(), Some("$4.20"));
        assert_eq!(format_dollars(0.004).as_deref(), Some("$0.00"));
    }

    #[test]
    fn hides_nothing_zero_and_broken_amounts() {
        assert_eq!(format_dollars(0.0), None);
        assert_eq!(format_dollars(-1.0), None);
        assert_eq!(format_dollars(f64::NAN), None);
    }
}
