use crate::{Rule, RuleInfo, metadata::diagnostic, metadata::rule_metadata};
use rudolint_buildkit::{
    TARGET_PLATFORM_VARIABLES, final_stage_uses_build_platform, frontend_requirements,
    frontend_version_is_too_old, has_multi_platform_intent, has_secret_like_arg_or_env_name,
    is_official_dockerfile_frontend, missing_buildkit_entitlements, parse_pinned_frontend_version,
    run_copies_secret_mount, run_uses_host_architecture_probe,
    run_uses_lock_based_package_manager_with_shared_cache, ssh_mount_scope_is_broad,
};
use rudolint_config::Config;
use rudolint_diagnostics::{Finding, Severity};
use rudolint_dockerfile::{Dockerfile, Instruction};
use rudolint_fix::{FixApplicability, FixPreview, TextEdit};

pub(crate) fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(BuildkitSyntaxWhenFeaturesUsed),
        Box::new(SecretLikeArgOrEnv),
        Box::new(SecretInRun),
        Box::new(CacheMountForPackageInstall),
        Box::new(SecretMountCopiedToLayer),
        Box::new(SshMountCommandScope),
        Box::new(CacheMountStableId),
        Box::new(CacheMountSafeSharing),
        Box::new(BuildkitEntitlementRequiresOptIn),
        Box::new(MultiPlatformHostArchitecture),
        Box::new(FrontendVersionSupportsSyntax),
    ]
}

rule_metadata!(
    BuildkitSyntaxWhenFeaturesUsed,
    "RDK1000",
    "buildkit-syntax-directive",
    Severity::Info,
    "require explicit syntax directive for BuildKit-only features",
    crate::FixAvailability::Safe
);

impl Rule for BuildkitSyntaxWhenFeaturesUsed {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        if doc.has_buildkit_features && doc.syntax.is_none() {
            let Some(instruction) = doc
                .instructions
                .iter()
                .find(|instruction| instruction.has_buildkit_features())
            else {
                return Vec::new();
            };
            vec![diagnostic(
                "RDK1000",
                Severity::Info,
                "BuildKit features are used without an explicit # syntax directive",
                instruction,
            )]
        } else {
            Vec::new()
        }
    }

    fn fix(&self, doc: &Dockerfile) -> Vec<FixPreview> {
        if doc.has_buildkit_features && doc.syntax.is_none() {
            vec![FixPreview {
                title: "insert BuildKit syntax directive".to_string(),
                applicability: FixApplicability::safe(),
                edits: vec![TextEdit::insert(1, 1, "# syntax=docker/dockerfile:1\n")],
            }]
        } else {
            Vec::new()
        }
    }
}

rule_metadata!(
    SecretLikeArgOrEnv,
    "RDK1001",
    "secret-like-arg-or-env",
    Severity::Warning,
    "reject secret-like ARG and ENV names"
);

