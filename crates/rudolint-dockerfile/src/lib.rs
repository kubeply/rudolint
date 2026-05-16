mod parser;

pub use parser::{
    CheckDirective, Dockerfile, EscapeDirective, Instruction, SyntaxDirective, parse_dockerfile,
};
