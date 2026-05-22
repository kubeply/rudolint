//! Language server integration points.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use lsp_types::{Diagnostic, DiagnosticSeverity, DiagnosticTag, NumberOrString, Position, Range};
use rudolint_diagnostics::{Finding, Severity};
use rudolint_dockerfile::parse_dockerfile;
use rudolint_rules::{Profile, RuleEngine};
use rudolint_settings::{Settings, SettingsOptions};
use rudolint_source::Span;
use url::Url;

/// Lints open editor documents using the same parser and rule engine as the CLI.
#[derive(Debug, Clone)]
pub struct DocumentLinter {
    profile: Profile,
    settings: Settings,
}

impl Default for DocumentLinter {
    fn default() -> Self {
        Self {
            profile: Profile::Default,
            settings: Settings::default(),
        }
    }
}

impl DocumentLinter {
    /// Creates a document linter from a rule profile and resolved settings.
    pub fn new(profile: Profile, settings: Settings) -> Self {
        Self { profile, settings }
    }

    /// Creates a document linter by discovering configuration from a document URI.
    pub fn discover_for_document(profile: Profile, uri: &lsp_types::Uri) -> Result<Self> {
        let search_starts = search_start_for_uri(uri).into_iter().collect::<Vec<_>>();
        let settings = rudolint_settings::resolve(
            &SettingsOptions::default().with_search_starts(search_starts),
        )?;
        Ok(Self::new(profile, settings))
    }

    /// Creates a document linter by discovering configuration from LSP workspace folders.
    pub fn discover_for_workspace(
        profile: Profile,
        workspace_folders: &[lsp_types::WorkspaceFolder],
    ) -> Result<Self> {
        let search_starts = workspace_folders
            .iter()
            .filter_map(|folder| search_start_for_uri(&folder.uri))
            .collect::<Vec<_>>();
        let settings = rudolint_settings::resolve(
            &SettingsOptions::default().with_search_starts(search_starts),
        )?;
        Ok(Self::new(profile, settings))
    }

    /// Lints a document received through `textDocument/didOpen`.
    pub fn lint_open_document(
        &self,
        document: &lsp_types::TextDocumentItem,
    ) -> Result<Vec<Diagnostic>> {
        self.lint_uri(&document.uri, &document.text)
    }

    /// Lints a full-document `textDocument/didChange` notification.
    ///
    /// This helper expects the client to use full document synchronization. It
    /// returns an error for incremental changes so callers do not accidentally
    /// lint a partial replacement as the whole Dockerfile.
    pub fn lint_changed_document(
        &self,
        params: &lsp_types::DidChangeTextDocumentParams,
    ) -> Result<Vec<Diagnostic>> {
        let Some(text) = full_document_text(params)? else {
            return Ok(Vec::new());
        };

        self.lint_uri(&params.text_document.uri, text)
    }

    fn lint_uri(&self, uri: &lsp_types::Uri, text: &str) -> Result<Vec<Diagnostic>> {
        let display_path = document_path(uri);
        let lint_path = lint_path_for_config(&self.settings.config_path, &display_path);
        let parsed =
            parse_dockerfile(text).with_context(|| format!("failed to parse {}", uri.as_str()))?;
        let engine = RuleEngine::new(self.profile, self.settings.config.clone());
        let findings = engine
            .lint_path(&lint_path, &parsed)
            .into_iter()
            .map(|finding| finding.with_path(&display_path))
            .collect::<Vec<_>>();

        Ok(diagnostics(&findings))
    }
}

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

fn document_path(uri: &lsp_types::Uri) -> PathBuf {
    file_path_for_uri(uri).unwrap_or_else(|| PathBuf::from(uri.as_str()))
}

fn search_start_for_uri(uri: &lsp_types::Uri) -> Option<PathBuf> {
    let path = file_path_for_uri(uri)?;
    if path.exists() {
        return Some(path);
    }

    path.parent().map(Path::to_path_buf)
}

