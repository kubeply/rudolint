use crate::{
    Rule, RuleInfo, RuleMetadata, buildkit, compat, metadata::profile_includes_code, shell,
};
use rudolint_policy::PolicyProfile;

pub(crate) fn implemented_rules(profile: PolicyProfile) -> Vec<Box<dyn Rule>> {
    let mut rules = compat::rules();
    if !profile.warns_on_legacy_suppressions() {
        rules.retain(|rule| rule.info().code != "RUD1001");
    }

    if profile.includes_buildkit_native_rules() {
        rules.extend(buildkit::rules());
    }

    if profile.is_signal_profile() {
        rules.retain(|rule| profile_includes_code(profile, rule.info().code));
    }

    rules
}

pub(crate) fn catalog(profile: PolicyProfile) -> Vec<RuleInfo> {
    let mut rules = implemented_rules(profile)
        .into_iter()
        .map(|rule| rule.info())
        .collect::<Vec<_>>();

    if profile.includes_compatibility_rules() && !profile.is_signal_profile() {
        rules.extend(
            compat::planned_catalog()
                .into_iter()
                .map(RuleMetadata::planned_compat)
                .map(RuleInfo::from_metadata),
        );
    }

    if profile.includes_shell_catalog() {
        rules.extend(
            shell::implemented_catalog()
                .into_iter()
                .filter(|rule| profile_includes_code(profile, rule.code)),
        );
        if !profile.is_signal_profile() {
            rules.extend(
                shell::planned_catalog()
                    .into_iter()
                    .map(RuleMetadata::planned_shell)
                    .map(RuleInfo::from_metadata),
            );
        }
    }

    rules.sort_by_key(|rule| rule.code);
    rules.dedup_by_key(|rule| rule.code);
    rules
}
