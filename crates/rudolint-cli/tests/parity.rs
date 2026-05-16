use std::{env, process::Command};

use rudolint_test::{fixture_path, normalized_json};

#[test]
#[ignore = "requires a pinned external oracle binary"]
fn parity_oracle_is_available() {
    let oracle = env::var("RUDOLINT_ORACLE_BIN").unwrap_or_else(|_| "hadolint".to_string());
    let output = Command::new(oracle)
        .arg("--version")
        .output()
        .expect("RUDOLINT_ORACLE_BIN must point to an installed oracle binary");
    assert!(output.status.success());
}

#[test]
#[ignore = "requires RUDOLINT_ORACLE_BIN and intentionally runs an external oracle"]
fn normalizes_hadolint_json_oracle_output() {
    let oracle = env::var("RUDOLINT_ORACLE_BIN")
        .expect("set RUDOLINT_ORACLE_BIN to run compatibility oracle tests");
    let fixture = fixture_path("compat/RDL3007.no-latest-tag/Dockerfile");

    let output = Command::new(oracle)
        .args(["-f", "json"])
        .arg(&fixture)
        .output()
        .expect("oracle should run");

    assert!(
        !output.stdout.is_empty(),
        "oracle should emit JSON diagnostics to stdout"
    );

    let stdout = String::from_utf8(output.stdout).expect("oracle stdout should be UTF-8");
    let normalized = normalized_json(&stdout);
    insta::assert_json_snapshot!("hadolint_rdl3007_oracle", normalized);
}
