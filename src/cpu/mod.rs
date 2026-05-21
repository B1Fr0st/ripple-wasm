pub mod alu;
pub mod execute;
pub mod memory;
pub mod registers;
pub mod syscall;

use crate::ast::AsmLine;
use crate::error::SimError;
use crate::program::{DATA_BASE, Program, STACK_TOP, TEXT_BASE};
use memory::Memory;
use registers::Registers;
use syscall::SyscallIo;

pub struct Cpu {
    pub regs: Registers,
    pub mem: Memory,
    pub prog: Program,
    pub halted: bool,
    pub steps: u64,
    pub debug: bool,
    pub max_steps: u64,
    // I/O buffers — JS/native polls these
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdin: Vec<u8>,
    stdin_pos: usize,
    pub exit_code: i32,
}

impl Cpu {
    pub fn new(prog: Program, debug: bool, max_steps: u64) -> Result<Self, SimError> {
        let mut mem = Memory::new();
        mem.write_bytes(DATA_BASE, &prog.data, 0)?;

        let mut regs = Registers::new();
        regs.write_named(crate::ast::Register::Rsp, STACK_TOP);
        regs.rip = prog.entry;

        Ok(Cpu {
            regs,
            mem,
            prog,
            halted: false,
            steps: 0,
            debug,
            max_steps,
            stdout: Vec::new(),
            stderr: Vec::new(),
            stdin: Vec::new(),
            stdin_pos: 0,
            exit_code: 0,
        })
    }

    fn fetch(&self) -> Result<&AsmLine, SimError> {
        let rip = self.regs.rip;
        if rip < TEXT_BASE {
            return Err(SimError::Runtime {
                rip,
                msg: format!("rip {:#x} below text base", rip),
            });
        }
        let idx = (rip - TEXT_BASE) as usize;
        self.prog
            .instructions
            .get(idx)
            .ok_or_else(|| SimError::Runtime {
                rip,
                msg: format!("rip {:#x} (idx {}) out of instruction range", rip, idx),
            })
    }

    /// Execute one instruction. Returns true when the program has halted.
    pub fn step(&mut self) -> Result<bool, SimError> {
        if self.halted {
            return Ok(true);
        }

        let instr = match self.fetch()? {
            AsmLine::Instruction(i) => i.clone(),
            AsmLine::Label(_) | AsmLine::Directive(_) => {
                self.regs.rip += 1;
                return Ok(false);
            }
        };

        self.regs.rip += 1;

        if self.debug {
            self.print_regs();
            self.stderr
                .extend_from_slice(format!("  → {:?} {:?}\n", instr.op, instr.operands).as_bytes());
        }

        let mut io = SyscallIo {
            stdout: &mut self.stdout,
            stderr: &mut self.stderr,
            stdin: &self.stdin,
            stdin_pos: &mut self.stdin_pos,
            exit_code: &mut self.exit_code,
        };

        let halt = execute::execute(
            &instr,
            &mut self.regs,
            &mut self.mem,
            &self.prog.symbols,
            &mut io,
        )?;
        self.steps += 1;

        // On native targets, flush buffers to real handles immediately for live output.
        #[cfg(not(target_arch = "wasm32"))]
        self.flush_native();

        if halt {
            self.halted = true;
            return Ok(true);
        }

        if self.steps >= self.max_steps {
            return Err(SimError::Runtime {
                rip: self.regs.rip,
                msg: format!("exceeded max steps ({})", self.max_steps),
            });
        }

        Ok(false)
    }

    pub fn run(&mut self) -> Result<(), SimError> {
        loop {
            if self.step()? {
                break;
            }
        }
        Ok(())
    }

    /// Drain stdout/stderr buffers to real I/O handles (native only).
    #[cfg(not(target_arch = "wasm32"))]
    fn flush_native(&mut self) {
        use std::io::Write;
        if !self.stdout.is_empty() {
            let _ = std::io::stdout().write_all(&self.stdout);
            let _ = std::io::stdout().flush();
            self.stdout.clear();
        }
        if !self.stderr.is_empty() {
            let _ = std::io::stderr().write_all(&self.stderr);
            self.stderr.clear();
        }
    }

    /// Take all buffered stdout since the last call (used by WASM host).
    pub fn take_stdout(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.stdout)
    }

    /// Take all buffered stderr since the last call (used by WASM host).
    pub fn take_stderr(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.stderr)
    }

    pub fn print_regs(&mut self) {
        let r = &self.regs;
        let s = format!(
            "rax={:#018x} rbx={:#018x} rcx={:#018x} rdx={:#018x}\n\
             rsi={:#018x} rdi={:#018x} rsp={:#018x} rbp={:#018x}\n\
             rip={:#018x}  CF={} ZF={} SF={} OF={} PF={}\n",
            r.gpr[0],
            r.gpr[1],
            r.gpr[2],
            r.gpr[3],
            r.gpr[4],
            r.gpr[5],
            r.gpr[6],
            r.gpr[7],
            r.rip,
            r.flags.cf as u8,
            r.flags.zf as u8,
            r.flags.sf as u8,
            r.flags.of as u8,
            r.flags.pf as u8,
        );
        self.stderr.extend_from_slice(s.as_bytes());
    }
}
