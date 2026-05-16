use std::fmt;
use std::path::{Path, PathBuf};

use clap::ValueEnum;
use rudolint_source::Span;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Ignore,
    Style,
    Info,
    Warning,
    Error,
}

impl Severity {
    pub fn is_failure(self, threshold: Severity) -> bool {
        threshold != Severity::Ignore && self >= threshold
    }

    pub fn sarif_level(self) -> &'static str {
        match self {
            Severity::Ignore | Severity::Style | Severity::Info => "note",
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Severity::Ignore => "ignore",
            Severity::Style => "style",
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub path: PathBuf,
    pub primary_span: Span,
    pub labels: Vec<DiagnosticLabel>,
    pub help: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticLabel {
    pub span: Span,
    pub message: String,
}

impl Finding {
    pub fn new(
        code: impl Into<String>,
        severity: Severity,
        message: impl Into<String>,
        line: usize,
        column: usize,
    ) -> Self {
        Self::with_span(code, severity, message, Span::point(line, column))
    }

    pub fn with_span(
        code: impl Into<String>,
        severity: Severity,
        message: impl Into<String>,
        primary_span: Span,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            message: message.into(),
            path: PathBuf::new(),
            primary_span,
            labels: Vec::new(),
            help: None,
        }
    }

    pub fn with_path(mut self, path: &Path) -> Self {
        self.path = path.to_path_buf();
        self
    }

    pub fn line(&self) -> usize {
        self.primary_span.start_line
    }

    pub fn column(&self) -> usize {
        self.primary_span.start_column
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn with_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(DiagnosticLabel {
            span,
            message: message.into(),
        });
        self
    }
}
