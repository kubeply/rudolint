use std::path::Path;

use crate::{Rule, RuleInfo, metadata::diagnostic, metadata::rule_metadata};
use rudolint_config::Config;
use rudolint_diagnostics::{Finding, Severity};
use rudolint_dockerfile::{Dockerfile, Instruction};
use rudolint_shell::{
    QuoteKind, ShellCommandInvocation, ShellExpansionKind, ShellProgram, ShellSpan, ShellToken,
    ShellTokenKind, analyze,
};
use rudolint_source::Span;

pub(crate) fn implemented_catalog() -> Vec<RuleInfo> {
    vec![
        UselessCat.metadata_info(),
        CommandChainAsCondition.metadata_info(),
        UnquotedCommandSubstitution.metadata_info(),
        UnquotedVariableExpansion.metadata_info(),
        DeclareAndAssignSeparately.metadata_info(),
        CheckCdExitStatus.metadata_info(),
        CheckExitCodeDirectly.metadata_info(),
    ]
}

pub(crate) fn planned_catalog() -> Vec<&'static str> {
    vec![
        "SC1000", "SC1001", "SC1007", "SC1010", "SC1018", "SC1035", "SC1045", "SC1065", "SC1066",
        "SC1077", "SC1078", "SC1079", "SC1081", "SC1083", "SC1086", "SC1095", "SC2026", "SC2035",
        "SC2140", "SC2154", "SC2196",
    ]
}

rule_metadata!(
    UselessCat,
    "SC2002",
    "avoid-useless-cat",
    Severity::Warning,
    "avoid piping cat output when the next command can read files directly"
);

impl Rule for UselessCat {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        shell_findings(doc, |instruction, program| {
            program
                .commands
                .iter()
                .any(|invocation| invocation_is_useless_cat(program, invocation))
                .then(|| {
                    diagnostic(
                        "SC2002",
                        Severity::Warning,
                        "avoid piping cat output when the next command can read files directly",
                        instruction,
                    )
                })
        })
    }
}

rule_metadata!(
    CommandChainAsCondition,
    "SC2015",
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
                    "SC2015",
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
    "SC2046",
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
            unquoted_command_substitution_token(program).map(|token| {
                shell_diagnostic(
                    "SC2046",
                    Severity::Warning,
                    "quote command substitutions to prevent word splitting",
                    instruction,
                    &token.span,
                )
            })
        })
    }
}

rule_metadata!(
    UnquotedVariableExpansion,
    "SC2086",
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
            unquoted_variable_expansion_token(program).map(|token| {
                shell_diagnostic(
                    "SC2086",
                    Severity::Warning,
                    "quote variable expansions to prevent word splitting and globbing",
                    instruction,
                    &token.span,
                )
            })
        })
    }
}

rule_metadata!(
    DeclareAndAssignSeparately,
    "SC2155",
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
                        "SC2155",
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
    "SC2164",
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
                        "SC2164",
                        Severity::Warning,
                        "check cd exit status before running more commands",
                        instruction,
                    )
                })
        })
    }
}

rule_metadata!(
    CheckExitCodeDirectly,
    "SC2181",
    "check-exit-code-directly",
    Severity::Warning,
    "check command exit status directly instead of testing $?"
);

impl Rule for CheckExitCodeDirectly {
    fn info(&self) -> RuleInfo {
        self.metadata_info()
    }

