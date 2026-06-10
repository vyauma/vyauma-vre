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
    Import,
    As,
    Try,
    Catch,
    Throw,
    For,
    Class,

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
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Semicolon,

    // Indentation Control
    Newline,
    Indent,
    Dedent,

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
            "new" => TokenKind::New,
            "import" => TokenKind::Import,
            "as" => TokenKind::As,
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "throw" => TokenKind::Throw,
            "for" => TokenKind::For,
            "class" => TokenKind::Class,
            _ => TokenKind::Identifier(ident.to_string()),
        }
    }
}

pub struct LexerIndent<'a> {
    input: &'a str,
    position: usize,
    read_position: usize,
    ch: Option<char>,
    indent_stack: Vec<usize>,
    pending_tokens: std::collections::VecDeque<Token>,
    at_line_start: bool,
    pub line: usize,
    pub col: usize,
}

impl<'a> LexerIndent<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lexer = LexerIndent {
            input,
            position: 0,
            read_position: 0,
            ch: None,
            indent_stack: vec![0],
            pending_tokens: std::collections::VecDeque::new(),
            at_line_start: true,
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
        // Track line and col on the *consumed* character
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

    fn skip_whitespace_inline(&mut self) {
        while let Some(c) = self.ch {
            if c == ' ' || c == '\t' || c == '\r' {
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
        }
    }

    pub fn next_token(&mut self) -> Token {
        if let Some(token) = self.pending_tokens.pop_front() {
            return token;
        }

        let mut start_line = self.line;
        let mut start_col = self.col;

        if self.at_line_start {
            let mut spaces = 0;
            let mut is_empty_line = false;
            
            loop {
                match self.ch {
                    Some(' ') => {
                        spaces += 1;
                        self.read_char();
                    }
                    Some('\t') => {
                        spaces += 4;
                        self.read_char();
                    }
                    Some('\r') => {
                        self.read_char(); // ignore CR
                    }
                    Some('\n') => {
                        // Empty line with only spaces
                        spaces = 0;
                        self.read_char();
                    }
                    Some('/') => {
                        if self.peek_char() == Some('/') {
                            // Line contains only comment
                            self.skip_comment();
                            spaces = 0;
                            // the comment loop breaks AT the newline but doesn't consume it
                        } else {
                            break;
                        }
                    }
                    None => {
                        is_empty_line = true;
                        break;
                    }
                    _ => break,
                }
            }
            
            if !is_empty_line && self.ch != Some('\n') {
                self.at_line_start = false;
                let current_indent = *self.indent_stack.last().unwrap();
                
                if spaces > current_indent {
                    self.indent_stack.push(spaces);
                    return Token { kind: TokenKind::Indent, line: self.line, col: self.col };
                } else if spaces < current_indent {
                    while *self.indent_stack.last().unwrap() > spaces {
                        self.indent_stack.pop();
                        self.pending_tokens.push_back(Token { kind: TokenKind::Dedent, line: self.line, col: self.col });
                    }
                    if *self.indent_stack.last().unwrap() != spaces {
                        return Token { kind: TokenKind::Error(format!("Inconsistent indentation: expected {}, found {}", self.indent_stack.last().unwrap(), spaces)), line: self.line, col: self.col };
                    }
                    if let Some(tok) = self.pending_tokens.pop_front() {
                        return tok;
                    }
                }
            }
        }

        loop {
            self.skip_whitespace_inline();
            if self.ch == Some('/') && self.peek_char() == Some('/') {
                self.skip_comment();
                continue;
            }
            break;
        }

        start_line = self.line;
        start_col = self.col;

        let kind = match self.ch {
            Some('\n') => {
                while self.ch == Some('\n') || self.ch == Some('\r') {
                    self.read_char();
                }
                self.at_line_start = true;
                return Token { kind: TokenKind::Newline, line: start_line, col: start_col };
            }
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
            Some('[') => TokenKind::LBracket,
            Some(']') => TokenKind::RBracket,
            Some('{') => TokenKind::LBrace,
            Some('}') => TokenKind::RBrace,
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
            None => {
                while self.indent_stack.len() > 1 {
                    self.indent_stack.pop();
                    self.pending_tokens.push_back(Token { kind: TokenKind::Dedent, line: start_line, col: start_col });
                }
                if let Some(tok) = self.pending_tokens.pop_front() {
                    return tok;
                }
                return Token { kind: TokenKind::Eof, line: start_line, col: start_col };
            }
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
    fn test_indentation_lexer() {
        let input = r#"
let x = 5
if x > 2:
    print(x)
    let y = 10
else:
    return 0
let z = 1
"#;

        let mut lexer = LexerIndent::new(input);
        
        let tests = vec![
            TokenKind::Let,
            TokenKind::Identifier("x".to_string()),
            TokenKind::Assign,
            TokenKind::Number(5.0),
            TokenKind::Newline,
            
            TokenKind::If,
            TokenKind::Identifier("x".to_string()),
            TokenKind::GreaterThan,
            TokenKind::Number(2.0),
            TokenKind::Colon,
            TokenKind::Newline,
            
            TokenKind::Indent,
            TokenKind::Identifier("print".to_string()),
            TokenKind::LParen,
            TokenKind::Identifier("x".to_string()),
            TokenKind::RParen,
            TokenKind::Newline,
            
            TokenKind::Let,
            TokenKind::Identifier("y".to_string()),
            TokenKind::Assign,
            TokenKind::Number(10.0),
            TokenKind::Newline,
            
            TokenKind::Dedent,
            TokenKind::Else,
            TokenKind::Colon,
            TokenKind::Newline,
            
            TokenKind::Indent,
            TokenKind::Return,
            TokenKind::Number(0.0),
            TokenKind::Newline,
            
            TokenKind::Dedent,
            TokenKind::Let,
            TokenKind::Identifier("z".to_string()),
            TokenKind::Assign,
            TokenKind::Number(1.0),
            TokenKind::Newline,
            
            TokenKind::Eof,
        ];

        for expected in tests {
            let t = lexer.next_token();
            assert_eq!(t.kind, expected);
        }
    }
}
