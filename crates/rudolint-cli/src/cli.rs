use std::path::PathBuf;

use clap::builder::Styles;
use clap::builder::styling::{AnsiColor, Color, RgbColor, Style};
use clap::{Args, Parser, Subcommand, ValueEnum};
use rudolint_diagnostics::Severity;
use rudolint_rules::Profile;

fn check_after_help() -> String {
    format!(
        "\
Finding category legend:
  {}    exits non-zero by default
  {}  exits non-zero by default
  {}     advisory finding
  {}    formatting or convention finding

Use --color never to disable ANSI colors in output.
",
        styled("x error", AnsiColor::Red),
        styled("! warning", AnsiColor::Yellow),
        styled("i info", AnsiColor::Cyan),
        styled("~ style", AnsiColor::Magenta),
    )
}

fn styled(text: &str, color: AnsiColor) -> String {
    let style = Style::new().fg_color(Some(Color::Ansi(color))).bold();
    format!("{}{text}{}", style.render(), style.render_reset())
}

fn cli_styles() -> Styles {
    Styles::styled()
        .literal(kubeply_accent_style())
        .placeholder(Style::new())
}

fn kubeply_accent_style() -> Style {
    Style::new()
        .fg_color(Some(Color::Rgb(RgbColor(0x14, 0xb8, 0xa6))))
        .bold()
}

#[derive(Debug, Parser)]
#[command(author, about, disable_version_flag = true, styles = cli_styles())]
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
    /// Upgrade rudolint using the official installer.
    Upgrade(UpgradeArgs),
}

impl Default for Command {
    fn default() -> Self {
        Self::Check(CheckArgs::default())
    }
}

#[derive(Debug, Clone, Args)]
#[command(after_help = check_after_help(), styles = cli_styles())]
pub struct CheckArgs {
    /// Dockerfile paths or directories. Directories are searched recursively.
    pub paths: Vec<PathBuf>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    /// When to use ANSI colors in output.
    #[arg(long, value_enum, default_value_t = ColorChoice::Auto)]
    pub color: ColorChoice,

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

    /// Include source excerpts in output.
    #[arg(long)]
    pub show_source: bool,

    /// Enable autofix planning.
    #[arg(long)]
    pub fix: bool,

    /// Convert `# hadolint ignore=...` comments to native `# rudolint ignore=...` comments.
    #[arg(long, requires = "fix")]
    pub migrate_hadolint_ignores: bool,

    /// Print the planned autofix output without writing files.
    #[arg(long, requires = "fix")]
    pub dry_run: bool,
}

impl Default for CheckArgs {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            format: OutputFormat::Text,
            color: ColorChoice::Auto,
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
            migrate_hadolint_ignores: false,
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

#[derive(Debug, Clone, Args)]
pub struct UpgradeArgs {
    /// Install a specific release tag instead of the latest stable release.
    #[arg(long, value_name = "TAG")]
    pub tag: Option<String>,

    /// Print the installer command without running it.
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum RulesOutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    #[value(alias = "human")]
    Text,
    Json,
    Sarif,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

#[cfg(test)]
mod tests {
    use clap::builder::styling::AnsiColor;
    use clap::{CommandFactory, Parser};
    use rudolint_rules::Profile;

    use super::{Cli, Command};

    #[test]
    fn accepts_hadolint_compat_profile_name() {
        let cli = Cli::try_parse_from(["rudolint", "check", "--profile", "hadolint-compat"])
            .expect("hadolint-compat should parse");

        let Some(Command::Check(args)) = cli.command else {
            panic!("expected check command");
        };
        assert_eq!(args.profile, Profile::HadolintCompat);
    }

    #[test]
    fn accepts_signal_profile_names() {
        for (name, expected) in [
            ("correctness", Profile::Correctness),
            ("performance", Profile::Performance),
            ("hardening", Profile::Hardening),
        ] {
            let cli = Cli::try_parse_from(["rudolint", "check", "--profile", name])
                .expect("signal profile should parse");

            let Some(Command::Check(args)) = cli.command else {
                panic!("expected check command");
            };
            assert_eq!(args.profile, expected);
        }
    }

    #[test]
    fn rejects_removed_compat_profile_alias() {
        let error = Cli::try_parse_from(["rudolint", "check", "--profile", "compat"])
            .expect_err("compat should no longer parse");

        assert_eq!(error.kind(), clap::error::ErrorKind::InvalidValue);
    }

    #[test]
    fn check_help_includes_finding_category_legend() {
        let mut command = Cli::command();
        let check = command
            .find_subcommand_mut("check")
            .expect("check subcommand should exist");
        let help = check.render_long_help().to_string();

        assert!(help.contains("Finding category legend:"));
        assert!(help.contains("x error"));
        assert!(help.contains("! warning"));
        assert!(help.contains("--color never"));
    }

    #[test]
    fn check_help_legend_uses_clap_color_policy() {
        let mut command = Cli::command().color(clap::ColorChoice::Always);
        let check = command
            .find_subcommand_mut("check")
            .expect("check subcommand should exist");
        let help = check.render_long_help().ansi().to_string();

        assert!(help.contains(&super::styled("x error", AnsiColor::Red)));
        assert!(help.contains(&super::styled("! warning", AnsiColor::Yellow)));
        assert!(help.contains(&super::styled("i info", AnsiColor::Cyan)));
        assert!(help.contains(&super::styled("~ style", AnsiColor::Magenta)));
    }

    #[test]
    fn check_help_styles_option_literals_but_not_placeholders() {
        let mut command = Cli::command().color(clap::ColorChoice::Always);
        let check = command
            .find_subcommand_mut("check")
            .expect("check subcommand should exist");
        let help = check.render_long_help().ansi().to_string();

        let styled_format = format!(
            "{}--format{}",
            super::kubeply_accent_style().render(),
            super::kubeply_accent_style().render_reset()
        );
        assert!(help.contains(&styled_format));
        assert!(help.contains("<FORMAT>"));
        let styled_placeholder = format!(
            "{}<FORMAT>{}",
            super::kubeply_accent_style().render(),
            super::kubeply_accent_style().render_reset()
        );
        assert!(!help.contains(&styled_placeholder));
    }

    #[test]
    fn check_format_uses_text_value_and_accepts_human_alias() {
        let cli = Cli::try_parse_from(["rudolint", "check", "--format", "text"])
            .expect("text output format should parse");

        let Some(Command::Check(args)) = cli.command else {
            panic!("expected check command");
        };
        assert!(matches!(args.format, super::OutputFormat::Text));

        let cli = Cli::try_parse_from(["rudolint", "check", "--format", "human"])
            .expect("human output format alias should parse");

        let Some(Command::Check(args)) = cli.command else {
            panic!("expected check command");
        };
        assert!(matches!(args.format, super::OutputFormat::Text));

        let mut command = Cli::command();
        let check = command
            .find_subcommand_mut("check")
            .expect("check subcommand should exist");
        let help = check.render_long_help().to_string();
        assert!(help.contains("[default: text]"));
        assert!(help.contains("[possible values: text, json, sarif]"));
        assert!(!help.contains("[possible values: human, json, sarif]"));
    }

    #[test]
    fn accepts_upgrade_dry_run() {
        let cli = Cli::try_parse_from(["rudolint", "upgrade", "--dry-run"])
            .expect("upgrade dry-run should parse");

        let Some(Command::Upgrade(args)) = cli.command else {
            panic!("expected upgrade command");
        };
        assert!(args.dry_run);
        assert_eq!(args.tag, None);
    }
}
