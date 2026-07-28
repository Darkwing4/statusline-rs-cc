use std::fs::{self, DirBuilder, OpenOptions};
use std::io::{Read, Write};
use std::os::raw::{c_int, c_long};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use super::session_root::ResolvedRoot;

const PROC_ROOT: &str = "/proc";
const O_NOFOLLOW: c_int = 0o400000;
const SC_CLK_TCK: c_int = 2;
const MAX_STATE_BYTES: u64 = 16 * 1024;

extern "C" {
    fn geteuid() -> u32;
    fn sysconf(name: c_int) -> c_long;
}

#[derive(Debug, Eq, PartialEq)]
struct CpuSnapshot {
    session: String,
    root_pid: u32,
    root_start: u64,
    cpu_ticks: u64,
    uptime_nanos: u64,
}

pub(super) fn sample(session_id: &str, root: ResolvedRoot, cpu_ticks: u64) -> Option<u64> {
    let uptime_nanos = read_uptime_nanos()?;
    let clock_ticks = positive_sysconf(SC_CLK_TCK)?;
    let session = hex_encode(session_id.as_bytes());
    let current = CpuSnapshot {
        session,
        root_pid: root.pid,
        root_start: root.start_time,
        cpu_ticks,
        uptime_nanos,
    };
    let directory = cache_directory()?;
    let state_path = directory.join(state_file_name(session_id, root));
    let previous = read_cpu_snapshot(&state_path).filter(|snapshot| {
        snapshot.session == current.session
            && snapshot.root_pid == current.root_pid
            && snapshot.root_start == current.root_start
    });
    let percent = previous
        .as_ref()
        .and_then(|snapshot| cpu_delta_percent(snapshot, &current, clock_ticks));

    let _ = write_cpu_snapshot(&state_path, &current);
    percent
}

fn cpu_delta_percent(
    previous: &CpuSnapshot,
    current: &CpuSnapshot,
    clock_ticks: u64,
) -> Option<u64> {
    if clock_ticks == 0
        || current.uptime_nanos <= previous.uptime_nanos
        || current.cpu_ticks < previous.cpu_ticks
    {
        return None;
    }

    let elapsed_nanos = current.uptime_nanos - previous.uptime_nanos;
    let cpu_ticks = current.cpu_ticks - previous.cpu_ticks;
    let numerator = cpu_ticks as f64 * 100.0 * 1_000_000_000.0;
    let denominator = clock_ticks as f64 * elapsed_nanos as f64;
    let percent = (numerator / denominator).round();

    if !percent.is_finite() || percent < 0.0 || percent > u64::MAX as f64 {
        return None;
    }

    Some(percent as u64)
}

fn read_uptime_nanos() -> Option<u64> {
    let body = fs::read_to_string(Path::new(PROC_ROOT).join("uptime")).ok()?;
    parse_uptime_nanos(body.split_whitespace().next()?)
}

fn parse_uptime_nanos(value: &str) -> Option<u64> {
    let (seconds, fraction) = value.split_once('.').unwrap_or((value, ""));
    let seconds: u64 = seconds.parse().ok()?;
    if !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    let mut fractional_nanos = 0_u64;
    let mut digits = 0_u32;
    for byte in fraction.bytes().take(9) {
        fractional_nanos = fractional_nanos
            .saturating_mul(10)
            .saturating_add(u64::from(byte - b'0'));
        digits += 1;
    }
    fractional_nanos = fractional_nanos.saturating_mul(10_u64.pow(9 - digits));

    seconds
        .checked_mul(1_000_000_000)?
        .checked_add(fractional_nanos)
}

fn positive_sysconf(name: c_int) -> Option<u64> {
    let value = unsafe { sysconf(name) };
    u64::try_from(value).ok().filter(|value| *value > 0)
}

fn cache_directory() -> Option<PathBuf> {
    let uid = effective_uid();

    if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR").filter(|value| !value.is_empty()) {
        let runtime = PathBuf::from(runtime);
        if runtime.is_absolute() && secure_directory(&runtime, uid) {
            if let Some(directory) =
                create_secure_directory(&runtime, "statusline-resource-usage", uid)
            {
                return Some(directory);
            }
        }
    }

    let temporary = std::env::temp_dir();
    create_secure_directory(&temporary, &format!("statusline-resource-usage-{uid}"), uid)
}

fn create_secure_directory(base: &Path, name: &str, uid: u32) -> Option<PathBuf> {
    let directory = base.join(name);
    let mut builder = DirBuilder::new();
    builder.mode(0o700);

    match builder.create(&directory) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => return None,
    }

    secure_directory(&directory, uid).then_some(directory)
}

fn secure_directory(path: &Path, uid: u32) -> bool {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return false;
    };

    metadata.file_type().is_dir() && metadata.uid() == uid && metadata.mode() & 0o077 == 0
}

fn effective_uid() -> u32 {
    unsafe { geteuid() }
}

