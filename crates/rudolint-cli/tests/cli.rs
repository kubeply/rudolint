use std::sync::OnceLock;

use predicates::prelude::PredicateBooleanExt;
use rudolint_test::{normalize_path_prefix, normalized_json, rudolint_cmd};
use serde_json::Value;
use tempfile::TempDir;

fn findings_schema() -> &'static jsonschema::Validator {
    static SCHEMA: OnceLock<jsonschema::Validator> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        let schema = serde_json::from_str(include_str!(
            "../../../schemas/rudolint-findings-v1.schema.json"
        ))
        .expect("findings schema should be valid JSON");
        jsonschema::validator_for(&schema).expect("findings schema should compile")
    })
}

fn assert_matches_findings_schema(output: &Value) {
    let errors = findings_schema()
        .iter_errors(output)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect::<Vec<_>>();
    assert!(
        errors.is_empty(),
        "JSON findings output should match the v1 schema:\n{}",
        errors.join("\n")
    );
}

fn normalized_findings_json(raw: &str) -> Value {
    let output = normalized_json(raw);
    assert_matches_findings_schema(&output);
    output
}

fn assert_no_internal_finding_fields(output: &Value) {
    let findings = output
        .get("findings")
        .and_then(Value::as_array)
        .expect("findings output should contain a findings array");
    assert!(
        !findings.is_empty(),
        "internal field regression needs at least one finding"
    );

    let internal_fields = [
        "line",
        "column",
        "end_line",
        "end_column",
        "source",
        "source_excerpt",
        "sourceExcerpt",
        "raw",
    ];
    for finding in findings {
        let finding = finding
            .as_object()
            .expect("each finding should serialize as an object");
        for field in internal_fields {
            assert!(
                !finding.contains_key(field),
                "finding JSON should not expose internal-only field `{field}`"
            );
        }
    }
}

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
    insta::assert_json_snapshot!("stdin_json_findings", normalized_findings_json(&output));
}

#[test]
fn json_findings_do_not_emit_internal_only_fields() {
    let output = rudolint_cmd()
        .args([
            "check",
            "--format",
            "json",
            "--show-source",
            "--failure-threshold",
            "error",
        ])
        .write_stdin("FROM alpine:latest\nWORKDIR app\n")
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let output = normalized_findings_json(&output);
    assert_no_internal_finding_fields(&output);
}

#[test]
fn copy_heredoc_operands_do_not_emit_copy_operand_findings() {
    let temp = TempDir::new().expect("temp dir should be created");
    let dockerfile = temp.path().join("Dockerfile");
    std::fs::write(
        &dockerfile,
        r#"FROM alpine:3.20
COPY --from=builder \
  /out/app /app
COPY <<"SCRIPT" /usr/local/bin/generated
#!/usr/bin/env sh
tar -xf app.tar.gz
echo "$UNQUOTED"
SCRIPT
COPY --chmod=755 \
  <<'SCRIPT' /usr/local/bin/continued
#!/usr/bin/env sh
echo continued
SCRIPT
COPY <<FIRST \
  <<SECOND /etc/continued-generated/
first continued body
FIRST
second continued body
SECOND
USER 1000
"#,
    )
    .expect("fixture should be written");

    let assert = rudolint_cmd()
        .args(["check", "--format", "json", "--exit-zero"])
        .arg(&dockerfile)
        .assert()
        .success();
    let output = assert.get_output().stdout.clone();
    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let output = normalized_json(&output);
    let findings = output
        .get("findings")
        .and_then(Value::as_array)
        .expect("findings should be an array");
    let blocked_codes = findings
        .iter()
        .filter_map(|finding| finding.get("code").and_then(Value::as_str))
        .filter(|code| matches!(*code, "DL3010" | "DL3021" | "DL3045"))
        .collect::<Vec<_>>();

    assert!(
        blocked_codes.is_empty(),
        "COPY heredoc and continuation operands should not emit copy operand findings: {blocked_codes:?}"
    );
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
    assert_matches_findings_schema(&output);
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
    std::fs::write(nested.join("Dockerfile_ubuntu_24"), "FROM debian:latest\n")
        .expect("underscore Dockerfile should be written");

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
    assert_matches_findings_schema(&output);
    insta::assert_json_snapshot!("directory_discovery_json_findings", output);
}

