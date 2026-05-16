//! Rule selection, profiles, severity overrides, and compatibility policy.

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
