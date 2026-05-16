use std::collections::BTreeSet;

use crate::{Rule, RuleInfo, metadata::diagnostic, metadata::rule_metadata};
use rudolint_diagnostics::{Finding, Severity};
use rudolint_dockerfile::{Comment, Dockerfile};
use rudolint_fix::{FixApplicability, FixPreview, TextEdit};
use rudolint_policy::LegacySuppression;

pub(crate) fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(InlineIgnore),
        Box::new(AbsoluteWorkdir),
        Box::new(LastUserNotRoot),
        Box::new(ExplicitFromTag),
        Box::new(NoLatestTag),
        Box::new(ValidExposePort),
        Box::new(SingleHealthcheck),
        Box::new(PreferCopy),
        Box::new(UniqueStageNames),
        Box::new(JsonEntrypoints),
        Box::new(DeprecatedMaintainer),
        Box::new(SingleCmd),
        Box::new(SingleEntrypoint),
    ]
}

rule_metadata!(
    InlineIgnore,
    "RDL1001",
    "legacy-external-suppression",
    Severity::Warning,
    "warn on legacy external linter suppression comments"
);

impl Rule for InlineIgnore {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.comments
            .iter()
            .filter_map(legacy_suppression_comment)
            .collect()
    }
}

fn legacy_suppression_comment(comment: &Comment) -> Option<Finding> {
    LegacySuppression::parse_comment(comment.line, &comment.text)?;
    Some(Finding::with_span(
        "RDL1001",
        Severity::Warning,
        "prefer native rudolint suppression comments over legacy external suppressions",
        comment.span,
    ))
}

rule_metadata!(
    AbsoluteWorkdir,
    "RDL3000",
    "absolute-workdir",
    Severity::Error,
    "require absolute WORKDIR paths"
);

