use std::collections::HashMap;
use crate::ast::*;
use crate::error::SimError;
use crate::program::{Program, TEXT_BASE, DATA_BASE};

pub struct Assembler {
    lines: Vec<AsmLine>,
}

impl Assembler {
    pub fn new(lines: Vec<AsmLine>) -> Self {
        Assembler { lines }
    }

    pub fn assemble(self) -> Result<Program, SimError> {
        // Separate data and text, collect labels and equ constants.
        // Two-pass over AsmLine: pass 1 assigns addresses, pass 2 validates.

        let mut symbols: HashMap<String, i64> = HashMap::new();
        let mut data: Vec<u8> = Vec::new();
        let mut instructions: Vec<AsmLine> = Vec::new();

        let mut in_data = false;
        let mut entry: Option<u64> = None;

        // Pass 1: build symbol table and data section, collect instructions
        for line in &self.lines {
            match line {
                AsmLine::Directive(Directive::Section(name)) => {
                    in_data = name == ".data" || name == "data";
                }
                AsmLine::Directive(Directive::Global(_)) => {}
                AsmLine::Directive(Directive::Equ(name, val)) => {
                    symbols.insert(name.clone(), *val);
                }
                AsmLine::Label(name) => {
                    let addr = if in_data {
                        (DATA_BASE + data.len() as u64) as i64
                    } else {
                        (TEXT_BASE + instructions.len() as u64) as i64
                    };
                    if symbols.insert(name.clone(), addr).is_some() {
                        return Err(SimError::Assemble {
                            msg: format!("duplicate label '{}'", name),
                        });
                    }
                    if name == "_start" {
                        entry = Some(addr as u64);
                    }
                }
                AsmLine::Directive(d) if in_data => {
                    emit_data(d, &mut data)?;
                }
                AsmLine::Instruction(_) => {
                    instructions.push(line.clone());
                }
                _ => {} // directives in text section outside of data are ignored
            }
        }

        // Resolve $ (current position) pseudo-labels — handled at parse time via equ
        // Validate that all label references in instructions exist
        for line in &instructions {
            if let AsmLine::Instruction(instr) = line {
                for op in &instr.operands {
                    let name = match op {
                        Operand::Label(n) => Some(n.as_str()),
                        Operand::Memory(m) => m.label.as_deref(),
                        _ => None,
                    };
                    if let Some(n) = name {
                        if !symbols.contains_key(n) {
                            return Err(SimError::Assemble {
                                msg: format!("undefined symbol '{}' at line {}", n, instr.line),
                            });
                        }
                    }
                }
            }
        }

        let entry = entry.unwrap_or(TEXT_BASE);

        Ok(Program { instructions, symbols, data, entry })
    }
}

fn emit_data(d: &Directive, out: &mut Vec<u8>) -> Result<(), SimError> {
    match d {
        Directive::Db(bytes) => out.extend_from_slice(bytes),
        Directive::Dw(words) => {
            for w in words { out.extend_from_slice(&w.to_le_bytes()); }
        }
        Directive::Dd(dwords) => {
            for d in dwords { out.extend_from_slice(&d.to_le_bytes()); }
        }
        Directive::Dq(qwords) => {
            for q in qwords { out.extend_from_slice(&q.to_le_bytes()); }
        }
        Directive::Times { count, inner } => {
            for _ in 0..*count {
                emit_data(inner, out)?;
            }
        }
        Directive::Section(_) | Directive::Global(_) | Directive::Equ(_, _) => {}
    }
    Ok(())
}
