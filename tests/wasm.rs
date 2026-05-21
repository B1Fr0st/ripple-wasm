#![cfg(target_arch = "wasm32")]
use ripple_wasm::wasm::Simulator;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_node_experimental);

// ── Helpers ───────────────────────────────────────────────────────────────────

struct Run {
    stdout: String,
    stderr: String,
    exit_code: i32,
}

/// Assemble, run to completion, return outputs.
/// Panics if assembly fails.
fn run(src: &str) -> Run {
    let mut sim = Simulator::new(src);
    assert!(
        sim.error().is_none(),
        "assembly error: {}",
        sim.error().unwrap_or_default()
    );
    sim.run();
    Run {
        stdout: sim.take_stdout(),
        stderr: sim.take_stderr(),
        exit_code: sim.exit_code(),
    }
}

/// Assemble and run; assert exit 0.
fn ok(src: &str) -> Run {
    let r = run(src);
    assert_eq!(
        r.exit_code, 0,
        "expected exit 0\nstdout: {:?}\nstderr: {:?}",
        r.stdout, r.stderr
    );
    r
}

/// Assemble and run a program whose only job is to check a condition and
/// exit 0 on pass / 1 on fail.  The program should end with:
///
///     .pass:  mov rax, 60 / xor rdi, rdi / syscall
///     .fail:  mov rax, 60 / mov rdi, 1   / syscall
fn assert_program(src: &str) {
    ok(src);
}

// ── Assembly / load-time errors ───────────────────────────────────────────────

#[wasm_bindgen_test]
fn error_on_empty_source() {
    let sim = Simulator::new("");
    // No _start → assemble succeeds but CPU traps immediately; or it may
    // surface as a runtime error.  Either way it must not panic.
    drop(sim);
}

#[wasm_bindgen_test]
fn error_on_undefined_label() {
    let sim = Simulator::new(
        "
section .text
global _start
_start:
    jmp nonexistent
    mov rax, 60
    xor rdi, rdi
    syscall
",
    );
    assert!(
        sim.error().is_some(),
        "expected assemble error for undefined label"
    );
}

#[wasm_bindgen_test]
fn error_message_is_string() {
    let sim = Simulator::new("this is not valid assembly @@@@");
    // Must return Some(String), never panic.
    let _ = sim.error();
}

// ── stdout / stderr / exit code ───────────────────────────────────────────────

