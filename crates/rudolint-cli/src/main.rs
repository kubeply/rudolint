mod cli;

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, ExitCode};

use anyhow::Context;
use clap::Parser;
use ignore::WalkBuilder;

use crate::cli::{
    Cli, ColorChoice, Command, ConfigCommand, OutputFormat, OutputGroupBy, RulesOutputFormat,
};
use rudolint_config::Config;
use rudolint_diagnostics::Finding;
use rudolint_dockerfile::parse_dockerfile;
use rudolint_fix::{FixPreview, TextEdit, apply_edits};
use rudolint_rules::{RuleEngine, RuleStatus};
use rudolint_settings::resolve_from_parts;
use rudolint_source::SourceSpan;

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
        Command::Config(command) => run_config(command),
        Command::Rules(args) => run_rules(args),
        Command::Explain(args) => run_explain(args),
        Command::Upgrade(args) => run_upgrade(args, cli.json),
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
    if inputs.is_empty() && args.fix && !args.dry_run {
        return Err(AppError::usage(
            "`--fix` write mode requires a Dockerfile path",
        ));
    }
    let starts = if inputs.is_empty() {
        args.paths.clone()
    } else {
        inputs.clone()
    };
    let settings = resolve_from_parts(args.config.as_deref(), args.no_config, starts)?;
    let config_path = settings.config_path.clone();
    let engine = RuleEngine::new(args.profile, settings.config);
    let input_count = if inputs.is_empty() { 1 } else { inputs.len() };
    let mut findings = Vec::new();
    let mut sources = BTreeMap::new();
    let mut fixes = Vec::<FixPreview>::new();

    if inputs.is_empty() {
        let mut source = String::new();
        io::stdin().read_to_string(&mut source).map_err(|error| {
            AppError::usage(format!("failed to read Dockerfile from stdin: {error}"))
        })?;
        let analysis = analyze_source(
            &args.stdin_filename,
            &args.stdin_filename,
            &source,
            &engine,
            args.fix,
            args.migrate_hadolint_ignores,
        )?;
        findings.extend(analysis.findings);
        fixes.extend(analysis.fixes);
        sources.insert(args.stdin_filename.clone(), source);
    } else {
        for path in inputs {
            let source = fs::read_to_string(&path).map_err(|error| {
                AppError::usage(format!("failed to read {}: {error}", path.display()))
            })?;
            let lint_path = lint_path_for_config(&config_path, &path);
            let analysis = analyze_source(
                &path,
                &lint_path,
                &source,
                &engine,
                args.fix,
                args.migrate_hadolint_ignores,
            )?;
            if args.fix && !args.dry_run {
                apply_fixes(&path, &source, &analysis.fixes)?;
            }
            findings.extend(analysis.findings);
            fixes.extend(analysis.fixes);
            sources.insert(path, source);
        }
    }

    if !args.quiet {
        let mut rendered = match args.format {
            OutputFormat::Text => rudolint_output::human_with_options(
                &findings,
                rudolint_output::HumanOptions {
                    color: should_colorize(args.color),
                    group_by: match args.group_by {
                        OutputGroupBy::Rule => rudolint_output::HumanGroupBy::Rule,
                        OutputGroupBy::File => rudolint_output::HumanGroupBy::File,
                    },
                    max_examples_per_group: args.max_examples_per_group,
                },
            ),
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
        if args.show_source && matches!(args.format, OutputFormat::Text) {
            rendered.push_str(&source_excerpt(&findings, &sources));
        }
        if args.fix && matches!(args.format, OutputFormat::Text) {
            rendered.push_str(&render_fix_section(args.dry_run, &fixes));
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

fn should_colorize(choice: ColorChoice) -> bool {
    match choice {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => {
            env::var_os("NO_COLOR").is_none()
                && env::var_os("CLICOLOR").is_none_or(|value| value != "0")
                && io::stdout().is_terminal()
        }
    }
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

fn run_config(command: ConfigCommand) -> Result<ExitCode, AppError> {
    match command {
        ConfigCommand::IgnoreTemplates(args) => run_config_ignore_templates(args),
    }
}

fn run_config_ignore_templates(args: cli::ConfigIgnoreTemplatesArgs) -> Result<ExitCode, AppError> {
    let template_patterns = template_ignore_patterns(&args.paths, &args.config)?;
    let rendered = updated_config_with_template_ignores(&args.config, &template_patterns)?;

    if args.dry_run {
        print!("{rendered}");
        return Ok(ExitCode::SUCCESS);
    }

    if let Some(parent) = args
        .config
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| {
            AppError::usage(format!(
                "failed to create config directory {}: {error}",
                parent.display()
            ))
        })?;
    }
    fs::write(&args.config, rendered).map_err(|error| {
        AppError::usage(format!(
            "failed to write {}: {error}",
            args.config.display()
        ))
    })?;
    println!("updated {}", args.config.display());
    Ok(ExitCode::SUCCESS)
}

fn template_ignore_patterns(
    paths: &[PathBuf],
    config_path: &Path,
) -> Result<BTreeSet<String>, AppError> {
    let mut patterns = BTreeSet::from([
        "*.template".to_string(),
        "**/*.template".to_string(),
        "*.tmpl".to_string(),
        "**/*.tmpl".to_string(),
    ]);

    for path in resolve_inputs(paths)? {
        let source = fs::read_to_string(&path).map_err(|error| {
            AppError::usage(format!("failed to read {}: {error}", path.display()))
        })?;
        if is_template_like_source(&source) && !is_common_template_path(&path) {
            patterns.insert(path_for_config_pattern(config_path, &path));
        }
    }

    Ok(patterns)
}

fn updated_config_with_template_ignores(
    config_path: &Path,
    patterns: &BTreeSet<String>,
) -> Result<String, AppError> {
    let mut config = if config_path.exists() {
        let raw = fs::read_to_string(config_path).map_err(|error| {
            AppError::usage(format!("failed to read {}: {error}", config_path.display()))
        })?;
        if raw.trim().is_empty() {
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
        } else {
            serde_yaml::from_str::<serde_yaml::Value>(&raw).map_err(|error| {
                AppError::usage(format!(
                    "failed to parse {}: {error}",
                    config_path.display()
                ))
            })?
        }
    } else {
        serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
    };

    let mapping = config
        .as_mapping_mut()
        .ok_or_else(|| AppError::usage("rudolint config must be a YAML mapping"))?;
    let per_file_key = serde_yaml::Value::String("per-file-ignores".to_string());
    mapping
        .entry(per_file_key.clone())
        .or_insert_with(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
    let per_file = mapping
        .get_mut(&per_file_key)
        .and_then(serde_yaml::Value::as_mapping_mut)
        .ok_or_else(|| AppError::usage("`per-file-ignores` must be a YAML mapping"))?;

    for pattern in patterns {
        merge_template_ignore_pattern(per_file, pattern)?;
    }

    let updated_config = serde_yaml::from_value::<Config>(config.clone())
        .map_err(|error| AppError::usage(format!("updated config would be invalid: {error}")))?;
    updated_config
        .validate(config_path)
        .map_err(|error| AppError::usage(format!("updated config would be invalid: {error:#}")))?;
    let mut rendered = serde_yaml::to_string(&config)
        .map_err(|error| AppError::internal(format!("failed to render config: {error}")))?;
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    Ok(rendered)
}

fn merge_template_ignore_pattern(
    per_file: &mut serde_yaml::Mapping,
    pattern: &str,
) -> Result<(), AppError> {
    let key = serde_yaml::Value::String(pattern.to_string());
    per_file
        .entry(key.clone())
        .or_insert_with(|| serde_yaml::Value::Sequence(Vec::new()));
    let rules = per_file
        .get_mut(&key)
        .and_then(serde_yaml::Value::as_sequence_mut)
        .ok_or_else(|| {
            AppError::usage(format!(
                "`per-file-ignores.{pattern}` must be a YAML sequence"
            ))
        })?;

    let existing = rules
        .iter()
        .filter_map(serde_yaml::Value::as_str)
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    for prefix in ["DL", "SC", "RDK", "RUD"] {
        if !existing.contains(prefix) {
            rules.push(serde_yaml::Value::String(prefix.to_string()));
        }
    }
    Ok(())
}

fn is_common_template_path(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name.ends_with(".template") || name.ends_with(".tmpl")
}

fn is_template_like_source(source: &str) -> bool {
    let trimmed = source.trim_start();
    trimmed.starts_with("{{") || trimmed.starts_with("{%") || trimmed.starts_with("<%")
}

fn path_for_config_pattern(config_path: &Path, path: &Path) -> String {
    let config_parent = config_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let absolute_base = config_parent
        .canonicalize()
        .unwrap_or_else(|_| config_parent.clone());
    let absolute_path = path.canonicalize().unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path)
        }
    });
    let relative = absolute_path
        .strip_prefix(&absolute_base)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| lint_path_for_config(&Some(config_path.to_path_buf()), path));
    path_to_glob_string(&relative)
}

