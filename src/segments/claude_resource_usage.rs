#[cfg(target_os = "linux")]
mod linux;

use serde::Deserialize;
use serde_json::Value;

use crate::segments::{GitCache, Segment};
use crate::types::Color;

#[derive(Deserialize)]
pub struct ClaudeResourceUsage {
    pub color: Color,
    pub cpu_prefix: String,
    pub memory_prefix: String,
}

#[cfg(target_os = "linux")]
struct ResourceUsage {
    cpu_percent: Option<u64>,
    memory_bytes: u64,
}

#[cfg(target_os = "linux")]
impl Segment for ClaudeResourceUsage {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let session_id = json
            .get("session_id")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())?;

        let usage = linux::collect(session_id)?;
        Some(
            self.color
                .paint(&format_usage(&self.cpu_prefix, &self.memory_prefix, &usage)),
        )
    }
}

#[cfg(not(target_os = "linux"))]
impl Segment for ClaudeResourceUsage {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let _ = (&self.color, &self.cpu_prefix, &self.memory_prefix, json);
        None
    }
}

#[cfg(target_os = "linux")]
fn format_usage(cpu_prefix: &str, memory_prefix: &str, usage: &ResourceUsage) -> String {
    let cpu = usage
        .cpu_percent
        .map(|percent| format!("{percent}%"))
        .unwrap_or_else(|| "—".to_string());
    let memory_mib = usage.memory_bytes.saturating_add(512 * 1024) / (1024 * 1024);

    format!("{cpu_prefix}{cpu} {memory_prefix}{memory_mib}M")
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::{format_usage, ResourceUsage};

    #[test]
    fn formats_first_sample_with_memory() {
        let usage = ResourceUsage {
            cpu_percent: None,
            memory_bytes: 684 * 1024 * 1024,
        };

        assert_eq!(format_usage("CPU ", "RAM ", &usage), "CPU — RAM 684M");
    }

    #[test]
    fn formats_live_cpu_and_rounds_memory() {
        let usage = ResourceUsage {
            cpu_percent: Some(7),
            memory_bytes: 684 * 1024 * 1024 + 600 * 1024,
        };

        assert_eq!(format_usage("CPU ", "RAM ", &usage), "CPU 7% RAM 685M");
    }
}
