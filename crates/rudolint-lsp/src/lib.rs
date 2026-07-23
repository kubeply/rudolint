//! Language server integration points.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[cfg(test)]
use anyhow::bail;
use anyhow::{Context, Result};
use lsp_types::{
    CodeAction, CodeActionKind, Diagnostic, DiagnosticSeverity, DiagnosticTag, Hover,
    HoverContents, MarkupContent, MarkupKind, NumberOrString, Position, Range,
    TextEdit as LspTextEdit, WorkspaceEdit,
};
use rudolint_diagnostics::{Finding, Severity};
use rudolint_dockerfile::parse_dockerfile;
use rudolint_fix::{FixPreview, TextEdit as FixTextEdit, detect_conflicts};
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
    fn new(profile: Profile, settings: Settings) -> Self {
        Self { profile, settings }
    }

    /// Creates a document linter by discovering configuration from a document [`lsp_types::Uri`].
    pub fn discover_for_document(profile: Profile, uri: &lsp_types::Uri) -> Result<Self> {
        let search_starts = search_start_for_uri(uri).into_iter().collect::<Vec<_>>();
        let settings = rudolint_settings::resolve(
            &SettingsOptions::default().with_search_starts(search_starts),
        )?;
        Ok(Self::new(profile, settings))
    }

    /// Creates a document linter by discovering configuration from LSP [`lsp_types::WorkspaceFolder`] values.
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
    #[cfg(test)]
    fn lint_changed_document(
        &self,
        params: &lsp_types::DidChangeTextDocumentParams,
    ) -> Result<Vec<Diagnostic>> {
        let Some(text) = full_document_text(params)? else {
            return Ok(Vec::new());
        };

        self.lint_uri(&params.text_document.uri, text)
    }

    /// Builds hover content explaining a `rudolint` rule code.
    pub fn hover_for_rule(&self, code: &str) -> Option<Hover> {
        let engine = RuleEngine::new(self.profile, self.settings.config.clone());
        let rule = engine
            .catalog()
            .into_iter()
            .find(|rule| rule.code.eq_ignore_ascii_case(code))?;

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: rule_hover_markdown(&rule),
            }),
            range: None,
        })
    }

    /// Builds quick-fix code actions from safe `rudolint` fix previews.
    pub fn code_actions_for_document(
        &self,
        document: &lsp_types::TextDocumentItem,
    ) -> Result<Vec<CodeAction>> {
        let display_path = document_path(&document.uri);
        let lint_path = lint_path_for_config(&self.settings.config_path, &display_path);
        let parsed = parse_dockerfile(&document.text)
            .with_context(|| format!("failed to parse {}", document.uri.as_str()))?;
        let engine = RuleEngine::new(self.profile, self.settings.config.clone());

        Ok(engine
            .fixes_path(&lint_path, &parsed)
            .iter()
            .filter_map(|fix| code_action(&document.uri, fix))
            .collect())
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
pub(crate) fn diagnostic(finding: &Finding) -> Diagnostic {
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
pub(crate) fn diagnostics(findings: &[Finding]) -> Vec<Diagnostic> {
    findings.iter().map(diagnostic).collect()
}

/// Converts a `rudolint` source [`Span`] into an LSP [`Range`].
pub(crate) fn range(span: Span) -> Range {
    Range {
        start: position(span.start_line, span.start_column),
        end: position(span.end_line, span.end_column),
    }
}

/// Converts a `rudolint` one-based line and column pair into an LSP
/// zero-based [`Position`].
pub(crate) fn position(line: usize, column: usize) -> Position {
    Position {
        line: zero_based(line),
        character: zero_based(column),
    }
}

/// Converts a `rudolint` [`Severity`] into LSP [`DiagnosticSeverity`].
fn severity(severity: Severity) -> DiagnosticSeverity {
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

#[cfg(test)]
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

fn rule_hover_markdown(rule: &rudolint_rules::RuleInfo) -> String {
    format!(
        "**{}** `{}`\n\n{}\n\nSeverity: `{}`\n\nStatus: `{}`\n\nCategory: `{}`\n\nAutofix: `{}`\n\n[Rule documentation]({})",
        rule.code,
        rule.metadata.name,
        rule.summary,
        rule.severity,
        rule.status,
        rule.metadata.category.as_str(),
        rule.metadata.fix.as_str(),
        rule.metadata.docs_url
    )
}

/// Converts a safe, non-conflicting fix preview into an LSP quick-fix action.
fn code_action(uri: &lsp_types::Uri, fix: &FixPreview) -> Option<CodeAction> {
    if !fix.applicability.is_automatically_applicable()
        || fix.edits.is_empty()
        || !detect_conflicts(&fix.edits).is_empty()
    {
        return None;
    }

    let edits = fix.edits.iter().map(lsp_text_edit).collect::<Vec<_>>();
    Some(CodeAction {
        title: fix.title.clone(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: None,
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(uri.clone(), edits)])),
            document_changes: None,
            change_annotations: None,
        }),
        command: None,
        is_preferred: Some(true),
        disabled: None,
        data: None,
    })
}

