use rudolint_diagnostics::{Finding, Severity};
use rudolint_dockerfile::Instruction;
use rudolint_policy::PolicyProfile;

use self::RuleSignal::{Correctness, Hardening, Performance};
use crate::RuleStatus;

/// Stable metadata describing a rule in the catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleMetadata {
    /// Stable rule code, such as `DL3000`.
    pub code: &'static str,
    /// Human-readable rule name used in docs and catalog output.
    pub name: &'static str,
    /// Short rule summary.
    pub summary: &'static str,
    /// Default severity before configuration overrides are applied.
    pub default_severity: Severity,
    /// Primary policy family for the rule.
    pub profile: PolicyProfile,
    /// Broad rule category.
    pub category: RuleCategory,
    /// Signal profiles this rule belongs to.
    pub signals: &'static [RuleSignal],
    /// Implementation status of the rule.
    pub status: RuleStatus,
    /// Documentation URL for the rule.
    pub docs_url: String,
    /// Whether an automated fix is available.
    pub fix: FixAvailability,
}

impl RuleMetadata {
    /// Builds metadata for an implemented native rule.
    pub(super) fn implemented(
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
            signals: signals_for_code(code),
            status: RuleStatus::Implemented,
            docs_url: docs_url(code),
            fix,
        }
    }

    /// Builds metadata for a planned compatibility rule.
    pub(super) fn planned_compat(code: &'static str) -> Self {
        Self {
            code,
            name: code,
            summary: "tracked for compatibility parity",
            default_severity: Severity::Warning,
            profile: PolicyProfile::HadolintCompat,
            category: RuleCategory::Compatibility,
            signals: &[],
            status: RuleStatus::Planned,
            docs_url: docs_url(code),
            fix: FixAvailability::None,
        }
    }

    /// Builds metadata for a planned shell-rule catalog entry.
    pub(super) fn planned_shell(code: &'static str) -> Self {
        Self {
            code,
            name: code,
            summary: "tracked for shell-analysis coverage",
            default_severity: Severity::Warning,
            profile: PolicyProfile::HadolintCompat,
            category: RuleCategory::Shell,
            signals: &[],
            status: RuleStatus::Planned,
            docs_url: docs_url(code),
            fix: FixAvailability::None,
        }
    }
}

/// User-facing signal profiles that group rules by intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleSignal {
    /// Rules that catch likely broken, surprising, or non-portable builds.
    Correctness,
    /// Rules that improve cache reuse, install speed, or image build efficiency.
    Performance,
    /// Rules that reduce secret exposure, unpinned inputs, or supply-chain risk.
    Hardening,
}

impl RuleSignal {
    /// Returns the stable string identifier for this signal.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Correctness => "correctness",
            Self::Performance => "performance",
            Self::Hardening => "hardening",
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
    /// Rudolint-native migration or project policy rules.
    Rudolint,
    /// Shell-analysis catalog entries.
    Shell,
}

impl RuleCategory {
    /// Returns the stable string identifier for this category.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Compatibility => "compatibility",
            Self::BuildKit => "buildkit",
            Self::Rudolint => "rudolint",
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

pub(crate) fn profile_includes_code(profile: PolicyProfile, code: &str) -> bool {
    match profile {
        PolicyProfile::Default | PolicyProfile::Strict => true,
        PolicyProfile::HadolintCompat => code.starts_with("DL") || code.starts_with("SC"),
        PolicyProfile::Correctness => signals_for_code(code).contains(&RuleSignal::Correctness),
        PolicyProfile::Performance => signals_for_code(code).contains(&RuleSignal::Performance),
        PolicyProfile::Hardening => signals_for_code(code).contains(&RuleSignal::Hardening),
    }
}

pub(crate) fn diagnostic(
    code: &'static str,
    severity: Severity,
    message: impl Into<String>,
    instruction: &Instruction,
) -> Finding {
    Finding::with_span(code, severity, message, instruction.raw_span)
}

fn category_for_code(code: &str) -> RuleCategory {
    if code.starts_with("RDK") {
        RuleCategory::BuildKit
    } else if code.starts_with("RUD") {
        RuleCategory::Rudolint
    } else if code.starts_with("SC") {
        RuleCategory::Shell
    } else {
        RuleCategory::Compatibility
    }
}

fn profile_for_code(code: &str) -> PolicyProfile {
    if code.starts_with("DL") || code.starts_with("SC") {
        PolicyProfile::HadolintCompat
    } else {
        PolicyProfile::Default
    }
}

fn signals_for_code(code: &str) -> &'static [RuleSignal] {
    match code {
        // Rudolint migration hygiene is intentionally kept out of signal profiles.
        "RUD1001" => &[],

        // Build structure and Dockerfile semantics.
        "DL3000" | "DL3001" | "DL3003" | "DL3011" | "DL3012" | "DL3021" | "DL3022" | "DL3023"
        | "DL3024" | "DL3027" | "DL3029" | "DL3030" | "DL3034" | "DL3035" | "DL3038" | "DL3043"
        | "DL3044" | "DL3045" | "DL3046" | "DL3048" | "DL3049" | "DL3050" | "DL3051" | "DL3052"
        | "DL3053" | "DL3054" | "DL3055" | "DL3056" | "DL3057" | "DL3058" | "DL3061" | "DL3063"
        | "DL4000" | "DL4003" | "DL4004" | "DL4005" | "DL4006" | "SC2015" | "SC2046" | "SC2086"
        | "SC2155" | "SC2164" | "SC2181" | "RDK1000" | "RDK1009" | "RDK1010" => &[Correctness],

        // Build cache, layer, and package install efficiency.
        "DL3009" | "DL3010" | "DL3014" | "DL3015" | "DL3019" | "DL3032" | "DL3036" | "DL3040"
        | "DL3042" | "DL3047" | "DL3059" | "DL3060" | "SC2002" | "RDK1003" | "RDK1006" => {
            &[Performance]
        }

        // User, provenance, pinning, and secret handling.
        "DL3002" | "DL3004" | "DL3006" | "DL3007" | "DL3008" | "DL3013" | "DL3016" | "DL3018"
        | "DL3026" | "DL3028" | "DL3033" | "DL3037" | "DL3041" | "DL3062" | "RDK1001"
        | "RDK1002" | "RDK1004" | "RDK1005" | "RDK1008" => &[Hardening],

        // Rules that are both a build-safety and hardening concern.
        "DL3020" | "DL3025" | "DL4001" | "RDK1007" => &[Correctness, Hardening],

        _ => &[],
    }
}

fn docs_url(code: &str) -> String {
    format!("https://github.com/kubeply/rudolint/blob/main/docs/rules/{code}.md")
}
