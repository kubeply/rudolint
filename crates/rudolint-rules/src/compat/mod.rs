use std::collections::{BTreeMap, BTreeSet};

use crate::{Rule, RuleInfo, metadata::diagnostic, metadata::rule_metadata};
use rudolint_config::Config;
use rudolint_diagnostics::{Finding, Severity};
use rudolint_dockerfile::{Comment, CopyKind, Dockerfile, Instruction, InstructionForm};
use rudolint_fix::{FixApplicability, FixPreview, TextEdit};
use rudolint_policy::{LegacySuppression, PolicyProfile};
use rudolint_shell::{detect_command_invocations, detect_disallowed_container_commands};

pub(super) fn rules() -> Vec<Box<dyn Rule>> {
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
        Box::new(UseAddForArchives),
        Box::new(ValidExposePort),
        Box::new(SingleHealthcheck),
        Box::new(PinPipVersions),
        Box::new(AptGetInstallAssumeYes),
        Box::new(AptGetNoInstallRecommends),
        Box::new(PinNpmVersions),
        Box::new(PinApkVersions),
        Box::new(ApkAddNoCache),
        Box::new(PreferCopy),
        Box::new(CopyMultipleDestinationSlash),
        Box::new(CopyFromPreviousStage),
        Box::new(CopyFromOwnStage),
        Box::new(UniqueStageNames),
        Box::new(JsonEntrypoints),
        Box::new(TrustedRegistries),
        Box::new(UseAptGet),
        Box::new(PinGemVersions),
        Box::new(NoFromPlatformFlag),
        Box::new(YumInstallAssumeYes),
        Box::new(YumCleanAll),
        Box::new(PinYumVersions),
        Box::new(ZypperInstallAssumeYes),
        Box::new(NoZypperDistUpgrade),
        Box::new(ZypperClean),
        Box::new(PinZypperVersions),
        Box::new(DnfInstallAssumeYes),
        Box::new(DnfCleanAll),
        Box::new(PinDnfVersions),
        Box::new(PipNoCacheDir),
        Box::new(NoOnbuildTrigger),
        Box::new(NoEnvSelfReference),
        Box::new(CopyRelativeWithoutWorkdir),
        Box::new(UseraddNoLogInit),
        Box::new(WgetProgress),
        Box::new(ValidLabelKey),
        Box::new(MissingRequiredLabels),
        Box::new(NoSuperfluousLabels),
        Box::new(NoEmptyLabels),
        Box::new(ValidUrlLabels),
        Box::new(ValidRfc3339Labels),
        Box::new(ValidSpdxLabels),
        Box::new(ValidGitHashLabels),
        Box::new(ValidSemverLabels),
        Box::new(MissingHealthcheck),
        Box::new(ValidEmailLabels),
        Box::new(ConsecutiveRun),
        Box::new(YarnCacheClean),
        Box::new(InstructionOrder),
        Box::new(PinGoVersions),
        Box::new(ReservedStageName),
        Box::new(DeprecatedMaintainer),
        Box::new(EitherWgetOrCurl),
        Box::new(UseShellForDefaultShell),
        Box::new(PipefailBeforePipe),
        Box::new(SingleCmd),
        Box::new(SingleEntrypoint),
    ]
}

rule_metadata!(
    InlineIgnore,
    "RUD1001",
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
        "RUD1001",
        Severity::Warning,
        "prefer native rudolint suppression comments over legacy external suppressions",
        comment.span,
    ))
}

