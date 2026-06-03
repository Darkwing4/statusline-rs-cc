use serde_json::Value;

use crate::segments::{GitCache, Segment};
use crate::types::Color;

use serde::Deserialize;

#[derive(Clone, Copy, Deserialize)]
pub enum Window {
    FiveHour,
    SevenDay,
}

pub struct RateLimit {
    pub window: Window,
    pub prefix: String,
    pub low_color: Color,
    pub mid_color: Color,
    pub high_color: Color,
}

impl Segment for RateLimit {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let key = match self.window {
            Window::FiveHour => "five_hour",
            Window::SevenDay => "seven_day",
        };

        let pct = json
            .get("rate_limits")?
            .get(key)?
            .get("used_percentage")?
            .as_f64()?;

        let rounded = pct.round() as i64;
        let text = format!("{}{}%", self.prefix, rounded);

        let color = if pct < 50.0 {
            self.low_color
        } else if pct <= 80.0 {
            self.mid_color
        } else {
            self.high_color
        };

        Some(color.paint(&text))
    }
}
