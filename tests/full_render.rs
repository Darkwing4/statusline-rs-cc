use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn render(input: &[u8], columns: Option<&str>) -> Output {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/full_render");
    let mut command = Command::new(env!("CARGO_BIN_EXE_statusline"));
    command
        .current_dir(&fixture_dir)
        .env_remove("COLUMNS")
        .env("HOME", "fixture-home")
        .env("USERPROFILE", "fixture-home")
        .env("PATH", fixture_dir.join("missing-bin"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(columns) = columns {
        command.env("COLUMNS", columns);
    }

    let mut child = command.spawn().unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();

    child.wait_with_output().unwrap()
}

#[test]
fn renders_default_statusline_end_to_end() {
    let output = render(
        include_bytes!("fixtures/full_render/input.json"),
        Some("1000"),
    );

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
            "\x1b[38;2;149;177;102m?h ▇\x1b[0m",
            "\x1b[90m \x1b[0m",
            "\x1b[38;2;200;155;92m?d ▄\x1b[0m",
            "\x1b[90m \x1b[0m",
            "\x1b[38;2;95;175;175m~/workspace\x1b[0m",
            "\x1b[90m \x1b[0m",
            "\x1b[91mno git\x1b[0m",
        )
        .as_bytes()
    );
}

#[test]
fn treats_invalid_columns_like_absent_columns() {
    let input = include_bytes!("fixtures/full_render/input.json");
    let without_columns = render(input, None);

    for columns in ["abc", "0", " ", "-5"] {
        let output = render(input, Some(columns));

        assert!(output.status.success(), "COLUMNS={columns:?}");
        assert!(output.stderr.is_empty(), "COLUMNS={columns:?}");
        assert_eq!(output.stdout, without_columns.stdout, "COLUMNS={columns:?}");
    }
}

#[test]
fn exits_quietly_on_invalid_input() {
    for input in [&b""[..], b"not json", b"[1, 2"] {
        let output = render(input, Some("1000"));

        assert!(output.status.success(), "{input:?}");
        assert!(output.stdout.is_empty(), "{input:?}");
        assert!(output.stderr.is_empty(), "{input:?}");
    }
}
