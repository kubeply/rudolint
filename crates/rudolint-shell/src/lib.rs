//! Shell command parsing and analysis for `RUN` instructions.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellProgram {
    pub source: String,
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
}