    fn check(&self, doc: &Dockerfile) -> Vec<Finding> {
        shell_findings(doc, |instruction, program| {
            program
                .commands
                .iter()
                .any(invocation_tests_previous_exit_status)
                .then(|| {
                    diagnostic(
                        "SC2181",
                        Severity::Warning,
                        "check command exit status directly instead of testing $?",
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

fn shell_diagnostic(
    code: &'static str,
    severity: Severity,
    message: &'static str,
    instruction: &Instruction,
    shell_span: &ShellSpan,
) -> Finding {
    let span =
        dockerfile_span_for_shell_span(instruction, shell_span).unwrap_or(instruction.raw_span);
    Finding::with_span(code, severity, message, span)
}

fn dockerfile_span_for_shell_span(
    instruction: &Instruction,
    shell_span: &ShellSpan,
) -> Option<Span> {
    let shell = instruction.run.as_ref()?.shell.as_ref()?;
    let start = shell.span.start_byte + shell_span.start;
    let end = shell.span.start_byte + shell_span.end;
    let start_position = shell_position(&shell.text, shell_span.start, &shell.span);
    let end_position = shell_position(&shell.text, shell_span.end, &shell.span);

    Some(Span {
        start_byte: start,
        end_byte: end,
        start_line: start_position.0,
        start_column: start_position.1,
        end_line: end_position.0,
        end_column: end_position.1,
    })
}

fn shell_position(shell: &str, byte_offset: usize, base_span: &Span) -> (usize, usize) {
    let mut line = base_span.start_line;
    let mut column = base_span.start_column;

    let Some(prefix) = shell.get(..byte_offset) else {
        return (base_span.start_line, base_span.start_column);
    };

    for character in prefix.chars() {
        if character == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    (line, column)
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
        "SC2002",
        config,
        path,
        findings,
        program
            .commands
            .iter()
            .any(|invocation| invocation_is_useless_cat(program, invocation))
            .then(|| {
                diagnostic(
                    "SC2002",
                    Severity::Warning,
                    "avoid piping cat output when the next command can read files directly",
                    instruction,
                )
            }),
    );
    push_if_enabled(
        "SC2015",
        config,
        path,
        findings,
        has_and_or_chain(program).then(|| {
            diagnostic(
                "SC2015",
                Severity::Warning,
                "A && B || C is not a safe conditional expression",
                instruction,
            )
        }),
    );
    push_if_enabled(
        "SC2046",
        config,
        path,
        findings,
        unquoted_command_substitution_token(program).map(|token| {
            shell_diagnostic(
                "SC2046",
                Severity::Warning,
                "quote command substitutions to prevent word splitting",
                instruction,
                &token.span,
            )
        }),
    );
    push_if_enabled(
        "SC2086",
        config,
        path,
        findings,
        unquoted_variable_expansion_token(program).map(|token| {
            shell_diagnostic(
                "SC2086",
                Severity::Warning,
                "quote variable expansions to prevent word splitting and globbing",
                instruction,
                &token.span,
            )
        }),
    );
    push_if_enabled(
        "SC2155",
        config,
        path,
        findings,
        program
            .commands
            .iter()
            .any(invocation_declares_assignment_with_command_substitution)
            .then(|| {
                diagnostic(
                    "SC2155",
                    Severity::Warning,
                    "declare and assign separately to avoid masking command failures",
                    instruction,
                )
            }),
    );
    push_if_enabled(
        "SC2164",
        config,
        path,
        findings,
        program
            .commands
            .iter()
            .any(|invocation| cd_without_checked_continuation(program, invocation))
            .then(|| {
                diagnostic(
                    "SC2164",
                    Severity::Warning,
                    "check cd exit status before running more commands",
                    instruction,
                )
            }),
    );
    push_if_enabled(
        "SC2181",
        config,
        path,
        findings,
        program
            .commands
            .iter()
            .any(invocation_tests_previous_exit_status)
            .then(|| {
                diagnostic(
                    "SC2181",
                    Severity::Warning,
                    "check command exit status directly instead of testing $?",
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

// Shell rules currently emit one finding per Dockerfile instruction, so this
// returns the first offending token to keep diagnostics precise without
// changing rule cardinality.
fn unquoted_command_substitution_token(program: &ShellProgram) -> Option<&ShellToken> {
    program.tokens.iter().find(|token| {
        token.kind == ShellTokenKind::Word
            && token.quote == QuoteKind::None
            && !is_assignment_word(&token.text)
            && token
                .expansions
                .iter()
                .any(|expansion| expansion.kind == ShellExpansionKind::Command)
    })
}

// Shell rules currently emit one finding per Dockerfile instruction, so this
// returns the first offending token to keep diagnostics precise without
// changing rule cardinality.
fn unquoted_variable_expansion_token(program: &ShellProgram) -> Option<&ShellToken> {
    program.tokens.iter().find(|token| {
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

fn invocation_is_useless_cat(program: &ShellProgram, invocation: &ShellCommandInvocation) -> bool {
    invocation.command_is("cat")
        && !invocation.argument_facts.is_empty()
        && invocation
            .argument_facts
            .iter()
            .all(|argument| !argument.text.starts_with('-'))
        && next_command_separator_after(program, invocation).is_some_and(|token| token.raw == "|")
}

fn invocation_tests_previous_exit_status(invocation: &ShellCommandInvocation) -> bool {
    invocation.command_is_any(&["test", "["])
        && invocation
            .argument_facts
            .iter()
            .any(|argument| argument.raw == "$?" || argument.text == "$?")
}

fn next_command_separator_after<'a>(
    program: &'a ShellProgram,
    invocation: &ShellCommandInvocation,
) -> Option<&'a rudolint_shell::ShellToken> {
    program.tokens.iter().find(|token| {
        token.span.start >= invocation.command_span.end && token.is_command_separator()
    })
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
