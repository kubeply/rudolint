mod buildkit;
mod catalog;
mod compat;
mod engine;
mod metadata;
mod shell;

use std::fmt;

use clap::ValueEnum;
use rudolint_config::Config;
use rudolint_diagnostics::{Finding, Severity};
use rudolint_dockerfile::Dockerfile;
use rudolint_fix::FixPreview;
use rudolint_policy::PolicyProfile;
use serde::Serialize;

pub use engine::RuleEngine;
pub use metadata::{FixAvailability, RuleCategory, RuleMetadata, RuleSignal};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum Profile {
    /// BuildKit-native rules plus broadly compatible Dockerfile checks.
    #[default]
    Default,
    /// Emit Hadolint-style diagnostics without BuildKit-native rules.
    #[value(name = "hadolint-compat")]
    HadolintCompat,
    /// Emit high-confidence build correctness diagnostics.
    Correctness,
    /// Emit cache, layer, and install performance diagnostics.
    Performance,
    /// Emit secret, provenance, pinning, and supply-chain hardening diagnostics.
    Hardening,
}

impl From<Profile> for PolicyProfile {
    fn from(profile: Profile) -> Self {
        match profile {
            Profile::Default => Self::Default,
            Profile::HadolintCompat => Self::HadolintCompat,
            Profile::Correctness => Self::Correctness,
            Profile::Performance => Self::Performance,
            Profile::Hardening => Self::Hardening,
        }
    }
}

impl Profile {
    /// Returns the policy profile that backs this CLI-facing profile.
    pub fn policy(self) -> PolicyProfile {
        self.into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuleStatus {
    Implemented,
    Planned,
    External,
}

impl fmt::Display for RuleStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuleStatus::Implemented => f.write_str("implemented"),
            RuleStatus::Planned => f.write_str("planned"),
            RuleStatus::External => f.write_str("external"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuleInfo {
    pub code: &'static str,
    pub severity: Severity,
    pub summary: &'static str,
    pub status: RuleStatus,
    /// Full catalog metadata for this rule.
    pub metadata: RuleMetadata,
}

impl RuleInfo {
    /// Creates rule info from catalog metadata.
    pub fn from_metadata(metadata: RuleMetadata) -> Self {
        Self {
            code: metadata.code,
            severity: metadata.default_severity,
            summary: metadata.summary,
            status: metadata.status,
            metadata,
        }
    }
}

/// A lint rule that can emit diagnostics and optional fix previews.
pub trait Rule: Send + Sync {
    /// Returns stable catalog metadata for this rule.
    fn info(&self) -> RuleInfo;
    /// Returns diagnostics for `document`.
    fn check(&self, document: &Dockerfile) -> Vec<Finding>;

    /// Returns diagnostics for `document` using configuration, when needed.
    fn check_with_config(&self, document: &Dockerfile, _config: &Config) -> Vec<Finding> {
        self.check(document)
    }

    /// Returns suggested fixes for `document`, if this rule supports them.
    fn fix(&self, _document: &Dockerfile) -> Vec<FixPreview> {
        Vec::new()
    }
}
