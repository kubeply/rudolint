//! Shell command parsing and analysis for `RUN` instructions.

/// Shell command text extracted from a Dockerfile `RUN` instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellProgram {
    /// Original shell source text.
    pub source: String,
}

/// Executable command detected at a shell command boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCommandInvocation {
    /// Command basename, with any leading path removed.
    pub command: String,
    /// Arguments following the command until the next shell command boundary.
    pub arguments: Vec<String>,
}

/// Package manager executable detected in shell command text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    /// Debian/Ubuntu `apt-get`.
    AptGet,
    /// Debian/Ubuntu `apt`.
    Apt,
    /// Alpine `apk`.
    Apk,
    /// Fedora/RHEL `dnf`.
    Dnf,
    /// RHEL/CentOS `yum`.
    Yum,
    /// Minimal RHEL/Fedora `microdnf`.
    Microdnf,
    /// Python `pip` or `pip3`.
    Pip,
    /// Node.js `npm`.
    Npm,
    /// Node.js `pnpm`.
    Pnpm,
    /// Node.js `yarn`.
    Yarn,
    /// Rust `cargo`.
    Cargo,
    /// Go toolchain package installer.
    Go,
}

/// Commands that rarely make sense inside Docker build `RUN` steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisallowedContainerCommand {
    /// OpenSSH client command.
    Ssh,
    /// Vim editor command.
    Vim,
    /// System shutdown command.
    Shutdown,
    /// Service manager command.
    Service,
    /// Process listing command.
    Ps,
    /// Memory usage command.
    Free,
    /// Interactive process monitor command.
    Top,
    /// Process signal command.
    Kill,
    /// Filesystem mount command.
    Mount,
    /// Legacy network interface command.
    Ifconfig,
}

impl DisallowedContainerCommand {
    /// Returns the command name as it appears in shell input.
    pub fn as_str(self) -> &'static str {
        match self {
            DisallowedContainerCommand::Ssh => "ssh",
            DisallowedContainerCommand::Vim => "vim",
            DisallowedContainerCommand::Shutdown => "shutdown",
            DisallowedContainerCommand::Service => "service",
            DisallowedContainerCommand::Ps => "ps",
            DisallowedContainerCommand::Free => "free",
            DisallowedContainerCommand::Top => "top",
            DisallowedContainerCommand::Kill => "kill",
            DisallowedContainerCommand::Mount => "mount",
            DisallowedContainerCommand::Ifconfig => "ifconfig",
        }
    }
}

impl PackageManager {
    /// Returns the canonical command name for this package manager.
    pub fn as_str(self) -> &'static str {
        match self {
            PackageManager::AptGet => "apt-get",
            PackageManager::Apt => "apt",
            PackageManager::Apk => "apk",
            PackageManager::Dnf => "dnf",
            PackageManager::Yum => "yum",
            PackageManager::Microdnf => "microdnf",
            PackageManager::Pip => "pip",
            PackageManager::Npm => "npm",
            PackageManager::Pnpm => "pnpm",
            PackageManager::Yarn => "yarn",
            PackageManager::Cargo => "cargo",
            PackageManager::Go => "go",
        }
    }
}

/// Detects package manager commands mentioned in shell command text.
///
/// The detector tokenizes on common shell separators and returns each detected
/// package manager once, preserving first-seen order.
pub fn detect_package_managers(shell: &str) -> Vec<PackageManager> {
    let mut managers = Vec::new();
    for token in shell.split(|character: char| {
        character.is_whitespace() || matches!(character, ';' | '&' | '|' | '(' | ')' | '{' | '}')
    }) {
        let manager = match token {
            "apt-get" => Some(PackageManager::AptGet),
            "apt" => Some(PackageManager::Apt),
            "apk" => Some(PackageManager::Apk),
            "dnf" => Some(PackageManager::Dnf),
            "yum" => Some(PackageManager::Yum),
            "microdnf" => Some(PackageManager::Microdnf),
            "pip" | "pip3" => Some(PackageManager::Pip),
            "npm" => Some(PackageManager::Npm),
            "pnpm" => Some(PackageManager::Pnpm),
            "yarn" => Some(PackageManager::Yarn),
            "cargo" => Some(PackageManager::Cargo),
            "go" => Some(PackageManager::Go),
            _ => None,
        };
        if let Some(manager) = manager
            && !managers.contains(&manager)
        {
            managers.push(manager);
        }
    }
    managers
}

/// Detects commands that rarely make sense inside Docker build `RUN` steps.
///
/// The detector treats shell command separators as command boundaries and
/// returns each detected command once, preserving first-seen order.
pub fn detect_disallowed_container_commands(shell: &str) -> Vec<DisallowedContainerCommand> {
    let mut commands = Vec::new();

    for invocation in detect_command_invocations(shell) {
        if let Some(command) = disallowed_container_command(&invocation.command)
            && !commands.contains(&command)
        {
            commands.push(command);
        }
    }

    commands
}

