use std::fmt;

use regex::Regex;
use rudolint_source::{SourceFile, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dockerfile {
    pub syntax: Option<SyntaxDirective>,
    /// Dockerfile `# escape=` parser directive, when present.
    pub escape: Option<EscapeDirective>,
    /// Dockerfile `# check=` parser directives, in source order.
    pub checks: Vec<CheckDirective>,
    /// Dockerfile comments, in source order.
    pub comments: Vec<Comment>,
    pub instructions: Vec<Instruction>,
    pub has_buildkit_features: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxDirective {
    pub image: String,
    pub line: usize,
}

/// Dockerfile `# escape=` parser directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EscapeDirective {
    /// Escape character selected by the directive.
    pub character: char,
    /// One-based source line where the directive appears.
    pub line: usize,
}

/// Dockerfile `# check=` parser directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckDirective {
    /// Raw check directive value after `=`.
    pub value: String,
    /// One-based source line where the directive appears.
    pub line: usize,
}

/// Dockerfile comment with its source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    /// Trimmed comment text, including the leading `#`.
    pub text: String,
    /// One-based source line where the comment appears.
    pub line: usize,
    /// Source span covering the original comment line.
    pub span: Span,
}

/// Line-continuation escape marker within an instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineContinuation {
    /// One-based source line containing the continuation marker.
    pub line: usize,
    /// Escape character used for the continuation.
    pub escape: char,
    /// Source span covering the continuation escape marker.
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    /// Uppercase Dockerfile instruction keyword.
    pub keyword: String,
    /// Source span covering the instruction keyword.
    pub keyword_span: Span,
    /// Trimmed instruction arguments.
    pub args: String,
    /// Source span covering the instruction arguments, when present.
    pub args_span: Option<Span>,
    /// Parsed instruction argument form.
    pub form: InstructionForm,
    /// Line-continuation markers used by this instruction.
    pub continuations: Vec<LineContinuation>,
    pub flags: Vec<(String, String)>,
    pub mounts: Vec<Mount>,
    pub heredocs: Vec<Heredoc>,
    pub from: Option<FromInstruction>,
    pub run: Option<RunInstruction>,
    pub copy: Option<CopyInstruction>,
    pub line: usize,
    /// Source span covering the raw instruction text.
    pub raw_span: Span,
    pub raw: String,
}

