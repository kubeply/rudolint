//! Shared test-only helpers.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::Value;

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("rudolint-test should live under crates/rudolint-test")
        .to_path_buf()
}

pub fn fixture_path(path: impl AsRef<Path>) -> PathBuf {
    workspace_root().join("fixtures").join(path)
}

pub fn read_fixture(path: impl AsRef<Path>) -> String {
    let path = fixture_path(path);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read fixture {}: {err}", path.display()))
}

pub fn rudolint_cmd() -> Command {
    Command::cargo_bin("rudolint").expect("rudolint binary should be built for integration tests")
}

pub fn normalize_json_paths(value: &mut Value) {
    normalize_paths(value);
}

pub fn normalized_json(raw: &str) -> Value {
    let mut value: Value = serde_json::from_str(raw).expect("output should be valid JSON");
    normalize_json_paths(&mut value);
    value
}

pub fn snapshot_name(parts: &[&str]) -> String {
    assert!(!parts.is_empty(), "snapshot names need at least one part");
    parts
        .iter()
        .map(|part| {
            assert!(!part.is_empty(), "snapshot name parts must not be empty");
            part.chars()
                .map(|character| {
                    if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                        character.to_ascii_lowercase()
                    } else {
                        '_'
                    }
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("__")
}

fn normalize_paths(value: &mut Value) {
    match value {
        Value::Array(items) => {
            for item in items {
                normalize_paths(item);
            }
        }
        Value::Object(map) => {
            for (key, value) in map {
                if matches!(
                    key.as_str(),
                    "path" | "uri" | "artifactLocation" | "absolutePath"
                ) {
                    replace_path_value(value);
                } else {
                    normalize_paths(value);
                }
            }
        }
        Value::String(text) => {
            if looks_like_workspace_path(text) {
                *text = text.replace(workspace_root().to_string_lossy().as_ref(), "$WORKSPACE");
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn replace_path_value(value: &mut Value) {
    match value {
        Value::String(text) => {
            if looks_like_workspace_path(text) {
                *text = text.replace(workspace_root().to_string_lossy().as_ref(), "$WORKSPACE");
            }
        }
        _ => normalize_paths(value),
    }
}

fn looks_like_workspace_path(text: &str) -> bool {
    text.contains(workspace_root().to_string_lossy().as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn fixture_path_resolves_under_workspace() {
        assert!(fixture_path("parser").ends_with("fixtures/parser"));
    }

    #[test]
    fn snapshot_name_normalizes_parts() {
        assert_eq!(
            snapshot_name(&["RDL3007", "No Latest Tag"]),
            "rdl3007__no_latest_tag"
        );
    }

    #[test]
    fn normalize_json_paths_replaces_workspace_paths() {
        let raw_path = workspace_root().join("fixtures/parser/Dockerfile");
        let mut value = json!({ "path": raw_path });

        normalize_json_paths(&mut value);

        assert_eq!(value["path"], "$WORKSPACE/fixtures/parser/Dockerfile");
    }
}