fn file_path_for_uri(uri: &lsp_types::Uri) -> Option<PathBuf> {
    Url::parse(uri.as_str())
        .ok()
        .filter(|url| url.scheme() == "file")
        .and_then(|url| url.to_file_path().ok())
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

fn full_document_text(params: &lsp_types::DidChangeTextDocumentParams) -> Result<Option<&str>> {
    if params
        .content_changes
        .iter()
        .any(|change| change.range.is_some() || change.range_length.is_some())
    {
        bail!(
            "incremental textDocument/didChange is not supported yet for {}",
            params.text_document.uri.as_str()
        );
    }

    let Some(change) = params.content_changes.last() else {
        return Ok(None);
    };

    Ok(Some(change.text.as_str()))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;
    use std::str::FromStr;

    use lsp_types::{DiagnosticSeverity, NumberOrString};
    use rudolint_config::Config;
    use rudolint_diagnostics::{Finding, Severity};
    use rudolint_settings::Settings;
    use rudolint_source::Span;

    use super::{DocumentLinter, diagnostic, diagnostics, position, range};

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

    #[test]
    fn lints_open_document_with_default_settings() {
        let linter = DocumentLinter::default();
        let document = lsp_types::TextDocumentItem::new(
            lsp_types::Uri::from_str("file:///workspace/Dockerfile").expect("uri should parse"),
            "dockerfile".to_string(),
            1,
            "FROM alpine:latest\n".to_string(),
        );

        let diagnostics = linter
            .lint_open_document(&document)
            .expect("open document should lint");

        assert!(diagnostics.iter().any(
            |diagnostic| diagnostic.code == Some(NumberOrString::String("RDL3007".to_string()))
        ));
    }

    #[test]
    fn open_document_linting_uses_resolved_settings() {
        let linter = DocumentLinter::new(
            rudolint_rules::Profile::Default,
            Settings {
                config: Config {
                    ignore: BTreeSet::from(["RDL3007".to_string()]),
                    ..Config::default()
                },
                config_path: None,
            },
        );
        let document = lsp_types::TextDocumentItem::new(
            lsp_types::Uri::from_str("untitled:Untitled-1").expect("uri should parse"),
            "dockerfile".to_string(),
            1,
            "FROM alpine:latest\n".to_string(),
        );

        let diagnostics = linter
            .lint_open_document(&document)
            .expect("open document should lint");

        assert!(diagnostics.iter().all(
            |diagnostic| diagnostic.code != Some(NumberOrString::String("RDL3007".to_string()))
        ));
    }

    #[test]
    fn lints_full_document_changes() {
        let linter = DocumentLinter::default();
        let params = lsp_types::DidChangeTextDocumentParams {
            text_document: lsp_types::VersionedTextDocumentIdentifier::new(
                lsp_types::Uri::from_str("file:///workspace/Dockerfile").expect("uri should parse"),
                2,
            ),
            content_changes: vec![lsp_types::TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "FROM alpine:latest\n".to_string(),
            }],
        };

        let diagnostics = linter
            .lint_changed_document(&params)
            .expect("full document change should lint");

        assert!(diagnostics.iter().any(
            |diagnostic| diagnostic.code == Some(NumberOrString::String("RDL3007".to_string()))
        ));
    }

    #[test]
    fn skips_empty_document_change_batches() {
        let linter = DocumentLinter::default();
        let params = lsp_types::DidChangeTextDocumentParams {
            text_document: lsp_types::VersionedTextDocumentIdentifier::new(
                lsp_types::Uri::from_str("file:///workspace/Dockerfile").expect("uri should parse"),
                2,
            ),
            content_changes: Vec::new(),
        };

        let diagnostics = linter
            .lint_changed_document(&params)
            .expect("empty change batch should be accepted");

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn rejects_incremental_document_changes() {
        let linter = DocumentLinter::default();
        let params = lsp_types::DidChangeTextDocumentParams {
            text_document: lsp_types::VersionedTextDocumentIdentifier::new(
                lsp_types::Uri::from_str("file:///workspace/Dockerfile").expect("uri should parse"),
                2,
            ),
            content_changes: vec![lsp_types::TextDocumentContentChangeEvent {
                range: Some(lsp_types::Range::new(
                    lsp_types::Position::new(0, 0),
                    lsp_types::Position::new(0, 4),
                )),
                range_length: None,
                text: "FROM".to_string(),
            }],
        };

        let error = linter
            .lint_changed_document(&params)
            .expect_err("incremental change should not be linted as a full document");

        assert!(error.to_string().contains("incremental"));
    }

    #[test]
    fn rejects_mixed_full_and_incremental_document_change_batches() {
        let linter = DocumentLinter::default();
        let params = lsp_types::DidChangeTextDocumentParams {
            text_document: lsp_types::VersionedTextDocumentIdentifier::new(
                lsp_types::Uri::from_str("file:///workspace/Dockerfile").expect("uri should parse"),
                2,
            ),
            content_changes: vec![
                lsp_types::TextDocumentContentChangeEvent {
                    range: Some(lsp_types::Range::new(
                        lsp_types::Position::new(0, 0),
                        lsp_types::Position::new(0, 4),
                    )),
                    range_length: None,
                    text: "FROM".to_string(),
                },
                lsp_types::TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: "FROM alpine:latest\n".to_string(),
                },
            ],
        };

        let error = linter
            .lint_changed_document(&params)
            .expect_err("mixed change batch should not be linted as a full document");

        assert!(error.to_string().contains("incremental"));
    }

    #[test]
    fn discovers_config_from_document_uri() {
        let temp = tempfile::TempDir::new().expect("temp dir should be created");
        std::fs::write(temp.path().join(".rudolint.yaml"), "ignore:\n  - RDL3007\n")
            .expect("config should be written");
        let dockerfile = temp.path().join("Dockerfile");
        let uri = file_uri(&dockerfile);
        let linter = DocumentLinter::discover_for_document(rudolint_rules::Profile::Default, &uri)
            .expect("document settings should resolve");
        let document = lsp_types::TextDocumentItem::new(
            uri,
            "dockerfile".to_string(),
            1,
            "FROM alpine:latest\n".to_string(),
        );

        let diagnostics = linter
            .lint_open_document(&document)
            .expect("open document should lint");

        assert!(diagnostics.iter().all(
            |diagnostic| diagnostic.code != Some(NumberOrString::String("RDL3007".to_string()))
        ));
    }

    #[test]
    fn discovers_config_from_workspace_folders() {
        let temp = tempfile::TempDir::new().expect("temp dir should be created");
        std::fs::write(temp.path().join(".rudolint.yaml"), "ignore:\n  - RDL3007\n")
            .expect("config should be written");
        let workspace = lsp_types::WorkspaceFolder {
            uri: file_uri(temp.path()),
            name: "workspace".to_string(),
        };
        let dockerfile = temp.path().join("service").join("Dockerfile");
        let linter =
            DocumentLinter::discover_for_workspace(rudolint_rules::Profile::Default, &[workspace])
                .expect("workspace settings should resolve");
        let document = lsp_types::TextDocumentItem::new(
            file_uri(&dockerfile),
            "dockerfile".to_string(),
            1,
            "FROM alpine:latest\n".to_string(),
        );

        let diagnostics = linter
            .lint_open_document(&document)
            .expect("open document should lint");

        assert!(diagnostics.iter().all(
            |diagnostic| diagnostic.code != Some(NumberOrString::String("RDL3007".to_string()))
        ));
    }

    fn file_uri(path: &Path) -> lsp_types::Uri {
        let url = if path.is_dir() {
            url::Url::from_directory_path(path)
        } else {
            url::Url::from_file_path(path)
        }
        .expect("file URL should be built");

        lsp_types::Uri::from_str(url.as_str()).expect("uri should parse")
    }
}
