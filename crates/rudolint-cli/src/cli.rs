use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use rudolint_diagnostics::Severity;
use rudolint_rules::Profile;

#[derive(Debug, Parser)]
#[command(author, about, disable_version_flag = true)]
pub struct Cli {
    /// Print version information.
    #[arg(long, global = true)]
    pub version: bool,

    /// Print global command output as JSON where supported.
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Lint Dockerfiles.
    Check(CheckArgs),
    /// Print the rule catalog.
    Rules(RulesArgs),
    /// Explain one rule.
    Explain(ExplainArgs),
}

impl Default for Command {
    fn default() -> Self {
        Self::Check(CheckArgs::default())
    }
}

#[derive(Debug, Clone, Args)]
pub struct CheckArgs {
    /// Dockerfile paths or directories. Directories are searched recursively.
    pub paths: Vec<PathBuf>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub format: OutputFormat,

    /// Rule profile.
    #[arg(long, value_enum, default_value_t = Profile::Default)]
    pub profile: Profile,

    /// Optional .rudolint.yaml config path.
    #[arg(long, conflicts_with = "no_config")]
    pub config: Option<PathBuf>,

    /// Disable configuration discovery.
    #[arg(long)]
    pub no_config: bool,

    /// Minimum severity that exits non-zero.
    #[arg(long, value_enum, default_value_t = Severity::Warning)]
    pub failure_threshold: Severity,

    /// Always exit successfully after rendering diagnostics.
    #[arg(long)]
    pub exit_zero: bool,

    /// Display path to use when reading Dockerfile source from stdin.
    #[arg(long, default_value = "<stdin>")]
    pub stdin_filename: PathBuf,

    /// Suppress diagnostic output while preserving exit status.
    #[arg(long)]
    pub quiet: bool,

    /// Print a short run summary to stderr.
    #[arg(long)]
    pub verbose: bool,

    /// Include source excerpts in human output.
    #[arg(long)]
    pub show_source: bool,

    /// Enable autofix planning.
    #[arg(long)]
    pub fix: bool,

    /// Print the planned autofix output without writing files.
    #[arg(long, requires = "fix")]
    pub dry_run: bool,
}

impl Default for CheckArgs {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            format: OutputFormat::Human,
            profile: Profile::Default,
            config: None,
            no_config: false,
            failure_threshold: Severity::Warning,
            exit_zero: false,
            stdin_filename: PathBuf::from("<stdin>"),
            quiet: false,
            verbose: false,
            show_source: false,
            fix: false,
            dry_run: false,
        }
    }
}

#[derive(Debug, Clone, Args)]
pub struct RulesArgs {
    /// Rule profile.
    #[arg(long, value_enum, default_value_t = Profile::Default)]
    pub profile: Profile,

    /// Output format.
    #[arg(long, value_enum, default_value_t = RulesOutputFormat::Text)]
    pub format: RulesOutputFormat,

    /// Only show rules implemented in this build.
    #[arg(long)]
    pub implemented: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ExplainArgs {
    /// Rule code to explain.
    pub rule: String,

    /// Rule profile.
    #[arg(long, value_enum, default_value_t = Profile::Default)]
    pub profile: Profile,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum RulesOutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
    Sarif,
}
