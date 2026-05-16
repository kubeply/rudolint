mod cli;

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Context;
use clap::Parser;
use ignore::WalkBuilder;

use crate::cli::{Cli, Command, OutputFormat, RulesOutputFormat};
use rudolint_config::Config;
use rudolint_diagnostics::Finding;
use rudolint_dockerfile::parse_dockerfile;
use rudolint_fix::FixPreview;
use rudolint_rules::{RuleEngine, RuleStatus};
use rudolint_settings::resolve_from_parts;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {}", error.message);
            error.code()
        }
    }
}

fn run() -> Result<ExitCode, AppError> {
    let cli = Cli::parse();
    if cli.version {
        return run_version(cli.json);
    }
    match cli.command.unwrap_or_default() {
        Command::Check(args) => run_check(args),
        Command::Rules(args) => run_rules(args),
        Command::Explain(args) => run_explain(args),
    }
}

#[derive(Debug)]
struct AppError {
    kind: AppErrorKind,
    message: String,
}

#[derive(Debug)]
enum AppErrorKind {
    Usage,
    Internal,
}

impl AppError {
    fn usage(message: impl Into<String>) -> Self {
        Self {
            kind: AppErrorKind::Usage,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            kind: AppErrorKind::Internal,
            message: message.into(),
        }
    }

    fn code(&self) -> ExitCode {
        match self.kind {
            AppErrorKind::Usage => ExitCode::from(2),
            AppErrorKind::Internal => ExitCode::from(3),
        }
    }
}

fn run_check(args: cli::CheckArgs) -> Result<ExitCode, AppError> {
    let inputs = resolve_inputs(&args.paths)?;
    let starts = if inputs.is_empty() {
        args.paths.clone()
    } else {
        inputs.clone()
    };
    let settings = resolve_from_parts(args.config.as_deref(), args.no_config, starts)?;
    let engine = RuleEngine::new(args.profile, settings.config);
    let input_count = if inputs.is_empty() { 1 } else { inputs.len() };
    let mut findings = Vec::new();
    let mut sources = BTreeMap::new();
    let fixes = Vec::<FixPreview>::new();

    if inputs.is_empty() {
        let mut source = String::new();
        io::stdin().read_to_string(&mut source).map_err(|error| {
            AppError::usage(format!("failed to read Dockerfile from stdin: {error}"))
        })?;
        findings.extend(lint_source(&args.stdin_filename, &source, &engine)?);
        sources.insert(args.stdin_filename.clone(), source);
    } else {
        for path in inputs {
            let source = fs::read_to_string(&path).map_err(|error| {
                AppError::usage(format!("failed to read {}: {error}", path.display()))
            })?;
            findings.extend(lint_source(&path, &source, &engine)?);
            sources.insert(path, source);
        }
    }

    if !args.quiet {
        let mut rendered = match args.format {
            OutputFormat::Human => rudolint_output::human(&findings),
            OutputFormat::Json => {
                if args.fix {
                    rudolint_output::json_with_fixes(&findings, &fixes).map_err(|error| {
                        AppError::internal(format!("failed to render JSON output: {error}"))
                    })?
                } else {
                    rudolint_output::json(&findings).map_err(|error| {
                        AppError::internal(format!("failed to render JSON output: {error}"))
                    })?
                }
            }
            OutputFormat::Sarif => rudolint_output::sarif(&findings).map_err(|error| {
                AppError::internal(format!("failed to render SARIF output: {error}"))
            })?,
        };
        if args.show_source && matches!(args.format, OutputFormat::Human) {
            rendered.push_str(&source_excerpt(&findings, &sources));
        }
        if args.fix && args.dry_run && matches!(args.format, OutputFormat::Human) {
            rendered.push_str("fixes: dry-run mode is enabled; no fixes are currently available\n");
        } else if args.fix && matches!(args.format, OutputFormat::Human) {
            rendered.push_str("fixes: write mode is enabled; no fixes are currently available\n");
        }
        print!("{rendered}");
    }

    if args.verbose {
        eprintln!(
            "checked {input_count} Dockerfile(s), emitted {} finding(s)",
            findings.len()
        );
    }

    if args.exit_zero {
        return Ok(ExitCode::SUCCESS);
    }

    if findings
        .iter()
        .any(|finding| finding.severity.is_failure(args.failure_threshold))
    {
        return Ok(ExitCode::from(1));
    }

    Ok(ExitCode::SUCCESS)
}

