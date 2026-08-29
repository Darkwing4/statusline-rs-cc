use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::statusline_notice_store::{self, Notice};

pub const REFRESH_FLAG: &str = "--refresh";
pub const NOTICE_FLAG: &str = "--notice";
pub const NOTICE_CLEAR_FLAG: &str = "--notice-clear";
pub const TTL_FLAG: &str = "--ttl";
pub const SESSION_FLAG: &str = "--session";

const SESSION_ENV: &str = "CLAUDE_CODE_SESSION_ID";

pub const USAGE: &str = concat!(
    "usage:\n",
    "  statusline                                  render a status line from stdin json\n",
    "  statusline --notice <text> [--ttl <secs>] [--session <id>]\n",
    "  statusline --notice-clear [--session <id>]\n",
);

pub enum Command {
    Render,
    Refresh(String),
    SetNotice {
        session: Option<String>,
        text: String,
        ttl_seconds: Option<u64>,
    },
    ClearNotice {
        session: Option<String>,
    },
}

pub fn parse<I>(args: I) -> Result<Command, String>
where
    I: IntoIterator<Item = String>,
{
    let args: Vec<String> = args.into_iter().collect();

    let Some(first) = args.first() else {
        return Ok(Command::Render);
    };

    match first.as_str() {
        REFRESH_FLAG => parse_refresh(&args[1..]),
        NOTICE_FLAG => parse_set_notice(&args[1..]),
        NOTICE_CLEAR_FLAG => parse_clear_notice(&args[1..]),
        unknown => Err(format!("unknown argument {}\n{}", unknown, USAGE)),
    }
}

pub fn apply_notice(command: Command) -> Result<(), String> {
    match command {
        Command::SetNotice {
            session,
            text,
            ttl_seconds,
        } => {
            let notice = Notice {
                text,
                expires_at: ttl_seconds.map(|ttl| now_seconds() + ttl as i64),
            };

            statusline_notice_store::store(&resolve_session(session)?, &notice).map(|_| ())
        }
        Command::ClearNotice { session } => {
            statusline_notice_store::clear(&resolve_session(session)?)
        }
        _ => Ok(()),
    }
}

fn parse_refresh(rest: &[String]) -> Result<Command, String> {
    let [fingerprint] = rest else {
        return Err(format!("{} takes exactly one fingerprint", REFRESH_FLAG));
    };

    Ok(Command::Refresh(fingerprint.clone()))
}

fn parse_set_notice(rest: &[String]) -> Result<Command, String> {
    let (text, rest) = rest
        .split_first()
        .ok_or_else(|| format!("{} needs a text\n{}", NOTICE_FLAG, USAGE))?;

    if text.trim().is_empty() {
        return Err(format!("{} needs a non-empty text", NOTICE_FLAG));
    }

    let mut ttl_seconds = None;
    let mut session = None;
    let mut index = 0;

    while index < rest.len() {
        match rest[index].as_str() {
            TTL_FLAG => {
                let value = option_value(rest, index, TTL_FLAG)?;
                ttl_seconds =
                    Some(value.parse::<u64>().map_err(|_| {
                        format!("{} expects whole seconds, got {}", TTL_FLAG, value)
                    })?);
            }
            SESSION_FLAG => session = Some(option_value(rest, index, SESSION_FLAG)?.to_string()),
            unknown => return Err(format!("unknown argument {}\n{}", unknown, USAGE)),
        }

        index += 2;
    }

    Ok(Command::SetNotice {
        session,
        text: text.clone(),
        ttl_seconds,
    })
}

fn parse_clear_notice(rest: &[String]) -> Result<Command, String> {
    let session = match rest {
        [] => None,
        [flag, value] if flag == SESSION_FLAG => Some(value.clone()),
        _ => {
            return Err(format!(
                "{} takes only {}\n{}",
                NOTICE_CLEAR_FLAG, SESSION_FLAG, USAGE
            ))
        }
    };

    Ok(Command::ClearNotice { session })
}

fn option_value<'a>(rest: &'a [String], index: usize, flag: &str) -> Result<&'a str, String> {
    rest.get(index + 1)
        .map(String::as_str)
        .ok_or_else(|| format!("{} needs a value", flag))
}

fn resolve_session(explicit: Option<String>) -> Result<String, String> {
    if let Some(session) = explicit.filter(|value| !value.trim().is_empty()) {
        return Ok(session);
    }

    env::var(SESSION_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            format!(
                "no session id: {} is unset, pass {} <id>",
                SESSION_ENV, SESSION_FLAG
            )
        })
}

fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{parse, Command};

    fn parse_args(args: &[&str]) -> Result<Command, String> {
        parse(args.iter().map(|arg| arg.to_string()))
    }

    #[test]
    fn renders_when_no_arguments_are_given() {
        assert!(matches!(parse_args(&[]), Ok(Command::Render)));
    }

    #[test]
    fn parses_a_refresh_request() {
        assert!(matches!(
            parse_args(&["--refresh", "abc123"]),
            Ok(Command::Refresh(fingerprint)) if fingerprint == "abc123"
        ));
        assert!(parse_args(&["--refresh"]).is_err());
        assert!(parse_args(&["--refresh", "abc123", "extra"]).is_err());
    }

    #[test]
    fn parses_a_notice_with_its_options_in_any_order() {
        let Ok(Command::SetNotice {
            session,
            text,
            ttl_seconds,
        }) = parse_args(&[
            "--notice",
            "созвон 15:00",
            "--session",
            "s-1",
            "--ttl",
            "600",
        ])
        else {
            panic!("expected a notice command");
        };

        assert_eq!(text, "созвон 15:00");
        assert_eq!(session.as_deref(), Some("s-1"));
        assert_eq!(ttl_seconds, Some(600));
    }

    #[test]
    fn parses_a_notice_without_options() {
        let Ok(Command::SetNotice {
            session,
            ttl_seconds,
            ..
        }) = parse_args(&["--notice", "текст"])
        else {
            panic!("expected a notice command");
        };

        assert_eq!(session, None);
        assert_eq!(ttl_seconds, None);
    }

    #[test]
    fn rejects_broken_notice_arguments() {
        assert!(parse_args(&["--notice"]).is_err());
        assert!(parse_args(&["--notice", "  "]).is_err());
        assert!(parse_args(&["--notice", "текст", "--ttl"]).is_err());
        assert!(parse_args(&["--notice", "текст", "--ttl", "soon"]).is_err());
        assert!(parse_args(&["--notice", "текст", "--session"]).is_err());
        assert!(parse_args(&["--notice", "текст", "--wat", "1"]).is_err());
    }

    #[test]
    fn parses_a_clear_request() {
        assert!(matches!(
            parse_args(&["--notice-clear"]),
            Ok(Command::ClearNotice { session: None })
        ));
        assert!(matches!(
            parse_args(&["--notice-clear", "--session", "s-1"]),
            Ok(Command::ClearNotice { session: Some(session) }) if session == "s-1"
        ));
        assert!(parse_args(&["--notice-clear", "s-1"]).is_err());
    }

    #[test]
    fn rejects_unknown_commands() {
        assert!(parse_args(&["--help"]).is_err());
    }
}
