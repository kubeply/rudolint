use std::collections::BTreeSet;

use crate::{Rule, RuleInfo, metadata::diagnostic, metadata::rule_metadata};
use rudolint_diagnostics::{Finding, Severity};
use rudolint_dockerfile::{Comment, Dockerfile};
use rudolint_fix::{FixApplicability, FixPreview, TextEdit};
use rudolint_policy::LegacySuppression;
use rudolint_shell::{detect_command_invocations, detect_disallowed_container_commands};

pub(crate) fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(InlineIgnore),
        Box::new(DisallowedContainerCommands),
        Box::new(AbsoluteWorkdir),
        Box::new(LastUserNotRoot),
        Box::new(UseWorkdirForCd),
        Box::new(NoSudo),
        Box::new(ExplicitFromTag),
        Box::new(NoLatestTag),
        Box::new(PinAptGetInstallVersions),
        Box::new(CleanAptLists),
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
    DisallowedContainerCommands,
    "RDL3001",
    "disallowed-container-commands",
    Severity::Info,
    "avoid commands that rarely make sense during Docker builds"
);

impl Rule for DisallowedContainerCommands {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .flat_map(|instruction| {
                detect_disallowed_container_commands(&instruction.args)
                    .into_iter()
                    .map(|command| {
                        diagnostic(
                            "RDL3001",
                            Severity::Info,
                            format!(
                                "avoid running `{}` in Docker build containers",
                                command.as_str()
                            ),
                            instruction,
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }
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
    UseWorkdirForCd,
    "RDL3003",
    "use-workdir-for-cd",
    Severity::Warning,
    "prefer WORKDIR over RUN cd"
);

impl Rule for UseWorkdirForCd {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| {
                detect_command_invocations(&instruction.args)
                    .into_iter()
                    .any(|invocation| invocation.command == "cd")
            })
            .map(|instruction| {
                diagnostic(
                    "RDL3003",
                    Severity::Warning,
                    "use WORKDIR instead of RUN cd",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    NoSudo,
    "RDL3004",
    "no-sudo",
    Severity::Error,
    "avoid sudo in Docker builds"
);

impl Rule for NoSudo {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| {
                detect_command_invocations(&instruction.args)
                    .into_iter()
                    .any(|invocation| invocation.command == "sudo")
            })
            .map(|instruction| {
                diagnostic(
                    "RDL3004",
                    Severity::Error,
                    "avoid sudo in Docker builds; use USER or a privilege-drop tool instead",
                    instruction,
                )
            })
            .collect()
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
        let mut stage_aliases = BTreeSet::new();
        let mut findings = Vec::new();

        for instruction in doc
            .instructions
            .iter()
            .filter(|instruction| instruction.keyword == "FROM")
        {
            let Some(image) = instruction.base_image() else {
                continue;
            };

            if image_needs_explicit_tag(image, &stage_aliases) {
                findings.push(diagnostic(
                    "RDL3006",
                    Severity::Warning,
                    "base image should use an explicit tag or digest",
                    instruction,
                ));
            }

            if let Some(alias) = instruction.stage_alias() {
                stage_aliases.insert(alias.to_ascii_lowercase());
            }
        }

        findings
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
    PinAptGetInstallVersions,
    "RDL3008",
    "pin-apt-get-install-versions",
    Severity::Warning,
    "pin versions in apt-get install"
);

impl Rule for PinAptGetInstallVersions {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| apt_get_install_has_unpinned_packages(&instruction.args))
            .map(|instruction| {
                diagnostic(
                    "RDL3008",
                    Severity::Warning,
                    "pin versions in apt-get install",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    CleanAptLists,
    "RDL3009",
    "clean-apt-lists",
    Severity::Info,
    "delete apt-get package lists after use"
);

impl Rule for CleanAptLists {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| apt_get_uses_package_lists(&instruction.args))
            .filter(|instruction| !removes_apt_lists(&instruction.args))
            .map(|instruction| {
                diagnostic(
                    "RDL3009",
                    Severity::Info,
                    "delete apt-get package lists after use",
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
                let sources = add_sources(&instruction.args);
                !sources
                    .iter()
                    .all(|source| is_url_source(source) || is_archive_source(source))
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
    "prefer JSON form for CMD and ENTRYPOINT",
    crate::FixAvailability::Manual
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

    fn fix(&self, doc: &Dockerfile) -> Vec<FixPreview> {
        doc.instructions
            .iter()
            .filter(|instruction| matches!(instruction.keyword.as_str(), "CMD" | "ENTRYPOINT"))
            .filter(|instruction| !instruction.args.trim_start().starts_with('['))
            .map(|instruction| FixPreview {
                title: format!("convert {} to exec/JSON form", instruction.keyword),
                applicability: FixApplicability::manual(),
                edits: Vec::new(),
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

fn image_needs_explicit_tag(image: &str, stage_aliases: &BTreeSet<String>) -> bool {
    if image == "scratch"
        || image.contains('@')
        || stage_aliases.contains(&image.to_ascii_lowercase())
    {
        return false;
    }

    !image.rsplit('/').next().unwrap_or("").contains(':')
}

fn add_sources(args: &str) -> Vec<&str> {
    let mut parts = args
        .split_whitespace()
        .filter(|part| !part.starts_with("--"))
        .collect::<Vec<_>>();
    if parts.len() <= 1 {
        return Vec::new();
    }

    parts.pop();
    parts
}

fn is_url_source(source: &str) -> bool {
    let source = normalized_add_source(source);
    source.starts_with("http://") || source.starts_with("https://")
}

fn is_archive_source(source: &str) -> bool {
    let source = normalized_add_source(source).to_ascii_lowercase();
    [
        ".tar", ".tar.gz", ".tgz", ".tar.bz2", ".tbz2", ".tar.xz", ".txz",
    ]
    .iter()
    .any(|suffix| source.ends_with(suffix))
}

fn normalized_add_source(source: &str) -> String {
    source
        .trim_matches(|character| matches!(character, '"' | '\'' | '[' | ']' | ','))
        .replace("\\\"", "\"")
        .replace("\\\\", "\\")
}

fn apt_get_install_has_unpinned_packages(shell: &str) -> bool {
    detect_command_invocations(shell)
        .into_iter()
        .filter(|invocation| invocation.command == "apt-get")
        .any(|invocation| {
            let Some(install_index) = invocation
                .arguments
                .iter()
                .position(|argument| argument == "install")
            else {
                return false;
            };

            let mut expect_option_value = false;
            for argument in invocation.arguments.iter().skip(install_index + 1) {
                if argument == "\\" {
                    continue;
                }

                if expect_option_value {
                    expect_option_value = false;
                    continue;
                }

                if apt_get_install_option_takes_value(argument) {
                    expect_option_value = true;
                    continue;
                }

                if argument.starts_with('-') {
                    continue;
                }

                if !argument.contains('=') {
                    return true;
                }
            }

            false
        })
}

fn apt_get_install_option_takes_value(argument: &str) -> bool {
    matches!(
        argument,
        "-t" | "--target-release" | "-o" | "--option" | "-c" | "--config-file"
    )
}

fn apt_get_uses_package_lists(shell: &str) -> bool {
    detect_command_invocations(shell)
        .into_iter()
        .filter(|invocation| invocation.command == "apt-get")
        .any(|invocation| {
            invocation
                .arguments
                .iter()
                .find(|argument| !argument.starts_with('-') && argument.as_str() != "\\")
                .is_some_and(|subcommand| matches!(subcommand.as_str(), "update" | "install"))
        })
}

fn removes_apt_lists(shell: &str) -> bool {
    detect_command_invocations(shell)
        .into_iter()
        .filter(|invocation| invocation.command == "rm")
        .flat_map(|invocation| invocation.arguments)
        .any(|argument| {
            let value = argument.trim_matches(|character| matches!(character, '"' | '\''));
            value == "/var/lib/apt/lists"
                || value == "/var/lib/apt/lists/*"
                || value.starts_with("/var/lib/apt/lists/")
        })
}

pub(crate) fn planned_catalog() -> Vec<&'static str> {
    vec![
        "RDL3010", "RDL3013", "RDL3014", "RDL3015", "RDL3016", "RDL3018", "RDL3019", "RDL3021",
        "RDL3022", "RDL3023", "RDL3026", "RDL3027", "RDL3028", "RDL3029", "RDL3030", "RDL3032",
        "RDL3033", "RDL3034", "RDL3035", "RDL3036", "RDL3037", "RDL3038", "RDL3040", "RDL3041",
        "RDL3042", "RDL3043", "RDL3044", "RDL3045", "RDL3046", "RDL3047", "RDL3048", "RDL3049",
        "RDL3050", "RDL3051", "RDL3052", "RDL3053", "RDL3054", "RDL3055", "RDL3056", "RDL3057",
        "RDL3058", "RDL3059", "RDL3060", "RDL3061", "RDL3062", "RDL3063", "RDL4001", "RDL4005",
        "RDL4006",
    ]
}
