use serde_json::Value;

use crate::segments::{GitCache, Segment};
use crate::statusline_input;
use crate::types::Color;

pub struct Renderer {
    pub separator: String,
    pub separator_color: Color,
    pub segments: Vec<Box<dyn Segment>>,
}

impl Renderer {
    pub fn render(&self, json: &Value) -> String {
        let cwd = statusline_input::cwd(json).unwrap_or("").to_string();
        let mut git = GitCache::new(cwd);

        let mut main_parts: Vec<String> = Vec::new();
        let mut tail_lines: Vec<String> = Vec::new();

        for segment in &self.segments {
            let Some(rendered) = segment.render(json, &mut git) else {
                continue;
            };
            if rendered.is_empty() {
                continue;
            }
            if segment.standalone() {
                tail_lines.push(rendered);
            } else {
                main_parts.push(rendered);
            }
        }

        let sep = self.separator_color.paint(&self.separator);

        let main_block = match terminal_width() {
            Some(cols) => {
                let max = cols.saturating_sub(4);
                if max == 0 {
                    main_parts.join(&sep)
                } else {
                    wrap_segments(&main_parts, &sep, max)
                }
            }
            None => main_parts.join(&sep),
        };

        let mut lines = vec![main_block];
        lines.extend(tail_lines);
        lines.join("\n")
    }
}

fn wrap_segments(parts: &[String], sep: &str, max: usize) -> String {
    if parts.is_empty() {
        return String::new();
    }

    let sep_w = visible_width(sep);
    let widths: Vec<usize> = parts.iter().map(|p| visible_width(p)).collect();

    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_w = 0usize;

    for (i, part) in parts.iter().enumerate() {
        let pw = widths[i];

        if current.is_empty() {
            current.push_str(part);
            current_w = pw;
            continue;
        }

        let projected = current_w + sep_w + pw;
        if projected > max {
            lines.push(std::mem::take(&mut current));
            current.push_str(part);
            current_w = pw;
        } else {
            current.push_str(sep);
            current.push_str(part);
            current_w = projected;
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines.join("\n")
}

fn visible_width(s: &str) -> usize {
    let mut width = 0usize;
    let bytes = s.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == 0x1b {
            i += 1;
            while i < bytes.len() && !bytes[i].is_ascii_alphabetic() {
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
        } else {
            let ch = s[i..].chars().next().unwrap();
            width += 1;
            i += ch.len_utf8();
        }
    }

    width
}

fn terminal_width() -> Option<usize> {
    if let Ok(s) = std::env::var("COLUMNS") {
        if let Ok(n) = s.trim().parse::<usize>() {
            if n > 0 {
                return Some(n);
            }
        }
    }
    platform_terminal_width()
}

#[cfg(unix)]
#[repr(C)]
struct Winsize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

#[cfg(unix)]
extern "C" {
    fn ioctl(fd: i32, request: std::os::raw::c_ulong, ...) -> i32;
}

#[cfg(unix)]
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
const TIOCGWINSZ: std::os::raw::c_ulong = 0x40087468;

#[cfg(unix)]
#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
)))]
const TIOCGWINSZ: std::os::raw::c_ulong = 0x5413;

#[cfg(unix)]
fn ioctl_winsize_cols(fd: i32) -> Option<usize> {
    let mut ws: Winsize = unsafe { std::mem::zeroed() };
    let res = unsafe { ioctl(fd, TIOCGWINSZ, &mut ws as *mut Winsize) };
    if res != 0 || ws.ws_col == 0 {
        return None;
    }
    Some(ws.ws_col as usize)
}

#[cfg(unix)]
fn platform_terminal_width() -> Option<usize> {
    use std::os::unix::io::AsRawFd;

    if let Ok(tty) = std::fs::File::open("/dev/tty") {
        if let Some(cols) = ioctl_winsize_cols(tty.as_raw_fd()) {
            return Some(cols);
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(cols) = linux_parent_tree_width() {
            return Some(cols);
        }
    }

    None
}

#[cfg(all(unix, target_os = "linux"))]
fn linux_parent_tree_width() -> Option<usize> {
    use std::os::unix::io::AsRawFd;

    fn read_ppid(pid: u32) -> Option<u32> {
        let path = format!("/proc/{}/status", pid);
        let content = std::fs::read_to_string(path).ok()?;
        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("PPid:") {
                return rest.trim().parse().ok();
            }
        }
        None
    }

    let mut pid = std::process::id();
    for _ in 0..32 {
        let parent = match read_ppid(pid) {
            Some(p) if p > 1 => p,
            _ => return None,
        };

        for fd_num in [0u32, 1, 2] {
            let path = format!("/proc/{}/fd/{}", parent, fd_num);
            if let Ok(file) = std::fs::File::open(&path) {
                if let Some(cols) = ioctl_winsize_cols(file.as_raw_fd()) {
                    return Some(cols);
                }
            }
        }

        pid = parent;
    }
    None
}

#[cfg(windows)]
fn platform_terminal_width() -> Option<usize> {
    use std::ffi::c_void;
    use std::ptr;

    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct Coord {
        x: i16,
        y: i16,
    }
    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct SmallRect {
        left: i16,
        top: i16,
        right: i16,
        bottom: i16,
    }
    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct ConsoleScreenBufferInfo {
        size: Coord,
        cursor_position: Coord,
        attributes: u16,
        window: SmallRect,
        maximum_window_size: Coord,
    }

    extern "system" {
        fn GetConsoleScreenBufferInfo(
            handle: *mut c_void,
            info: *mut ConsoleScreenBufferInfo,
        ) -> i32;
        fn CreateFileW(
            file_name: *const u16,
            desired_access: u32,
            share_mode: u32,
            security_attributes: *mut c_void,
            creation_disposition: u32,
            flags_and_attributes: u32,
            template_file: *mut c_void,
        ) -> *mut c_void;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }

    const GENERIC_READ: u32 = 0x80000000;
    const GENERIC_WRITE: u32 = 0x40000000;
    const FILE_SHARE_READ: u32 = 0x00000001;
    const FILE_SHARE_WRITE: u32 = 0x00000002;
    const OPEN_EXISTING: u32 = 3;
    let invalid_handle: *mut c_void = -1isize as *mut c_void;

    let name: Vec<u16> = "CONOUT$\0".encode_utf16().collect();
    let handle = unsafe {
        CreateFileW(
            name.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            ptr::null_mut(),
            OPEN_EXISTING,
            0,
            ptr::null_mut(),
        )
    };

    if handle.is_null() || handle == invalid_handle {
        return None;
    }

    let mut info: ConsoleScreenBufferInfo = ConsoleScreenBufferInfo::default();
    let res = unsafe { GetConsoleScreenBufferInfo(handle, &mut info) };
    unsafe {
        CloseHandle(handle);
    }

    if res == 0 {
        return None;
    }

    let width = info.window.right as i32 - info.window.left as i32 + 1;
    if width <= 0 {
        return None;
    }
    Some(width as usize)
}

#[cfg(not(any(unix, windows)))]
fn platform_terminal_width() -> Option<usize> {
    None
}
