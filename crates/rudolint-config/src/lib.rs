use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use globset::Glob;
use rudolint_diagnostics::Severity;
use serde::Deserialize;

/// Project configuration loaded from a rudolint configuration file.
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// Rule code prefixes or exact codes to select explicitly.
    pub select: BTreeSet<String>,
    /// Rule codes to ignore.
    pub ignore: BTreeSet<String>,
    /// Additional rule codes to ignore without replacing default ignores.
    pub extend_ignore: BTreeSet<String>,
    /// Per-rule severity overrides keyed by rule code.
    pub severity: BTreeMap<String, Severity>,
    /// Registry hostnames considered trusted by registry-sensitive rules.
    pub trusted_registries: Vec<String>,
    /// Required label keys and expected schema values used by label validation rules.
    pub label_schema: BTreeMap<String, String>,
    /// BuildKit entitlements allowed by policy.
    pub strict_labels: bool,
    pub allow_entitlements: BTreeSet<String>,
    /// Rule ignores scoped to path patterns.
    pub per_file_ignores: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
struct RawConfig {
    #[serde(default)]
    select: BTreeSet<String>,
    #[serde(default)]
    ignore: BTreeSet<String>,
    #[serde(default)]
    ignored: BTreeSet<String>,
    #[serde(default)]
    extend_ignore: BTreeSet<String>,
    #[serde(default)]
    severity: BTreeMap<String, Severity>,
    #[serde(default)]
    trusted_registries: Vec<String>,
    #[serde(default)]
    label_schema: BTreeMap<String, String>,
    #[serde(default)]
    strict_labels: bool,
    #[serde(default)]
    allow_entitlements: BTreeSet<String>,
    #[serde(default)]
    per_file_ignores: BTreeMap<String, BTreeSet<String>>,
}

impl<'de> Deserialize<'de> for Config {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = RawConfig::deserialize(deserializer)?;
        let mut ignore = raw.ignore;
        ignore.extend(raw.ignored);

        Ok(Self {
            select: raw.select,
            ignore,
            extend_ignore: raw.extend_ignore,
            severity: raw.severity,
            trusted_registries: raw.trusted_registries,
            label_schema: raw.label_schema,
            strict_labels: raw.strict_labels,
            allow_entitlements: raw.allow_entitlements,
            per_file_ignores: raw.per_file_ignores,
        })
    }
}

impl Config {
    /// Loads configuration from `path`, or returns the default config when no path is provided.
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let Some(path) = path else {
            return Ok(Self::default());
        };
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let config: Self = serde_yaml::from_str(&raw)
            .map_err(|error| parse_error(path, error))
            .with_context(|| format!("failed to parse {}", path.display()))?;
        config.validate(path)?;
        Ok(config)
    }

    /// Loads an explicit config or discovers `.rudolint.yaml` from the provided start paths.
    pub fn load_discovered(explicit: Option<&Path>, starts: &[PathBuf]) -> Result<Self> {
        if let Some(path) = explicit {
            return Self::load(Some(path));
        }
        let Some(path) = discover(starts)? else {
            return Ok(Self::default());
        };
        Self::load(Some(&path))
    }

    /// Returns true when `code` is ignored by either ignore list.
    pub fn ignores(&self, code: &str) -> bool {
        code_matches_any(code, &self.ignore) || code_matches_any(code, &self.extend_ignore)
    }

    /// Returns true when `code` is selected by configuration.
    ///
    /// An empty select list means all rules selected by the active profile are enabled.
    /// Entries may be exact rule codes such as `DL3000` or prefixes such as `RDK`.
    pub fn selects(&self, code: &str) -> bool {
        self.select.is_empty() || code_matches_any(code, &self.select)
    }

    /// Returns true when `code` is ignored for `path` by a per-file ignore pattern.
    pub fn ignores_for_path(&self, code: &str, path: &Path) -> bool {
        self.per_file_ignores.iter().any(|(pattern, codes)| {
            code_matches_any(code, codes) && path_matches_pattern(path, pattern)
        })
    }

    /// Returns the configured severity override for `code`, if present.
    pub fn severity_override(&self, code: &str) -> Option<Severity> {
        self.severity.get(code).copied()
    }

    /// Validates config fields that depend on runtime parsing.
    pub fn validate(&self, path: &Path) -> Result<()> {
        validate_per_file_ignore_patterns(self, path)
    }
}

