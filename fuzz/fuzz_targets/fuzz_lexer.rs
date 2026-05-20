//! Lexer-only fuzzer: arbitrary bytes → tokenise.
//!
//! Cheaper per iteration than fuzz_source so it can cover more of the
//! tokeniser's character-handling paths quickly.  Accepts raw bytes
//! (not restricted to UTF-8) because the lexer is the first thing that
//! sees user input.
//!
//! Run:
//!   cargo fuzz run fuzz_lexer

#![no_main]

use libfuzzer_sys::fuzz_target;
use asm_sim::lexer::Lexer;

fuzz_target!(|data: &[u8]| {
    // Feed raw bytes; non-UTF-8 should produce a LexError, never a panic.
    let src = String::from_utf8_lossy(data);
    let _ = Lexer::new(&src).tokenize();
});
