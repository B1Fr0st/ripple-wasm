use crate::error::SimError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Ident(String),
    Number(i64),
    StringLit(Vec<u8>),
    Colon,
    Comma,
    LBracket,
    RBracket,
    Plus,
    Minus,
    Star,
    Newline,
    Eof,
}

pub struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
    pub line: usize,
    pub col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Lexer { src: src.as_bytes(), pos: 0, line: 1, col: 1 }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<u8> {
        self.src.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let b = self.src.get(self.pos).copied()?;
        self.pos += 1;
        if b == b'\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(b)
    }

    fn skip_whitespace(&mut self) {
        while let Some(b) = self.peek() {
            if b == b' ' || b == b'\t' || b == b'\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        while let Some(b) = self.peek() {
            if b == b'\n' { break; }
            self.advance();
        }
    }

    fn read_ident(&mut self) -> String {
        let mut s = String::new();
        while let Some(b) = self.peek() {
            if b.is_ascii_alphanumeric() || b == b'_' || b == b'.' {
                s.push(self.advance().unwrap() as char);
            } else {
                break;
            }
        }
        s
    }

    fn read_number(&mut self) -> Result<i64, SimError> {
        let line = self.line;
        let col = self.col;

        let negative = if self.peek() == Some(b'-') {
            self.advance();
            true
        } else {
            false
        };

        let (radix, s) = if self.peek() == Some(b'0') && matches!(self.peek2(), Some(b'x') | Some(b'X')) {
            self.advance(); self.advance();
            let s = self.read_digits(|b| b.is_ascii_hexdigit());
            (16u32, s)
        } else if self.peek() == Some(b'0') && matches!(self.peek2(), Some(b'b') | Some(b'B')) {
            self.advance(); self.advance();
            let s = self.read_digits(|b| b == b'0' || b == b'1');
            (2u32, s)
        } else if self.peek() == Some(b'0') && matches!(self.peek2(), Some(b'o') | Some(b'O')) {
            self.advance(); self.advance();
            let s = self.read_digits(|b| (b'0'..=b'7').contains(&b));
            (8u32, s)
        } else {
            let s = self.read_digits(|b| b.is_ascii_digit());
            (10u32, s)
        };

        if s.is_empty() {
            return Err(SimError::Lex { line, col, msg: "expected digits".into() });
        }

        // For hex/binary/octal, allow full u64 range (e.g. 0xffffffffffffffff = -1i64).
        let val: i64 = if !negative && radix != 10 {
            u64::from_str_radix(&s, radix)
                .map(|u| u as i64)
                .map_err(|_| SimError::Lex { line, col, msg: format!("number overflow: {}", s) })?
        } else {
            i64::from_str_radix(&s, radix)
                .map_err(|_| SimError::Lex { line, col, msg: format!("number overflow: {}", s) })?
        };

        Ok(if negative { -val } else { val })
    }

    fn read_digits(&mut self, pred: impl Fn(u8) -> bool) -> String {
        let mut s = String::new();
        while let Some(b) = self.peek() {
            if pred(b) {
                s.push(self.advance().unwrap() as char);
            } else if b == b'_' {
                self.advance();
            } else {
                break;
            }
        }
        s
    }

    fn read_string(&mut self) -> Result<Vec<u8>, SimError> {
        let line = self.line;
        let col = self.col;
        self.advance(); // consume opening quote
        let mut bytes = Vec::new();
        loop {
            match self.advance() {
                None => return Err(SimError::Lex { line, col, msg: "unterminated string".into() }),
                Some(b'"') => break,
                Some(b'\\') => {
                    match self.advance() {
                        Some(b'n')  => bytes.push(b'\n'),
                        Some(b'r')  => bytes.push(b'\r'),
                        Some(b't')  => bytes.push(b'\t'),
                        Some(b'0')  => bytes.push(0),
                        Some(b'\\') => bytes.push(b'\\'),
                        Some(b'"')  => bytes.push(b'"'),
                        Some(c) => bytes.push(c),
                        None => return Err(SimError::Lex { line, col, msg: "unterminated escape".into() }),
                    }
                }
                Some(c) => bytes.push(c),
            }
        }
        Ok(bytes)
    }

    pub fn tokenize(&mut self) -> Result<Vec<(Token, usize)>, SimError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace();
            let line = self.line;
            match self.peek() {
                None => { tokens.push((Token::Eof, line)); break; }
                Some(b'\n') => { self.advance(); tokens.push((Token::Newline, line)); }
                Some(b';') | Some(b'#') => self.skip_comment(),
                Some(b':') => { self.advance(); tokens.push((Token::Colon, line)); }
                Some(b',') => { self.advance(); tokens.push((Token::Comma, line)); }
                Some(b'[') => { self.advance(); tokens.push((Token::LBracket, line)); }
                Some(b']') => { self.advance(); tokens.push((Token::RBracket, line)); }
                Some(b'+') => { self.advance(); tokens.push((Token::Plus, line)); }
                Some(b'*') => { self.advance(); tokens.push((Token::Star, line)); }
                Some(b'"') => {
                    let bytes = self.read_string()?;
                    tokens.push((Token::StringLit(bytes), line));
                }
                Some(b'-') => {
                    // Could be negative number or minus operator
                    if matches!(self.peek2(), Some(b) if b.is_ascii_digit()) {
                        let n = self.read_number()?;
                        tokens.push((Token::Number(n), line));
                    } else {
                        self.advance();
                        tokens.push((Token::Minus, line));
                    }
                }
                Some(b) if b.is_ascii_digit() => {
                    let n = self.read_number()?;
                    tokens.push((Token::Number(n), line));
                }
                Some(b) if b.is_ascii_alphabetic() || b == b'_' || b == b'.' => {
                    let s = self.read_ident();
                    tokens.push((Token::Ident(s), line));
                }
                Some(b) => {
                    return Err(SimError::Lex {
                        line: self.line,
                        col: self.col,
                        msg: format!("unexpected character: {:?}", b as char),
                    });
                }
            }
        }
        Ok(tokens)
    }
}
