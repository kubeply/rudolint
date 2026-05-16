//! Source text, spans, comments, and edit ranges.

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    pub path: PathBuf,
    pub display_path: PathBuf,
    pub text: String,
    pub line_index: LineIndex,
}

impl SourceFile {
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

    pub fn with_display_path(mut self, display_path: impl Into<PathBuf>) -> Self {
        self.display_path = display_path.into();
        self
    }

    pub fn span(&self, start: usize, end: usize) -> SourceRange {
        self.line_index.range(start, end)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineIndex {
    line_starts: Vec<usize>,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![0];
        for (index, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(index + 1);
            }
        }
        Self { line_starts }
    }

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

    pub fn range(&self, start: usize, end: usize) -> SourceRange {
        SourceRange {
            start: self.position(start),
            end: self.position(end),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePosition {
    pub line: usize,
    pub column: usize,
    pub byte: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRange {
    pub start: SourcePosition,
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
