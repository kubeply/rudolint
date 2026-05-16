use rudolint_diagnostics::{Finding, Severity};
use rudolint_dockerfile::Instruction;
use rudolint_policy::PolicyProfile;

use crate::RuleStatus;

/// Stable metadata describing a rule in the catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleMetadata {
    /// Stable rule code, such as `RDL3000`.
    pub code: &'static str,
    /// Human-readable rule name used in docs and catalog output.
    pub name: &'static str,
    /// Short rule summary.
    pub summary: &'static str,
    /// Default severity before configuration overrides are applied.
    pub default_severity: Severity,
    /// Policy profile that owns the rule.
    pub profile: PolicyProfile,
    /// Broad rule category.
    pub category: RuleCategory,
    /// Implementation status of the rule.
    pub status: RuleStatus,
    /// Documentation URL for the rule.
    pub docs_url: String,
    /// Whether an automated fix is available.
    pub fix: FixAvailability,
}

impl RuleMetadata {
    /// Builds metadata for an implemented native rule.
    pub fn implemented(
        code: &'static str,
        name: &'static str,
        default_severity: Severity,
        summary: &'static str,
        fix: FixAvailability,
    ) -> Self {
        Self {
            code,
            name,
            summary,
            default_severity,
            profile: profile_for_code(code),
            category: category_for_code(code),
            status: RuleStatus::Implemented,
            docs_url: docs_url(code),
            fix,
        }
    }

    /// Builds metadata for a planned compatibility rule.
    pub fn planned_compat(code: &'static str) -> Self {
        Self {
            code,
            name: code,
            summary: "tracked for compatibility parity",
            default_severity: Severity::Warning,
            profile: PolicyProfile::Compat,
            category: RuleCategory::Compatibility,
            status: RuleStatus::Planned,
            docs_url: docs_url(code),
            fix: FixAvailability::None,
        }
    }

    /// Builds metadata for a shell-rule catalog entry handled outside this crate.
    pub fn external_shell(code: &'static str) -> Self {
        Self {
            code,
            name: code,
            summary: "shell diagnostics delegated to the shell-analysis layer",
            default_severity: Severity::Warning,
            profile: PolicyProfile::Default,
            category: RuleCategory::Shell,
            status: RuleStatus::External,
            docs_url: docs_url(code),
            fix: FixAvailability::None,
        }
    }
}

/// Broad category for grouping rules in catalog output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleCategory {
    /// Dockerfile compatibility and parity rules.
    Compatibility,
    /// BuildKit-native policy rules.
    BuildKit,
    /// Shell-analysis catalog entries.
    Shell,
}

impl RuleCategory {
    /// Returns the stable string identifier for this category.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Compatibility => "compatibility",
            Self::BuildKit => "buildkit",
            Self::Shell => "shell",
        }
    }
}

/// Indicates whether a rule has an automated fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixAvailability {
    /// A safe automatic fix is available.
    Safe,
    /// A fix requires manual review or edits.
    Manual,
    /// No automated fix is available.
    None,
}

impl FixAvailability {
    /// Returns the stable string identifier for this fix availability.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Manual => "manual",
            Self::None => "none",
        }
    }
}

macro_rules! rule {
    ($type_name:ident, $code:literal, $name:literal, $severity:expr, $summary:literal, $body:expr) => {
        pub(crate) struct $type_name;
        impl crate::Rule for $type_name {
            fn info(&self) -> crate::RuleInfo {
                crate::RuleInfo::from_metadata(crate::RuleMetadata::implemented(
                    $code,
                    $name,
                    $severity,
                    $summary,
                    crate::FixAvailability::None,
                ))
            }

            fn check(
                &self,
                document: &rudolint_dockerfile::Dockerfile,
            ) -> Vec<rudolint_diagnostics::Finding> {
                $body(document)
            }
        }
    };
}

pub(crate) use rule;

pub(crate) fn diagnostic(
    code: &'static str,
    severity: Severity,
    message: impl Into<String>,
    instruction: &Instruction,
) -> Finding {
    Finding::new(code, severity, message, instruction.line, 1)
}

fn category_for_code(code: &str) -> RuleCategory {
    if code.starts_with("RDK") {
        RuleCategory::BuildKit
    } else {
        RuleCategory::Compatibility
    }
}

fn profile_for_code(code: &str) -> PolicyProfile {
    if code.starts_with("RDL") || code.starts_with("RSC") {
        PolicyProfile::Compat
    } else {
        PolicyProfile::Default
    }
}

fn docs_url(code: &str) -> String {
    format!("https://github.com/kubeply/rudolint/blob/main/docs/rules/{code}.md")
}
