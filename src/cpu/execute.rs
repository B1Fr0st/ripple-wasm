use crate::ast::*;
use crate::error::SimError;
use crate::cpu::registers::Registers;
use crate::cpu::memory::Memory;
use crate::cpu::alu::{self, bits_for_width, bits_for_size};
use crate::cpu::syscall::SyscallIo;
use std::collections::HashMap;

fn resolve_label(name: &str, symbols: &HashMap<String, i64>, rip: u64) -> Result<u64, SimError> {
    symbols.get(name).copied().map(|v| v as u64).ok_or_else(|| SimError::Runtime {
        rip,
        msg: format!("undefined symbol '{}'", name),
    })
}

/// Compute effective address from MemAddr
fn effective_addr(
    m: &MemAddr,
    regs: &Registers,
    symbols: &HashMap<String, i64>,
    rip: u64,
) -> Result<u64, SimError> {
    let label_base = if let Some(name) = &m.label {
        resolve_label(name, symbols, rip)? as i64
    } else {
        0i64
    };
    let base = m.base.map(|r| regs.read(r) as i64).unwrap_or(0);
    let idx  = m.index.map(|r| (regs.read(r) * m.scale as u64) as i64).unwrap_or(0);
    Ok((label_base + base + idx + m.disp) as u64)
}

/// Read a value from a register or immediate or memory
pub fn read_operand(
    op: &Operand,
    regs: &Registers,
    mem: &Memory,
    symbols: &HashMap<String, i64>,
    default_width: RegWidth,
) -> Result<u64, SimError> {
    let rip = regs.rip;
    match op {
        Operand::Register(r) => Ok(regs.read(*r)),
        Operand::Immediate(n) => Ok(*n as u64),
        Operand::Label(name) => resolve_label(name, symbols, rip),
        Operand::Memory(m) => {
            let addr = effective_addr(m, regs, symbols, rip)?;
            let size = m.size.unwrap_or_else(|| width_to_size(default_width));
            match size {
                Size::Byte  => mem.read_u8(addr, rip).map(|v| v as u64),
                Size::Word  => mem.read_u16(addr, rip).map(|v| v as u64),
                Size::Dword => mem.read_u32(addr, rip).map(|v| v as u64),
                Size::Qword => mem.read_u64(addr, rip),
            }
        }
    }
}

fn width_to_size(w: RegWidth) -> Size {
    match w {
        RegWidth::Byte | RegWidth::ByteHi => Size::Byte,
        RegWidth::Word   => Size::Word,
        RegWidth::Dword  => Size::Dword,
        RegWidth::Qword  => Size::Qword,
    }
}

/// Determine the bit-width of an operand pair (left operand is authoritative for registers)
fn operand_bits(dst: &Operand, src: &Operand) -> u32 {
    match dst {
        Operand::Register(r) => bits_for_width(r.width),
        Operand::Memory(m) => m.size.map(bits_for_size).unwrap_or_else(|| {
            match src {
                Operand::Register(r) => bits_for_width(r.width),
                _ => 64,
            }
        }),
        _ => 64,
    }
}

/// Write value back to a register or memory destination
fn write_dst(
    op: &Operand,
    val: u64,
    regs: &mut Registers,
    mem: &mut Memory,
    symbols: &HashMap<String, i64>,
    default_width: RegWidth,
) -> Result<(), SimError> {
    let rip = regs.rip;
    match op {
        Operand::Register(r) => { regs.write(*r, val); Ok(()) }
        Operand::Memory(m) => {
            let addr = effective_addr(m, regs, symbols, rip)?;
            let size = m.size.unwrap_or_else(|| width_to_size(default_width));
            match size {
                Size::Byte  => mem.write_u8(addr, val as u8, rip),
                Size::Word  => mem.write_u16(addr, val as u16, rip),
                Size::Dword => mem.write_u32(addr, val as u32, rip),
                Size::Qword => mem.write_u64(addr, val, rip),
            }
        }
        _ => Err(SimError::Runtime { rip, msg: "invalid destination operand".into() }),
    }
}

fn default_width_of(op: &Operand) -> RegWidth {
    match op {
        Operand::Register(r) => r.width,
        _ => RegWidth::Qword,
    }
}

