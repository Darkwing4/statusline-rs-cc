pub(crate) fn format_duration(seconds: i64) -> String {
    let s = seconds.max(0);
    let h = s / 3600;
    let m = (s % 3600) / 60;
    let sec = s % 60;

    if h > 0 {
        format!("{}h{:02}m", h, m)
    } else if m > 0 {
        format!("{}m{:02}s", m, sec)
    } else {
        format!("{}s", sec)
    }
}

#[cfg(test)]
mod tests {
    use super::format_duration;

    #[test]
    fn pads_the_smaller_unit_in_every_branch() {
        assert_eq!(format_duration(-1), "0s");
        assert_eq!(format_duration(45), "45s");
        assert_eq!(format_duration(65), "1m05s");
        assert_eq!(format_duration(3905), "1h05m");
    }
}
