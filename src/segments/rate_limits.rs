mod claude_usage_cache;
mod claude_usage_fetch;

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::config_schema::Color;
pub use crate::config_schema::{ColorMode, Fill, RateLimit, Style, Window};
use crate::gradient::gradient;
use crate::iso8601::parse_iso8601_utc;
use crate::segments::{json_field, GitCache, Segment};

pub use self::claude_usage_fetch::refresh_usage;

const COUNTDOWN_TOKEN: &str = "{t}";
const UNKNOWN_COUNTDOWN: &str = "?";
const FABLE_MODEL_NAME: &str = "fable";
const WEEKLY_SCOPED_KIND: &str = "weekly_scoped";

struct WindowSample {
    used_percentage: f64,
    resets_at: Option<i64>,
    severity: Option<String>,
    is_active: bool,
}

impl RateLimit {
    fn sample(&self, json: &Value) -> Option<WindowSample> {
        match self.window {
            Window::FiveHour => reported_window(json, "five_hour"),
            Window::SevenDay => reported_window(json, "seven_day"),
            Window::Fable => self.fable_window(json),
        }
    }

    fn fable_window(&self, json: &Value) -> Option<WindowSample> {
        if !model_is_fable(json) {
            return None;
        }

        let usage = claude_usage_cache::snapshot(self.usage_ttl_seconds)?;

        scoped_fable_window(&usage)
    }

    fn resolve_prefix(&self, sample: &WindowSample, now: Option<i64>) -> String {
        if !self.prefix.contains(COUNTDOWN_TOKEN) {
            return self.prefix.clone();
        }

        let remaining = match self.remaining_units(sample, now) {
            Some(value) => format!("{value:.1}"),
            None => UNKNOWN_COUNTDOWN.to_string(),
        };
        self.prefix.replace(COUNTDOWN_TOKEN, &remaining)
    }

    fn remaining_units(&self, sample: &WindowSample, now: Option<i64>) -> Option<f64> {
        let unit_secs = match self.window {
            Window::FiveHour => 3600.0,
            Window::SevenDay | Window::Fable => 86400.0,
        };
        let now = now?;
        let remaining = sample.resets_at?.saturating_sub(now).max(0) as f64;
        Some(remaining / unit_secs)
    }

    fn markers(&self, sample: &WindowSample) -> String {
        let severity = sample
            .severity
            .as_deref()
            .and_then(|severity| self.severity_marker(severity))
            .unwrap_or("");

        let active = if sample.is_active {
            self.active_marker.as_str()
        } else {
            ""
        };

        format!("{}{}", severity, active)
    }

    fn severity_marker(&self, severity: &str) -> Option<&str> {
        self.severity_markers
            .iter()
            .find(|(name, _)| name == severity)
            .map(|(_, marker)| marker.as_str())
    }
}

fn reported_window(json: &Value, key: &str) -> Option<WindowSample> {
    let window_data = json.get("rate_limits")?.get(key)?;

    Some(WindowSample {
        used_percentage: window_data.get("used_percentage")?.as_f64()?,
        resets_at: window_data.get("resets_at").and_then(Value::as_i64),
        severity: None,
        is_active: false,
    })
}

fn scoped_fable_window(usage: &Value) -> Option<WindowSample> {
    let limits = usage.get("limits")?.as_array()?;

    limits
        .iter()
        .find(|limit| is_fable_limit(limit))
        .map(|limit| WindowSample {
            used_percentage: limit
                .get("percent")
                .and_then(Value::as_f64)
                .unwrap_or_default(),
            resets_at: json_field(limit, "/resets_at").and_then(parse_iso8601_utc),
            severity: json_field(limit, "/severity").map(str::to_string),
            is_active: limit
                .get("is_active")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        })
}

fn is_fable_limit(limit: &Value) -> bool {
    let is_weekly_scoped = limit.get("kind").and_then(Value::as_str) == Some(WEEKLY_SCOPED_KIND);

    is_weekly_scoped
        && json_field(limit, "/scope/model/display_name")
            .is_some_and(|name| name.eq_ignore_ascii_case(FABLE_MODEL_NAME))
}

