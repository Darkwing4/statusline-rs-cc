use std::env;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::segments::background_command::REQUEST_FLAG;
use crate::statusline_hook::{self, Outcome};
use crate::statusline_notice_store::{self, Notice};
use crate::statusline_reminder_store::{self, Reminder};

pub const CONFIG_FLAG: &str = "--config";
pub const CHECK_CONFIG_FLAG: &str = "--check-config";
pub const SCHEMA_FLAG: &str = "--schema";
pub const REFRESH_FLAG: &str = "--refresh";
pub const USAGE_REFRESH_FLAG: &str = "--refresh-usage";
pub const NOTICE_FLAG: &str = "--notice";
pub const NOTICE_CLEAR_FLAG: &str = "--notice-clear";
pub const TTL_FLAG: &str = "--ttl";
pub const SESSION_FLAG: &str = "--session";
pub const REMIND_FLAG: &str = "--remind";
pub const REMIND_CLEAR_FLAG: &str = "--remind-clear";
pub const IN_FLAG: &str = "--in";
pub const FOR_FLAG: &str = "--for";
pub const ALL_FLAG: &str = "--all";
pub const HOOK_FLAG: &str = "--hook";

const SESSION_ENV: &str = "CLAUDE_CODE_SESSION_ID";
const DEFAULT_REMINDER_LIFETIME_SECONDS: u64 = 3600;
const DEFAULT_HOOK_NOTICE_SECONDS: u64 = 900;

pub const USAGE: &str = concat!(
    "usage:\n",
    "  statusline [--config <path>]                render a status line from stdin json\n",
    "  statusline --check-config <path>            validate a config without rendering\n",
    "  statusline --schema                         print the segment catalogue as json\n",
    "  statusline --notice <text> [--ttl <secs>] [--session <id>]\n",
    "  statusline --notice-clear [--session <id>]\n",
    "  statusline --remind <text> --in <30m> [--for <2h>]\n",
    "  statusline --remind-clear [--all]\n",
    "  statusline --hook [--ttl <secs>]            PostToolUse hook: report failed commands\n",
);

pub enum Command {
    Render {
        config_path: Option<PathBuf>,
    },
    CheckConfig {
        path: PathBuf,
    },
    Schema,
    Refresh {
        fingerprint: String,
        request_base: Option<PathBuf>,
    },
    RefreshUsage,
    SetNotice {
        session: Option<String>,
        text: String,
        ttl_seconds: Option<u64>,
    },
    ClearNotice {
        session: Option<String>,
    },
    AddReminder {
        text: String,
        in_seconds: u64,
        for_seconds: u64,
    },
    ClearReminders {
        all: bool,
    },
    Hook {
        ttl_seconds: u64,
    },
}

pub fn parse<I>(args: I) -> Result<Command, String>
where
    I: IntoIterator<Item = String>,
{
    let args: Vec<String> = args.into_iter().collect();

    let Some(first) = args.first() else {
        return Ok(Command::Render { config_path: None });
    };

    match first.as_str() {
        CONFIG_FLAG => parse_config(&args[1..]),
        CHECK_CONFIG_FLAG => parse_check_config(&args[1..]),
        SCHEMA_FLAG => parse_schema(&args[1..]),
        REFRESH_FLAG => parse_refresh(&args[1..]),
        USAGE_REFRESH_FLAG => parse_refresh_usage(&args[1..]),
        NOTICE_FLAG => parse_set_notice(&args[1..]),
        NOTICE_CLEAR_FLAG => parse_clear_notice(&args[1..]),
        REMIND_FLAG => parse_add_reminder(&args[1..]),
        REMIND_CLEAR_FLAG => parse_clear_reminders(&args[1..]),
        HOOK_FLAG => parse_hook(&args[1..]),
        unknown => Err(format!("unknown argument {}\n{}", unknown, USAGE)),
    }
}

pub fn apply(command: Command) -> Result<(), String> {
    match command {
        Command::SetNotice {
            session,
            text,
            ttl_seconds,
        } => {
            let notice = Notice {
                text,
                expires_at: ttl_seconds.map(|ttl| now_seconds() + ttl as i64),
                source: None,
            };

            statusline_notice_store::store(&resolve_session(session)?, &notice).map(|_| ())
        }
        Command::ClearNotice { session } => {
            statusline_notice_store::clear(&resolve_session(session)?)
        }
        Command::AddReminder {
            text,
            in_seconds,
            for_seconds,
        } => {
            let now = now_seconds();
            let show_at = now + in_seconds as i64;

            statusline_reminder_store::add(
                Reminder {
                    text,
                    show_at,
                    expires_at: show_at + for_seconds as i64,
                },
                now,
            )
        }
        Command::ClearReminders { all } => {
            statusline_reminder_store::clear(!all, now_seconds()).map(|_| ())
        }
        Command::Hook { ttl_seconds } => apply_hook(ttl_seconds),
        _ => Ok(()),
    }
}

