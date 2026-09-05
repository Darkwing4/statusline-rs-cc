use std::process::Command;

use serde_json::Value;

fn catalog() -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_statusline"))
        .arg("--schema")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());

    serde_json::from_slice(&output.stdout).unwrap()
}

fn segment_names(catalog: &Value) -> Vec<&str> {
    catalog["segments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|segment| segment["name"].as_str().unwrap())
        .collect()
}

#[test]
fn prints_the_catalogue_with_the_binary_version() {
    let catalog = catalog();

    assert_eq!(catalog["version"], env!("CARGO_PKG_VERSION"));

    for segment in catalog["segments"].as_array().unwrap() {
        let name = segment["name"].as_str().unwrap();

        assert!(!segment["pitch"].as_str().unwrap().is_empty(), "{name}");
        assert!(!segment["fields"].as_array().unwrap().is_empty(), "{name}");
    }
}

#[test]
fn describes_every_segment_the_default_config_uses() {
    let catalog = catalog();
    let names = segment_names(&catalog);

    let used: Vec<&str> = include_str!("../config/default.ron")
        .lines()
        .map(str::trim)
        .filter_map(|line| line.strip_suffix('('))
        .filter(|name| name.chars().next().is_some_and(char::is_uppercase))
        .collect();

    assert!(!used.is_empty());

    for name in used {
        assert!(
            names.contains(&name),
            "{name} is missing from the catalogue"
        );
    }
}