#[test]
fn emits_json_findings_for_multiple_files() {
    let temp = TempDir::new().expect("temp dir should be created");
    std::fs::write(temp.path().join("Dockerfile"), "FROM alpine:latest\n")
        .expect("root Dockerfile should be written");
    std::fs::write(
        temp.path().join("Dockerfile.api"),
        "FROM busybox\nWORKDIR app\n",
    )
    .expect("second Dockerfile should be written");

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
    assert_matches_findings_schema(&output);
    insta::assert_json_snapshot!("multi_file_json_findings", output);
}

#[test]
fn emits_sarif_findings_for_multiple_files() {
    let temp = TempDir::new().expect("temp dir should be created");
    std::fs::write(temp.path().join("Dockerfile"), "FROM alpine:latest\n")
        .expect("root Dockerfile should be written");
    std::fs::write(
        temp.path().join("Dockerfile.api"),
        "FROM busybox\nWORKDIR app\n",
    )
    .expect("second Dockerfile should be written");

    let output = rudolint_cmd()
        .args(["check", "--format", "sarif", "--failure-threshold", "error"])
        .arg(temp.path())
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let mut output = normalized_json(&output);
    normalize_path_prefix(&mut output, temp.path(), "$TEMP");
    insta::assert_json_snapshot!("multi_file_sarif_findings", output);
}

#[test]
fn emits_sarif_source_spans_and_github_required_fields() {
    let output = rudolint_cmd()
        .args(["check", "--format", "sarif", "--failure-threshold", "error"])
        .write_stdin("FROM alpine:latest\nWORKDIR app\n")
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: Value = serde_json::from_str(&output).expect("SARIF should be JSON");
    assert_eq!(value["version"], "2.1.0");
    assert_eq!(value["runs"][0]["tool"]["driver"]["name"], "rudolint");
    assert!(
        value["runs"][0]["tool"]["driver"]["rules"]
            .as_array()
            .is_some_and(|rules| !rules.is_empty())
    );
    assert!(
        value["runs"][0]["results"]
            .as_array()
            .expect("SARIF results should be an array")
            .iter()
            .all(|result| {
                let location = &result["locations"][0]["physicalLocation"];
                location["artifactLocation"]["uri"].is_string()
                    && location["region"]["startLine"].is_number()
                    && location["region"]["startColumn"].is_number()
            })
    );

    insta::assert_json_snapshot!("stdin_sarif_source_spans", normalized_json(&output));
}

#[test]
fn emits_human_findings_grouped_by_file() {
    let temp = TempDir::new().expect("temp dir should be created");
    std::fs::write(temp.path().join("Dockerfile"), "FROM alpine:latest\n")
        .expect("root Dockerfile should be written");
    std::fs::write(
        temp.path().join("Dockerfile.api"),
        "FROM busybox\nWORKDIR app\n",
    )
    .expect("second Dockerfile should be written");

    let output = rudolint_cmd()
        .args([
            "check",
            "--group-by",
            "file",
            "--failure-threshold",
            "error",
        ])
        .arg(temp.path())
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let temp_path = temp.path().to_str().expect("temp path should be UTF-8");
    insta::assert_snapshot!(
        "multi_file_human_grouped",
        output.replace(temp_path, "$TEMP")
    );
}

#[test]
fn emits_human_findings_grouped_by_rule_by_default() {
    let temp = TempDir::new().expect("temp dir should be created");
    std::fs::write(temp.path().join("Dockerfile"), "FROM alpine:latest\n")
        .expect("root Dockerfile should be written");
    std::fs::write(
        temp.path().join("Dockerfile.api"),
        "FROM busybox\nWORKDIR app\n",
    )
    .expect("second Dockerfile should be written");

    let output = rudolint_cmd()
        .args(["check", "--failure-threshold", "error"])
        .arg(temp.path())
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let temp_path = temp.path().to_str().expect("temp path should be UTF-8");
    insta::assert_snapshot!(
        "multi_file_human_grouped_by_rule",
        output.replace(temp_path, "$TEMP")
    );
}

