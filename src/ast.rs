#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Register {
    Rax, Rbx, Rcx, Rdx, Rsi, Rdi, Rsp, Rbp,
    R8, R9, R10, R11, R12, R13, R14, R15,
}

impl Register {
    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegWidth {
    Byte,   // 8-bit low  (al, bl, ...)
    ByteHi, // 8-bit high (ah, bh, ...) — only rax/rbx/rcx/rdx
    Word,   // 16-bit
    Dword,  // 32-bit
    Qword,  // 64-bit
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegRef {
    pub reg: Register,
    pub width: RegWidth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    Byte,
    Word,
    Dword,
    Qword,
}

impl Size {
    pub fn bytes(self) -> u64 {
        match self {
            Size::Byte  => 1,
            Size::Word  => 2,
            Size::Dword => 4,
            Size::Qword => 8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemAddr {
    pub size:  Option<Size>,
    pub base:  Option<RegRef>,
    pub index: Option<RegRef>,
    pub scale: u8,
    pub disp:  i64,
    pub label: Option<String>, // label contributing to base address
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    Register(RegRef),
    Immediate(i64),
    Memory(MemAddr),
    Label(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    // Data movement
    Mov, Push, Pop, Lea, Xchg, Movzx, Movsx,
    // Arithmetic
    Add, Sub, Mul, Imul, Div, Idiv, Inc, Dec, Neg, Adc, Sbb,
    // Logic
    And, Or, Xor, Not, Shl, Shr, Sar, Rol, Ror,
    // Compare
    Cmp, Test,
    // Control flow
    Jmp, Call, Ret, Loop,
    Je, Jne, Jl, Jle, Jg, Jge,
    Js, Jns, Jc, Jnc, Jo, Jno,
    // Syscall
    Syscall,
    // Pseudo: define equ constant
    Equ,
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub op: Op,
    pub operands: Vec<Operand>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub enum Directive {
    Section(String),
    Global(String),
    Db(Vec<u8>),
    Dw(Vec<u16>),
    Dd(Vec<u32>),
    Dq(Vec<u64>),
    Times { count: usize, inner: Box<Directive> },
    Equ(String, i64),
}

#[derive(Debug, Clone)]
pub enum AsmLine {
    Label(String),
    Instruction(Instruction),
    Directive(Directive),
}