fn path_to_glob_string(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
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

fn run_upgrade(args: cli::UpgradeArgs, json: bool) -> Result<ExitCode, AppError> {
    run_upgrade_with(args, json, resolve_latest_release_tag, run_installer)
}

fn run_upgrade_with<L, I>(
    args: cli::UpgradeArgs,
    json: bool,
    latest_release_tag: L,
    install: I,
) -> Result<ExitCode, AppError>
where
    L: FnOnce() -> Result<String, AppError>,
    I: FnOnce(&str) -> Result<(), AppError>,
{
    if args.dry_run {
        let installer_url = upgrade_installer_url(args.tag.as_deref())?;
        let command = installer_command(&installer_url);
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "installer_url": installer_url,
                    "command": command,
                })
            );
        } else {
            println!("{command}");
        }
        return Ok(ExitCode::SUCCESS);
    }

    let target_tag = upgrade_target_tag(args.tag.as_deref(), latest_release_tag)?;
    let current_tag = current_release_tag();
    if target_tag == current_tag {
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "status": "up_to_date",
                    "current_version": current_tag,
                    "target_version": target_tag,
                })
            );
        } else {
            println!("rudolint is already up to date ({current_tag})");
        }
        return Ok(ExitCode::SUCCESS);
    }

    let installer_url = upgrade_installer_url(Some(&target_tag))?;
    let command = installer_command(&installer_url);
    install(&command)?;

    Ok(ExitCode::SUCCESS)
}

