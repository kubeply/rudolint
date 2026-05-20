//! Shell command parsing and analysis for `RUN` instructions.

use std::ops::Range;

/// Shell command text extracted from a Dockerfile `RUN` instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellProgram {
    /// Original shell source text.
    pub source: String,
    /// Lexical tokens extracted from the shell source.
    pub tokens: Vec<ShellToken>,
    /// Command invocations recognized at command boundaries.
    pub commands: Vec<ShellCommandInvocation>,
    /// Package-manager invocations recognized from command facts.
    pub package_managers: Vec<PackageManagerInvocation>,
}

/// A byte span in shell source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellSpan {
    /// Start byte offset, inclusive.
    pub start: usize,
    /// End byte offset, exclusive.
    pub end: usize,
}

impl ShellSpan {
    /// Returns the span as a standard byte range.
    pub fn range(&self) -> Range<usize> {
        self.start..self.end
    }

    /// Returns this span shifted into an enclosing Dockerfile source.
    pub fn with_base_offset(&self, base_offset: usize) -> Self {
        Self {
            start: self.start + base_offset,
            end: self.end + base_offset,
        }
    }
}

/// Shell token category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellTokenKind {
    /// A shell word, including quoted words and assignments.
    Word,
    /// A command separator such as `;`, `&`, `&&`, `|`, `||`, `(`, or `)`.
    Separator,
    /// A redirection operator such as `>`, `2>`, or `<<`.
    Redirection,
}

/// Quoting state observed while tokenizing a word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteKind {
    /// No shell quotes were present in the token.
    None,
    /// The token contains single-quoted text.
    Single,
    /// The token contains double-quoted text.
    Double,
    /// The token contains both single-quoted and double-quoted text.
    Mixed,
}

impl QuoteKind {
    fn with_quote(self, quote: char) -> Self {
        match (self, quote) {
            (QuoteKind::None, '\'') => QuoteKind::Single,
            (QuoteKind::None, '"') => QuoteKind::Double,
            (QuoteKind::Single, '\'') | (QuoteKind::Double, '"') => self,
            (QuoteKind::Single | QuoteKind::Double | QuoteKind::Mixed, _) => QuoteKind::Mixed,
            _ => self,
        }
    }
}

/// A token extracted from shell source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellToken {
    /// Token text after quote and escape syntax has been removed where safe.
    pub text: String,
    /// Original token text exactly as it appeared in source.
    pub raw: String,
    /// Token category.
    pub kind: ShellTokenKind,
    /// Token byte span relative to shell source text.
    pub span: ShellSpan,
    /// Quoting state observed for this token.
    pub quote: QuoteKind,
    /// Variable expansions found inside this token.
    pub expansions: Vec<ShellExpansion>,
}

impl ShellToken {
    /// Returns true when this token is a command boundary.
    pub fn is_command_separator(&self) -> bool {
        self.kind == ShellTokenKind::Separator
    }

    /// Returns true when this token is a command-chain operator.
    pub fn is_command_chain(&self) -> bool {
        matches!(self.raw.as_str(), "&&" | "||")
    }

    /// Returns true when this token is a pipeline operator.
    pub fn is_pipeline(&self) -> bool {
        self.raw == "|"
    }
}

/// A shell variable or command expansion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellExpansion {
    /// Expansion kind.
    pub kind: ShellExpansionKind,
    /// Expansion text without the leading shell sigil when available.
    pub text: String,
    /// Expansion span relative to shell source text.
    pub span: ShellSpan,
}

/// Shell expansion kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellExpansionKind {
    /// `$NAME` or `${NAME}`.
    Variable,
    /// `$(command)`.
    Command,
}

/// A parsed environment assignment preceding a simple command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvAssignment {
    /// Assignment variable name.
    pub name: String,
    /// Assignment value, if one was provided.
    pub value: Option<String>,
    /// Assignment span relative to shell source text.
    pub span: ShellSpan,
}

/// An argument word belonging to a command invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellArgument {
    /// Argument text after simple quote and escape syntax has been removed.
    pub text: String,
    /// Original argument text.
    pub raw: String,
    /// Argument span relative to shell source text.
    pub span: ShellSpan,
    /// Quoting state observed for this argument.
    pub quote: QuoteKind,
}

