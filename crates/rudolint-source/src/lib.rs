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

    pub fn span(&self, start: usize, end: usize) -> Span {
        self.line_index.span(start, end)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineIndex {
    line_starts: Vec<usize>,
    text: String,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![0];
        for (index, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(index + 1);
            }
        }
        Self {
            line_starts,
            text: text.to_string(),
        }
    }

    pub fn position(&self, byte: usize) -> SourcePosition {
        let line_index = match self.line_starts.binary_search(&byte) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        };
        let line_start = self.line_starts[line_index];
        SourcePosition {
            line: line_index + 1,
            column: self.column(line_start, byte),
            byte,
        }
    }

    pub fn range(&self, start: usize, end: usize) -> SourceRange {
        SourceRange {
            start: self.position(start),
            end: self.position(end),
        }
    }

    pub fn span(&self, start: usize, end: usize) -> Span {
        let range = self.range(start, end);
        Span {
            start_byte: start,
            end_byte: end,
            start_line: range.start.line,
            start_column: range.start.column,
            end_line: range.end.line,
            end_column: range.end.column,
        }
    }

    fn column(&self, line_start: usize, byte: usize) -> usize {
        self.text[line_start..byte]
            .chars()
            .filter(|character| *character != '\r')
            .count()
            + 1
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
pub struct Span {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
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
            Span {
                start_byte: 12,
                end_byte: 20,
                start_line: 2,
                start_column: 1,
                end_line: 2,
                end_column: 9,
            }
        );
    }

    #[test]
    fn counts_utf8_columns_by_scalar_value() {
        let file = SourceFile::new("Dockerfile", "LABEL name=\"cafe\"\nLABEL name=\"cafe\"\n");
        let source = file.text.replacen("cafe", "café", 1);
        let file = SourceFile::new("Dockerfile", source);
        let start = file
            .text
            .find("café")
            .expect("fixture contains label value");
        let end = start + "café".len();

        assert_eq!(
            file.span(start, end),
            Span {
                start_byte: 12,
                end_byte: 17,
                start_line: 1,
                start_column: 13,
                end_line: 1,
                end_column: 17,
            }
        );
    }

    #[test]
    fn maps_crlf_line_starts() {
        let file = SourceFile::new("Dockerfile", "FROM alpine\r\nRUN echo hi\r\n");
        let start = file.text.find("RUN").expect("fixture contains run");

        assert_eq!(
            file.span(start, start + 3),
            Span {
                start_byte: 13,
                end_byte: 16,
                start_line: 2,
                start_column: 1,
                end_line: 2,
                end_column: 4,
            }
        );
    }

    #[test]
    fn maps_trailing_newline_and_no_trailing_newline_inputs() {
        let with_newline = SourceFile::new("Dockerfile", "FROM alpine\n");
        let without_newline = SourceFile::new("Dockerfile", "FROM alpine");

        assert_eq!(with_newline.line_index.position(12).line, 2);
        assert_eq!(with_newline.line_index.position(12).column, 1);
        assert_eq!(without_newline.line_index.position(11).line, 1);
        assert_eq!(without_newline.line_index.position(11).column, 12);
    }
}
