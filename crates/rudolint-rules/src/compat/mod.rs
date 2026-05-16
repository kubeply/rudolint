use std::collections::{BTreeMap, BTreeSet};

use crate::{Rule, RuleInfo, metadata::diagnostic, metadata::rule_metadata};
use rudolint_config::Config;
use rudolint_diagnostics::{Finding, Severity};
use rudolint_dockerfile::{Comment, CopyKind, Dockerfile, Instruction, InstructionForm};
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
    UseAddForArchives,
    "RDL3010",
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
            .filter(|instruction| instruction.keyword == "COPY")
            .filter(|instruction| !copy_uses_from_flag(&instruction.args))
            .filter(|instruction| {
                add_sources(&instruction.args)
                    .iter()
                    .any(|source| is_archive_source(source))
            })
            .map(|instruction| {
                diagnostic(
                    "RDL3010",
                    Severity::Info,
                    "use ADD when local archive extraction is intended",
                    instruction,
                )
            })
            .collect()
    }
}

fn copy_uses_from_flag(args: &str) -> bool {
    let mut parts = args.split_whitespace();
    while let Some(part) = parts.next() {
        if part.starts_with("--from=") {
            return true;
        }

        if part == "--from" && parts.next().is_some() {
            return true;
        }
    }

    false
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
    PinPipVersions,
    "RDL3013",
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
                    "RDL3013",
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
    "RDL3014",
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
                    "RDL3014",
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
    "RDL3015",
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
                    "RDL3015",
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
    "RDL3016",
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
                    "RDL3016",
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
    "RDL3018",
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
                    "RDL3018",
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
    "RDL3019",
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
                    "RDL3019",
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
    CopyMultipleDestinationSlash,
    "RDL3021",
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
                    "RDL3021",
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
    "RDL3022",
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
                    "RDL3022",
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
    "RDL3023",
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
                    "RDL3023",
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
    TrustedRegistries,
    "RDL3026",
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
                    "RDL3026",
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
    "RDL3027",
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
                    "RDL3027",
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
    "RDL3028",
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
                    "RDL3028",
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
    "RDL3029",
    "no-from-platform-flag",
    Severity::Warning,
    "avoid --platform in FROM"
);

impl Rule for NoFromPlatformFlag {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "FROM")
            .filter(|instruction| instruction.flags.iter().any(|(name, _)| name == "platform"))
            .map(|instruction| {
                diagnostic(
                    "RDL3029",
                    Severity::Warning,
                    "avoid `--platform` in FROM; prefer build-time platform selection",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    YumInstallAssumeYes,
    "RDL3030",
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
                    "RDL3030",
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
    "RDL3032",
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
                    "RDL3032",
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
    "RDL3033",
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
                    "RDL3033",
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
    "RDL3034",
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
                    "RDL3034",
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
    "RDL3035",
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
                    "RDL3035",
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
    "RDL3036",
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
                    "RDL3036",
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
    "RDL3037",
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
                    "RDL3037",
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
    "RDL3038",
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
                    "RDL3038",
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
    "RDL3040",
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
                    "RDL3040",
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
    "RDL3041",
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
                    "RDL3041",
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
    "RDL3042",
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
                        "RDL3042",
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
    "RDL3043",
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
                    "RDL3043",
                    Severity::Error,
                    "`ONBUILD`, `FROM`, or `MAINTAINER` triggered from within `ONBUILD` instruction",
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

fn pip_install_has_unpinned_packages(shell: &str) -> bool {
    detect_command_invocations(shell)
        .into_iter()
        .filter(|invocation| matches!(invocation.command.as_str(), "pip" | "pip3"))
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
    if matches!(invocation.command.as_str(), "pip" | "pip3") {
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
        matches!(mount.mount_type.as_str(), "cache" | "tmpfs")
            && mount
                .options
                .iter()
                .any(|(name, value)| name == "target" && value.contains(".cache/pip"))
    })
}

fn onbuild_has_disallowed_trigger(args: &str) -> bool {
    let mut tokens = args.split_whitespace();
    matches!(
        tokens.next().map(|token| token.to_ascii_uppercase()),
        Some(trigger) if matches!(trigger.as_str(), "ONBUILD" | "FROM" | "MAINTAINER")
    )
}

fn apt_get_install_missing_yes(shell: &str) -> bool {
    detect_command_invocations(shell)
        .into_iter()
        .filter(|invocation| invocation.command == "apt-get")
        .any(|invocation| {
            let has_install = apt_get_subcommand_index(&invocation.arguments)
                .is_some_and(|index| invocation.arguments[index] == "install");
            let has_assume_yes = invocation
                .arguments
                .iter()
                .any(|argument| matches!(argument.as_str(), "-y" | "--yes" | "--assume-yes"));

            has_install && !has_assume_yes
        })
}

fn apt_get_install_missing_no_install_recommends(shell: &str) -> bool {
    detect_command_invocations(shell)
        .into_iter()
        .filter(|invocation| invocation.command == "apt-get")
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
        .filter(|invocation| invocation.command == "npm")
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
        .filter(|invocation| invocation.command == "apk")
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
        .filter(|invocation| invocation.command == "apk")
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
            .is_some_and(|destination| !destination.ends_with('/'))
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
        .any(|invocation| invocation.command == "apt")
}

fn gem_install_has_unpinned_packages(shell: &str) -> bool {
    detect_command_invocations(shell)
        .into_iter()
        .filter(|invocation| invocation.command == "gem")
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
        .filter(|invocation| invocation.command == "yum")
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
        invocation.command == "yum"
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
        .filter(|invocation| invocation.command == "yum")
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
        .filter(|invocation| invocation.command == "zypper")
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
        .filter(|invocation| invocation.command == "zypper")
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
        .filter(|invocation| invocation.command == "zypper")
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
        .filter(|invocation| invocation.command == "dnf")
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
        invocation.command == "dnf"
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
        .filter(|invocation| invocation.command == "dnf")
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

pub(crate) fn planned_catalog() -> Vec<&'static str> {
    vec![
        "RDL3044", "RDL3045", "RDL3046", "RDL3047", "RDL3048", "RDL3049", "RDL3050", "RDL3051",
        "RDL3052", "RDL3053", "RDL3054", "RDL3055", "RDL3056", "RDL3057", "RDL3058", "RDL3059",
        "RDL3060", "RDL3061", "RDL3062", "RDL3063", "RDL4001", "RDL4005", "RDL4006",
    ]
}