/// Executable command detected at a shell command boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCommandInvocation {
    /// Command basename, with any leading path removed.
    pub command: String,
    /// Command word as it appeared after quote and escape processing.
    pub command_word: String,
    /// Command source span relative to shell source text.
    pub command_span: ShellSpan,
    /// Arguments following the command until the next shell command boundary.
    pub arguments: Vec<String>,
    /// Argument facts following the command until the next shell command boundary.
    pub argument_facts: Vec<ShellArgument>,
    /// Environment assignments immediately preceding the command.
    pub env: Vec<EnvAssignment>,
}

impl ShellCommandInvocation {
    /// Returns `true` when this invocation's command basename equals `command`.
    pub fn command_is(&self, command: &str) -> bool {
        self.command == command
    }

    /// Returns `true` when this invocation's command basename matches any candidate.
    pub fn command_is_any(&self, commands: &[&str]) -> bool {
        commands.iter().any(|command| self.command_is(command))
    }

    /// Returns `true` when the invocation contains `expected` as a contiguous argument sequence.
    pub fn has_arg_sequence(&self, expected: &[&str]) -> bool {
        expected.is_empty()
            || self.arguments.windows(expected.len()).any(|window| {
                window
                    .iter()
                    .map(String::as_str)
                    .eq(expected.iter().copied())
            })
    }

    /// Returns `true` when both the command and argument sequence match.
    pub fn command_has_args(&self, command: &str, expected: &[&str]) -> bool {
        self.command_is(command) && self.has_arg_sequence(expected)
    }
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

/// Package-manager command fact detected in shell command text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageManagerInvocation {
    /// Canonical package manager.
    pub manager: PackageManager,
    /// Command invocation that used the package manager.
    pub command: ShellCommandInvocation,
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

/// Parses shell source into token, command, and package-manager facts.
pub fn analyze(shell: &str) -> ShellProgram {
    let tokens = tokenize(shell);
    let commands = command_invocations_from_tokens(&tokens);
    let package_managers = package_manager_invocations_from_commands(&commands);

    ShellProgram {
        source: shell.to_string(),
        tokens,
        commands,
        package_managers,
    }
}

/// Tokenizes shell source and preserves byte spans relative to that source.
pub fn tokenize(shell: &str) -> Vec<ShellToken> {
    let bytes = shell.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index].is_ascii_whitespace() {
            index += 1;
            continue;
        }

        if let Some((operator, end)) = read_operator(shell, index) {
            tokens.push(ShellToken {
                text: operator.to_string(),
                raw: operator.to_string(),
                kind: operator_kind(operator),
                span: ShellSpan { start: index, end },
                quote: QuoteKind::None,
                expansions: Vec::new(),
            });
            index = end;
            continue;
        }

        let start = index;
        let mut text = String::new();
        let mut quote = QuoteKind::None;
        let mut active_quote = None;
        let mut expansions = Vec::new();

        while index < bytes.len() {
            let Some(character) = char_at(shell, index) else {
                break;
            };

            if active_quote.is_none() && character.is_whitespace() {
                break;
            }

            if active_quote.is_none() && read_operator(shell, index).is_some() {
                break;
            }

            if character == '\'' && active_quote != Some('"') {
                active_quote = if active_quote == Some('\'') {
                    None
                } else {
                    Some('\'')
                };
                quote = quote.with_quote(character);
                index += character.len_utf8();
                continue;
            }

            if character == '"' && active_quote != Some('\'') {
                active_quote = if active_quote == Some('"') {
                    None
                } else {
                    Some('"')
                };
                quote = quote.with_quote(character);
                index += character.len_utf8();
                continue;
            }

            if character == '\\' && active_quote != Some('\'') {
                index += character.len_utf8();
                if index >= bytes.len() {
                    text.push('\\');
                    break;
                }
                let Some(escaped) = char_at(shell, index) else {
                    text.push('\\');
                    break;
                };
                text.push(escaped);
                index += escaped.len_utf8();
                if escaped == '\n' {
                    text.pop();
                }
                continue;
            }

            if character == '$'
                && active_quote != Some('\'')
                && let Some((expansion, end)) = read_expansion(shell, index)
            {
                expansions.push(expansion);
                text.push_str(&shell[index..end]);
                index = end;
                continue;
            }

            text.push(character);
            index += character.len_utf8();
        }

