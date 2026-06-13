#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Import,
    Fn,
    Struct,
    Let,
    Return,
    
    // Literals
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Identifier(String),
    
    // Symbols
    LBrace, RBrace,
    LParen, RParen,
    LBracket, RBracket,
    Dot,
    Comma,
    Colon,
    Equals,
    Arrow,
    SemiColon,
    
    EOF,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

pub struct Lexer<'a> {
    input: std::iter::Peekable<std::str::Chars<'a>>,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Lexer {
            input: source.chars().peekable(),
            line: 1,
            col: 1,
        }
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.input.next()?;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn peek(&mut self) -> Option<&char> {
        self.input.peek()
    }

    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        let start_line = self.line;
        let start_col = self.col;

        let kind = match self.advance() {
            Some(c) => match c {
                '{' => TokenKind::LBrace,
                '}' => TokenKind::RBrace,
                '(' => TokenKind::LParen,
                ')' => TokenKind::RParen,
                '[' => TokenKind::LBracket,
                ']' => TokenKind::RBracket,
                '.' => TokenKind::Dot,
                ',' => TokenKind::Comma,
                ':' => TokenKind::Colon,
                ';' => TokenKind::SemiColon,
                '=' => TokenKind::Equals,
                '-' => {
                    if let Some(&'>') = self.peek() {
                        self.advance();
                        TokenKind::Arrow
                    } else {
                        // For now we don't handle subtraction in this minimum set, just identifiers or symbols
                        panic!("Unexpected character '-'");
                    }
                }
                '"' => {
                    let mut s = String::new();
                    while let Some(&nc) = self.peek() {
                        if nc == '"' {
                            self.advance();
                            break;
                        }
                        s.push(self.advance().unwrap());
                    }
                    TokenKind::String(s)
                }
                c if c.is_alphabetic() || c == '_' => {
                    let mut s = String::new();
                    s.push(c);
                    while let Some(&nc) = self.peek() {
                        if nc.is_alphanumeric() || nc == '_' {
                            s.push(self.advance().unwrap());
                        } else {
                            break;
                        }
                    }
                    match s.as_str() {
                        "import" => TokenKind::Import,
                        "fn" => TokenKind::Fn,
                        "struct" => TokenKind::Struct,
                        "let" => TokenKind::Let,
                        "return" => TokenKind::Return,
                        "true" => TokenKind::Boolean(true),
                        "false" => TokenKind::Boolean(false),
                        _ => TokenKind::Identifier(s),
                    }
                }
                c if c.is_numeric() => {
                    let mut s = String::new();
                    s.push(c);
                    let mut is_float = false;
                    while let Some(&nc) = self.peek() {
                        if nc.is_numeric() {
                            s.push(self.advance().unwrap());
                        } else if nc == '.' {
                            is_float = true;
                            s.push(self.advance().unwrap());
                        } else {
                            break;
                        }
                    }
                    if is_float {
                        TokenKind::Float(s.parse().unwrap())
                    } else {
                        TokenKind::Integer(s.parse().unwrap())
                    }
                }
                _ => panic!("Unexpected character: {}", c),
            },
            None => TokenKind::EOF,
        };

        Token {
            kind,
            line: start_line,
            col: start_col,
        }
    }
    
    pub fn tokenize(mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let t = self.next_token();
            if t.kind == TokenKind::EOF {
                tokens.push(t);
                break;
            }
            tokens.push(t);
        }
        tokens
    }
}