fn run_rules(args: cli::RulesArgs) -> Result<ExitCode, AppError> {
    let engine = RuleEngine::new(args.profile, Config::default());
    let rules = engine
        .catalog()
        .into_iter()
        .filter(|rule| !args.implemented || rule.status == RuleStatus::Implemented)
        .collect::<Vec<_>>();

    match args.format {
        RulesOutputFormat::Text => {
            for rule in rules {
                println!(
                    "{:<8} {:<8} {:<12} {}",
                    rule.code, rule.severity, rule.status, rule.summary
                );
            }
        }
        RulesOutputFormat::Json => {
            let rules = rules
                .into_iter()
                .map(|rule| {
                    serde_json::json!({
                        "code": rule.code,
                        "severity": rule.severity,
                        "summary": rule.summary,
                        "status": rule.status,
                    })
                })
                .collect::<Vec<_>>();
            println!(
                "{}",
                serde_json::to_string_pretty(&rules).map_err(|error| {
                    AppError::internal(format!("failed to render rule catalog JSON: {error}"))
                })?
            );
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn run_version(json: bool) -> Result<ExitCode, AppError> {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "name": env!("CARGO_PKG_NAME"),
                "version": env!("CARGO_PKG_VERSION"),
            })
        );
    } else {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    }
    Ok(ExitCode::SUCCESS)
}

fn run_explain(args: cli::ExplainArgs) -> Result<ExitCode, AppError> {
    let engine = RuleEngine::new(args.profile, Config::default());
    let rule = engine
        .catalog()
        .into_iter()
        .find(|rule| rule.code.eq_ignore_ascii_case(&args.rule))
        .ok_or_else(|| AppError::usage(format!("unknown rule `{}`", args.rule)))?;

    println!("{} {}", rule.code, rule.summary);
    println!("severity: {}", rule.severity);
    println!("status: {}", rule.status);
    Ok(ExitCode::SUCCESS)
}

fn lint_source(path: &Path, source: &str, engine: &RuleEngine) -> Result<Vec<Finding>, AppError> {
    let document = parse_dockerfile(source)
        .with_context(|| {
            format!(
                "failed to parse {}",
                path.to_str().unwrap_or("<non-utf8-path>")
            )
        })
        .map_err(|error| AppError::usage(error.to_string()))?;
    Ok(engine
        .lint(&document)
        .into_iter()
        .map(|finding| finding.with_path(path))
        .collect())
}

fn resolve_inputs(paths: &[PathBuf]) -> Result<Vec<PathBuf>, AppError> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_file() {
            files.push(path.clone());
            continue;
        }
        if !path.is_dir() {
            return Err(AppError::usage(format!(
                "{} is not a file or directory",
                path.display()
            )));
        }
        for entry in WalkBuilder::new(path).hidden(false).build() {
            let entry = entry.map_err(|error| AppError::usage(error.to_string()))?;
            if !entry.file_type().is_some_and(|kind| kind.is_file()) {
                continue;
            }
            let name = entry.file_name().to_string_lossy();
            if name == "Dockerfile" || name.starts_with("Dockerfile.") {
                files.push(entry.into_path());
            }
        }
    }
    files.sort();
    Ok(files)
}

fn source_excerpt(findings: &[Finding], sources: &BTreeMap<PathBuf, String>) -> String {
    let mut rendered = String::new();
    for finding in findings {
        let Some(source) = sources.get(&finding.path) else {
            continue;
        };
        let Some(line) = source.lines().nth(finding.line().saturating_sub(1)) else {
            continue;
        };
        rendered.push_str(&format!(
            "  |\n{:>3} | {}\n  | {}^\n",
            finding.line(),
            line,
            " ".repeat(finding.column().saturating_sub(1))
        ));
    }
    rendered
}

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        AppError::usage(format!("{error:#}"))
    }
}
