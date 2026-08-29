pub(super) fn format_duration(seconds: i64) -> String {
    let total = seconds.max(0);
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let secs = total % 60;

    if hours > 0 {
        format!("{}h{:02}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m{:02}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

#[cfg(test)]
mod tests {
    use super::format_duration;

    #[test]
    fn formats_hours_minutes_and_seconds() {
        assert_eq!(format_duration(3_720), "1h02m");
        assert_eq!(format_duration(252), "4m12s");
        assert_eq!(format_duration(5), "5s");
        assert_eq!(format_duration(-5), "0s");
    }
}
