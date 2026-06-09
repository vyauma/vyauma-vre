#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Fn,
    Let,
    If,
    Else,
    While,
    Return,
    Struct,
    New,
    Import,
    Try,
    Catch,
    Throw,

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

impl Token {
    pub fn lookup_ident(ident: &str) -> Token {
        match ident {
            "fn" => Token::Fn,
            "let" => Token::Let,
            "if" => Token::If,
            "else" => Token::Else,
            "while" => Token::While,
            "return" => Token::Return,
            "true" => Token::Boolean(true),
            "false" => Token::Boolean(false),
            "struct" => Token::Struct,
            "new" => Token::New,
            "import" => Token::Import,
            "try" => Token::Try,
            "catch" => Token::Catch,
            "throw" => Token::Throw,
            _ => Token::Identifier(ident.to_string()),
        }
    }
}

pub struct Lexer<'a> {
    input: &'a str,
    position: usize,
    read_position: usize,
    ch: Option<char>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Lexer {
            input,
            position: 0,
            read_position: 0,
            ch: None,
        };
        lexer.read_char();
        lexer
    }

    fn read_char(&mut self) -> Option<char> {
        let c = if self.read_position >= self.input.len() {
            None
        } else {
            self.input[self.read_position..].chars().next()
        };
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

        let token = match self.ch {
            Some('=') => {
                if self.peek_char() == Some('=') {
                    self.read_char();
                    Token::Equals
                } else {
                    Token::Assign
                }
            }
            Some('+') => Token::Plus,
            Some('-') => Token::Minus,
            Some('*') => Token::Star,
            Some('/') => Token::Slash,
            Some('<') => {
                if self.peek_char() == Some('=') {
                    self.read_char();
                    Token::LessThanOrEq
                } else {
                    Token::LessThan
                }
            }
            Some('>') => {
                if self.peek_char() == Some('=') {
                    self.read_char();
                    Token::GreaterThanOrEq
                } else {
                    Token::GreaterThan
                }
            }
            Some('!') => {
                if self.peek_char() == Some('=') {
                    self.read_char();
                    Token::NotEquals
                } else {
                    Token::Error("Expected '=' after '!'".to_string())
                }
            }
            Some('(') => Token::LParen,
            Some(')') => Token::RParen,
            Some('{') => Token::LBrace,
            Some('}') => Token::RBrace,
            Some('[') => Token::LBracket,
            Some(']') => Token::RBracket,
            Some(',') => Token::Comma,
            Some(';') => Token::Semicolon,
            Some('.') => Token::Dot,
            Some(':') => Token::Colon,
            Some('"') => return self.read_string(),
            Some(c) if c.is_alphabetic() || c == '_' => {
                let ident = self.read_identifier();
                return Token::lookup_ident(&ident);
            }
            Some(c) if c.is_ascii_digit() => {
                let num_str = self.read_number();
                if let Ok(num) = num_str.parse::<f64>() {
                    return Token::Number(num);
                } else {
                    return Token::Error(format!("Invalid number: {}", num_str));
                }
            }
            Some(c) => Token::Error(format!("Unexpected character: {}", c)),
            None => Token::Eof,
        };

        self.read_char();
        token
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

    fn read_string(&mut self) -> Token {
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
        Token::String(string)
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
            Token::Let,
            Token::Identifier("five".to_string()),
            Token::Assign,
            Token::Number(5.0),
            Token::Semicolon,
            
            Token::Let,
            Token::Identifier("ten".to_string()),
            Token::Assign,
            Token::Number(10.0),
            Token::Semicolon,

            Token::Fn,
            Token::Identifier("add".to_string()),
            Token::LParen,
            Token::Identifier("x".to_string()),
            Token::Comma,
            Token::Identifier("y".to_string()),
            Token::RParen,
            Token::LBrace,
            Token::Return,
            Token::Identifier("x".to_string()),
            Token::Plus,
            Token::Identifier("y".to_string()),
            Token::Semicolon,
            Token::RBrace,

            Token::Let,
            Token::Identifier("result".to_string()),
            Token::Assign,
            Token::Identifier("add".to_string()),
            Token::LParen,
            Token::Identifier("five".to_string()),
            Token::Comma,
            Token::Identifier("ten".to_string()),
            Token::RParen,
            Token::Semicolon,

            Token::If,
            Token::Identifier("result".to_string()),
            Token::GreaterThanOrEq,
            Token::Number(15.0),
            Token::LBrace,
            Token::Identifier("print".to_string()),
            Token::LParen,
            Token::Identifier("result".to_string()),
            Token::RParen,
            Token::Semicolon,
            Token::RBrace,
            Token::Else,
            Token::LBrace,
            Token::Return,
            Token::Number(0.0),
            Token::Semicolon,
            Token::RBrace,
            Token::Eof,
        ];

        let mut lexer = Lexer::new(input);

        for expected_token in tests {
            let tok = lexer.next_token();
            assert_eq!(tok, expected_token);
        }
    }
}
