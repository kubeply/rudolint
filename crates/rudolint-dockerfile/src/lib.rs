mod parser;

pub use parser::{
    CheckDirective, Comment, Dockerfile, EscapeDirective, FromInstruction, Heredoc, Instruction,
    InstructionForm, LineContinuation, SyntaxDirective, parse_dockerfile,
};
