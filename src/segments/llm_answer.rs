use serde_json::Value;

pub use crate::config_schema::LlmAnswer;
use crate::segments::background_command::BackgroundCommand;
use crate::segments::{GitCache, Segment};

impl Segment for LlmAnswer {
    fn render(&self, _json: &Value, _git: &mut GitCache) -> Option<String> {
        let text = self.background_command().cached_line()?;

        Some(self.color.paint(&format!("{}{}", self.prefix, text)))
    }

    fn standalone(&self) -> bool {
        true
    }
}

impl LlmAnswer {
    pub(super) fn background_command(&self) -> BackgroundCommand {
        BackgroundCommand {
            command: self.command.clone(),
            args: self.args.clone(),
            stdin_input: self.prompt.clone(),
            ttl_seconds: self.ttl_seconds,
            max_chars: self.max_chars,
        }
    }
}