        if !(text.is_empty() && quote == QuoteKind::None) {
            tokens.push(ShellToken {
                text,
                raw: shell[start..index].to_string(),
                kind: ShellTokenKind::Word,
                span: ShellSpan { start, end: index },
                quote,
                expansions,
            });
        }
    }

    tokens
}

/// Detects package manager commands mentioned in shell command text.
///
/// The detector returns each detected package manager once, preserving
/// first-seen order.
pub fn detect_package_managers(shell: &str) -> Vec<PackageManager> {
    let mut managers = Vec::new();
    for invocation in analyze(shell).package_managers {
        if !managers.contains(&invocation.manager) {
            managers.push(invocation.manager);
        }
    }
    managers
}

/// Returns package-manager command facts in first-seen order.
pub fn detect_package_manager_invocations(shell: &str) -> Vec<PackageManagerInvocation> {
    analyze(shell).package_managers
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
    analyze(shell).commands
}

fn command_invocations_from_tokens(tokens: &[ShellToken]) -> Vec<ShellCommandInvocation> {
    let mut commands = Vec::new();
    let mut current_command: Option<ShellCommandInvocation> = None;
    let mut pending_env = Vec::new();
    let mut expect_command = true;
    let mut skip_redirection_operand = false;

    for token in tokens {
        match token.kind {
            ShellTokenKind::Separator => {
                finish_command(&mut commands, &mut current_command);
                pending_env.clear();
                expect_command = true;
                skip_redirection_operand = false;
            }
            ShellTokenKind::Redirection => {
                skip_redirection_operand = true;
            }
            ShellTokenKind::Word if skip_redirection_operand => {
                skip_redirection_operand = false;
            }
            ShellTokenKind::Word if expect_command => {
                if let Some(assignment) = env_assignment(token) {
                    pending_env.push(assignment);
                    continue;
                }

                finish_command(&mut commands, &mut current_command);
                let command_word = token.text.clone();
                let command = command_word
                    .rsplit('/')
                    .next()
                    .unwrap_or(command_word.as_str())
                    .to_string();
                current_command = Some(ShellCommandInvocation {
                    command,
                    command_word,
                    command_span: token.span.clone(),
                    arguments: Vec::new(),
                    argument_facts: Vec::new(),
                    env: std::mem::take(&mut pending_env),
                });
                expect_command = false;
            }
            ShellTokenKind::Word => {
                if let Some(command) = &mut current_command {
                    command.arguments.push(token.text.clone());
                    command.argument_facts.push(ShellArgument {
                        text: token.text.clone(),
                        raw: token.raw.clone(),
                        span: token.span.clone(),
                        quote: token.quote,
                    });
                }
            }
        }
    }

    finish_command(&mut commands, &mut current_command);
    commands
}

fn finish_command(
    commands: &mut Vec<ShellCommandInvocation>,
    current_command: &mut Option<ShellCommandInvocation>,
) {
    if let Some(command) = current_command.take()
        && !command.command.is_empty()
    {
        commands.push(command);
    }
}

fn package_manager_invocations_from_commands(
    commands: &[ShellCommandInvocation],
) -> Vec<PackageManagerInvocation> {
    commands
        .iter()
        .filter_map(|command| {
            Some(PackageManagerInvocation {
                manager: package_manager(&command.command)?,
                command: command.clone(),
            })
        })
        .collect()
}

fn read_operator(shell: &str, index: usize) -> Option<(&'static str, usize)> {
    let rest = &shell[index..];
    for operator in ["&&", "||", ">>", "<<", "2>", "2>>", "1>", "1>>"] {
        if rest.starts_with(operator) {
            return Some((operator, index + operator.len()));
        }
    }

    let character = rest.chars().next()?;
    match character {
        ';' | '&' | '|' | '(' | ')' | '{' | '}' => Some((
            character_to_static_str(character),
            index + character.len_utf8(),
        )),
        '<' | '>' => Some((
            character_to_static_str(character),
            index + character.len_utf8(),
        )),
        _ => None,
    }
}

