use crate::{Rule, RuleInfo, RuleMetadata, buildkit, compat, shell};
use rudolint_policy::PolicyProfile;

pub(crate) fn implemented_rules(profile: PolicyProfile) -> Vec<Box<dyn Rule>> {
    let mut rules = compat::rules();

    if profile.includes_buildkit_native_rules() {
        rules.extend(buildkit::rules());
    }

    rules
}

pub(crate) fn catalog(profile: PolicyProfile) -> Vec<RuleInfo> {
    let mut rules = implemented_rules(profile)
        .into_iter()
        .map(|rule| rule.info())
        .collect::<Vec<_>>();

    if profile.includes_compatibility_rules() {
        rules.extend(
            compat::planned_catalog()
                .into_iter()
                .map(RuleMetadata::planned_compat)
                .map(RuleInfo::from_metadata),
        );
    }

    if profile.includes_shell_catalog() {
        rules.extend(
            shell::catalog()
                .into_iter()
                .map(RuleMetadata::external_shell)
                .map(RuleInfo::from_metadata),
        );
    }

    rules.sort_by_key(|rule| rule.code);
    rules.dedup_by_key(|rule| rule.code);
    rules
}
