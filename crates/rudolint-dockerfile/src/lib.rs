mod parser;

pub use parser::{
    CheckDirective, Comment, Dockerfile, EscapeDirective, FromInstruction, Heredoc, Instruction,
    InstructionForm, LineContinuation, RunInstruction, ShellBody, SyntaxDirective,
    parse_dockerfile,
};