/// Detects executable commands at shell command boundaries.
pub fn detect_command_invocations(shell: &str) -> Vec<ShellCommandInvocation> {
    let mut commands = Vec::new();
    let mut current_command: Option<ShellCommandInvocation> = None;
    let mut expect_command = true;

    for raw_token in shell_tokens(shell) {
        if raw_token.is_separator {
            if let Some(command) = current_command.take() {
                commands.push(command);
            }
            expect_command = true;
            continue;
        }

        let token = raw_token
            .text
            .trim_matches(|character| matches!(character, '\'' | '"' | '(' | ')'))
            .trim_matches(|character| matches!(character, ';' | '&' | '|'));

        if token.is_empty() {
            if let Some(command) = current_command.take() {
                commands.push(command);
            }
            expect_command = true;
            continue;
        }

        if expect_command {
            if let Some(command) = current_command.take() {
                commands.push(command);
            }

            if is_env_assignment(token) {
                continue;
            }

            let command = token.rsplit('/').next().unwrap_or(token);
            current_command = Some(ShellCommandInvocation {
                command: command.to_string(),
                arguments: Vec::new(),
            });
            expect_command = false;
        } else if let Some(command) = &mut current_command {
            command.arguments.push(token.to_string());
        }
    }

    if let Some(command) = current_command {
        commands.push(command);
    }

    commands
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ShellToken {
    text: String,
    is_separator: bool,
}

fn shell_tokens(shell: &str) -> Vec<ShellToken> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote = None;

    for character in shell.chars() {
        if matches!(character, '\'' | '"') {
            if quote == Some(character) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(character);
            }
            current.push(character);
            continue;
        }

        if quote.is_none() && character.is_whitespace() {
            push_shell_token(&mut tokens, &mut current, false);
            continue;
        }

        if quote.is_none() && matches!(character, ';' | '&' | '|' | '(' | ')') {
            push_shell_token(&mut tokens, &mut current, false);
            push_shell_token(&mut tokens, &mut character.to_string(), true);
            continue;
        }

        current.push(character);
    }

    push_shell_token(&mut tokens, &mut current, false);
    tokens
}

fn push_shell_token(tokens: &mut Vec<ShellToken>, token: &mut String, is_separator: bool) {
    if token.is_empty() {
        return;
    }

    tokens.push(ShellToken {
        text: std::mem::take(token),
        is_separator,
    });
}

fn is_env_assignment(token: &str) -> bool {
    let Some((name, _)) = token.split_once('=') else {
        return false;
    };

    !name.is_empty()
        && name
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn disallowed_container_command(command: &str) -> Option<DisallowedContainerCommand> {
    match command {
        "ssh" => Some(DisallowedContainerCommand::Ssh),
        "vim" => Some(DisallowedContainerCommand::Vim),
        "shutdown" => Some(DisallowedContainerCommand::Shutdown),
        "service" => Some(DisallowedContainerCommand::Service),
        "ps" => Some(DisallowedContainerCommand::Ps),
        "free" => Some(DisallowedContainerCommand::Free),
        "top" => Some(DisallowedContainerCommand::Top),
        "kill" => Some(DisallowedContainerCommand::Kill),
        "mount" => Some(DisallowedContainerCommand::Mount),
        "ifconfig" => Some(DisallowedContainerCommand::Ifconfig),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn snapshots_package_manager_detection() {
        let cases = [
            "apt-get update && apt-get install -y curl",
            "apt update && apt install -y curl",
            "apk add --no-cache git",
            "dnf install -y gcc && pip install maturin",
            "pip3 install -r requirements.txt",
            "npm ci && pnpm install && yarn install",
            "cargo install cargo-deny && go install example.com/tool@latest",
            "microdnf install shadow-utils || yum install shadow-utils",
            "(apt-get update)",
        ];
        let values = cases
            .iter()
            .map(|case| {
                json!({
                    "shell": case,
                    "package_managers": detect_package_managers(case)
                        .into_iter()
                        .map(PackageManager::as_str)
                        .collect::<Vec<_>>(),
                })
            })
            .collect::<Vec<_>>();

        insta::assert_json_snapshot!(values);
    }

    #[test]
    fn snapshots_disallowed_container_command_detection() {
        let cases = [
            "ssh localhost",
            "apk add vim",
            "cd /tmp && vim file",
            "FOO=bar /usr/bin/service nginx start",
            "ps aux | grep nginx",
            "printf '%s' kill",
            "mount -t proc proc /proc; ifconfig",
            "mount -t proc proc /proc;ifconfig",
            "vim file && /usr/bin/vim other",
        ];
        let values = cases
            .iter()
            .map(|case| {
                json!({
                    "shell": case,
                    "commands": detect_disallowed_container_commands(case)
                        .into_iter()
                        .map(DisallowedContainerCommand::as_str)
                        .collect::<Vec<_>>(),
                })
            })
            .collect::<Vec<_>>();

        insta::assert_json_snapshot!(values);
    }

    #[test]
    fn snapshots_command_invocation_detection() {
        let cases = [
            "cd /tmp && make",
            "apk add vim",
            "FOO=bar /usr/bin/service nginx start",
            "printf '%s' cd",
            "mount -t proc proc /proc; ifconfig",
            "mount -t proc proc /proc;ifconfig",
        ];
        let values = cases
            .iter()
            .map(|case| {
                json!({
                    "shell": case,
                    "commands": detect_command_invocations(case)
                        .into_iter()
                        .map(|invocation| {
                            json!({
                                "command": invocation.command,
                                "arguments": invocation.arguments,
                            })
                        })
                        .collect::<Vec<_>>(),
                })
            })
            .collect::<Vec<_>>();

        insta::assert_json_snapshot!(values);
    }
}
