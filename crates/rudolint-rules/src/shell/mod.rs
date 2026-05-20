use std::path::Path;

use crate::{Rule, RuleInfo, metadata::diagnostic, metadata::rule_metadata};
use rudolint_config::Config;
use rudolint_diagnostics::{Finding, Severity};
use rudolint_dockerfile::{Dockerfile, Instruction};
use rudolint_shell::{
    QuoteKind, ShellCommandInvocation, ShellExpansionKind, ShellProgram, ShellTokenKind, analyze,
};

pub(crate) fn implemented_catalog() -> Vec<RuleInfo> {
    vec![
        CommandChainAsCondition.metadata_info(),
        UnquotedCommandSubstitution.metadata_info(),
        UnquotedVariableExpansion.metadata_info(),
        DeclareAndAssignSeparately.metadata_info(),
        CheckCdExitStatus.metadata_info(),
    ]
}

pub(crate) fn planned_catalog() -> Vec<&'static str> {
    vec![
        "RSC1000", "RSC1001", "RSC1007", "RSC1010", "RSC1018", "RSC1035", "RSC1045", "RSC1065",
        "RSC1066", "RSC1077", "RSC1078", "RSC1079", "RSC1081", "RSC1083", "RSC1086", "RSC1095",
        "RSC2002", "RSC2026", "RSC2035", "RSC2140", "RSC2154", "RSC2181", "RSC2196",
    ]
}

rule_metadata!(
    CommandChainAsCondition,
    "RSC2015",
    "command-chain-as-condition",
    Severity::Warning,
    "avoid using A && B || C as a conditional expression"
);

impl Rule for CommandChainAsCondition {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        shell_findings(doc, |instruction, program| {
            has_and_or_chain(program).then(|| {
                diagnostic(
                    "RSC2015",
                    Severity::Warning,
                    "A && B || C is not a safe conditional expression",
                    instruction,
                )
            })
        })
    }
}

rule_metadata!(
    UnquotedCommandSubstitution,
    "RSC2046",
    "quote-command-substitutions",
    Severity::Warning,
    "quote command substitutions to prevent word splitting"
);

impl Rule for UnquotedCommandSubstitution {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        shell_findings(doc, |instruction, program| {
            program
                .tokens
                .iter()
                .any(|token| {
                    token.quote == QuoteKind::None
                        && token
                            .expansions
                            .iter()
                            .any(|expansion| expansion.kind == ShellExpansionKind::Command)
                })
                .then(|| {
                    diagnostic(
                        "RSC2046",
                        Severity::Warning,
                        "quote command substitutions to prevent word splitting",
                        instruction,
                    )
                })
        })
    }
}

rule_metadata!(
    UnquotedVariableExpansion,
    "RSC2086",
    "quote-variable-expansions",
    Severity::Warning,
    "quote variable expansions to prevent word splitting and globbing"
);

impl Rule for UnquotedVariableExpansion {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        shell_findings(doc, |instruction, program| {
            program
                .tokens
                .iter()
                .any(|token| {
                    token.kind == ShellTokenKind::Word
                        && token.quote == QuoteKind::None
                        && !is_assignment_word(&token.text)
                        && token
                            .expansions
                            .iter()
                            .any(|expansion| expansion.kind == ShellExpansionKind::Variable)
                })
                .then(|| {
                    diagnostic(
                        "RSC2086",
                        Severity::Warning,
                        "quote variable expansions to prevent word splitting and globbing",
                        instruction,
                    )
                })
        })
    }
}

rule_metadata!(
    DeclareAndAssignSeparately,
    "RSC2155",
    "declare-and-assign-separately",
    Severity::Warning,
    "declare and assign separately to avoid masking command failures"
);

impl Rule for DeclareAndAssignSeparately {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        shell_findings(doc, |instruction, program| {
            program
                .commands
                .iter()
                .any(invocation_declares_assignment_with_command_substitution)
                .then(|| {
                    diagnostic(
                        "RSC2155",
                        Severity::Warning,
                        "declare and assign separately to avoid masking command failures",
                        instruction,
                    )
                })
        })
    }
}

rule_metadata!(
    CheckCdExitStatus,
    "RSC2164",
    "check-cd-exit-status",
    Severity::Warning,
    "check cd exit status before running more commands"
);

impl Rule for CheckCdExitStatus {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        shell_findings(doc, |instruction, program| {
            program
                .commands
                .iter()
                .any(|invocation| cd_without_checked_continuation(program, invocation))
                .then(|| {
                    diagnostic(
                        "RSC2164",
                        Severity::Warning,
                        "check cd exit status before running more commands",
                        instruction,
                    )
                })
        })
    }
}

fn shell_findings(
    doc: &Dockerfile,
    check: impl Fn(&Instruction, &ShellProgram) -> Option<Finding>,
) -> Vec<Finding> {
    doc.instructions
        .iter()
        .filter(|instruction| instruction.keyword_is("RUN"))
        .filter_map(|instruction| {
            let shell = instruction.run.as_ref()?.shell.as_ref()?;
            let program = analyze(&shell.text);
            check(instruction, &program)
        })
        .collect()
}

