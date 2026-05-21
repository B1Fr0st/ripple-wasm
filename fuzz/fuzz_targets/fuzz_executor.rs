//! Structured executor fuzzer.
//!
//! Instead of random bytes, this target uses `arbitrary::Unstructured`
//! to drive a program generator that always produces syntactically
//! valid assembly.  This means nearly every input reaches the CPU,
//! maximising coverage of instruction dispatch, flag logic, memory
//! addressing, call/ret, and syscall handling.
//!
//! Generator contract:
//!   - _start is always defined (assembler entry point)
//!   - All jump/call targets reference labels that exist in the program
//!   - rsp is never used as a data register (avoids clobbering the stack)
//!   - The program always ends with exit(0) so the CPU halts cleanly
//!   - max_steps = 50_000 to bound execution time
//!
//! Run:
//!   cargo fuzz run fuzz_executor
//!   cargo fuzz run fuzz_executor fuzz/corpus/fuzz_executor

#![no_main]

use arbitrary::Unstructured;
use asm_sim::{assembler::Assembler, cpu::Cpu, lexer::Lexer, parser::Parser};
use libfuzzer_sys::fuzz_target;

// ── Register pools ────────────────────────────────────────────────────────────

// Safe general-purpose registers (rsp excluded to avoid stack corruption).
const GP64: &[&str] = &[
    "rax", "rbx", "rcx", "rdx", "rsi", "rdi", "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15",
];
const GP32: &[&str] = &[
    "eax", "ebx", "ecx", "edx", "esi", "edi", "r8d", "r9d", "r10d", "r11d", "r12d", "r13d", "r14d",
    "r15d",
];
const GP8: &[&str] = &["al", "bl", "cl", "dl", "sil", "dil"];

// Shift count register.
const SHIFT_REG: &str = "cl";

// ── Helpers ───────────────────────────────────────────────────────────────────

fn pick<'a, T>(u: &mut Unstructured, slice: &'a [T]) -> arbitrary::Result<&'a T> {
    let idx = u.int_in_range(0..=slice.len() - 1)?;
    Ok(&slice[idx])
}

fn pick_label<'a>(u: &mut Unstructured, labels: &'a [String]) -> arbitrary::Result<&'a str> {
    Ok(pick(u, labels)?.as_str())
}

// ── Instruction generator ─────────────────────────────────────────────────────

