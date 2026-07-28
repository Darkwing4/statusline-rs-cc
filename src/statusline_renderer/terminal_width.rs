pub(super) fn terminal_width() -> Option<usize> {
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

    let mut pid = std::process::id();
    for _ in 0..32 {
        let parent = match crate::process_stat::read(pid).map(|stat| stat.ppid) {
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
