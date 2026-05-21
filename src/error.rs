use std::fmt;

#[derive(Debug)]
pub enum SimError {
    Lex {
        line: usize,
        col: usize,
        msg: String,
    },
    Parse {
        line: usize,
        msg: String,
    },
    Assemble {
        msg: String,
    },
    Runtime {
        rip: u64,
        msg: String,
    },
}

impl fmt::Display for SimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SimError::Lex { line, col, msg } => write!(f, "lex error at {}:{}: {}", line, col, msg),
            SimError::Parse { line, msg } => write!(f, "parse error at line {}: {}", line, msg),
            SimError::Assemble { msg } => write!(f, "assemble error: {}", msg),
            SimError::Runtime { rip, msg } => write!(f, "runtime error at rip={:#x}: {}", rip, msg),
        }
    }
}

impl std::error::Error for SimError {}
