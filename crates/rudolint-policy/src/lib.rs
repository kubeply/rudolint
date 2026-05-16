//! Rule selection, profiles, severity overrides, and compatibility policy.

use std::collections::BTreeSet;

/// Rule-selection profile used by the policy layer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PolicyProfile {
    /// Default rudolint behavior with compatibility and BuildKit-native rules.
    #[default]
    Default,
    /// Compatibility-focused behavior that excludes native-only rules.
    Compat,
    /// Reserved stricter behavior for future opt-in checks.
    Strict,
}

impl PolicyProfile {
    /// Returns the stable string identifier for the profile.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Compat => "compat",
            Self::Strict => "strict",
        }
    }

    /// Returns true when BuildKit-native rules are enabled.
    pub const fn includes_buildkit_native_rules(self) -> bool {
        match self {
            Self::Default | Self::Strict => true,
            Self::Compat => false,
        }
    }

    /// Returns true when compatibility catalog entries are included.
    pub const fn includes_compatibility_rules(self) -> bool {
        true
    }

    /// Returns true when external shell catalog entries are included.
    pub const fn includes_shell_catalog(self) -> bool {
        true
    }

    /// Returns true for the strict policy profile.
    pub const fn is_strict(self) -> bool {
        matches!(self, Self::Strict)
    }
}

/// Backwards-compatible alias for the active policy profile type.
pub type PolicyMode = PolicyProfile;

/// Project-native inline suppression parsed from a Dockerfile comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineSuppression {
    /// Source line where the suppression comment appears.
    pub line: usize,
    /// Rule target covered by the suppression.
    pub target: SuppressionTarget,
}

impl InlineSuppression {
    /// Parses a `# rudolint ignore=...` comment.
    pub fn parse_comment(line: usize, text: &str) -> Option<Self> {
        let body = text.strip_prefix('#')?.trim_start();
        let command_end = body.find(char::is_whitespace).unwrap_or(body.len());
        let (command, rest) = body.split_at(command_end);
        if !command.eq_ignore_ascii_case("rudolint") {
            return None;
        }

        let ignored = rest.trim_start().strip_prefix("ignore=")?.trim();
        let target = SuppressionTarget::parse(ignored)?;

        Some(Self { line, target })
    }

    /// Returns true when this suppression applies to `code`.
    pub fn matches(&self, code: &str) -> bool {
        self.target.matches(code)
    }
}

/// Rule set targeted by an inline suppression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuppressionTarget {
    /// Suppress every rule for the targeted instruction.
    All,
    /// Suppress the listed rule codes for the targeted instruction.
    Codes(BTreeSet<String>),
}

impl SuppressionTarget {
    fn parse(value: &str) -> Option<Self> {
        if value.eq_ignore_ascii_case("all") {
            return Some(Self::All);
        }

        let codes = value
            .split(',')
            .map(str::trim)
            .filter(|code| !code.is_empty())
            .map(|code| code.to_ascii_uppercase())
            .collect::<BTreeSet<_>>();

        if codes.is_empty() {
            None
        } else {
            Some(Self::Codes(codes))
        }
    }

    fn matches(&self, code: &str) -> bool {
        match self {
            Self::All => true,
            Self::Codes(codes) => codes.contains(&code.to_ascii_uppercase()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{InlineSuppression, PolicyProfile, SuppressionTarget};

    #[test]
    fn default_profile_combines_compatibility_and_native_rules() {
        let profile = PolicyProfile::Default;

        assert_eq!(profile.as_str(), "default");
        assert!(profile.includes_compatibility_rules());
        assert!(profile.includes_buildkit_native_rules());
        assert!(profile.includes_shell_catalog());
        assert!(!profile.is_strict());
    }

    #[test]
    fn compat_profile_excludes_buildkit_native_rules() {
        let profile = PolicyProfile::Compat;

        assert_eq!(profile.as_str(), "compat");
        assert!(profile.includes_compatibility_rules());
        assert!(!profile.includes_buildkit_native_rules());
        assert!(profile.includes_shell_catalog());
        assert!(!profile.is_strict());
    }

    #[test]
    fn strict_profile_is_reserved_for_stricter_default_behavior() {
        let profile = PolicyProfile::Strict;

        assert_eq!(profile.as_str(), "strict");
        assert!(profile.includes_compatibility_rules());
        assert!(profile.includes_buildkit_native_rules());
        assert!(profile.includes_shell_catalog());
        assert!(profile.is_strict());
    }

    #[test]
    fn parses_project_native_inline_suppression_comments() {
        let suppression =
            InlineSuppression::parse_comment(12, "# rudolint ignore=rdl3000, RDK1001")
                .expect("suppression should parse");

        assert_eq!(suppression.line, 12);
        assert!(matches!(suppression.target, SuppressionTarget::Codes(_)));
        assert!(suppression.matches("RDL3000"));
        assert!(suppression.matches("RDK1001"));
        assert!(!suppression.matches("RDL3007"));
    }

    #[test]
    fn parses_project_native_all_suppression() {
        let suppression = InlineSuppression::parse_comment(3, "# rudolint ignore=all")
            .expect("suppression should parse");

        assert_eq!(suppression.target, SuppressionTarget::All);
        assert!(suppression.matches("RDL3000"));
        assert!(suppression.matches("RDK1003"));
    }

    #[test]
    fn ignores_unrelated_or_empty_suppression_comments() {
        assert!(InlineSuppression::parse_comment(1, "# hadolint ignore=DL3000").is_none());
        assert!(InlineSuppression::parse_comment(1, "# rudolint ignore=").is_none());
        assert!(InlineSuppression::parse_comment(1, "# regular comment").is_none());
    }
}