impl Rule for SecretLikeArgOrEnv {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        let secret_words = ["SECRET", "TOKEN", "PASSWORD", "PRIVATE_KEY", "ACCESS_KEY"];
        doc.instructions
            .iter()
            .filter(|instruction| matches!(instruction.keyword.as_str(), "ARG" | "ENV"))
            .filter(|instruction| {
                has_secret_like_arg_or_env_name(
                    instruction.keyword.as_str(),
                    &instruction.args,
                    &secret_words,
                )
            })
            .map(|instruction| {
                diagnostic(
                    "RDK1001",
                    Severity::Warning,
                    "secret-like values should use BuildKit secret mounts instead of ARG or ENV",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    SecretInRun,
    "RDK1002",
    "secret-in-run",
    Severity::Warning,
    "prefer BuildKit secret mounts for secret-consuming RUN steps"
);

impl Rule for SecretInRun {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword_is("RUN"))
            .filter(|instruction| {
                let upper = instruction.args.to_ascii_uppercase();
                upper.contains("TOKEN=") || upper.contains("PASSWORD=") || upper.contains("SECRET=")
            })
            .filter(|instruction| {
                !instruction
                    .mounts
                    .iter()
                    .any(|mount| mount.type_is("secret"))
            })
            .map(|instruction| {
                diagnostic(
                    "RDK1002",
                    Severity::Warning,
                    "RUN appears to pass a secret without a type=secret mount",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    CacheMountForPackageInstall,
    "RDK1003",
    "cache-mount-for-package-install",
    Severity::Info,
    "prefer BuildKit cache mounts for package-manager caches"
);

impl Rule for CacheMountForPackageInstall {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword_is("RUN"))
            .filter(|instruction| {
                instruction.args.contains("apt-get install")
                    || instruction.args.contains("apk add")
                    || instruction.args.contains("dnf install")
                    || instruction.args.contains("yum install")
            })
            .filter(|instruction| {
                !instruction
                    .mounts
                    .iter()
                    .any(|mount| mount.type_is("cache"))
            })
            .map(|instruction| {
                diagnostic(
                    "RDK1003",
                    Severity::Info,
                    "package install step can use a BuildKit cache mount for repeat builds",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    SecretMountCopiedToLayer,
    "RDK1004",
    "secret-mount-copied-to-layer",
    Severity::Warning,
    "avoid copying BuildKit secret mount contents into image layers"
);

impl Rule for SecretMountCopiedToLayer {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword_is("RUN"))
            .filter(|instruction| run_copies_secret_mount(instruction))
            .map(|instruction| {
                diagnostic(
                    "RDK1004",
                    Severity::Warning,
                    "secret mount contents appear to be copied into the image layer",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    SshMountCommandScope,
    "RDK1005",
    "ssh-mount-command-scope",
    Severity::Warning,
    "scope BuildKit SSH mounts to the command that needs the agent"
);

impl Rule for SshMountCommandScope {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword_is("RUN"))
            .filter(|instruction| instruction.mounts.iter().any(|mount| mount.type_is("ssh")))
            .filter(|instruction| {
                instruction
                    .run
                    .as_ref()
                    .and_then(|run| run.shell.as_ref())
                    .is_some_and(|shell| ssh_mount_scope_is_broad(&shell.text))
            })
            .map(|instruction| {
                diagnostic(
                    "RDK1005",
                    Severity::Warning,
                    "SSH mount scope is broader than a single command invocation",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    CacheMountStableId,
    "RDK1006",
    "cache-mount-stable-id",
    Severity::Info,
    "require stable cache mount ids in multi-stage builds"
);

impl Rule for CacheMountStableId {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        if doc
            .instructions
            .iter()
            .filter(|instruction| instruction.keyword_is("FROM"))
            .count()
            < 2
        {
            return Vec::new();
        }

        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword_is("RUN"))
            .filter(|instruction| {
                instruction
                    .mounts
                    .iter()
                    .any(|mount| mount.type_is("cache") && mount.option("id").is_none())
            })
            .map(|instruction| {
                diagnostic(
                    "RDK1006",
                    Severity::Info,
                    "cache mount in multi-stage build should declare a stable id",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    CacheMountSafeSharing,
    "RDK1007",
    "cache-mount-safe-sharing",
    Severity::Warning,
    "require safe cache mount sharing for lock-based package managers"
);

impl Rule for CacheMountSafeSharing {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword_is("RUN"))
            .filter(|instruction| {
                run_uses_lock_based_package_manager_with_shared_cache(instruction)
            })
            .map(|instruction| {
                diagnostic(
                    "RDK1007",
                    Severity::Warning,
                    "cache mount for lock-based package manager should use sharing=locked",
                    instruction,
                )
            })
            .collect()
    }
}

rule_metadata!(
    BuildkitEntitlementRequiresOptIn,
    "RDK1008",
    "buildkit-entitlement-opt-in",
    Severity::Warning,
    "require config opt-in for BuildKit network and security entitlements"
);

impl Rule for BuildkitEntitlementRequiresOptIn {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        self.check_with_config(doc, &Config::default())
    }

    fn check_with_config(&self, doc: &Dockerfile, config: &Config) -> Vec<Finding> {
        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword_is("RUN"))
            .flat_map(|instruction| {
                missing_buildkit_entitlements(instruction, &config.allow_entitlements)
                    .into_iter()
                    .map(|entitlement| {
                        diagnostic(
                            "RDK1008",
                            Severity::Warning,
                            format!(
                                "BuildKit entitlement {entitlement} requires allow-entitlements opt-in"
                            ),
                            instruction,
                        )
                    })
            })
            .collect()
    }
}

rule_metadata!(
    MultiPlatformHostArchitecture,
    "RDK1009",
    "multi-platform-host-architecture",
    Severity::Warning,
    "avoid host architecture detection in multi-platform builds"
);

impl Rule for MultiPlatformHostArchitecture {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        if !has_multi_platform_intent(doc) {
            return Vec::new();
        }

        let final_from_index = doc
            .instructions
            .iter()
            .rposition(|instruction| instruction.keyword_is("FROM"));

        let mut stage_has_target_platform_intent = false;
        let mut findings = Vec::new();

        for (index, instruction) in doc.instructions.iter().enumerate() {
            if instruction.keyword_is("FROM") {
                stage_has_target_platform_intent = false;
            }

            let final_stage_mismatch =
                final_stage_uses_build_platform(final_from_index, index, instruction);
            let host_architecture_probe =
                stage_has_target_platform_intent && run_uses_host_architecture_probe(instruction);

            if final_stage_mismatch || host_architecture_probe {
                findings.push(diagnostic(
                    "RDK1009",
                    Severity::Warning,
                    "multi-platform build should use target platform variables instead of host architecture",
                    instruction,
                ));
            }

            if instruction_has_target_platform_intent(instruction) {
                stage_has_target_platform_intent = true;
            }
        }

        findings
    }
}

rule_metadata!(
    FrontendVersionSupportsSyntax,
    "RDK1010",
    "frontend-version-supports-syntax",
    Severity::Warning,
    "require a new enough Dockerfile frontend for used syntax"
);

impl Rule for FrontendVersionSupportsSyntax {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        let Some(frontend) = doc
            .syntax
            .as_ref()
            .filter(|syntax| is_official_dockerfile_frontend(&syntax.image))
            .and_then(|syntax| parse_pinned_frontend_version(&syntax.image))
        else {
            return Vec::new();
        };

        doc.instructions
            .iter()
            .flat_map(|instruction| {
                frontend_requirements(instruction)
                    .into_iter()
                    .filter(|requirement| frontend_version_is_too_old(frontend, requirement))
                    .map(|requirement| {
                        diagnostic(
                            "RDK1010",
                            Severity::Warning,
                            format!(
                                "{} requires Dockerfile frontend {}, but syntax directive pins {}",
                                requirement.feature,
                                requirement.version.display(),
                                frontend.display(),
                            ),
                            instruction,
                        )
                    })
            })
            .collect()
    }
}

fn instruction_has_target_platform_intent(instruction: &Instruction) -> bool {
    instruction
        .arg
        .as_ref()
        .is_some_and(|arg| target_platform_variable(&arg.name))
        || instruction_references_target_platform(&instruction.args)
        || instruction
            .from
            .as_ref()
            .and_then(|from| from.platform.as_deref())
            .is_some_and(instruction_references_target_platform)
}

fn instruction_references_target_platform(value: &str) -> bool {
    TARGET_PLATFORM_VARIABLES
        .iter()
        .any(|variable| value.contains(variable))
}

fn target_platform_variable(name: &str) -> bool {
    matches!(name, "TARGETARCH" | "TARGETOS" | "TARGETPLATFORM")
}
