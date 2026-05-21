use crate::{
    assembler::Assembler,
    cpu::{Cpu, StepKind},
    error::SimError,
    lexer::Lexer,
    parser::Parser,
    program::TEXT_BASE,
};
use wasm_bindgen::prelude::*;

/// Assemble source into a Cpu, returning a structured error tuple on failure.
/// Error tuple: (kind, line, col, message)
///   kind: 1=lex, 2=parse, 3=assemble, 4=runtime
fn build(source: &str) -> Result<Cpu, (u32, u32, u32, String)> {
    let tokens = Lexer::new(source).tokenize().map_err(|e| match e {
        SimError::Lex { line, col, msg } => (1u32, line as u32, col as u32, msg),
        e => (1, 0, 0, e.to_string()),
    })?;
    let lines = Parser::new(tokens).parse().map_err(|e| match e {
        SimError::Parse { line, msg } => (2u32, line as u32, 0, msg),
        e => (2, 0, 0, e.to_string()),
    })?;
    let prog = Assembler::new(lines).assemble().map_err(|e| match e {
        SimError::Assemble { msg } => (3u32, 0, 0, msg),
        e => (3, 0, 0, e.to_string()),
    })?;
    Cpu::new(prog, false, 1_000_000).map_err(|e| (4u32, 0, 0, e.to_string()))
}

/// Snapshot of the CPU register file returned by `Simulator::regs()`.
#[wasm_bindgen]
#[derive(Default)]
pub struct WasmRegisters {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rsp: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub cf: bool,
    pub zf: bool,
    pub sf: bool,
    pub of: bool,
    pub pf: bool,
}

/// A single assembled + loaded simulator instance.
#[wasm_bindgen]
pub struct Simulator {
    cpu: Option<Cpu>,
    init_error: Option<String>,
    err_kind: u32,
    err_line: u32,
    err_col: u32,
}

#[wasm_bindgen]
impl Simulator {
    /// Assemble and load `source`. Check `error()` before using.
    #[wasm_bindgen(constructor)]
    pub fn new(source: &str) -> Simulator {
        match build(source) {
            Ok(cpu) => Simulator {
                cpu: Some(cpu),
                init_error: None,
                err_kind: 0,
                err_line: 0,
                err_col: 0,
            },
            Err((kind, line, col, msg)) => Simulator {
                cpu: None,
                init_error: Some(msg),
                err_kind: kind,
                err_line: line,
                err_col: col,
            },
        }
    }

    /// Returns the assembly error message, or `undefined` if successful.
    pub fn error(&self) -> Option<String> {
        self.init_error.clone()
    }

    /// Error kind: 0=none, 1=lex, 2=parse, 3=assemble, 4=runtime.
    pub fn error_kind(&self) -> u32 {
        self.err_kind
    }

    /// Source line of the error (1-indexed), or 0 if not applicable.
    pub fn error_line(&self) -> u32 {
        self.err_line
    }

    /// Column of the error (1-indexed), or 0 if not applicable.
    pub fn error_col(&self) -> u32 {
        self.err_col
    }

    /// Returns true when the program has halted (exit syscall or error).
    pub fn is_halted(&self) -> bool {
        self.cpu.as_ref().map_or(true, |c| c.halted)
    }

    /// Returns true when execution is paused at a breakpoint.
    pub fn is_at_breakpoint(&self) -> bool {
        self.cpu.as_ref().map_or(false, |c| c.at_breakpoint)
    }

    /// Exit code set by the program's exit(N) syscall.
    pub fn exit_code(&self) -> i32 {
        self.cpu.as_ref().map_or(0, |c| c.exit_code)
    }

    /// Execute one instruction.
    /// Returns: 0=running, 1=halted, 2=breakpoint hit.
    /// Runtime errors surface via `take_stderr()` and halt (returns 1).
    pub fn step(&mut self) -> u32 {
        let cpu = match self.cpu.as_mut() {
            Some(c) => c,
            None => return 1,
        };
        match cpu.step() {
            Ok(StepKind::Running) => 0,
            Ok(StepKind::Halted) => 1,
            Ok(StepKind::Breakpoint) => 2,
            Err(e) => {
                let msg = format!("[runtime error] {}\n", e);
                cpu.stderr.extend_from_slice(msg.as_bytes());
                cpu.halted = true;
                1
            }
        }
    }

    /// Run to completion or until a breakpoint.
    /// Returns: 1=halted, 2=breakpoint hit.
    pub fn run(&mut self) -> u32 {
        let cpu = match self.cpu.as_mut() {
            Some(c) => c,
            None => return 1,
        };
        match cpu.run() {
            Ok(StepKind::Halted) => 1,
            Ok(StepKind::Breakpoint) => 2,
            Ok(StepKind::Running) => 0,
            Err(e) => {
                let msg = format!("[runtime error] {}\n", e);
                cpu.stderr.extend_from_slice(msg.as_bytes());
                cpu.halted = true;
                1
            }
        }
    }