fn run_installer(command: &str) -> Result<(), AppError> {
    if cfg!(windows) {
        return Err(AppError::usage(
            "`rudolint upgrade` currently requires a Unix-like shell",
        ));
    }

    let status = ProcessCommand::new("sh")
        .arg("-c")
        .arg(command)
        .status()
        .map_err(|error| AppError::internal(format!("failed to run installer: {error}")))?;

    if !status.success() {
        return Err(AppError::internal(format!(
            "installer exited with status {status}"
        )));
    }

    Ok(())
}

fn upgrade_target_tag<L>(version: Option<&str>, latest_release_tag: L) -> Result<String, AppError>
where
    L: FnOnce() -> Result<String, AppError>,
{
    match version {
        Some(version) => normalize_release_tag(version),
        None => latest_release_tag(),
    }
}

fn current_release_tag() -> String {
    format!("v{}", env!("CARGO_PKG_VERSION"))
}

fn resolve_latest_release_tag() -> Result<String, AppError> {
    let output = ProcessCommand::new("curl")
        .args([
            "--proto",
            "=https",
            "--tlsv1.2",
            "-LsSf",
            "https://api.github.com/repos/kubeply/rudolint/releases/latest",
        ])
        .output()
        .map_err(|error| AppError::internal(format!("failed to query latest release: {error}")))?;

    if !output.status.success() {
        return Err(AppError::internal(format!(
            "failed to query latest release: curl exited with status {}",
            output.status
        )));
    }

    let response: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| AppError::internal(format!("failed to parse latest release: {error}")))?;
    let tag = response
        .get("tag_name")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| AppError::internal("latest release response did not include tag_name"))?;

    normalize_release_tag(tag)
}

fn upgrade_installer_url(version: Option<&str>) -> Result<String, AppError> {
    let Some(version) = version else {
        return Ok("https://kubeply.com/rudolint/install.sh".to_string());
    };

    let normalized = normalize_release_tag(version)?;
    Ok(format!(
        "https://kubeply.com/rudolint/{normalized}/install.sh"
    ))
}

fn normalize_release_tag(version: &str) -> Result<String, AppError> {
    let trimmed = version.trim();
    let version = trimmed.strip_prefix('v').unwrap_or(trimmed);
    semver::Version::parse(version).map_err(|error| {
        AppError::usage(format!("invalid release version `{trimmed}`: {error}"))
    })?;

    Ok(format!("v{version}"))
}

fn installer_command(installer_url: &str) -> String {
    format!("curl --proto '=https' --tlsv1.2 -LsSf {installer_url} | sh")
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

struct Analysis {
    findings: Vec<Finding>,
    fixes: Vec<FixPreview>,
}

fn analyze_source(
    path: &Path,
    lint_path: &Path,
    source: &str,
    engine: &RuleEngine,
    collect_fixes: bool,
    migrate_hadolint_ignores: bool,
) -> Result<Analysis, AppError> {
    let document = parse_dockerfile(source)
        .with_context(|| {
            format!(
                "failed to parse {}",
                path.to_str().unwrap_or("<non-utf8-path>")
            )
        })
        .map_err(|error| AppError::usage(error.to_string()))?;
    let findings = engine
        .lint_path(lint_path, &document)
        .into_iter()
        .map(|finding| finding.with_path(path))
        .collect();
    let fixes = if collect_fixes {
        let mut fixes = engine.fixes_path(lint_path, &document);
        if migrate_hadolint_ignores {
            fixes.extend(hadolint_ignore_migration_fixes(source));
        }
        fixes
    } else {
        Vec::new()
    };
    Ok(Analysis { findings, fixes })
}

fn hadolint_ignore_migration_fixes(source: &str) -> Vec<FixPreview> {
    source
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let column = hadolint_ignore_command_column(line)?;
            Some(FixPreview {
                title: "convert hadolint inline suppression to rudolint".to_string(),
                applicability: rudolint_fix::FixApplicability::safe(),
                edits: vec![TextEdit::replace(
                    SourceSpan {
                        line: index + 1,
                        column,
                        length: "hadolint".len(),
                    },
                    "rudolint",
                )],
            })
        })
        .collect()
}

