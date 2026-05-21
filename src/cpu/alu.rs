use crate::cpu::registers::Flags;

fn parity(v: u64) -> bool {
    (v & 0xFF).count_ones() % 2 == 0
}

pub struct AluResult {
    pub val: u64,
    pub flags: Flags,
}

fn flags_from_result(val: u64, bits: u32) -> Flags {
    let mask = mask_for(bits);
    let masked = val & mask;
    let sign_bit = 1u64 << (bits - 1);
    Flags {
        cf: val != masked, // carry = overflow past bit width
        zf: masked == 0,
        sf: (masked & sign_bit) != 0,
        of: false, // overflow set by caller for arithmetic
        pf: parity(masked),
    }
}

fn mask_for(bits: u32) -> u64 {
    if bits >= 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    }
}

pub fn add(a: u64, b: u64, bits: u32) -> AluResult {
    let mask = mask_for(bits);
    let result_masked = a.wrapping_add(b) & mask;
    let cf = (a & mask)
        .checked_add(b & mask)
        .map_or(true, |v| v & mask != v);
    let sign_bit = 1u64 << (bits - 1);
    let sf = (result_masked & sign_bit) != 0;
    let a_sign = (a & sign_bit) != 0;
    let b_sign = (b & sign_bit) != 0;
    let of = (a_sign == b_sign) && (sf != a_sign);
    AluResult {
        val: result_masked,
        flags: Flags {
            cf,
            zf: result_masked == 0,
            sf,
            of,
            pf: parity(result_masked),
        },
    }
}

pub fn add_with_carry(a: u64, b: u64, carry: bool, bits: u32) -> AluResult {
    let c = if carry { 1u64 } else { 0 };
    let r1 = add(a, b, bits);
    let r2 = add(r1.val, c, bits);
    AluResult {
        val: r2.val,
        flags: Flags {
            cf: r1.flags.cf || r2.flags.cf,
            of: r1.flags.of || r2.flags.of,
            ..r2.flags
        },
    }
}

pub fn sub(a: u64, b: u64, bits: u32) -> AluResult {
    let mask = mask_for(bits);
    let result_masked = a.wrapping_sub(b) & mask;
    let sign_bit = 1u64 << (bits - 1);
    let cf = (a & mask) < (b & mask);
    let sf = (result_masked & sign_bit) != 0;
    let a_sign = (a & sign_bit) != 0;
    let b_sign = (b & sign_bit) != 0;
    let of = (a_sign != b_sign) && (sf != a_sign);
    AluResult {
        val: result_masked,
        flags: Flags {
            cf,
            zf: result_masked == 0,
            sf,
            of,
            pf: parity(result_masked),
        },
    }
}

pub fn sub_with_borrow(a: u64, b: u64, borrow: bool, bits: u32) -> AluResult {
    let c = if borrow { 1u64 } else { 0 };
    let r1 = sub(a, b, bits);
    let r2 = sub(r1.val, c, bits);
    AluResult {
        val: r2.val,
        flags: Flags {
            cf: r1.flags.cf || r2.flags.cf,
            of: r1.flags.of || r2.flags.of,
            ..r2.flags
        },
    }
}

pub fn and(a: u64, b: u64, bits: u32) -> AluResult {
    let mask = mask_for(bits);
    let val = (a & b) & mask;
    let mut f = flags_from_result(val, bits);
    f.cf = false;
    f.of = false;
    AluResult { val, flags: f }
}

pub fn or(a: u64, b: u64, bits: u32) -> AluResult {
    let mask = mask_for(bits);
    let val = (a | b) & mask;
    let mut f = flags_from_result(val, bits);
    f.cf = false;
    f.of = false;
    AluResult { val, flags: f }
}

pub fn xor(a: u64, b: u64, bits: u32) -> AluResult {
    let mask = mask_for(bits);
    let val = (a ^ b) & mask;
    let mut f = flags_from_result(val, bits);
    f.cf = false;
    f.of = false;
    AluResult { val, flags: f }
}

pub fn shl(a: u64, count: u64, bits: u32) -> AluResult {
    let mask = mask_for(bits);
    let count = count & 63;
    if count == 0 {
        let val = a & mask;
        return AluResult {
            val,
            flags: flags_from_result(val, bits),
        };
    }
    let cf = if count <= bits as u64 {
        ((a >> (bits as u64 - count)) & 1) != 0
    } else {
        false
    };
    let val = a.checked_shl(count as u32).unwrap_or(0) & mask;
    let sign_bit = 1u64 << (bits - 1);
    let sf = (val & sign_bit) != 0;
    let of = if count == 1 { sf != cf } else { false };
    AluResult {
        val,
        flags: Flags {
            cf,
            zf: val == 0,
            sf,
            of,
            pf: parity(val),
        },
    }
}