fn character_to_static_str(character: char) -> &'static str {
    match character {
        ';' => ";",
        '&' => "&",
        '|' => "|",
        '(' => "(",
        ')' => ")",
        '{' => "{",
        '}' => "}",
        '<' => "<",
        '>' => ">",
        _ => "",
    }
}

fn operator_kind(operator: &str) -> ShellTokenKind {
    if operator.contains('<') || operator.contains('>') {
        ShellTokenKind::Redirection
    } else {
        ShellTokenKind::Separator
    }
}

fn read_expansion(shell: &str, start: usize) -> Option<(ShellExpansion, usize)> {
    let mut chars = shell[start..].char_indices();
    let (_, dollar) = chars.next()?;
    if dollar != '$' {
        return None;
    }

    let (relative_index, next) = chars.next()?;
    let next_index = start + relative_index;
    match next {
        '{' => read_braced_variable(shell, start, next_index),
        '(' => read_command_substitution(shell, start, next_index),
        '_' | 'A'..='Z' | 'a'..='z' => read_simple_variable(shell, start, next_index),
        _ => None,
    }
}

fn char_at(shell: &str, index: usize) -> Option<char> {
    shell.get(index..)?.chars().next()
}

fn read_braced_variable(
    shell: &str,
    start: usize,
    open_brace_index: usize,
) -> Option<(ShellExpansion, usize)> {
    let name_start = open_brace_index + 1;
    let close_relative = shell[name_start..].find('}')?;
    let end = name_start + close_relative + 1;
    Some((
        ShellExpansion {
            kind: ShellExpansionKind::Variable,
            text: shell[name_start..name_start + close_relative].to_string(),
            span: ShellSpan { start, end },
        },
        end,
    ))
}

fn read_simple_variable(
    shell: &str,
    start: usize,
    name_start: usize,
) -> Option<(ShellExpansion, usize)> {
    let mut end = name_start;
    for character in shell[name_start..].chars() {
        if character == '_' || character.is_ascii_alphanumeric() {
            end += character.len_utf8();
        } else {
            break;
        }
    }

    if end == name_start {
        return None;
    }

    Some((
        ShellExpansion {
            kind: ShellExpansionKind::Variable,
            text: shell[name_start..end].to_string(),
            span: ShellSpan { start, end },
        },
        end,
    ))
}

fn read_command_substitution(
    shell: &str,
    start: usize,
    open_paren_index: usize,
) -> Option<(ShellExpansion, usize)> {
    let mut depth = 1usize;
    let mut index = open_paren_index + 1;
    let mut quote = None;

    while index < shell.len() {
        let Some(character) = char_at(shell, index) else {
            break;
        };
        if character == '\\' {
            index += character.len_utf8();
            if index < shell.len() {
                let Some(escaped) = char_at(shell, index) else {
                    break;
                };
                index += escaped.len_utf8();
            }
            continue;
        }
        if matches!(character, '\'' | '"') {
            if quote == Some(character) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(character);
            }
            index += character.len_utf8();
            continue;
        }
        if quote.is_none() && character == '(' {
            depth += 1;
        } else if quote.is_none() && character == ')' {
            depth -= 1;
            if depth == 0 {
                let end = index + character.len_utf8();
                return Some((
                    ShellExpansion {
                        kind: ShellExpansionKind::Command,
                        text: shell[open_paren_index + 1..index].to_string(),
                        span: ShellSpan { start, end },
                    },
                    end,
                ));
            }
        }
        index += character.len_utf8();
    }

    None
}

fn env_assignment(token: &ShellToken) -> Option<EnvAssignment> {
    let (name, value) = token.text.split_once('=')?;
    if !is_name(name) {
        return None;
    }

    Some(EnvAssignment {
        name: name.to_string(),
        value: Some(value.to_string()),
        span: token.span.clone(),
    })
}

