use std::path::Path;

use rudolint_config::Config;
use rudolint_diagnostics::Finding;
use rudolint_dockerfile::{Comment, Dockerfile, Instruction};
use rudolint_fix::FixPreview;
use rudolint_policy::{InlineSuppression, LegacySuppression, PolicyProfile};

use crate::{Profile, Rule, RuleInfo, catalog, metadata::profile_includes_code, shell};

/// Executes configured lint rules and fix providers for a policy profile.
pub struct RuleEngine {
    rules: Vec<EnabledRule>,
    policy: PolicyProfile,
    config: Config,
}

struct EnabledRule {
    code: &'static str,
    rule: Box<dyn Rule>,
}

impl RuleEngine {
    /// Creates a rule engine for `profile` using the supplied configuration.
    pub fn new(profile: Profile, config: Config) -> Self {
        let policy = profile.policy();
        let rules = catalog::implemented_rules(policy)
            .into_iter()
            .map(|rule| EnabledRule {
                code: rule.info().code,
                rule,
            })
            .collect();
        Self {
            rules,
            policy,
            config,
        }
    }

    /// Returns diagnostics emitted by all enabled rules for `document`.
    pub fn lint(&self, document: &Dockerfile) -> Vec<Finding> {
        self.lint_inner(None, document)
    }

    /// Returns diagnostics emitted by all enabled rules for `document` at `path`.
    pub fn lint_path(&self, path: &Path, document: &Dockerfile) -> Vec<Finding> {
        self.lint_inner(Some(path), document)
    }

    fn lint_inner(&self, path: Option<&Path>, document: &Dockerfile) -> Vec<Finding> {
        let suppressions = targeted_suppressions(document);
        let mut findings = Vec::new();
        for enabled in &self.rules {
            if !self.config.selects(enabled.code)
                || self.config.ignores(enabled.code)
                || path.is_some_and(|path| self.config.ignores_for_path(enabled.code, path))
            {
                continue;
            }
            findings.extend(
                enabled
                    .rule
                    .check_with_config(document, &self.config)
                    .into_iter()
                    .map(|mut finding| {
                        if let Some(severity) = self.config.severity_override(&finding.code) {
                            finding.severity = severity;
                        }
                        finding
                    }),
            );
        }
        if self.policy.includes_shell_catalog() {
            findings.extend(
                shell::lint(document, &self.config, path)
                    .into_iter()
                    .map(|mut finding| {
                        if let Some(severity) = self.config.severity_override(&finding.code) {
                            finding.severity = severity;
                        }
                        finding
                    })
                    .filter(|finding| profile_includes_code(self.policy, &finding.code)),
            );
        }
        findings.retain(|finding| !is_suppressed(finding, &suppressions));
        findings.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then(left.line().cmp(&right.line()))
                .then(left.column().cmp(&right.column()))
                .then(left.code.cmp(&right.code))
        });
        findings
    }

    /// Returns catalog metadata for rules in this engine's policy profile.
    pub fn catalog(&self) -> Vec<RuleInfo> {
        catalog::catalog(self.policy)
    }

    /// Returns fix previews emitted by all enabled rules for `document`.
    pub fn fixes(&self, document: &Dockerfile) -> Vec<FixPreview> {
        self.fixes_inner(None, document)
    }

    /// Returns fix previews emitted by all enabled rules for `document` at `path`.
    pub fn fixes_path(&self, path: &Path, document: &Dockerfile) -> Vec<FixPreview> {
        self.fixes_inner(Some(path), document)
    }

    fn fixes_inner(&self, path: Option<&Path>, document: &Dockerfile) -> Vec<FixPreview> {
        let mut fixes = Vec::new();
        for enabled in &self.rules {
            if !self.config.selects(enabled.code)
                || self.config.ignores(enabled.code)
                || path.is_some_and(|path| self.config.ignores_for_path(enabled.code, path))
            {
                continue;
            }
            fixes.extend(enabled.rule.fix(document));
        }
        fixes
    }
}

#[derive(Debug, Clone)]
struct TargetedSuppression {
    instruction_start_line: usize,
    instruction_end_line: usize,
    target: Suppression,
}

#[derive(Debug, Clone)]
enum Suppression {
    Native(InlineSuppression),
    Legacy(LegacySuppression),
}

fn targeted_suppressions(document: &Dockerfile) -> Vec<TargetedSuppression> {
    document
        .comments
        .iter()
        .filter_map(|comment| targeted_suppression(comment, &document.instructions))
        .collect()
}

fn targeted_suppression(
    comment: &Comment,
    instructions: &[Instruction],
) -> Option<TargetedSuppression> {
    let target = InlineSuppression::parse_comment(comment.line, &comment.text)
        .map(Suppression::Native)
        .or_else(|| {
            LegacySuppression::parse_comment(comment.line, &comment.text).map(Suppression::Legacy)
        })?;
    let instruction = instructions
        .iter()
        .find(|instruction| instruction.line > comment.line)?;
    let instruction_start_line = instruction.line;
    let instruction_end_line =
        instruction_start_line + instruction.raw.lines().count().saturating_sub(1);

    Some(TargetedSuppression {
        instruction_start_line,
        instruction_end_line,
        target,
    })
}

fn is_suppressed(finding: &Finding, suppressions: &[TargetedSuppression]) -> bool {
    suppressions.iter().any(|suppression| {
        (suppression.instruction_start_line..=suppression.instruction_end_line)
            .contains(&finding.line())
            && match &suppression.target {
                Suppression::Native(native) => native.matches(&finding.code),
                Suppression::Legacy(legacy) => legacy.matches(&finding.code),
            }
    })
}
