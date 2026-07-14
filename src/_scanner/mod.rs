#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    // Single-character tokens.
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,

    // One or two character tokens.
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    // Literals.
    Identifier,
    String,
    Number,

    // Keywords.
    And,
    Class,
    Else,
    False,
    For,
    Fun,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,

    Error,
    Eof,
}

use std::convert::TryFrom;

impl TryFrom<u8> for TokenType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, <TokenType as TryFrom<u8>>::Error> {
        match value {
            0 => Ok(TokenType::LeftParen),
            1 => Ok(TokenType::RightParen),
            2 => Ok(TokenType::LeftBrace),
            3 => Ok(TokenType::RightBrace),
            4 => Ok(TokenType::Comma),
            5 => Ok(TokenType::Dot),
            6 => Ok(TokenType::Minus),
            7 => Ok(TokenType::Plus),
            8 => Ok(TokenType::Semicolon),
            9 => Ok(TokenType::Slash),
            10 => Ok(TokenType::Star),

            11 => Ok(TokenType::Bang),
            12 => Ok(TokenType::BangEqual),
            13 => Ok(TokenType::Equal),
            14 => Ok(TokenType::EqualEqual),
            15 => Ok(TokenType::Greater),
            16 => Ok(TokenType::GreaterEqual),
            17 => Ok(TokenType::Less),
            18 => Ok(TokenType::LessEqual),

            19 => Ok(TokenType::Identifier),
            20 => Ok(TokenType::String),
            21 => Ok(TokenType::Number),

            22 => Ok(TokenType::And),
            23 => Ok(TokenType::Class),
            24 => Ok(TokenType::Else),
            25 => Ok(TokenType::False),
            26 => Ok(TokenType::For),
            27 => Ok(TokenType::Fun),
            28 => Ok(TokenType::If),
            29 => Ok(TokenType::Nil),
            30 => Ok(TokenType::Or),
            31 => Ok(TokenType::Print),
            32 => Ok(TokenType::Return),
            33 => Ok(TokenType::Super),
            34 => Ok(TokenType::This),
            35 => Ok(TokenType::True),
            36 => Ok(TokenType::Var),
            37 => Ok(TokenType::While),

            38 => Ok(TokenType::Error),
            39 => Ok(TokenType::Eof),

            _ => Err(()),
        }
    }
}

pub struct Scanner {
    pub source: Vec<char>,
    pub start: usize,
    pub current: usize,
    pub line: usize,
}

#[derive(Clone)]
pub struct Token {
    pub t_type: TokenType,
    pub lexeme: String,
    pub line: usize,
}

