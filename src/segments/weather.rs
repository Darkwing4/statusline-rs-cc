mod system_location;

use serde_json::Value;

pub use crate::config_schema::Weather;
use crate::segments::background_command::BackgroundCommand;
use crate::segments::{GitCache, Segment};

use self::system_location::system_location;

const CURL: &str = "curl";
const REQUEST_TIMEOUT_SECONDS: &str = "10";

impl Segment for Weather {
    fn render(&self, _json: &Value, _git: &mut GitCache) -> Option<String> {
        let text = self.background_command().cached_line()?;

        Some(self.color.paint(&format!("{}{}", self.prefix, text)))
    }
}

impl Weather {
    pub(super) fn background_command(&self) -> BackgroundCommand {
        BackgroundCommand {
            command: CURL.to_string(),
            args: vec![
                "-s".to_string(),
                "--max-time".to_string(),
                REQUEST_TIMEOUT_SECONDS.to_string(),
                self.request_url(),
            ],
            stdin_input: String::new(),
            ttl_seconds: self.ttl_seconds,
            max_chars: self.max_chars,
        }
    }

    fn request_url(&self) -> String {
        let location = self.resolved_location();

        report_url(&location, &self.format)
    }

    fn resolved_location(&self) -> String {
        system_location()
            .filter(|location| !location.is_empty())
            .unwrap_or_else(|| self.location.clone())
    }
}

fn report_url(location: &str, format: &str) -> String {
    let path = url_path(location);

    format!("https://wttr.in/{}?format={}", path, url_query(format))
}

fn url_path(location: &str) -> String {
    location
        .chars()
        .filter(|character| {
            character.is_alphanumeric() || matches!(character, ' ' | '-' | '_' | ',' | '.')
        })
        .map(|character| if character == ' ' { '+' } else { character })
        .collect()
}

fn url_query(format: &str) -> String {
    format
        .chars()
        .map(|character| match character {
            ' ' => "+".to_string(),
            '&' | '#' | '?' => format!("%{:02X}", character as u8),
            other => other.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{report_url, Weather};
    use crate::types::Color;

    fn segment(location: &str) -> Weather {
        Weather {
            color: Color::Named(90),
            prefix: "".to_string(),
            location: location.to_string(),
            format: "%c+%t".to_string(),
            ttl_seconds: 1800,
            max_chars: 24,
        }
    }

    #[test]
    fn builds_a_wttr_url_from_a_city_name() {
        assert_eq!(
            report_url("New York", "%c+%t"),
            "https://wttr.in/New+York?format=%c+%t"
        );
    }

    #[test]
    fn drops_characters_that_do_not_belong_in_a_location() {
        assert_eq!(
            report_url("Moscow/../etc?x=1", "%l"),
            "https://wttr.in/Moscow..etcx1?format=%l"
        );
    }

    #[test]
    fn escapes_query_separators_inside_the_format() {
        assert_eq!(
            report_url("Berlin", "%c %t&evil=1"),
            "https://wttr.in/Berlin?format=%c+%t%26evil=1"
        );
    }

    #[test]
    fn falls_back_to_the_configured_location_when_the_system_has_none() {
        let configured = segment("Lisbon");
        let url = configured.request_url();

        assert!(url.starts_with("https://wttr.in/"));
        assert!(url.ends_with("?format=%c+%t"));
    }
}
