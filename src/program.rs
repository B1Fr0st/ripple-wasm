use std::collections::HashMap;
use crate::ast::AsmLine;

pub const TEXT_BASE: u64 = 0x1000;
pub const DATA_BASE: u64 = 0x0010_0000;
pub const STACK_TOP: u64 = 0x00FF_0000;
pub const MEM_SIZE:  usize = 0x0100_0000; // 16 MiB

pub struct Program {
    /// Instructions in order. Index i is at virtual address TEXT_BASE + i (we interpret, not encode).
    pub instructions: Vec<AsmLine>,
    /// Resolved symbol table: label/equ name → value (virtual address or constant).
    pub symbols: HashMap<String, i64>,
    /// Initialized data region (placed at DATA_BASE).
    pub data: Vec<u8>,
    /// Entry point virtual address (index into instructions, biased by TEXT_BASE).
    pub entry: u64,
}