fn jump_to(target: &Operand, regs: &mut Registers, symbols: &HashMap<String, i64>) -> Result<(), SimError> {
    let rip = regs.rip;
    let addr = match target {
        Operand::Label(name) => resolve_label(name, symbols, rip)?,
        Operand::Register(r) => regs.read(*r),
        Operand::Immediate(n) => *n as u64,
        op => return Err(SimError::Runtime { rip, msg: format!("invalid jump target: {:?}", op) }),
    };
    // Convert virtual address to instruction index
    regs.rip = addr;
    Ok(())
}

pub fn execute(
    instr: &Instruction,
    regs: &mut Registers,
    mem: &mut Memory,
    symbols: &HashMap<String, i64>,
    io: &mut SyscallIo<'_>,
) -> Result<bool, SimError> {
    let rip = regs.rip;
    let ops = &instr.operands;

    macro_rules! require {
        ($n:expr) => {
            if ops.len() < $n {
                return Err(SimError::Runtime {
                    rip,
                    msg: format!("{:?} requires {} operand(s), got {}", instr.op, $n, ops.len()),
                });
            }
        };
    }

    macro_rules! dst_src_bits {
        () => {{
            require!(2);
            let bits = operand_bits(&ops[0], &ops[1]);
            let dw = default_width_of(&ops[0]);
            let src = read_operand(&ops[1], regs, mem, symbols, dw)?;
            let dst = read_operand(&ops[0], regs, mem, symbols, dw)?;
            (dst, src, bits, dw)
        }};
    }

    match &instr.op {
        Op::Mov => {
            require!(2);
            let dw = default_width_of(&ops[0]);
            let val = read_operand(&ops[1], regs, mem, symbols, dw)?;
            write_dst(&ops[0], val, regs, mem, symbols, dw)?;
        }

        Op::Movzx => {
            require!(2);
            let dw = default_width_of(&ops[1]);
            let val = read_operand(&ops[1], regs, mem, symbols, dw)?;
            // Zero-extend — just write to dst (register write handles width)
            let dw_dst = default_width_of(&ops[0]);
            write_dst(&ops[0], val, regs, mem, symbols, dw_dst)?;
        }

        Op::Movsx => {
            require!(2);
            let src_dw = default_width_of(&ops[1]);
            let src_bits = bits_for_width(src_dw);
            let val = read_operand(&ops[1], regs, mem, symbols, src_dw)?;
            // Sign-extend
            let sign_bit = 1u64 << (src_bits - 1);
            let extended = if val & sign_bit != 0 {
                val | !((1u64 << src_bits) - 1)
            } else {
                val
            };
            let dw_dst = default_width_of(&ops[0]);
            write_dst(&ops[0], extended, regs, mem, symbols, dw_dst)?;
        }

        Op::Lea => {
            require!(2);
            let addr = match &ops[1] {
                Operand::Memory(m) => effective_addr(m, regs, symbols, rip)?,
                Operand::Label(name) => resolve_label(name, symbols, rip)?,
                _ => return Err(SimError::Runtime { rip, msg: "lea requires memory operand".into() }),
            };
            let dw = default_width_of(&ops[0]);
            write_dst(&ops[0], addr, regs, mem, symbols, dw)?;
        }

        Op::Push => {
            require!(1);
            let dw = default_width_of(&ops[0]);
            let val = read_operand(&ops[0], regs, mem, symbols, dw)?;
            let rsp = regs.read_named(Register::Rsp).wrapping_sub(8);
            regs.write_named(Register::Rsp, rsp);
            mem.write_u64(rsp, val, rip)?;
        }

        Op::Pop => {
            require!(1);
            let rsp = regs.read_named(Register::Rsp);
            let val = mem.read_u64(rsp, rip)?;
            regs.write_named(Register::Rsp, rsp.wrapping_add(8));
            let dw = default_width_of(&ops[0]);
            write_dst(&ops[0], val, regs, mem, symbols, dw)?;
        }

        Op::Xchg => {
            require!(2);
            let dw = default_width_of(&ops[0]);
            let a = read_operand(&ops[0], regs, mem, symbols, dw)?;
            let b = read_operand(&ops[1], regs, mem, symbols, dw)?;
            write_dst(&ops[0], b, regs, mem, symbols, dw)?;
            write_dst(&ops[1], a, regs, mem, symbols, dw)?;
        }

        Op::Add => {
            let (dst, src, bits, dw) = dst_src_bits!();
            let r = alu::add(dst, src, bits);
            regs.flags = r.flags;
            write_dst(&ops[0], r.val, regs, mem, symbols, dw)?;
        }

        Op::Adc => {
            let (dst, src, bits, dw) = dst_src_bits!();
            let r = alu::add_with_carry(dst, src, regs.flags.cf, bits);
            regs.flags = r.flags;
            write_dst(&ops[0], r.val, regs, mem, symbols, dw)?;
        }

        Op::Sub => {
            let (dst, src, bits, dw) = dst_src_bits!();
            let r = alu::sub(dst, src, bits);
            regs.flags = r.flags;
            write_dst(&ops[0], r.val, regs, mem, symbols, dw)?;
        }

        Op::Sbb => {
            let (dst, src, bits, dw) = dst_src_bits!();
            let r = alu::sub_with_borrow(dst, src, regs.flags.cf, bits);
            regs.flags = r.flags;
            write_dst(&ops[0], r.val, regs, mem, symbols, dw)?;
        }

        Op::Inc => {
            require!(1);
            let dw = default_width_of(&ops[0]);
            let bits = bits_for_width(dw);
            let val = read_operand(&ops[0], regs, mem, symbols, dw)?;
            let r = alu::add(val, 1, bits);
            // INC does not modify CF
            let cf_saved = regs.flags.cf;
            regs.flags = r.flags;
            regs.flags.cf = cf_saved;
            write_dst(&ops[0], r.val, regs, mem, symbols, dw)?;
        }

        Op::Dec => {
            require!(1);
            let dw = default_width_of(&ops[0]);
            let bits = bits_for_width(dw);
            let val = read_operand(&ops[0], regs, mem, symbols, dw)?;
            let r = alu::sub(val, 1, bits);
            let cf_saved = regs.flags.cf;
            regs.flags = r.flags;
            regs.flags.cf = cf_saved;
            write_dst(&ops[0], r.val, regs, mem, symbols, dw)?;
        }

        Op::Neg => {
            require!(1);
            let dw = default_width_of(&ops[0]);
            let bits = bits_for_width(dw);
            let val = read_operand(&ops[0], regs, mem, symbols, dw)?;
            let r = alu::neg(val, bits);
            regs.flags = r.flags;
            write_dst(&ops[0], r.val, regs, mem, symbols, dw)?;
        }

        Op::Mul => {
            // unsigned: rdx:rax = rax * src
            require!(1);
            let dw = default_width_of(&ops[0]);
            let src = read_operand(&ops[0], regs, mem, symbols, dw)?;
            let rax = regs.read_named(Register::Rax);
            let result = rax as u128 * src as u128;
            let lo = result as u64;
            let hi = (result >> 64) as u64;
            regs.write_named(Register::Rax, lo);
            regs.write_named(Register::Rdx, hi);
            regs.flags.cf = hi != 0;
            regs.flags.of = hi != 0;
            regs.flags.zf = lo == 0 && hi == 0;
            regs.flags.sf = (lo & (1 << 63)) != 0;
        }

        Op::Imul => {
            match ops.len() {
                1 => {
                    // rax * src → rdx:rax (signed)
                    let dw = default_width_of(&ops[0]);
                    let src = read_operand(&ops[0], regs, mem, symbols, dw)? as i64;
                    let rax = regs.read_named(Register::Rax) as i64;
                    let result = rax as i128 * src as i128;
                    let lo = result as u64;
                    let hi = (result >> 64) as u64;
                    regs.write_named(Register::Rax, lo);
                    regs.write_named(Register::Rdx, hi);
                    let overflow = hi != 0 && hi != u64::MAX;
                    regs.flags.cf = overflow;
                    regs.flags.of = overflow;
                }
                2 => {
                    let dw = default_width_of(&ops[0]);
                    let dst = read_operand(&ops[0], regs, mem, symbols, dw)? as i64;
                    let src = read_operand(&ops[1], regs, mem, symbols, dw)? as i64;
                    let result = (dst as i128 * src as i128) as u64;
                    // truncated result, set OF/CF if overflow
                    let expected = dst.wrapping_mul(src);
                    let overflow = expected as i128 != (dst as i128 * src as i128);
                    regs.flags.cf = overflow;
                    regs.flags.of = overflow;
                    write_dst(&ops[0], result, regs, mem, symbols, dw)?;
                }
                3 => {
                    let dw = default_width_of(&ops[0]);
                    let src = read_operand(&ops[1], regs, mem, symbols, dw)? as i64;
                    let imm = read_operand(&ops[2], regs, mem, symbols, dw)? as i64;
                    let result = (src as i128 * imm as i128) as u64;
                    let overflow = (src as i128 * imm as i128) != (src.wrapping_mul(imm) as i128);
                    regs.flags.cf = overflow;
                    regs.flags.of = overflow;
                    write_dst(&ops[0], result, regs, mem, symbols, dw)?;
                }
                _ => return Err(SimError::Runtime { rip, msg: "imul: invalid operand count".into() }),
            }
        }

        Op::Div => {
            require!(1);
            let dw = default_width_of(&ops[0]);
            let src = read_operand(&ops[0], regs, mem, symbols, dw)?;
            if src == 0 {
                return Err(SimError::Runtime { rip, msg: "division by zero".into() });
            }
            let dividend_lo = regs.read_named(Register::Rax);
            let dividend_hi = regs.read_named(Register::Rdx);
            let dividend = (dividend_hi as u128) << 64 | dividend_lo as u128;
            let quot = dividend / src as u128;
            let rem  = dividend % src as u128;
            if quot > u64::MAX as u128 {
                return Err(SimError::Runtime { rip, msg: "div overflow".into() });
            }
            regs.write_named(Register::Rax, quot as u64);
            regs.write_named(Register::Rdx, rem as u64);
        }

        Op::Idiv => {
            require!(1);
            let dw = default_width_of(&ops[0]);
            let src = read_operand(&ops[0], regs, mem, symbols, dw)? as i64;
            if src == 0 {
                return Err(SimError::Runtime { rip, msg: "division by zero".into() });
            }
            let dividend_lo = regs.read_named(Register::Rax);
            let dividend_hi = regs.read_named(Register::Rdx) as i64;
            let dividend = ((dividend_hi as i128) << 64) | (dividend_lo as i128);
            let quot = dividend / src as i128;
            let rem  = dividend % src as i128;
            if quot > i64::MAX as i128 || quot < i64::MIN as i128 {
                return Err(SimError::Runtime { rip, msg: "idiv overflow".into() });
            }
            regs.write_named(Register::Rax, quot as u64);
            regs.write_named(Register::Rdx, rem as u64);
        }

        Op::And => {
            let (dst, src, bits, dw) = dst_src_bits!();
            let r = alu::and(dst, src, bits);
            regs.flags = r.flags;
            write_dst(&ops[0], r.val, regs, mem, symbols, dw)?;
        }

        Op::Or => {
            let (dst, src, bits, dw) = dst_src_bits!();
            let r = alu::or(dst, src, bits);
            regs.flags = r.flags;
            write_dst(&ops[0], r.val, regs, mem, symbols, dw)?;
        }

        Op::Xor => {
            let (dst, src, bits, dw) = dst_src_bits!();
            let r = alu::xor(dst, src, bits);
            regs.flags = r.flags;
            write_dst(&ops[0], r.val, regs, mem, symbols, dw)?;
        }

        Op::Not => {
            require!(1);
            let dw = default_width_of(&ops[0]);
            let bits = bits_for_width(dw);
            let val = read_operand(&ops[0], regs, mem, symbols, dw)?;
            let mask = if bits >= 64 { u64::MAX } else { (1u64 << bits) - 1 };
            write_dst(&ops[0], (!val) & mask, regs, mem, symbols, dw)?;
        }

        Op::Shl => {
            let (dst, src, bits, dw) = dst_src_bits!();
            let r = alu::shl(dst, src, bits);
            regs.flags = r.flags;
            write_dst(&ops[0], r.val, regs, mem, symbols, dw)?;
        }

        Op::Shr => {
            let (dst, src, bits, dw) = dst_src_bits!();
            let r = alu::shr(dst, src, bits);
            regs.flags = r.flags;
            write_dst(&ops[0], r.val, regs, mem, symbols, dw)?;
        }

        Op::Sar => {
            let (dst, src, bits, dw) = dst_src_bits!();
            let r = alu::sar(dst, src, bits);
            regs.flags = r.flags;
            write_dst(&ops[0], r.val, regs, mem, symbols, dw)?;
        }

        Op::Rol => {
            let (dst, src, bits, dw) = dst_src_bits!();
            let r = alu::rol(dst, src, bits);
            regs.flags = r.flags;
            write_dst(&ops[0], r.val, regs, mem, symbols, dw)?;
        }

        Op::Ror => {
            let (dst, src, bits, dw) = dst_src_bits!();
            let r = alu::ror(dst, src, bits);
            regs.flags = r.flags;
            write_dst(&ops[0], r.val, regs, mem, symbols, dw)?;
        }

        Op::Cmp => {
            let (dst, src, bits, _) = dst_src_bits!();
            let r = alu::sub(dst, src, bits);
            regs.flags = r.flags;
        }

        Op::Test => {
            let (dst, src, bits, _) = dst_src_bits!();
            let r = alu::and(dst, src, bits);
            regs.flags = r.flags;
        }

        Op::Jmp => {
            require!(1);
            jump_to(&ops[0], regs, symbols)?;
            return Ok(false);
        }

        Op::Je  => { require!(1); if regs.flags.zf                              { jump_to(&ops[0], regs, symbols)?; return Ok(false); } }
        Op::Jne => { require!(1); if !regs.flags.zf                             { jump_to(&ops[0], regs, symbols)?; return Ok(false); } }
        Op::Jl  => { require!(1); if regs.flags.sf != regs.flags.of             { jump_to(&ops[0], regs, symbols)?; return Ok(false); } }
        Op::Jle => { require!(1); if regs.flags.zf || regs.flags.sf != regs.flags.of { jump_to(&ops[0], regs, symbols)?; return Ok(false); } }
        Op::Jg  => { require!(1); if !regs.flags.zf && regs.flags.sf == regs.flags.of { jump_to(&ops[0], regs, symbols)?; return Ok(false); } }
        Op::Jge => { require!(1); if regs.flags.sf == regs.flags.of             { jump_to(&ops[0], regs, symbols)?; return Ok(false); } }
        Op::Js  => { require!(1); if regs.flags.sf                              { jump_to(&ops[0], regs, symbols)?; return Ok(false); } }
        Op::Jns => { require!(1); if !regs.flags.sf                             { jump_to(&ops[0], regs, symbols)?; return Ok(false); } }
        Op::Jc  => { require!(1); if regs.flags.cf                              { jump_to(&ops[0], regs, symbols)?; return Ok(false); } }
        Op::Jnc => { require!(1); if !regs.flags.cf                             { jump_to(&ops[0], regs, symbols)?; return Ok(false); } }
        Op::Jo  => { require!(1); if regs.flags.of                              { jump_to(&ops[0], regs, symbols)?; return Ok(false); } }
        Op::Jno => { require!(1); if !regs.flags.of                             { jump_to(&ops[0], regs, symbols)?; return Ok(false); } }

        Op::Call => {
            require!(1);
            let ret_addr = regs.rip; // already points to next instruction
            let rsp = regs.read_named(Register::Rsp).wrapping_sub(8);
            regs.write_named(Register::Rsp, rsp);
            mem.write_u64(rsp, ret_addr, rip)?;
            jump_to(&ops[0], regs, symbols)?;
            return Ok(false);
        }

        Op::Ret => {
            let rsp = regs.read_named(Register::Rsp);
            let ret_addr = mem.read_u64(rsp, rip)?;
            regs.write_named(Register::Rsp, rsp.wrapping_add(8));
            if let Some(imm) = ops.first() {
                let extra = read_operand(imm, regs, mem, symbols, RegWidth::Qword)?;
                let rsp = regs.read_named(Register::Rsp);
                regs.write_named(Register::Rsp, rsp.wrapping_add(extra));
            }
            regs.rip = ret_addr;
            return Ok(false);
        }

        Op::Loop => {
            require!(1);
            let rcx = regs.read_named(Register::Rcx).wrapping_sub(1);
            regs.write_named(Register::Rcx, rcx);
            if rcx != 0 {
                jump_to(&ops[0], regs, symbols)?;
                return Ok(false);
            }
        }

        Op::Syscall => {
            let halt = crate::cpu::syscall::dispatch(regs, mem, io)?;
            if halt { return Ok(true); }
        }

        Op::Equ => {} // resolved at assembly time
    }

    Ok(false)
}