#[test]
fn human_rule_grouping_respects_max_examples_per_group() {
    let temp = TempDir::new().expect("temp dir should be created");
    for name in ["Dockerfile.a", "Dockerfile.b", "Dockerfile.c"] {
        std::fs::write(temp.path().join(name), "FROM alpine:latest\n")
            .expect("Dockerfile should be written");
    }

    let output = rudolint_cmd()
        .args([
            "check",
            "--max-examples-per-group",
            "2",
            "--failure-threshold",
            "error",
        ])
        .arg(temp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let temp_path = temp.path().to_str().expect("temp path should be UTF-8");
    insta::assert_snapshot!(
        "multi_file_human_rule_group_example_limit",
        output.replace(temp_path, "$TEMP")
    );
}

#[test]
fn explicit_config_can_ignore_rule() {
    let temp = TempDir::new().expect("temp dir should be created");
    let dockerfile = temp.path().join("Dockerfile");
    let config = temp.path().join(".rudolint.yaml");
    std::fs::write(&dockerfile, "FROM alpine:latest\nWORKDIR app\n")
        .expect("fixture should be written");
    std::fs::write(&config, "ignore:\n  - DL3000\n").expect("config should be written");

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
    assert_matches_findings_schema(&output);
    insta::assert_json_snapshot!("config_ignore_json_findings", output);
}

#[test]
fn explicit_config_can_select_rule_prefix() {
    let temp = TempDir::new().expect("temp dir should be created");
    let dockerfile = temp.path().join("Dockerfile");
    let config = temp.path().join(".rudolint.yaml");
    std::fs::write(&dockerfile, "FROM alpine:latest\nWORKDIR app\n")
        .expect("fixture should be written");
    std::fs::write(&config, "select:\n  - DL3000\n").expect("config should be written");

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
        .code(1)
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let mut output = normalized_json(&output);
    normalize_path_prefix(&mut output, temp.path(), "$TEMP");
    assert_matches_findings_schema(&output);
    insta::assert_json_snapshot!("config_select_json_findings", output);
}

#[test]
fn config_per_file_ignores_match_paths_relative_to_config() {
    let temp = TempDir::new().expect("temp dir should be created");
    let service = temp.path().join("service");
    std::fs::create_dir(&service).expect("service dir should be created");
    let dockerfile = service.join("Dockerfile");
    let config = temp.path().join(".rudolint.yaml");
    std::fs::write(&dockerfile, "FROM alpine:latest\nWORKDIR app\n")
        .expect("fixture should be written");
    std::fs::write(&config, "per-file-ignores:\n  service/**:\n    - DL3000\n")
        .expect("config should be written");

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
    assert_matches_findings_schema(&output);
    insta::assert_json_snapshot!("config_per_file_ignore_json_findings", output);
}

#[test]
fn explicit_config_can_override_severity() {
    let temp = TempDir::new().expect("temp dir should be created");
    let dockerfile = temp.path().join("Dockerfile");
    let config = temp.path().join(".rudolint.yaml");
    std::fs::write(&dockerfile, "FROM alpine:latest\n").expect("fixture should be written");
    std::fs::write(&config, "severity:\n  DL3007: error\n").expect("config should be written");

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
    assert_matches_findings_schema(&output);
    insta::assert_json_snapshot!("config_severity_json_findings", output);
}

#[test]
fn discovers_nearest_dot_config_from_input_directory() {
    let temp = TempDir::new().expect("temp dir should be created");
    let nested = temp.path().join("service");
    std::fs::create_dir(&nested).expect("nested dir should be created");
    std::fs::write(temp.path().join(".rudolint.yaml"), "ignore:\n  - DL3000\n")
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
    assert_matches_findings_schema(&output);
    insta::assert_json_snapshot!("discovered_config_json_findings", output);
}

#[test]
fn explicit_config_takes_precedence_over_discovered_config() {
    let temp = TempDir::new().expect("temp dir should be created");
    let dockerfile = temp.path().join("Dockerfile");
    let explicit = temp.path().join("explicit.yaml");
    std::fs::write(temp.path().join(".rudolint.yaml"), "ignore:\n  - DL3000\n")
        .expect("discovered config should be written");
    std::fs::write(&explicit, "ignore:\n  - DL3007\n").expect("explicit config should be written");
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
    assert_matches_findings_schema(&output);
    insta::assert_json_snapshot!("explicit_config_precedence_json_findings", output);
}

#[test]
fn no_config_disables_discovery() {
    let temp = TempDir::new().expect("temp dir should be created");
    let dockerfile = temp.path().join("Dockerfile");
    std::fs::write(temp.path().join(".rudolint.yaml"), "ignore:\n  - DL3000\n")
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
    std::fs::write(&config, "ignore: [DL3000\n").expect("config should be written");

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
    std::fs::write(&config, "ignore:\n  - DL3000\n").expect("config should be written");

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
    insta::assert_json_snapshot!(
        "stdin_filename_json_findings",
        normalized_findings_json(&output)
    );
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
fn fix_write_mode_migrates_hadolint_inline_ignores() {
    let temp = tempfile::TempDir::new().expect("temp dir should be created");
    let dockerfile = temp.path().join("Dockerfile");
    std::fs::write(
        &dockerfile,
        "# hadolint ignore=DL3007, SC2086\nFROM alpine:latest\nRUN echo ok # hadolint ignore=SC2086\n",
    )
    .expect("Dockerfile should be written");

    rudolint_cmd()
        .args([
            "check",
            "--fix",
            "--migrate-hadolint-ignores",
            "--exit-zero",
        ])
        .arg(&dockerfile)
        .assert()
        .success();

    let output = std::fs::read_to_string(&dockerfile).expect("Dockerfile should exist");
    insta::assert_snapshot!("fix_write_mode_migrate_hadolint_ignores_file", output);
}

#[test]
fn fix_dry_run_migrates_hadolint_inline_ignores_without_writing() {
    let temp = tempfile::TempDir::new().expect("temp dir should be created");
    let dockerfile = temp.path().join("Dockerfile");
    let original = "# hadolint ignore=DL3007\nFROM alpine:latest\n";
    std::fs::write(&dockerfile, original).expect("Dockerfile should be written");

    let output = rudolint_cmd()
        .args([
            "check",
            "--fix",
            "--migrate-hadolint-ignores",
            "--dry-run",
            "--exit-zero",
        ])
        .arg(&dockerfile)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    assert_eq!(
        std::fs::read_to_string(&dockerfile).expect("Dockerfile should still exist"),
        original
    );
    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    assert!(
        output.contains("convert hadolint inline suppression to rudolint"),
        "dry-run output should render the migration preview:\n{output}"
    );
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
        .stdout(predicates::str::contains("DL3015"))
        .stdout(predicates::str::contains("DL3016"))
        .stdout(predicates::str::contains("DL3018"))
        .stdout(predicates::str::contains("DL3019"))
        .stdout(predicates::str::contains("DL3021"))
        .stdout(predicates::str::contains("DL3022"))
        .stdout(predicates::str::contains("DL3023"))
        .stdout(predicates::str::contains("DL3026"))
        .stdout(predicates::str::contains("DL3027"))
        .stdout(predicates::str::contains("DL3028"))
        .stdout(predicates::str::contains("DL3029"))
        .stdout(predicates::str::contains("DL3030"))
        .stdout(predicates::str::contains("DL3032"))
        .stdout(predicates::str::contains("DL3033"))
        .stdout(predicates::str::contains("DL3034"))
        .stdout(predicates::str::contains("DL3035"))
        .stdout(predicates::str::contains("DL3036"))
        .stdout(predicates::str::contains("DL3037"))
        .stdout(predicates::str::contains("DL3038"))
        .stdout(predicates::str::contains("DL3040"))
        .stdout(predicates::str::contains("DL3041"))
        .stdout(predicates::str::contains("DL3042"))
        .stdout(predicates::str::contains("DL3043"))
        .stdout(predicates::str::contains("DL3044"))
        .stdout(predicates::str::contains("DL3045"))
        .stdout(predicates::str::contains("DL3046"))
        .stdout(predicates::str::contains("DL3047"))
        .stdout(predicates::str::contains("DL3048"))
        .stdout(predicates::str::contains("DL3049"))
        .stdout(predicates::str::contains("DL3050"))
        .stdout(predicates::str::contains("DL3051"))
        .stdout(predicates::str::contains("DL3052"))
        .stdout(predicates::str::contains("DL3053"))
        .stdout(predicates::str::contains("DL3054"))
        .stdout(predicates::str::contains("DL3055"))
        .stdout(predicates::str::contains("DL3056"))
        .stdout(predicates::str::contains("DL3057"))
        .stdout(predicates::str::contains("DL3058"))
        .stdout(predicates::str::contains("DL3059"))
        .stdout(predicates::str::contains("DL3060"))
        .stdout(predicates::str::contains("DL3061"))
        .stdout(predicates::str::contains("DL3062"))
        .stdout(predicates::str::contains("DL3063"))
        .stdout(predicates::str::contains("DL4001"))
        .stdout(predicates::str::contains("DL4005"))
        .stdout(predicates::str::contains("DL4006"))
        .stdout(predicates::str::contains("SC1000").not());
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
        .args(["explain", "DL3007"])
        .assert()
        .success()
        .stdout(predicates::str::contains("DL3007"))
        .stdout(predicates::str::contains("reject latest base image tags"));
}

#[test]
fn unknown_explain_rule_exits_with_code_two() {
    rudolint_cmd().args(["explain", "DL9999"]).assert().code(2);
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
    let output: Value = serde_json::from_str(&output).expect("version output should be JSON");
    assert_eq!(output["name"], "rudolint");
    assert_eq!(output["version"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn emits_plain_version() {
    rudolint_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::contains(format!(
            "rudolint {}",
            env!("CARGO_PKG_VERSION")
        )));
}

#[test]
fn upgrade_dry_run_prints_latest_installer_command() {
    rudolint_cmd()
        .args(["upgrade", "--dry-run"])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "curl --proto '=https' --tlsv1.2 -LsSf https://kubeply.com/rudolint/install.sh | sh",
        ));
}

#[test]
fn upgrade_dry_run_accepts_pinned_release_tag() {
    rudolint_cmd()
        .args(["upgrade", "--dry-run", "--tag", "1.1.1"])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "https://kubeply.com/rudolint/v1.1.1/install.sh",
        ));
}

