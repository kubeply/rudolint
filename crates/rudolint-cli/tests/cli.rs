use predicates::prelude::PredicateBooleanExt;
use rudolint_test::{normalize_path_prefix, normalized_json, rudolint_cmd};
use tempfile::TempDir;

#[test]
fn emits_json_findings_for_stdin() {
    let output = rudolint_cmd()
        .args(["check", "--format", "json", "--failure-threshold", "error"])
        .write_stdin("FROM alpine:latest\nWORKDIR app\n")
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_json_snapshot!("stdin_json_findings", normalized_json(&output));
}

#[test]
fn emits_sarif_findings_for_stdin() {
    let output = rudolint_cmd()
        .args(["check", "--format", "sarif", "--failure-threshold", "error"])
        .write_stdin("FROM alpine:latest\nWORKDIR app\n")
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_json_snapshot!("stdin_sarif_findings", normalized_json(&output));
}

#[test]
fn emits_human_findings_for_stdin() {
    let output = rudolint_cmd()
        .args(["check", "--failure-threshold", "error"])
        .write_stdin("FROM alpine:latest\nWORKDIR app\n")
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("stdin_human_findings", output);
}

#[test]
fn checks_explicit_dockerfile_path() {
    let temp = TempDir::new().expect("temp dir should be created");
    let dockerfile = temp.path().join("Dockerfile");
    std::fs::write(&dockerfile, "FROM alpine:latest\n").expect("fixture should be written");

    let output = rudolint_cmd()
        .args(["check", "--format", "json", "--failure-threshold", "error"])
        .arg(&dockerfile)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let mut output = normalized_json(&output);
    normalize_path_prefix(&mut output, temp.path(), "$TEMP");
    insta::assert_json_snapshot!("explicit_path_json_findings", output);
}

#[test]
fn discovers_dockerfiles_in_directory() {
    let temp = TempDir::new().expect("temp dir should be created");
    let nested = temp.path().join("service");
    std::fs::create_dir(&nested).expect("nested dir should be created");
    std::fs::write(temp.path().join("Dockerfile"), "FROM alpine:latest\n")
        .expect("root Dockerfile should be written");
    std::fs::write(nested.join("Dockerfile.api"), "FROM busybox\nWORKDIR app\n")
        .expect("nested Dockerfile should be written");

    let output = rudolint_cmd()
        .args(["check", "--format", "json", "--failure-threshold", "error"])
        .arg(temp.path())
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let mut output = normalized_json(&output);
    normalize_path_prefix(&mut output, temp.path(), "$TEMP");
    insta::assert_json_snapshot!("directory_discovery_json_findings", output);
}

#[test]
fn explicit_config_can_ignore_rule() {
    let temp = TempDir::new().expect("temp dir should be created");
    let dockerfile = temp.path().join("Dockerfile");
    let config = temp.path().join(".rudolint.yaml");
    std::fs::write(&dockerfile, "FROM alpine:latest\nWORKDIR app\n")
        .expect("fixture should be written");
    std::fs::write(&config, "ignore:\n  - RDL3000\n").expect("config should be written");

    let output = rudolint_cmd()
        .args([
            "check",
            "--format",
            "json",
            "--failure-threshold",
            "error",
            "--config",
        ])
        .arg(&config)
        .arg(&dockerfile)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let mut output = normalized_json(&output);
    normalize_path_prefix(&mut output, temp.path(), "$TEMP");
    insta::assert_json_snapshot!("config_ignore_json_findings", output);
}

#[test]
fn explicit_config_can_override_severity() {
    let temp = TempDir::new().expect("temp dir should be created");
    let dockerfile = temp.path().join("Dockerfile");
    let config = temp.path().join(".rudolint.yaml");
    std::fs::write(&dockerfile, "FROM alpine:latest\n").expect("fixture should be written");
    std::fs::write(&config, "severity:\n  RDL3007: error\n").expect("config should be written");

    let output = rudolint_cmd()
        .args([
            "check",
            "--format",
            "json",
            "--failure-threshold",
            "error",
            "--config",
        ])
        .arg(&config)
        .arg(&dockerfile)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let mut output = normalized_json(&output);
    normalize_path_prefix(&mut output, temp.path(), "$TEMP");
    insta::assert_json_snapshot!("config_severity_json_findings", output);
}

