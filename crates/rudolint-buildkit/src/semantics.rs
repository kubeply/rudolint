use rudolint_dockerfile::{Dockerfile, Instruction, Mount, multi_platform_facts};
use rudolint_shell::{
    PackageManager, ShellCommandInvocation, detect_command_invocations, detect_package_managers,
};
use std::collections::BTreeSet;

pub fn has_secret_like_arg_or_env_name(keyword: &str, args: &str, secret_words: &[&str]) -> bool {
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

pub fn secret_mount_target(mount: &Mount) -> Option<String> {
    mount
        .target()
        .map(ToString::to_string)
        .or_else(|| mount.option("id").map(|id| format!("/run/secrets/{id}")))
        .filter(|target| target != "/")
}

pub fn invocation_copies_secret(
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

pub fn run_copies_secret_mount(instruction: &Instruction) -> bool {
    let secret_targets = instruction
        .mounts
        .iter()
        .filter(|mount| mount.type_is("secret"))
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
                    .any(|invocation| invocation_copies_secret(invocation, &secret_targets))
        })
}

pub fn source_operands<'a>(command: &str, arguments: &'a [String]) -> Vec<&'a str> {
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
        if has_inline_value || flag_behavior.skip_next {
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

pub fn path_is_at_or_under(path: &str, target: &str) -> bool {
    let path = path.trim_end_matches('/');
    let target = target.trim_end_matches('/');

    path == target
        || path
            .strip_prefix(target)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

pub fn ssh_mount_scope_is_broad(shell: &str) -> bool {
    let invocations = detect_command_invocations(shell);
    invocations.len() > 1
        || invocations
            .first()
            .is_some_and(|invocation| shell_wrapper_command(&invocation.command))
}

pub fn shell_wrapper_command(command: &str) -> bool {
    let wrapper_name = command.rsplit('/').next().unwrap_or(command);
    matches!(wrapper_name, "sh" | "bash" | "dash" | "ash" | "zsh")
}

pub fn run_uses_lock_based_package_manager_with_shared_cache(instruction: &Instruction) -> bool {
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
            mount.type_is("cache")
                && cache_mount_matches_lock_manager(mount, &detected_lock_managers)
                && matches!(mount.option("sharing"), None | Some("shared"))
        })
}

