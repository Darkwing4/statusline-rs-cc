use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::Value;

use super::claude_usage_cache;
use crate::claude_config_dir::claude_config_dir;

const CURL: &str = "curl";
const REQUEST_TIMEOUT_SECONDS: &str = "10";
const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const OAUTH_BETA_HEADER: &str = "anthropic-beta: oauth-2025-04-20";
const CREDENTIALS_FILE: &str = ".credentials.json";
const ACCESS_TOKEN_POINTER: &str = "/claudeAiOauth/accessToken";

pub fn refresh_usage() {
    let Some(token) = access_token() else {
        return;
    };

    let Some(body) = fetch(&token) else {
        return;
    };

    claude_usage_cache::store(&body);
}

fn access_token() -> Option<String> {
    let body = fs::read_to_string(credentials_path()?).ok()?;
    let credentials: Value = serde_json::from_str(&body).ok()?;
    let token = credentials.pointer(ACCESS_TOKEN_POINTER)?.as_str()?;

    is_safe_header_value(token).then(|| token.to_string())
}

fn credentials_path() -> Option<PathBuf> {
    Some(claude_config_dir()?.join(CREDENTIALS_FILE))
}

fn is_safe_header_value(token: &str) -> bool {
    !token.is_empty()
        && token
            .chars()
            .all(|character| character.is_ascii_graphic() && character != '"' && character != '\\')
}

fn fetch(token: &str) -> Option<String> {
    let mut child = Command::new(CURL)
        .args([
            "--silent",
            "--fail",
            "--max-time",
            REQUEST_TIMEOUT_SECONDS,
            "--config",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(curl_config(token).as_bytes());
    }

    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }

    let body = String::from_utf8(output.stdout).ok()?;
    serde_json::from_str::<Value>(&body).ok()?;

    Some(body)
}

fn curl_config(token: &str) -> String {
    format!(
        "url = \"{}\"\nheader = \"Authorization: Bearer {}\"\nheader = \"{}\"\nheader = \"Content-Type: application/json\"\n",
        USAGE_URL, token, OAUTH_BETA_HEADER
    )
}

#[cfg(test)]
mod tests {
    use super::{curl_config, is_safe_header_value, OAUTH_BETA_HEADER, USAGE_URL};

    #[test]
    fn rejects_tokens_that_would_break_out_of_the_curl_config() {
        assert!(is_safe_header_value("sk-ant-oat01-AbC_123.xyz-"));
        assert!(!is_safe_header_value(""));
        assert!(!is_safe_header_value("token\"\nurl = \"http://evil"));
        assert!(!is_safe_header_value("token\\"));
        assert!(!is_safe_header_value("with space"));
        assert!(!is_safe_header_value("with\ttab"));
    }

    #[test]
    fn writes_url_and_headers_on_separate_config_lines() {
        let config = curl_config("token-1");
        let lines: Vec<&str> = config.lines().collect();

        assert_eq!(
            lines,
            vec![
                format!("url = \"{}\"", USAGE_URL).as_str(),
                "header = \"Authorization: Bearer token-1\"",
                format!("header = \"{}\"", OAUTH_BETA_HEADER).as_str(),
                "header = \"Content-Type: application/json\"",
            ]
        );
    }
}
