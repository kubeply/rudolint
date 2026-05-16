mod parser;

pub use parser::{
    CheckDirective, Comment, Dockerfile, EscapeDirective, Heredoc, Instruction, InstructionForm,
    LineContinuation, SyntaxDirective, parse_dockerfile,
};