fn apply_hook(ttl_seconds: u64) -> Result<(), String> {
    let Some(payload) = crate::statusline_input::read() else {
        return Ok(());
    };

    match statusline_hook::outcome(&payload) {
        Outcome::Write { session, text } => {
            let notice = Notice {
                text,
                expires_at: Some(now_seconds() + ttl_seconds as i64),
                source: Some(statusline_hook::HOOK_SOURCE.to_string()),
            };

            statusline_notice_store::store(&session, &notice).map(|_| ())
        }
        Outcome::ClearOwnNotice { session } => {
            statusline_notice_store::clear_from_source(&session, statusline_hook::HOOK_SOURCE)
        }
        Outcome::Ignore => Ok(()),
    }
}

fn parse_config(rest: &[String]) -> Result<Command, String> {
    match rest {
        [path] => Ok(Command::Render {
            config_path: Some(PathBuf::from(path)),
        }),
        _ => Err(format!("{} takes a path\n{}", CONFIG_FLAG, USAGE)),
    }
}

fn parse_check_config(rest: &[String]) -> Result<Command, String> {
    match rest {
        [path] => Ok(Command::CheckConfig {
            path: PathBuf::from(path),
        }),
        _ => Err(format!("{} takes a path\n{}", CHECK_CONFIG_FLAG, USAGE)),
    }
}

fn parse_schema(rest: &[String]) -> Result<Command, String> {
    if !rest.is_empty() {
        return Err(format!("{} takes no arguments", SCHEMA_FLAG));
    }

    Ok(Command::Schema)
}

fn parse_refresh(rest: &[String]) -> Result<Command, String> {
    let (fingerprint, request_base) = match rest {
        [fingerprint] => (fingerprint, None),
        [fingerprint, flag, base] if flag == REQUEST_FLAG => {
            (fingerprint, Some(PathBuf::from(base)))
        }
        _ => {
            return Err(format!(
                "{} takes a fingerprint and an optional {} <path>",
                REFRESH_FLAG, REQUEST_FLAG
            ))
        }
    };

    Ok(Command::Refresh {
        fingerprint: fingerprint.clone(),
        request_base,
    })
}

