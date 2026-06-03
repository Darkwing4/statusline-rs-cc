use serde::Deserialize;

pub const RESET: &str = "\x1b[0m";

#[derive(Clone, Copy, Deserialize)]
pub enum Color {
    Named(u8),
    Rgb(u8, u8, u8),
    Gradient,
}

impl Color {
    pub fn paint(&self, body: &str) -> String {
        match self {
            Color::Gradient => body.to_string(),
            Color::Named(code) => format!("\x1b[{}m{}{}", code, body, RESET),
            Color::Rgb(r, g, b) => format!("\x1b[38;2;{};{};{}m{}{}", r, g, b, body, RESET),
        }
    }
}