#[wasm_bindgen_test]
fn hello_world() {
    let r = ok("
section .data
    msg db \"Hello, World!\", 10
msg_len equ 14

section .text
global _start
_start:
    mov  rax, 1
    mov  rdi, 1
    lea  rsi, [msg]
    mov  rdx, msg_len
    syscall
    mov  rax, 60
    xor  rdi, rdi
    syscall
");
    assert_eq!(r.stdout, "Hello, World!\n");
}

#[wasm_bindgen_test]
fn exit_code_propagated() {
    let r = run("
section .text
global _start
_start:
    mov rax, 60
    mov rdi, 42
    syscall
");
    assert_eq!(r.exit_code, 42);
}

#[wasm_bindgen_test]
fn write_to_stderr() {
    let r = ok("
section .data
    msg db \"err\", 10
section .text
global _start
_start:
    mov rax, 1
    mov rdi, 2
    lea rsi, [msg]
    mov rdx, 4
    syscall
    mov rax, 60
    xor rdi, rdi
    syscall
");
    assert_eq!(r.stderr, "err\n");
    assert_eq!(r.stdout, "");
}

// ── Step mode ─────────────────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn step_advances_one_instruction() {
    let mut sim = Simulator::new(
        "
section .text
global _start
_start:
    mov rax, 1
    mov rbx, 2
    mov rax, 60
    xor rdi, rdi
    syscall
",
    );
    assert!(sim.error().is_none());
    assert!(!sim.is_halted());
    assert_eq!(sim.steps(), 0);
    sim.step();
    assert_eq!(sim.steps(), 1);
    assert_eq!(sim.regs().rax, 1);
    sim.step();
    assert_eq!(sim.steps(), 2);
    assert_eq!(sim.regs().rbx, 2);
    sim.run();
    assert!(sim.is_halted());
}

#[wasm_bindgen_test]
fn step_halts_on_exit() {
    let mut sim = Simulator::new(
        "
section .text
global _start
_start:
    mov rax, 60
    xor rdi, rdi
    syscall
",
    );
    assert!(sim.error().is_none());
    while !sim.is_halted() {
        sim.step();
    }
    assert!(sim.is_halted());
    // Stepping past halt is a no-op (returns 1 = halted).
    let result = sim.step();
    assert_ne!(result, 0);
}

#[wasm_bindgen_test]
fn take_stdout_drains_buffer() {
    let mut sim = Simulator::new(
        "
section .data
    a db \"a\"
    b db \"b\"
section .text
global _start
_start:
    mov rax, 1
    mov rdi, 1
    lea rsi, [a]
    mov rdx, 1
    syscall
    mov rax, 1
    mov rdi, 1
    lea rsi, [b]
    mov rdx, 1
    syscall
    mov rax, 60
    xor rdi, rdi
    syscall
",
    );
    assert!(sim.error().is_none());
    // Run just far enough for the first write (7 instructions).
    for _ in 0..7 {
        sim.step();
    }
    let first = sim.take_stdout();
    // Drain should clear; subsequent take returns only new output.
    let empty = sim.take_stdout();
    assert_eq!(empty, "");
    while !sim.is_halted() {
        sim.step();
    }
    let second = sim.take_stdout();
    assert_eq!(first + &second, "ab");
}

// ── Arithmetic invariants (programs as oracle) ────────────────────────────────

#[wasm_bindgen_test]
fn add_commutative() {
    assert_program(
        "
section .text
global _start
_start:
    mov  rax, 12345
    add  rax, 67890     ; rax = 80235
    mov  rbx, 67890
    add  rbx, 12345     ; rbx = 80235
    cmp  rax, rbx
    jne  .fail
    mov  rax, 60
    xor  rdi, rdi
    syscall
.fail:
    mov  rax, 60
    mov  rdi, 1
    syscall
",
    );
}

#[wasm_bindgen_test]
fn sub_self_is_zero() {
    assert_program(
        "
section .text
global _start
_start:
    mov  rax, 0xdeadbeef
    sub  rax, rax
    test rax, rax
    jnz  .fail
    mov  rax, 60
    xor  rdi, rdi
    syscall
.fail:
    mov  rax, 60
    mov  rdi, 1
    syscall
",
    );
}

#[wasm_bindgen_test]
fn xor_self_is_zero() {
    assert_program(
        "
section .text
global _start
_start:
    mov  rax, 0xffffffffffffffff
    xor  rax, rax
    test rax, rax
    jnz  .fail
    mov  rax, 60
    xor  rdi, rdi
    syscall
.fail:
    mov  rax, 60
    mov  rdi, 1
    syscall
",
    );
}

#[wasm_bindgen_test]
fn xor_double_is_identity() {
    assert_program(
        "
section .text
global _start
_start:
    mov  rax, 0xabcdef0123456789
    mov  rbx, 0x1111111111111111
    xor  rax, rbx
    xor  rax, rbx       ; rax should be back to original
    mov  rcx, 0xabcdef0123456789
    cmp  rax, rcx
    jne  .fail
    mov  rax, 60
    xor  rdi, rdi
    syscall
.fail:
    mov  rax, 60
    mov  rdi, 1
    syscall
",
    );
}

#[wasm_bindgen_test]
fn neg_double_is_identity() {
    assert_program(
        "
section .text
global _start
_start:
    mov  rax, 12345678
    neg  rax
    neg  rax
    cmp  rax, 12345678
    jne  .fail
    mov  rax, 60
    xor  rdi, rdi
    syscall
.fail:
    mov  rax, 60
    mov  rdi, 1
    syscall
",
    );
}

#[wasm_bindgen_test]
fn add_sub_roundtrip() {
    assert_program(
        "
section .text
global _start
_start:
    mov  rax, 99999
    add  rax, 12345
    sub  rax, 12345
    cmp  rax, 99999
    jne  .fail
    mov  rax, 60
    xor  rdi, rdi
    syscall
.fail:
    mov  rax, 60
    mov  rdi, 1
    syscall
",
    );
}

// ── Flags and conditional jumps ───────────────────────────────────────────────

#[wasm_bindgen_test]
fn zf_set_on_zero_result() {
    assert_program(
        "
section .text
global _start
_start:
    xor  rax, rax
    jnz  .fail          ; ZF must be set
    mov  rax, 60
    xor  rdi, rdi
    syscall
.fail:
    mov  rax, 60
    mov  rdi, 1
    syscall
",
    );
}

#[wasm_bindgen_test]
fn cf_set_on_unsigned_underflow() {
    assert_program(
        "
section .text
global _start
_start:
    mov  rax, 0
    sub  rax, 1         ; 0 - 1 wraps; CF must be set
    jnc  .fail
    mov  rax, 60
    xor  rdi, rdi
    syscall
.fail:
    mov  rax, 60
    mov  rdi, 1
    syscall
",
    );
}

#[wasm_bindgen_test]
fn sf_set_on_negative_result() {
    assert_program(
        "
section .text
global _start
_start:
    mov  rax, 0
    sub  rax, 1         ; result is -1 (0xfff...); SF must be set
    jns  .fail
    mov  rax, 60
    xor  rdi, rdi
    syscall
.fail:
    mov  rax, 60
    mov  rdi, 1
    syscall
",
    );
}

#[wasm_bindgen_test]
fn all_conditional_jumps() {
    assert_program(
        "
section .text
global _start
_start:
    ; je / jne
    mov  rax, 5
    cmp  rax, 5
    jne  .fail
    cmp  rax, 6
    je   .fail

    ; jl / jge  (signed)
    mov  rax, 3
    cmp  rax, 5
    jge  .fail
    cmp  rax, 3
    jl   .fail

    ; jg / jle  (signed)
    mov  rax, 7
    cmp  rax, 5
    jle  .fail
    cmp  rax, 7
    jg   .fail

    ; js / jns
    mov  rax, -1
    test rax, rax
    jns  .fail
    mov  rax, 1
    test rax, rax
    js   .fail

    mov  rax, 60
    xor  rdi, rdi
    syscall
.fail:
    mov  rax, 60
    mov  rdi, 1
    syscall
",
    );
}

// ── Control flow ──────────────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn loop_instruction() {
    // loop decrements rcx and jumps if rcx != 0
    assert_program(
        "
section .text
global _start
_start:
    mov  rcx, 5
    mov  rax, 0
.top:
    inc  rax
    loop .top
    cmp  rax, 5
    jne  .fail
    mov  rax, 60
    xor  rdi, rdi
    syscall
.fail:
    mov  rax, 60
    mov  rdi, 1
    syscall
",
    );
}

#[wasm_bindgen_test]
fn call_and_ret() {
    assert_program(
        "
section .text
global _start

double:
    imul rax, rax, 2
    ret

_start:
    mov  rax, 21
    call double         ; rax = 42
    cmp  rax, 42
    jne  .fail
    mov  rax, 60
    xor  rdi, rdi
    syscall
.fail:
    mov  rax, 60
    mov  rdi, 1
    syscall
",
    );
}

#[wasm_bindgen_test]
fn push_pop_roundtrip() {
    assert_program(
        "
section .text
global _start
_start:
    mov  rax, 0xdeadbeef
    push rax
    mov  rax, 0             ; clobber
    pop  rax
    cmp  rax, 0xdeadbeef
    jne  .fail
    mov  rax, 60
    xor  rdi, rdi
    syscall
.fail:
    mov  rax, 60
    mov  rdi, 1
    syscall
",
    );
}

// ── Register widths ───────────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn dword_write_zero_extends() {
    // Writing to eax must zero the upper 32 bits of rax.
    assert_program(
        "
section .text
global _start
_start:
    mov  rax, 0xffffffffffffffff
    mov  eax, 1             ; upper 32 bits must become 0
    mov  rcx, 0x0000000000000001
    cmp  rax, rcx
    jne  .fail
    mov  rax, 60
    xor  rdi, rdi
    syscall
.fail:
    mov  rax, 60
    mov  rdi, 1
    syscall
",
    );
}

