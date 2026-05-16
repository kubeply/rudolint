use crate::{Rule, RuleInfo, metadata::diagnostic, metadata::rule_metadata};
use rudolint_diagnostics::{Finding, Severity};
use rudolint_dockerfile::Dockerfile;

pub(crate) fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(BuildkitSyntaxWhenFeaturesUsed),
        Box::new(SecretLikeArgOrEnv),
        Box::new(SecretInRun),
        Box::new(CacheMountForPackageInstall),
    ]
}

rule_metadata!(
    BuildkitSyntaxWhenFeaturesUsed,
    "RDK1000",
    "buildkit-syntax-directive",
    Severity::Info,
    "require explicit syntax directive for BuildKit-only features"
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
                let upper = instruction.args.to_ascii_uppercase();
                secret_words.iter().any(|word| upper.contains(word))
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
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| {
                let upper = instruction.args.to_ascii_uppercase();
                upper.contains("TOKEN=") || upper.contains("PASSWORD=") || upper.contains("SECRET=")
            })
            .filter(|instruction| {
                !instruction
                    .mounts
                    .iter()
                    .any(|mount| mount.mount_type == "secret")
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
            .filter(|instruction| instruction.keyword == "RUN")
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
                    .any(|mount| mount.mount_type == "cache")
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
