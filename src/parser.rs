use crate::ast::*;
use crate::error::SimError;
use crate::lexer::Token;

pub struct Parser {
    tokens: Vec<(Token, usize)>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<(Token, usize)>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos].0
    }

    fn line(&self) -> usize {
        self.tokens[self.pos].1
    }

    fn advance(&mut self) -> &Token {
        let t = &self.tokens[self.pos].0;
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn expect_newline_or_eof(&mut self) -> Result<(), SimError> {
        self.skip_newlines();
        Ok(())
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Token::Newline) {
            self.advance();
        }
    }

    fn eat_newlines(&mut self) {
        while matches!(self.peek(), Token::Newline) {
            self.advance();
        }
    }

    fn parse_register(s: &str) -> Option<RegRef> {
        let (reg, width) = match s.to_ascii_lowercase().as_str() {
            "rax" => (Register::Rax, RegWidth::Qword),
            "rbx" => (Register::Rbx, RegWidth::Qword),
            "rcx" => (Register::Rcx, RegWidth::Qword),
            "rdx" => (Register::Rdx, RegWidth::Qword),
            "rsi" => (Register::Rsi, RegWidth::Qword),
            "rdi" => (Register::Rdi, RegWidth::Qword),
            "rsp" => (Register::Rsp, RegWidth::Qword),
            "rbp" => (Register::Rbp, RegWidth::Qword),
            "r8"  => (Register::R8,  RegWidth::Qword),
            "r9"  => (Register::R9,  RegWidth::Qword),
            "r10" => (Register::R10, RegWidth::Qword),
            "r11" => (Register::R11, RegWidth::Qword),
            "r12" => (Register::R12, RegWidth::Qword),
            "r13" => (Register::R13, RegWidth::Qword),
            "r14" => (Register::R14, RegWidth::Qword),
            "r15" => (Register::R15, RegWidth::Qword),
            "eax" => (Register::Rax, RegWidth::Dword),
            "ebx" => (Register::Rbx, RegWidth::Dword),
            "ecx" => (Register::Rcx, RegWidth::Dword),
            "edx" => (Register::Rdx, RegWidth::Dword),
            "esi" => (Register::Rsi, RegWidth::Dword),
            "edi" => (Register::Rdi, RegWidth::Dword),
            "esp" => (Register::Rsp, RegWidth::Dword),
            "ebp" => (Register::Rbp, RegWidth::Dword),
            "r8d"  => (Register::R8,  RegWidth::Dword),
            "r9d"  => (Register::R9,  RegWidth::Dword),
            "r10d" => (Register::R10, RegWidth::Dword),
            "r11d" => (Register::R11, RegWidth::Dword),
            "r12d" => (Register::R12, RegWidth::Dword),
            "r13d" => (Register::R13, RegWidth::Dword),
            "r14d" => (Register::R14, RegWidth::Dword),
            "r15d" => (Register::R15, RegWidth::Dword),
            "ax"  => (Register::Rax, RegWidth::Word),
            "bx"  => (Register::Rbx, RegWidth::Word),
            "cx"  => (Register::Rcx, RegWidth::Word),
            "dx"  => (Register::Rdx, RegWidth::Word),
            "si"  => (Register::Rsi, RegWidth::Word),
            "di"  => (Register::Rdi, RegWidth::Word),
            "sp"  => (Register::Rsp, RegWidth::Word),
            "bp"  => (Register::Rbp, RegWidth::Word),
            "r8w"  => (Register::R8,  RegWidth::Word),
            "r9w"  => (Register::R9,  RegWidth::Word),
            "r10w" => (Register::R10, RegWidth::Word),
            "r11w" => (Register::R11, RegWidth::Word),
            "r12w" => (Register::R12, RegWidth::Word),
            "r13w" => (Register::R13, RegWidth::Word),
            "r14w" => (Register::R14, RegWidth::Word),
            "r15w" => (Register::R15, RegWidth::Word),
            "al"  => (Register::Rax, RegWidth::Byte),
            "bl"  => (Register::Rbx, RegWidth::Byte),
            "cl"  => (Register::Rcx, RegWidth::Byte),
            "dl"  => (Register::Rdx, RegWidth::Byte),
            "sil" => (Register::Rsi, RegWidth::Byte),
            "dil" => (Register::Rdi, RegWidth::Byte),
            "spl" => (Register::Rsp, RegWidth::Byte),
            "bpl" => (Register::Rbp, RegWidth::Byte),
            "ah"  => (Register::Rax, RegWidth::ByteHi),
            "bh"  => (Register::Rbx, RegWidth::ByteHi),
            "ch"  => (Register::Rcx, RegWidth::ByteHi),
            "dh"  => (Register::Rdx, RegWidth::ByteHi),
            "r8b"  => (Register::R8,  RegWidth::Byte),
            "r9b"  => (Register::R9,  RegWidth::Byte),
            "r10b" => (Register::R10, RegWidth::Byte),
            "r11b" => (Register::R11, RegWidth::Byte),
            "r12b" => (Register::R12, RegWidth::Byte),
            "r13b" => (Register::R13, RegWidth::Byte),
            "r14b" => (Register::R14, RegWidth::Byte),
            "r15b" => (Register::R15, RegWidth::Byte),
            _ => return None,
        };
        Some(RegRef { reg, width })
    }

    fn parse_op(s: &str) -> Option<Op> {
        match s.to_ascii_lowercase().as_str() {
            "mov"    => Some(Op::Mov),
            "push"   => Some(Op::Push),
            "pop"    => Some(Op::Pop),
            "lea"    => Some(Op::Lea),
            "xchg"   => Some(Op::Xchg),
            "movzx"  => Some(Op::Movzx),
            "movsx"  => Some(Op::Movsx),
            "add"    => Some(Op::Add),
            "sub"    => Some(Op::Sub),
            "mul"    => Some(Op::Mul),
            "imul"   => Some(Op::Imul),
            "div"    => Some(Op::Div),
            "idiv"   => Some(Op::Idiv),
            "inc"    => Some(Op::Inc),
            "dec"    => Some(Op::Dec),
            "neg"    => Some(Op::Neg),
            "adc"    => Some(Op::Adc),
            "sbb"    => Some(Op::Sbb),
            "and"    => Some(Op::And),
            "or"     => Some(Op::Or),
            "xor"    => Some(Op::Xor),
            "not"    => Some(Op::Not),
            "shl"    | "sal" => Some(Op::Shl),
            "shr"    => Some(Op::Shr),
            "sar"    => Some(Op::Sar),
            "rol"    => Some(Op::Rol),
            "ror"    => Some(Op::Ror),
            "cmp"    => Some(Op::Cmp),
            "test"   => Some(Op::Test),
            "jmp"    => Some(Op::Jmp),
            "call"   => Some(Op::Call),
            "ret"    => Some(Op::Ret),
            "loop"   => Some(Op::Loop),
            "je"  | "jz"   => Some(Op::Je),
            "jne" | "jnz"  => Some(Op::Jne),
            "jl"  | "jnge" => Some(Op::Jl),
            "jle" | "jng"  => Some(Op::Jle),
            "jg"  | "jnle" => Some(Op::Jg),
            "jge" | "jnl"  => Some(Op::Jge),
            "js"           => Some(Op::Js),
            "jns"          => Some(Op::Jns),
            "jc"  | "jb"   => Some(Op::Jc),
            "jnc" | "jae"  => Some(Op::Jnc),
            "jo"           => Some(Op::Jo),
            "jno"          => Some(Op::Jno),
            "syscall"      => Some(Op::Syscall),
            _ => None,
        }
    }

    fn parse_size_override(&mut self) -> Option<Size> {
        if let Token::Ident(s) = self.peek() {
            let size = match s.to_ascii_lowercase().as_str() {
                "byte"  => Some(Size::Byte),
                "word"  => Some(Size::Word),
                "dword" => Some(Size::Dword),
                "qword" => Some(Size::Qword),
                _ => None,
            };
            if let Some(sz) = size {
                self.advance();
                // consume optional "ptr"
                if let Token::Ident(p) = self.peek() {
                    if p.to_ascii_lowercase() == "ptr" {
                        self.advance();
                    }
                }
                return Some(sz);
            }
        }
        None
    }

    fn parse_mem_addr(&mut self, size: Option<Size>, line: usize) -> Result<MemAddr, SimError> {
        // Already consumed '['
        let mut base: Option<RegRef> = None;
        let mut index: Option<RegRef> = None;
        let mut scale: u8 = 1;
        let mut disp: i64 = 0;
        let mut label: Option<String> = None;

        // Parse terms until ']'
        loop {
            match self.peek().clone() {
                Token::RBracket => { self.advance(); break; }
                Token::Ident(s) => {
                    if let Some(reg) = Self::parse_register(&s) {
                        self.advance();
                        // Check for *scale
                        if matches!(self.peek(), Token::Star) {
                            self.advance();
                            if let Token::Number(n) = self.peek().clone() {
                                let n = n as u8;
                                self.advance();
                                if base.is_none() {
                                    // treat as index*scale
                                    index = Some(reg);
                                    scale = n;
                                } else {
                                    index = Some(reg);
                                    scale = n;
                                }
                            } else {
                                return Err(SimError::Parse { line, msg: "expected scale after *".into() });
                            }
                        } else if base.is_none() {
                            base = Some(reg);
                        } else {
                            index = Some(reg);
                        }
                    } else {
                        // Unknown ident inside [] — treat as label base address
                        label = Some(s.clone());
                        self.advance();
                    }
                }
                Token::Number(n) => {
                    let n = n;
                    self.advance();
                    disp = disp.wrapping_add(n);
                }
                Token::Plus  => { self.advance(); }
                Token::Minus => {
                    self.advance();
                    if let Token::Number(n) = self.peek().clone() {
                        self.advance();
                        disp = disp.wrapping_sub(n);
                    } else {
                        return Err(SimError::Parse { line, msg: "expected number after '-' in memory expression".into() });
                    }
                }
                _ => return Err(SimError::Parse { line, msg: format!("unexpected token in memory expression: {:?}", self.peek()) }),
            }
        }

        Ok(MemAddr { size, base, index, scale, disp, label })
    }

    fn parse_operand(&mut self) -> Result<Operand, SimError> {
        let line = self.line();

        // Size override?
        let size = self.parse_size_override();

        match self.peek().clone() {
            Token::LBracket => {
                self.advance();
                let mem = self.parse_mem_addr(size, line)?;
                Ok(Operand::Memory(mem))
            }
            Token::Ident(s) => {
                if let Some(reg) = Self::parse_register(&s) {
                    self.advance();
                    Ok(Operand::Register(reg))
                } else {
                    self.advance();
                    Ok(Operand::Label(s))
                }
            }
            Token::Number(n) => {
                let n = n;
                self.advance();
                Ok(Operand::Immediate(n))
            }
            t => Err(SimError::Parse { line, msg: format!("expected operand, got {:?}", t) }),
        }
    }

    fn parse_operands(&mut self) -> Result<Vec<Operand>, SimError> {
        let mut ops = Vec::new();
        if matches!(self.peek(), Token::Newline | Token::Eof) {
            return Ok(ops);
        }
        ops.push(self.parse_operand()?);
        while matches!(self.peek(), Token::Comma) {
            self.advance();
            ops.push(self.parse_operand()?);
        }
        Ok(ops)
    }

    fn parse_db_values(&mut self) -> Result<Vec<u8>, SimError> {
        let line = self.line();
        let mut bytes = Vec::new();
        loop {
            match self.peek().clone() {
                Token::StringLit(s) => {
                    self.advance();
                    bytes.extend_from_slice(&s);
                }
                Token::Number(n) => {
                    self.advance();
                    let b = n as u8;
                    bytes.push(b);
                }
                Token::Ident(s) => {
                    // Could be a label reference — for now error
                    return Err(SimError::Parse { line, msg: format!("unexpected ident '{}' in db", s) });
                }
                _ => break,
            }
            if matches!(self.peek(), Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        Ok(bytes)
    }

    fn parse_word_values<T: TryFrom<i64>>(&mut self, name: &str) -> Result<Vec<T>, SimError>
    where
        <T as TryFrom<i64>>::Error: std::fmt::Debug,
    {
        let line = self.line();
        let mut vals = Vec::new();
        loop {
            match self.peek().clone() {
                Token::Number(n) => {
                    self.advance();
                    let v = T::try_from(n).map_err(|e| SimError::Parse {
                        line,
                        msg: format!("{} value out of range: {:?}", name, e),
                    })?;
                    vals.push(v);
                }
                _ => break,
            }
            if matches!(self.peek(), Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        Ok(vals)
    }

    pub fn parse(&mut self) -> Result<Vec<AsmLine>, SimError> {
        let mut lines = Vec::new();
        self.eat_newlines();

        while !matches!(self.peek(), Token::Eof) {
            let line = self.line();

            match self.peek().clone() {
                Token::Newline => { self.advance(); continue; }
                Token::Ident(s) => {
                    let lower = s.to_ascii_lowercase();

                    // Directive?
                    match lower.as_str() {
                        "section" => {
                            self.advance();
                            if let Token::Ident(name) = self.peek().clone() {
                                self.advance();
                                lines.push(AsmLine::Directive(Directive::Section(name)));
                            } else {
                                return Err(SimError::Parse { line, msg: "expected section name".into() });
                            }
                        }
                        "global" | "extern" => {
                            self.advance();
                            if let Token::Ident(name) = self.peek().clone() {
                                self.advance();
                                lines.push(AsmLine::Directive(Directive::Global(name)));
                            } else {
                                return Err(SimError::Parse { line, msg: "expected symbol name".into() });
                            }
                        }
                        "db" => {
                            self.advance();
                            let bytes = self.parse_db_values()?;
                            lines.push(AsmLine::Directive(Directive::Db(bytes)));
                        }
                        "dw" => {
                            self.advance();
                            let vals = self.parse_word_values::<u16>("dw")?;
                            lines.push(AsmLine::Directive(Directive::Dw(vals)));
                        }
                        "dd" => {
                            self.advance();
                            let vals = self.parse_word_values::<u32>("dd")?;
                            lines.push(AsmLine::Directive(Directive::Dd(vals)));
                        }
                        "dq" => {
                            self.advance();
                            let vals = self.parse_word_values::<u64>("dq")?;
                            lines.push(AsmLine::Directive(Directive::Dq(vals)));
                        }
                        "times" => {
                            self.advance();
                            let count = if let Token::Number(n) = self.peek().clone() {
                                self.advance();
                                n as usize
                            } else {
                                return Err(SimError::Parse { line, msg: "expected count after times".into() });
                            };
                            // parse the inner directive
                            let inner = if let Token::Ident(d) = self.peek().clone() {
                                match d.to_ascii_lowercase().as_str() {
                                    "db" => {
                                        self.advance();
                                        let bytes = self.parse_db_values()?;
                                        Directive::Db(bytes)
                                    }
                                    _ => return Err(SimError::Parse { line, msg: "expected directive after times".into() }),
                                }
                            } else {
                                return Err(SimError::Parse { line, msg: "expected directive after times count".into() });
                            };
                            lines.push(AsmLine::Directive(Directive::Times { count, inner: Box::new(inner) }));
                        }
                        _ => {
                            // Check if next token is ':' (label)
                            // Peek ahead without consuming
                            let next_pos = self.pos + 1;
                            if next_pos < self.tokens.len() && self.tokens[next_pos].0 == Token::Colon {
                                let label = s.clone();
                                self.advance(); // ident
                                self.advance(); // colon

                                // Check for `equ` definition: label equ value
                                if let Token::Ident(kw) = self.peek().clone() {
                                    if kw.to_ascii_lowercase() == "equ" {
                                        self.advance();
                                        if let Token::Number(n) = self.peek().clone() {
                                            self.advance();
                                            lines.push(AsmLine::Directive(Directive::Equ(label, n)));
                                            self.expect_newline_or_eof()?;
                                            continue;
                                        } else {
                                            return Err(SimError::Parse { line, msg: "expected number after equ".into() });
                                        }
                                    }
                                }

                                lines.push(AsmLine::Label(label));
                                // Don't consume newline; fall through to parse next line
                                continue;
                            }

                            // Check for `name equ value` (NASM style without colon)
                            let next_pos = self.pos + 1;
                            if next_pos < self.tokens.len() {
                                if let Token::Ident(kw) = &self.tokens[next_pos].0 {
                                    if kw.to_ascii_lowercase() == "equ" {
                                        let name = s.clone();
                                        self.advance(); // name
                                        self.advance(); // equ
                                        if let Token::Number(n) = self.peek().clone() {
                                            self.advance();
                                            lines.push(AsmLine::Directive(Directive::Equ(name, n)));
                                            self.expect_newline_or_eof()?;
                                            continue;
                                        } else {
                                            return Err(SimError::Parse { line, msg: "expected number after equ".into() });
                                        }
                                    }
                                }
                            }

                            // Check for NASM-style `label db/dw/dd/dq ...` (no colon)
                            let next_pos = self.pos + 1;
                            let next_is_data_directive = next_pos < self.tokens.len() && {
                                if let Token::Ident(kw) = &self.tokens[next_pos].0 {
                                    matches!(kw.to_ascii_lowercase().as_str(), "db" | "dw" | "dd" | "dq" | "times")
                                } else {
                                    false
                                }
                            };
                            if next_is_data_directive {
                                let label = s.clone();
                                self.advance(); // consume label ident
                                lines.push(AsmLine::Label(label));
                                continue; // re-enter loop to parse directive on same line
                            }

                            // Instruction
                            if let Some(op) = Self::parse_op(&s) {
                                self.advance();
                                let operands = self.parse_operands()?;
                                lines.push(AsmLine::Instruction(Instruction { op, operands, line }));
                            } else {
                                return Err(SimError::Parse { line, msg: format!("unknown instruction or directive: '{}'", s) });
                            }
                        }
                    }
                }
                t => {
                    return Err(SimError::Parse { line, msg: format!("unexpected token: {:?}", t) });
                }
            }

            // Consume trailing newline(s)
            while matches!(self.peek(), Token::Newline) {
                self.advance();
            }
        }

        Ok(lines)
    }
}
