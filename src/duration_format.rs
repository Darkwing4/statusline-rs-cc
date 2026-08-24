pub(crate) fn format_duration(seconds: i64) -> String {
    let (h, m, s) = split(seconds);

    if h > 0 {
        format!("{}h{}m", h, m)
    } else if m > 0 {
        format!("{}m{}s", m, s)
    } else {
        format!("{}s", s)
    }
}

pub(crate) fn format_duration_padded(seconds: i64) -> String {
    let (h, m, s) = split(seconds);

    if h > 0 {
        format!("{}h{:02}m", h, m)
    } else if m > 0 {
        format!("{}m{:02}s", m, s)
    } else {
        format!("{}s", s)
    }
}

fn split(seconds: i64) -> (i64, i64, i64) {
    let total = seconds.max(0);

    (total / 3600, (total % 3600) / 60, total % 60)
}

#[cfg(test)]
mod tests {
    use super::{format_duration, format_duration_padded};

    #[test]
    fn pads_the_smaller_unit_in_every_branch() {
        assert_eq!(format_duration_padded(-1), "0s");
        assert_eq!(format_duration_padded(45), "45s");
        assert_eq!(format_duration_padded(65), "1m05s");
        assert_eq!(format_duration_padded(3905), "1h05m");
    }

    #[test]
    fn leaves_the_smaller_unit_unpadded_in_every_branch() {
        assert_eq!(format_duration(-1), "0s");
        assert_eq!(format_duration(45), "45s");
        assert_eq!(format_duration(65), "1m5s");
        assert_eq!(format_duration(3905), "1h5m");
    }
}
