use rudolint_test::{normalized_json, rudolint_cmd};

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
