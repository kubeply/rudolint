use anyhow::Result;
use rudolint_diagnostics::Finding;
use serde_json::json;

pub fn human(findings: &[Finding]) -> String {
    if findings.is_empty() {
        return String::new();
    }

    let mut rendered = String::new();
    for finding in findings {
        rendered.push_str(&format!(
            "{}:{}:{}: {} {} {}\n",
            finding.path.display(),
            finding.line(),
            finding.column(),
            finding.severity,
            finding.code,
            finding.message
        ));
    }
    rendered
}

pub fn json(findings: &[Finding]) -> Result<String> {
    Ok(format!("{}\n", serde_json::to_string_pretty(findings)?))
}

pub fn sarif(findings: &[Finding]) -> Result<String> {
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
                        "rules": []
                    }
                },
                "results": results
            }]
        }))?
    ))
}