    /// Source line number (1-indexed) of the instruction at the current rip.
    /// Returns 0 when halted or the rip is out of range.
    pub fn current_source_line(&self) -> u32 {
        self.cpu
            .as_ref()
            .map_or(0, |c| c.current_source_line() as u32)
    }

    /// Set a breakpoint on the given source line (1-indexed).
    pub fn set_breakpoint(&mut self, line: u32) {
        if let Some(cpu) = self.cpu.as_mut() {
            cpu.breakpoints.insert(line as usize);
        }
    }

    /// Clear a breakpoint on the given source line.
    pub fn clear_breakpoint(&mut self, line: u32) {
        if let Some(cpu) = self.cpu.as_mut() {
            cpu.breakpoints.remove(&(line as usize));
        }
    }

    /// Clear all breakpoints.
    pub fn clear_breakpoints(&mut self) {
        if let Some(cpu) = self.cpu.as_mut() {
            cpu.breakpoints.clear();
        }
    }

    /// Drain all stdout bytes produced since the last call.
    pub fn take_stdout(&mut self) -> String {
        let bytes = self.cpu.as_mut().map_or(vec![], |c| c.take_stdout());
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// Drain all stderr bytes produced since the last call.
    pub fn take_stderr(&mut self) -> String {
        let bytes = self.cpu.as_mut().map_or(vec![], |c| c.take_stderr());
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// Feed bytes into stdin.
    pub fn feed_stdin(&mut self, data: &[u8]) {
        if let Some(cpu) = self.cpu.as_mut() {
            cpu.stdin.extend_from_slice(data);
        }
    }

    /// Current register state as a human-readable string.
    pub fn dump_regs(&self) -> String {
        let cpu = match self.cpu.as_ref() {
            Some(c) => c,
            None => return String::new(),
        };
        let r = &cpu.regs;
        format!(
            "rax={:#018x} rbx={:#018x} rcx={:#018x} rdx={:#018x}\n\
             rsi={:#018x} rdi={:#018x} rsp={:#018x} rbp={:#018x}\n\
             r8={:#018x} r9={:#018x} r10={:#018x} r11={:#018x}\n\
             rip={:#018x}  CF={} ZF={} SF={} OF={} PF={}\n\
             steps={} halted={}",
            r.gpr[0],
            r.gpr[1],
            r.gpr[2],
            r.gpr[3],
            r.gpr[4],
            r.gpr[5],
            r.gpr[6],
            r.gpr[7],
            r.gpr[8],
            r.gpr[9],
            r.gpr[10],
            r.gpr[11],
            r.rip,
            r.flags.cf as u8,
            r.flags.zf as u8,
            r.flags.sf as u8,
            r.flags.of as u8,
            r.flags.pf as u8,
            cpu.steps,
            cpu.halted,
        )
    }

    /// Number of instructions executed so far.
    pub fn steps(&self) -> u64 {
        self.cpu.as_ref().map_or(0, |c| c.steps)
    }

    /// Snapshot of the current CPU register state.
    pub fn regs(&self) -> WasmRegisters {
        self.cpu.as_ref().map_or_else(WasmRegisters::default, |c| {
            let r = &c.regs;
            WasmRegisters {
                rax: r.gpr[0],
                rbx: r.gpr[1],
                rcx: r.gpr[2],
                rdx: r.gpr[3],
                rsi: r.gpr[4],
                rdi: r.gpr[5],
                rsp: r.gpr[6],
                rbp: r.gpr[7],
                r8: r.gpr[8],
                r9: r.gpr[9],
                r10: r.gpr[10],
                r11: r.gpr[11],
                r12: r.gpr[12],
                r13: r.gpr[13],
                r14: r.gpr[14],
                r15: r.gpr[15],
                rip: r.rip,
                cf: r.flags.cf,
                zf: r.flags.zf,
                sf: r.flags.sf,
                of: r.flags.of,
                pf: r.flags.pf,
            }
        })
    }

    /// Read raw bytes from memory. Returns empty vec on out-of-range access.
    pub fn read_mem(&self, addr: u32, len: u32) -> Vec<u8> {
        let cpu = match self.cpu.as_ref() {
            Some(c) => c,
            None => return vec![],
        };
        let mut out = Vec::with_capacity(len as usize);
        for i in 0..len as u64 {
            match cpu.mem.read_u8(addr as u64 + i, TEXT_BASE) {
                Ok(b) => out.push(b),
                Err(_) => break,
            }
        }
        out
    }
}