impl Rule for AbsoluteWorkdir {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "WORKDIR")
            .filter(|instruction| {
                let path = instruction.args.trim_matches('"').trim_matches('\'');
                !(path.starts_with('/') || path.starts_with('$') || is_windows_absolute(path))
            })
            .map(|instruction| {
                diagnostic(
                    "RDL3000",
                    Severity::Error,
                    "WORKDIR should be absolute",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    LastUserNotRoot,
    "RDL3002",
    "final-user-not-root",
    Severity::Warning,
    "require the final USER to be non-root"
);

impl Rule for LastUserNotRoot {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        let Some(last_user) = doc
            .instructions
            .iter()
            .rev()
            .find(|instruction| instruction.keyword == "USER")
        else {
            return Vec::new();
        };
        let user = last_user.args.trim();
        if matches!(user, "root" | "0" | "0:0") {
            vec![diagnostic(
                "RDL3002",
                Severity::Warning,
                "the final image user should not be root",
                last_user,
            )]
        } else {
            Vec::new()
        }
    }
}

rule_metadata!(
    ExplicitFromTag,
    "RDL3006",
    "explicit-from-tag",
    Severity::Warning,
    "require explicit image tags in FROM"
);

impl Rule for ExplicitFromTag {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "FROM")
            .filter(|instruction| {
                instruction.base_image().is_some_and(|image| {
                    !image.contains('@') && !image.rsplit('/').next().unwrap_or("").contains(':')
                })
            })
            .map(|instruction| {
                diagnostic(
                    "RDL3006",
                    Severity::Warning,
                    "base image should use an explicit tag or digest",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    NoLatestTag,
    "RDL3007",
    "no-latest-tag",
    Severity::Warning,
    "reject latest base image tags"
);

impl Rule for NoLatestTag {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "FROM")
            .filter(|instruction| {
                instruction.base_image().is_some_and(|image| {
                    image.rsplit('/').next().unwrap_or("").ends_with(":latest")
                })
            })
            .map(|instruction| {
                diagnostic(
                    "RDL3007",
                    Severity::Warning,
                    "avoid mutable latest base image tags",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    ValidExposePort,
    "RDL3011",
    "valid-expose-port",
    Severity::Error,
    "validate EXPOSE port numbers"
);

impl Rule for ValidExposePort {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "EXPOSE")
            .flat_map(|instruction| {
                instruction
                    .args
                    .split_whitespace()
                    .filter_map(|port| {
                        let port = port.split('/').next().unwrap_or(port);
                        let valid = port.parse::<u32>().is_ok_and(|value| value <= 65535);
                        (!valid).then(|| {
                            diagnostic(
                                "RDL3011",
                                Severity::Error,
                                format!("invalid exposed port `{port}`"),
                                instruction,
                            )
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}

rule_metadata!(
    SingleHealthcheck,
    "RDL3012",
    "single-healthcheck",
    Severity::Error,
    "allow only one HEALTHCHECK instruction"
);

impl Rule for SingleHealthcheck {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        duplicates(
            doc,
            "HEALTHCHECK",
            "RDL3012",
            Severity::Error,
            "only one HEALTHCHECK is allowed",
        )
    }
}

rule_metadata!(
    PreferCopy,
    "RDL3020",
    "prefer-copy",
    Severity::Error,
    "prefer COPY for plain local files"
);

impl Rule for PreferCopy {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "ADD")
            .filter(|instruction| {
                let source = instruction.args.split_whitespace().next().unwrap_or("");
                !(source.starts_with("http://")
                    || source.starts_with("https://")
                    || source.ends_with(".tar")
                    || source.ends_with(".tar.gz")
                    || source.ends_with(".tgz"))
            })
            .map(|instruction| {
                diagnostic(
                    "RDL3020",
                    Severity::Error,
                    "use COPY for local files unless archive extraction or remote fetch is intended",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    UniqueStageNames,
    "RDL3024",
    "unique-stage-names",
    Severity::Error,
    "require unique multi-stage aliases"
);

impl Rule for UniqueStageNames {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        let mut seen = BTreeSet::new();
        let mut findings = Vec::new();
        for instruction in &doc.instructions {
            let Some(alias) = instruction.stage_alias() else {
                continue;
            };
            if !seen.insert(alias.clone()) {
                findings.push(diagnostic(
                    "RDL3024",
                    Severity::Error,
                    format!("stage alias `{alias}` is defined more than once"),
                    instruction,
                ));
            }
        }
        findings
    }
}

rule_metadata!(
    JsonEntrypoints,
    "RDL3025",
    "json-entrypoints",
    Severity::Warning,
    "prefer JSON form for CMD and ENTRYPOINT"
);

impl Rule for JsonEntrypoints {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| matches!(instruction.keyword.as_str(), "CMD" | "ENTRYPOINT"))
            .filter(|instruction| !instruction.args.trim_start().starts_with('['))
            .map(|instruction| {
                diagnostic(
                    "RDL3025",
                    Severity::Warning,
                    "use exec/JSON form for CMD and ENTRYPOINT",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    DeprecatedMaintainer,
    "RDL4000",
    "deprecated-maintainer",
    Severity::Error,
    "reject deprecated MAINTAINER instructions",
    crate::FixAvailability::Safe
);

impl Rule for DeprecatedMaintainer {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "MAINTAINER")
            .map(|instruction| {
                diagnostic(
                    "RDL4000",
                    Severity::Error,
                    "use OCI labels instead of MAINTAINER",
                    instruction,
                )
            })
            .collect()
    }

    fn fix(&self, doc: &Dockerfile) -> Vec<FixPreview> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "MAINTAINER")
            .filter_map(|instruction| {
                let maintainer = instruction.args.trim();
                if maintainer.is_empty() || maintainer.contains(['"', '\\']) {
                    return None;
                }
                Some(FixPreview {
                    title: "replace MAINTAINER with OCI authors label".to_string(),
                    applicability: FixApplicability::safe(),
                    edits: vec![TextEdit::replace(
                        rudolint_source::SourceSpan {
                            line: instruction.line,
                            column: 1,
                            length: instruction.raw.chars().count(),
                        },
                        format!("LABEL org.opencontainers.image.authors=\"{maintainer}\""),
                    )],
                })
            })
            .collect()
    }
}

rule_metadata!(
    SingleCmd,
    "RDL4003",
    "single-cmd",
    Severity::Warning,
    "allow only one CMD instruction"
);

impl Rule for SingleCmd {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        duplicates(
            doc,
            "CMD",
            "RDL4003",
            Severity::Warning,
            "only the final CMD is used",
        )
    }
}

rule_metadata!(
    SingleEntrypoint,
    "RDL4004",
    "single-entrypoint",
    Severity::Error,
    "allow only one ENTRYPOINT instruction"
);

impl Rule for SingleEntrypoint {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        duplicates(
            doc,
            "ENTRYPOINT",
            "RDL4004",
            Severity::Error,
            "only the final ENTRYPOINT is used",
        )
    }
}

fn duplicates(
    doc: &Dockerfile,
    keyword: &str,
    code: &'static str,
    severity: Severity,
    message: &'static str,
) -> Vec<Finding> {
    let mut seen = false;
    let mut findings = Vec::new();
    for instruction in doc
        .instructions
        .iter()
        .filter(|instruction| instruction.keyword == keyword)
    {
        if seen {
            findings.push(diagnostic(code, severity, message, instruction));
        }
        seen = true;
    }
    findings
}

fn is_windows_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'\\'
}

pub(crate) fn planned_catalog() -> Vec<&'static str> {
    vec![
        "RDL3001", "RDL3003", "RDL3004", "RDL3008", "RDL3009", "RDL3010", "RDL3013", "RDL3014",
        "RDL3015", "RDL3016", "RDL3018", "RDL3019", "RDL3021", "RDL3022", "RDL3023", "RDL3026",
        "RDL3027", "RDL3028", "RDL3029", "RDL3030", "RDL3032", "RDL3033", "RDL3034", "RDL3035",
        "RDL3036", "RDL3037", "RDL3038", "RDL3040", "RDL3041", "RDL3042", "RDL3043", "RDL3044",
        "RDL3045", "RDL3046", "RDL3047", "RDL3048", "RDL3049", "RDL3050", "RDL3051", "RDL3052",
        "RDL3053", "RDL3054", "RDL3055", "RDL3056", "RDL3057", "RDL3058", "RDL3059", "RDL3060",
        "RDL3061", "RDL3062", "RDL3063", "RDL4001", "RDL4005", "RDL4006",
    ]
}
