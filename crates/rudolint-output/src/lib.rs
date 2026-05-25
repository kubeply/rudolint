//! Render lint findings and fix previews in user-facing output formats.

use std::collections::{BTreeMap, HashSet};

use anyhow::Result;
use rudolint_diagnostics::{Finding, Severity};
use rudolint_fix::FixPreview;
use serde_json::json;

const FINDINGS_SCHEMA_VERSION: &str = "v1";

/// Human output rendering options.
#[derive(Debug, Clone, Copy, Default)]
pub struct HumanOptions {
    /// Enable ANSI color styling.
    pub color: bool,
}

/// Renders findings as line-oriented human-readable diagnostics.
pub fn human(findings: &[Finding]) -> String {
    human_with_options(findings, HumanOptions::default())
}

/// Renders findings as grouped, human-readable diagnostics.
pub fn human_with_options(findings: &[Finding], options: HumanOptions) -> String {
    if findings.is_empty() {
        return String::new();
    }

    let mut rendered = String::new();
    let mut grouped = BTreeMap::new();
    for finding in findings {
        grouped
            .entry(&finding.path)
            .or_insert_with(Vec::new)
            .push(finding);
    }

    rendered.push_str(&format!(
        "{}\n\n",
        style(
            &format!(
                "rudolint found {} in {} ({})",
                plural(findings.len(), "finding", "findings"),
                plural(grouped.len(), "file", "files"),
                severity_summary(findings)
            ),
            Style::Bold,
            options.color
        )
    ));

    let mut first_group = true;
    for (path, mut group) in grouped {
        if !first_group {
            rendered.push('\n');
        }
        first_group = false;
        rendered.push_str(&format!(
            "{}\n",
            style(&path.display().to_string(), Style::Bold, options.color)
        ));
        group.sort_by_key(|finding| (finding.line(), finding.column()));
        for finding in group {
            let severity = finding.severity.to_string();
            let severity_label = format!("{severity:<7}");
            let code_label = format!("{:<7}", finding.code);
            let location = format!("{:>4}:{:<3}", finding.line(), finding.column());
            rendered.push_str(&format!(
                "  {} {} {} {} {}\n",
                style(
                    severity_icon(finding.severity),
                    severity_style(finding.severity),
                    options.color
                ),
                style(
                    &severity_label,
                    severity_style(finding.severity),
                    options.color
                ),
                style(&code_label, Style::Bold, options.color),
                location,
                finding.message
            ));
        }
    }
    rendered
}

fn severity_summary(findings: &[Finding]) -> String {
    let mut counts = BTreeMap::<Severity, usize>::new();
    for finding in findings {
        *counts.entry(finding.severity).or_default() += 1;
    }

    [
        Severity::Error,
        Severity::Warning,
        Severity::Info,
        Severity::Style,
        Severity::Ignore,
    ]
    .into_iter()
    .filter_map(|severity| {
        counts
            .get(&severity)
            .copied()
            .filter(|count| *count > 0)
            .map(|count| {
                plural(
                    count,
                    severity_name(severity),
                    severity_plural_name(severity),
                )
            })
    })
    .collect::<Vec<_>>()
    .join(", ")
}

fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Ignore => "ignore",
        Severity::Style => "style",
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
}

fn severity_plural_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Ignore => "ignored",
        Severity::Style => "style",
        Severity::Info => "info",
        Severity::Warning => "warnings",
        Severity::Error => "errors",
    }
}

fn plural(count: usize, singular: &str, plural: &str) -> String {
    let word = if count == 1 { singular } else { plural };
    format!("{count} {word}")
}

fn severity_icon(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "x",
        Severity::Warning => "!",
        Severity::Info => "i",
        Severity::Style => "~",
        Severity::Ignore => "-",
    }
}

fn severity_style(severity: Severity) -> Style {
    match severity {
        Severity::Error => Style::RedBold,
        Severity::Warning => Style::YellowBold,
        Severity::Info => Style::CyanBold,
        Severity::Style => Style::MagentaBold,
        Severity::Ignore => Style::Dim,
    }
}

#[derive(Debug, Clone, Copy)]
enum Style {
    Bold,
    RedBold,
    YellowBold,
    CyanBold,
    MagentaBold,
    Dim,
}