fn cache_mount_matches_lock_manager(mount: &Mount, managers: &[PackageManager]) -> bool {
    mount.target().is_some_and(|target| {
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

pub fn missing_buildkit_entitlements(
    instruction: &Instruction,
    allowed_entitlements: &BTreeSet<String>,
) -> Vec<&'static str> {
    let Some(run) = instruction.run.as_ref() else {
        return Vec::new();
    };

    let mut entitlements = Vec::new();
    if run.network.as_deref() == Some("host") && !allowed_entitlements.contains("network.host") {
        entitlements.push("network.host");
    }
    if run.security.as_deref() == Some("insecure")
        && !allowed_entitlements.contains("security.insecure")
    {
        entitlements.push("security.insecure");
    }

    entitlements
}

pub fn has_multi_platform_intent(doc: &Dockerfile) -> bool {
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

pub fn final_stage_uses_build_platform(
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

pub fn run_uses_host_architecture_probe(instruction: &Instruction) -> bool {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrontendVersion {
    pub major: u16,
    pub minor: Option<u16>,
    pub patch: Option<u16>,
    pub labs: bool,
}

impl FrontendVersion {
    pub fn display(self) -> String {
        let mut version = self.major.to_string();
        if let Some(minor) = self.minor {
            version.push('.');
            version.push_str(&minor.to_string());
        }
        if let Some(patch) = self.patch {
            version.push('.');
            version.push_str(&patch.to_string());
        }
        if self.labs {
            version.push_str("-labs");
        }
        version
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrontendRequirement {
    pub feature: &'static str,
    pub version: FrontendVersion,
}

impl FrontendRequirement {
    const fn stable(feature: &'static str, major: u16, minor: u16) -> Self {
        Self {
            feature,
            version: FrontendVersion {
                major,
                minor: Some(minor),
                patch: None,
                labs: false,
            },
        }
    }

    const fn labs(feature: &'static str, major: u16, minor: u16) -> Self {
        Self {
            feature,
            version: FrontendVersion {
                major,
                minor: Some(minor),
                patch: None,
                labs: true,
            },
        }
    }
}

pub fn is_official_dockerfile_frontend(image: &str) -> bool {
    let reference = image
        .split_once('@')
        .map_or(image, |(reference, _)| reference);
    let Some((name, _)) = reference.rsplit_once(':') else {
        return false;
    };

    matches!(
        name,
        "docker/dockerfile"
            | "docker.io/docker/dockerfile"
            | "index.docker.io/docker/dockerfile"
            | "registry-1.docker.io/docker/dockerfile"
    )
}

pub fn parse_pinned_frontend_version(image: &str) -> Option<FrontendVersion> {
    let reference = image
        .split_once('@')
        .map_or(image, |(reference, _)| reference);
    let (_, tag) = reference.rsplit_once(':')?;
    let (version, labs) = tag
        .strip_suffix("-labs")
        .map_or((tag, false), |version| (version, true));
    let parts = version.split('.').collect::<Vec<_>>();
    if parts.len() < 2 || parts.len() > 3 {
        return None;
    }

    let major = parts.first()?.parse().ok()?;
    let minor = parts.get(1)?.parse().ok()?;
    let patch = match parts.get(2) {
        Some(patch) => Some(patch.parse().ok()?),
        None => None,
    };

    Some(FrontendVersion {
        major,
        minor: Some(minor),
        patch,
        labs,
    })
}

pub fn frontend_requirements(instruction: &Instruction) -> Vec<FrontendRequirement> {
    let mut requirements = Vec::new();

    if !instruction.heredocs.is_empty() {
        requirements.push(FrontendRequirement::stable(
            "Dockerfile here-document syntax",
            1,
            4,
        ));
    }

    match instruction.keyword.as_str() {
        "RUN" => {
            if instruction.has_flag("device") {
                requirements.push(FrontendRequirement::labs("RUN --device", 1, 14));
            }
            if instruction.has_flag("mount") {
                requirements.push(FrontendRequirement::stable("RUN --mount", 1, 2));
            }
            if instruction.has_flag("network") {
                requirements.push(FrontendRequirement::stable("RUN --network", 1, 3));
            }
            if instruction.has_flag("security") {
                requirements.push(FrontendRequirement::stable("RUN --security", 1, 20));
            }
        }
        "ADD" => {
            if instruction.has_flag("keep-git-dir") {
                requirements.push(FrontendRequirement::stable("ADD --keep-git-dir", 1, 1));
            }
            if instruction.has_flag("checksum") {
                requirements.push(FrontendRequirement::stable("ADD --checksum", 1, 6));
            }
            if let Some(value) = instruction.flag_value("chmod") {
                requirements.push(chmod_frontend_requirement("ADD --chmod", value));
            }
            if instruction.has_flag("link") {
                requirements.push(FrontendRequirement::stable("ADD --link", 1, 4));
            }
            if instruction.has_flag("unpack") {
                requirements.push(FrontendRequirement::stable("ADD --unpack", 1, 17));
            }
            if instruction.has_flag("exclude") {
                requirements.push(FrontendRequirement::stable("ADD --exclude", 1, 19));
            }
        }
        "COPY" => {
            if let Some(value) = instruction.flag_value("chmod") {
                requirements.push(chmod_frontend_requirement("COPY --chmod", value));
            }
            if instruction.has_flag("link") {
                requirements.push(FrontendRequirement::stable("COPY --link", 1, 4));
            }
            if instruction.has_flag("parents") {
                requirements.push(FrontendRequirement::stable("COPY --parents", 1, 20));
            }
            if instruction.has_flag("exclude") {
                requirements.push(FrontendRequirement::stable("COPY --exclude", 1, 19));
            }
        }
        _ => {}
    }

    requirements
}

fn chmod_frontend_requirement(feature: &'static str, value: &str) -> FrontendRequirement {
    if chmod_value_is_symbolic(value) {
        FrontendRequirement::stable(feature, 1, 14)
    } else {
        FrontendRequirement::stable(feature, 1, 2)
    }
}

pub fn chmod_value_is_symbolic(value: &str) -> bool {
    value.chars().any(|character| {
        matches!(
            character,
            '+' | '-' | '=' | 'u' | 'g' | 'o' | 'a' | 'r' | 'w' | 'x' | 'X' | 's' | 't'
        )
    })
}

pub fn frontend_version_is_too_old(
    frontend: FrontendVersion,
    requirement: &FrontendRequirement,
) -> bool {
    if requirement.version.labs && !frontend.labs {
        return true;
    }
    if frontend.major != requirement.version.major {
        return frontend.major < requirement.version.major;
    }

    let frontend_minor = frontend.minor.unwrap_or_default();
    let required_minor = requirement.version.minor.unwrap_or_default();
    if frontend_minor != required_minor {
        return frontend_minor < required_minor;
    }

    frontend.patch.unwrap_or_default() < requirement.version.patch.unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
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
                arguments: vec!["/run/secrets".to_string(), "/app/secrets".to_string()],
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

    #[test]
    fn frontend_version_parser_handles_digests_and_rejects_invalid_patches() {
        let version =
            parse_pinned_frontend_version("docker/dockerfile:1.20@sha256:abcdef").unwrap();
        assert_eq!(version.display(), "1.20");

        let version = parse_pinned_frontend_version("docker/dockerfile:1.14-labs").unwrap();
        assert_eq!(version.display(), "1.14-labs");

        assert!(parse_pinned_frontend_version("docker/dockerfile:1").is_none());
        assert!(parse_pinned_frontend_version("docker/dockerfile:1.2.bad").is_none());
    }

    #[test]
    fn frontend_version_rule_gates_official_frontends_and_symbolic_chmod() {
        assert!(is_official_dockerfile_frontend("docker/dockerfile:1.20"));
        assert!(is_official_dockerfile_frontend(
            "docker.io/docker/dockerfile:1.20@sha256:abcdef"
        ));
        assert!(!is_official_dockerfile_frontend(
            "registry.example.com/custom/dockerfile:1.20"
        ));

        assert!(!chmod_value_is_symbolic("0755"));
        assert!(chmod_value_is_symbolic("+x"));
        assert!(chmod_value_is_symbolic("u=rwX,go=rX"));
    }
}
