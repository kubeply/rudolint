use crate::{Rule, RuleInfo, metadata::diagnostic, metadata::rule_metadata};
use rudolint_config::Config;
use rudolint_diagnostics::{Finding, Severity};
use rudolint_dockerfile::{Dockerfile, Instruction, Mount, multi_platform_facts};
use rudolint_fix::{FixApplicability, FixPreview, TextEdit};
use rudolint_shell::{
    PackageManager, ShellCommandInvocation, detect_command_invocations, detect_package_managers,
};

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
        shell_wrapper_command, source_operands,
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
                arguments: vec!["/run/secrets".to_string(), "/app/secrets".to_string(),],
            },
            &secret_targets,
        ));
        assert!(!invocation_copies_secret(
            &ShellCommandInvocation {
                command: "cp".to_string(),
                arguments: vec![
                    "-t/var".to_string(),
                    "/run/secrets".to_string(),
                    "/app/secrets".to_string(),
                ],
            },
            &secret_targets,
        ));
        assert!(!invocation_copies_secret(
            &ShellCommandInvocation {
                command: "rsync".to_string(),
                arguments: vec![
                    "-R".to_string(),
                    "/run/secrets".to_string(),
                    "/app/secrets".to_string(),
                ],
            },
            &secret_targets,
        ));
        assert!(!invocation_copies_secret(
            &ShellCommandInvocation {
                command: "install".to_string(),
                arguments: vec![
                    "-d".to_string(),
                    "/run/secrets".to_string(),
                    "/app/secrets".to_string(),
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
        assert!(invocation_copies_secret(
            &ShellCommandInvocation {
                command: "cp".to_string(),
                arguments: vec![
                    "-r".to_string(),
                    "/run/secrets".to_string(),
                    "/app/secrets".to_string(),
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
            source_operands("cp", &["-t/app".to_string(), "secret".to_string()]),
            vec!["secret"]
        );
        assert_eq!(
            source_operands("cp", &["-rt/app".to_string(), "secret".to_string()]),
            vec!["secret"]
        );
        assert_eq!(
            source_operands(
                "cp",
                &["-rt".to_string(), "/app".to_string(), "secret".to_string()]
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
        assert_eq!(
            source_operands(
                "rsync",
                &[
                    "--files-from".to_string(),
                    "/run/secrets/list".to_string(),
                    "src/".to_string(),
                    "/dest/".to_string(),
                ]
            ),
            vec!["src/"]
        );
        assert_eq!(
            source_operands(
                "install",
                &[
                    "--owner=root".to_string(),
                    "--group".to_string(),
                    "root".to_string(),
                    "secret".to_string(),
                    "/dest".to_string(),
                ]
            ),
            vec!["secret"]
        );
    }

    #[test]
    fn ssh_scope_detection_accepts_path_qualified_shell_wrappers() {
        assert!(shell_wrapper_command("sh"));
        assert!(shell_wrapper_command("/bin/sh"));
        assert!(shell_wrapper_command("/usr/bin/bash"));
        assert!(!shell_wrapper_command("git"));
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

    let copies_directory_contents =
        copies_directory_contents(&invocation.command, &invocation.arguments);

    source_operands(&invocation.command, &invocation.arguments)
        .into_iter()
        .any(|operand| {
            secret_targets.iter().any(|target| {
                path_is_at_or_under(operand, target)
                    || (copies_directory_contents && path_is_at_or_under(target, operand))
            })
        })
}

fn source_operands<'a>(command: &str, arguments: &'a [String]) -> Vec<&'a str> {
    let mut operands = Vec::new();
    let mut target_directory = false;
    let mut directory_mode = false;
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
            _ if argument.starts_with("--") => {
                let behavior = long_option_behavior(command, argument);
                target_directory |= behavior.target_directory;
                directory_mode |= behavior.directory_mode;
                skip_next = behavior.skip_next;
            }
            _ if argument.starts_with('-') => {
                let behavior = short_option_behavior(command, argument);
                target_directory |= behavior.target_directory;
                directory_mode |= behavior.directory_mode;
                skip_next = behavior.skip_next;
            }
            _ => operands.push(argument.as_str()),
        }
    }

    if directory_mode {
        Vec::new()
    } else if target_directory {
        operands
    } else {
        let source_count = operands.len().saturating_sub(1);
        operands.into_iter().take(source_count).collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OptionBehavior {
    skip_next: bool,
    target_directory: bool,
    directory_mode: bool,
}

impl OptionBehavior {
    fn value_argument(takes_value: bool, has_inline_value: bool) -> Self {
        Self {
            skip_next: takes_value && !has_inline_value,
            target_directory: false,
            directory_mode: false,
        }
    }
}

fn long_option_behavior(command: &str, argument: &str) -> OptionBehavior {
    let (name, has_inline_value) = argument
        .split_once('=')
        .map_or((argument, false), |(name, _)| (name, true));

    let takes_value = match command {
        "cp" => matches!(
            name,
            "-S" | "--suffix" | "-t" | "--target-directory" | "--context"
        ),
        "install" => matches!(
            name,
            "-g" | "--group"
                | "-m"
                | "--mode"
                | "-o"
                | "--owner"
                | "-S"
                | "--suffix"
                | "-t"
                | "--target-directory"
                | "--context"
                | "--strip-program"
        ),
        "rsync" => matches!(
            name,
            "-B" | "--block-size"
                | "-e"
                | "--rsh"
                | "-f"
                | "--filter"
                | "-M"
                | "--remote-option"
                | "--files-from"
                | "--exclude-from"
                | "--include-from"
                | "--address"
                | "--backup-dir"
                | "--bwlimit"
                | "--checksum-choice"
                | "--chmod"
                | "--chown"
                | "--compare-dest"
                | "--compress-choice"
                | "--compress-level"
                | "--contimeout"
                | "--copy-dest"
                | "--debug"
                | "--groupmap"
                | "--info"
                | "--link-dest"
                | "--log-file"
                | "--log-file-format"
                | "--max-size"
                | "--min-size"
                | "--modify-window"
                | "--out-format"
                | "--partial-dir"
                | "--port"
                | "--rsync-path"
                | "--sockopts"
                | "--suffix"
                | "--target-directory"
                | "--timeout"
                | "--usermap"
        ),
        _ => false,
    };

    let target_directory = matches!(
        (command, name),
        ("cp" | "install" | "rsync", "--target-directory")
    );
    let directory_mode = matches!((command, name), ("install", "--directory"));

    OptionBehavior {
        skip_next: takes_value && !has_inline_value,
        target_directory,
        directory_mode,
    }
}

fn short_option_behavior(command: &str, argument: &str) -> OptionBehavior {
    let mut behavior = OptionBehavior {
        skip_next: false,
        target_directory: false,
        directory_mode: false,
    };

    let Some(flags) = argument.strip_prefix('-') else {
        return behavior;
    };
    let mut char_indices = flags.char_indices().peekable();
    while let Some((_, flag)) = char_indices.next() {
        let name = match flag {
            'a' => "-a",
            'B' => "-B",
            'd' => "-d",
            'e' => "-e",
            'f' => "-f",
            'g' => "-g",
            'm' => "-m",
            'M' => "-M",
            'o' => "-o",
            'r' => "-r",
            'R' => "-R",
            'S' => "-S",
            't' => "-t",
            _ => "",
        };
        let has_inline_value =
            char_indices.peek().is_some() && short_flag_takes_value(command, name);
        let flag_behavior = short_flag_behavior(command, name, has_inline_value);
        behavior.target_directory |= flag_behavior.target_directory;
        behavior.directory_mode |= flag_behavior.directory_mode;
        behavior.skip_next |= flag_behavior.skip_next;
        if has_inline_value {
            break;
        }
        if flag_behavior.skip_next {
            break;
        }
    }

    behavior
}

fn short_flag_behavior(command: &str, name: &str, has_inline_value: bool) -> OptionBehavior {
    let takes_value = short_flag_takes_value(command, name);
    let mut behavior = OptionBehavior::value_argument(takes_value, has_inline_value);
    behavior.target_directory = matches!((command, name), ("cp" | "install", "-t"));
    behavior.directory_mode = matches!((command, name), ("install", "-d"));
    behavior
}

fn short_flag_takes_value(command: &str, name: &str) -> bool {
    match command {
        "cp" => matches!(name, "-S" | "-t"),
        "install" => matches!(name, "-g" | "-m" | "-o" | "-S" | "-t"),
        "rsync" => matches!(name, "-B" | "-e" | "-f" | "-M"),
        _ => false,
    }
}

fn copies_directory_contents(command: &str, arguments: &[String]) -> bool {
    if !matches!(command, "cp" | "rsync") {
        return false;
    }

    let mut skip_next = false;
    for argument in arguments {
        if skip_next {
            skip_next = false;
            continue;
        }

        if argument == "--" {
            return false;
        }

        if argument.starts_with("--") {
            let name = argument
                .split_once('=')
                .map_or(argument.as_str(), |(name, _)| name);
            if matches!(name, "--archive" | "--recursive") {
                return true;
            }
            skip_next = long_option_behavior(command, argument).skip_next;
            continue;
        }

        if argument.starts_with('-') {
            let behavior = short_recursive_behavior(command, argument);
            if behavior.recursive {
                return true;
            }
            skip_next = behavior.skip_next;
        }
    }

    false
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RecursiveBehavior {
    recursive: bool,
    skip_next: bool,
}

fn short_recursive_behavior(command: &str, argument: &str) -> RecursiveBehavior {
    let Some(flags) = argument.strip_prefix('-') else {
        return RecursiveBehavior {
            recursive: false,
            skip_next: false,
        };
    };

    let mut char_indices = flags.char_indices().peekable();
    while let Some((_, flag)) = char_indices.next() {
        let recursive = match command {
            "cp" => matches!(flag, 'a' | 'r' | 'R'),
            "rsync" => matches!(flag, 'a' | 'r'),
            _ => false,
        };
        if recursive {
            return RecursiveBehavior {
                recursive: true,
                skip_next: false,
            };
        }

        let name = match flag {
            'B' => "-B",
            'e' => "-e",
            'f' => "-f",
            'M' => "-M",
            'S' => "-S",
            't' => "-t",
            _ => "",
        };
        let has_inline_value =
            char_indices.peek().is_some() && short_flag_takes_value(command, name);
        let flag_behavior = short_flag_behavior(command, name, has_inline_value);
        if has_inline_value || flag_behavior.skip_next {
            return RecursiveBehavior {
                recursive: false,
                skip_next: flag_behavior.skip_next,
            };
        }
    }

    RecursiveBehavior {
        recursive: false,
        skip_next: false,
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
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| {
                instruction
                    .mounts
                    .iter()
                    .any(|mount| mount.mount_type == "ssh")
            })
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

fn ssh_mount_scope_is_broad(shell: &str) -> bool {
    let invocations = detect_command_invocations(shell);
    invocations.len() > 1
        || invocations
            .first()
            .is_some_and(|invocation| shell_wrapper_command(&invocation.command))
}

fn shell_wrapper_command(command: &str) -> bool {
    let wrapper_name = command.rsplit('/').next().unwrap_or(command);
    matches!(wrapper_name, "sh" | "bash" | "dash" | "ash" | "zsh")
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
            .filter(|instruction| instruction.keyword == "FROM")
            .count()
            < 2
        {
            return Vec::new();
        }

        doc.instructions
            .iter()
            .filter(|instruction| instruction.keyword == "RUN")
            .filter(|instruction| {
                instruction
                    .mounts
                    .iter()
                    .any(|mount| mount.mount_type == "cache" && mount_option(mount, "id").is_none())
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

fn mount_option<'a>(mount: &'a Mount, name: &str) -> Option<&'a str> {
    mount
        .options
        .iter()
        .find_map(|(key, value)| (key == name && !value.is_empty()).then_some(value.as_str()))
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
            .filter(|instruction| instruction.keyword == "RUN")
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

fn run_uses_lock_based_package_manager_with_shared_cache(instruction: &Instruction) -> bool {
    let detected_lock_managers = instruction
        .run
        .as_ref()
        .and_then(|run| run.shell.as_ref())
        .map(|shell| {
            detect_package_managers(&shell.text)
                .into_iter()
                .filter(|manager| package_manager_needs_locked_cache(*manager))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    !detected_lock_managers.is_empty()
        && instruction.mounts.iter().any(|mount| {
            mount.mount_type == "cache"
                && cache_mount_matches_lock_manager(mount, &detected_lock_managers)
                && matches!(mount_option(mount, "sharing"), None | Some("shared"))
        })
}

fn cache_mount_matches_lock_manager(mount: &Mount, managers: &[PackageManager]) -> bool {
    let target = mount_option(mount, "target")
        .or_else(|| mount_option(mount, "dst"))
        .or_else(|| mount_option(mount, "destination"));

    target.is_some_and(|target| {
        managers.iter().any(|manager| match manager {
            PackageManager::Apt | PackageManager::AptGet => {
                target.starts_with("/var/cache/apt") || target.starts_with("/var/lib/apt")
            }
            PackageManager::Dnf | PackageManager::Microdnf => target.starts_with("/var/cache/dnf"),
            PackageManager::Yum => target.starts_with("/var/cache/yum"),
            _ => false,
        })
    })
}

fn package_manager_needs_locked_cache(manager: PackageManager) -> bool {
    matches!(
        manager,
        PackageManager::AptGet
            | PackageManager::Apt
            | PackageManager::Dnf
            | PackageManager::Yum
            | PackageManager::Microdnf
    )
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
            .filter(|instruction| instruction.keyword == "RUN")
            .flat_map(|instruction| {
                missing_buildkit_entitlements(instruction, config)
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

fn missing_buildkit_entitlements(instruction: &Instruction, config: &Config) -> Vec<&'static str> {
    let Some(run) = instruction.run.as_ref() else {
        return Vec::new();
    };

    let mut entitlements = Vec::new();
    if run.network.as_deref() == Some("host") && !config.allow_entitlements.contains("network.host")
    {
        entitlements.push("network.host");
    }
    if run.security.as_deref() == Some("insecure")
        && !config.allow_entitlements.contains("security.insecure")
    {
        entitlements.push("security.insecure");
    }

    entitlements
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
            .rposition(|instruction| instruction.keyword == "FROM");

        doc.instructions
            .iter()
            .enumerate()
            .filter(|(index, instruction)| {
                final_stage_uses_build_platform(final_from_index, *index, instruction)
                    || run_uses_host_architecture_probe(instruction)
            })
            .map(|(_, instruction)| {
                diagnostic(
                    "RDK1009",
                    Severity::Warning,
                    "multi-platform build should use target platform variables instead of host architecture",
                    instruction,
                )
            })
            .collect()
    }
}

fn has_multi_platform_intent(doc: &Dockerfile) -> bool {
    let facts = multi_platform_facts(doc);
    facts.targetplatform.is_some()
        || facts.buildplatform.is_some()
        || facts.targetarch.is_some()
        || facts.targetos.is_some()
        || facts
            .stage_platforms
            .iter()
            .any(|stage| platform_references_buildx_variable(&stage.platform))
        || doc.instructions.iter().any(|instruction| {
            instruction.arg.as_ref().is_some_and(|arg| {
                matches!(
                    arg.name.as_str(),
                    "TARGETPLATFORM" | "BUILDPLATFORM" | "TARGETARCH" | "TARGETOS"
                )
            })
        })
}

fn platform_references_buildx_variable(platform: &str) -> bool {
    [
        "$TARGETPLATFORM",
        "${TARGETPLATFORM}",
        "$BUILDPLATFORM",
        "${BUILDPLATFORM}",
    ]
    .iter()
    .any(|variable| platform.contains(variable))
}

fn final_stage_uses_build_platform(
    final_from_index: Option<usize>,
    index: usize,
    instruction: &Instruction,
) -> bool {
    final_from_index == Some(index)
        && instruction
            .from
            .as_ref()
            .and_then(|from| from.platform.as_deref())
            .is_some_and(platform_references_build_platform)
}

fn platform_references_build_platform(platform: &str) -> bool {
    ["$BUILDPLATFORM", "${BUILDPLATFORM}"]
        .iter()
        .any(|variable| platform.contains(variable))
}

fn run_uses_host_architecture_probe(instruction: &Instruction) -> bool {
    instruction
        .run
        .as_ref()
        .and_then(|run| run.shell.as_ref())
        .is_some_and(|shell| {
            !shell_references_target_platform(&shell.text)
                && (shell_contains_host_architecture_probe(&shell.text)
                    || detect_command_invocations(&shell.text)
                        .iter()
                        .any(invocation_uses_host_architecture_probe))
        })
}

fn shell_references_target_platform(shell: &str) -> bool {
    [
        "$TARGETARCH",
        "${TARGETARCH}",
        "$TARGETPLATFORM",
        "${TARGETPLATFORM}",
        "$TARGETOS",
        "${TARGETOS}",
    ]
    .iter()
    .any(|variable| shell.contains(variable))
}

fn shell_contains_host_architecture_probe(shell: &str) -> bool {
    [
        "uname -m",
        "uname -p",
        "uname -i",
        "uname --machine",
        "uname --processor",
        "uname --hardware-platform",
        "dpkg --print-architecture",
        "apk --print-arch",
    ]
    .iter()
    .any(|probe| shell.contains(probe))
}

fn invocation_uses_host_architecture_probe(invocation: &ShellCommandInvocation) -> bool {
    match invocation.command.as_str() {
        "arch" => true,
        "uname" => invocation.arguments.iter().any(|argument| {
            matches!(
                argument.as_str(),
                "-m" | "-p" | "-i" | "--machine" | "--processor" | "--hardware-platform"
            )
        }),
        "dpkg" => invocation
            .arguments
            .iter()
            .any(|argument| argument == "--print-architecture"),
        "apk" => invocation
            .arguments
            .iter()
            .any(|argument| argument == "--print-arch"),
        _ => false,
    }
}
