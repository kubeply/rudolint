//! Rule selection, profiles, severity overrides, and compatibility policy.

use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PolicyProfile {
    #[default]
    Default,
    Compat,
    Strict,
}

impl PolicyProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Compat => "compat",
            Self::Strict => "strict",
        }
    }

    pub const fn includes_buildkit_native_rules(self) -> bool {
        match self {
            Self::Default | Self::Strict => true,
            Self::Compat => false,
        }
    }

    pub const fn includes_compatibility_rules(self) -> bool {
        true
    }

    pub const fn includes_shell_catalog(self) -> bool {
        true
    }

    pub const fn is_strict(self) -> bool {
        matches!(self, Self::Strict)
    }
}

pub type PolicyMode = PolicyProfile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacySuppressionTool {
    Hadolint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacySuppression {
    pub line: usize,
    pub tool: LegacySuppressionTool,
}

impl LegacySuppression {
    pub fn parse_comment(line: usize, text: &str) -> Option<Self> {
        let body = text.strip_prefix('#')?.trim_start();
        let mut fields = body.split_whitespace();
        let tool = match fields.next()? {
            value if value.eq_ignore_ascii_case("hadolint") => LegacySuppressionTool::Hadolint,
            _ => return None,
        };

        let ignored = fields.find_map(ignore_field)?;
        if ignored.trim().is_empty() {
            return None;
        }

        Some(Self { line, tool })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineSuppression {
    pub line: usize,
    pub target: SuppressionTarget,
}

impl InlineSuppression {
    pub fn parse_comment(line: usize, text: &str) -> Option<Self> {
        let body = text.strip_prefix('#')?.trim_start();
        let mut fields = body.split_whitespace();
        if !fields.next()?.eq_ignore_ascii_case("rudolint") {
            return None;
        }

        let ignored = fields.find_map(ignore_field)?;
        let target = SuppressionTarget::parse(ignored)?;

        Some(Self { line, target })
    }

    pub fn matches(&self, code: &str) -> bool {
        self.target.matches(code)
    }
}

fn ignore_field(field: &str) -> Option<&str> {
    let (name, value) = field.split_once('=')?;
    if name.eq_ignore_ascii_case("ignore") {
        Some(value)
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuppressionTarget {
    All,
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
    use super::{
        InlineSuppression, LegacySuppression, LegacySuppressionTool, PolicyProfile,
        SuppressionTarget,
    };

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
        let suppression = InlineSuppression::parse_comment(12, "# rudolint ignore=rdl3000,RDK1001")
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

    #[test]
    fn parses_legacy_hadolint_suppression_comments() {
        let suppression = LegacySuppression::parse_comment(8, "# hadolint ignore=DL3007")
            .expect("legacy suppression should parse");

        assert_eq!(suppression.line, 8);
        assert_eq!(suppression.tool, LegacySuppressionTool::Hadolint);
    }

    #[test]
    fn ignores_unrelated_or_empty_legacy_suppression_comments() {
        assert!(LegacySuppression::parse_comment(1, "# rudolint ignore=RDL3000").is_none());
        assert!(LegacySuppression::parse_comment(1, "# hadolint ignore=").is_none());
        assert!(LegacySuppression::parse_comment(1, "# regular comment").is_none());
    }
}