fn hadolint_ignore_command_column(line: &str) -> Option<usize> {
    let hash_index = line.find('#')?;
    if !line[..hash_index].trim().is_empty() {
        return None;
    }
    let after_hash = &line[hash_index + 1..];
    let leading_whitespace = after_hash
        .char_indices()
        .find_map(|(index, character)| (!character.is_whitespace()).then_some(index))
        .unwrap_or(after_hash.len());
    let command_start = hash_index + 1 + leading_whitespace;
    let rest = &line[command_start..];
    let command = rest.get(.."hadolint".len())?;
    if !command.eq_ignore_ascii_case("hadolint") {
        return None;
    }
    let after_command = &rest["hadolint".len()..];
    let whitespace = after_command
        .char_indices()
        .find_map(|(index, character)| (!character.is_whitespace()).then_some(index))
        .unwrap_or(after_command.len());
    let directive = &after_command[whitespace..];
    directive
        .starts_with("ignore=")
        .then_some(command_start + 1)
}

fn lint_path_for_config(config_path: &Option<PathBuf>, path: &Path) -> PathBuf {
    let Some(config_parent) = config_path.as_ref().and_then(|path| path.parent()) else {
        return path.to_path_buf();
    };

    let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let canonical_parent = config_parent
        .canonicalize()
        .unwrap_or_else(|_| config_parent.to_path_buf());

    canonical_path
        .strip_prefix(canonical_parent)
        .map(Path::to_path_buf)
        .unwrap_or(canonical_path)
}

fn apply_fixes(path: &Path, source: &str, fixes: &[FixPreview]) -> Result<(), AppError> {
    let edits = safe_edits(fixes);
    if edits.is_empty() {
        return Ok(());
    }
    let edited = apply_edits(source, &edits)
        .map_err(|error| AppError::internal(format!("failed to apply fixes: {error:?}")))?;
    fs::write(path, edited)
        .map_err(|error| AppError::usage(format!("failed to write {}: {error}", path.display())))
}

fn safe_edits(fixes: &[FixPreview]) -> Vec<TextEdit> {
    fixes
        .iter()
        .filter(|fix| fix.applicability.is_automatically_applicable())
        .flat_map(|fix| fix.edits.iter().cloned())
        .collect()
}

fn render_fix_section(dry_run: bool, fixes: &[FixPreview]) -> String {
    let mode = if dry_run { "dry-run" } else { "write" };
    if fixes.is_empty() {
        return format!("fixes: {mode} mode is enabled; no fixes are currently available\n");
    }

    let mut rendered = format!(
        "fixes: {mode} mode is enabled; {} fix(es) available\n",
        fixes.len()
    );
    for fix in fixes {
        rendered.push_str(&fix.render());
    }
    rendered
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
            if is_discovered_dockerfile_name(&name) {
                files.push(entry.into_path());
            }
        }
    }
    files.sort();
    Ok(files)
}

fn is_discovered_dockerfile_name(name: &str) -> bool {
    name == "Dockerfile"
        || name.starts_with("Dockerfile.")
        || name.starts_with("Dockerfile_")
        || name.starts_with("Dockerfile-")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrade_skips_installer_when_latest_is_current_version() {
        let args = cli::UpgradeArgs {
            tag: None,
            dry_run: false,
        };

        let result = run_upgrade_with(
            args,
            false,
            || Ok(current_release_tag()),
            |_| panic!("installer should not run when rudolint is current"),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn upgrade_runs_installer_when_latest_is_newer() {
        let args = cli::UpgradeArgs {
            tag: None,
            dry_run: false,
        };
        let mut install_command = None;

        let result = run_upgrade_with(
            args,
            false,
            || Ok("v999.0.0".to_string()),
            |command| {
                install_command = Some(command.to_string());
                Ok(())
            },
        );

        assert!(result.is_ok());
        let install_command = install_command.expect("installer should run");
        assert!(install_command.contains("https://kubeply.com/rudolint/v999.0.0/install.sh"));
    }
}
