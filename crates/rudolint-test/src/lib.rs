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

fn normalize_json_paths(value: &mut Value) {
    normalize_paths(value);
}

pub fn normalize_path_prefix(value: &mut Value, prefix: &Path, placeholder: &str) {
    normalize_path_prefix_inner(value, prefix, placeholder);
}

pub fn normalized_json(raw: &str) -> Value {
    let mut value: Value = serde_json::from_str(raw).expect("output should be valid JSON");
    normalize_json_paths(&mut value);
    value
}

#[cfg(test)]
fn snapshot_name(parts: &[&str]) -> String {
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

fn normalize_path_prefix_inner(value: &mut Value, prefix: &Path, placeholder: &str) {
    let prefix = prefix.to_string_lossy();
    match value {
        Value::Array(items) => {
            for item in items {
                normalize_path_prefix_inner(item, Path::new(prefix.as_ref()), placeholder);
            }
        }
        Value::Object(map) => {
            for value in map.values_mut() {
                normalize_path_prefix_inner(value, Path::new(prefix.as_ref()), placeholder);
            }
        }
        Value::String(text) => {
            if text.contains(prefix.as_ref()) {
                *text = text.replace(prefix.as_ref(), placeholder);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
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
    fn real_world_corpus_fixtures_are_self_contained() {
        let corpus = fixture_path("corpus/real-world");
        let mut fixture_dirs = fs::read_dir(&corpus)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", corpus.display()))
            .map(|entry| {
                entry
                    .expect("fixture directory entry should be readable")
                    .path()
            })
            .filter(|path| path.is_dir())
            .collect::<Vec<_>>();
        fixture_dirs.sort();

        assert!(
            !fixture_dirs.is_empty(),
            "real-world corpus should contain fixture directories"
        );

        for fixture_dir in fixture_dirs {
            let dockerfile = fixture_dir.join("Dockerfile");
            let metadata = fixture_dir.join("metadata.md");

            assert!(
                dockerfile.is_file(),
                "{} should contain a Dockerfile",
                fixture_dir.display()
            );
            assert!(
                metadata.is_file(),
                "{} should contain metadata.md",
                fixture_dir.display()
            );

            let dockerfile_source = fs::read_to_string(&dockerfile)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", dockerfile.display()));
            let metadata_source = fs::read_to_string(&metadata)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", metadata.display()));

            assert!(
                !dockerfile_source.trim().is_empty(),
                "{} should not be empty",
                dockerfile.display()
            );
            assert!(
                !metadata_source.trim().is_empty(),
                "{} should not be empty",
                metadata.display()
            );

            let mut files = fs::read_dir(&fixture_dir)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", fixture_dir.display()))
                .map(|entry| entry.expect("fixture file entry should be readable").path())
                .filter(|path| path.is_file())
                .map(|path| {
                    path.file_name()
                        .expect("fixture file should have a name")
                        .to_string_lossy()
                        .into_owned()
                })
                .collect::<Vec<_>>();
            files.sort();

            assert_eq!(
                files,
                ["Dockerfile".to_string(), "metadata.md".to_string()],
                "{} should be self-contained and avoid support files that require network, Docker, or package manager setup",
                fixture_dir.display()
            );
        }
    }

    #[test]
    fn snapshot_name_normalizes_parts() {
        assert_eq!(
            snapshot_name(&["DL3007", "No Latest Tag"]),
            "dl3007__no_latest_tag"
        );
    }

    #[test]
    fn normalize_json_paths_replaces_workspace_paths() {
        let raw_path = workspace_root().join("fixtures/parser/Dockerfile");
        let mut value = json!({ "path": raw_path });

        normalize_json_paths(&mut value);

        assert_eq!(value["path"], "$WORKSPACE/fixtures/parser/Dockerfile");
    }

    #[test]
    fn normalize_path_prefix_replaces_custom_prefix() {
        let mut value = json!({ "path": "/tmp/example/Dockerfile" });

        normalize_path_prefix(&mut value, Path::new("/tmp/example"), "$TEMP");

        assert_eq!(value["path"], "$TEMP/Dockerfile");
    }
}
