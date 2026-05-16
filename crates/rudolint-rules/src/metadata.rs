use rudolint_diagnostics::{Finding, Severity};
use rudolint_dockerfile::Instruction;
use rudolint_policy::PolicyProfile;

use crate::RuleStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleMetadata {
    pub code: &'static str,
    pub name: &'static str,
    pub summary: &'static str,
    pub default_severity: Severity,
    pub profile: PolicyProfile,
    pub category: RuleCategory,
    pub status: RuleStatus,
    pub docs_url: String,
    pub fix: FixAvailability,
}

impl RuleMetadata {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleCategory {
    Compatibility,
    BuildKit,
    Shell,
}

impl RuleCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Compatibility => "compatibility",
            Self::BuildKit => "buildkit",
            Self::Shell => "shell",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixAvailability {
    Safe,
    Manual,
    None,
}

impl FixAvailability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Manual => "manual",
            Self::None => "none",
        }
    }
}

macro_rules! rule_metadata {
    ($type_name:ident, $code:literal, $name:literal, $severity:expr, $summary:literal) => {
        $crate::metadata::rule_metadata!(
            $type_name,
            $code,
            $name,
            $severity,
            $summary,
            $crate::FixAvailability::None
        );
    };
    ($type_name:ident, $code:literal, $name:literal, $severity:expr, $summary:literal, $fix:expr) => {
        pub(crate) struct $type_name;
        impl $type_name {
            fn metadata_info(&self) -> crate::RuleInfo {
                crate::RuleInfo::from_metadata(crate::RuleMetadata::implemented(
                    $code, $name, $severity, $summary, $fix,
                ))
            }
        }
    };
}

pub(crate) use rule_metadata;

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
    } else if code.starts_with("RSC") {
        RuleCategory::Shell
    } else {
        RuleCategory::Compatibility
    }
}

fn profile_for_code(code: &str) -> PolicyProfile {
    if code.starts_with("RDL") {
        PolicyProfile::Compat
    } else {
        PolicyProfile::Default
    }
}

fn docs_url(code: &str) -> String {
    format!("https://github.com/kubeply/rudolint/blob/main/docs/rules/{code}.md")
}
