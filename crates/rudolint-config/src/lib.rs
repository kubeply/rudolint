use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rudolint_diagnostics::Severity;
use serde::Deserialize;

/// Project configuration loaded from a rudolint configuration file.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
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
    /// BuildKit entitlements allowed by policy.
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
        serde_yaml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
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
        self.ignore.contains(code) || self.extend_ignore.contains(code)
    }

    /// Returns the configured severity override for `code`, if present.
    pub fn severity_override(&self, code: &str) -> Option<Severity> {
        self.severity.get(code).copied()
    }
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
allow-entitlements:
  - security.insecure
per-file-ignores:
  fixtures/**:
    - RDL3000
"#,
        )
        .expect("config should parse");

        assert!(config.select.contains("RDL"));
        assert!(config.ignores("RDL1001"));
        assert!(config.ignores("RDL3007"));
        assert_eq!(config.severity_override("RDL3000"), Some(Severity::Error));
        assert_eq!(config.trusted_registries, ["ghcr.io"]);
        assert!(config.allow_entitlements.contains("security.insecure"));
        assert!(config.per_file_ignores["fixtures/**"].contains("RDL3000"));
    }
}