fn style(text: &str, style: Style, color: bool) -> String {
    if !color {
        return text.to_string();
    }

    let code = match style {
        Style::Bold => "1",
        Style::RedBold => "1;31",
        Style::YellowBold => "1;33",
        Style::CyanBold => "1;36",
        Style::MagentaBold => "1;35",
        Style::Dim => "2",
    };
    format!("\x1b[{code}m{text}\x1b[0m")
}

/// Renders findings as the versioned JSON findings envelope.
pub fn json(findings: &[Finding]) -> Result<String> {
    Ok(format!(
        "{}\n",
        serde_json::to_string_pretty(&json!({
            "schemaVersion": FINDINGS_SCHEMA_VERSION,
            "findings": findings,
        }))?
    ))
}

/// Renders findings and fix previews as the check command's JSON envelope.
pub fn json_with_fixes(findings: &[Finding], fixes: &[FixPreview]) -> Result<String> {
    Ok(format!(
        "{}\n",
        serde_json::to_string_pretty(&json!({
            "schemaVersion": FINDINGS_SCHEMA_VERSION,
            "findings": findings,
            "fixes": fixes,
        }))?
    ))
}

/// Renders findings as a SARIF 2.1.0 report.
pub fn sarif(findings: &[Finding]) -> Result<String> {
    let mut rule_codes = HashSet::with_capacity(findings.len());
    let rules = findings
        .iter()
        .filter_map(|finding| {
            if !rule_codes.insert(finding.code.as_str()) {
                return None;
            }
            Some(json!({
                "id": finding.code,
                "shortDescription": { "text": finding.code },
                "defaultConfiguration": {
                    "level": finding.severity.sarif_level()
                }
            }))
        })
        .collect::<Vec<_>>();

    let results = findings
        .iter()
        .map(|finding| {
            json!({
                "ruleId": finding.code,
                "level": finding.severity.sarif_level(),
                "message": { "text": finding.message },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": finding.path.to_string_lossy() },
                        "region": {
                            "startLine": finding.primary_span.start_line,
                            "startColumn": finding.primary_span.start_column,
                            "endLine": finding.primary_span.end_line,
                            "endColumn": finding.primary_span.end_column
                        }
                    }
                }]
            })
        })
        .collect::<Vec<_>>();

    Ok(format!(
        "{}\n",
        serde_json::to_string_pretty(&json!({
            "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
            "version": "2.1.0",
            "runs": [{
                "tool": {
                    "driver": {
                        "name": "rudolint",
                        "informationUri": "https://github.com/kubeply/rudolint",
                        "rules": rules
                    }
                },
                "results": results
            }]
        }))?
    ))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use rudolint_diagnostics::{Finding, Severity};

    use super::{HumanOptions, human_with_options};

    #[test]
    fn human_output_groups_findings_with_summary() {
        let findings = vec![
            Finding::new(
                "DL3007",
                Severity::Warning,
                "avoid mutable latest base image tags",
                1,
                1,
            )
            .with_path(Path::new("Dockerfile")),
            Finding::new(
                "DL3000",
                Severity::Error,
                "WORKDIR should be absolute",
                2,
                1,
            )
            .with_path(Path::new("Dockerfile")),
        ];

        assert_eq!(
            human_with_options(&findings, HumanOptions { color: false }),
            "rudolint found 2 findings in 1 file (1 error, 1 warning)\n\nDockerfile\n  ! warning DL3007     1:1   avoid mutable latest base image tags\n  x error   DL3000     2:1   WORKDIR should be absolute\n"
        );
    }

    #[test]
    fn human_output_can_render_ansi_colors() {
        let findings = vec![
            Finding::new(
                "DL3000",
                Severity::Error,
                "WORKDIR should be absolute",
                2,
                1,
            )
            .with_path(Path::new("Dockerfile")),
        ];

        let output = human_with_options(&findings, HumanOptions { color: true });

        assert!(output.contains("\x1b[1mDockerfile\x1b[0m"));
        assert!(output.contains("\x1b[1;31mx\x1b[0m"));
        assert!(output.contains("\x1b[1;31merror  \x1b[0m"));
    }
}
