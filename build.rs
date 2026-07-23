use std::env;
use std::fs;
use std::path::PathBuf;

#[allow(dead_code)]
#[path = "src/config_schema.rs"]
mod config_schema;

const ENV_VAR: &str = "STATUSLINE_CONFIG";
const DEFAULT_REL_PATH: &str = "config/default.ron";
const EMBED_FILENAME: &str = "embedded_config.ron";

fn main() {
    println!("cargo:rerun-if-env-changed={}", ENV_VAR);

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));

    let configured = env::var(ENV_VAR).ok().filter(|s| !s.is_empty());

    let source = match configured {
        Some(p) => {
            let pb = PathBuf::from(&p);
            if pb.is_absolute() { pb } else { manifest_dir.join(p) }
        }
        None => manifest_dir.join(DEFAULT_REL_PATH),
    };

    println!("cargo:rerun-if-changed={}", source.display());

    let body = fs::read_to_string(&source).unwrap_or_else(|e| {
        panic!(
            "statusline build: cannot read config '{}': {}",
            source.display(),
            e
        )
    });

    config_schema::parse(&body).unwrap_or_else(|e| {
        panic!(
            "statusline build: invalid config '{}': {}",
            source.display(),
            e
        )
    });

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let dst = out_dir.join(EMBED_FILENAME);
    fs::write(&dst, body).expect("write embedded config");
}
