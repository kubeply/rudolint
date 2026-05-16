use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use rudolint_diagnostics::Severity;
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(default)]
    pub select: BTreeSet<String>,
    #[serde(default)]
    pub ignore: BTreeSet<String>,
    #[serde(default)]
    pub extend_ignore: BTreeSet<String>,
    #[serde(default)]
    pub severity: BTreeMap<String, Severity>,
    #[serde(default)]
    pub trusted_registries: Vec<String>,
    #[serde(default)]
    pub allow_entitlements: BTreeSet<String>,
    #[serde(default)]
    pub per_file_ignores: BTreeMap<String, BTreeSet<String>>,
}

impl Config {
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let Some(path) = path else {
            return Ok(Self::default());
        };
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        serde_yaml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
    }

    pub fn ignores(&self, code: &str) -> bool {
        self.ignore.contains(code) || self.extend_ignore.contains(code)
    }

    pub fn severity_override(&self, code: &str) -> Option<Severity> {
        self.severity.get(code).copied()
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
