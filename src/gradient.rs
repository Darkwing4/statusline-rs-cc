pub(crate) type Rgb = (u8, u8, u8);

#[derive(Clone, Copy)]
pub(crate) enum Quantization {
    Truncate,
    Nearest,
}

pub(crate) fn gradient(stops: &[(f64, Rgb)], p: f64, quantization: Quantization) -> Rgb {
    if p.is_nan() {
        return (0, 0, 0);
    }

    let Some(&(first_position, first_color)) = stops.first() else {
        return (0, 0, 0);
    };
    if p <= first_position {
        return first_color;
    }

    for pair in stops.windows(2) {
        let (start_position, start_color) = pair[0];
        let (end_position, end_color) = pair[1];
        if p > end_position {
            continue;
        }

        let span = end_position - start_position;
        if span <= 0.0 {
            return end_color;
        }

        let t = ((p - start_position) / span).clamp(0.0, 1.0);
        return interpolate(start_color, end_color, t, quantization);
    }

    stops.last().map(|stop| stop.1).unwrap_or(first_color)
}

fn interpolate(start: Rgb, end: Rgb, t: f64, quantization: Quantization) -> Rgb {
    let channel = |start: u8, end: u8| {
        let value = start as f64 + (end as f64 - start as f64) * t;
        match quantization {
            Quantization::Truncate => value.clamp(0.0, 255.0) as u8,
            Quantization::Nearest => value.round().clamp(0.0, 255.0) as u8,
        }
    };

    (
        channel(start.0, end.0),
        channel(start.1, end.1),
        channel(start.2, end.2),
    )
}

#[cfg(test)]
mod tests {
    use super::{gradient, Quantization};

    const STOPS: &[(f64, (u8, u8, u8))] =
        &[(0.0, (0, 2, 4)), (50.0, (1, 3, 5)), (100.0, (2, 4, 6))];

    #[test]
    fn uses_endpoints_and_interpolates_both_halves() {
        assert_eq!(gradient(STOPS, 0.0, Quantization::Nearest), (0, 2, 4));
        assert_eq!(gradient(STOPS, 50.0, Quantization::Nearest), (1, 3, 5));
        assert_eq!(gradient(STOPS, 100.0, Quantization::Nearest), (2, 4, 6));
        assert_eq!(gradient(STOPS, 75.0, Quantization::Nearest), (2, 4, 6));
    }

    #[test]
    fn preserves_requested_quantization() {
        assert_eq!(gradient(STOPS, 25.0, Quantization::Truncate), (0, 2, 4));
        assert_eq!(gradient(STOPS, 25.0, Quantization::Nearest), (1, 3, 5));
    }

    #[test]
    fn uses_configured_stop_position() {
        let stops = [
            (0.0, (0, 0, 0)),
            (25.0, (100, 100, 100)),
            (100.0, (200, 200, 200)),
        ];

        assert_eq!(
            gradient(&stops, 25.0, Quantization::Nearest),
            (100, 100, 100)
        );
        assert_eq!(
            gradient(&stops, 62.5, Quantization::Nearest),
            (150, 150, 150)
        );
    }

    #[test]
    fn clamps_to_outer_stops() {
        assert_eq!(gradient(STOPS, -1.0, Quantization::Nearest), (0, 2, 4));
        assert_eq!(gradient(STOPS, 101.0, Quantization::Nearest), (2, 4, 6));
    }

    #[test]
    fn handles_missing_stops_and_nan() {
        assert_eq!(gradient(&[], 50.0, Quantization::Nearest), (0, 0, 0));
        assert_eq!(gradient(STOPS, f64::NAN, Quantization::Nearest), (0, 0, 0));
    }
}
