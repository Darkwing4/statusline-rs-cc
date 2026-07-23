pub use crate::config_schema::Color;

pub const RESET: &str = "\x1b[0m";

impl Color {
    pub fn paint(&self, body: &str) -> String {
        match self {
            Color::Gradient => body.to_string(),
            Color::Named(code) => format!("\x1b[{}m{}{}", code, body, RESET),
            Color::Rgb(r, g, b) => format!("\x1b[38;2;{};{};{}m{}{}", r, g, b, body, RESET),
        }
    }
}
