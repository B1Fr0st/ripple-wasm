pub mod error;
pub mod ast;
pub mod lexer;
pub mod parser;
pub mod assembler;
pub mod program;
pub mod cpu;

#[cfg(target_arch = "wasm32")]
pub mod wasm;
