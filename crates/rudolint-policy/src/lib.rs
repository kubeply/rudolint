//! Rule selection, profiles, severity overrides, and compatibility policy.

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

#[cfg(test)]
mod tests {
    use super::PolicyProfile;

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
}
