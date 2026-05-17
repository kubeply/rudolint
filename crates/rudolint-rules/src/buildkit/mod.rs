use crate::{Rule, RuleInfo, metadata::diagnostic, metadata::rule_metadata};
use rudolint_diagnostics::{Finding, Severity};
use rudolint_dockerfile::{Dockerfile, Mount};
use rudolint_fix::{FixApplicability, FixPreview, TextEdit};
use rudolint_shell::{ShellCommandInvocation, detect_command_invocations};

pub(crate) fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(BuildkitSyntaxWhenFeaturesUsed),
        Box::new(SecretLikeArgOrEnv),
        Box::new(SecretInRun),
        Box::new(CacheMountForPackageInstall),
        Box::new(SecretMountCopiedToLayer),
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

#[cfg(test)]
mod tests {
    use super::{
        has_secret_like_arg_or_env_name, invocation_copies_secret, path_is_at_or_under,
        source_operands,
    };
    use rudolint_shell::ShellCommandInvocation;

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

    #[test]
    fn secret_copy_detection_uses_source_operands_only() {
        let secret_targets = vec!["/run/secrets/api_token".to_string()];

        assert!(invocation_copies_secret(
            &ShellCommandInvocation {
                command: "install".to_string(),
                arguments: vec![
                    "-m".to_string(),
                    "0600".to_string(),
                    "/run/secrets/api_token".to_string(),
                    "/app/token".to_string(),
                ],
            },
            &secret_targets,
        ));
        assert!(invocation_copies_secret(
            &ShellCommandInvocation {
                command: "cp".to_string(),
                arguments: vec![
                    "-t".to_string(),
                    "/app".to_string(),
                    "/run/secrets/api_token".to_string(),
                ],
            },
            &secret_targets,
        ));
        assert!(!invocation_copies_secret(
            &ShellCommandInvocation {
                command: "cp".to_string(),
                arguments: vec![
                    "/tmp/source".to_string(),
                    "/run/secrets/api_token".to_string(),
                ],
            },
            &secret_targets,
        ));
        assert!(path_is_at_or_under(
            "/run/secrets/api_token",
            "/run/secrets/api_token"
        ));
        assert!(!path_is_at_or_under(
            "/run/secrets/api_token.bak",
            "/run/secrets/api_token"
        ));
    }

    #[test]
    fn source_operand_detection_handles_target_directory_flags() {
        assert_eq!(
            source_operands(
                "cp",
                &["-t".to_string(), "/app".to_string(), "secret".to_string()]
            ),
            vec!["secret"]
        );
        assert_eq!(
            source_operands(
                "install",
                &[
                    "-m".to_string(),
                    "0600".to_string(),
                    "--target-directory".to_string(),
                    "/app".to_string(),
                    "secret".to_string(),
                ],
            ),
            vec!["secret"]
        );
        assert_eq!(
            source_operands("rsync", &["secret".to_string(), "/app".to_string()]),
            vec!["secret"]
        );
        assert_eq!(
            source_operands(
                "rsync",
                &["--target-directory=/app".to_string(), "secret".to_string()]
            ),
            vec!["secret"]
        );
        assert_eq!(
            source_operands(
                "rsync",
                &[
                    "--target-directory".to_string(),
                    "/app".to_string(),
                    "secret".to_string()
                ]
            ),
            vec!["secret"]
        );
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
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| {
                let secret_targets = instruction
                    .mounts
                    .iter()
                    .filter(|mount| mount.mount_type == "secret")
                    .filter_map(secret_mount_target)
                    .collect::<Vec<_>>();

                instruction
                    .run
                    .as_ref()
                    .and_then(|run| run.shell.as_ref())
                    .is_some_and(|shell| {
                        !secret_targets.is_empty()
                            && detect_command_invocations(&shell.text)
                                .iter()
                                .any(|invocation| {
                                    invocation_copies_secret(invocation, &secret_targets)
                                })
                    })
            })
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

fn secret_mount_target(mount: &Mount) -> Option<String> {
    let option = |name: &str| {
        mount
            .options
            .iter()
            .find_map(|(key, value)| (key == name && !value.is_empty()).then(|| value.clone()))
    };

    option("target")
        .or_else(|| option("dst"))
        .or_else(|| option("destination"))
        .or_else(|| option("id").map(|id| format!("/run/secrets/{id}")))
        .filter(|target| target != "/")
}

fn invocation_copies_secret(
    invocation: &ShellCommandInvocation,
    secret_targets: &[String],
) -> bool {
    if !matches!(invocation.command.as_str(), "cp" | "install" | "rsync") {
        return false;
    }

    source_operands(&invocation.command, &invocation.arguments)
        .into_iter()
        .any(|operand| {
            secret_targets
                .iter()
                .any(|target| path_is_at_or_under(operand, target))
        })
}

fn source_operands<'a>(command: &str, arguments: &'a [String]) -> Vec<&'a str> {
    let mut operands = Vec::new();
    let mut target_directory = false;
    let mut skip_next = false;
    let mut end_of_options = false;

    for argument in arguments {
        if skip_next {
            skip_next = false;
            continue;
        }

        if end_of_options {
            operands.push(argument.as_str());
            continue;
        }

        match argument.as_str() {
            "--" => {
                end_of_options = true;
            }
            "-t" | "--target-directory" if matches!(command, "cp" | "install") => {
                target_directory = true;
                skip_next = true;
            }
            "--target-directory" if command == "rsync" => {
                target_directory = true;
                skip_next = true;
            }
            "-m" | "--mode" | "-o" | "--owner" | "-g" | "--group" if command == "install" => {
                skip_next = true;
            }
            _ if argument.starts_with("--target-directory=")
                && matches!(command, "cp" | "install" | "rsync") =>
            {
                target_directory = true;
            }
            _ if argument.starts_with('-') => {}
            _ => operands.push(argument.as_str()),
        }
    }

    if target_directory {
        operands
    } else {
        let source_count = operands.len().saturating_sub(1);
        operands.into_iter().take(source_count).collect()
    }
}

fn path_is_at_or_under(path: &str, target: &str) -> bool {
    let path = path.trim_end_matches('/');
    let target = target.trim_end_matches('/');

    path == target
        || path
            .strip_prefix(target)
            .is_some_and(|suffix| suffix.starts_with('/'))
}