fn model_is_fable(json: &Value) -> bool {
    ["/model/id", "/model/display_name"]
        .into_iter()
        .filter_map(|pointer| json_field(json, pointer))
        .any(|value| value.to_ascii_lowercase().contains(FABLE_MODEL_NAME))
}

const BAR_GLYPHS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
const RADIAL_GLYPHS: [char; 5] = ['○', '◔', '◑', '◕', '●'];

fn glyph(glyphs: &[char], pct: f64) -> char {
    let idx = (pct * glyphs.len() as f64 / 100.0) as isize;
    let clamped = idx.clamp(0, glyphs.len() as isize - 1) as usize;
    glyphs[clamped]
}

impl Segment for RateLimit {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let sample = self.sample(json)?;

        let pct = sample.used_percentage;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|duration| i64::try_from(duration.as_secs()).ok());
        let prefix = self.resolve_prefix(&sample, now);

        let rounded = pct.round() as i64;

        let glyph_pct = match self.fill {
            Fill::Used => pct,
            Fill::Remaining => 100.0 - pct,
        };

        let body = match self.style {
            Style::Percent => format!("{}{}%", prefix, rounded),
            Style::Bar => format!("{}{}", prefix, glyph(&BAR_GLYPHS, glyph_pct)),
            Style::BarPercent => {
                format!("{}{} {}%", prefix, glyph(&BAR_GLYPHS, glyph_pct), rounded)
            }
            Style::Radial => format!("{}{}", prefix, glyph(&RADIAL_GLYPHS, glyph_pct)),
            Style::RadialPercent => {
                format!(
                    "{}{} {}%",
                    prefix,
                    glyph(&RADIAL_GLYPHS, glyph_pct),
                    rounded
                )
            }
        };

        let text = format!("{}{}", body, self.markers(&sample));

        let painted = match self.color_mode {
            ColorMode::Steps => {
                let color = if pct < 50.0 {
                    self.low_color
                } else if pct <= 80.0 {
                    self.mid_color
                } else {
                    self.high_color
                };
                color.paint(&text)
            }
            ColorMode::Gradient => {
                let low = color_to_rgb(self.low_color, (60, 200, 60));
                let mid = color_to_rgb(self.mid_color, (220, 200, 40));
                let high = color_to_rgb(self.high_color, (220, 60, 60));
                let stops = [
                    (0.0, low),
                    (self.gradient_midpoint_percentage, mid),
                    (100.0, high),
                ];
                let (r, g, b) = gradient(&stops, pct);
                Color::Rgb(r, g, b).paint(&text)
            }
        };

        Some(painted)
    }
}

