use crate::{
    assembler::Assembler,
    cpu::Cpu,
    lexer::Lexer,
    parser::Parser,
};
use wasm_bindgen::prelude::*;

/// Build a Cpu from assembly source, returning Err(message) on failure.
fn build(source: &str) -> Result<Cpu, String> {
    let tokens = Lexer::new(source).tokenize().map_err(|e| e.to_string())?;
    let lines = Parser::new(tokens).parse().map_err(|e| e.to_string())?;
    let prog = Assembler::new(lines)
        .assemble()
        .map_err(|e| e.to_string())?;
    Cpu::new(prog, false, 1_000_000).map_err(|e| e.to_string())
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
    pub r8:  u64,
    pub r9:  u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub cf:  bool,
    pub zf:  bool,
    pub sf:  bool,
    /// Overflow flag (`of` is a Rust keyword, so exposed as `of_` in Rust; JS sees `of_`).
    pub of_: bool,
    pub pf:  bool,
}

/// A single assembled + loaded simulator instance.
///
/// JS usage:
/// ```js
/// const sim = new Simulator(src);
/// if (sim.error()) { console.error(sim.error()); return; }
///
/// // step-by-step for live output:
/// while (!sim.is_halted()) {
///   sim.step();
///   process.stdout.write(sim.take_stdout());
///   process.stderr.write(sim.take_stderr());
/// }
///
/// // — or — run to completion:
/// sim.run();
/// console.log(sim.take_stdout());
///
/// // inspect registers at any point (does not advance execution):
/// const r = sim.regs();
/// console.log(r.rax, r.rbx, r.rip);  // u64 values exposed as BigInt in JS
/// console.log(r.zf, r.cf, r.sf, r.of_, r.pf);  // boolean flags
/// ```
#[wasm_bindgen]
pub struct Simulator {
    cpu: Option<Cpu>,
    init_error: Option<String>,
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
            },
            Err(e) => Simulator {
                cpu: None,
                init_error: Some(e),
            },
        }
    }

    /// Returns the assembly/load error message, or `undefined` if successful.
    pub fn error(&self) -> Option<String> {
        self.init_error.clone()
    }

    /// Returns true when the program has halted (exit syscall or error).
    pub fn is_halted(&self) -> bool {
        self.cpu.as_ref().map_or(true, |c| c.halted)
    }

    /// Exit code set by the program's exit(N) syscall.
    pub fn exit_code(&self) -> i32 {
        self.cpu.as_ref().map_or(0, |c| c.exit_code)
    }

    /// Execute one instruction. Returns `true` on halt.
    /// Any runtime error is surfaced via `take_stderr()` and `is_halted()`.
    pub fn step(&mut self) -> bool {
        let cpu = match self.cpu.as_mut() {
            Some(c) => c,
            None => return true,
        };
        match cpu.step() {
            Ok(halted) => halted,
            Err(e) => {
                let msg = format!("[runtime error] {}\n", e);
                cpu.stderr.extend_from_slice(msg.as_bytes());
                cpu.halted = true;
                true
            }
        }
    }

    /// Run to completion. Any runtime error is surfaced via `take_stderr()`.
    pub fn run(&mut self) {
        let cpu = match self.cpu.as_mut() {
            Some(c) => c,
            None => return,
        };
        if let Err(e) = cpu.run() {
            let msg = format!("[runtime error] {}\n", e);
            cpu.stderr.extend_from_slice(msg.as_bytes());
            cpu.halted = true;
        }
    }

    /// Drain all stdout bytes produced since the last call.
    /// Returns a UTF-8 string (invalid bytes replaced with '?').
    pub fn take_stdout(&mut self) -> String {
        let bytes = self.cpu.as_mut().map_or(vec![], |c| c.take_stdout());
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// Drain all stderr bytes produced since the last call.
    pub fn take_stderr(&mut self) -> String {
        let bytes = self.cpu.as_mut().map_or(vec![], |c| c.take_stderr());
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// Feed bytes into stdin (call before `run()` / `step()` for programs that read).
    pub fn feed_stdin(&mut self, data: &[u8]) {
        if let Some(cpu) = self.cpu.as_mut() {
            cpu.stdin.extend_from_slice(data);
        }
    }

    /// Current register state as a human-readable string (does not advance execution).
    pub fn dump_regs(&self) -> String {
        let cpu = match self.cpu.as_ref() {
            Some(c) => c,
            None => return String::new(),
        };
        let r = &cpu.regs;
        format!(
            "rax={:#018x} rbx={:#018x} rcx={:#018x} rdx={:#018x}\n\
             rsi={:#018x} rdi={:#018x} rsp={:#018x} rbp={:#018x}\n\
             r8 ={:#018x} r9 ={:#018x} r10={:#018x} r11={:#018x}\n\
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

    /// Returns a snapshot of the current CPU register state (zeroed if not loaded).
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
                r8:  r.gpr[8],
                r9:  r.gpr[9],
                r10: r.gpr[10],
                r11: r.gpr[11],
                r12: r.gpr[12],
                r13: r.gpr[13],
                r14: r.gpr[14],
                r15: r.gpr[15],
                rip: r.rip,
                cf:  r.flags.cf,
                zf:  r.flags.zf,
                sf:  r.flags.sf,
                of_: r.flags.of,
                pf:  r.flags.pf,
            }
        })
    }
}
