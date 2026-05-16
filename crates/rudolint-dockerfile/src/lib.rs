mod parser;

pub use parser::{
    ArgInstruction, CheckDirective, Comment, CopyInstruction, CopyKind, Dockerfile,
    EscapeDirective, FromInstruction, HealthcheckInstruction, Heredoc, Instruction,
    InstructionForm, LineContinuation, RunInstruction, ShellBody, SyntaxDirective,
    parse_dockerfile,
};
