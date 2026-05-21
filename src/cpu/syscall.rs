use crate::ast::Register;
use crate::cpu::memory::Memory;
use crate::cpu::registers::Registers;
use crate::error::SimError;

pub struct SyscallIo<'a> {
    pub stdout: &'a mut Vec<u8>,
    pub stderr: &'a mut Vec<u8>,
    pub stdin: &'a [u8],
    pub stdin_pos: &'a mut usize,
    pub exit_code: &'a mut i32,
}

/// Returns true if the CPU should halt (exit syscall).
pub fn dispatch(
    regs: &mut Registers,
    mem: &mut Memory,
    io: &mut SyscallIo<'_>,
) -> Result<bool, SimError> {
    let rip = regs.rip;
    let rax = regs.read_named(Register::Rax);
    match rax {
        // read(fd, buf, count)
        0 => {
            let fd = regs.read_named(Register::Rdi);
            let buf = regs.read_named(Register::Rsi);
            let len = regs.read_named(Register::Rdx) as usize;
            if fd != 0 {
                regs.write_named(Register::Rax, u64::MAX);
                return Ok(false);
            }
            let available = io.stdin.len().saturating_sub(*io.stdin_pos);
            let n = len.min(available);
            let slice = &io.stdin[*io.stdin_pos..*io.stdin_pos + n];
            mem.write_bytes(buf, slice, rip)?;
            *io.stdin_pos += n;
            regs.write_named(Register::Rax, n as u64);
            Ok(false)
        }
        // write(fd, buf, count)
        1 | 2 => {
            let fd = regs.read_named(Register::Rdi);
            let buf = regs.read_named(Register::Rsi);
            let len = regs.read_named(Register::Rdx) as usize;
            let bytes = mem.read_bytes(buf, len, rip)?.to_vec();
            let n = bytes.len();
            match fd {
                1 => io.stdout.extend_from_slice(&bytes),
                2 => io.stderr.extend_from_slice(&bytes),
                _ => {
                    regs.write_named(Register::Rax, u64::MAX);
                    return Ok(false);
                }
            }
            regs.write_named(Register::Rax, n as u64);
            Ok(false)
        }
        // exit(code)
        60 => {
            *io.exit_code = regs.read_named(Register::Rdi) as i32;
            Ok(true)
        }
        _ => Err(SimError::Runtime {
            rip,
            msg: format!("unknown syscall: rax={}", rax),
        }),
    }
}
