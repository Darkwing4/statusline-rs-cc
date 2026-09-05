use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY_ID: AtomicU64 = AtomicU64::new(0);

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new() -> Self {
        let id = NEXT_DIRECTORY_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "statusline-runtime-config-{}-{}",
            std::process::id(),
            id
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).unwrap();
    }
}

fn write_config(path: &Path, prefix: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        path,
        format!(
            r#"(
                separator: " ",
                separator_color: Named(90),
                segments: [
                    Model(
                        color: Named(32),
                        prefix: "{prefix}",
                    ),
                ],
            )"#
        ),
    )
    .unwrap();
}

fn render(home: &Path, args: &[&Path]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_statusline"));
    command
        .env("HOME", home)
        .env("USERPROFILE", home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(path) = args.first() {
        command.arg("--config").arg(path);
    }

    let mut child = command.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(br#"{"model":{"display_name":"Opus"}}"#)
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn uses_default_runtime_config_when_present() {
    let directory = TestDirectory::new();
    let config_path = directory
        .path
        .join(".claude")
        .join("statusline")
        .join("config.ron");
    write_config(&config_path, "default ");

    let output = render(&directory.path, &[]);

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, b"\x1b[32mdefault Opus\x1b[0m");
}

#[test]
fn uses_explicit_runtime_config() {
    let directory = TestDirectory::new();
    let config_path = directory.path.join("explicit.ron");
    write_config(&config_path, "explicit ");

    let output = render(&directory.path, &[&config_path]);

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, b"\x1b[32mexplicit Opus\x1b[0m");
}

#[test]
fn checks_config_without_reading_statusline_input() {
    let directory = TestDirectory::new();
    let config_path = directory.path.join("valid.ron");
    write_config(&config_path, "");

    let output = Command::new(env!("CARGO_BIN_EXE_statusline"))
        .arg("--check-config")
        .arg(&config_path)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[test]
fn reports_invalid_config_path_and_parse_error() {
    let directory = TestDirectory::new();
    let config_path = directory.path.join("invalid.ron");
    fs::write(&config_path, "(").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_statusline"))
        .arg("--check-config")
        .arg(&config_path)
        .output()
        .unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(stderr.contains("statusline: invalid config"));
    assert!(stderr.contains(config_path.to_str().unwrap()));
}

#[test]
fn reports_missing_explicit_config() {
    let directory = TestDirectory::new();
    let config_path = directory.path.join("missing.ron");

    let output = Command::new(env!("CARGO_BIN_EXE_statusline"))
        .arg("--check-config")
        .arg(&config_path)
        .output()
        .unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(stderr.contains("statusline: cannot read config"));
    assert!(stderr.contains(config_path.to_str().unwrap()));
}
