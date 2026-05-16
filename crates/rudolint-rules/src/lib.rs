mod core;

use std::fmt;

use clap::ValueEnum;
use rudolint_config::Config;
use rudolint_diagnostics::{Finding, Severity};
use rudolint_dockerfile::Dockerfile;
use rudolint_policy::PolicyProfile;
use serde::Serialize;

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

pub struct RuleEngine {
    rules: Vec<Box<dyn Rule>>,
    policy: PolicyProfile,
    config: Config,
}

impl RuleEngine {
    pub fn new(profile: Profile, config: Config) -> Self {
        let policy = profile.policy();
        let _trusted_registry_count = config.trusted_registries.len();
        Self {
            rules: core::implemented_rules(policy),
            policy,
            config,
        }
    }

    pub fn lint(&self, document: &Dockerfile) -> Vec<Finding> {
        let mut findings = Vec::new();
        for rule in &self.rules {
            let info = rule.info();
            if self.config.ignores(info.code) {
                continue;
            }
            findings.extend(rule.check(document).into_iter().map(|mut finding| {
                if let Some(severity) = self.config.severity_override(&finding.code) {
                    finding.severity = severity;
                }
                finding
            }));
        }
        findings.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then(left.line().cmp(&right.line()))
                .then(left.column().cmp(&right.column()))
                .then(left.code.cmp(&right.code))
        });
        findings
    }

    pub fn catalog(&self) -> Vec<RuleInfo> {
        core::catalog(self.policy)
    }
}
