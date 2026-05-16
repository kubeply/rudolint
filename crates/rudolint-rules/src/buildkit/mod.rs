use crate::{Rule, metadata::diagnostic, metadata::rule};
use rudolint_diagnostics::Severity;
use rudolint_dockerfile::Dockerfile;

pub(crate) fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(BuildkitSyntaxWhenFeaturesUsed),
        Box::new(SecretLikeArgOrEnv),
        Box::new(SecretInRun),
        Box::new(CacheMountForPackageInstall),
    ]
}

rule!(
    BuildkitSyntaxWhenFeaturesUsed,
    "RDK1000",
    "buildkit-syntax-directive",
    Severity::Info,
    "require explicit syntax directive for BuildKit-only features",
    |doc: &Dockerfile| {
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
);

rule!(
    SecretLikeArgOrEnv,
    "RDK1001",
    "secret-like-arg-or-env",
    Severity::Warning,
    "reject secret-like ARG and ENV names",
    |doc: &Dockerfile| {
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
);

fn has_secret_like_arg_or_env_name(keyword: &str, args: &str, secret_words: &[&str]) -> bool {
    let has_secret_like_name = |name: &str| {
        let upper_name = name.to_ascii_uppercase();
        secret_words.iter().any(|word| upper_name.contains(word))
    };

    match keyword {
        "ARG" => args
            .split('=')
            .next()
            .is_some_and(|name| has_secret_like_name(name.trim())),
        "ENV" => {
            let args = args.trim();
            if args.contains('=') {
                args.split_whitespace()
                    .filter_map(|pair| pair.split('=').next())
                    .any(|name| has_secret_like_name(name.trim()))
            } else {
                args.split_whitespace()
                    .next()
                    .is_some_and(|name| has_secret_like_name(name.trim()))
            }
        }
        _ => false,
    }
}

rule!(
    SecretInRun,
    "RDK1002",
    "secret-in-run",
    Severity::Warning,
    "prefer BuildKit secret mounts for secret-consuming RUN steps",
    |doc: &Dockerfile| {
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
);

#[cfg(test)]
mod tests {
    use super::has_secret_like_arg_or_env_name;

    const SECRET_WORDS: &[&str] = &["SECRET", "TOKEN", "PASSWORD", "PRIVATE_KEY", "ACCESS_KEY"];

    #[test]
    fn secret_like_arg_or_env_detection_uses_names_only() {
        assert!(has_secret_like_arg_or_env_name(
            "ARG",
            "API_TOKEN=placeholder",
            SECRET_WORDS
        ));
        assert!(has_secret_like_arg_or_env_name(
            "ENV",
            "API_TOKEN=placeholder OTHER=value",
            SECRET_WORDS
        ));
        assert!(has_secret_like_arg_or_env_name(
            "ENV",
            "API_TOKEN placeholder",
            SECRET_WORDS
        ));

        assert!(!has_secret_like_arg_or_env_name(
            "ARG",
            "NAME=TOKEN",
            SECRET_WORDS
        ));
        assert!(!has_secret_like_arg_or_env_name(
            "ENV",
            "NAME=TOKEN OTHER=value",
            SECRET_WORDS
        ));
        assert!(!has_secret_like_arg_or_env_name(
            "ENV",
            "NAME TOKEN",
            SECRET_WORDS
        ));
    }
}

rule!(
    CacheMountForPackageInstall,
    "RDK1003",
    "cache-mount-for-package-install",
    Severity::Info,
    "prefer BuildKit cache mounts for package-manager caches",
    |doc: &Dockerfile| {
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
);