fn is_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn package_manager(command: &str) -> Option<PackageManager> {
    match command {
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
    }
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
    fn snapshots_tokenization() {
        let cases = [
            "apt-get update && apt-get install -y curl",
            "printf '%s' \"$TARGETARCH\"",
            "FOO=bar PATH=/usr/bin:$PATH make build",
            "echo $(uname -m) > /tmp/arch",
            "cat <<EOF\nhello\nEOF",
            "printf escaped\\ value",
            "printf \\$PATH \"$PATH\"",
            "cmd 2>/tmp/error || true",
        ];
        let values = cases
            .iter()
            .map(|case| {
                json!({
                    "shell": case,
                    "tokens": tokenize(case)
                        .into_iter()
                        .map(|token| {
                            json!({
                                "text": token.text,
                                "raw": token.raw,
                                "kind": format!("{:?}", token.kind),
                                "span": token.span.range(),
                                "quote": format!("{:?}", token.quote),
                                "expansions": token.expansions.into_iter().map(|expansion| {
                                    json!({
                                        "kind": format!("{:?}", expansion.kind),
                                        "text": expansion.text,
                                        "span": expansion.span.range(),
                                    })
                                }).collect::<Vec<_>>(),
                            })
                        })
                        .collect::<Vec<_>>(),
                })
            })
            .collect::<Vec<_>>();

        insta::assert_json_snapshot!(values);
    }

    #[test]
    fn snapshots_command_facts() {
        let cases = [
            "cd /tmp && make",
            "apk add vim",
            "FOO=bar /usr/bin/service nginx start",
            "printf '%s' cd",
            "mount -t proc proc /proc; ifconfig",
            "mount -t proc proc /proc;ifconfig",
            "echo $(uname -m) | tee /tmp/arch",
            "cat < input.txt > output.txt && rm input.txt",
        ];
        let values = cases
            .iter()
            .map(|case| {
                json!({
                    "shell": case,
                    "commands": detect_command_invocations(case)
                        .into_iter()
                        .map(command_json)
                        .collect::<Vec<_>>(),
                })
            })
            .collect::<Vec<_>>();

        insta::assert_json_snapshot!(values);
    }

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
            "printf '%s' apt-get",
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
                    "invocations": detect_package_manager_invocations(case)
                        .into_iter()
                        .map(|invocation| {
                            json!({
                                "manager": invocation.manager.as_str(),
                                "command": command_json(invocation.command),
                            })
                        })
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
    fn command_invocation_matching_handles_argument_sequences() {
        let invocation = ShellCommandInvocation {
            command: "yarn".to_string(),
            command_word: "yarn".to_string(),
            command_span: ShellSpan { start: 0, end: 4 },
            arguments: vec!["cache".to_string(), "clean".to_string()],
            argument_facts: vec![
                ShellArgument {
                    text: "cache".to_string(),
                    raw: "cache".to_string(),
                    span: ShellSpan { start: 5, end: 10 },
                    quote: QuoteKind::None,
                },
                ShellArgument {
                    text: "clean".to_string(),
                    raw: "clean".to_string(),
                    span: ShellSpan { start: 11, end: 16 },
                    quote: QuoteKind::None,
                },
            ],
            env: Vec::new(),
        };

        assert!(invocation.command_is("yarn"));
        assert!(invocation.command_is_any(&["npm", "yarn"]));
        assert!(invocation.has_arg_sequence(&["cache", "clean"]));
        assert!(invocation.command_has_args("yarn", &["cache", "clean"]));
        assert!(!invocation.command_has_args("npm", &["cache", "clean"]));
        assert!(!invocation.has_arg_sequence(&["clean", "cache"]));
    }

    #[test]
    fn shell_span_can_shift_to_dockerfile_offset() {
        assert_eq!(
            ShellSpan { start: 3, end: 7 }.with_base_offset(20),
            ShellSpan { start: 23, end: 27 }
        );
    }

    fn command_json(invocation: ShellCommandInvocation) -> serde_json::Value {
        json!({
            "command": invocation.command,
            "command_word": invocation.command_word,
            "command_span": invocation.command_span.range(),
            "arguments": invocation.arguments,
            "argument_facts": invocation.argument_facts.into_iter().map(|argument| {
                json!({
                    "text": argument.text,
                    "raw": argument.raw,
                    "span": argument.span.range(),
                    "quote": format!("{:?}", argument.quote),
                })
            }).collect::<Vec<_>>(),
            "env": invocation.env.into_iter().map(|assignment| {
                json!({
                    "name": assignment.name,
                    "value": assignment.value,
                    "span": assignment.span.range(),
                })
            }).collect::<Vec<_>>(),
        })
    }
}
