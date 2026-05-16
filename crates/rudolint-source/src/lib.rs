//! Source text, spans, comments, and edit ranges.

use std::path::PathBuf;

/// Source text plus cached line offset data for diagnostics and edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    /// Canonical path used for file reads and identity.
    pub path: PathBuf,
    /// Path rendered in diagnostics and user-facing output.
    pub display_path: PathBuf,
    /// Full source text.
    pub text: String,
    /// Byte-offset index for mapping spans to line and column positions.
    pub line_index: LineIndex,
}

impl SourceFile {
    /// Creates a source file using the same path for identity and display.
    pub fn new(path: impl Into<PathBuf>, text: impl Into<String>) -> Self {
        let path = path.into();
        let text = text.into();
        Self {
            display_path: path.clone(),
            line_index: LineIndex::new(&text),
            path,
            text,
        }
    }

    /// Overrides the path used in diagnostics and user-facing output.
    pub fn with_display_path(mut self, display_path: impl Into<PathBuf>) -> Self {
        self.display_path = display_path.into();
        self
    }

    /// Maps a byte range in this file to line and column positions.
    pub fn span(&self, start: usize, end: usize) -> SourceRange {
        self.line_index.range(start, end)
    }
}

/// Byte-offset index for mapping source offsets to line and column positions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineIndex {
    line_starts: Vec<usize>,
}

impl LineIndex {
    /// Builds a line index from UTF-8 source text.
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![0];
        for (index, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(index + 1);
            }
        }
        Self { line_starts }
    }

    /// Maps a byte offset to a one-based line and byte-column position.
    pub fn position(&self, byte: usize) -> SourcePosition {
        let line_index = match self.line_starts.binary_search(&byte) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        };
        let line_start = self.line_starts[line_index];
        SourcePosition {
            line: line_index + 1,
            column: byte.saturating_sub(line_start) + 1,
            byte,
        }
    }

    /// Maps a byte range to one-based start and end positions.
    pub fn range(&self, start: usize, end: usize) -> SourceRange {
        SourceRange {
            start: self.position(start),
            end: self.position(end),
        }
    }
}

/// One-based line and byte-column position for a source byte offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePosition {
    /// One-based line number.
    pub line: usize,
    /// One-based byte column within the line.
    pub column: usize,
    /// Zero-based byte offset in the source text.
    pub byte: usize,
}

/// Start and end positions for a source byte range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRange {
    /// Inclusive start position.
    pub start: SourcePosition,
    /// Exclusive end position.
    pub end: SourcePosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    pub line: usize,
    pub column: usize,
    pub length: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_byte_offsets_to_line_and_column() {
        let index = LineIndex::new("FROM alpine\nWORKDIR /app\n");

        assert_eq!(
            index.position(12),
            SourcePosition {
                line: 2,
                column: 1,
                byte: 12,
            }
        );
        assert_eq!(
            index.position(20),
            SourcePosition {
                line: 2,
                column: 9,
                byte: 20,
            }
        );
    }

    #[test]
    fn maps_ranges() {
        let file = SourceFile::new("Dockerfile", "FROM alpine\nWORKDIR /app\n");

        assert_eq!(
            file.span(12, 20),
            SourceRange {
                start: SourcePosition {
                    line: 2,
                    column: 1,
                    byte: 12,
                },
                end: SourcePosition {
                    line: 2,
                    column: 9,
                    byte: 20,
                },
            }
        );
    }
}