/// Dockerfile instruction argument form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstructionForm {
    /// Instruction has no arguments.
    Empty,
    /// Instruction uses JSON exec form.
    Json(Vec<String>),
    /// Instruction starts like JSON exec form but could not be parsed.
    InvalidJson {
        /// Raw argument string.
        raw: String,
        /// JSON parser error message.
        error: String,
    },
    /// Instruction uses shell form.
    Shell {
        /// Shell-form argument text.
        text: String,
        /// Source span covering the shell-form argument text.
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mount {
    pub mount_type: String,
    pub options: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heredoc {
    pub delimiter: String,
    pub quoted: bool,
    pub target_instruction: String,
    pub body: String,
    pub body_span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FromInstruction {
    pub image: String,
    pub digest: Option<String>,
    pub alias: Option<String>,
    pub platform: Option<String>,
    pub flags: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunInstruction {
    pub flags: Vec<(String, String)>,
    pub mounts: Vec<Mount>,
    pub network: Option<String>,
    pub security: Option<String>,
    pub shell: Option<ShellBody>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellBody {
    pub text: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyInstruction {
    pub kind: CopyKind,
    pub flags: Vec<(String, String)>,
    pub from: Option<String>,
    pub chown: Option<String>,
    pub chmod: Option<String>,
    pub sources: Vec<String>,
    pub destination: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyKind {
    Copy,
    Add,
}

#[derive(Debug, Clone)]
pub struct ParserError {
    message: String,
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ParserError {}

pub fn parse_dockerfile(source: &str) -> Result<Dockerfile, ParserError> {
    let source_file = SourceFile::new("Dockerfile", source);
    let mut syntax = None;
    let mut escape = None;
    let mut escape_character = '\\';
    let mut checks = Vec::new();
    let mut comments = Vec::new();
    let mut instructions = Vec::new();
    let mut current = String::new();
    let mut start_line = 0;
    let mut start_byte = 0;
    let mut start_escape = '\\';
    let mut byte_offset = 0;

    let segments = source.split_inclusive('\n').collect::<Vec<_>>();
    let mut index = 0;
    while index < segments.len() {
        let segment = segments[index];
        let line_number = index + 1;
        let line = dockerfile_line(segment);
        let trimmed = line.trim();

        if current.is_empty() {
            if trimmed.is_empty() {
                byte_offset += segment.len();
                index += 1;
                continue;
            }
            if trimmed.starts_with('#') {
                comments.push(Comment {
                    text: trimmed.to_string(),
                    line: line_number,
                    span: source_file.span(byte_offset, byte_offset + line.len()),
                });
                if let Some(value) = directive_value(trimmed, "syntax") {
                    syntax = Some(SyntaxDirective {
                        image: value.trim().to_string(),
                        line: line_number,
                    });
                } else if let Some(value) = directive_value(trimmed, "escape") {
                    if let Some(character) = value.trim().chars().next() {
                        escape_character = character;
                        escape = Some(EscapeDirective {
                            character,
                            line: line_number,
                        });
                    }
                } else if let Some(value) = directive_value(trimmed, "check") {
                    checks.push(CheckDirective {
                        value: value.trim().to_string(),
                        line: line_number,
                    });
                }
                byte_offset += segment.len();
                index += 1;
                continue;
            }
            start_line = line_number;
            start_byte = byte_offset;
            start_escape = escape_character;
        }

        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);

        if continues(line, start_escape) {
            byte_offset += segment.len();
            index += 1;
            continue;
        }

        let heredoc_delimiters = heredoc_delimiters(&current)?;
        if !heredoc_delimiters.is_empty() {
            byte_offset += segment.len();
            index += 1;
            for delimiter in heredoc_delimiters {
                while index < segments.len() {
                    let body_segment = segments[index];
                    let body_line = dockerfile_line(body_segment);
                    if !current.is_empty() {
                        current.push('\n');
                    }
                    current.push_str(body_line);
                    byte_offset += body_segment.len();
                    index += 1;
                    if body_line.trim() == delimiter {
                        break;
                    }
                }
            }

            if let Some(instruction) =
                parse_instruction(&current, start_line, start_byte, &source_file, start_escape)?
            {
                instructions.push(instruction);
            }
            current.clear();
            continue;
        }

        if let Some(instruction) =
            parse_instruction(&current, start_line, start_byte, &source_file, start_escape)?
        {
            instructions.push(instruction);
        }
        current.clear();
        byte_offset += segment.len();
        index += 1;
    }

    if !current.trim().is_empty()
        && let Some(instruction) =
            parse_instruction(&current, start_line, start_byte, &source_file, start_escape)?
    {
        instructions.push(instruction);
    }

    let has_buildkit_features = instructions.iter().any(Instruction::has_buildkit_features);
    Ok(Dockerfile {
        syntax,
        escape,
        checks,
        comments,
        instructions,
        has_buildkit_features,
    })
}

fn dockerfile_line(segment: &str) -> &str {
    segment
        .strip_suffix('\n')
        .unwrap_or(segment)
        .strip_suffix('\r')
        .unwrap_or_else(|| segment.strip_suffix('\n').unwrap_or(segment))
}

fn directive_value<'a>(comment: &'a str, name: &str) -> Option<&'a str> {
    let rest = comment.strip_prefix('#')?.trim_start();
    let (directive_name, value) = rest.split_once('=')?;
    directive_name
        .eq_ignore_ascii_case(name)
        .then_some(value.trim())
}

fn continues(line: &str, escape_character: char) -> bool {
    let trimmed = line.trim_end();
    trimmed.ends_with(escape_character) && !ends_with_escaped_escape(trimmed, escape_character)
}

fn ends_with_escaped_escape(trimmed: &str, escape_character: char) -> bool {
    let mut count = 0;
    for character in trimmed.chars().rev() {
        if character == escape_character {
            count += 1;
        } else {
            break;
        }
    }
    count > 1 && count % 2 == 0
}

fn parse_instruction(
    raw: &str,
    line: usize,
    start_byte: usize,
    source_file: &SourceFile,
    escape_character: char,
) -> Result<Option<Instruction>, ParserError> {
    let trimmed = raw.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Ok(None);
    }
    let leading_whitespace = raw.len() - trimmed.len();
    let keyword_start = start_byte + leading_whitespace;
    let raw_span = source_file.span(start_byte, start_byte + raw.len());

    let Some(keyword_width) = trimmed.find(char::is_whitespace) else {
        let keyword_span = source_file.span(keyword_start, keyword_start + trimmed.len());
        return Ok(Some(Instruction {
            keyword: trimmed.to_ascii_uppercase(),
            keyword_span,
            args: String::new(),
            args_span: None,
            form: InstructionForm::Empty,
            continuations: parse_continuations(
                raw,
                line,
                start_byte,
                source_file,
                escape_character,
            ),
            flags: Vec::new(),
            mounts: Vec::new(),
            heredocs: Vec::new(),
            from: None,
            run: None,
            copy: None,
            line,
            raw_span,
            raw: raw.to_string(),
        }));
    };

    let keyword = &trimmed[..keyword_width];
    let rest = &trimmed[keyword_width..];
    let keyword = keyword.to_ascii_uppercase();
    let keyword_span = source_file.span(keyword_start, keyword_start + keyword_width);
    let rest_leading_whitespace = rest.len() - rest.trim_start().len();
    let args_start = keyword_start + keyword_width + rest_leading_whitespace;
    let args = rest.trim().to_string();
    let args_span =
        (!args.is_empty()).then(|| source_file.span(args_start, args_start + args.len()));
    let form = parse_instruction_form(&args, args_span);
    let continuations = parse_continuations(raw, line, start_byte, source_file, escape_character);
    let flags = parse_flags(&args);
    let from = (keyword == "FROM")
        .then(|| parse_from(&args, &flags))
        .flatten();
    let mounts = flags
        .iter()
        .filter(|(name, _)| name == "mount")
        .filter_map(|(_, value)| parse_mount(value))
        .collect::<Vec<_>>();
    let run =
        (keyword == "RUN").then(|| parse_run(&args, args_start, &flags, &mounts, source_file));
    let copy =
        matches!(keyword.as_str(), "COPY" | "ADD").then(|| parse_copy(&keyword, &args, &flags));
    let heredocs = parse_heredocs(raw, start_byte, &keyword, source_file)?;

    Ok(Some(Instruction {
        keyword,
        keyword_span,
        args,
        args_span,
        form,
        continuations,
        flags,
        mounts,
        heredocs,
        from,
        run,
        copy,
        line,
        raw_span,
        raw: raw.to_string(),
    }))
}

fn parse_run(
    args: &str,
    args_start: usize,
    flags: &[(String, String)],
    mounts: &[Mount],
    source_file: &SourceFile,
) -> RunInstruction {
    let network = flags
        .iter()
        .find(|(name, _)| name == "network")
        .map(|(_, value)| value.clone());
    let security = flags
        .iter()
        .find(|(name, _)| name == "security")
        .map(|(_, value)| value.clone());
    let (shell_text, shell_start) = strip_leading_flags(args, args_start);
    let shell = (!shell_text.is_empty()).then(|| ShellBody {
        text: shell_text.to_string(),
        span: source_file.span(shell_start, shell_start + shell_text.len()),
    });

    RunInstruction {
        flags: flags.to_vec(),
        mounts: mounts.to_vec(),
        network,
        security,
        shell,
    }
}

fn strip_leading_flags(args: &str, args_start: usize) -> (&str, usize) {
    let mut remaining = args;
    let mut offset = 0;
    loop {
        let trimmed = remaining.trim_start();
        offset += remaining.len() - trimmed.len();
        remaining = trimmed;
        if !remaining.starts_with("--") {
            return (remaining, args_start + offset);
        }
        let Some(width) = remaining.find(char::is_whitespace) else {
            return ("", args_start + args.len());
        };
        offset += width;
        remaining = &remaining[width..];
    }
}

fn parse_copy(keyword: &str, args: &str, flags: &[(String, String)]) -> CopyInstruction {
    let from = flags
        .iter()
        .find(|(name, _)| name == "from")
        .map(|(_, value)| value.clone());
    let chown = flags
        .iter()
        .find(|(name, _)| name == "chown")
        .map(|(_, value)| value.clone());
    let chmod = flags
        .iter()
        .find(|(name, _)| name == "chmod")
        .map(|(_, value)| value.clone());
    let (operand_text, _) = strip_leading_flags(args, 0);
    let operands = if operand_text.trim_start().starts_with('[') {
        serde_json::from_str::<Vec<String>>(operand_text).unwrap_or_default()
    } else {
        operand_text
            .split_whitespace()
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    let destination = operands.last().cloned();
    let sources = operands[..operands.len().saturating_sub(1)].to_vec();

    CopyInstruction {
        kind: if keyword == "COPY" {
            CopyKind::Copy
        } else {
            CopyKind::Add
        },
        flags: flags.to_vec(),
        from,
        chown,
        chmod,
        sources,
        destination,
    }
}

fn parse_from(args: &str, flags: &[(String, String)]) -> Option<FromInstruction> {
    let parts = args.split_whitespace().collect::<Vec<_>>();
    let image = parts.iter().find(|part| !part.starts_with("--"))?;
    let (image, digest) = image
        .split_once('@')
        .map_or((*image, None), |(name, digest)| (name, Some(digest)));
    let alias = parts
        .windows(2)
        .find(|window| window[0].eq_ignore_ascii_case("AS"))
        .map(|window| window[1].to_string());
    let platform = flags
        .iter()
        .find(|(name, _)| name == "platform")
        .map(|(_, value)| value.clone());

    Some(FromInstruction {
        image: image.to_string(),
        digest: digest.map(str::to_string),
        alias,
        platform,
        flags: flags.to_vec(),
    })
}

fn parse_instruction_form(args: &str, args_span: Option<Span>) -> InstructionForm {
    if args.is_empty() {
        return InstructionForm::Empty;
    }

    let Some(span) = args_span else {
        return InstructionForm::Empty;
    };

    if args.starts_with('[') {
        return match serde_json::from_str::<Vec<String>>(args) {
            Ok(values) => InstructionForm::Json(values),
            Err(error) => InstructionForm::InvalidJson {
                raw: args.to_string(),
                error: error.to_string(),
            },
        };
    }

    InstructionForm::Shell {
        text: args.to_string(),
        span,
    }
}

fn parse_flags(args: &str) -> Vec<(String, String)> {
    let mut flags = Vec::new();
    for token in args.split_whitespace() {
        let Some(flag) = token.strip_prefix("--") else {
            break;
        };
        let (name, value) = flag.split_once('=').unwrap_or((flag, "true"));
        flags.push((name.to_string(), value.trim_matches('"').to_string()));
    }
    flags
}

fn parse_continuations(
    raw: &str,
    start_line: usize,
    start_byte: usize,
    source_file: &SourceFile,
    escape_character: char,
) -> Vec<LineContinuation> {
    let mut continuations = Vec::new();
    let mut line_start_byte = start_byte;
    for (line_index, line) in raw.split('\n').enumerate() {
        if continues(line, escape_character) {
            let continuation_byte =
                line_start_byte + line.trim_end().len() - escape_character.len_utf8();
            continuations.push(LineContinuation {
                line: start_line + line_index,
                escape: escape_character,
                span: source_file.span(
                    continuation_byte,
                    continuation_byte + escape_character.len_utf8(),
                ),
            });
        }
        line_start_byte += line.len() + 1;
    }
    continuations
}

fn parse_mount(value: &str) -> Option<Mount> {
    let mut mount_type = String::new();
    let mut options = Vec::new();
    for part in value.split(',') {
        let (key, value) = part.split_once('=').unwrap_or((part, "true"));
        if key == "type" {
            mount_type = value.to_string();
        }
        options.push((key.to_string(), value.to_string()));
    }
    if mount_type.is_empty() {
        return None;
    }
    Some(Mount {
        mount_type,
        options,
    })
}

fn heredoc_delimiters(raw: &str) -> Result<Vec<String>, ParserError> {
    let re = Regex::new(r#"<<-?\s*['"]?([A-Za-z_][A-Za-z0-9_-]*)['"]?"#).map_err(|error| {
        ParserError {
            message: error.to_string(),
        }
    })?;
    Ok(re
        .captures_iter(raw)
        .filter_map(|captures| captures.get(1).map(|m| m.as_str().to_string()))
        .collect())
}

fn parse_heredocs(
    raw: &str,
    start_byte: usize,
    target_instruction: &str,
    source_file: &SourceFile,
) -> Result<Vec<Heredoc>, ParserError> {
    let re =
        Regex::new(r#"(?P<prefix><<-?\s*)(?P<quote>['"]?)(?P<delimiter>[A-Za-z_][A-Za-z0-9_-]*)"#)
            .map_err(|error| ParserError {
                message: error.to_string(),
            })?;

    let mut heredocs = Vec::new();
    for captures in re.captures_iter(raw) {
        let Some(delimiter_match) = captures.name("delimiter") else {
            continue;
        };
        let delimiter = delimiter_match.as_str();
        let quoted = captures
            .name("quote")
            .is_some_and(|quote| !quote.as_str().is_empty());
        let Some(opener) = captures.get(0) else {
            continue;
        };
        let Some(body_start_relative) = raw[opener.end()..]
            .find('\n')
            .map(|index| opener.end() + index + 1)
        else {
            continue;
        };
        let closing_marker = format!("\n{delimiter}");
        let closing_start_relative = raw[body_start_relative..]
            .find(&closing_marker)
            .map(|index| body_start_relative + index + 1)
            .unwrap_or(raw.len());
        let body = raw[body_start_relative..closing_start_relative].to_string();
        heredocs.push(Heredoc {
            delimiter: delimiter.to_string(),
            quoted,
            target_instruction: target_instruction.to_string(),
            body,
            body_span: source_file.span(
                start_byte + body_start_relative,
                start_byte + closing_start_relative,
            ),
        });
    }

    Ok(heredocs)
}

impl Instruction {
    pub fn has_buildkit_features(&self) -> bool {
        !self.mounts.is_empty()
            || !self.heredocs.is_empty()
            || self
                .flags
                .iter()
                .any(|(name, _)| matches!(name.as_str(), "network" | "security" | "ssh"))
    }

    pub fn stage_alias(&self) -> Option<String> {
        if self.keyword != "FROM" {
            return None;
        }
        let parts = self.args.split_whitespace().collect::<Vec<_>>();
        parts
            .windows(2)
            .find(|window| window[0].eq_ignore_ascii_case("AS"))
            .map(|window| window[1].to_ascii_lowercase())
    }

    pub fn base_image(&self) -> Option<&str> {
        if self.keyword != "FROM" {
            return None;
        }
        self.args
            .split_whitespace()
            .find(|part| !part.starts_with("--") && !part.eq_ignore_ascii_case("AS"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_buildkit_mounts_and_syntax_directive() {
        let doc = parse_dockerfile(
            r#"# syntax=docker/dockerfile:1.7
FROM alpine:3.20 AS base
RUN --mount=type=cache,target=/var/cache/apk apk add curl
"#,
        )
        .unwrap();

        assert_eq!(doc.syntax.as_ref().unwrap().image, "docker/dockerfile:1.7");
        assert!(doc.has_buildkit_features);
        assert_eq!(doc.instructions[1].mounts[0].mount_type, "cache");
    }

    #[test]
    fn parses_parser_directives() {
        let doc = parse_dockerfile(
            r#"# syntax=docker/dockerfile:1.7
# escape=`
# check=skip=JSONArgsRecommended
FROM alpine:3.20
"#,
        )
        .unwrap();

        assert_eq!(doc.syntax.unwrap().image, "docker/dockerfile:1.7");
        assert_eq!(doc.escape.unwrap().character, '`');
        assert_eq!(doc.checks[0].value, "skip=JSONArgsRecommended");
    }

    #[test]
    fn parses_custom_escape_line_continuations() {
        let doc = parse_dockerfile(
            r#"# escape=`
RUN echo hello `
  && echo world
"#,
        )
        .unwrap();

        assert_eq!(doc.instructions[0].continuations[0].escape, '`');
        assert_eq!(doc.instructions[0].continuations[0].line, 2);
    }
}
