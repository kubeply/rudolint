mod parser;

pub use parser::{
    CheckDirective, Comment, Dockerfile, EscapeDirective, Instruction, InstructionForm,
    LineContinuation, SyntaxDirective, parse_dockerfile,
};