fn state_file_name(session_id: &str, root: ResolvedRoot) -> String {
    format!(
        "claude-resource-{:016x}-{}-{}.state",
        stable_hash(session_id.as_bytes()),
        root.pid,
        root.start_time
    )
}

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn hex_encode(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len().saturating_mul(2));

    for byte in bytes {
        encoded.push(DIGITS[usize::from(byte >> 4)] as char);
        encoded.push(DIGITS[usize::from(byte & 0x0f)] as char);
    }

    encoded
}

fn read_cpu_snapshot(path: &Path) -> Option<CpuSnapshot> {
    let body = read_regular_file(path, MAX_STATE_BYTES, effective_uid())?;
    parse_cpu_snapshot(&body)
}

fn parse_cpu_snapshot(body: &str) -> Option<CpuSnapshot> {
    let mut lines = body.lines();
    if lines.next()? != "1" {
        return None;
    }

    let snapshot = CpuSnapshot {
        session: lines.next()?.to_string(),
        root_pid: lines.next()?.parse().ok()?,
        root_start: lines.next()?.parse().ok()?,
        cpu_ticks: lines.next()?.parse().ok()?,
        uptime_nanos: lines.next()?.parse().ok()?,
    };

    if lines.next().is_some() {
        return None;
    }

    Some(snapshot)
}

fn write_cpu_snapshot(path: &Path, snapshot: &CpuSnapshot) -> Option<()> {
    let parent = path.parent()?;
    let file_name = path.file_name()?.to_str()?;
    let temporary = parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        snapshot.uptime_nanos
    ));
    let body = format!(
        "1\n{}\n{}\n{}\n{}\n{}\n",
        snapshot.session,
        snapshot.root_pid,
        snapshot.root_start,
        snapshot.cpu_ticks,
        snapshot.uptime_nanos
    );
    let mut options = OpenOptions::new();
    options
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(O_NOFOLLOW);
    let mut file = options.open(&temporary).ok()?;

    if file.write_all(body.as_bytes()).is_err() {
        drop(file);
        let _ = fs::remove_file(&temporary);
        return None;
    }
    drop(file);

    if fs::rename(&temporary, path).is_err() {
        let _ = fs::remove_file(&temporary);
        return None;
    }

    Some(())
}

fn read_regular_file(path: &Path, max_bytes: u64, owner: u32) -> Option<String> {
    let mut options = OpenOptions::new();
    options.read(true).custom_flags(O_NOFOLLOW);
    let file = options.open(path).ok()?;
    let metadata = file.metadata().ok()?;

    if !metadata.file_type().is_file() || metadata.uid() != owner || metadata.mode() & 0o077 != 0 {
        return None;
    }

    let mut bytes = Vec::new();
    file.take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > max_bytes {
        return None;
    }

    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::{cpu_delta_percent, parse_cpu_snapshot, parse_uptime_nanos, CpuSnapshot};

    #[test]
    fn computes_cpu_delta_without_capping_multiple_cores() {
        let previous = CpuSnapshot {
            session: "abc".to_string(),
            root_pid: 77,
            root_start: 98765,
            cpu_ticks: 100,
            uptime_nanos: 1_000_000_000,
        };
        let current = CpuSnapshot {
            session: "abc".to_string(),
            root_pid: 77,
            root_start: 98765,
            cpu_ticks: 350,
            uptime_nanos: 2_000_000_000,
        };

        assert_eq!(cpu_delta_percent(&previous, &current, 100), Some(250));
    }

    #[test]
    fn resets_cpu_delta_when_counter_or_clock_moves_back() {
        let previous = CpuSnapshot {
            session: "abc".to_string(),
            root_pid: 77,
            root_start: 98765,
            cpu_ticks: 100,
            uptime_nanos: 2_000_000_000,
        };
        let lower_cpu = CpuSnapshot {
            session: "abc".to_string(),
            root_pid: 77,
            root_start: 98765,
            cpu_ticks: 99,
            uptime_nanos: 3_000_000_000,
        };
        let lower_uptime = CpuSnapshot {
            session: "abc".to_string(),
            root_pid: 77,
            root_start: 98765,
            cpu_ticks: 110,
            uptime_nanos: 1_000_000_000,
        };

        assert_eq!(cpu_delta_percent(&previous, &lower_cpu, 100), None);
        assert_eq!(cpu_delta_percent(&previous, &lower_uptime, 100), None);
    }

    #[test]
    fn parses_uptime_without_floating_point() {
        assert_eq!(parse_uptime_nanos("12.34"), Some(12_340_000_000));
        assert_eq!(parse_uptime_nanos("12.1234567899"), Some(12_123_456_789));
        assert_eq!(parse_uptime_nanos("12.bad"), None);
    }

    #[test]
    fn rejects_partial_or_extra_snapshot_state() {
        let complete = "1\n616263\n77\n98765\n350\n2000000000\n";
        assert_eq!(
            parse_cpu_snapshot(complete),
            Some(CpuSnapshot {
                session: "616263".to_string(),
                root_pid: 77,
                root_start: 98765,
                cpu_ticks: 350,
                uptime_nanos: 2_000_000_000,
            })
        );
        assert_eq!(parse_cpu_snapshot("1\n616263\n77\n"), None);
        assert_eq!(
            parse_cpu_snapshot("1\n616263\n77\n98765\n350\n2000000000\nextra\n"),
            None
        );
    }
}
