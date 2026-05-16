use crate::{Rule, RuleInfo, RuleStatus, buildkit, compat, shell};
use rudolint_diagnostics::Severity;
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
        rules.extend(compat::planned_catalog().into_iter().map(|code| RuleInfo {
            code,
            severity: Severity::Warning,
            summary: "tracked for compatibility parity",
            status: RuleStatus::Planned,
        }));
    }

    if profile.includes_shell_catalog() {
        rules.extend(shell::catalog().into_iter().map(|code| RuleInfo {
            code,
            severity: Severity::Warning,
            summary: "shell diagnostics delegated to the shell-analysis layer",
            status: RuleStatus::External,
        }));
    }

    rules.sort_by_key(|rule| rule.code);
    rules.dedup_by_key(|rule| rule.code);
    rules
}