impl Scanner {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            start: 0,
            current: 0,
            line: 1,
        }
    }

    pub fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    pub fn make_token(&self, t_type: TokenType) -> Token {
        let text = self.source[self.start..self.current].iter().collect();
        Token {
            t_type,
            lexeme: text,
            line: self.line,
        }
    }

    pub fn error_token(&self, message: &str) -> Token {
        Token {
            t_type: TokenType::Error,
            lexeme: message.to_string(),
            line: self.line,
        }
    }

    pub fn advance(&mut self) -> char {
        self.current += 1;
        return self.source[self.current - 1];
    }

    pub fn match_expr(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        };
        if self.source[self.current] != expected {
            return false;
        };
        self.current += 1;
        true
    }

    pub fn skip_whitespace(&mut self) {
        loop {
            let c = self.peek();
            match c {
                ' ' => {
                    self.advance();
                }
                '\r' => {
                    self.advance();
                }
                '\t' => {
                    self.advance();
                }
                '\n' => {
                    self.line += 1;
                    self.advance();
                }
                '/' => {
                    if self.peek_next() == '/' {
                        while self.peek() != '\n' && !self.is_at_end() {
                            self.advance();
                        }
                    } else {
                        return;
                    }
                }
                _ => return,
            };
        }
    }

    pub fn peek(&self) -> char {
        if self.is_at_end() {
            return '\0';
        };
        return self.source[self.current];
    }

    pub fn peek_next(&self) -> char {
        if self.is_at_end() {
            return '\0';
        };
        return self.source[self.current+1];
    }

    pub fn make_string(&mut self) -> Token {
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            };
            self.advance();
        }

        if self.is_at_end() {
            return self.error_token("Unterminated string.");
        };

        self.advance();
        self.make_token(TokenType::String)
    }

    pub fn make_number(&mut self) -> Token {
        while self.peek().is_digit(10) {
            self.advance();
        }

        if self.peek() == '.' && self.peek_next().is_digit(10) {
            self.advance();

            while self.peek().is_digit(10) {
                self.advance();
            }
        };

        self.make_token(TokenType::Number)
    }

    pub fn identifier(&mut self) -> Token {
        while is_alpha(self.peek()) || self.peek().is_digit(10) {
            self.advance();
        }
        self.make_token(self.identifier_type())
    }

    pub fn identifier_type(
        &self
    ) -> TokenType {
        match self.source[self.start] {
            'a' => self.check_keyword(1, 2, "nd", TokenType::And),
            'c' => self.check_keyword(1, 4, "lass", TokenType::Class),
            'e' => self.check_keyword(1, 3, "lse", TokenType::Else),
            'f' => {
                if self.current - self.start > 1 {
                    return match self.source[self.start+1] {
                        'a' => self.check_keyword(2, 3, "lse", TokenType::False),
                        'o' => self.check_keyword(2, 1, "r", TokenType::For),
                        'u' => self.check_keyword(2,1, "n", TokenType::Fun),
                        _ => TokenType::Identifier
                    }
                }

                TokenType::Identifier
            }
            'i' => self.check_keyword(1, 1, "f", TokenType::If),
            'n' => self.check_keyword(1, 2, "il", TokenType::Nil),
            'o' => self.check_keyword(1, 1, "r", TokenType::Or),
            'p' => self.check_keyword(1, 4, "rint", TokenType::Print),
            'r' => self.check_keyword(1, 5, "eturn", TokenType::Return),
            's' => self.check_keyword(1, 4, "uper", TokenType::Super),
            't' => {
                if self.current - self.start > 1 {
                    return match self.source[self.start+1] {
                        'h' => self.check_keyword(2, 2, "is", TokenType::This),
                        'r' => self.check_keyword(2, 2, "ue", TokenType::True),
                        _ => TokenType::Identifier
                    }
                }

                TokenType::Identifier
            }
            'v' => self.check_keyword(1, 2, "ar", TokenType::Var),
            'w' => self.check_keyword(1, 4, "hile", TokenType::While),

            _ => TokenType::Identifier,
        }
    }

    fn check_keyword(
        &self,
        start: usize,
        length: usize,
        rest: &str,
        token_type: TokenType,
    ) -> TokenType {
        let end = self.start + start + length;

        if end <= self.source.len() {
            let keyword: String = self.source[self.start + start..end].iter().collect();

            if keyword == rest {
                return token_type;
            }
        }

        TokenType::Identifier
    }

    pub fn scan_token(&mut self) -> Token {
        self.skip_whitespace();
        self.start = self.current.clone();

        if self.is_at_end() {
            return self.make_token(TokenType::Eof);
        }

        let c = self.advance();

        if is_alpha(c) {
            return self.identifier();
        };
        if c.is_digit(10) {
            return self.make_number();
        };

        match c {
            '(' => self.make_token(TokenType::LeftParen),
            ')' => self.make_token(TokenType::RightParen),
            '{' => self.make_token(TokenType::LeftBrace),
            '}' => self.make_token(TokenType::RightBrace),
            ';' => self.make_token(TokenType::Semicolon),
            ',' => self.make_token(TokenType::Comma),
            '.' => self.make_token(TokenType::Dot),
            '-' => self.make_token(TokenType::Minus),
            '+' => self.make_token(TokenType::Plus),
            '/' => self.make_token(TokenType::Slash),
            '*' => self.make_token(TokenType::Star),
            '!' => {
                let a = if self.match_expr('=') {
                    TokenType::BangEqual
                } else {
                    TokenType::Bang
                };
                self.make_token(a)
            }
            '=' => {
                let a = if self.match_expr('=') {
                    TokenType::EqualEqual
                } else {
                    TokenType::Equal
                };
                self.make_token(a)
            }
            '<' => {
                let a = if self.match_expr('=') {
                    TokenType::LessEqual
                } else {
                    TokenType::Less
                };
                self.make_token(a)
            }
            '>' => {
                let a = if self.match_expr('=') {
                    TokenType::GreaterEqual
                } else {
                    TokenType::Greater
                };
                self.make_token(a)
            }
            '"' => self.make_string(),
            _ => self.error_token("Unexpected character."),
        }
    }
}

pub fn is_alpha(c: char) -> bool {
    return c.is_alphabetic() || c == '_';
}