pub fn shr(a: u64, count: u64, bits: u32) -> AluResult {
    let mask = mask_for(bits);
    let a = a & mask;
    let count = count & 63;
    if count == 0 {
        return AluResult {
            val: a,
            flags: flags_from_result(a, bits),
        };
    }
    let cf = if count <= bits as u64 {
        ((a >> (count - 1)) & 1) != 0
    } else {
        false
    };
    let val = a.checked_shr(count as u32).unwrap_or(0);
    let sign_bit = 1u64 << (bits - 1);
    let sf = (val & sign_bit) != 0;
    let of = if count == 1 {
        (a & sign_bit) != 0
    } else {
        false
    };
    AluResult {
        val,
        flags: Flags {
            cf,
            zf: val == 0,
            sf,
            of,
            pf: parity(val),
        },
    }
}

pub fn sar(a: u64, count: u64, bits: u32) -> AluResult {
    let mask = mask_for(bits);
    let sign_bit = 1u64 << (bits - 1);
    // Sign-extend a to i64 for arithmetic shift
    let signed: i64 = if (a & sign_bit) != 0 {
        (a | !mask) as i64
    } else {
        (a & mask) as i64
    };
    let count = count & 63;
    if count == 0 {
        let val = a & mask;
        return AluResult {
            val,
            flags: flags_from_result(val, bits),
        };
    }
    let cf = if count <= bits as u64 {
        ((signed >> (count - 1)) & 1) != 0
    } else {
        signed < 0
    };
    let val = (signed >> count.min(63)) as u64 & mask;
    let sf = (val & sign_bit) != 0;
    AluResult {
        val,
        flags: Flags {
            cf,
            zf: val == 0,
            sf,
            of: false,
            pf: parity(val),
        },
    }
}

pub fn rol(a: u64, count: u64, bits: u32) -> AluResult {
    let mask = mask_for(bits);
    let a = a & mask;
    let count = (count & 63) % bits as u64;
    let val = if count == 0 {
        a
    } else {
        ((a << count) | (a >> (bits as u64 - count))) & mask
    };
    let cf = val & 1 != 0;
    let sign_bit = 1u64 << (bits - 1);
    let of = if count == 1 {
        cf != ((val & sign_bit) != 0)
    } else {
        false
    };
    AluResult {
        val,
        flags: Flags {
            cf,
            zf: val == 0,
            sf: (val & sign_bit) != 0,
            of,
            pf: parity(val),
        },
    }
}

pub fn ror(a: u64, count: u64, bits: u32) -> AluResult {
    let mask = mask_for(bits);
    let a = a & mask;
    let count = (count & 63) % bits as u64;
    let val = if count == 0 {
        a
    } else {
        ((a >> count) | (a << (bits as u64 - count))) & mask
    };
    let sign_bit = 1u64 << (bits - 1);
    let cf = (val & sign_bit) != 0;
    let of = if count == 1 {
        (val & sign_bit) != ((val << 1) & sign_bit)
    } else {
        false
    };
    AluResult {
        val,
        flags: Flags {
            cf,
            zf: val == 0,
            sf: (val & sign_bit) != 0,
            of,
            pf: parity(val),
        },
    }
}

pub fn neg(a: u64, bits: u32) -> AluResult {
    let mask = mask_for(bits);
    let val = (0u64.wrapping_sub(a)) & mask;
    let sign_bit = 1u64 << (bits - 1);
    let cf = val != 0;
    let sf = (val & sign_bit) != 0;
    let of = val == sign_bit;
    AluResult {
        val,
        flags: Flags {
            cf,
            zf: val == 0,
            sf,
            of,
            pf: parity(val),
        },
    }
}

pub fn bits_for_width(w: crate::ast::RegWidth) -> u32 {
    match w {
        crate::ast::RegWidth::Qword => 64,
        crate::ast::RegWidth::Dword => 32,
        crate::ast::RegWidth::Word => 16,
        crate::ast::RegWidth::Byte | crate::ast::RegWidth::ByteHi => 8,
    }
}

pub fn bits_for_size(s: crate::ast::Size) -> u32 {
    match s {
        crate::ast::Size::Byte => 8,
        crate::ast::Size::Word => 16,
        crate::ast::Size::Dword => 32,
        crate::ast::Size::Qword => 64,
    }
}