#[test]
fn upgrade_dry_run_json_reports_installer_command() {
    let output = rudolint_cmd()
        .args(["--json", "upgrade", "--dry-run", "--tag", "v1.1.1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let output: Value = serde_json::from_str(&output).expect("upgrade output should be JSON");
    assert_eq!(
        output["installer_url"],
        "https://kubeply.com/rudolint/v1.1.1/install.sh"
    );
    assert_eq!(
        output["command"],
        "curl --proto '=https' --tlsv1.2 -LsSf https://kubeply.com/rudolint/v1.1.1/install.sh | sh"
    );
}

#[test]
fn upgrade_skips_current_release_tag() {
    rudolint_cmd()
        .args(["upgrade", "--tag", env!("CARGO_PKG_VERSION")])
        .assert()
        .success()
        .stdout(predicates::str::contains(format!(
            "rudolint is already up to date (v{})",
            env!("CARGO_PKG_VERSION")
        )));
}

#[test]
fn upgrade_skips_current_release_tag_in_json() {
    let output = rudolint_cmd()
        .args(["--json", "upgrade", "--tag", env!("CARGO_PKG_VERSION")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).expect("stdout should be UTF-8");
    let output: Value = serde_json::from_str(&output).expect("upgrade output should be JSON");
    assert_eq!(output["status"], "up_to_date");
    assert_eq!(
        output["current_version"],
        format!("v{}", env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(
        output["target_version"],
        format!("v{}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn upgrade_rejects_invalid_release_version() {
    rudolint_cmd()
        .args(["upgrade", "--dry-run", "--tag", "latest;rm"])
        .assert()
        .code(2)
        .stderr(predicates::str::contains("invalid release version"));
}
