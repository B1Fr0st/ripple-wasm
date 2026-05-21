use crate::ast::{RegRef, RegWidth, Register};

#[derive(Debug, Default, Clone)]
pub struct Flags {
    pub cf: bool,
    pub zf: bool,
    pub sf: bool,
    pub of: bool,
    pub pf: bool,
}

#[derive(Debug, Clone)]
pub struct Registers {
    pub gpr: [u64; 16],
    pub rip: u64,
    pub flags: Flags,
}

impl Registers {
    pub fn new() -> Self {
        Registers {
            gpr: [0u64; 16],
            rip: 0,
            flags: Flags::default(),
        }
    }

    pub fn read(&self, r: RegRef) -> u64 {
        let raw = self.gpr[r.reg.index()];
        match r.width {
            RegWidth::Qword => raw,
            RegWidth::Dword => raw & 0xFFFF_FFFF,
            RegWidth::Word => raw & 0xFFFF,
            RegWidth::Byte => raw & 0xFF,
            RegWidth::ByteHi => (raw >> 8) & 0xFF,
        }
    }

    pub fn write(&mut self, r: RegRef, val: u64) {
        let slot = &mut self.gpr[r.reg.index()];
        match r.width {
            RegWidth::Qword => *slot = val,
            // 32-bit writes zero-extend to 64 bits (x86_64 semantics)
            RegWidth::Dword => *slot = val & 0xFFFF_FFFF,
            RegWidth::Word => *slot = (*slot & !0xFFFF) | (val & 0xFFFF),
            RegWidth::Byte => *slot = (*slot & !0xFF) | (val & 0xFF),
            RegWidth::ByteHi => *slot = (*slot & !0xFF00) | ((val & 0xFF) << 8),
        }
    }

    pub fn read_named(&self, r: Register) -> u64 {
        self.gpr[r.index()]
    }

    pub fn write_named(&mut self, r: Register, val: u64) {
        self.gpr[r.index()] = val;
    }
}
