//! Language server integration points.

use lsp_types::{Diagnostic, DiagnosticSeverity, DiagnosticTag, NumberOrString, Position, Range};
use rudolint_diagnostics::{Finding, Severity};
use rudolint_source::Span;

/// Converts a [`Finding`] into an LSP [`Diagnostic`].
///
/// `rudolint` spans are one-based line and column positions. LSP ranges use
/// zero-based line and character positions, so the conversion saturates at zero
/// for defensive handling of synthetic diagnostics.
pub fn diagnostic(finding: &Finding) -> Diagnostic {
    Diagnostic {
        range: range(finding.primary_span),
        severity: Some(severity(finding.severity)),
        code: Some(NumberOrString::String(finding.code.clone())),
        code_description: None,
        source: Some("rudolint".to_string()),
        message: message(finding),
        related_information: None,
        tags: tags(finding.severity),
        data: None,
    }
}

/// Converts a set of [`Finding`] values into LSP [`Diagnostic`] values.
pub fn diagnostics(findings: &[Finding]) -> Vec<Diagnostic> {
    findings.iter().map(diagnostic).collect()
}

/// Converts a `rudolint` source [`Span`] into an LSP [`Range`].
pub fn range(span: Span) -> Range {
    Range {
        start: position(span.start_line, span.start_column),
        end: position(span.end_line, span.end_column),
    }
}

/// Converts a `rudolint` one-based line and column pair into an LSP
/// zero-based [`Position`].
pub fn position(line: usize, column: usize) -> Position {
    Position {
        line: zero_based(line),
        character: zero_based(column),
    }
}

/// Converts a `rudolint` [`Severity`] into LSP [`DiagnosticSeverity`].
pub fn severity(severity: Severity) -> DiagnosticSeverity {
    match severity {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Info => DiagnosticSeverity::INFORMATION,
        Severity::Style | Severity::Ignore => DiagnosticSeverity::HINT,
    }
}

fn message(finding: &Finding) -> String {
    match &finding.help {
        Some(help) => format!("{}\n\n{}", finding.message, help),
        None => finding.message.clone(),
    }
}

fn tags(severity: Severity) -> Option<Vec<DiagnosticTag>> {
    (severity == Severity::Ignore).then(|| vec![DiagnosticTag::UNNECESSARY])
}

fn zero_based(value: usize) -> u32 {
    value.saturating_sub(1).try_into().unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use lsp_types::{DiagnosticSeverity, NumberOrString};
    use rudolint_diagnostics::{Finding, Severity};
    use rudolint_source::Span;

    use super::{diagnostic, diagnostics, position, range};

    #[test]
    fn converts_finding_to_lsp_diagnostic() {
        let finding = Finding::with_span(
            "RDL3007",
            Severity::Warning,
            "Avoid latest tag",
            Span {
                start_byte: 5,
                end_byte: 11,
                start_line: 2,
                start_column: 6,
                end_line: 2,
                end_column: 12,
            },
        )
        .with_help("Pin the image tag.");

        let diagnostic = diagnostic(&finding);

        assert_eq!(diagnostic.range.start.line, 1);
        assert_eq!(diagnostic.range.start.character, 5);
        assert_eq!(diagnostic.range.end.line, 1);
        assert_eq!(diagnostic.range.end.character, 11);
        assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::WARNING));
        assert_eq!(
            diagnostic.code,
            Some(NumberOrString::String("RDL3007".to_string()))
        );
        assert_eq!(diagnostic.source.as_deref(), Some("rudolint"));
        assert_eq!(diagnostic.message, "Avoid latest tag\n\nPin the image tag.");
    }

    #[test]
    fn converts_many_findings_without_reordering() {
        let first = Finding::new("RDL3000", Severity::Error, "Use absolute WORKDIR", 1, 1);
        let second = Finding::new("RDL4000", Severity::Info, "MAINTAINER is deprecated", 3, 1);

        let diagnostics = diagnostics(&[first, second]);

        assert_eq!(diagnostics.len(), 2);
        assert_eq!(
            diagnostics[0].code,
            Some(NumberOrString::String("RDL3000".to_string()))
        );
        assert_eq!(
            diagnostics[1].code,
            Some(NumberOrString::String("RDL4000".to_string()))
        );
        assert_eq!(
            diagnostics[1].severity,
            Some(DiagnosticSeverity::INFORMATION)
        );
    }

    #[test]
    fn converts_positions_and_ranges_to_lsp_zero_based_coordinates() {
        assert_eq!(position(1, 1), lsp_types::Position::new(0, 0));

        let range = range(Span {
            start_byte: 0,
            end_byte: 0,
            start_line: 0,
            start_column: 0,
            end_line: 2,
            end_column: 4,
        });

        assert_eq!(range.start, lsp_types::Position::new(0, 0));
        assert_eq!(range.end, lsp_types::Position::new(1, 3));
    }

    #[test]
    fn marks_ignored_severity_as_unnecessary_hint() {
        let diagnostic = diagnostic(&Finding::new(
            "RDL3007",
            Severity::Ignore,
            "Avoid latest tag",
            1,
            1,
        ));

        assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::HINT));
        assert_eq!(
            diagnostic.tags,
            Some(vec![lsp_types::DiagnosticTag::UNNECESSARY])
        );
    }
}
