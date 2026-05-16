mod parser;

pub use parser::{
    CheckDirective, Comment, Dockerfile, EscapeDirective, Instruction, SyntaxDirective,
    parse_dockerfile,
};