fn parse_refresh_usage(rest: &[String]) -> Result<Command, String> {
    if !rest.is_empty() {
        return Err(format!("{} takes no arguments", USAGE_REFRESH_FLAG));
    }

    Ok(Command::RefreshUsage)
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

fn parse_add_reminder(rest: &[String]) -> Result<Command, String> {
    let (text, rest) = rest
        .split_first()
        .ok_or_else(|| format!("{} needs a text\n{}", REMIND_FLAG, USAGE))?;

    if text.trim().is_empty() {
        return Err(format!("{} needs a non-empty text", REMIND_FLAG));
    }

    let mut in_seconds = None;
    let mut for_seconds = DEFAULT_REMINDER_LIFETIME_SECONDS;
    let mut index = 0;

    while index < rest.len() {
        match rest[index].as_str() {
            IN_FLAG => in_seconds = Some(parse_duration(option_value(rest, index, IN_FLAG)?)?),
            FOR_FLAG => for_seconds = parse_duration(option_value(rest, index, FOR_FLAG)?)?,
            unknown => return Err(format!("unknown argument {}\n{}", unknown, USAGE)),
        }

        index += 2;
    }

    let in_seconds =
        in_seconds.ok_or_else(|| format!("{} needs {} <30m>\n{}", REMIND_FLAG, IN_FLAG, USAGE))?;

    Ok(Command::AddReminder {
        text: text.clone(),
        in_seconds,
        for_seconds,
    })
}

fn parse_hook(rest: &[String]) -> Result<Command, String> {
    let ttl_seconds = match rest {
        [] => DEFAULT_HOOK_NOTICE_SECONDS,
        [flag, value] if flag == TTL_FLAG => parse_duration(value)?,
        _ => return Err(format!("{} takes only {}\n{}", HOOK_FLAG, TTL_FLAG, USAGE)),
    };

    Ok(Command::Hook { ttl_seconds })
}

fn parse_clear_reminders(rest: &[String]) -> Result<Command, String> {
    match rest {
        [] => Ok(Command::ClearReminders { all: false }),
        [flag] if flag == ALL_FLAG => Ok(Command::ClearReminders { all: true }),
        _ => Err(format!(
            "{} takes only {}\n{}",
            REMIND_CLEAR_FLAG, ALL_FLAG, USAGE
        )),
    }
}

fn parse_duration(value: &str) -> Result<u64, String> {
    let trimmed = value.trim();
    let (digits, unit_seconds) = match trimmed.chars().last() {
        Some('s') => (&trimmed[..trimmed.len() - 1], 1),
        Some('m') => (&trimmed[..trimmed.len() - 1], 60),
        Some('h') => (&trimmed[..trimmed.len() - 1], 3_600),
        Some('d') => (&trimmed[..trimmed.len() - 1], 86_400),
        _ => (trimmed, 1),
    };

    let amount = digits
        .parse::<u64>()
        .map_err(|_| format!("expected a duration like 45s, 30m, 2h, got {}", value))?;

    amount
        .checked_mul(unit_seconds)
        .ok_or_else(|| format!("duration is too large: {}", value))
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
        assert!(matches!(
            parse_args(&[]),
            Ok(Command::Render { config_path: None })
        ));
    }

    #[test]
    fn renders_with_an_explicit_config() {
        assert!(matches!(
            parse_args(&["--config", "custom.ron"]),
            Ok(Command::Render { config_path: Some(path) })
                if path == std::path::Path::new("custom.ron")
        ));
        assert!(parse_args(&["--config"]).is_err());
        assert!(parse_args(&["--config", "custom.ron", "extra"]).is_err());
    }

    #[test]
    fn parses_a_config_check() {
        assert!(matches!(
            parse_args(&["--check-config", "custom.ron"]),
            Ok(Command::CheckConfig { path }) if path == std::path::Path::new("custom.ron")
        ));
        assert!(parse_args(&["--check-config"]).is_err());
    }

    #[test]
    fn parses_a_schema_request() {
        assert!(matches!(parse_args(&["--schema"]), Ok(Command::Schema)));
        assert!(parse_args(&["--schema", "extra"]).is_err());
    }

    #[test]
    fn parses_a_refresh_request() {
        assert!(matches!(
            parse_args(&["--refresh", "abc123"]),
            Ok(Command::Refresh { fingerprint, request_base: None }) if fingerprint == "abc123"
        ));
        assert!(matches!(
            parse_args(&["--refresh", "abc123", "--request", "/cache/insight-abc"]),
            Ok(Command::Refresh { request_base: Some(base), .. })
                if base == std::path::Path::new("/cache/insight-abc")
        ));
        assert!(parse_args(&["--refresh"]).is_err());
        assert!(parse_args(&["--refresh", "abc123", "extra"]).is_err());
    }

    #[test]
    fn parses_a_usage_refresh_request() {
        assert!(matches!(
            parse_args(&["--refresh-usage"]),
            Ok(Command::RefreshUsage)
        ));
        assert!(parse_args(&["--refresh-usage", "extra"]).is_err());
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

    #[test]
    fn parses_a_reminder_with_its_delay_and_lifetime() {
        let Ok(Command::AddReminder {
            text,
            in_seconds,
            for_seconds,
        }) = parse_args(&["--remind", "созвон", "--in", "30m", "--for", "2h"])
        else {
            panic!("expected a reminder command");
        };

        assert_eq!(text, "созвон");
        assert_eq!(in_seconds, 1_800);
        assert_eq!(for_seconds, 7_200);
    }

    #[test]
    fn reads_durations_in_seconds_minutes_hours_and_days() {
        for (argument, expected) in [
            ("45s", 45),
            ("90", 90),
            ("2m", 120),
            ("1h", 3_600),
            ("1d", 86_400),
        ] {
            let Ok(Command::AddReminder { in_seconds, .. }) =
                parse_args(&["--remind", "текст", "--in", argument])
            else {
                panic!("expected a reminder command for {argument}");
            };

            assert_eq!(in_seconds, expected);
        }
    }

    #[test]
    fn defaults_a_reminder_to_an_hour_on_screen() {
        let Ok(Command::AddReminder { for_seconds, .. }) =
            parse_args(&["--remind", "текст", "--in", "5m"])
        else {
            panic!("expected a reminder command");
        };

        assert_eq!(for_seconds, 3_600);
    }

    #[test]
    fn rejects_broken_reminder_arguments() {
        assert!(parse_args(&["--remind", "текст"]).is_err());
        assert!(parse_args(&["--remind", "  ", "--in", "5m"]).is_err());
        assert!(parse_args(&["--remind", "текст", "--in", "soon"]).is_err());
        assert!(parse_args(&["--remind", "текст", "--in", "5x"]).is_err());
        assert!(parse_args(&["--remind", "текст", "--in"]).is_err());
    }

    #[test]
    fn parses_a_reminder_clear_request() {
        assert!(matches!(
            parse_args(&["--remind-clear"]),
            Ok(Command::ClearReminders { all: false })
        ));
        assert!(matches!(
            parse_args(&["--remind-clear", "--all"]),
            Ok(Command::ClearReminders { all: true })
        ));
        assert!(parse_args(&["--remind-clear", "everything"]).is_err());
    }
}