#[test]
fn discovers_nearest_dot_config_from_input_directory() {
    let temp = TempDir::new().expect("temp dir should be created");
    let nested = temp.path().join("service");
    std::fs::create_dir(&nested).expect("nested dir should be created");
    std::fs::write(temp.path().join(".rudolint.yaml"), "ignore:\n  - RDL3000\n")
        .expect("config should be written");
    std::fs::write(
        nested.join("Dockerfile"),
        "FROM alpine:latest\nWORKDIR app\n",
    )
    .expect("fixture should be written");

    let output = rudolint_cmd()
        .args(["check", "--format", "json", "--failure-threshold", "error"])
        .arg(temp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let mut output = normalized_json(&output);
    normalize_path_prefix(&mut output, temp.path(), "$TEMP");
    insta::assert_json_snapshot!("discovered_config_json_findings", output);
}

#[test]
fn explicit_config_takes_precedence_over_discovered_config() {
    let temp = TempDir::new().expect("temp dir should be created");
    let dockerfile = temp.path().join("Dockerfile");
    let explicit = temp.path().join("explicit.yaml");
    std::fs::write(temp.path().join(".rudolint.yaml"), "ignore:\n  - RDL3000\n")
        .expect("discovered config should be written");
    std::fs::write(&explicit, "ignore:\n  - RDL3007\n").expect("explicit config should be written");
    std::fs::write(&dockerfile, "FROM alpine:latest\nWORKDIR app\n")
        .expect("fixture should be written");

    let output = rudolint_cmd()
        .args([
            "check",
            "--format",
            "json",
            "--failure-threshold",
            "error",
            "--config",
        ])
        .arg(&explicit)
        .arg(&dockerfile)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let mut output = normalized_json(&output);
    normalize_path_prefix(&mut output, temp.path(), "$TEMP");
    insta::assert_json_snapshot!("explicit_config_precedence_json_findings", output);
}

#[test]
fn no_config_disables_discovery() {
    let temp = TempDir::new().expect("temp dir should be created");
    let dockerfile = temp.path().join("Dockerfile");
    std::fs::write(temp.path().join(".rudolint.yaml"), "ignore:\n  - RDL3000\n")
        .expect("config should be written");
    std::fs::write(&dockerfile, "FROM alpine:latest\nWORKDIR app\n")
        .expect("fixture should be written");

    rudolint_cmd()
        .args(["check", "--no-config", "--failure-threshold", "error"])
        .arg(&dockerfile)
        .assert()
        .code(1);
}

#[test]
fn config_parse_errors_include_line_and_column() {
    let temp = TempDir::new().expect("temp dir should be created");
    let dockerfile = temp.path().join("Dockerfile");
    let config = temp.path().join(".rudolint.yaml");
    std::fs::write(&dockerfile, "FROM alpine:3.20\n").expect("fixture should be written");
    std::fs::write(&config, "ignore: [RDL3000\n").expect("config should be written");

    rudolint_cmd()
        .args(["check"])
        .arg(&dockerfile)
        .assert()
        .code(2)
        .stderr(predicates::str::contains("line 1").and(predicates::str::contains("column")));
}

#[test]
fn clean_input_exits_successfully() {
    rudolint_cmd()
        .args(["check", "--failure-threshold", "warning"])
        .write_stdin("FROM alpine:3.20\nWORKDIR /app\nUSER app\n")
        .assert()
        .success();
}

#[test]
fn findings_exit_with_code_one() {
    rudolint_cmd()
        .args(["check", "--failure-threshold", "error"])
        .write_stdin("FROM alpine:latest\nWORKDIR app\n")
        .assert()
        .code(1);
}

#[test]
fn exit_zero_succeeds_with_findings() {
    rudolint_cmd()
        .args(["check", "--failure-threshold", "error", "--exit-zero"])
        .write_stdin("FROM alpine:latest\nWORKDIR app\n")
        .assert()
        .success();
}

#[test]
fn no_config_flag_is_accepted() {
    rudolint_cmd()
        .args(["check", "--no-config", "--failure-threshold", "error"])
        .write_stdin("FROM alpine:latest\n")
        .assert()
        .success();
}

#[test]
fn config_and_no_config_conflict() {
    let temp = TempDir::new().expect("temp dir should be created");
    let config = temp.path().join(".rudolint.yaml");
    std::fs::write(&config, "ignore:\n  - RDL3000\n").expect("config should be written");

    rudolint_cmd()
        .args(["check", "--no-config", "--config"])
        .arg(&config)
        .write_stdin("FROM alpine:latest\n")
        .assert()
        .code(2);
}

#[test]
fn stdin_filename_changes_display_path() {
    let output = rudolint_cmd()
        .args([
            "check",
            "--format",
            "json",
            "--failure-threshold",
            "error",
            "--stdin-filename",
            "Dockerfile.custom",
        ])
        .write_stdin("FROM alpine:latest\nWORKDIR app\n")
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_json_snapshot!("stdin_filename_json_findings", normalized_json(&output));
}

#[test]
fn quiet_suppresses_output_but_preserves_failure() {
    rudolint_cmd()
        .args(["check", "--quiet", "--failure-threshold", "error"])
        .write_stdin("FROM alpine:latest\nWORKDIR app\n")
        .assert()
        .code(1)
        .stdout("");
}

#[test]
fn verbose_emits_summary_to_stderr() {
    rudolint_cmd()
        .args(["check", "--verbose", "--failure-threshold", "error"])
        .write_stdin("FROM alpine:latest\nWORKDIR app\n")
        .assert()
        .code(1)
        .stderr(predicates::str::contains(
            "checked 1 Dockerfile(s), emitted 2 finding(s)",
        ));
}

#[test]
fn show_source_adds_human_source_excerpts() {
    let output = rudolint_cmd()
        .args(["check", "--show-source", "--failure-threshold", "error"])
        .write_stdin("FROM alpine:latest\nWORKDIR app\n")
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("stdin_human_show_source", output);
}

#[test]
fn fix_dry_run_reports_without_writing_files() {
    let temp = tempfile::TempDir::new().expect("temp dir should be created");
    let dockerfile = temp.path().join("Dockerfile");
    let original = "FROM alpine:latest\nWORKDIR app\n";
    std::fs::write(&dockerfile, original).expect("Dockerfile should be written");

    let output = rudolint_cmd()
        .args(["check", "--fix", "--dry-run"])
        .arg(&dockerfile)
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();

    assert_eq!(
        std::fs::read_to_string(&dockerfile).expect("Dockerfile should still exist"),
        original
    );
    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let temp_path = temp
        .path()
        .to_str()
        .expect("temp path should be valid UTF-8");
    let output = output.replace(temp_path, "$TEMP");
    insta::assert_snapshot!("fix_dry_run_human", output);
}

#[test]
fn fix_dry_run_renders_safe_fix_preview() {
    let output = rudolint_cmd()
        .args(["check", "--fix", "--dry-run"])
        .write_stdin("FROM alpine:3.20\nRUN --mount=type=cache,target=/tmp/cache echo ok\n")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("fix_dry_run_safe_preview", output);
}

#[test]
fn fix_write_mode_reports_without_writing_when_no_fixes_exist() {
    let temp = tempfile::TempDir::new().expect("temp dir should be created");
    let dockerfile = temp.path().join("Dockerfile");
    let original = "FROM alpine:latest\nWORKDIR app\n";
    std::fs::write(&dockerfile, original).expect("Dockerfile should be written");

    let output = rudolint_cmd()
        .args(["check", "--fix"])
        .arg(&dockerfile)
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();

    assert_eq!(
        std::fs::read_to_string(&dockerfile).expect("Dockerfile should still exist"),
        original
    );
    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let temp_path = temp
        .path()
        .to_str()
        .expect("temp path should be valid UTF-8");
    let output = output.replace(temp_path, "$TEMP");
    insta::assert_snapshot!("fix_write_mode_human", output);
}

#[test]
fn fix_write_mode_applies_safe_fix_idempotently() {
    let temp = tempfile::TempDir::new().expect("temp dir should be created");
    let dockerfile = temp.path().join("Dockerfile");
    std::fs::write(
        &dockerfile,
        "FROM alpine:3.20\nRUN --mount=type=cache,target=/tmp/cache echo ok\n",
    )
    .expect("Dockerfile should be written");

    rudolint_cmd()
        .args(["check", "--fix"])
        .arg(&dockerfile)
        .assert()
        .success();

    let once = std::fs::read_to_string(&dockerfile).expect("Dockerfile should exist");
    insta::assert_snapshot!("fix_write_mode_applied_file", once);

    rudolint_cmd()
        .args(["check", "--fix"])
        .arg(&dockerfile)
        .assert()
        .success();

    let twice = std::fs::read_to_string(&dockerfile).expect("Dockerfile should exist");
    assert_eq!(twice, once);
}

#[test]
fn fix_write_mode_replaces_maintainer_when_deterministic() {
    let temp = tempfile::TempDir::new().expect("temp dir should be created");
    let dockerfile = temp.path().join("Dockerfile");
    std::fs::write(
        &dockerfile,
        "FROM alpine:3.20\nMAINTAINER ops@example.com\n",
    )
    .expect("Dockerfile should be written");

    rudolint_cmd()
        .args(["check", "--fix"])
        .arg(&dockerfile)
        .assert()
        .code(1);

    let output = std::fs::read_to_string(&dockerfile).expect("Dockerfile should exist");
    insta::assert_snapshot!("fix_write_mode_maintainer_file", output);
}

#[test]
fn fix_write_mode_leaves_nonrewritable_maintainer_values() {
    let temp = tempfile::TempDir::new().expect("temp dir should be created");
    let dockerfile = temp.path().join("Dockerfile");
    let original = "FROM alpine:3.20\nMAINTAINER \"bad@example.com\nMAINTAINER bad\\user@example.com\nMAINTAINER \n";
    std::fs::write(&dockerfile, original).expect("Dockerfile should be written");

    rudolint_cmd()
        .args(["check", "--fix"])
        .arg(&dockerfile)
        .assert()
        .code(1);

    let output = std::fs::read_to_string(&dockerfile).expect("Dockerfile should exist");
    assert_eq!(output, original);
    insta::assert_snapshot!("fix_write_mode_maintainer_nonrewritable_file", output);
}

#[test]
fn fix_dry_run_renders_manual_json_entrypoint_suggestion() {
    let output = rudolint_cmd()
        .args(["check", "--fix", "--dry-run"])
        .write_stdin("FROM alpine:3.20\nCMD echo hello\n")
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_snapshot!("fix_dry_run_manual_json_entrypoint", output);
}

#[test]
fn fix_write_mode_does_not_apply_manual_json_entrypoint_suggestion() {
    let temp = tempfile::TempDir::new().expect("temp dir should be created");
    let dockerfile = temp.path().join("Dockerfile");
    let original = "FROM alpine:3.20\nCMD echo hello\n";
    std::fs::write(&dockerfile, original).expect("Dockerfile should be written");

    rudolint_cmd()
        .args(["check", "--fix"])
        .arg(&dockerfile)
        .assert()
        .code(1);

    assert_eq!(
        std::fs::read_to_string(&dockerfile).expect("Dockerfile should exist"),
        original
    );
}

#[test]
fn fix_dry_run_json_includes_fix_envelope() {
    let output = rudolint_cmd()
        .args(["check", "--fix", "--dry-run", "--format", "json"])
        .write_stdin("FROM alpine:3.20\nRUN --mount=type=cache,target=/tmp/cache echo ok\n")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_json_snapshot!("fix_dry_run_json", normalized_json(&output));
}

#[test]
fn emits_rules_json() {
    let output = rudolint_cmd()
        .args(["rules", "--implemented", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_json_snapshot!("rules_implemented_json", normalized_json(&output));
}

#[test]
fn emits_rules_implemented_text() {
    rudolint_cmd()
        .args(["rules", "--implemented"])
        .assert()
        .success()
        .stdout(predicates::str::contains("RDK1000"))
        .stdout(predicates::str::contains("implemented"))
        .stdout(predicates::str::contains("RDL3004").not())
        .stdout(predicates::str::contains("RSC1000").not());
}

#[test]
fn emits_full_rules_json() {
    let output = rudolint_cmd()
        .args(["rules", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_json_snapshot!("rules_full_json", normalized_json(&output));
}

#[test]
fn explains_rule() {
    rudolint_cmd()
        .args(["explain", "RDL3007"])
        .assert()
        .success()
        .stdout(predicates::str::contains("RDL3007"))
        .stdout(predicates::str::contains("reject latest base image tags"));
}

#[test]
fn unknown_explain_rule_exits_with_code_two() {
    rudolint_cmd().args(["explain", "RDL9999"]).assert().code(2);
}

#[test]
fn missing_input_exits_with_code_two() {
    let temp = TempDir::new().expect("temp dir should be created");
    let missing = temp.path().join("missing.Dockerfile");

    rudolint_cmd()
        .args(["check"])
        .arg(&missing)
        .assert()
        .code(2);
}

#[test]
fn emits_json_version() {
    let output = rudolint_cmd()
        .args(["--version", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    insta::assert_json_snapshot!("version_json", normalized_json(&output));
}

#[test]
fn emits_plain_version() {
    rudolint_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::contains("rudolint 0.1.0"));
}
