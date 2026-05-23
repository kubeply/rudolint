use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use globset::Glob;
use rudolint_diagnostics::Severity;
use serde::Deserialize;

/// Project configuration loaded from a rudolint configuration file.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Rule code prefixes or exact codes to select explicitly.
    #[serde(default)]
    pub select: BTreeSet<String>,
    /// Rule codes to ignore.
    #[serde(default)]
    pub ignore: BTreeSet<String>,
    /// Additional rule codes to ignore without replacing default ignores.
    #[serde(default)]
    pub extend_ignore: BTreeSet<String>,
    /// Per-rule severity overrides keyed by rule code.
    #[serde(default)]
    pub severity: BTreeMap<String, Severity>,
    /// Registry hostnames considered trusted by registry-sensitive rules.
    #[serde(default)]
    pub trusted_registries: Vec<String>,
    /// Required label keys and expected schema values used by label validation rules.
    #[serde(default)]
    pub label_schema: BTreeMap<String, String>,
    /// BuildKit entitlements allowed by policy.
    #[serde(default)]
    pub strict_labels: bool,
    #[serde(default)]
    pub allow_entitlements: BTreeSet<String>,
    /// Rule ignores scoped to path patterns.
    #[serde(default)]
    pub per_file_ignores: BTreeMap<String, BTreeSet<String>>,
}

impl Config {
    /// Loads configuration from `path`, or returns the default config when no path is provided.
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let Some(path) = path else {
            return Ok(Self::default());
        };
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let config = serde_yaml::from_str(&raw)
            .map_err(|error| parse_error(path, error))
            .with_context(|| format!("failed to parse {}", path.display()))?;
        validate_per_file_ignore_patterns(&config, path)?;
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
    /// Entries may be exact rule codes such as `RDL3000` or prefixes such as `RDK`.
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

    #[test]
    fn parses_full_config_schema() {
        let config = serde_yaml::from_str::<Config>(
            r#"
select:
  - RDL
ignore:
  - RDL1001
extend-ignore:
  - RDL3007
severity:
  RDL3000: error
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
    - RDL3000
"#,
        )
        .expect("config should parse");

        assert!(config.select.contains("RDL"));
        assert!(config.selects("RDL3000"));
        assert!(!config.selects("RDK1000"));
        assert!(config.ignores("RDL1001"));
        assert!(config.ignores("RDL3007"));
        assert_eq!(config.severity_override("RDL3000"), Some(Severity::Error));
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
        assert!(config.per_file_ignores["fixtures/**"].contains("RDL3000"));
        assert!(config.ignores_for_path("RDL3000", Path::new("fixtures/rules/Dockerfile")));
        assert!(!config.ignores_for_path("RDL3000", Path::new("src/Dockerfile")));
    }

    #[test]
    fn parse_errors_include_line_and_column_when_available() {
        let temp = tempfile::TempDir::new().expect("temp dir should be created");
        let path = temp.path().join(".rudolint.yaml");
        std::fs::write(&path, "ignore: [RDL3000\n").expect("config should be written");

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
            "per-file-ignores:\n  '[unterminated':\n    - RDL3000\n",
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
        std::fs::write(&path, "ignroe:\n  - RDL3000\n").expect("config should be written");

        let error = Config::load(Some(&path)).expect_err("config should fail to load");
        let message = format!("{error:#}");

        assert!(message.contains("unknown field"));
        assert!(message.contains("ignroe"));
    }
}
