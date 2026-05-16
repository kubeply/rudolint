//! Autofix edit planning and patch generation.

use std::fmt::Write;

use rudolint_source::SourceSpan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub span: SourceSpan,
    pub kind: EditKind,
}

impl TextEdit {
    pub fn replace(span: SourceSpan, replacement: impl Into<String>) -> Self {
        Self {
            span,
            kind: EditKind::Replace {
                replacement: replacement.into(),
            },
        }
    }

    pub fn insert(line: usize, column: usize, content: impl Into<String>) -> Self {
        Self {
            span: SourceSpan {
                line,
                column,
                length: 0,
            },
            kind: EditKind::Insert {
                content: content.into(),
            },
        }
    }

    pub fn delete(span: SourceSpan) -> Self {
        Self {
            span,
            kind: EditKind::Delete,
        }
    }

    pub fn replacement_text(&self) -> &str {
        match &self.kind {
            EditKind::Replace { replacement } => replacement,
            EditKind::Insert { content } => content,
            EditKind::Delete => "",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditKind {
    Replace { replacement: String },
    Insert { content: String },
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixApplicability {
    Safe,
    Manual,
    NotAvailable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixPreview {
    pub title: String,
    pub applicability: FixApplicability,
    pub edits: Vec<TextEdit>,
}

impl FixPreview {
    pub fn render(&self) -> String {
        let mut output = String::new();
        let _ = writeln!(output, "title: {}", self.title);
        match &self.applicability {
            FixApplicability::Safe => {
                let _ = writeln!(output, "applicability: safe");
            }
            FixApplicability::Manual => {
                let _ = writeln!(output, "applicability: manual");
            }
            FixApplicability::NotAvailable { reason } => {
                let _ = writeln!(output, "applicability: not-available");
                let _ = writeln!(output, "reason: {reason}");
            }
        }
        let _ = writeln!(output, "edits:");
        for edit in &self.edits {
            match &edit.kind {
                EditKind::Replace { replacement } => {
                    let _ = writeln!(
                        output,
                        "- replace: line {}, column {}, length {}, with {:?}",
                        edit.span.line, edit.span.column, edit.span.length, replacement
                    );
                }
                EditKind::Insert { content } => {
                    let _ = writeln!(
                        output,
                        "- insert: line {}, column {}, content {:?}",
                        edit.span.line, edit.span.column, content
                    );
                }
                EditKind::Delete => {
                    let _ = writeln!(
                        output,
                        "- delete: line {}, column {}, length {}",
                        edit.span.line, edit.span.column, edit.span.length
                    );
                }
            }
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshots_safe_fix_preview() {
        let preview = FixPreview {
            title: "replace latest tag".to_string(),
            applicability: FixApplicability::Safe,
            edits: vec![TextEdit::replace(
                SourceSpan {
                    line: 1,
                    column: 13,
                    length: 6,
                },
                "3.20",
            )],
        };

        insta::assert_snapshot!("safe_fix_preview", preview.render());
    }

    #[test]
    fn snapshots_edit_primitives() {
        let preview = FixPreview {
            title: "edit primitive matrix".to_string(),
            applicability: FixApplicability::Manual,
            edits: vec![
                TextEdit::replace(
                    SourceSpan {
                        line: 1,
                        column: 6,
                        length: 6,
                    },
                    "3.20",
                ),
                TextEdit::insert(1, 1, "# syntax=docker/dockerfile:1\n"),
                TextEdit::delete(SourceSpan {
                    line: 4,
                    column: 1,
                    length: 12,
                }),
            ],
        };

        insta::assert_snapshot!("edit_primitives", preview.render());
    }

    #[test]
    fn snapshots_no_fix_rationale() {
        let preview = FixPreview {
            title: "secret-like build argument".to_string(),
            applicability: FixApplicability::NotAvailable {
                reason: "cannot infer the correct secret mount without build context".to_string(),
            },
            edits: Vec::new(),
        };

        insta::assert_snapshot!("no_fix_rationale", preview.render());
    }
}
