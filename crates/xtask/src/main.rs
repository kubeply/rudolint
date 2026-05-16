use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    match (args.next().as_deref(), args.next().as_deref(), args.next()) {
        (Some("update-oracle"), Some("hadolint"), None) => update_hadolint_oracle(),
        _ => Err("usage: cargo run -p xtask -- update-oracle hadolint".to_string()),
    }
}

fn update_hadolint_oracle() -> Result<(), String> {
    let binary = env::var("RUDOLINT_ORACLE_BIN").unwrap_or_else(|_| "hadolint".to_string());
    let output = Command::new(&binary)
        .arg("--version")
        .output()
        .map_err(|error| format!("failed to run {binary}: {error}"))?;

    if !output.status.success() {
        return Err(format!("{binary} --version exited with {}", output.status));
    }

    let version_output = String::from_utf8(output.stdout)
        .map_err(|error| format!("oracle version output was not UTF-8: {error}"))?
        .trim()
        .to_string();

    let metadata = json!({
        "tool": "hadolint",
        "binary": binary,
        "version_output": version_output,
        "install": {
            "env": "RUDOLINT_ORACLE_BIN",
            "fallback": "hadolint on PATH"
        }
    });

    let output_path = workspace_root().join("fixtures/compat/oracles/hadolint.json");
    let parent = output_path
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", output_path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    fs::write(
        &output_path,
        serde_json::to_string_pretty(&metadata)
            .map_err(|error| format!("failed to serialize oracle metadata: {error}"))?
            + "\n",
    )
    .map_err(|error| format!("failed to write {}: {error}", output_path.display()))?;

    println!("wrote {}", output_path.display());
    Ok(())
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("xtask should live under crates/xtask")
        .to_path_buf()
}
