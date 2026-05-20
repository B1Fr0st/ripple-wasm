use crate::error::SimError;
use crate::program::{DATA_BASE, TEXT_BASE, STACK_TOP, MEM_SIZE};

pub struct Memory {
    pub data: Vec<u8>,
    pub text_start: u64,
    pub data_start: u64,
    pub stack_start: u64,
}

impl Memory {
    pub fn new() -> Self {
        Memory {
            data: vec![0u8; MEM_SIZE],
            text_start: TEXT_BASE,
            data_start: DATA_BASE,
            stack_start: STACK_TOP,
        }
    }

    fn check(&self, addr: u64, len: u64, write: bool, rip: u64) -> Result<(), SimError> {
        let addr_end = addr.saturating_add(len);
        if addr_end as usize > MEM_SIZE {
            return Err(SimError::Runtime {
                rip,
                msg: format!("segfault: address {:#x} out of bounds", addr),
            });
        }
        if write && addr < DATA_BASE && addr >= TEXT_BASE {
            return Err(SimError::Runtime {
                rip,
                msg: format!("segfault: write to text segment at {:#x}", addr),
            });
        }
        Ok(())
    }

    pub fn read_u8(&self, addr: u64, rip: u64) -> Result<u8, SimError> {
        self.check(addr, 1, false, rip)?;
        Ok(self.data[addr as usize])
    }

    pub fn read_u16(&self, addr: u64, rip: u64) -> Result<u16, SimError> {
        self.check(addr, 2, false, rip)?;
        let i = addr as usize;
        Ok(u16::from_le_bytes([self.data[i], self.data[i+1]]))
    }

    pub fn read_u32(&self, addr: u64, rip: u64) -> Result<u32, SimError> {
        self.check(addr, 4, false, rip)?;
        let i = addr as usize;
        Ok(u32::from_le_bytes(self.data[i..i+4].try_into().unwrap()))
    }

    pub fn read_u64(&self, addr: u64, rip: u64) -> Result<u64, SimError> {
        self.check(addr, 8, false, rip)?;
        let i = addr as usize;
        Ok(u64::from_le_bytes(self.data[i..i+8].try_into().unwrap()))
    }

    pub fn write_u8(&mut self, addr: u64, val: u8, rip: u64) -> Result<(), SimError> {
        self.check(addr, 1, true, rip)?;
        self.data[addr as usize] = val;
        Ok(())
    }

    pub fn write_u16(&mut self, addr: u64, val: u16, rip: u64) -> Result<(), SimError> {
        self.check(addr, 2, true, rip)?;
        let i = addr as usize;
        self.data[i..i+2].copy_from_slice(&val.to_le_bytes());
        Ok(())
    }

    pub fn write_u32(&mut self, addr: u64, val: u32, rip: u64) -> Result<(), SimError> {
        self.check(addr, 4, true, rip)?;
        let i = addr as usize;
        self.data[i..i+4].copy_from_slice(&val.to_le_bytes());
        Ok(())
    }

    pub fn write_u64(&mut self, addr: u64, val: u64, rip: u64) -> Result<(), SimError> {
        self.check(addr, 8, true, rip)?;
        let i = addr as usize;
        self.data[i..i+8].copy_from_slice(&val.to_le_bytes());
        Ok(())
    }

    pub fn write_bytes(&mut self, addr: u64, bytes: &[u8], rip: u64) -> Result<(), SimError> {
        self.check(addr, bytes.len() as u64, true, rip)?;
        let i = addr as usize;
        self.data[i..i + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }

    pub fn read_bytes(&self, addr: u64, len: usize, rip: u64) -> Result<&[u8], SimError> {
        self.check(addr, len as u64, false, rip)?;
        let i = addr as usize;
        Ok(&self.data[i..i + len])
    }
}