fn code_matches_any(code: &str, targets: &BTreeSet<String>) -> bool {
    targets
        .iter()
        .any(|target| code_matches_target(code, target))
}

fn code_matches_target(code: &str, target: &str) -> bool {
    code == target || code.starts_with(target)
}

fn path_matches_pattern(path: &Path, pattern: &str) -> bool {
    Glob::new(pattern)
        .expect("per-file ignore glob should be validated at config load time")
        .compile_matcher()
        .is_match(path)
}

fn validate_per_file_ignore_patterns(config: &Config, path: &Path) -> Result<()> {
    for pattern in config.per_file_ignores.keys() {
        Glob::new(pattern).with_context(|| {
            format!(
                "invalid per-file-ignores pattern `{pattern}` in {}",
                path.display()
            )
        })?;
    }
    Ok(())
}

fn parse_error(path: &Path, error: serde_yaml::Error) -> anyhow::Error {
    let location = error
        .location()
        .map(|location| format!(" at line {}, column {}", location.line(), location.column()))
        .unwrap_or_default();
    anyhow::anyhow!("{}{}: {}", path.display(), location, error)
}

/// Discovers the nearest `.rudolint.yaml` by walking upward from the start paths.
pub fn discover(starts: &[PathBuf]) -> Result<Option<PathBuf>> {
    if starts.is_empty() {
        return discover_from(std::env::current_dir().context("failed to get current directory")?);
    }

    for start in starts {
        let start = if start.is_file() {
            start.parent().unwrap_or(start).to_path_buf()
        } else {
            start.clone()
        };
        if let Some(path) = discover_from(start)? {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

fn discover_from(start: PathBuf) -> Result<Option<PathBuf>> {
    let mut current = start
        .canonicalize()
        .with_context(|| format!("failed to resolve {}", start.display()))?;
    loop {
        let candidate = current.join(".rudolint.yaml");
        if candidate.is_file() {
            return Ok(Some(candidate));
        }
        if !current.pop() {
            return Ok(None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;

    fn config_schema() -> &'static jsonschema::Validator {
        static SCHEMA: OnceLock<jsonschema::Validator> = OnceLock::new();
        SCHEMA.get_or_init(|| {
            let schema = serde_json::from_str(include_str!(
                "../../../schemas/rudolint-config-v1.schema.json"
            ))
            .expect("config schema should be valid JSON");
            jsonschema::validator_for(&schema).expect("config schema should compile")
        })
    }

    fn assert_matches_config_schema(path: &Path) {
        let raw = fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        let yaml = serde_yaml::from_str::<serde_yaml::Value>(&raw)
            .unwrap_or_else(|err| panic!("{} should parse as YAML: {err}", path.display()));
        let value = serde_json::to_value(yaml)
            .unwrap_or_else(|err| panic!("{} should convert to JSON: {err}", path.display()));
        let errors = config_schema()
            .iter_errors(&value)
            .map(|error| error.to_string())
            .collect::<Vec<_>>();

        assert!(
            errors.is_empty(),
            "{} should match the v1 config schema:\n{}",
            path.display(),
            errors.join("\n")
        );
    }

    fn collect_config_paths(root: &Path) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        let root_config = root.join("rudolint.yaml");
        if root_config.is_file() {
            paths.push(root_config);
        }

        let hidden_root_config = root.join(".rudolint.yaml");
        if hidden_root_config.is_file() {
            paths.push(hidden_root_config);
        }

        let fixtures = root.join("fixtures");
        if fixtures.is_dir() {
            collect_fixture_config_paths(&fixtures, &mut paths);
        }

        paths.sort();
        paths
    }

    fn collect_fixture_config_paths(dir: &Path, paths: &mut Vec<PathBuf>) {
        let entries = fs::read_dir(dir)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", dir.display()));

        for entry in entries {
            let entry = entry
                .unwrap_or_else(|err| panic!("failed to read entry in {}: {err}", dir.display()));
            let path = entry.path();
            if path.is_dir() {
                collect_fixture_config_paths(&path, paths);
            } else if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == "rudolint.yaml" || name == ".rudolint.yaml")
            {
                paths.push(path);
            }
        }
    }

    #[test]
    fn parses_full_config_schema() {
        let config = serde_yaml::from_str::<Config>(
            r#"
select:
  - DL
ignore:
  - RUD1001
extend-ignore:
  - DL3007
severity:
  DL3000: error
trusted-registries:
  - ghcr.io
label-schema:
  org.opencontainers.image.title: text
  org.opencontainers.image.source: url
strict-labels: true
allow-entitlements:
  - security.insecure
per-file-ignores:
  fixtures/**:
    - DL3000
"#,
        )
        .expect("config should parse");

        assert!(config.select.contains("DL"));
        assert!(config.selects("DL3000"));
        assert!(!config.selects("RDK1000"));
        assert!(config.ignores("RUD1001"));
        assert!(config.ignores("DL3007"));
        assert_eq!(config.severity_override("DL3000"), Some(Severity::Error));
        assert_eq!(config.trusted_registries, ["ghcr.io"]);
        assert_eq!(
            config.label_schema["org.opencontainers.image.title"],
            "text"
        );
        assert_eq!(
            config.label_schema["org.opencontainers.image.source"],
            "url"
        );
        assert!(config.strict_labels);
        assert!(config.allow_entitlements.contains("security.insecure"));
        assert!(config.per_file_ignores["fixtures/**"].contains("DL3000"));
        assert!(config.ignores_for_path("DL3000", Path::new("fixtures/rules/Dockerfile")));
        assert!(!config.ignores_for_path("DL3000", Path::new("src/Dockerfile")));
    }

    #[test]
    fn parses_hadolint_ignored_alias() {
        let config = serde_yaml::from_str::<Config>(
            r#"
ignored:
  - DL3008
  - SC2086
"#,
        )
        .expect("hadolint-style ignored key should parse");

        assert!(config.ignores("DL3008"));
        assert!(config.ignores("SC2086"));
    }

    #[test]
    fn merges_native_ignore_and_hadolint_ignored_alias() {
        let config = serde_yaml::from_str::<Config>(
            r#"
ignore:
  - DL3008
ignored:
  - SC2086
"#,
        )
        .expect("native and hadolint ignore keys should parse together");

        assert!(config.ignores("DL3008"));
        assert!(config.ignores("SC2086"));
    }

    #[test]
    fn repo_config_files_match_v1_schema() {
        let paths = collect_config_paths(&rudolint_test::workspace_root());
        assert!(!paths.is_empty(), "expected at least one config file");

        for path in paths {
            assert_matches_config_schema(&path);
        }
    }

    #[test]
    fn parse_errors_include_line_and_column_when_available() {
        let temp = tempfile::TempDir::new().expect("temp dir should be created");
        let path = temp.path().join(".rudolint.yaml");
        std::fs::write(&path, "ignore: [DL3000\n").expect("config should be written");

        let error = Config::load(Some(&path)).expect_err("config should fail to parse");
        let message = format!("{error:#}");

        assert!(message.contains("line 1"));
        assert!(message.contains("column"));
    }

    #[test]
    fn load_rejects_invalid_per_file_ignore_globs() {
        let temp = tempfile::TempDir::new().expect("temp dir should be created");
        let path = temp.path().join(".rudolint.yaml");
        std::fs::write(
            &path,
            "per-file-ignores:\n  '[unterminated':\n    - DL3000\n",
        )
        .expect("config should be written");

        let error = Config::load(Some(&path)).expect_err("config should fail to load");
        let message = format!("{error:#}");

        assert!(message.contains("invalid per-file-ignores pattern"));
        assert!(message.contains("[unterminated"));
    }

    #[test]
    fn load_rejects_unknown_config_keys() {
        let temp = tempfile::TempDir::new().expect("temp dir should be created");
        let path = temp.path().join(".rudolint.yaml");
        std::fs::write(&path, "ignroe:\n  - DL3000\n").expect("config should be written");

        let error = Config::load(Some(&path)).expect_err("config should fail to load");
        let message = format!("{error:#}");

        assert!(message.contains("unknown field"));
        assert!(message.contains("ignroe"));
    }
}
