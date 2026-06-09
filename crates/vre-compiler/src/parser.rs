use crate::ast::*;
use crate::lexer::{Lexer, Token};

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: Token,
    peek_token: Token,
}

impl<'a> Parser<'a> {
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let current_token = lexer.next_token();
        let peek_token = lexer.next_token();
        Parser {
            lexer,
            current_token,
            peek_token,
        }
    }

    fn next_token(&mut self) {
        self.current_token = self.peek_token.clone();
        self.peek_token = self.lexer.next_token();
    }

    fn expect_peek(&mut self, token: Token) -> Result<(), String> {
        if self.peek_token == token {
            self.next_token();
            Ok(())
        } else {
            Err(format!("Expected {:?}, got {:?}", token, self.peek_token))
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, String> {
        let mut imports = Vec::new();
        let mut functions = Vec::new();
        let mut structs = Vec::new();

        while self.current_token != Token::Eof {
            if self.current_token == Token::Import {
                self.next_token();
                match &self.current_token {
                    Token::String(path) => {
                        imports.push(path.clone());
                        if self.peek_token == Token::Semicolon {
                            self.next_token();
                        }
                    }
                    _ => return Err("Expected string literal after import".to_string()),
                }
            } else if self.current_token == Token::Fn {
                functions.push(self.parse_function()?);
            } else if self.current_token == Token::Struct {
                structs.push(self.parse_struct_decl()?);
            } else {
                return Err(format!("Unexpected token at top level: {:?}", self.current_token));
            }
            self.next_token();
        }

        Ok(Program { imports, functions, structs })
    }

    fn parse_struct_decl(&mut self) -> Result<Stmt, String> {
        // Current is 'struct'
        self.next_token();
        let name = match &self.current_token {
            Token::Identifier(id) => id.clone(),
            _ => return Err("Expected struct name".to_string()),
        };
        self.expect_peek(Token::LBrace)?;
        
        let mut fields = Vec::new();
        if self.peek_token != Token::RBrace {
            self.next_token();
            match &self.current_token {
                Token::Identifier(id) => fields.push(id.clone()),
                _ => return Err("Expected field name".to_string()),
            }

            while self.peek_token == Token::Comma {
                self.next_token(); // comma
                self.next_token(); // id
                match &self.current_token {
                    Token::Identifier(id) => fields.push(id.clone()),
                    _ => return Err("Expected field name after comma".to_string()),
                }
            }
        }
        self.expect_peek(Token::RBrace)?;
        Ok(Stmt::StructDecl(name, fields))
    }

    fn parse_function(&mut self) -> Result<Function, String> {
        // Current is 'fn'
        
        // Name
        let name = match &self.peek_token {
            Token::Identifier(id) => id.clone(),
            _ => return Err(format!("Expected function name, got {:?}", self.peek_token)),
        };
        self.next_token();

        self.expect_peek(Token::LParen)?;

        let mut params = Vec::new();
        if self.peek_token != Token::RParen {
            self.next_token();
            match &self.current_token {
                Token::Identifier(id) => params.push(id.clone()),
                _ => return Err("Expected parameter name".to_string()),
            }

            while self.peek_token == Token::Comma {
                self.next_token(); // consume comma
                self.next_token(); // move to id
                match &self.current_token {
                    Token::Identifier(id) => params.push(id.clone()),
                    _ => return Err("Expected parameter name after comma".to_string()),
                }
            }
        }
        self.expect_peek(Token::RParen)?;
        self.expect_peek(Token::LBrace)?;

        let body = self.parse_block()?;

        Ok(Function {
            name,
            params,
            body,
        })
    }

    fn parse_block(&mut self) -> Result<Block, String> {
        self.next_token(); // Move past '{'
        let mut stmts = Vec::new();

        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            stmts.push(self.parse_statement()?);
            self.next_token();
        }

        if self.current_token != Token::RBrace {
            return Err("Expected '}' at end of block".to_string());
        }

        Ok(stmts)
    }

    fn parse_statement(&mut self) -> Result<Stmt, String> {
        match self.current_token {
            Token::Let => self.parse_let_statement(),
            Token::Return => self.parse_return_statement(),
            Token::If => self.parse_if_statement(),
            Token::While => self.parse_while_statement(),
            Token::Try => self.parse_try_catch_statement(),
            Token::Throw => self.parse_throw_statement(),
            Token::Identifier(_) if self.peek_token == Token::Assign => self.parse_assign_statement(),
            Token::Identifier(_) if self.peek_token == Token::LBracket => self.parse_assign_index_statement(),
            _ => {
                let expr = self.parse_expression(Precedence::Lowest)?;
                
                if self.peek_token == Token::Assign {
                    self.next_token(); // move to '='
                    self.next_token(); // move to rhs
                    let rhs = self.parse_expression(Precedence::Lowest)?;
                    if self.peek_token == Token::Semicolon {
                        self.next_token();
                    }
                    match expr {
                        Expr::Identifier(name) => Ok(Stmt::Assign(name, rhs)),
                        Expr::PropertyAccess(obj, prop) => Ok(Stmt::AssignProperty(obj, prop, rhs)),
                        Expr::IndexAccess(arr, idx) => {
                            if let Expr::Identifier(name) = *arr {
                                Ok(Stmt::AssignIndex(name, *idx, rhs))
                            } else {
                                Err("Invalid array assignment target".to_string())
                            }
                        }
                        _ => Err("Invalid assignment target".to_string()),
                    }
                } else {
                    if self.peek_token == Token::Semicolon {
                        self.next_token();
                    }
                    Ok(Stmt::Expr(expr))
                }
            }
        }
    }

    fn parse_let_statement(&mut self) -> Result<Stmt, String> {
        let name = match &self.peek_token {
            Token::Identifier(id) => id.clone(),
            _ => return Err("Expected identifier after 'let'".to_string()),
        };
        self.next_token();

        self.expect_peek(Token::Assign)?;
        self.next_token(); // move past '='

        let expr = self.parse_expression(Precedence::Lowest)?;
        
        if self.peek_token == Token::Semicolon {
            self.next_token();
        }

        Ok(Stmt::Let(name, expr))
    }

    fn parse_try_catch_statement(&mut self) -> Result<Stmt, String> {
        self.expect_peek(Token::LBrace)?; 
        let try_block = self.parse_block()?;

        self.expect_peek(Token::Catch)?;
        self.expect_peek(Token::LParen)?;
        
        self.next_token();
        let catch_param = match &self.current_token {
            Token::Identifier(id) => id.clone(),
            _ => return Err("Expected identifier for catch parameter".to_string()),
        };
        
        self.expect_peek(Token::RParen)?;
        self.expect_peek(Token::LBrace)?;
        
        let catch_block = self.parse_block()?;
        
        Ok(Stmt::TryCatch(try_block, catch_param, catch_block))
    }

    fn parse_throw_statement(&mut self) -> Result<Stmt, String> {
        self.next_token(); 
        
        let expr = self.parse_expression(Precedence::Lowest)?;
        
        if self.peek_token == Token::Semicolon {
            self.next_token();
        }
        
        Ok(Stmt::Throw(expr))
    }

    fn parse_assign_statement(&mut self) -> Result<Stmt, String> {
        let name = match &self.current_token {
            Token::Identifier(id) => id.clone(),
            _ => unreachable!(),
        };

        self.expect_peek(Token::Assign)?;
        self.next_token();

        let expr = self.parse_expression(Precedence::Lowest)?;

        if self.peek_token == Token::Semicolon {
            self.next_token();
        }

        Ok(Stmt::Assign(name, expr))
    }

    fn parse_assign_index_statement(&mut self) -> Result<Stmt, String> {
        let name = match &self.current_token {
            Token::Identifier(id) => id.clone(),
            _ => unreachable!(),
        };

        self.expect_peek(Token::LBracket)?; // consume '['
        self.next_token(); // move to index expr
        
        let index = self.parse_expression(Precedence::Lowest)?;
        
        self.expect_peek(Token::RBracket)?;
        self.expect_peek(Token::Assign)?;
        self.next_token();

        let value = self.parse_expression(Precedence::Lowest)?;

        if self.peek_token == Token::Semicolon {
            self.next_token();
        }

        Ok(Stmt::AssignIndex(name, index, value))
    }

    fn parse_return_statement(&mut self) -> Result<Stmt, String> {
        if self.peek_token == Token::Semicolon {
            self.next_token();
            Ok(Stmt::Return(None))
        } else {
            self.next_token();
            let expr = self.parse_expression(Precedence::Lowest)?;
            if self.peek_token == Token::Semicolon {
                self.next_token();
            }
            Ok(Stmt::Return(Some(expr)))
        }
    }

    fn parse_if_statement(&mut self) -> Result<Stmt, String> {
        self.next_token(); // move past 'if'
        let condition = self.parse_expression(Precedence::Lowest)?;
        
        self.expect_peek(Token::LBrace)?;
        let consequence = self.parse_block()?;

        let mut alternative = None;
        if self.peek_token == Token::Else {
            self.next_token();
            self.expect_peek(Token::LBrace)?;
            alternative = Some(self.parse_block()?);
        }

        Ok(Stmt::If(condition, consequence, alternative))
    }

    fn parse_while_statement(&mut self) -> Result<Stmt, String> {
        self.next_token(); // move past 'while'
        let condition = self.parse_expression(Precedence::Lowest)?;

        self.expect_peek(Token::LBrace)?;
        let body = self.parse_block()?;

        Ok(Stmt::While(condition, body))
    }

    fn parse_expression(&mut self, precedence: Precedence) -> Result<Expr, String> {
        let mut left = self.parse_prefix()?;

        while self.peek_token != Token::Semicolon && precedence < self.peek_precedence() {
            self.next_token();
            left = self.parse_infix(left)?;
        }

        Ok(left)
    }

    fn parse_prefix(&mut self) -> Result<Expr, String> {
        match &self.current_token {
            Token::Identifier(id) => {
                if self.peek_token == Token::LParen {
                    self.parse_call_expression(id.clone())
                } else {
                    Ok(Expr::Identifier(id.clone()))
                }
            }
            Token::Number(val) => Ok(Expr::Number(*val)),
            Token::String(s) => Ok(Expr::StringLiteral(s.clone())),
            Token::LParen => {
                self.next_token();
                let expr = self.parse_expression(Precedence::Lowest)?;
                self.expect_peek(Token::RParen)?;
                Ok(expr)
            }
            Token::LBracket => {
                self.parse_array_literal()
            }
            Token::LBrace => {
                self.parse_dict_literal()
            }
            Token::New => {
                self.parse_struct_init()
            }
            _ => Err(format!("No prefix parse function for {:?}", self.current_token)),
        }
    }

    fn parse_dict_literal(&mut self) -> Result<Expr, String> {
        let mut elements = Vec::new();
        if self.peek_token != Token::RBrace {
            self.next_token(); // Move to first key
            let key = self.parse_expression(Precedence::Lowest)?;
            
            self.expect_peek(Token::Colon)?;
            self.next_token(); // Move to value
            
            let val = self.parse_expression(Precedence::Lowest)?;
            elements.push((key, val));

            while self.peek_token == Token::Comma {
                self.next_token(); // consume comma
                self.next_token(); // move to key
                let key = self.parse_expression(Precedence::Lowest)?;
                
                self.expect_peek(Token::Colon)?;
                self.next_token(); // Move to value
                let val = self.parse_expression(Precedence::Lowest)?;
                elements.push((key, val));
            }
        }
        self.expect_peek(Token::RBrace)?;
        Ok(Expr::DictLiteral(elements))
    }

    fn parse_struct_init(&mut self) -> Result<Expr, String> {
        self.next_token(); // Move past 'new'
        let name = match &self.current_token {
            Token::Identifier(id) => id.clone(),
            _ => return Err("Expected struct name after new".to_string()),
        };
        
        self.expect_peek(Token::LBrace)?;
        let mut fields = Vec::new();

        if self.peek_token != Token::RBrace {
            self.next_token(); // Move to first key
            let key = match &self.current_token {
                Token::Identifier(id) => id.clone(),
                _ => return Err("Expected field name".to_string()),
            };
            self.expect_peek(Token::Colon)?;
            self.next_token(); // Move to value
            let val = self.parse_expression(Precedence::Lowest)?;
            fields.push((key, val));

            while self.peek_token == Token::Comma {
                self.next_token(); // consume comma
                self.next_token(); // move to key
                let key = match &self.current_token {
                    Token::Identifier(id) => id.clone(),
                    _ => return Err("Expected field name".to_string()),
                };
                self.expect_peek(Token::Colon)?;
                self.next_token();
                let val = self.parse_expression(Precedence::Lowest)?;
                fields.push((key, val));
            }
        }
        self.expect_peek(Token::RBrace)?;

        Ok(Expr::StructInit(name, fields))
    }

    fn parse_array_literal(&mut self) -> Result<Expr, String> {
        let mut elements = Vec::new();
        if self.peek_token != Token::RBracket {
            self.next_token();
            elements.push(self.parse_expression(Precedence::Lowest)?);

            while self.peek_token == Token::Comma {
                self.next_token(); // consume comma
                self.next_token();
                elements.push(self.parse_expression(Precedence::Lowest)?);
            }
        }
        self.expect_peek(Token::RBracket)?;
        Ok(Expr::ArrayLiteral(elements))
    }

    fn parse_call_expression(&mut self, func_name: String) -> Result<Expr, String> {
        self.expect_peek(Token::LParen)?; // Should be '('
        
        let mut args = Vec::new();
        if self.peek_token != Token::RParen {
            self.next_token();
            args.push(self.parse_expression(Precedence::Lowest)?);

            while self.peek_token == Token::Comma {
                self.next_token(); // consume comma
                self.next_token(); // move to next expr
                args.push(self.parse_expression(Precedence::Lowest)?);
            }
        }
        self.expect_peek(Token::RParen)?;

        Ok(Expr::Call(func_name, args))
    }

    fn parse_infix(&mut self, left: Expr) -> Result<Expr, String> {
        let operator = match self.current_token {
            Token::Plus => BinaryOperator::Add,
            Token::Minus => BinaryOperator::Subtract,
            Token::Star => BinaryOperator::Multiply,
            Token::Slash => BinaryOperator::Divide,
            Token::Equals => BinaryOperator::Equals,
            Token::NotEquals => BinaryOperator::NotEquals,
            Token::LessThan => BinaryOperator::LessThan,
            Token::GreaterThan => BinaryOperator::GreaterThan,
            Token::LessThanOrEq => BinaryOperator::LessThanOrEq,
            Token::GreaterThanOrEq => BinaryOperator::GreaterThanOrEq,
            Token::LBracket => return self.parse_index_access(left),
            Token::Dot => return self.parse_property_access(left),
            _ => return Err(format!("Unknown infix operator: {:?}", self.current_token)),
        };

        let precedence = self.current_precedence();
        self.next_token();
        let right = self.parse_expression(precedence)?;

        Ok(Expr::BinaryOp(Box::new(left), operator, Box::new(right)))
    }

    fn parse_index_access(&mut self, left: Expr) -> Result<Expr, String> {
        // current token is '['
        self.next_token();
        let index = self.parse_expression(Precedence::Lowest)?;
        self.expect_peek(Token::RBracket)?;
        Ok(Expr::IndexAccess(Box::new(left), Box::new(index)))
    }

    fn parse_property_access(&mut self, left: Expr) -> Result<Expr, String> {
        self.next_token(); // current token is now the property identifier
        let prop = match &self.current_token {
            Token::Identifier(id) => id.clone(),
            _ => return Err("Expected property name after '.'".to_string()),
        };
        Ok(Expr::PropertyAccess(Box::new(left), prop))
    }

    fn current_precedence(&self) -> Precedence {
        token_precedence(&self.current_token)
    }

    fn peek_precedence(&self) -> Precedence {
        token_precedence(&self.peek_token)
    }
}

#[derive(PartialEq, PartialOrd, Debug, Clone, Copy)]
enum Precedence {
    Lowest,
    Equals,
    LessGreater,
    Sum,
    Product,
    Call,
}

fn token_precedence(token: &Token) -> Precedence {
    match token {
        Token::Equals | Token::NotEquals => Precedence::Equals,
        Token::LessThan | Token::GreaterThan | Token::LessThanOrEq | Token::GreaterThanOrEq => Precedence::LessGreater,
        Token::Plus | Token::Minus => Precedence::Sum,
        Token::Star | Token::Slash => Precedence::Product,
        Token::LParen | Token::LBracket | Token::Dot => Precedence::Call,
        _ => Precedence::Lowest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_program() {
        let input = "
            fn main() {
                let x = 10;
                let y = x + 5 * 2;
                if y > 15 {
                    print(y);
                } else {
                    return 0;
                }
            }
        ";
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();

        assert_eq!(program.functions.len(), 1);
        let main_fn = &program.functions[0];
        assert_eq!(main_fn.name, "main");
        assert_eq!(main_fn.body.len(), 3); // let, let, if
    }
}