fn gen_instr(u: &mut Unstructured, labels: &[String], out: &mut String) -> arbitrary::Result<()> {
    let kind: u8 = u.int_in_range(0..=16)?;
    match kind {
        // mov reg64, reg64
        0 => {
            let dst = *pick(u, GP64)?;
            let src = *pick(u, GP64)?;
            out.push_str(&format!("    mov {dst}, {src}\n"));
        }
        // mov reg64, imm32
        1 => {
            let dst = *pick(u, GP64)?;
            let imm: i32 = u.arbitrary()?;
            out.push_str(&format!("    mov {dst}, {imm}\n"));
        }
        // mov reg8, imm8
        2 => {
            let dst = *pick(u, GP8)?;
            let imm: i8 = u.arbitrary()?;
            out.push_str(&format!("    mov {dst}, {imm}\n"));
        }
        // alu reg64, reg64  (add sub and or xor)
        3 => {
            let op = *pick(u, &["add", "sub", "and", "or", "xor"])?;
            let dst = *pick(u, GP64)?;
            let src = *pick(u, GP64)?;
            out.push_str(&format!("    {op} {dst}, {src}\n"));
        }
        // alu reg64, imm32
        4 => {
            let op = *pick(u, &["add", "sub", "and", "or", "xor"])?;
            let dst = *pick(u, GP64)?;
            let imm: i32 = u.arbitrary()?;
            out.push_str(&format!("    {op} {dst}, {imm}\n"));
        }
        // inc / dec / neg / not
        5 => {
            let op = *pick(u, &["inc", "dec", "neg", "not"])?;
            let dst = *pick(u, GP64)?;
            out.push_str(&format!("    {op} {dst}\n"));
        }
        // imul reg64, reg64
        6 => {
            let dst = *pick(u, GP64)?;
            let src = *pick(u, GP64)?;
            out.push_str(&format!("    imul {dst}, {src}\n"));
        }
        // imul reg64, reg64, imm8
        7 => {
            let dst = *pick(u, GP64)?;
            let src = *pick(u, GP64)?;
            let imm: i8 = u.arbitrary()?;
            out.push_str(&format!("    imul {dst}, {src}, {imm}\n"));
        }
        // shift reg64, imm8
        8 => {
            let op = *pick(u, &["shl", "shr", "sar", "rol", "ror"])?;
            let dst = *pick(u, GP64)?;
            let count = u.int_in_range(1u8..=63)?;
            out.push_str(&format!("    {op} {dst}, {count}\n"));
        }
        // shift reg64, cl
        9 => {
            let op = *pick(u, &["shl", "shr", "sar"])?;
            let dst = *pick(u, GP64)?;
            out.push_str(&format!("    {op} {dst}, {SHIFT_REG}\n"));
        }
        // push / pop (safe register only)
        10 => {
            let reg = *pick(u, GP64)?;
            out.push_str(&format!("    push {reg}\n    pop  {reg}\n"));
        }
        // cmp + conditional jump to a known label
        11 => {
            let jop = *pick(
                u,
                &[
                    "je", "jne", "jl", "jle", "jg", "jge", "js", "jns", "jc", "jnc",
                ],
            )?;
            let a = *pick(u, GP64)?;
            let b = *pick(u, GP64)?;
            let tgt = pick_label(u, labels)?;
            out.push_str(&format!("    cmp {a}, {b}\n    {jop} {tgt}\n"));
        }
        // test + conditional jump
        12 => {
            let jop = *pick(u, &["jz", "jnz"])?;
            let a = *pick(u, GP64)?;
            let tgt = pick_label(u, labels)?;
            out.push_str(&format!("    test {a}, {a}\n    {jop} {tgt}\n"));
        }
        // unconditional jmp to a known label
        13 => {
            let tgt = pick_label(u, labels)?;
            out.push_str(&format!("    jmp {tgt}\n"));
        }
        // call + ret (call a known label that immediately returns)
        14 => {
            let tgt = pick_label(u, labels)?;
            out.push_str(&format!("    call {tgt}\n"));
        }

        15 => {
            out.push_str(&format!("    nop\n"));
        }
        // xchg two registers
        _ => {
            let a = *pick(u, GP64)?;
            let b = *pick(u, GP64)?;
            out.push_str(&format!("    xchg {a}, {b}\n"));
        }
    }
    Ok(())
}

// ── Program generator ─────────────────────────────────────────────────────────

fn generate(u: &mut Unstructured) -> arbitrary::Result<String> {
    // Build a flat list of labels: _start always first, then .L0 .. .LN.
    let n_extra: usize = u.int_in_range(0usize..=4)?;
    let extra: Vec<String> = (0..n_extra).map(|i| format!(".L{i}")).collect();
    let all_labels: Vec<String> = std::iter::once("_start".to_string())
        .chain(extra.iter().cloned())
        .collect();

    // Optional data section (a few bytes for memory-addressing exercises).
    let mut src = String::new();
    let n_data: usize = u.int_in_range(0usize..=3)?;
    if n_data > 0 {
        src.push_str("section .data\n");
        for i in 0..n_data {
            let val: u8 = u.arbitrary()?;
            src.push_str(&format!("    dat{i} db {val}\n"));
        }
    }

    src.push_str("section .text\nglobal _start\n");

    // Emit each labeled block in forward order so jumps to later labels
    // are forward refs and to earlier labels are backward refs — both are
    // exercised by the flat label list.
    for (block_idx, label) in all_labels.iter().enumerate() {
        src.push_str(&format!("{label}:\n"));

        // Last block always exits so the CPU halts.
        if block_idx == all_labels.len() - 1 {
            src.push_str("    mov rax, 60\n    xor rdi, rdi\n    syscall\n");
            break;
        }

        let n_instrs: usize = u.int_in_range(0usize..=8)?;
        for _ in 0..n_instrs {
            gen_instr(u, &all_labels, &mut src)?;
        }

        // Fall through to next label (no explicit jump needed — label follows).
    }

    Ok(src)
}

// ── Fuzz target ───────────────────────────────────────────────────────────────

fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);
    let Ok(src) = generate(&mut u) else { return };

    let Ok(tokens) = Lexer::new(&src).tokenize() else {
        return;
    };
    let Ok(lines) = Parser::new(tokens).parse() else {
        return;
    };
    let Ok(prog) = Assembler::new(lines).assemble() else {
        return;
    };
    let Ok(mut cpu) = Cpu::new(prog, false, 50_000) else {
        return;
    };
    let _ = cpu.run();
});