pub(crate) fn lint(doc: &Dockerfile, config: &Config, path: Option<&Path>) -> Vec<Finding> {
    let mut findings = Vec::new();
    for instruction in doc
        .instructions
        .iter()
        .filter(|instruction| instruction.keyword_is("RUN"))
    {
        let Some(shell) = instruction.run.as_ref().and_then(|run| run.shell.as_ref()) else {
            continue;
        };
        let program = analyze(&shell.text);
        lint_program(instruction, &program, config, path, &mut findings);
    }
    findings
}

fn lint_program(
    instruction: &Instruction,
    program: &ShellProgram,
    config: &Config,
    path: Option<&Path>,
    findings: &mut Vec<Finding>,
) {
    push_if_enabled(
        "RSC2015",
        config,
        path,
        findings,
        has_and_or_chain(program).then(|| {
            diagnostic(
                "RSC2015",
                Severity::Warning,
                "A && B || C is not a safe conditional expression",
                instruction,
            )
        }),
    );
    push_if_enabled(
        "RSC2046",
        config,
        path,
        findings,
        has_unquoted_command_substitution(program).then(|| {
            diagnostic(
                "RSC2046",
                Severity::Warning,
                "quote command substitutions to prevent word splitting",
                instruction,
            )
        }),
    );
    push_if_enabled(
        "RSC2086",
        config,
        path,
        findings,
        has_unquoted_variable_expansion(program).then(|| {
            diagnostic(
                "RSC2086",
                Severity::Warning,
                "quote variable expansions to prevent word splitting and globbing",
                instruction,
            )
        }),
    );
    push_if_enabled(
        "RSC2155",
        config,
        path,
        findings,
        program
            .commands
            .iter()
            .any(invocation_declares_assignment_with_command_substitution)
            .then(|| {
                diagnostic(
                    "RSC2155",
                    Severity::Warning,
                    "declare and assign separately to avoid masking command failures",
                    instruction,
                )
            }),
    );
    push_if_enabled(
        "RSC2164",
        config,
        path,
        findings,
        program
            .commands
            .iter()
            .any(|invocation| cd_without_checked_continuation(program, invocation))
            .then(|| {
                diagnostic(
                    "RSC2164",
                    Severity::Warning,
                    "check cd exit status before running more commands",
                    instruction,
                )
            }),
    );
}

fn push_if_enabled(
    code: &'static str,
    config: &Config,
    path: Option<&Path>,
    findings: &mut Vec<Finding>,
    finding: Option<Finding>,
) {
    if config.selects(code)
        && !config.ignores(code)
        && !path.is_some_and(|path| config.ignores_for_path(code, path))
        && let Some(finding) = finding
    {
        findings.push(finding);
    }
}

fn has_unquoted_command_substitution(program: &ShellProgram) -> bool {
    program.tokens.iter().any(|token| {
        token.quote == QuoteKind::None
            && token
                .expansions
                .iter()
                .any(|expansion| expansion.kind == ShellExpansionKind::Command)
    })
}

fn has_unquoted_variable_expansion(program: &ShellProgram) -> bool {
    program.tokens.iter().any(|token| {
        token.kind == ShellTokenKind::Word
            && token.quote == QuoteKind::None
            && !is_assignment_word(&token.text)
            && token
                .expansions
                .iter()
                .any(|expansion| expansion.kind == ShellExpansionKind::Variable)
    })
}

fn has_and_or_chain(program: &ShellProgram) -> bool {
    program
        .tokens
        .windows(3)
        .any(|window| window[1].raw == "&&" && window.iter().any(|token| token.raw == "||"))
        || program
            .tokens
            .iter()
            .enumerate()
            .any(|(index, token)| token.raw == "&&" && following_separator_is(program, index, "||"))
}

fn following_separator_is(program: &ShellProgram, start: usize, expected: &str) -> bool {
    for token in program.tokens.iter().skip(start + 1) {
        if token.kind == ShellTokenKind::Separator {
            return token.raw == expected;
        }
    }

    false
}

fn invocation_declares_assignment_with_command_substitution(
    invocation: &ShellCommandInvocation,
) -> bool {
    matches!(
        invocation.command.as_str(),
        "export" | "readonly" | "local" | "declare"
    ) && invocation.argument_facts.iter().any(|argument| {
        argument.text.contains('=')
            && analyze(&argument.raw).tokens.iter().any(|token| {
                token
                    .expansions
                    .iter()
                    .any(|expansion| expansion.kind == ShellExpansionKind::Command)
            })
    })
}

fn cd_without_checked_continuation(
    program: &ShellProgram,
    invocation: &ShellCommandInvocation,
) -> bool {
    if !invocation.command_is("cd") {
        return false;
    }

    let Some(separator) = program.tokens.iter().find(|token| {
        token.span.start >= invocation.command_span.end && token.is_command_separator()
    }) else {
        return true;
    };

    !matches!(separator.raw.as_str(), "&&" | "||")
}

fn is_assignment_word(text: &str) -> bool {
    let Some((name, _)) = text.split_once('=') else {
        return false;
    };

    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}
