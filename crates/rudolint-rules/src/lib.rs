mod buildkit;
mod catalog;
mod compat;
mod engine;
mod metadata;
mod shell;

use std::fmt;

use clap::ValueEnum;
use rudolint_diagnostics::{Finding, Severity};
use rudolint_dockerfile::Dockerfile;
use rudolint_policy::PolicyProfile;
use serde::Serialize;

pub use engine::RuleEngine;

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum Profile {
    /// BuildKit-native rules plus broadly compatible Dockerfile checks.
    #[default]
    Default,
    /// Emit only diagnostics intended to match established Dockerfile rule IDs.
    Compat,
}

impl From<Profile> for PolicyProfile {
    fn from(profile: Profile) -> Self {
        match profile {
            Profile::Default => Self::Default,
            Profile::Compat => Self::Compat,
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
}

pub trait Rule: Send + Sync {
    fn info(&self) -> RuleInfo;
    fn check(&self, document: &Dockerfile) -> Vec<Finding>;
}
