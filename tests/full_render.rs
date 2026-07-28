use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

#[test]
fn renders_default_statusline_end_to_end() {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/full_render");
    let mut child = Command::new(env!("CARGO_BIN_EXE_statusline"))
        .current_dir(&fixture_dir)
        .env("COLUMNS", "1000")
        .env("HOME", "fixture-home")
        .env("USERPROFILE", "fixture-home")
        .env("PATH", fixture_dir.join("missing-bin"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .take()
        .unwrap()
        .write_all(include_bytes!("fixtures/full_render/input.json"))
        .unwrap();

    let output = child.wait_with_output().unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    assert_eq!(
        output.stdout,
        concat!(
            "\x1b[38;2;180;142;173mOpus\x1b[0m",
            "\x1b[90m \x1b[0m",
            "\x1b[90mhigh\x1b[0m",
            "\x1b[90m \x1b[0m",
            "\x1b[38;2;220;60;60m43%\x1b[0m",
            "\x1b[90m \x1b[0m",
            "\x1b[38;2;149;177;102m5.0h ▇\x1b[0m",
            "\x1b[90m \x1b[0m",
            "\x1b[38;2;200;155;92m7.0d ▄\x1b[0m",
            "\x1b[90m \x1b[0m",
            "\x1b[38;2;95;175;175m~/workspace\x1b[0m",
            "\x1b[90m \x1b[0m",
            "\x1b[91mno git\x1b[0m",
        )
        .as_bytes()
    );
}
