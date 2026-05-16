//! Shell command parsing and analysis for `RUN` instructions.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellProgram {
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCommandInvocation {
    pub command: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    AptGet,
    Apt,
    Apk,
    Dnf,
    Yum,
    Microdnf,
    Pip,
    Npm,
    Pnpm,
    Yarn,
    Cargo,
    Go,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisallowedContainerCommand {
    Ssh,
    Vim,
    Shutdown,
    Service,
    Ps,
    Free,
    Top,
    Kill,
    Mount,
    Ifconfig,
}

impl DisallowedContainerCommand {
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

pub fn detect_package_managers(shell: &str) -> Vec<PackageManager> {
    let mut managers = Vec::new();
    for token in shell.split(|character: char| {
        character.is_whitespace() || character == ';' || character == '&' || character == '|'
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

pub fn detect_disallowed_container_commands(shell: &str) -> Vec<DisallowedContainerCommand> {
    detect_command_invocations(shell)
        .into_iter()
        .filter_map(|invocation| disallowed_container_command(&invocation.command))
        .collect()
}

pub fn detect_command_invocations(shell: &str) -> Vec<ShellCommandInvocation> {
    let mut commands = Vec::new();
    let mut expect_command = true;

    for raw_token in shell.split_whitespace() {
        if is_command_separator(raw_token) {
            expect_command = true;
            continue;
        }

        let ends_with_separator =
            raw_token.ends_with(';') || raw_token.ends_with('&') || raw_token.ends_with('|');
        let token = raw_token
            .trim_matches(|character| matches!(character, '\'' | '"' | '(' | ')'))
            .trim_matches(|character| matches!(character, ';' | '&' | '|'));

        if token.is_empty() {
            expect_command = true;
            continue;
        }

        if expect_command {
            if is_env_assignment(token) {
                continue;
            }

            let command = token.rsplit('/').next().unwrap_or(token);
            commands.push(ShellCommandInvocation {
                command: command.to_string(),
            });
            expect_command = false;
        }

        if ends_with_separator {
            expect_command = true;
        }
    }

    commands
}

fn is_command_separator(token: &str) -> bool {
    matches!(token, ";" | "&&" | "||" | "|")
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
            "apk add --no-cache git",
            "dnf install -y gcc && pip install maturin",
            "npm ci && pnpm install && yarn install",
            "cargo install cargo-deny && go install example.com/tool@latest",
            "microdnf install shadow-utils || yum install shadow-utils",
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
        ];
        let values = cases
            .iter()
            .map(|case| {
                json!({
                    "shell": case,
                    "commands": detect_command_invocations(case)
                        .into_iter()
                        .map(|invocation| invocation.command)
                        .collect::<Vec<_>>(),
                })
            })
            .collect::<Vec<_>>();

        insta::assert_json_snapshot!(values);
    }
}
