#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Fn,
    Let,
    If,
    Else,
    While,
    Return,
    Struct,
    New,
    Class,
    Import,
    As,
    Try,
    Catch,
    Throw,
    For,

    // Identifiers and Literals
    Identifier(String),
    Number(f64),
    String(String),
    Boolean(bool),

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Assign,
    Equals,
    NotEquals,
    LessThan,
    GreaterThan,
    LessThanOrEq,
    GreaterThanOrEq,
    Dot,
    Colon,
    ReturnArrow,
    And,
    Or,
    DoubleColon,

    // Punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semicolon,

    // Special
    Eof,
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

impl TokenKind {
    pub fn lookup_ident(ident: &str) -> TokenKind {
        match ident {
            "fn" => TokenKind::Fn,
            "let" => TokenKind::Let,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "return" => TokenKind::Return,
            "true" => TokenKind::Boolean(true),
            "false" => TokenKind::Boolean(false),
            "struct" => TokenKind::Struct,
            "class" => TokenKind::Class,
            "new" => TokenKind::New,
            "import" => TokenKind::Import,
            "as" => TokenKind::As,
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "throw" => TokenKind::Throw,
            "for" => TokenKind::For,
            _ => TokenKind::Identifier(ident.to_string()),
        }
    }
}

pub struct Lexer<'a> {
    input: &'a str,
    position: usize,
    read_position: usize,
    ch: Option<char>,
    pub line: usize,
    pub col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Lexer {
            input,
            position: 0,
            read_position: 0,
            ch: None,
            line: 1,
            col: 0,
        };
        lexer.read_char();
        lexer
    }

    /// Returns the current (line, col) position as a span string "line:col".
    pub fn current_span(&self) -> String {
        format!("{}:{}", self.line, self.col)
    }

    fn read_char(&mut self) -> Option<char> {
        let c = if self.read_position >= self.input.len() {
            None
        } else {
            self.input[self.read_position..].chars().next()
        };
        // Track current char position for newlines
        if let Some('\n') = self.ch {
            self.line += 1;
            self.col = 0;
        } else {
            self.col += 1;
        }
        self.position = self.read_position;
        if let Some(ch) = c {
            self.read_position += ch.len_utf8();
        }
        self.ch = c;
        c
    }

    fn peek_char(&self) -> Option<char> {
        if self.read_position >= self.input.len() {
            None
        } else {
            self.input[self.read_position..].chars().next()
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.ch {
            if c.is_whitespace() {
                self.read_char();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        if self.ch == Some('/') && self.peek_char() == Some('/') {
            while let Some(c) = self.ch {
                if c == '\n' {
                    break;
                }
                self.read_char();
            }
            self.skip_whitespace();
        }
    }

    pub fn next_token(&mut self) -> Token {
        loop {
            self.skip_whitespace();
            if self.ch == Some('/') && self.peek_char() == Some('/') {
                self.skip_comment();
                continue;
            }
            break;
        }

        let start_line = self.line;
        let start_col = self.col;

        let kind = match self.ch {
            Some('=') => {
                if self.peek_char() == Some('=') {
                    self.read_char();
                    TokenKind::Equals
                } else {
                    TokenKind::Assign
                }
            }
            Some('+') => TokenKind::Plus,
            Some('-') => {
                if self.peek_char() == Some('>') {
                    self.read_char();
                    TokenKind::ReturnArrow
                } else {
                    TokenKind::Minus
                }
            }
            Some('*') => TokenKind::Star,
            Some('/') => TokenKind::Slash,
            Some('<') => {
                if self.peek_char() == Some('=') {
                    self.read_char();
                    TokenKind::LessThanOrEq
                } else {
                    TokenKind::LessThan
                }
            }
            Some('>') => {
                if self.peek_char() == Some('=') {
                    self.read_char();
                    TokenKind::GreaterThanOrEq
                } else {
                    TokenKind::GreaterThan
                }
            }
            Some('!') => {
                if self.peek_char() == Some('=') {
                    self.read_char();
                    TokenKind::NotEquals
                } else {
                    TokenKind::Error("Expected '=' after '!'".to_string())
                }
            }
            Some('&') => {
                if self.peek_char() == Some('&') {
                    self.read_char();
                    TokenKind::And
                } else {
                    TokenKind::Error("Expected '&' after '&'".to_string())
                }
            }
            Some('|') => {
                if self.peek_char() == Some('|') {
                    self.read_char();
                    TokenKind::Or
                } else {
                    TokenKind::Error("Expected '|' after '|'".to_string())
                }
            }
            Some('(') => TokenKind::LParen,
            Some(')') => TokenKind::RParen,
            Some('{') => TokenKind::LBrace,
            Some('}') => TokenKind::RBrace,
            Some('[') => TokenKind::LBracket,
            Some(']') => TokenKind::RBracket,
            Some(',') => TokenKind::Comma,
            Some(';') => TokenKind::Semicolon,
            Some('.') => TokenKind::Dot,
            Some(':') => {
                if self.peek_char() == Some(':') {
                    self.read_char();
                    TokenKind::DoubleColon
                } else {
                    TokenKind::Colon
                }
            }
            Some('"') => {
                let kind = self.read_string();
                return Token { kind, line: start_line, col: start_col };
            }
            Some(c) if c.is_alphabetic() || c == '_' => {
                let ident = self.read_identifier();
                let kind = TokenKind::lookup_ident(&ident);
                return Token { kind, line: start_line, col: start_col };
            }
            Some(c) if c.is_ascii_digit() => {
                let num_str = self.read_number();
                let kind = if let Ok(num) = num_str.parse::<f64>() {
                    TokenKind::Number(num)
                } else {
                    TokenKind::Error(format!("Invalid number: {}", num_str))
                };
                return Token { kind, line: start_line, col: start_col };
            }
            Some(c) => TokenKind::Error(format!("Unexpected character: {}", c)),
            None => TokenKind::Eof,
        };

        self.read_char();
        Token { kind, line: start_line, col: start_col }
    }

    fn read_identifier(&mut self) -> String {
        let position = self.position;
        while let Some(c) = self.ch {
            if c.is_alphanumeric() || c == '_' {
                self.read_char();
            } else {
                break;
            }
        }
        self.input[position..self.position].to_string()
    }

    fn read_string(&mut self) -> TokenKind {
        self.read_char(); // consume quote
        let mut string = String::new();
        while let Some(c) = self.ch {
            if c == '"' {
                break;
            }
            if c == '\\' {
                self.read_char();
                if let Some(esc) = self.ch {
                    string.push(match esc {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '"' => '"',
                        '\\' => '\\',
                        _ => esc,
                    });
                }
            } else {
                string.push(c);
            }
            self.read_char();
        }
        self.read_char(); // consume closing quote
        TokenKind::String(string)
    }

    fn read_number(&mut self) -> String {
        let position = self.position;
        while let Some(c) = self.ch {
            if c.is_ascii_digit() || c == '.' {
                self.read_char();
            } else {
                break;
            }
        }
        self.input[position..self.position].to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_token() {
        let input = r#"
            // This is a comment
            let five = 5;
            let ten = 10;

            fn add(x, y) {
                return x + y;
            }

            let result = add(five, ten);
            
            if result >= 15 {
                print(result);
            } else {
                return 0;
            }
        "#;

        let tests = vec![
            TokenKind::Let,
            TokenKind::Identifier("five".to_string()),
            TokenKind::Assign,
            TokenKind::Number(5.0),
            TokenKind::Semicolon,
            
            TokenKind::Let,
            TokenKind::Identifier("ten".to_string()),
            TokenKind::Assign,
            TokenKind::Number(10.0),
            TokenKind::Semicolon,

            TokenKind::Fn,
            TokenKind::Identifier("add".to_string()),
            TokenKind::LParen,
            TokenKind::Identifier("x".to_string()),
            TokenKind::Comma,
            TokenKind::Identifier("y".to_string()),
            TokenKind::RParen,
            TokenKind::LBrace,
            TokenKind::Return,
            TokenKind::Identifier("x".to_string()),
            TokenKind::Plus,
            TokenKind::Identifier("y".to_string()),
            TokenKind::Semicolon,
            TokenKind::RBrace,

            TokenKind::Let,
            TokenKind::Identifier("result".to_string()),
            TokenKind::Assign,
            TokenKind::Identifier("add".to_string()),
            TokenKind::LParen,
            TokenKind::Identifier("five".to_string()),
            TokenKind::Comma,
            TokenKind::Identifier("ten".to_string()),
            TokenKind::RParen,
            TokenKind::Semicolon,

            TokenKind::If,
            TokenKind::Identifier("result".to_string()),
            TokenKind::GreaterThanOrEq,
            TokenKind::Number(15.0),
            TokenKind::LBrace,
            TokenKind::Identifier("print".to_string()),
            TokenKind::LParen,
            TokenKind::Identifier("result".to_string()),
            TokenKind::RParen,
            TokenKind::Semicolon,
            TokenKind::RBrace,
            TokenKind::Else,
            TokenKind::LBrace,
            TokenKind::Return,
            TokenKind::Number(0.0),
            TokenKind::Semicolon,
            TokenKind::RBrace,
            TokenKind::Eof,
        ];

        let mut lexer = Lexer::new(input);

        for expected_token in tests {
            let tok = lexer.next_token();
            assert_eq!(tok.kind, expected_token);
        }
    }
}