#[wasm_bindgen_test]
fn byte_write_does_not_clear_upper_bits() {
    // Writing al must leave bits 8–63 of rax unchanged.
    assert_program(
        "
section .text
global _start
_start:
    mov  rax, 0x1122334455667788
    mov  al,  0xff
    mov  rcx, 0x11223344556677ff
    cmp  rax, rcx
    jne  .fail
    mov  rax, 60
    xor  rdi, rdi
    syscall
.fail:
    mov  rax, 60
    mov  rdi, 1
    syscall
",
    );
}

#[wasm_bindgen_test]
fn movzx_zero_extends() {
    assert_program(
        "
section .text
global _start
_start:
    mov  rax, 0xffffffffffffffff
    mov  al,  0x42
    movzx rbx, al           ; rbx = 0x42, upper bits zero
    cmp  rbx, 0x42
    jne  .fail
    mov  rax, 60
    xor  rdi, rdi
    syscall
.fail:
    mov  rax, 60
    mov  rdi, 1
    syscall
",
    );
}

#[wasm_bindgen_test]
fn movsx_sign_extends() {
    assert_program(
        "
section .text
global _start
_start:
    mov  al, 0x80           ; -128 as signed byte
    movsx rax, al           ; rax = 0xffffffffffffff80
    mov  rcx, -128
    cmp  rax, rcx
    jne  .fail
    mov  rax, 60
    xor  rdi, rdi
    syscall
.fail:
    mov  rax, 60
    mov  rdi, 1
    syscall
",
    );
}

