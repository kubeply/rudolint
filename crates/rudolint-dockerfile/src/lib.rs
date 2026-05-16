mod parser;

pub use parser::{
    CheckDirective, Comment, Dockerfile, EscapeDirective, Instruction, LineContinuation,
    SyntaxDirective, parse_dockerfile,
};