rule_metadata!(
    DisallowedContainerCommands,
    "DL3001",
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
                            "DL3001",
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
    "DL3000",
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
                    "DL3000",
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
    "DL3002",
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
                "DL3002",
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
    "DL3003",
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
                    .any(|invocation| invocation.command_is("cd"))
            })
            .map(|instruction| {
                diagnostic(
                    "DL3003",
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
    "DL3004",
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
                    .any(|invocation| invocation.command_is("sudo"))
            })
            .map(|instruction| {
                diagnostic(
                    "DL3004",
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
    "DL3006",
    "explicit-from-tag",
    Severity::Warning,
    "require explicit image tags in FROM"
);

impl Rule for ExplicitFromTag {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        let global_args = global_arg_defaults(doc);
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

            let resolved_image = resolve_from_arg_image(image, &global_args).unwrap_or(image);

            if image_needs_explicit_tag(resolved_image, &stage_aliases) {
                findings.push(diagnostic(
                    "DL3006",
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
    "DL3007",
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
                    "DL3007",
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
    "DL3008",
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
                    "DL3008",
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
    "DL3009",
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
                    "DL3009",
                    Severity::Info,
                    "delete apt-get package lists after use",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    UseAddForArchives,
    "DL3010",
    "use-add-for-archives",
    Severity::Info,
    "use ADD for extracting local archives"
);

impl Rule for UseAddForArchives {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| {
                instruction.copy.as_ref().is_some_and(|copy| {
                    copy.kind == CopyKind::Copy
                        && copy.from.is_none()
                        && copy.sources.iter().any(|source| is_archive_source(source))
                })
            })
            .map(|instruction| {
                diagnostic(
                    "DL3010",
                    Severity::Info,
                    "use ADD when local archive extraction is intended",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    ValidExposePort,
    "DL3011",
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
                    .expose
                    .iter()
                    .flat_map(|expose| expose.ports.iter())
                    .filter_map(|exposed| {
                        let port = exposed.port.as_str();
                        let valid = is_valid_expose_port(port);
                        (!valid).then(|| {
                            diagnostic(
                                "DL3011",
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

fn is_valid_expose_port(port: &str) -> bool {
    is_continuation_marker(port)
        || is_dockerfile_variable_reference(port)
        || is_valid_port_number(port)
        || is_valid_port_range(port)
}

fn is_continuation_marker(value: &str) -> bool {
    value == "\\"
}

fn is_valid_port_range(value: &str) -> bool {
    let Some((start, end)) = value.split_once('-') else {
        return false;
    };

    match (parse_port_bound(start), parse_port_bound(end)) {
        (Some(PortBound::Number(start)), Some(PortBound::Number(end))) => start <= end,
        (Some(_), Some(_)) => true,
        _ => false,
    }
}

fn parse_port_bound(value: &str) -> Option<PortBound> {
    if is_dockerfile_variable_reference(value) {
        Some(PortBound::Variable)
    } else {
        value
            .parse::<u32>()
            .ok()
            .filter(|value| *value <= 65535)
            .map(PortBound::Number)
    }
}

fn is_valid_port_number(value: &str) -> bool {
    // TCP and UDP ports are 16-bit values, so 65535 is the upper bound.
    value.parse::<u32>().is_ok_and(|value| value <= 65535)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PortBound {
    Number(u32),
    Variable,
}

fn is_dockerfile_variable_reference(value: &str) -> bool {
    let value = value.trim_matches(|character| matches!(character, '\'' | '"'));
    if let Some(name) = value
        .strip_prefix("${")
        .and_then(|rest| rest.strip_suffix('}'))
    {
        return is_valid_variable_name(name);
    }

    value.strip_prefix('$').is_some_and(is_valid_variable_name)
}

fn is_valid_variable_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphanumeric())
}

rule_metadata!(
    SingleHealthcheck,
    "DL3012",
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
            "DL3012",
            Severity::Error,
            "only one HEALTHCHECK is allowed",
            false,
        )
    }
}

rule_metadata!(
    PinPipVersions,
    "DL3013",
    "pin-pip-versions",
    Severity::Warning,
    "pin versions in pip install"
);

impl Rule for PinPipVersions {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| pip_install_has_unpinned_packages(&instruction.args))
            .map(|instruction| {
                diagnostic(
                    "DL3013",
                    Severity::Warning,
                    "pin versions in pip install",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    AptGetInstallAssumeYes,
    "DL3014",
    "apt-get-install-assume-yes",
    Severity::Warning,
    "use -y with apt-get install"
);

impl Rule for AptGetInstallAssumeYes {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| apt_get_install_missing_yes(&instruction.args))
            .map(|instruction| {
                diagnostic(
                    "DL3014",
                    Severity::Warning,
                    "use -y with apt-get install",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    AptGetNoInstallRecommends,
    "DL3015",
    "apt-get-no-install-recommends",
    Severity::Info,
    "avoid recommended packages in apt-get install"
);

impl Rule for AptGetNoInstallRecommends {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| apt_get_install_missing_no_install_recommends(&instruction.args))
            .map(|instruction| {
                diagnostic(
                    "DL3015",
                    Severity::Info,
                    "use --no-install-recommends with apt-get install",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    PinNpmVersions,
    "DL3016",
    "pin-npm-versions",
    Severity::Warning,
    "pin versions in npm install"
);

impl Rule for PinNpmVersions {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| npm_install_has_unpinned_packages(&instruction.args))
            .map(|instruction| {
                diagnostic(
                    "DL3016",
                    Severity::Warning,
                    "pin versions in npm install",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    PinApkVersions,
    "DL3018",
    "pin-apk-versions",
    Severity::Warning,
    "pin versions in apk add"
);

impl Rule for PinApkVersions {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| apk_add_has_unpinned_packages(&instruction.args))
            .map(|instruction| {
                diagnostic(
                    "DL3018",
                    Severity::Warning,
                    "pin versions in apk add",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    ApkAddNoCache,
    "DL3019",
    "apk-add-no-cache",
    Severity::Info,
    "use --no-cache with apk add"
);

impl Rule for ApkAddNoCache {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| apk_add_missing_no_cache(&instruction.args))
            .map(|instruction| {
                diagnostic(
                    "DL3019",
                    Severity::Info,
                    "use `apk add --no-cache` to avoid persisting the apk package cache",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    PreferCopy,
    "DL3020",
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
            .filter(|instruction| {
                instruction.copy.as_ref().is_some_and(|copy| {
                    copy.kind == CopyKind::Add
                        && !copy
                            .sources
                            .iter()
                            .all(|source| is_url_source(source) || is_archive_source(source))
                })
            })
            .map(|instruction| {
                diagnostic(
                    "DL3020",
                    Severity::Error,
                    "use COPY for local files unless archive extraction or remote fetch is intended",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    CopyMultipleDestinationSlash,
    "DL3021",
    "copy-multiple-destination-slash",
    Severity::Error,
    "require trailing slash for COPY destinations with multiple sources"
);

impl Rule for CopyMultipleDestinationSlash {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| copy_multiple_sources_without_directory_destination(instruction))
            .map(|instruction| {
                diagnostic(
                    "DL3021",
                    Severity::Error,
                    "`COPY` with multiple sources requires the destination to end with `/`",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    CopyFromPreviousStage,
    "DL3022",
    "copy-from-previous-stage",
    Severity::Warning,
    "require COPY --from references to resolve to previous build stages"
);

impl Rule for CopyFromPreviousStage {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        let mut previous_aliases = BTreeSet::new();
        let mut current_alias = None;
        let mut stage_count = 0usize;
        let mut findings = Vec::new();

        for instruction in &doc.instructions {
            if instruction.keyword == "FROM" {
                if let Some(alias) = current_alias.take() {
                    previous_aliases.insert(alias);
                }
                stage_count += 1;
                current_alias = instruction
                    .stage_alias()
                    .map(|alias| alias.to_ascii_lowercase());
                continue;
            }

            if copy_from_reference_is_unresolved(instruction, &previous_aliases, stage_count) {
                findings.push(diagnostic(
                    "DL3022",
                    Severity::Warning,
                    "`COPY --from` should reference a previously defined `FROM` alias",
                    instruction,
                ));
            }
        }

        findings
    }
}

rule_metadata!(
    CopyFromOwnStage,
    "DL3023",
    "copy-from-own-stage",
    Severity::Error,
    "forbid COPY --from references to the current stage"
);

impl Rule for CopyFromOwnStage {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        let mut current_alias = None::<String>;
        let mut current_stage_index = None::<usize>;
        let mut stage_count = 0usize;
        let mut findings = Vec::new();

        for instruction in &doc.instructions {
            if instruction.keyword == "FROM" {
                current_alias = instruction.stage_alias();
                current_stage_index = Some(stage_count);
                stage_count += 1;
                continue;
            }

            if copy_from_references_current_stage(
                instruction,
                current_alias.as_deref(),
                current_stage_index,
            ) {
                findings.push(diagnostic(
                    "DL3023",
                    Severity::Error,
                    "`COPY --from` cannot reference the current build stage",
                    instruction,
                ));
            }
        }

        findings
    }
}

rule_metadata!(
    UniqueStageNames,
    "DL3024",
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
                    "DL3024",
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
    "DL3025",
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
                    "DL3025",
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
    TrustedRegistries,
    "DL3026",
    "trusted-registries",
    Severity::Error,
    "restrict FROM images to trusted registries"
);

impl Rule for TrustedRegistries {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, _doc: &Dockerfile) -> Vec<Finding> {
        Vec::new()
    }

    fn check_with_config(&self, doc: &Dockerfile, config: &Config) -> Vec<Finding> {
        if config.trusted_registries.is_empty() {
            return Vec::new();
        }

        let mut stage_aliases = BTreeSet::new();
        let mut findings = Vec::new();
        for instruction in &doc.instructions {
            if instruction.keyword != "FROM" {
                continue;
            }
            let Some(from) = &instruction.from else {
                continue;
            };

            if !from_image_uses_trusted_registry(&from.image, &stage_aliases, config) {
                findings.push(diagnostic(
                    "DL3026",
                    Severity::Error,
                    format!("base image `{}` is not from a trusted registry", from.image),
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
    UseAptGet,
    "DL3027",
    "use-apt-get",
    Severity::Warning,
    "prefer apt-get or apt-cache over apt"
);

impl Rule for UseAptGet {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| shell_uses_apt(&instruction.args))
            .map(|instruction| {
                diagnostic(
                    "DL3027",
                    Severity::Warning,
                    "use apt-get or apt-cache instead of apt in Docker builds",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    PinGemVersions,
    "DL3028",
    "pin-gem-versions",
    Severity::Warning,
    "pin versions in gem install"
);

impl Rule for PinGemVersions {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| gem_install_has_unpinned_packages(&instruction.args))
            .map(|instruction| {
                diagnostic(
                    "DL3028",
                    Severity::Warning,
                    "pin versions in gem install",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    NoFromPlatformFlag,
    "DL3029",
    "no-from-platform-flag",
    Severity::Warning,
    "avoid --platform in FROM"
);

impl Rule for NoFromPlatformFlag {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        no_from_platform_flag_findings(doc, PolicyProfile::HadolintCompat)
    }

    fn check_with_policy(
        &self,
        doc: &Dockerfile,
        _config: &Config,
        policy: PolicyProfile,
    ) -> Vec<Finding> {
        no_from_platform_flag_findings(doc, policy)
    }
}

fn no_from_platform_flag_findings(doc: &Dockerfile, policy: PolicyProfile) -> Vec<Finding> {
    doc.instructions
        .iter()
        .filter(|instruction| instruction.keyword_is("FROM"))
        .filter(|instruction| from_platform_flag_is_diagnostic(instruction, policy))
        .map(|instruction| {
            diagnostic(
                "DL3029",
                Severity::Warning,
                "avoid `--platform` in FROM; prefer build-time platform selection",
                instruction,
            )
        })
        .collect()
}

fn from_platform_flag_is_diagnostic(instruction: &Instruction, policy: PolicyProfile) -> bool {
    let Some(platform) = instruction.flag_value("platform") else {
        return false;
    };

    policy == PolicyProfile::HadolintCompat || !platform_uses_build_variable(platform)
}

fn platform_uses_build_variable(platform: &str) -> bool {
    platform.contains('$')
}

rule_metadata!(
    YumInstallAssumeYes,
    "DL3030",
    "yum-install-assume-yes",
    Severity::Warning,
    "use -y with yum install"
);

impl Rule for YumInstallAssumeYes {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| yum_install_missing_yes(&instruction.args))
            .map(|instruction| {
                diagnostic(
                    "DL3030",
                    Severity::Warning,
                    "use `yum install -y` to avoid interactive prompts",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    YumCleanAll,
    "DL3032",
    "yum-clean-all",
    Severity::Info,
    "clean yum metadata after installs"
);

impl Rule for YumCleanAll {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| yum_install_missing_clean_all(&instruction.args))
            .map(|instruction| {
                diagnostic(
                    "DL3032",
                    Severity::Info,
                    "clean yum metadata with `yum clean all` in the same RUN layer",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    PinYumVersions,
    "DL3033",
    "pin-yum-versions",
    Severity::Warning,
    "pin versions in yum install"
);

impl Rule for PinYumVersions {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| yum_install_has_unpinned_packages(&instruction.args))
            .map(|instruction| {
                diagnostic(
                    "DL3033",
                    Severity::Warning,
                    "pin versions in yum install",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    ZypperInstallAssumeYes,
    "DL3034",
    "zypper-install-assume-yes",
    Severity::Warning,
    "use a non-interactive flag with zypper install"
);

impl Rule for ZypperInstallAssumeYes {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| zypper_install_missing_yes(&instruction.args))
            .map(|instruction| {
                diagnostic(
                    "DL3034",
                    Severity::Warning,
                    "use `-y`, `-n`, or `--non-interactive` with `zypper install` to avoid interactive prompts",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    NoZypperDistUpgrade,
    "DL3035",
    "no-zypper-dist-upgrade",
    Severity::Warning,
    "avoid zypper dist-upgrade"
);

impl Rule for NoZypperDistUpgrade {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| shell_uses_zypper_dist_upgrade(&instruction.args))
            .map(|instruction| {
                diagnostic(
                    "DL3035",
                    Severity::Warning,
                    "avoid `zypper dist-upgrade` in Docker builds",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    ZypperClean,
    "DL3036",
    "zypper-clean",
    Severity::Warning,
    "clean zypper metadata after use"
);

impl Rule for ZypperClean {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| zypper_use_missing_clean(&instruction.args))
            .map(|instruction| {
                diagnostic(
                    "DL3036",
                    Severity::Warning,
                    "clean zypper metadata with `zypper clean` in the same RUN layer",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    PinZypperVersions,
    "DL3037",
    "pin-zypper-versions",
    Severity::Warning,
    "pin versions in zypper install"
);

impl Rule for PinZypperVersions {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| zypper_install_has_unpinned_packages(&instruction.args))
            .map(|instruction| {
                diagnostic(
                    "DL3037",
                    Severity::Warning,
                    "pin versions in zypper install",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    DnfInstallAssumeYes,
    "DL3038",
    "dnf-install-assume-yes",
    Severity::Warning,
    "use -y with dnf install"
);

impl Rule for DnfInstallAssumeYes {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| dnf_install_missing_yes(&instruction.args))
            .map(|instruction| {
                diagnostic(
                    "DL3038",
                    Severity::Warning,
                    "use `dnf install -y` to avoid interactive prompts",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    DnfCleanAll,
    "DL3040",
    "dnf-clean-all",
    Severity::Info,
    "clean dnf metadata after installs"
);

impl Rule for DnfCleanAll {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| dnf_install_missing_clean_all(&instruction.args))
            .map(|instruction| {
                diagnostic(
                    "DL3040",
                    Severity::Info,
                    "clean dnf metadata with `dnf clean all` in the same RUN layer",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    PinDnfVersions,
    "DL3041",
    "pin-dnf-versions",
    Severity::Warning,
    "pin versions in dnf install"
);

impl Rule for PinDnfVersions {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| dnf_install_has_unpinned_packages(&instruction.args))
            .map(|instruction| {
                diagnostic(
                    "DL3041",
                    Severity::Warning,
                    "pin versions in dnf install",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    PipNoCacheDir,
    "DL3042",
    "pip-no-cache-dir",
    Severity::Warning,
    "avoid pip cache directories"
);

impl Rule for PipNoCacheDir {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        let mut findings = Vec::new();
        let mut stage_no_cache = BTreeMap::new();
        let mut current_stage_keys = Vec::new();
        let mut stage_index = 0usize;
        let mut pip_no_cache_dir = false;

        for instruction in &doc.instructions {
            match instruction.keyword.as_str() {
                "FROM" => {
                    if let Some(from) = &instruction.from {
                        let inherited_from = from.image.to_ascii_lowercase();
                        pip_no_cache_dir = stage_no_cache
                            .get(&inherited_from)
                            .copied()
                            .unwrap_or_default();

                        current_stage_keys.clear();
                        current_stage_keys.push(stage_index.to_string());
                        if let Some(alias) = &from.alias {
                            current_stage_keys.push(alias.to_ascii_lowercase());
                        }
                        for stage_key in &current_stage_keys {
                            stage_no_cache.insert(stage_key.clone(), pip_no_cache_dir);
                        }
                        stage_index += 1;
                    }
                }
                "ENV" => {
                    if let Some(env) = &instruction.env {
                        for assignment in &env.assignments {
                            if assignment.name == "PIP_NO_CACHE_DIR" {
                                pip_no_cache_dir = pip_no_cache_dir_truthy(&assignment.value);
                                for stage_key in &current_stage_keys {
                                    stage_no_cache.insert(stage_key.clone(), pip_no_cache_dir);
                                }
                            }
                        }
                    }
                }
                "RUN"
                    if !has_pip_cache_mount(instruction)
                        && pip_install_missing_no_cache_dir(
                            &instruction.args,
                            pip_no_cache_dir,
                        ) =>
                {
                    findings.push(diagnostic(
                        "DL3042",
                        Severity::Warning,
                        "avoid pip cache directories by using `pip install --no-cache-dir`",
                        instruction,
                    ));
                }
                _ => {}
            }
        }

        findings
    }
}

rule_metadata!(
    NoOnbuildTrigger,
    "DL3043",
    "no-onbuild-trigger",
    Severity::Error,
    "reject ONBUILD triggers for ONBUILD, FROM, or MAINTAINER"
);

impl Rule for NoOnbuildTrigger {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "ONBUILD")
            .filter(|instruction| onbuild_has_disallowed_trigger(&instruction.args))
            .map(|instruction| {
                diagnostic(
                    "DL3043",
                    Severity::Error,
                    "`ONBUILD`, `FROM`, or `MAINTAINER` triggered from within `ONBUILD` instruction",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    NoEnvSelfReference,
    "DL3044",
    "no-env-self-reference",
    Severity::Error,
    "reject same-statement ENV references"
);

impl Rule for NoEnvSelfReference {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        let mut known_variables = BTreeSet::from(["PATH".to_string()]);
        let mut findings = Vec::new();

        for instruction in &doc.instructions {
            if let Some(arg) = &instruction.arg {
                known_variables.insert(arg.name.clone());
            }

            let Some(env) = &instruction.env else {
                continue;
            };

            if env_references_same_statement_variable(env, &known_variables) {
                findings.push(diagnostic(
                    "DL3044",
                    Severity::Error,
                    "do not refer to an environment variable within the same `ENV` statement where it is defined",
                    instruction,
                ));
            }

            known_variables.extend(
                env.assignments
                    .iter()
                    .map(|assignment| assignment.name.clone()),
            );
        }

        findings
    }
}

rule_metadata!(
    CopyRelativeWithoutWorkdir,
    "DL3045",
    "copy-relative-without-workdir",
    Severity::Warning,
    "avoid COPY to relative destinations without WORKDIR"
);

impl Rule for CopyRelativeWithoutWorkdir {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        let mut alias_to_stage = BTreeMap::new();
        let mut stage_workdir = BTreeMap::new();
        let mut current_stage = None;
        let mut workdir_set = false;
        let mut stage_idx = 0usize;
        let mut findings = Vec::new();

        for instruction in &doc.instructions {
            match instruction.keyword.as_str() {
                "FROM" => {
                    if let Some(from) = &instruction.from {
                        let inherited_stage = from
                            .image
                            .parse::<usize>()
                            .ok()
                            .map(|index| format!("__stage_{index}"))
                            .or_else(|| {
                                alias_to_stage
                                    .get(&from.image.to_ascii_lowercase())
                                    .cloned()
                            });

                        workdir_set = inherited_stage
                            .as_ref()
                            .and_then(|stage| stage_workdir.get(stage))
                            .copied()
                            .unwrap_or_else(|| {
                                inherited_workdir_from_variable_stage_reference(
                                    &from.image,
                                    &alias_to_stage,
                                    &stage_workdir,
                                )
                            });

                        let stage_key = format!("__stage_{stage_idx}");
                        stage_workdir.insert(stage_key.clone(), workdir_set);

                        if let Some(alias) = &from.alias {
                            alias_to_stage.insert(alias.to_ascii_lowercase(), stage_key.clone());
                        }

                        current_stage = Some(stage_key);
                        stage_idx += 1;
                    }
                }
                "WORKDIR" => {
                    workdir_set = true;
                    if let Some(stage) = &current_stage {
                        stage_workdir.insert(stage.clone(), true);
                    }
                }
                "COPY"
                    if !workdir_set
                        && instruction.copy.as_ref().is_some_and(|copy| {
                            copy.destination
                                .as_deref()
                                .is_some_and(is_relative_copy_destination)
                        }) =>
                {
                    findings.push(diagnostic(
                        "DL3045",
                        Severity::Warning,
                        "`COPY` to a relative destination without `WORKDIR` set",
                        instruction,
                    ));
                }
                _ => {}
            }
        }

        findings
    }
}

fn inherited_workdir_from_variable_stage_reference(
    image: &str,
    alias_to_stage: &BTreeMap<String, String>,
    stage_workdir: &BTreeMap<String, bool>,
) -> bool {
    // Only infer inherited WORKDIR for variable-expanded stage names when every
    // possible alias match resolves to the same known WORKDIR state. Ambiguous
    // or unresolved references stay conservative and behave as if no WORKDIR
    // was inherited.
    let image = image.to_ascii_lowercase();
    if !image.contains('$') {
        return false;
    }

    let mut matching_states = alias_to_stage
        .iter()
        .filter(|(alias, _)| variable_stage_reference_could_match(&image, alias))
        .filter_map(|(_, stage)| stage_workdir.get(stage).copied());

    let Some(first_state) = matching_states.next() else {
        return false;
    };

    matching_states.all(|state| state == first_state) && first_state
}

fn variable_stage_reference_could_match(reference: &str, alias: &str) -> bool {
    let mut remaining = alias;
    let mut literal = String::new();
    let mut chars = reference.chars().peekable();

    while let Some(character) = chars.next() {
        match character {
            '$' => {
                if !literal.is_empty() {
                    let Some(index) = remaining.find(&literal) else {
                        return false;
                    };
                    remaining = &remaining[index + literal.len()..];
                    literal.clear();
                }

                if matches!(chars.peek(), Some('{')) {
                    chars.next();
                    for variable_character in chars.by_ref() {
                        if variable_character == '}' {
                            break;
                        }
                    }
                } else {
                    while matches!(chars.peek(), Some(next) if *next == '_' || next.is_ascii_alphanumeric())
                    {
                        chars.next();
                    }
                }
            }
            _ => literal.push(character),
        }
    }

    literal.is_empty() || remaining.ends_with(&literal)
}

rule_metadata!(
    UseraddNoLogInit,
    "DL3046",
    "useradd-no-log-init",
    Severity::Warning,
    "use no-log-init with high useradd UIDs"
);

impl Rule for UseraddNoLogInit {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter_map(|instruction| {
                instruction_shell_or_onbuild_run(instruction).map(|shell| (instruction, shell))
            })
            .filter(|(_, shell)| useradd_missing_no_log_init(shell))
            .map(|(instruction, _)| {
                diagnostic(
                    "DL3046",
                    Severity::Warning,
                    "`useradd` without flag `-l` and high UID will result in excessively large image",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    WgetProgress,
    "DL3047",
    "wget-progress",
    Severity::Info,
    "avoid noisy wget progress output"
);

impl Rule for WgetProgress {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter_map(|instruction| {
                instruction_shell_or_onbuild_run(instruction).map(|shell| (instruction, shell))
            })
            .filter(|(_, shell)| wget_missing_progress_control(shell))
            .map(|(instruction, _)| {
                diagnostic(
                    "DL3047",
                    Severity::Info,
                    "avoid wget without `--progress=dot:giga`, `-q`, or `-nv`",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    ValidLabelKey,
    "DL3048",
    "valid-label-key",
    Severity::Style,
    "reject invalid label keys"
);

impl Rule for ValidLabelKey {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        let mut findings = Vec::new();

        for instruction in &doc.instructions {
            if let Some(label) = &instruction.label {
                for pair in label
                    .pairs
                    .iter()
                    .filter(|pair| !is_valid_docker_label_key(&pair.key))
                {
                    findings.push(diagnostic(
                        "DL3048",
                        Severity::Style,
                        format!("invalid label key `{}`", pair.key),
                        instruction,
                    ));
                }
            }
        }

        findings
    }
}

rule_metadata!(
    MissingRequiredLabels,
    "DL3049",
    "missing-required-labels",
    Severity::Info,
    "require configured labels"
);

impl Rule for MissingRequiredLabels {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, _doc: &Dockerfile) -> Vec<Finding> {
        Vec::new()
    }

    fn check_with_config(&self, doc: &Dockerfile, config: &Config) -> Vec<Finding> {
        if config.label_schema.is_empty() {
            return Vec::new();
        }

        missing_required_labels(doc, config)
    }
}

rule_metadata!(
    NoSuperfluousLabels,
    "DL3050",
    "no-superfluous-labels",
    Severity::Info,
    "reject labels outside the configured label schema"
);

impl Rule for NoSuperfluousLabels {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, _doc: &Dockerfile) -> Vec<Finding> {
        Vec::new()
    }

    fn check_with_config(&self, doc: &Dockerfile, config: &Config) -> Vec<Finding> {
        if !config.strict_labels {
            return Vec::new();
        }

        doc.instructions
            .iter()
            .filter(|instruction| {
                instruction.label.as_ref().is_some_and(|label| {
                    label
                        .pairs
                        .iter()
                        .any(|pair| !config.label_schema.contains_key(&pair.key))
                })
            })
            .map(|instruction| {
                diagnostic(
                    "DL3050",
                    Severity::Info,
                    "superfluous label(s) present",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    NoEmptyLabels,
    "DL3051",
    "no-empty-labels",
    Severity::Warning,
    "reject empty configured label values"
);

impl Rule for NoEmptyLabels {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, _doc: &Dockerfile) -> Vec<Finding> {
        Vec::new()
    }

    fn check_with_config(&self, doc: &Dockerfile, config: &Config) -> Vec<Finding> {
        if config.label_schema.is_empty() {
            return Vec::new();
        }

        let mut findings = Vec::new();

        for instruction in &doc.instructions {
            if let Some(label) = &instruction.label {
                for pair in label.pairs.iter().filter(|pair| {
                    config.label_schema.contains_key(&pair.key)
                        && docker_label_value_is_empty(&pair.value)
                }) {
                    findings.push(diagnostic(
                        "DL3051",
                        Severity::Warning,
                        format!("configured label `{}` value is empty", pair.key),
                        instruction,
                    ));
                }
            }
        }

        findings
    }
}

rule_metadata!(
    ValidUrlLabels,
    "DL3052",
    "valid-url-labels",
    Severity::Warning,
    "validate URL label values"
);

impl Rule for ValidUrlLabels {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, _doc: &Dockerfile) -> Vec<Finding> {
        Vec::new()
    }

    fn check_with_config(&self, doc: &Dockerfile, config: &Config) -> Vec<Finding> {
        if config.label_schema.is_empty() {
            return Vec::new();
        }

        doc.instructions
            .iter()
            .filter(|instruction| {
                instruction.label.as_ref().is_some_and(|label| {
                    label.pairs.iter().any(|pair| {
                        config
                            .label_schema
                            .get(&pair.key)
                            .is_some_and(|schema| schema == "url")
                            && !docker_label_value_is_empty(&pair.value)
                            && !is_valid_url_label_value(&pair.value)
                    })
                })
            })
            .map(|instruction| {
                diagnostic(
                    "DL3052",
                    Severity::Warning,
                    "configured URL label is not a valid URL",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    ValidRfc3339Labels,
    "DL3053",
    "valid-rfc3339-labels",
    Severity::Warning,
    "validate RFC3339 label values"
);

impl Rule for ValidRfc3339Labels {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, _doc: &Dockerfile) -> Vec<Finding> {
        Vec::new()
    }

    fn check_with_config(&self, doc: &Dockerfile, config: &Config) -> Vec<Finding> {
        if config.label_schema.is_empty() {
            return Vec::new();
        }

        doc.instructions
            .iter()
            .filter(|instruction| {
                instruction.label.as_ref().is_some_and(|label| {
                    label.pairs.iter().any(|pair| {
                        config
                            .label_schema
                            .get(&pair.key)
                            .is_some_and(|schema| schema == "rfc3339")
                            && !docker_label_value_is_empty(&pair.value)
                            && !is_valid_rfc3339_label_value(&pair.value)
                    })
                })
            })
            .map(|instruction| {
                diagnostic(
                    "DL3053",
                    Severity::Warning,
                    "configured RFC3339 label is not a valid timestamp",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    ValidSpdxLabels,
    "DL3054",
    "valid-spdx-labels",
    Severity::Warning,
    "validate SPDX license label values"
);

impl Rule for ValidSpdxLabels {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, _doc: &Dockerfile) -> Vec<Finding> {
        Vec::new()
    }

    fn check_with_config(&self, doc: &Dockerfile, config: &Config) -> Vec<Finding> {
        if config.label_schema.is_empty() {
            return Vec::new();
        }

        doc.instructions
            .iter()
            .filter(|instruction| {
                instruction.label.as_ref().is_some_and(|label| {
                    label.pairs.iter().any(|pair| {
                        config
                            .label_schema
                            .get(&pair.key)
                            .is_some_and(|schema| schema == "spdx")
                            && !docker_label_value_is_empty(&pair.value)
                            && !is_valid_spdx_label_value(&pair.value)
                    })
                })
            })
            .map(|instruction| {
                diagnostic(
                    "DL3054",
                    Severity::Warning,
                    "configured SPDX label is not a valid SPDX expression",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    ValidGitHashLabels,
    "DL3055",
    "valid-git-hash-labels",
    Severity::Warning,
    "validate git hash label values"
);

impl Rule for ValidGitHashLabels {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, _doc: &Dockerfile) -> Vec<Finding> {
        Vec::new()
    }

    fn check_with_config(&self, doc: &Dockerfile, config: &Config) -> Vec<Finding> {
        if config.label_schema.is_empty() {
            return Vec::new();
        }

        doc.instructions
            .iter()
            .filter(|instruction| {
                instruction.label.as_ref().is_some_and(|label| {
                    label.pairs.iter().any(|pair| {
                        config
                            .label_schema
                            .get(&pair.key)
                            .is_some_and(|schema| schema == "git-hash")
                            && !docker_label_value_is_empty(&pair.value)
                            && !is_valid_git_hash_label_value(&pair.value)
                    })
                })
            })
            .map(|instruction| {
                diagnostic(
                    "DL3055",
                    Severity::Warning,
                    "configured git hash label is not a valid git hash",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    ValidSemverLabels,
    "DL3056",
    "valid-semver-labels",
    Severity::Warning,
    "validate semantic version label values"
);

impl Rule for ValidSemverLabels {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, _doc: &Dockerfile) -> Vec<Finding> {
        Vec::new()
    }

    fn check_with_config(&self, doc: &Dockerfile, config: &Config) -> Vec<Finding> {
        if config.label_schema.is_empty() {
            return Vec::new();
        }

        doc.instructions
            .iter()
            .filter(|instruction| {
                instruction.label.as_ref().is_some_and(|label| {
                    label.pairs.iter().any(|pair| {
                        config
                            .label_schema
                            .get(&pair.key)
                            .is_some_and(|schema| schema == "semver")
                            && !docker_label_value_is_empty(&pair.value)
                            && !is_valid_semver_label_value(&pair.value)
                    })
                })
            })
            .map(|instruction| {
                diagnostic(
                    "DL3056",
                    Severity::Warning,
                    "configured semantic version label is not valid semver",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    MissingHealthcheck,
    "DL3057",
    "missing-healthcheck",
    Severity::Ignore,
    "require HEALTHCHECK instructions"
);

impl Rule for MissingHealthcheck {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, _doc: &Dockerfile) -> Vec<Finding> {
        Vec::new()
    }

    fn check_with_config(&self, doc: &Dockerfile, config: &Config) -> Vec<Finding> {
        if config
            .severity_override("DL3057")
            .is_none_or(|severity| severity == Severity::Ignore)
        {
            return Vec::new();
        }

        missing_healthcheck_findings(doc)
    }
}

rule_metadata!(
    ValidEmailLabels,
    "DL3058",
    "valid-email-labels",
    Severity::Warning,
    "validate email label values"
);

impl Rule for ValidEmailLabels {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, _doc: &Dockerfile) -> Vec<Finding> {
        Vec::new()
    }

    fn check_with_config(&self, doc: &Dockerfile, config: &Config) -> Vec<Finding> {
        if config.label_schema.is_empty() {
            return Vec::new();
        }

        doc.instructions
            .iter()
            .filter(|instruction| {
                instruction.label.as_ref().is_some_and(|label| {
                    label.pairs.iter().any(|pair| {
                        config
                            .label_schema
                            .get(&pair.key)
                            .is_some_and(|schema| schema == "email")
                            && !docker_label_value_is_empty(&pair.value)
                            && !is_valid_email_label_value(&pair.value)
                    })
                })
            })
            .map(|instruction| {
                diagnostic(
                    "DL3058",
                    Severity::Warning,
                    "configured email label is not a valid email address",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    ConsecutiveRun,
    "DL3059",
    "consecutive-run",
    Severity::Info,
    "combine consecutive RUN instructions"
);

impl Rule for ConsecutiveRun {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        let mut previous_run: Option<&Instruction> = None;
        let mut non_posix_shell = false;
        let mut findings = Vec::new();

        for instruction in &doc.instructions {
            if instruction.keyword_is("FROM") {
                non_posix_shell = false;
                previous_run = None;
                continue;
            }

            if instruction.keyword_is("SHELL") {
                non_posix_shell = shell_instruction_is_non_posix(instruction);
                previous_run = None;
                continue;
            }

            if instruction.keyword != "RUN" {
                previous_run = None;
                continue;
            }

            if !non_posix_shell
                && let Some(previous) = previous_run
                && consecutive_run_instructions_should_be_combined(previous, instruction)
            {
                findings.push(diagnostic(
                    "DL3059",
                    Severity::Info,
                    "combine consecutive RUN instructions to reduce image layers",
                    instruction,
                ));
            }

            previous_run = Some(instruction);
        }

        findings
    }
}

rule_metadata!(
    YarnCacheClean,
    "DL3060",
    "yarn-cache-clean",
    Severity::Info,
    "clean yarn cache after yarn install"
);

impl Rule for YarnCacheClean {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| {
                instruction.keyword == "RUN" && yarn_install_missing_cache_clean(instruction)
            })
            .map(|instruction| {
                diagnostic(
                    "DL3060",
                    Severity::Info,
                    "`yarn cache clean` missing after `yarn install` was run",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    InstructionOrder,
    "DL3061",
    "instruction-order",
    Severity::Error,
    "require Dockerfiles to begin with FROM, ARG, or comments"
);

impl Rule for InstructionOrder {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        let mut seen_from = false;
        doc.instructions
            .iter()
            .filter_map(|instruction| match instruction.keyword.as_str() {
                "FROM" => {
                    seen_from = true;
                    None
                }
                "ARG" if !seen_from => None,
                _ if seen_from => None,
                _ => Some(diagnostic(
                    "DL3061",
                    Severity::Error,
                    "Dockerfile must begin with FROM, ARG, or comment",
                    instruction,
                )),
            })
            .collect()
    }
}

rule_metadata!(
    PinGoVersions,
    "DL3062",
    "pin-go-versions",
    Severity::Warning,
    "pin Go package versions"
);

impl Rule for PinGoVersions {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| {
                instruction.keyword == "RUN" && go_package_command_has_unpinned_package(instruction)
            })
            .map(|instruction| {
                diagnostic(
                    "DL3062",
                    Severity::Warning,
                    "pin versions in go package commands",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    ReservedStageName,
    "DL3063",
    "reserved-stage-name",
    Severity::Warning,
    "avoid reserved stage names"
);

impl Rule for ReservedStageName {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| {
                instruction
                    .from
                    .as_ref()
                    .and_then(|from| from.alias.as_deref())
                    .is_some_and(|alias| {
                        alias.eq_ignore_ascii_case("scratch")
                            || alias.eq_ignore_ascii_case("context")
                    })
            })
            .map(|instruction| {
                diagnostic(
                    "DL3063",
                    Severity::Warning,
                    "stage name should not be a reserved word",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    DeprecatedMaintainer,
    "DL4000",
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
                    "DL4000",
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
    EitherWgetOrCurl,
    "DL4001",
    "either-wget-or-curl",
    Severity::Warning,
    "use either wget or curl in a stage"
);

impl Rule for EitherWgetOrCurl {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        let mut seen_curl = false;
        let mut seen_wget = false;
        let mut mixed_tools_reported = false;
        let mut findings = Vec::new();

        for instruction in &doc.instructions {
            if instruction.keyword == "FROM" {
                seen_curl = false;
                seen_wget = false;
                mixed_tools_reported = false;
                continue;
            }

            if instruction.keyword != "RUN" {
                continue;
            }

            let commands = download_commands(instruction);
            if commands.is_empty() {
                continue;
            }

            seen_curl |= commands.contains(&"curl");
            seen_wget |= commands.contains(&"wget");

            if seen_curl && seen_wget && !mixed_tools_reported {
                findings.push(diagnostic(
                    "DL4001",
                    Severity::Warning,
                    "either use wget or curl but not both",
                    instruction,
                ));
                mixed_tools_reported = true;
            }
        }

        findings
    }
}

rule_metadata!(
    UseShellForDefaultShell,
    "DL4005",
    "use-shell-for-default-shell",
    Severity::Warning,
    "use SHELL to change the default shell"
);

impl Rule for UseShellForDefaultShell {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| {
                instruction.keyword == "RUN" && run_links_default_shell(instruction)
            })
            .map(|instruction| {
                diagnostic(
                    "DL4005",
                    Severity::Warning,
                    "use SHELL to change the default shell",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    PipefailBeforePipe,
    "DL4006",
    "pipefail-before-pipe",
    Severity::Warning,
    "set pipefail before RUN instructions with pipes"
);

impl Rule for PipefailBeforePipe {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        let mut shell_handles_pipes = false;
        let mut findings = Vec::new();

        for instruction in &doc.instructions {
            match instruction.keyword.as_str() {
                "FROM" => shell_handles_pipes = false,
                "SHELL" => shell_handles_pipes = shell_instruction_handles_pipes(instruction),
                "RUN" if !shell_handles_pipes && run_has_pipe(instruction) => {
                    findings.push(diagnostic(
                        "DL4006",
                        Severity::Warning,
                        "set the SHELL option -o pipefail before RUN with a pipe in it",
                        instruction,
                    ));
                }
                _ => {}
            }
        }

        findings
    }
}

rule_metadata!(
    SingleCmd,
    "DL4003",
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
            "DL4003",
            Severity::Warning,
            "only the final CMD is used",
            true,
        )
    }
}

rule_metadata!(
    SingleEntrypoint,
    "DL4004",
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
            "DL4004",
            Severity::Error,
            "only the final ENTRYPOINT is used",
            true,
        )
    }
}

fn duplicates(
    doc: &Dockerfile,
    keyword: &str,
    code: &'static str,
    severity: Severity,
    message: &'static str,
    reset_on_from: bool,
) -> Vec<Finding> {
    let mut seen = false;
    let mut findings = Vec::new();
    for instruction in &doc.instructions {
        if reset_on_from && instruction.keyword == "FROM" {
            seen = false;
            continue;
        }

        if instruction.keyword == keyword {
            if seen {
                findings.push(diagnostic(code, severity, message, instruction));
            }
            seen = true;
        }
    }
    findings
}

fn is_windows_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/')
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

fn global_arg_defaults(doc: &Dockerfile) -> BTreeMap<String, String> {
    doc.instructions
        .iter()
        .take_while(|instruction| instruction.keyword != "FROM")
        .filter_map(|instruction| instruction.arg.as_ref())
        .filter_map(|arg| {
            arg.default
                .as_ref()
                .map(|default| (arg.name.clone(), default.clone()))
        })
        .collect()
}

fn resolve_from_arg_image<'a>(
    image: &'a str,
    global_args: &'a BTreeMap<String, String>,
) -> Option<&'a str> {
    let variable_name = image
        .strip_prefix("${")
        .and_then(|name| name.strip_suffix('}'))
        .or_else(|| image.strip_prefix('$'))?;

    global_args.get(variable_name).map(String::as_str)
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
        .filter(|invocation| invocation.command_is("apt-get"))
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
        .filter(|invocation| invocation.command_is("apt-get"))
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
        .filter(|invocation| invocation.command_is("rm"))
        .flat_map(|invocation| invocation.arguments)
        .any(|argument| {
            let value = argument.trim_matches(|character| matches!(character, '"' | '\''));
            value == "/var/lib/apt/lists"
                || value == "/var/lib/apt/lists/*"
                || value.starts_with("/var/lib/apt/lists/")
        })
}

fn pip_install_has_unpinned_packages(shell: &str) -> bool {
    detect_command_invocations(shell)
        .into_iter()
        .filter(|invocation| invocation.command_is_any(&["pip", "pip3"]))
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

                if pip_install_option_takes_value(argument) {
                    expect_option_value = true;
                    continue;
                }

                if argument.starts_with('-') {
                    continue;
                }

                if !argument.contains("==") {
                    return true;
                }
            }

            false
        })
}

fn pip_install_option_takes_value(argument: &str) -> bool {
    matches!(
        argument,
        "-r" | "--requirement"
            | "-c"
            | "--constraint"
            | "--build-constraint"
            | "--requirements-from-script"
            | "-e"
            | "--editable"
            | "-t"
            | "-i"
            | "--index-url"
            | "--extra-index-url"
            | "-f"
            | "--find-links"
            | "--trusted-host"
            | "--platform"
            | "--python-version"
            | "--implementation"
            | "--abi"
            | "--root"
            | "--prefix"
            | "--src"
            | "--target"
            | "--upgrade-strategy"
            | "-C"
            | "--config-settings"
            | "--progress-bar"
            | "--report"
            | "--group"
            | "--uploaded-prior-to"
            | "--no-binary"
            | "--only-binary"
    )
}

fn pip_install_missing_no_cache_dir(shell: &str, stage_no_cache_dir: bool) -> bool {
    shell
        .split("&&")
        .flat_map(|segment| segment.split("||"))
        .flat_map(|segment| segment.split(';'))
        .any(|segment| {
            !shell_segment_pip_no_cache_dir(segment).unwrap_or(stage_no_cache_dir)
                && detect_command_invocations(segment)
                    .into_iter()
                    .any(|invocation| {
                        pip_install_arguments(&invocation)
                            .is_some_and(pip_args_missing_no_cache_dir)
                    })
        })
}

fn shell_segment_pip_no_cache_dir(segment: &str) -> Option<bool> {
    for token in segment.split_whitespace() {
        let Some((name, value)) = token.split_once('=') else {
            break;
        };

        if !is_shell_assignment_name(name) {
            break;
        }

        if name == "PIP_NO_CACHE_DIR" {
            return Some(pip_no_cache_dir_truthy(value));
        }
    }

    None
}

fn is_shell_assignment_name(name: &str) -> bool {
    let mut characters = name.chars();
    let Some(first_character) = characters.next() else {
        return false;
    };

    (first_character == '_' || first_character.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn pip_install_arguments(invocation: &rudolint_shell::ShellCommandInvocation) -> Option<&[String]> {
    if invocation.command_is_any(&["pip", "pip3"]) {
        return Some(&invocation.arguments);
    }

    if invocation.command.starts_with("python")
        && invocation
            .arguments
            .windows(2)
            .any(|window| window[0] == "-m" && window[1] == "pip")
    {
        let pip_index = invocation
            .arguments
            .windows(2)
            .position(|window| window[0] == "-m" && window[1] == "pip")?
            + 2;
        return Some(&invocation.arguments[pip_index..]);
    }

    None
}

fn pip_args_missing_no_cache_dir(arguments: &[String]) -> bool {
    pip_install_subcommand_index(arguments).is_some_and(|index| arguments[index] == "install")
        && !arguments
            .iter()
            .any(|argument| argument == "--no-cache-dir")
}

fn pip_install_subcommand_index(arguments: &[String]) -> Option<usize> {
    let mut expect_option_value = false;
    for (index, argument) in arguments.iter().enumerate() {
        if argument == "\\" {
            continue;
        }

        if expect_option_value {
            expect_option_value = false;
            continue;
        }

        if pip_install_option_takes_value(argument) {
            expect_option_value = true;
            continue;
        }

        if argument.starts_with('-') {
            continue;
        }

        return Some(index);
    }

    None
}

fn pip_no_cache_dir_truthy(value: &str) -> bool {
    matches!(
        value.trim_matches(|character| matches!(character, '\'' | '"')),
        "1" | "true" | "True" | "TRUE" | "on" | "On" | "ON" | "yes" | "Yes" | "YES"
    )
}

fn has_pip_cache_mount(instruction: &Instruction) -> bool {
    instruction.mounts.iter().any(|mount| {
        (mount.type_is("cache") || mount.type_is("tmpfs"))
            && mount
                .target()
                .is_some_and(|value| value.contains(".cache/pip"))
    })
}

fn onbuild_has_disallowed_trigger(args: &str) -> bool {
    let mut tokens = args.split_whitespace();
    matches!(
        tokens.next().map(|token| token.to_ascii_uppercase()),
        Some(trigger) if matches!(trigger.as_str(), "ONBUILD" | "FROM" | "MAINTAINER")
    )
}

fn env_references_same_statement_variable(
    env: &rudolint_dockerfile::EnvInstruction,
    known_variables: &BTreeSet<String>,
) -> bool {
    env.assignments.iter().any(|candidate| {
        env.assignments
            .iter()
            .filter(|assignment| assignment.name != candidate.name)
            .filter(|assignment| !known_variables.contains(&assignment.name))
            .any(|assignment| shell_value_references_variable(&candidate.value, &assignment.name))
    })
}

fn shell_value_references_variable(value: &str, name: &str) -> bool {
    value.contains(&format!("${{{name}}}")) || contains_bare_shell_variable(value, name)
}

fn contains_bare_shell_variable(value: &str, name: &str) -> bool {
    let needle = format!("${name}");
    value.match_indices(&needle).any(|(index, _)| {
        value[index + needle.len()..]
            .chars()
            .next()
            .is_none_or(|character| !is_shell_variable_character(character))
    })
}

fn is_shell_variable_character(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}

fn is_relative_copy_destination(destination: &str) -> bool {
    let destination = destination.trim_matches(|character| matches!(character, '\'' | '"'));
    !destination.starts_with('/')
        && !destination.starts_with('$')
        && !is_windows_absolute(destination)
}

fn instruction_shell_or_onbuild_run(instruction: &Instruction) -> Option<&str> {
    match instruction.keyword.as_str() {
        "RUN" => Some(&instruction.args),
        "ONBUILD" => strip_instruction_prefix(&instruction.args, "RUN"),
        _ => None,
    }
}

fn strip_instruction_prefix<'a>(args: &'a str, keyword: &str) -> Option<&'a str> {
    let trimmed = args.trim_start();
    let (head, tail) = trimmed
        .split_once(char::is_whitespace)
        .unwrap_or((trimmed, ""));
    head.eq_ignore_ascii_case(keyword)
        .then_some(tail.trim_start())
}

fn useradd_missing_no_log_init(shell: &str) -> bool {
    detect_command_invocations(shell)
        .into_iter()
        .filter(|invocation| invocation.command_is("useradd"))
        .any(|invocation| {
            !has_useradd_no_log_init_flag(&invocation.arguments)
                && useradd_uid_values(&invocation.arguments)
                    .into_iter()
                    .any(|uid| uid.parse::<u64>().is_ok_and(|value| value > 99_999))
        })
}

fn has_useradd_no_log_init_flag(arguments: &[String]) -> bool {
    arguments
        .iter()
        .any(|argument| matches!(argument.as_str(), "-l" | "--no-log-init"))
}

fn useradd_uid_values(arguments: &[String]) -> Vec<&str> {
    let mut values = Vec::new();
    let mut index = 0;

    while index < arguments.len() {
        let argument = arguments[index].as_str();
        if matches!(argument, "-u" | "--uid") {
            if let Some(value) = arguments.get(index + 1) {
                values.push(value.as_str());
            }
            index += 2;
            continue;
        }
        if let Some(value) = argument.strip_prefix("--uid=") {
            values.push(value);
        } else if !argument.starts_with("--")
            && let Some(value) = argument.strip_prefix("-u")
        {
            values.push(value.trim_start_matches('='));
        }
        index += 1;
    }

    values
}

fn wget_missing_progress_control(shell: &str) -> bool {
    detect_command_invocations(shell)
        .into_iter()
        .filter(|invocation| invocation.command_is("wget"))
        .any(|invocation| !wget_has_progress_control(&invocation.arguments))
}

fn wget_has_progress_control(arguments: &[String]) -> bool {
    arguments.iter().any(|argument| {
        let option_name = argument
            .split_once('=')
            .map_or(argument.as_str(), |(name, _)| name);

        matches!(
            option_name,
            "-q" | "--quiet"
                | "-nv"
                | "--no-verbose"
                | "-o"
                | "--output-file"
                | "-a"
                | "--append-output"
        ) || option_name.starts_with("--progress")
            || short_option_bundle_contains(option_name, 'q')
            || argument.starts_with("-o")
            || argument.starts_with("-a")
    })
}

fn short_option_bundle_contains(argument: &str, option: char) -> bool {
    if !argument.starts_with('-') || argument.starts_with("--") {
        return false;
    }

    for character in argument.chars().skip(1) {
        if character == option {
            return true;
        }
        if wget_short_option_takes_inline_value(character) {
            return false;
        }
    }

    false
}

fn wget_short_option_takes_inline_value(option: char) -> bool {
    matches!(option, 'O' | 'U')
}

fn is_valid_docker_label_key(key: &str) -> bool {
    key.starts_with(|character: char| character.is_ascii_lowercase())
        && key.ends_with(|character: char| {
            character.is_ascii_lowercase() || character.is_ascii_digit()
        })
        && key.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '.' | '-' | '_' | '/')
        })
        && !key.starts_with("com.docker.")
        && !key.starts_with("io.docker.")
        && !key.starts_with("org.dockerproject.")
        && !key.contains("..")
        && !key.contains("--")
}

fn docker_label_value_is_empty(value: &str) -> bool {
    value
        .trim_matches(|character| matches!(character, '\'' | '"'))
        .trim()
        .is_empty()
}

fn is_valid_url_label_value(value: &str) -> bool {
    let value = value.trim_matches(|character| matches!(character, '\'' | '"'));
    let Ok(url) = url::Url::parse(value) else {
        return false;
    };
    let scheme = url.scheme();

    if scheme.is_empty()
        || !scheme.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.')
        })
    {
        return false;
    }

    if matches!(scheme, "mailto" | "urn") {
        return !url.path().is_empty();
    }

    url.has_host()
}

fn is_valid_rfc3339_label_value(value: &str) -> bool {
    let value = value.trim_matches(|character| matches!(character, '\'' | '"'));
    let Some((date, time_and_offset)) = value.split_once('T').or_else(|| value.split_once('t'))
    else {
        return false;
    };
    if !valid_rfc3339_date(date) {
        return false;
    }

    let Some((time, offset)) = split_rfc3339_time_offset(time_and_offset) else {
        return false;
    };

    valid_rfc3339_time(time) && valid_rfc3339_offset(offset)
}

fn valid_rfc3339_date(date: &str) -> bool {
    let parts = date.split('-').collect::<Vec<_>>();
    if parts.len() != 3 {
        return false;
    }

    let Some(year) = parse_fixed_digits(parts[0], 4) else {
        return false;
    };
    let Some(month) = parse_fixed_digits(parts[1], 2) else {
        return false;
    };
    let Some(day) = parse_fixed_digits(parts[2], 2) else {
        return false;
    };

    year > 0 && (1..=12).contains(&month) && (1..=days_in_month(year, month)).contains(&day)
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u32) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

fn split_rfc3339_time_offset(value: &str) -> Option<(&str, &str)> {
    if let Some(time) = value.strip_suffix('Z').or_else(|| value.strip_suffix('z')) {
        return Some((time, "Z"));
    }

    let offset_start = value.rfind(['+', '-'])?;
    Some((&value[..offset_start], &value[offset_start..]))
}

fn valid_rfc3339_time(time: &str) -> bool {
    let parts = time.split(':').collect::<Vec<_>>();
    if parts.len() != 3 {
        return false;
    }

    let Some(hour) = parse_fixed_digits(parts[0], 2) else {
        return false;
    };
    let Some(minute) = parse_fixed_digits(parts[1], 2) else {
        return false;
    };
    let second = parts[2]
        .split_once('.')
        .map_or(parts[2], |(second, fraction)| {
            if fraction.is_empty() || !fraction.chars().all(|character| character.is_ascii_digit())
            {
                return "";
            }
            second
        });
    let Some(second) = parse_fixed_digits(second, 2) else {
        return false;
    };

    hour <= 23 && minute <= 59 && second <= 60
}

fn valid_rfc3339_offset(offset: &str) -> bool {
    if offset == "Z" {
        return true;
    }

    let Some(rest) = offset.strip_prefix(['+', '-']) else {
        return false;
    };
    let parts = rest.split(':').collect::<Vec<_>>();
    if parts.len() != 2 {
        return false;
    }

    let Some(hour) = parse_fixed_digits(parts[0], 2) else {
        return false;
    };
    let Some(minute) = parse_fixed_digits(parts[1], 2) else {
        return false;
    };

    hour <= 23 && minute <= 59
}

fn parse_fixed_digits(value: &str, width: usize) -> Option<u32> {
    (value.len() == width && value.chars().all(|character| character.is_ascii_digit()))
        .then(|| value.parse().ok())
        .flatten()
}

fn is_valid_spdx_label_value(value: &str) -> bool {
    let value = value.trim_matches(|character| matches!(character, '\'' | '"'));
    spdx::Expression::parse(value).is_ok()
}

fn is_valid_git_hash_label_value(value: &str) -> bool {
    let value = value.trim_matches(|character| matches!(character, '\'' | '"'));
    matches!(value.len(), 7 | 40)
        && value
            .chars()
            .all(|character| matches!(character, '0'..='9' | 'a'..='f'))
}

fn is_valid_semver_label_value(value: &str) -> bool {
    let value = value.trim_matches(|character| matches!(character, '\'' | '"'));
    semver::Version::parse(value).is_ok()
}

fn is_valid_email_label_value(value: &str) -> bool {
    let value = value.trim_matches(|character| matches!(character, '\'' | '"'));
    email_address::EmailAddress::is_valid(value)
}

fn consecutive_run_instructions_should_be_combined(
    previous: &Instruction,
    current: &Instruction,
) -> bool {
    let Some(previous_run) = previous.run.as_ref() else {
        return false;
    };
    let Some(current_run) = current.run.as_ref() else {
        return false;
    };

    previous_run.flags == current_run.flags
        && shell_command_count(previous) <= 2
        && shell_command_count(current) <= 2
}

fn shell_command_count(instruction: &Instruction) -> usize {
    instruction
        .run
        .as_ref()
        .and_then(|run| run.shell.as_ref())
        .map_or(0, |shell| detect_command_invocations(&shell.text).len())
}

fn yarn_install_missing_cache_clean(instruction: &Instruction) -> bool {
    if has_yarn_cache_mount(instruction) {
        return false;
    }

    let Some(shell) = instruction.run.as_ref().and_then(|run| run.shell.as_ref()) else {
        return false;
    };

    let invocations = detect_command_invocations(&shell.text);
    invocations
        .iter()
        .enumerate()
        .filter(|(_, invocation)| invocation.command_has_args("yarn", &["install"]))
        .any(|(install_index, _)| {
            !invocations
                .iter()
                .enumerate()
                .any(|(clean_index, invocation)| {
                    clean_index > install_index
                        && invocation.command_has_args("yarn", &["cache", "clean"])
                })
        })
}

fn has_yarn_cache_mount(instruction: &Instruction) -> bool {
    instruction.mounts.iter().any(|mount| {
        (mount.type_is("cache") || mount.type_is("tmpfs"))
            && mount.target().is_some_and(is_yarn_cache_target)
    })
}

fn is_yarn_cache_target(value: &str) -> bool {
    let target = value.trim_end_matches('/');
    target == ".cache/yarn" || target.ends_with("/.cache/yarn") || target.contains("/.cache/yarn/")
}

fn go_package_command_has_unpinned_package(instruction: &Instruction) -> bool {
    let Some(shell) = instruction.run.as_ref().and_then(|run| run.shell.as_ref()) else {
        return false;
    };

    detect_command_invocations(&shell.text)
        .iter()
        .flat_map(go_packages_from_invocation)
        .any(|package| !go_package_has_pinned_version(package))
}

fn go_packages_from_invocation(invocation: &rudolint_shell::ShellCommandInvocation) -> Vec<&str> {
    if invocation.command != "go" {
        return Vec::new();
    }

    let args = invocation
        .arguments
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    let Some(command_index) = go_next_non_option_operand_index(&args, 0) else {
        return Vec::new();
    };

    if args.get(command_index) == Some(&"run") {
        return go_non_option_operands(&args, command_index + 1)
            .into_iter()
            .take(1)
            .collect();
    }

    if matches!(args.get(command_index), Some(&"install" | &"get")) {
        return go_non_option_operands(&args, command_index + 1)
            .into_iter()
            .filter(|argument| *argument != "tool")
            .collect();
    }

    Vec::new()
}

fn go_non_option_operands<'a>(args: &[&'a str], start: usize) -> Vec<&'a str> {
    let mut operands = Vec::new();
    let mut index = start;

    while let Some(operand_index) = go_next_non_option_operand_index(args, index) {
        operands.push(args[operand_index]);
        index = operand_index + 1;
    }

    operands
}

fn go_next_non_option_operand_index(args: &[&str], start: usize) -> Option<usize> {
    let mut index = start;

    while index < args.len() {
        let argument = args[index];
        if argument == "\\" {
            index += 1;
            continue;
        }

        if argument.starts_with('-') {
            let option_name = argument.split_once('=').map_or(argument, |(name, _)| name);
            if argument.find('=').is_none() && go_option_takes_value(option_name) {
                let mut value_index = index + 1;
                while args.get(value_index).is_some_and(|value| *value == "\\") {
                    value_index += 1;
                }

                if args
                    .get(value_index)
                    .is_some_and(|value| !value.starts_with('-'))
                {
                    index = value_index;
                }
            }
        } else {
            return Some(index);
        }
        index += 1;
    }

    None
}

fn go_package_has_pinned_version(package: &str) -> bool {
    go_package_is_local_path(package)
        || go_package_version(package)
            .is_some_and(|version| !version.is_empty() && !matches!(version, "latest" | "none"))
}

fn go_package_is_local_path(package: &str) -> bool {
    package == "."
        || package.starts_with('/')
        || package.starts_with('.')
        || package.ends_with(".go")
}

fn go_option_takes_value(option: &str) -> bool {
    matches!(
        option,
        "-C" | "-asmflags"
            | "-buildmode"
            | "-compiler"
            | "-exec"
            | "-gccgoflags"
            | "-gcflags"
            | "-installsuffix"
            | "-ldflags"
            | "-mod"
            | "-modfile"
            | "-overlay"
            | "-p"
            | "-pkgdir"
            | "-tags"
            | "-toolexec"
    )
}

fn go_package_version(package: &str) -> Option<&str> {
    package.rsplit_once('@').map(|(_, version)| version)
}

fn download_commands(instruction: &Instruction) -> BTreeSet<&'static str> {
    let Some(shell) = instruction.run.as_ref().and_then(|run| run.shell.as_ref()) else {
        return BTreeSet::new();
    };

    detect_command_invocations(&shell.text)
        .iter()
        .filter_map(|invocation| match invocation.command.as_str() {
            "curl" => Some("curl"),
            "wget" => Some("wget"),
            _ => None,
        })
        .collect()
}

fn run_links_default_shell(instruction: &Instruction) -> bool {
    let Some(shell) = instruction.run.as_ref().and_then(|run| run.shell.as_ref()) else {
        return false;
    };

    detect_command_invocations(&shell.text)
        .iter()
        .any(|invocation| {
            invocation.command_is("ln")
                && invocation
                    .arguments
                    .iter()
                    .rfind(|argument| !argument.starts_with('-'))
                    .is_some_and(|argument| argument == "/bin/sh")
                && invocation.arguments.iter().any(|argument| {
                    argument == "--symbolic"
                        || (argument.starts_with('-')
                            && !argument.starts_with("--")
                            && argument.chars().skip(1).any(|flag| flag == 's'))
                })
        })
}

fn shell_instruction_handles_pipes(instruction: &Instruction) -> bool {
    match &instruction.form {
        InstructionForm::Json(parts) => {
            let Some(shell) = parts.first().map(String::as_str) else {
                return false;
            };
            let shell = normalized_shell_executable(shell);

            shell_is_non_posix(&shell)
                || (shell_supports_pipefail(&shell)
                    && parts
                        .windows(2)
                        .any(|window| window[0] == "-o" && window[1] == "pipefail"))
        }
        InstructionForm::Shell { text, .. } => {
            let shell = text
                .split_whitespace()
                .next()
                .map(normalized_shell_executable)
                .unwrap_or_default();

            shell_is_non_posix(&shell)
                || (shell_supports_pipefail(&shell)
                    && text
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .windows(2)
                        .any(|window| window[0] == "-o" && window[1] == "pipefail"))
        }
        _ => false,
    }
}

fn shell_instruction_is_non_posix(instruction: &Instruction) -> bool {
    match &instruction.form {
        InstructionForm::Json(parts) => parts
            .first()
            .map(String::as_str)
            .map(normalized_shell_executable)
            .is_some_and(|shell| shell_is_non_posix(&shell)),
        InstructionForm::Shell { text, .. } => shell_form_executable(text)
            .map(normalized_shell_executable)
            .is_some_and(|shell| shell_is_non_posix(&shell)),
        _ => false,
    }
}

fn shell_form_executable(text: &str) -> Option<&str> {
    let trimmed = text.trim_start();
    if let Some(rest) = trimmed.strip_prefix('"') {
        return rest.split('"').next();
    }
    if let Some(rest) = trimmed.strip_prefix('\'') {
        return rest.split('\'').next();
    }
    trimmed.split_whitespace().next()
}

fn normalized_shell_executable(shell: &str) -> String {
    shell
        .trim_matches('"')
        .trim_matches('\'')
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(shell)
        .to_ascii_lowercase()
}

fn shell_is_non_posix(shell: &str) -> bool {
    matches!(
        shell,
        "powershell" | "powershell.exe" | "pwsh" | "pwsh.exe" | "cmd" | "cmd.exe"
    )
}

fn shell_supports_pipefail(shell: &str) -> bool {
    matches!(shell, "bash" | "bash.exe" | "zsh" | "zsh.exe" | "ash")
}

fn run_has_pipe(instruction: &Instruction) -> bool {
    instruction
        .run
        .as_ref()
        .and_then(|run| run.shell.as_ref())
        .is_some_and(|shell| shell_text_has_pipeline_operator(&shell.text))
}

fn shell_text_has_pipeline_operator(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut index = 0usize;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    while index < bytes.len() {
        let byte = bytes[index];

        if escaped {
            escaped = false;
            index += 1;
            continue;
        }

        if byte == b'\\' && !in_single_quote {
            escaped = true;
            index += 1;
            continue;
        }

        if byte == b'\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            index += 1;
            continue;
        }

        if byte == b'"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            index += 1;
            continue;
        }

        if !in_single_quote && !in_double_quote && byte == b'|' {
            let previous_is_pipe = index > 0 && bytes[index - 1] == b'|';
            let next_is_pipe = index + 1 < bytes.len() && bytes[index + 1] == b'|';
            if !previous_is_pipe && !next_is_pipe {
                return true;
            }
        }

        index += 1;
    }

    false
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct HealthcheckStage {
    parent: Option<usize>,
    instruction_index: usize,
}

fn missing_healthcheck_findings(doc: &Dockerfile) -> Vec<Finding> {
    let mut current_stage = None;
    let mut next_stage_id = 0usize;
    let mut stage_aliases = BTreeMap::new();
    let mut stages = BTreeMap::new();
    let mut good_stages = BTreeSet::new();

    for (instruction_index, instruction) in doc.instructions.iter().enumerate() {
        if let Some(from) = &instruction.from {
            let stage_id = next_stage_id;
            let parent = resolve_stage_reference(&from.image, &stage_aliases, next_stage_id);
            let stage = HealthcheckStage {
                parent,
                instruction_index,
            };
            next_stage_id += 1;

            if let Some(alias) = &from.alias {
                stage_aliases.insert(alias.to_ascii_lowercase(), stage_id);
            }

            if stage_inherits_healthcheck(parent, &stages, &good_stages) {
                good_stages.insert(stage_id);
            }
            stages.insert(stage_id, stage);
            current_stage = Some(stage_id);
            continue;
        }

        if instruction.healthcheck.is_some() {
            let Some(stage_id) = current_stage else {
                continue;
            };

            good_stages.insert(stage_id);
        }
    }

    stages
        .iter()
        .filter(|(stage_id, stage)| {
            !good_stages.contains(*stage_id)
                && !stage_inherits_healthcheck(stage.parent, &stages, &good_stages)
        })
        .filter_map(|(_, stage)| doc.instructions.get(stage.instruction_index))
        .map(|instruction| {
            diagnostic(
                "DL3057",
                Severity::Ignore,
                "HEALTHCHECK instruction is missing",
                instruction,
            )
        })
        .collect()
}

fn stage_inherits_healthcheck(
    parent: Option<usize>,
    stages: &BTreeMap<usize, HealthcheckStage>,
    good_stages: &BTreeSet<usize>,
) -> bool {
    let mut visited = BTreeSet::new();
    let mut next_parent = parent;

    while let Some(stage_id) = next_parent {
        if !visited.insert(stage_id) {
            return false;
        }

        if good_stages.contains(&stage_id) {
            return true;
        }

        next_parent = stages.get(&stage_id).and_then(|stage| stage.parent);
    }

    false
}

fn missing_required_labels(doc: &Dockerfile, config: &Config) -> Vec<Finding> {
    let mut current_stage = None;
    let mut current_stage_line = None;
    let mut next_stage_id = 0usize;
    let mut stage_aliases = BTreeMap::new();
    let mut stage_labels: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
    let mut stage_lines: BTreeMap<usize, usize> = BTreeMap::new();
    let mut referenced_stages = BTreeSet::new();

    for instruction in &doc.instructions {
        match instruction.keyword.as_str() {
            "FROM" => {
                if let Some(from) = &instruction.from {
                    let parent_labels =
                        resolve_stage_reference(&from.image, &stage_aliases, next_stage_id)
                            .and_then(|stage| stage_labels.get(&stage).cloned())
                            .unwrap_or_default();
                    let stage = next_stage_id;
                    next_stage_id += 1;

                    stage_labels.insert(stage, parent_labels);
                    stage_lines.insert(stage, instruction.line);
                    if let Some(alias) = &from.alias {
                        stage_aliases.insert(alias.to_ascii_lowercase(), stage);
                    }
                    current_stage = Some(stage);
                    current_stage_line = Some(instruction.line);
                }
            }
            "LABEL" => {
                if let (Some(stage), Some(label)) = (&current_stage, &instruction.label) {
                    let labels = stage_labels.entry(*stage).or_default();
                    labels.extend(label.pairs.iter().map(|pair| pair.key.clone()));
                }
            }
            "COPY" => {
                if let Some(source_stage) = instruction
                    .copy
                    .as_ref()
                    .and_then(|copy| copy.from.as_ref())
                    .and_then(|source| {
                        resolve_stage_reference(source, &stage_aliases, next_stage_id)
                    })
                {
                    referenced_stages.insert(source_stage);
                }
            }
            _ => {}
        }
    }

    let required = config.label_schema.keys().cloned().collect::<BTreeSet<_>>();
    stage_labels
        .iter()
        .filter(|(stage, _)| !referenced_stages.contains(*stage))
        .flat_map(|(stage, labels)| {
            let line = stage_lines
                .get(stage)
                .copied()
                .or(current_stage_line)
                .unwrap_or(1);
            required.difference(labels).map(move |label| {
                Finding::new(
                    "DL3049",
                    Severity::Info,
                    format!("Label `{label}` is missing"),
                    line,
                    1,
                )
            })
        })
        .collect()
}

fn resolve_stage_reference(
    reference: &str,
    stage_aliases: &BTreeMap<String, usize>,
    next_stage_id: usize,
) -> Option<usize> {
    if let Ok(stage_index) = reference.parse::<usize>() {
        return (stage_index < next_stage_id).then_some(stage_index);
    }

    if is_external_stage_reference(reference) {
        return None;
    }

    stage_aliases.get(&reference.to_ascii_lowercase()).copied()
}

fn is_external_stage_reference(reference: &str) -> bool {
    reference.contains('/') || reference.contains(':') || reference.contains('@')
}

fn apt_get_install_missing_yes(shell: &str) -> bool {
    detect_command_invocations(shell)
        .into_iter()
        .filter(|invocation| invocation.command_is("apt-get"))
        .any(|invocation| {
            let has_install = apt_get_subcommand_index(&invocation.arguments)
                .is_some_and(|index| invocation.arguments[index] == "install");
            let has_assume_yes = invocation
                .arguments
                .iter()
                .any(|argument| apt_get_argument_is_assume_yes(argument));

            has_install && !has_assume_yes
        })
}

fn apt_get_argument_is_assume_yes(argument: &str) -> bool {
    matches!(argument, "-y" | "--yes" | "--assume-yes")
        || apt_get_short_option_bundle_contains(argument, 'y')
}

fn apt_get_short_option_bundle_contains(argument: &str, option: char) -> bool {
    if !argument.starts_with('-') || argument.starts_with("--") {
        return false;
    }

    for character in argument.chars().skip(1) {
        if character == option {
            return true;
        }
        if apt_get_short_option_takes_inline_value(character) {
            return false;
        }
    }

    false
}

fn apt_get_short_option_takes_inline_value(option: char) -> bool {
    matches!(option, 'c' | 'o')
}

fn apt_get_install_missing_no_install_recommends(shell: &str) -> bool {
    detect_command_invocations(shell)
        .into_iter()
        .filter(|invocation| invocation.command_is("apt-get"))
        .any(|invocation| {
            let has_install = apt_get_subcommand_index(&invocation.arguments)
                .is_some_and(|index| invocation.arguments[index] == "install");
            let has_no_install_recommends = invocation
                .arguments
                .iter()
                .any(|argument| argument == "--no-install-recommends");

            has_install && !has_no_install_recommends
        })
}

fn apt_get_subcommand_index(arguments: &[String]) -> Option<usize> {
    let mut expect_option_value = false;
    for (index, argument) in arguments.iter().enumerate() {
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

        return Some(index);
    }

    None
}

fn npm_install_has_unpinned_packages(shell: &str) -> bool {
    detect_command_invocations(shell)
        .into_iter()
        .filter(|invocation| invocation.command_is("npm"))
        .any(|invocation| {
            let Some(install_index) = npm_subcommand_index(&invocation.arguments) else {
                return false;
            };
            if !matches!(
                invocation.arguments[install_index].as_str(),
                "install" | "i" | "add"
            ) {
                return false;
            }

            let mut expect_option_value = false;
            for argument in invocation.arguments.iter().skip(install_index + 1) {
                if argument == "\\" {
                    continue;
                }

                if expect_option_value {
                    expect_option_value = false;
                    continue;
                }

                if npm_install_option_takes_value(argument) {
                    expect_option_value = true;
                    continue;
                }

                if argument.starts_with('-') {
                    continue;
                }

                if argument.contains("://") || argument.starts_with("git+") {
                    continue;
                }

                if !npm_package_has_version(argument) {
                    return true;
                }
            }

            false
        })
}

fn npm_package_has_version(package: &str) -> bool {
    let search_start = usize::from(package.starts_with('@'));
    package[search_start..]
        .rfind('@')
        .is_some_and(|index| search_start + index + 1 < package.len())
}

fn npm_subcommand_index(arguments: &[String]) -> Option<usize> {
    let mut expect_option_value = false;
    for (index, argument) in arguments.iter().enumerate() {
        if argument == "\\" {
            continue;
        }

        if expect_option_value {
            expect_option_value = false;
            continue;
        }

        if npm_install_option_takes_value(argument) {
            expect_option_value = true;
            continue;
        }

        if argument.starts_with('-') {
            continue;
        }

        return Some(index);
    }

    None
}

fn npm_install_option_takes_value(argument: &str) -> bool {
    matches!(
        argument,
        "-w" | "--workspace"
            | "--registry"
            | "--scope"
            | "--cache"
            | "--globalconfig"
            | "--init-module"
            | "--install-strategy"
            | "--local-address"
            | "--loglevel"
            | "--omit"
            | "--only"
            | "--prefix"
            | "--save-prefix"
            | "--tag"
            | "--userconfig"
    )
}

fn apk_add_has_unpinned_packages(shell: &str) -> bool {
    detect_command_invocations(shell)
        .into_iter()
        .filter(|invocation| invocation.command_is("apk"))
        .any(|invocation| {
            let Some(add_index) = apk_subcommand_index(&invocation.arguments) else {
                return false;
            };
            if invocation.arguments[add_index] != "add" {
                return false;
            }

            let mut expect_option_value = false;
            for argument in invocation.arguments.iter().skip(add_index + 1) {
                if argument == "\\" {
                    continue;
                }

                if expect_option_value {
                    expect_option_value = false;
                    continue;
                }

                if apk_add_option_takes_value(argument) {
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

fn apk_subcommand_index(arguments: &[String]) -> Option<usize> {
    let mut expect_option_value = false;
    for (index, argument) in arguments.iter().enumerate() {
        if argument == "\\" {
            continue;
        }

        if expect_option_value {
            expect_option_value = false;
            continue;
        }

        if apk_global_option_takes_value(argument) {
            expect_option_value = true;
            continue;
        }

        if argument.starts_with('-') {
            continue;
        }

        return Some(index);
    }

    None
}

fn apk_global_option_takes_value(argument: &str) -> bool {
    matches!(
        argument,
        "-p" | "--root"
            | "-X"
            | "--repository"
            | "--arch"
            | "--keys-dir"
            | "--repositories-file"
            | "--cache-dir"
    )
}

fn apk_add_option_takes_value(argument: &str) -> bool {
    matches!(argument, "-t" | "--virtual" | "-X" | "--repository")
}

fn apk_add_missing_no_cache(shell: &str) -> bool {
    detect_command_invocations(shell)
        .into_iter()
        .filter(|invocation| invocation.command_is("apk"))
        .any(|invocation| {
            apk_subcommand_index(&invocation.arguments)
                .is_some_and(|index| invocation.arguments[index] == "add")
                && !invocation
                    .arguments
                    .iter()
                    .any(|argument| argument == "--no-cache")
        })
}

fn copy_multiple_sources_without_directory_destination(instruction: &Instruction) -> bool {
    let Some(copy) = &instruction.copy else {
        return false;
    };
    if copy.kind != CopyKind::Copy {
        return false;
    }

    let operands = match &instruction.form {
        InstructionForm::Json(values) => values.clone(),
        _ => {
            let mut operands = copy.sources.clone();
            operands.extend(copy.destination.iter().cloned());
            operands
        }
    };

    operands.len() > 2
        && operands
            .last()
            .is_some_and(|destination| !copy_destination_ends_with_directory_slash(destination))
}

fn copy_destination_ends_with_directory_slash(destination: &str) -> bool {
    destination
        .trim_matches(|character| matches!(character, '\'' | '"'))
        .ends_with('/')
}

fn copy_from_reference_is_unresolved(
    instruction: &Instruction,
    aliases: &BTreeSet<String>,
    stage_count: usize,
) -> bool {
    let Some(copy) = &instruction.copy else {
        return false;
    };
    if copy.kind != CopyKind::Copy {
        return false;
    }
    let Some(from) = &copy.from else {
        return false;
    };
    let last_segment = from.rsplit('/').next().unwrap_or(from);
    if last_segment.contains(':') || last_segment.contains('@') {
        return false;
    }
    if aliases.contains(&from.to_ascii_lowercase()) {
        return false;
    }
    from.parse::<usize>()
        .map_or(true, |index| index >= stage_count.saturating_sub(1))
}

fn copy_from_references_current_stage(
    instruction: &Instruction,
    current_alias: Option<&str>,
    current_stage_index: Option<usize>,
) -> bool {
    let Some(copy) = &instruction.copy else {
        return false;
    };
    if copy.kind != CopyKind::Copy {
        return false;
    }
    let Some(from) = &copy.from else {
        return false;
    };

    current_alias.is_some_and(|alias| from.eq_ignore_ascii_case(alias))
        || current_stage_index.is_some_and(|index| {
            from.parse::<usize>()
                .is_ok_and(|from_index| from_index == index)
        })
}

fn from_image_uses_trusted_registry(
    image: &str,
    stage_aliases: &BTreeSet<String>,
    config: &Config,
) -> bool {
    if image == "scratch" || stage_aliases.contains(&image.to_ascii_lowercase()) {
        return true;
    }

    image_registry(image).is_some_and(|registry| {
        config
            .trusted_registries
            .iter()
            .any(|trusted| trusted.trim_end_matches('/') == registry)
    })
}

fn image_registry(image: &str) -> Option<&str> {
    let first_component = image.split('/').next()?;
    if first_component == "localhost"
        || first_component.contains('.')
        || first_component.contains(':')
    {
        Some(first_component)
    } else {
        None
    }
}

fn shell_uses_apt(shell: &str) -> bool {
    detect_command_invocations(shell)
        .into_iter()
        .any(|invocation| invocation.command_is("apt"))
}

fn gem_install_has_unpinned_packages(shell: &str) -> bool {
    detect_command_invocations(shell)
        .into_iter()
        .filter(|invocation| invocation.command_is("gem"))
        .any(|invocation| {
            let Some(install_index) = gem_subcommand_index(&invocation.arguments) else {
                return false;
            };
            if invocation.arguments[install_index] != "install" {
                return false;
            }

            let mut expect_option_value = false;
            let mut expect_version_value = false;
            let mut has_version_option = false;
            let mut has_unpinned_package = false;
            for argument in invocation.arguments.iter().skip(install_index + 1) {
                if argument == "\\" {
                    continue;
                }

                if expect_option_value {
                    if expect_version_value {
                        has_version_option = true;
                    }
                    expect_option_value = false;
                    expect_version_value = false;
                    continue;
                }

                if matches!(argument.as_str(), "-v" | "--version") {
                    expect_option_value = true;
                    expect_version_value = true;
                    continue;
                }

                if let Some(version) = argument.strip_prefix("--version=") {
                    has_version_option = !version.is_empty();
                    continue;
                }

                if gem_option_takes_value(argument) {
                    expect_option_value = true;
                    continue;
                }

                if argument.starts_with('-') {
                    continue;
                }

                if !argument.contains(':') {
                    has_unpinned_package = true;
                }
            }

            has_unpinned_package && !has_version_option
        })
}

fn gem_subcommand_index(arguments: &[String]) -> Option<usize> {
    let mut expect_option_value = false;
    for (index, argument) in arguments.iter().enumerate() {
        if argument == "\\" {
            continue;
        }

        if expect_option_value {
            expect_option_value = false;
            continue;
        }

        if gem_option_takes_value(argument) {
            expect_option_value = true;
            continue;
        }

        if argument.starts_with('-') {
            continue;
        }

        return Some(index);
    }

    None
}

fn gem_option_takes_value(argument: &str) -> bool {
    matches!(
        argument,
        "--config-file"
            | "--source"
            | "--platform"
            | "-i"
            | "--install-dir"
            | "-n"
            | "--bindir"
            | "--build-root"
    )
}

fn yum_install_missing_yes(shell: &str) -> bool {
    detect_command_invocations(shell)
        .into_iter()
        .filter(|invocation| invocation.command_is("yum"))
        .any(|invocation| {
            yum_subcommand_index(&invocation.arguments)
                .is_some_and(|index| invocation.arguments[index] == "install")
                && !invocation
                    .arguments
                    .iter()
                    .any(|argument| matches!(argument.as_str(), "-y" | "--assumeyes"))
        })
}

fn yum_subcommand_index(arguments: &[String]) -> Option<usize> {
    let mut expect_option_value = false;
    for (index, argument) in arguments.iter().enumerate() {
        if argument == "\\" {
            continue;
        }

        if expect_option_value {
            expect_option_value = false;
            continue;
        }

        if yum_option_takes_value(argument) {
            expect_option_value = true;
            continue;
        }

        if argument.starts_with('-') {
            continue;
        }

        return Some(index);
    }

    None
}

fn yum_option_takes_value(argument: &str) -> bool {
    matches!(
        argument,
        "-c" | "--config"
            | "--installroot"
            | "--releasever"
            | "--setopt"
            | "--disablerepo"
            | "--enablerepo"
            | "-x"
            | "--exclude"
            | "--disableplugin"
            | "--enableplugin"
    )
}

fn yum_install_missing_clean_all(shell: &str) -> bool {
    let invocations = detect_command_invocations(shell);
    let last_install_index = invocations.iter().rposition(|invocation| {
        invocation.command_is("yum")
            && yum_subcommand_index(&invocation.arguments)
                .is_some_and(|index| invocation.arguments[index] == "install")
    });
    let has_clean_after_last_install = last_install_index.is_some_and(|install_index| {
        invocations
            .iter()
            .skip(install_index + 1)
            .any(|invocation| {
                if invocation.command != "yum" {
                    return false;
                }

                let Some(clean_index) = yum_subcommand_index(&invocation.arguments) else {
                    return false;
                };

                invocation.arguments[clean_index] == "clean"
                    && invocation
                        .arguments
                        .iter()
                        .skip(clean_index + 1)
                        .any(|argument| argument == "all")
            })
    });

    last_install_index.is_some() && !has_clean_after_last_install
}

fn yum_install_has_unpinned_packages(shell: &str) -> bool {
    detect_command_invocations(shell)
        .into_iter()
        .filter(|invocation| invocation.command_is("yum"))
        .any(|invocation| {
            let Some(install_index) = yum_subcommand_index(&invocation.arguments) else {
                return false;
            };
            if invocation.arguments[install_index] != "install" {
                return false;
            }

            let mut expect_option_value = false;
            for argument in invocation.arguments.iter().skip(install_index + 1) {
                if argument == "\\" {
                    continue;
                }

                if expect_option_value {
                    expect_option_value = false;
                    continue;
                }

                if yum_option_takes_value(argument) {
                    expect_option_value = true;
                    continue;
                }

                if argument.starts_with('-') {
                    continue;
                }

                if !rpm_package_has_version(argument) {
                    return true;
                }
            }

            false
        })
}

fn rpm_package_has_version(package: &str) -> bool {
    package.contains('=')
        || package.rsplit_once('-').is_some_and(|(_, version)| {
            version.starts_with(|character: char| character.is_ascii_digit())
        })
}

fn zypper_install_missing_yes(shell: &str) -> bool {
    detect_command_invocations(shell)
        .into_iter()
        .filter(|invocation| invocation.command_is("zypper"))
        .any(|invocation| {
            zypper_subcommand_index(&invocation.arguments).is_some_and(|index| {
                matches!(invocation.arguments[index].as_str(), "install" | "in")
            }) && !invocation
                .arguments
                .iter()
                .any(|argument| matches!(argument.as_str(), "-y" | "-n" | "--non-interactive"))
        })
}

fn zypper_subcommand_index(arguments: &[String]) -> Option<usize> {
    let mut expect_option_value = false;
    for (index, argument) in arguments.iter().enumerate() {
        if argument == "\\" {
            continue;
        }

        if expect_option_value {
            expect_option_value = false;
            continue;
        }

        if zypper_option_takes_value(argument) {
            expect_option_value = true;
            continue;
        }

        if argument.starts_with('-') {
            continue;
        }

        return Some(index);
    }

    None
}

fn zypper_option_takes_value(argument: &str) -> bool {
    matches!(
        argument,
        "-R" | "--root"
            | "--reposd-dir"
            | "--cache-dir"
            | "--raw-cache-dir"
            | "--solv-cache-dir"
            | "--pkg-cache-dir"
            | "-c"
            | "--config"
            | "-r"
            | "--repo"
            | "-t"
            | "--type"
            | "--from"
    )
}

fn shell_uses_zypper_dist_upgrade(shell: &str) -> bool {
    detect_command_invocations(shell)
        .into_iter()
        .filter(|invocation| invocation.command_is("zypper"))
        .any(|invocation| {
            zypper_subcommand_index(&invocation.arguments).is_some_and(|index| {
                matches!(invocation.arguments[index].as_str(), "dist-upgrade" | "dup")
            })
        })
}

fn zypper_use_missing_clean(shell: &str) -> bool {
    let invocations = detect_command_invocations(shell);
    let last_zypper_use_index = invocations.iter().rposition(|invocation| {
        if invocation.command != "zypper" {
            return false;
        }

        zypper_subcommand_index(&invocation.arguments)
            .is_some_and(|index| !matches!(invocation.arguments[index].as_str(), "clean" | "cc"))
    });

    let has_clean_after_last_use = last_zypper_use_index.is_some_and(|use_index| {
        invocations.iter().skip(use_index + 1).any(|invocation| {
            if invocation.command != "zypper" {
                return false;
            }

            zypper_subcommand_index(&invocation.arguments)
                .is_some_and(|index| matches!(invocation.arguments[index].as_str(), "clean" | "cc"))
        })
    });

    last_zypper_use_index.is_some() && !has_clean_after_last_use
}

fn zypper_install_has_unpinned_packages(shell: &str) -> bool {
    detect_command_invocations(shell)
        .into_iter()
        .filter(|invocation| invocation.command_is("zypper"))
        .any(|invocation| {
            let Some(install_index) = invocation
                .arguments
                .iter()
                .position(|argument| argument == "install")
            else {
                return false;
            };

            invocation
                .arguments
                .iter()
                .skip(install_index + 1)
                .filter(|argument| !argument.starts_with('-'))
                .any(|package| !rpm_package_has_version(package))
        })
}

fn dnf_install_missing_yes(shell: &str) -> bool {
    detect_command_invocations(shell)
        .into_iter()
        .filter(|invocation| invocation.command_is("dnf"))
        .any(|invocation| {
            dnf_subcommand_index(&invocation.arguments)
                .is_some_and(|index| invocation.arguments[index] == "install")
                && !invocation
                    .arguments
                    .iter()
                    .any(|argument| matches!(argument.as_str(), "-y" | "--assumeyes"))
        })
}

fn dnf_subcommand_index(arguments: &[String]) -> Option<usize> {
    // dnf_subcommand_index intentionally delegates to yum_subcommand_index
    // because DNF preserves YUM-compatible command-line parsing here.
    yum_subcommand_index(arguments)
}

fn dnf_install_missing_clean_all(shell: &str) -> bool {
    let invocations = detect_command_invocations(shell);
    let last_install_index = invocations.iter().rposition(|invocation| {
        invocation.command_is("dnf")
            && dnf_subcommand_index(&invocation.arguments)
                .is_some_and(|index| invocation.arguments[index] == "install")
    });

    let has_clean_after_last_install = last_install_index.is_some_and(|install_index| {
        invocations
            .iter()
            .skip(install_index + 1)
            .any(|invocation| {
                if invocation.command != "dnf" {
                    return false;
                }

                let Some(clean_index) = dnf_subcommand_index(&invocation.arguments) else {
                    return false;
                };

                invocation.arguments[clean_index] == "clean"
                    && invocation
                        .arguments
                        .iter()
                        .skip(clean_index + 1)
                        .any(|argument| argument == "all")
            })
    });

    last_install_index.is_some() && !has_clean_after_last_install
}

fn dnf_install_has_unpinned_packages(shell: &str) -> bool {
    detect_command_invocations(shell)
        .into_iter()
        .filter(|invocation| invocation.command_is("dnf"))
        .any(|invocation| {
            let Some(install_index) = dnf_subcommand_index(&invocation.arguments) else {
                return false;
            };
            if invocation.arguments[install_index] != "install" {
                return false;
            }

            let mut expect_option_value = false;
            for argument in invocation.arguments.iter().skip(install_index + 1) {
                if argument == "\\" {
                    continue;
                }

                if expect_option_value {
                    expect_option_value = false;
                    continue;
                }

                if yum_option_takes_value(argument) {
                    expect_option_value = true;
                    continue;
                }

                if argument.starts_with('-') {
                    continue;
                }

                if !rpm_package_has_version(argument) {
                    return true;
                }
            }

            false
        })
}

pub(super) fn planned_catalog() -> Vec<&'static str> {
    Vec::new()
}