// ── Multiply / divide ─────────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn imul_two_op() {
    assert_program(
        "
section .text
global _start
_start:
    mov  rax, 6
    imul rax, rax, 7    ; rax = 42
    cmp  rax, 42
    jne  .fail
    mov  rax, 60
    xor  rdi, rdi
    syscall
.fail:
    mov  rax, 60
    mov  rdi, 1
    syscall
",
    );
}

#[wasm_bindgen_test]
fn div_quotient_and_remainder() {
    assert_program(
        "
section .text
global _start
_start:
    mov  rax, 100
    xor  rdx, rdx
    mov  rcx, 7
    div  rcx            ; rax = 14, rdx = 2
    cmp  rax, 14
    jne  .fail
    cmp  rdx, 2
    jne  .fail
    mov  rax, 60
    xor  rdi, rdi
    syscall
.fail:
    mov  rax, 60
    mov  rdi, 1
    syscall
",
    );
}

// ── Shifts ────────────────────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn shl_shr_roundtrip() {
    assert_program(
        "
section .text
global _start
_start:
    mov  rax, 0b1011
    shl  rax, 4         ; rax = 0b10110000
    shr  rax, 4         ; rax = 0b1011
    cmp  rax, 0b1011
    jne  .fail
    mov  rax, 60
    xor  rdi, rdi
    syscall
.fail:
    mov  rax, 60
    mov  rdi, 1
    syscall
",
    );
}

#[wasm_bindgen_test]
fn sar_preserves_sign() {
    assert_program(
        "
section .text
global _start
_start:
    mov  rax, -256      ; negative value
    sar  rax, 4         ; arithmetic: rax = -16
    cmp  rax, -16
    jne  .fail
    mov  rax, 60
    xor  rdi, rdi
    syscall
.fail:
    mov  rax, 60
    mov  rdi, 1
    syscall
",
    );
}

// ── Memory addressing ─────────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn mem_write_read_roundtrip() {
    assert_program(
        "
section .data
    buf db 0, 0, 0, 0, 0, 0, 0, 0
section .text
global _start
_start:
    lea  rdi, [buf]
    mov  qword ptr [rdi], 0xdeadbeefcafebabe
    mov  rax, qword ptr [rdi]
    mov  rcx, 0xdeadbeefcafebabe
    cmp  rax, rcx
    jne  .fail
    mov  rax, 60
    xor  rdi, rdi
    syscall
.fail:
    mov  rax, 60
    mov  rdi, 1
    syscall
",
    );
}

#[wasm_bindgen_test]
fn mem_indexed_addressing() {
    assert_program(
        "
section .data
    arr db 10, 20, 30, 40
section .text
global _start
_start:
    lea  rsi, [arr]
    mov  rcx, 2
    movzx rax, byte ptr [rsi + rcx]   ; arr[2] = 30
    cmp  rax, 30
    jne  .fail
    mov  rax, 60
    xor  rdi, rdi
    syscall
.fail:
    mov  rax, 60
    mov  rdi, 1
    syscall
",
    );
}
