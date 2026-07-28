#[cfg(target_os = "linux")]
mod linux;

use serde_json::Value;

pub use crate::config_schema::ClaudeResourceUsage;
use crate::segments::{GitCache, Segment};

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
fn format_usage(cpu_prefix: &str, rss_prefix: &str, usage: &ResourceUsage) -> String {
    let cpu = usage
        .cpu_percent
        .map(|percent| {
            let whole_cores = percent / 100;
            let fractional_cores = percent % 100;
            format!("{whole_cores}.{fractional_cores:02}c")
        })
        .unwrap_or_else(|| "—".to_string());
    let rss_mib = usage.memory_bytes.saturating_add(512 * 1024) / (1024 * 1024);

    format!("{cpu_prefix}{cpu} {rss_prefix}{rss_mib} MiB")
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::{format_usage, ResourceUsage};

    #[test]
    fn formats_first_sample_with_rss() {
        let usage = ResourceUsage {
            cpu_percent: None,
            memory_bytes: 684 * 1024 * 1024,
        };

        assert_eq!(format_usage("CPU ", "RSS ", &usage), "CPU — RSS 684 MiB");
    }

    #[test]
    fn formats_cpu_as_cores_and_rounds_rss() {
        let usage = ResourceUsage {
            cpu_percent: Some(110),
            memory_bytes: 684 * 1024 * 1024 + 600 * 1024,
        };

        assert_eq!(
            format_usage("CPU ", "RSS ", &usage),
            "CPU 1.10c RSS 685 MiB"
        );
    }
}
