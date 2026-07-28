use std::fs;
use std::os::raw::{c_int, c_long};
use std::path::Path;

const PROC_ROOT: &str = "/proc";
const SC_PAGESIZE: c_int = 30;

extern "C" {
    fn sysconf(name: c_int) -> c_long;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ProcessStat {
    pub(super) pid: u32,
    pub(super) ppid: u32,
    pub(super) start_time: u64,
    pub(super) cpu_ticks: u64,
    pub(super) rss_pages: u64,
}

pub(super) fn read(pid: u32) -> Option<ProcessStat> {
    let path = Path::new(PROC_ROOT).join(pid.to_string()).join("stat");
    let body = fs::read_to_string(path).ok()?;
    parse(&body)
}

pub(super) fn page_size() -> Option<u64> {
    let value = unsafe { sysconf(SC_PAGESIZE) };
    u64::try_from(value).ok().filter(|value| *value > 0)
}

fn parse(body: &str) -> Option<ProcessStat> {
    let open = body.find('(')?;
    let close = body.rfind(") ")?;
    if close <= open {
        return None;
    }

    let pid = body[..open].trim().parse().ok()?;
    let fields: Vec<&str> = body[close + 2..].split_whitespace().collect();
    if fields.len() < 22 {
        return None;
    }

    let ppid = fields[1].parse().ok()?;
    let user_ticks: u64 = fields[11].parse().ok()?;
    let system_ticks: u64 = fields[12].parse().ok()?;
    let child_user_ticks = fields[13].parse::<i64>().ok()?.max(0) as u64;
    let child_system_ticks = fields[14].parse::<i64>().ok()?.max(0) as u64;
    let start_time = fields[19].parse().ok()?;
    let rss_pages = fields[21].parse::<i64>().ok()?.max(0) as u64;
    let cpu_ticks = user_ticks
        .saturating_add(system_ticks)
        .saturating_add(child_user_ticks)
        .saturating_add(child_system_ticks);

    Some(ProcessStat {
        pid,
        ppid,
        start_time,
        cpu_ticks,
        rss_pages,
    })
}

#[cfg(test)]
mod tests {
    use super::{parse, read, ProcessStat};

    fn process(
        pid: u32,
        ppid: u32,
        start_time: u64,
        cpu_ticks: u64,
        rss_pages: u64,
    ) -> ProcessStat {
        ProcessStat {
            pid,
            ppid,
            start_time,
            cpu_ticks,
            rss_pages,
        }
    }

    fn stat_line() -> String {
        let mut fields = vec!["0"; 22];
        fields[0] = "S";
        fields[1] = "42";
        fields[11] = "11";
        fields[12] = "7";
        fields[13] = "5";
        fields[14] = "3";
        fields[19] = "98765";
        fields[21] = "1234";
        format!("77 (claude (worker) name) {}", fields.join(" "))
    }

    #[test]
    fn parses_stat_with_spaces_and_parentheses_in_comm() {
        assert_eq!(parse(&stat_line()), Some(process(77, 42, 98765, 26, 1234)));
    }

    #[test]
    fn rejects_truncated_stat() {
        assert_eq!(parse("77 (claude) S 1 2 3"), None);
    }

    #[test]
    fn clamps_negative_child_cpu_ticks() {
        let mut fields = vec!["0"; 22];
        fields[0] = "S";
        fields[1] = "42";
        fields[11] = "11";
        fields[12] = "7";
        fields[13] = "-5";
        fields[14] = "-3";
        fields[19] = "98765";
        fields[21] = "1234";
        let body = format!("77 (claude) {}", fields.join(" "));

        assert_eq!(parse(&body), Some(process(77, 42, 98765, 18, 1234)));
    }

    #[test]
    fn reads_current_process() {
        let pid = std::process::id();
        let stat = read(pid).unwrap();

        assert_eq!(stat.pid, pid);
    }
}
