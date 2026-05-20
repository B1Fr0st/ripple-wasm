//! Full pipeline fuzzer: raw bytes → lex → parse → assemble → run.
//!
//! Invariant: the pipeline must never panic regardless of input.
//! SimError returns are expected and fine; panics are bugs.
//!
//! Run:
//!   cargo fuzz run fuzz_source
//!   cargo fuzz run fuzz_source -- -max_len=4096
//!   cargo fuzz run fuzz_source fuzz/corpus/fuzz_source   (with seeds)
//!
//! With AddressSanitizer (recommended):
//!   RUSTFLAGS="-Zsanitizer=address" cargo fuzz run fuzz_source

#![no_main]

use libfuzzer_sys::fuzz_target;
use asm_sim::{lexer::Lexer, parser::Parser, assembler::Assembler, cpu::Cpu};

fuzz_target!(|data: &[u8]| {
    // Accept only valid UTF-8; non-UTF-8 bytes are tested by fuzz_lexer.
    let Ok(src) = std::str::from_utf8(data) else { return };
    run_pipeline(src);
});

fn run_pipeline(src: &str) {
    let tokens = match Lexer::new(src).tokenize() {
        Ok(t)  => t,
        Err(_) => return,
    };
    let lines = match Parser::new(tokens).parse() {
        Ok(l)  => l,
        Err(_) => return,
    };
    let prog = match Assembler::new(lines).assemble() {
        Ok(p)  => p,
        Err(_) => return,
    };
    let mut cpu = match Cpu::new(prog, false, 100_000) {
        Ok(c)  => c,
        Err(_) => return,
    };
    // Errors during execution are expected; panics are not.
    let _ = cpu.run();
}
