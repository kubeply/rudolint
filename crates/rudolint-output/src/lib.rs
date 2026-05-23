//! Render lint findings and fix previews in user-facing output formats.

use std::collections::{BTreeMap, HashSet};

use anyhow::Result;
use rudolint_diagnostics::Finding;
use rudolint_fix::FixPreview;
use serde_json::json;

const FINDINGS_SCHEMA_VERSION: &str = "v1";

/// Renders findings as line-oriented human-readable diagnostics.
pub fn human(findings: &[Finding]) -> String {
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

    for (path, mut group) in grouped {
        rendered.push_str(&format!("{}:\n", path.display()));
        group.sort_by_key(|finding| (finding.line(), finding.column()));
        for finding in group {
            rendered.push_str(&format!(
                "  {}:{} {} {} {}\n",
                finding.line(),
                finding.column(),
                finding.severity,
                finding.code,
                finding.message
            ));
        }
    }
    rendered
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
