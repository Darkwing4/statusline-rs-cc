use serde_json::Value;

const FAILURE_MARKER: &str = "✗";
const MAX_COMMAND_CHARS: usize = 60;

pub const HOOK_SOURCE: &str = "hook";

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Write { session: String, text: String },
    ClearOwnNotice { session: String },
    Ignore,
}

pub fn outcome(payload: &Value) -> Outcome {
    if payload.get("tool_name").and_then(Value::as_str) != Some("Bash") {
        return Outcome::Ignore;
    }

    let Some(session) = payload
        .get("session_id")
        .and_then(Value::as_str)
        .filter(|session| !session.is_empty())
    else {
        return Outcome::Ignore;
    };

    let response = payload.get("tool_response");
    let exit_code = response
        .and_then(|response| response.get("exit_code"))
        .and_then(Value::as_i64);
    let interrupted = response
        .and_then(|response| response.get("interrupted"))
        .and_then(Value::as_bool)
        == Some(true);

    let command = payload
        .get("tool_input")
        .and_then(|input| input.get("command"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();

    let session = session.to_string();

    match failure_reason(exit_code, interrupted) {
        Some(reason) => Outcome::Write {
            session,
            text: format!("{} {} ({})", FAILURE_MARKER, short_command(command), reason),
        },
        None => Outcome::ClearOwnNotice { session },
    }
}

fn failure_reason(exit_code: Option<i64>, interrupted: bool) -> Option<String> {
    if interrupted {
        return Some("interrupted".to_string());
    }

    match exit_code {
        Some(0) | None => None,
        Some(code) => Some(format!("exit {}", code)),
    }
}

fn short_command(command: &str) -> String {
    let first_line = command.lines().next().unwrap_or("").trim();

    if first_line.chars().count() <= MAX_COMMAND_CHARS {
        return first_line.to_string();
    }

    let kept: String = first_line.chars().take(MAX_COMMAND_CHARS - 1).collect();

    format!("{}…", kept)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{outcome, Outcome};

    fn event(tool: &str, command: &str, response: serde_json::Value) -> serde_json::Value {
        json!({
            "hook_event_name": "PostToolUse",
            "session_id": "s-1",
            "tool_name": tool,
            "tool_input": {"command": command},
            "tool_response": response,
        })
    }

    #[test]
    fn writes_a_notice_for_a_command_that_failed() {
        assert_eq!(
            outcome(&event("Bash", "cargo test", json!({"exit_code": 101}))),
            Outcome::Write {
                session: "s-1".to_string(),
                text: "✗ cargo test (exit 101)".to_string(),
            }
        );
    }

    #[test]
    fn writes_a_notice_for_a_command_that_was_interrupted() {
        assert_eq!(
            outcome(&event(
                "Bash",
                "sleep 100",
                json!({"exit_code": 0, "interrupted": true})
            )),
            Outcome::Write {
                session: "s-1".to_string(),
                text: "✗ sleep 100 (interrupted)".to_string(),
            }
        );
    }

    #[test]
    fn clears_its_own_notice_once_a_command_succeeds() {
        assert_eq!(
            outcome(&event("Bash", "cargo test", json!({"exit_code": 0}))),
            Outcome::ClearOwnNotice {
                session: "s-1".to_string()
            }
        );
    }

    #[test]
    fn ignores_events_it_cannot_judge() {
        assert_eq!(outcome(&event("Edit", "", json!({}))), Outcome::Ignore);
        assert_eq!(
            outcome(&json!({"tool_name": "Bash", "tool_response": {"exit_code": 1}})),
            Outcome::Ignore
        );
    }

    #[test]
    fn keeps_only_the_first_line_of_a_long_command() {
        let Outcome::Write { text, .. } = outcome(&event(
            "Bash",
            "for file in *.rs; do echo checking a rather long file name $file; done\nsecond line",
            json!({"exit_code": 2}),
        )) else {
            panic!("expected a notice");
        };

        assert_eq!(
            text,
            "✗ for file in *.rs; do echo checking a rather long file name … (exit 2)"
        );
    }
}
