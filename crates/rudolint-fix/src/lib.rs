//! Autofix edit planning and patch generation.

use std::fmt::Write;

use rudolint_source::SourceSpan;

/// A source edit that can be previewed or applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    /// Span affected by the edit, or insertion point for insertions.
    pub span: SourceSpan,
    /// Kind of edit to apply at `span`.
    pub kind: EditKind,
}

impl TextEdit {
    /// Creates a replacement edit for `span`.
    pub fn replace(span: SourceSpan, replacement: impl Into<String>) -> Self {
        Self {
            span,
            kind: EditKind::Replace {
                replacement: replacement.into(),
            },
        }
    }

    /// Creates an insertion edit at a 1-based `line` and `column`.
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

    /// Creates a deletion edit for `span`.
    pub fn delete(span: SourceSpan) -> Self {
        Self {
            span,
            kind: EditKind::Delete,
        }
    }

    /// Returns the text inserted by this edit, or an empty string for deletions.
    pub fn replacement_text(&self) -> &str {
        match &self.kind {
            EditKind::Replace { replacement } => replacement,
            EditKind::Insert { content } => content,
            EditKind::Delete => "",
        }
    }

    /// Returns the exclusive end column for the edited span.
    pub fn end_column(&self) -> usize {
        self.span.column + self.span.length
    }
}

/// The operation performed by a [`TextEdit`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditKind {
    /// Replace the associated span with `replacement`.
    Replace { replacement: String },
    /// Insert `content` at the edit span.
    Insert { content: String },
    /// Delete the associated span.
    Delete,
}

/// A pair of edits that cannot be applied together without ordering ambiguity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditConflict {
    /// First conflicting edit.
    pub first: TextEdit,
    /// Second conflicting edit.
    pub second: TextEdit,
}

/// Returns all conflicting edit pairs from `edits`.
pub fn detect_conflicts(edits: &[TextEdit]) -> Vec<EditConflict> {
    let mut conflicts = Vec::new();
    for (left_index, left) in edits.iter().enumerate() {
        for right in edits.iter().skip(left_index + 1) {
            if edits_conflict(left, right) {
                conflicts.push(EditConflict {
                    first: left.clone(),
                    second: right.clone(),
                });
            }
        }
    }
    conflicts
}

fn edits_conflict(left: &TextEdit, right: &TextEdit) -> bool {
    if left.span.line != right.span.line {
        return false;
    }

    let left_start = left.span.column;
    let left_end = left.end_column();
    let right_start = right.span.column;
    let right_end = right.end_column();

    match (left.span.length == 0, right.span.length == 0) {
        (true, true) => left_start == right_start,
        (true, false) => right_start <= left_start && left_start < right_end,
        (false, true) => left_start <= right_start && right_start < left_end,
        (false, false) => left_start < right_end && right_start < left_end,
    }
}

/// Describes whether a suggested [`FixPreview`] can be applied automatically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixApplicability {
    /// The edits can be applied automatically and reversed from source control.
    Safe,
    /// The suggestion requires human review or manual steps before applying.
    Manual,
    /// No edit can be applied; `reason` explains the missing information.
    NotAvailable { reason: String },
}

/// Human-facing preview of a suggested fix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixPreview {
    /// Short description of the fix.
    pub title: String,
    /// Applicability classification for the suggested [`TextEdit`] values.
    pub applicability: FixApplicability,
    /// Source edits that would be applied.
    pub edits: Vec<TextEdit>,
}

impl FixPreview {
    /// Renders a human-readable, YAML-like preview for debugging and tests.
    ///
    /// The output includes `title`, `applicability`, an optional `reason`, and
    /// `edits` with each `edit.span` and `edit.replacement`. Replacements are
    /// formatted as Rust debug strings. This is not a canonical machine format.
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
    fn detects_overlapping_edit_conflicts() {
        let edits = vec![
            TextEdit::replace(
                SourceSpan {
                    line: 1,
                    column: 5,
                    length: 4,
                },
                "FROM",
            ),
            TextEdit::delete(SourceSpan {
                line: 1,
                column: 7,
                length: 3,
            }),
            TextEdit::insert(2, 1, "# syntax=docker/dockerfile:1\n"),
        ];

        let conflicts = detect_conflicts(&edits);

        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].first, edits[0]);
        assert_eq!(conflicts[0].second, edits[1]);
    }

    #[test]
    fn allows_adjacent_edits_and_detects_same_position_inserts() {
        let adjacent = vec![
            TextEdit::replace(
                SourceSpan {
                    line: 1,
                    column: 1,
                    length: 4,
                },
                "COPY",
            ),
            TextEdit::delete(SourceSpan {
                line: 1,
                column: 5,
                length: 3,
            }),
        ];
        assert!(detect_conflicts(&adjacent).is_empty());

        let insert_at_span_start = vec![
            TextEdit::insert(1, 1, "# prefix\n"),
            TextEdit::replace(
                SourceSpan {
                    line: 1,
                    column: 1,
                    length: 4,
                },
                "COPY",
            ),
        ];
        assert_eq!(detect_conflicts(&insert_at_span_start).len(), 1);

        let inserts = vec![
            TextEdit::insert(1, 1, "# first\n"),
            TextEdit::insert(1, 1, "# second\n"),
        ];
        assert_eq!(detect_conflicts(&inserts).len(), 1);
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
