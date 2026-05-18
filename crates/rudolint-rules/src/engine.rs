use std::path::Path;

use rudolint_config::Config;
use rudolint_diagnostics::Finding;
use rudolint_dockerfile::{Comment, Dockerfile, Instruction};
use rudolint_fix::FixPreview;
use rudolint_policy::{InlineSuppression, PolicyProfile};

use crate::{Profile, Rule, RuleInfo, catalog};

/// Executes configured lint rules and fix providers for a policy profile.
pub struct RuleEngine {
    rules: Vec<Box<dyn Rule>>,
    policy: PolicyProfile,
    config: Config,
}

impl RuleEngine {
    /// Creates a rule engine for `profile` using the supplied configuration.
    pub fn new(profile: Profile, config: Config) -> Self {
        let policy = profile.policy();
        Self {
            rules: catalog::implemented_rules(policy),
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
        for rule in &self.rules {
            let info = rule.info();
            if !self.config.selects(info.code)
                || self.config.ignores(info.code)
                || path.is_some_and(|path| self.config.ignores_for_path(info.code, path))
            {
                continue;
            }
            findings.extend(
                rule.check_with_config(document, &self.config)
                    .into_iter()
                    .map(|mut finding| {
                        if let Some(severity) = self.config.severity_override(&finding.code) {
                            finding.severity = severity;
                        }
                        finding
                    }),
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
        for rule in &self.rules {
            let info = rule.info();
            if !self.config.selects(info.code)
                || self.config.ignores(info.code)
                || path.is_some_and(|path| self.config.ignores_for_path(info.code, path))
            {
                continue;
            }
            fixes.extend(rule.fix(document));
        }
        fixes
    }
}

#[derive(Debug, Clone)]
struct TargetedSuppression {
    instruction_line: usize,
    suppression: InlineSuppression,
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
    let suppression = InlineSuppression::parse_comment(comment.line, &comment.text)?;
    let instruction_line = instructions
        .iter()
        .find(|instruction| instruction.line > comment.line)?
        .line;

    Some(TargetedSuppression {
        instruction_line,
        suppression,
    })
}

fn is_suppressed(finding: &Finding, suppressions: &[TargetedSuppression]) -> bool {
    suppressions.iter().any(|suppression| {
        suppression.instruction_line == finding.line()
            && suppression.suppression.matches(&finding.code)
    })
}
