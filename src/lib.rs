#![cfg(target_arch = "wasm32")]

pub mod assembler;
pub mod ast;
pub mod cpu;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod program;

pub mod wasm;