fn color_to_rgb(c: Color, fallback: (u8, u8, u8)) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => fallback,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::{
        glyph, model_is_fable, reported_window, scoped_fable_window, ColorMode, Fill, RateLimit,
        Segment, Style, Window, WindowSample, BAR_GLYPHS, RADIAL_GLYPHS,
    };
    use crate::config_schema::Color;
    use crate::segments::GitCache;

    fn rate_limit(window: Window, prefix: &str) -> RateLimit {
        RateLimit {
            window,
            style: Style::Percent,
            fill: Fill::Used,
            color_mode: ColorMode::Steps,
            gradient_midpoint_percentage: 50.0,
            prefix: prefix.to_string(),
            usage_ttl_seconds: 300,
            active_marker: String::new(),
            severity_markers: Vec::new(),
            low_color: Color::Named(32),
            mid_color: Color::Named(33),
            high_color: Color::Named(31),
        }
    }

    fn sample(resets_at: Option<i64>) -> WindowSample {
        WindowSample {
            used_percentage: 0.0,
            resets_at,
            severity: None,
            is_active: false,
        }
    }

    fn usage_response(limits: Value) -> Value {
        json!({
            "five_hour": {"utilization": 24.0, "resets_at": "2026-09-05T20:00:00.467032+00:00"},
            "seven_day": {"utilization": 9.0, "resets_at": "2026-09-07T05:00:00.467055+00:00"},
            "extra_usage": {"is_enabled": false, "monthly_limit": null, "used_credits": null},
            "limits": limits,
        })
    }

    fn live_limits() -> Value {
        json!([
            {
                "kind": "session",
                "group": "session",
                "percent": 24,
                "severity": "normal",
                "resets_at": "2026-09-05T20:00:00.467032+00:00",
                "scope": null,
                "is_active": true
            },
            {
                "kind": "weekly_all",
                "group": "weekly",
                "percent": 9,
                "severity": "normal",
                "resets_at": "2026-09-07T05:00:00.467055+00:00",
                "scope": null,
                "is_active": false
            },
            {
                "kind": "weekly_scoped",
                "group": "weekly",
                "percent": 6,
                "severity": "normal",
                "resets_at": "2026-09-07T05:00:00.467296+00:00",
                "scope": {"model": {"id": null, "display_name": "Fable"}, "surface": null},
                "is_active": false
            }
        ])
    }

    #[test]
    fn bar_glyph_uses_all_thresholds_and_clamps() {
        let cases = [
            (-1.0, '▁'),
            (0.0, '▁'),
            (12.499, '▁'),
            (12.5, '▂'),
            (25.0, '▃'),
            (37.5, '▄'),
            (50.0, '▅'),
            (62.5, '▆'),
            (75.0, '▇'),
            (87.5, '█'),
            (100.0, '█'),
            (101.0, '█'),
        ];

        for (pct, expected) in cases {
            assert_eq!(glyph(&BAR_GLYPHS, pct), expected, "{pct}");
        }
    }

    #[test]
    fn radial_glyph_uses_all_thresholds_and_clamps() {
        let cases = [
            (-1.0, '○'),
            (0.0, '○'),
            (19.999, '○'),
            (20.0, '◔'),
            (40.0, '◑'),
            (60.0, '◕'),
            (80.0, '●'),
            (100.0, '●'),
            (101.0, '●'),
        ];

        for (pct, expected) in cases {
            assert_eq!(glyph(&RADIAL_GLYPHS, pct), expected, "{pct}");
        }
    }

    #[test]
    fn resolve_prefix_returns_unchanged_text_without_countdown_token() {
        let segment = rate_limit(Window::FiveHour, "limit ");

        assert_eq!(
            segment.resolve_prefix(&sample(Some(i64::MIN)), Some(i64::MAX)),
            "limit "
        );
    }

    #[test]
    fn remaining_units_returns_none_without_real_reset_data() {
        assert_eq!(
            rate_limit(Window::FiveHour, "").remaining_units(&sample(None), Some(1_000)),
            None
        );
        assert_eq!(
            rate_limit(Window::SevenDay, "").remaining_units(&sample(None), Some(1_000)),
            None
        );
        assert_eq!(
            rate_limit(Window::Fable, "").remaining_units(&sample(None), Some(1_000)),
            None
        );
        assert_eq!(
            rate_limit(Window::FiveHour, "").remaining_units(&sample(Some(10_000)), None),
            None
        );
        assert_eq!(
            rate_limit(Window::SevenDay, "").remaining_units(&sample(Some(10_000)), None),
            None
        );
    }

    #[test]
    fn resolve_prefix_replaces_every_countdown_token() {
        let segment = rate_limit(Window::SevenDay, "{t} days, then {t} days");

        assert_eq!(
            segment.resolve_prefix(&sample(None), Some(1_000)),
            "? days, then ? days"
        );
    }

    #[test]
    fn remaining_units_calculates_exact_future_countdown_for_each_window() {
        let five_hour = rate_limit(Window::FiveHour, "{t}");
        let seven_day = rate_limit(Window::SevenDay, "{t}");
        let fable = rate_limit(Window::Fable, "{t}");

        assert_eq!(
            five_hour.remaining_units(&sample(Some(6_400)), Some(1_000)),
            Some(1.5)
        );
        assert_eq!(
            five_hour.resolve_prefix(&sample(Some(6_400)), Some(1_000)),
            "1.5"
        );
        assert_eq!(
            seven_day.remaining_units(&sample(Some(130_600)), Some(1_000)),
            Some(1.5)
        );
        assert_eq!(
            seven_day.resolve_prefix(&sample(Some(130_600)), Some(1_000)),
            "1.5"
        );
        assert_eq!(
            fable.remaining_units(&sample(Some(130_600)), Some(1_000)),
            Some(1.5)
        );
    }

    #[test]
    fn remaining_units_returns_zero_for_expired_and_extreme_resets() {
        let segment = rate_limit(Window::FiveHour, "{t}h");

        assert_eq!(
            segment.remaining_units(&sample(Some(999)), Some(1_000)),
            Some(0.0)
        );
        assert_eq!(
            segment.resolve_prefix(&sample(Some(999)), Some(1_000)),
            "0.0h"
        );
        assert_eq!(
            segment.remaining_units(&sample(Some(i64::MIN)), Some(i64::MAX)),
            Some(0.0)
        );
        assert_eq!(
            segment.resolve_prefix(&sample(Some(i64::MIN)), Some(i64::MAX)),
            "0.0h"
        );
    }

    #[test]
    fn reported_window_reads_percentage_and_only_an_integer_reset() {
        let json = json!({
            "rate_limits": {
                "five_hour": {"used_percentage": 25.0, "resets_at": 6_400},
                "seven_day": {"used_percentage": 60.0, "resets_at": "2026-09-07T05:00:00Z"}
            }
        });

        let five_hour = reported_window(&json, "five_hour").expect("five hour window");
        assert_eq!(five_hour.used_percentage, 25.0);
        assert_eq!(five_hour.resets_at, Some(6_400));

        let seven_day = reported_window(&json, "seven_day").expect("seven day window");
        assert_eq!(seven_day.used_percentage, 60.0);
        assert_eq!(seven_day.resets_at, None);

        assert!(reported_window(&json!({}), "five_hour").is_none());
    }

    #[test]
    fn scoped_fable_window_picks_the_weekly_scoped_fable_entry() {
        let usage = usage_response(live_limits());

        let window = scoped_fable_window(&usage).expect("fable window");

        assert_eq!(window.used_percentage, 6.0);
        assert_eq!(
            window.resets_at,
            crate::iso8601::parse_iso8601_utc("2026-09-07T05:00:00Z")
        );
        assert_eq!(window.severity.as_deref(), Some("normal"));
        assert!(!window.is_active);
    }

    #[test]
    fn scoped_fable_window_ignores_other_models_and_missing_limits() {
        let other_model = json!([{
            "kind": "weekly_scoped",
            "group": "weekly",
            "percent": 42,
            "resets_at": "2026-09-07T05:00:00Z",
            "scope": {"model": {"id": null, "display_name": "Opus"}},
            "is_active": false
        }]);

        assert!(scoped_fable_window(&usage_response(other_model)).is_none());
        assert!(scoped_fable_window(&usage_response(json!([]))).is_none());
        assert!(scoped_fable_window(&json!({})).is_none());
    }

    #[test]
    fn model_is_fable_matches_the_id_or_the_display_name() {
        assert!(model_is_fable(
            &json!({"model": {"id": "claude-fable-5-1"}})
        ));
        assert!(model_is_fable(
            &json!({"model": {"id": "", "display_name": "Claude Fable 5.1"}})
        ));
        assert!(!model_is_fable(&json!({"model": {"id": "claude-opus-5"}})));
        assert!(!model_is_fable(&json!({})));
    }

    #[test]
    fn fable_window_stays_hidden_on_other_models() {
        let mut git = GitCache::new(String::new());
        let segment = rate_limit(Window::Fable, "");

        let json = json!({
            "model": {"id": "claude-opus-5", "display_name": "Opus 5"},
            "rate_limits": {"five_hour": {"used_percentage": 25.0}}
        });

        assert_eq!(segment.render(&json, &mut git), None);
    }

    #[test]
    fn markers_show_severity_and_active_state_only_when_they_match() {
        let mut segment = rate_limit(Window::Fable, "");
        segment.active_marker = "*".to_string();
        segment.severity_markers = vec![("warning".to_string(), "!".to_string())];

        let marked = WindowSample {
            used_percentage: 6.0,
            resets_at: None,
            severity: Some("warning".to_string()),
            is_active: true,
        };
        assert_eq!(segment.markers(&marked), "!*");

        let quiet = WindowSample {
            used_percentage: 6.0,
            resets_at: None,
            severity: Some("normal".to_string()),
            is_active: false,
        };
        assert_eq!(segment.markers(&quiet), "");

        let unknown_severity = WindowSample {
            severity: Some("apocalyptic".to_string()),
            is_active: true,
            ..quiet
        };
        assert_eq!(segment.markers(&unknown_severity), "*");
    }
}