/// Converts a `rudolint-fix` source edit into an LSP text edit.
fn lsp_text_edit(edit: &FixTextEdit) -> LspTextEdit {
    LspTextEdit::new(
        Range::new(
            position(edit.span.line, edit.span.column),
            position(edit.span.line, edit.end_column()),
        ),
        edit.replacement_text().to_string(),
    )
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
            "DL3007",
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
            Some(NumberOrString::String("DL3007".to_string()))
        );
        assert_eq!(diagnostic.source.as_deref(), Some("rudolint"));
        assert_eq!(diagnostic.message, "Avoid latest tag\n\nPin the image tag.");
    }

    #[test]
    fn converts_many_findings_without_reordering() {
        let first = Finding::new("DL3000", Severity::Error, "Use absolute WORKDIR", 1, 1);
        let second = Finding::new("DL4000", Severity::Info, "MAINTAINER is deprecated", 3, 1);

        let diagnostics = diagnostics(&[first, second]);

        assert_eq!(diagnostics.len(), 2);
        assert_eq!(
            diagnostics[0].code,
            Some(NumberOrString::String("DL3000".to_string()))
        );
        assert_eq!(
            diagnostics[1].code,
            Some(NumberOrString::String("DL4000".to_string()))
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
            "DL3007",
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
            |diagnostic| diagnostic.code == Some(NumberOrString::String("DL3007".to_string()))
        ));
    }

    /// Open-document diagnostics preserve source spans as zero-based LSP ranges.
    #[test]
    fn open_document_diagnostics_use_source_ranges() {
        let linter = DocumentLinter::default();
        let document = lsp_types::TextDocumentItem::new(
            lsp_types::Uri::from_str("file:///workspace/Dockerfile").expect("uri should parse"),
            "dockerfile".to_string(),
            1,
            "FROM alpine:latest\nMAINTAINER ops@example.com\n".to_string(),
        );

        let diagnostics = linter
            .lint_open_document(&document)
            .expect("open document should lint");
        let latest = diagnostic_by_code(&diagnostics, "DL3007");
        let maintainer = diagnostic_by_code(&diagnostics, "DL4000");

        assert_eq!(
            latest.range,
            lsp_types::Range::new(
                lsp_types::Position::new(0, 0),
                lsp_types::Position::new(0, 18)
            )
        );
        assert_eq!(
            maintainer.range,
            lsp_types::Range::new(
                lsp_types::Position::new(1, 0),
                lsp_types::Position::new(1, 26)
            )
        );
    }

    #[test]
    fn open_document_linting_uses_resolved_settings() {
        let linter = DocumentLinter::new(
            rudolint_rules::Profile::Default,
            Settings {
                config: Config {
                    ignore: BTreeSet::from(["DL3007".to_string()]),
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
            |diagnostic| diagnostic.code != Some(NumberOrString::String("DL3007".to_string()))
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
            |diagnostic| diagnostic.code == Some(NumberOrString::String("DL3007".to_string()))
        ));
    }

    /// Full-change diagnostics preserve source spans after leading document lines.
    #[test]
    fn changed_document_diagnostics_use_source_ranges() {
        let linter = DocumentLinter::default();
        let params = lsp_types::DidChangeTextDocumentParams {
            text_document: lsp_types::VersionedTextDocumentIdentifier::new(
                lsp_types::Uri::from_str("file:///workspace/Dockerfile").expect("uri should parse"),
                2,
            ),
            content_changes: vec![lsp_types::TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "# comment\nFROM alpine:latest\n".to_string(),
            }],
        };

        let diagnostics = linter
            .lint_changed_document(&params)
            .expect("full document change should lint");
        let latest = diagnostic_by_code(&diagnostics, "DL3007");

        assert_eq!(
            latest.range,
            lsp_types::Range::new(
                lsp_types::Position::new(1, 0),
                lsp_types::Position::new(1, 18)
            )
        );
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
        std::fs::write(temp.path().join(".rudolint.yaml"), "ignore:\n  - DL3007\n")
            .expect("config should be written");
        let dockerfile = temp.path().join("Dockerfile");
        std::fs::write(&dockerfile, "FROM alpine:latest\n").expect("Dockerfile should be written");
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
            |diagnostic| diagnostic.code != Some(NumberOrString::String("DL3007".to_string()))
        ));
    }

    #[test]
    fn discovers_config_from_workspace_folders() {
        let temp = tempfile::TempDir::new().expect("temp dir should be created");
        std::fs::write(temp.path().join(".rudolint.yaml"), "ignore:\n  - DL3007\n")
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
            |diagnostic| diagnostic.code != Some(NumberOrString::String("DL3007".to_string()))
        ));
    }

    #[test]
    fn builds_rule_explanation_hover() {
        let linter = DocumentLinter::default();

        let hover = linter
            .hover_for_rule("dl3007")
            .expect("rule hover should be built");

        let lsp_types::HoverContents::Markup(content) = hover.contents else {
            panic!("hover should use markup content");
        };
        assert_eq!(content.kind, lsp_types::MarkupKind::Markdown);
        assert!(content.value.contains("**DL3007**"));
        assert!(content.value.contains("Severity: `warning`"));
        assert!(content.value.contains("Rule documentation"));
    }

    #[test]
    fn skips_unknown_rule_hover() {
        let linter = DocumentLinter::default();

        assert!(linter.hover_for_rule("DL9999").is_none());
    }

    /// Safe rule fixes are surfaced as LSP quick-fix workspace edits.
    #[test]
    fn builds_code_actions_for_safe_fixes() {
        let linter = DocumentLinter::default();
        let document = lsp_types::TextDocumentItem::new(
            lsp_types::Uri::from_str("file:///workspace/Dockerfile").expect("uri should parse"),
            "dockerfile".to_string(),
            1,
            "RUN --mount=type=cache,target=/var/cache/apt apt-get update\n".to_string(),
        );

        let actions = linter
            .code_actions_for_document(&document)
            .expect("code actions should be built");

        let action = actions
            .iter()
            .find(|action| action.title == "insert BuildKit syntax directive")
            .expect("safe BuildKit syntax fix should be exposed");
        assert_eq!(action.kind, Some(lsp_types::CodeActionKind::QUICKFIX));
        assert_eq!(action.is_preferred, Some(true));

        let edit = action.edit.as_ref().expect("action should include edit");
        let edit_json = serde_json::to_value(edit).expect("workspace edit should serialize");
        let text_edits = &edit_json["changes"][document.uri.as_str()];
        assert_eq!(
            text_edits
                .as_array()
                .expect("edit should be an array")
                .len(),
            1
        );
        assert_eq!(
            text_edits[0]["range"],
            serde_json::json!({
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 0 }
            })
        );
        assert_eq!(
            text_edits[0]["newText"],
            serde_json::json!("# syntax=docker/dockerfile:1\n")
        );
    }

    /// Manual fix previews are intentionally not exposed as code actions.
    #[test]
    fn skips_manual_code_actions() {
        let linter = DocumentLinter::default();
        let document = lsp_types::TextDocumentItem::new(
            lsp_types::Uri::from_str("file:///workspace/Dockerfile").expect("uri should parse"),
            "dockerfile".to_string(),
            1,
            "FROM alpine:3.20\nCMD echo ok\n".to_string(),
        );

        let actions = linter
            .code_actions_for_document(&document)
            .expect("code actions should be built");

        assert!(
            actions
                .iter()
                .all(|action| !action.title.contains("convert CMD"))
        );
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

    /// Returns the first diagnostic with a matching rule code.
    fn diagnostic_by_code<'a>(
        diagnostics: &'a [lsp_types::Diagnostic],
        code: &str,
    ) -> &'a lsp_types::Diagnostic {
        diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == Some(NumberOrString::String(code.to_string())))
            .unwrap_or_else(|| panic!("{code} diagnostic should be emitted"))
    }
}
