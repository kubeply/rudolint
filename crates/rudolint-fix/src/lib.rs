//! Autofix edit planning and patch generation.

use std::fmt::Write;

use rudolint_source::SourceSpan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    pub span: SourceSpan,
    pub replacement: String,
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
    pub edits: Vec<Fix>,
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
            let _ = writeln!(
                output,
                "- line: {}, column: {}, length: {}, replacement: {:?}",
                edit.span.line, edit.span.column, edit.span.length, edit.replacement
            );
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
            edits: vec![Fix {
                span: SourceSpan {
                    line: 1,
                    column: 13,
                    length: 6,
                },
                replacement: "3.20".to_string(),
            }],
        };

        insta::assert_snapshot!("safe_fix_preview", preview.render());
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
