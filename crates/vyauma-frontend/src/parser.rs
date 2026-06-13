use crate::ast::*;
use crate::lexer::{Token, TokenKind};
use std::collections::HashMap;

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, current: 0 }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::EOF
    }

    fn check(&self, kind: TokenKind) -> bool {
        if self.is_at_end() {
            false
        } else {
            self.peek().kind == kind
        }
    }

    fn match_token(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume(&mut self, kind: TokenKind, message: &str) -> Result<&Token, String> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(format!("{} at line {}:{}", message, self.peek().line, self.peek().col))
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Statement>, String> {
        let mut statements = Vec::new();
        while !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }
        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
        if self.match_token(TokenKind::Import) {
            self.parse_import()
        } else if self.match_token(TokenKind::Struct) {
            self.parse_struct()
        } else if self.match_token(TokenKind::Fn) {
            self.parse_function()
        } else if self.match_token(TokenKind::Let) {
            self.parse_variable()
        } else if self.match_token(TokenKind::Return) {
            self.parse_return()
        } else {
            let expr = self.parse_expression()?;
            if self.check(TokenKind::SemiColon) {
                self.advance();
            }
            Ok(Statement::Expression(expr))
        }
    }

    fn parse_import(&mut self) -> Result<Statement, String> {
        // Handle `import math` or `import utils.validation`
        let mut path_parts = Vec::new();
        
        loop {
            if let TokenKind::Identifier(ref name) = self.peek().kind.clone() {
                path_parts.push(name.clone());
                self.advance();
            } else {
                return Err("Expected identifier in import path".into());
            }

            if self.match_token(TokenKind::Dot) {
                continue;
            } else {
                break;
            }
        }
        
        // optional semicolon
        if self.check(TokenKind::SemiColon) {
            self.advance();
        }

        Ok(Statement::Import(ImportStmt {
            path: path_parts.join("."),
        }))
    }

    fn parse_struct(&mut self) -> Result<Statement, String> {
        let name = if let TokenKind::Identifier(ref n) = self.peek().kind.clone() {
            n.clone()
        } else {
            return Err("Expected struct name".into());
        };
        self.advance();

        self.consume(TokenKind::LBrace, "Expected '{' before struct body")?;
        
        let mut fields = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            let field_name = if let TokenKind::Identifier(ref n) = self.peek().kind.clone() {
                n.clone()
            } else {
                return Err("Expected field name".into());
            };
            self.advance();

            self.consume(TokenKind::Colon, "Expected ':' after field name")?;

            let field_type = if let TokenKind::Identifier(ref n) = self.peek().kind.clone() {
                n.clone()
            } else {
                return Err("Expected field type".into());
            };
            self.advance();

            fields.push(Parameter {
                name: field_name,
                param_type: field_type,
            });

            // Optional comma or newline (in our minimal lexer, we ignore whitespace)
            // But we allow commas or semicolons between struct fields
            if self.check(TokenKind::Comma) || self.check(TokenKind::SemiColon) {
                self.advance();
            }
        }

        self.consume(TokenKind::RBrace, "Expected '}' after struct body")?;

        Ok(Statement::Struct(StructDecl { name, fields }))
    }

    fn parse_function(&mut self) -> Result<Statement, String> {
        let name = if let TokenKind::Identifier(ref n) = self.peek().kind.clone() {
            n.clone()
        } else {
            return Err("Expected function name".into());
        };
        self.advance();

        self.consume(TokenKind::LParen, "Expected '(' after function name")?;
        
        let mut params = Vec::new();
        if !self.check(TokenKind::RParen) {
            loop {
                let param_name = if let TokenKind::Identifier(ref n) = self.peek().kind.clone() {
                    n.clone()
                } else {
                    return Err("Expected parameter name".into());
                };
                self.advance();

                self.consume(TokenKind::Colon, "Expected ':' after parameter name")?;

                let param_type = if let TokenKind::Identifier(ref n) = self.peek().kind.clone() {
                    n.clone()
                } else {
                    return Err("Expected parameter type".into());
                };
                self.advance();

                params.push(Parameter {
                    name: param_name,
                    param_type,
                });

                if !self.match_token(TokenKind::Comma) {
                    break;
                }
            }
        }
        self.consume(TokenKind::RParen, "Expected ')' after parameters")?;

        let mut return_type = None;
        if self.match_token(TokenKind::Arrow) {
            if let TokenKind::Identifier(ref n) = self.peek().kind.clone() {
                return_type = Some(n.clone());
                self.advance();
            } else {
                return Err("Expected return type".into());
            }
        }

        self.consume(TokenKind::LBrace, "Expected '{' before function body")?;
        
        let mut statements = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }
        
        self.consume(TokenKind::RBrace, "Expected '}' after function body")?;

        Ok(Statement::Function(FunctionDecl {
            name,
            params,
            return_type,
            body: Block { statements },
        }))
    }

    fn parse_variable(&mut self) -> Result<Statement, String> {
        let name = if let TokenKind::Identifier(ref n) = self.peek().kind.clone() {
            n.clone()
        } else {
            return Err("Expected variable name".into());
        };
        self.advance();

        self.consume(TokenKind::Equals, "Expected '=' in variable declaration")?;

        let value = self.parse_expression()?;

        if self.check(TokenKind::SemiColon) {
            self.advance();
        }

        Ok(Statement::Variable(VariableDecl { name, value }))
    }

    fn parse_return(&mut self) -> Result<Statement, String> {
        let value = if !self.check(TokenKind::SemiColon) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        if self.check(TokenKind::SemiColon) {
            self.advance();
        }

        Ok(Statement::Return(ReturnStmt { value }))
    }

    fn parse_expression(&mut self) -> Result<Expression, String> {
        // Pratt parsing is standard, but for the required syntax:
        // `user.name`, `greet(user.name)`, `User(name="Manvirr", age=30)`
        // We only need literal, identifier, member access, and function calls.
        
        let mut expr = self.parse_primary()?;

        loop {
            if self.match_token(TokenKind::Dot) {
                let member = if let TokenKind::Identifier(ref n) = self.peek().kind.clone() {
                    n.clone()
                } else {
                    return Err("Expected member name after '.'".into());
                };
                self.advance();
                expr = Expression::MemberAccess {
                    object: Box::new(expr),
                    member,
                };
            } else if self.match_token(TokenKind::LParen) {
                let mut args = Vec::new();
                let mut named_args = HashMap::new();
                
                if !self.check(TokenKind::RParen) {
                    loop {
                        // check if named arg: identifier = expr
                        let mut is_named = false;
                        if let TokenKind::Identifier(ref name) = self.peek().kind.clone() {
                            // lookahead 1
                            if self.tokens[self.current + 1].kind == TokenKind::Equals {
                                is_named = true;
                                self.advance(); // consume ident
                                self.advance(); // consume equals
                                let val = self.parse_expression()?;
                                named_args.insert(name.clone(), val);
                            }
                        }
                        
                        if !is_named {
                            args.push(self.parse_expression()?);
                        }

                        if !self.match_token(TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.consume(TokenKind::RParen, "Expected ')' after arguments")?;
                expr = Expression::Call {
                    callee: Box::new(expr),
                    args,
                    named_args,
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expression, String> {
        let token = self.advance();
        match &token.kind {
            TokenKind::String(s) => Ok(Expression::Literal(LiteralValue::String(s.clone()))),
            TokenKind::Integer(i) => Ok(Expression::Literal(LiteralValue::Integer(*i))),
            TokenKind::Float(f) => Ok(Expression::Literal(LiteralValue::Float(*f))),
            TokenKind::Boolean(b) => Ok(Expression::Literal(LiteralValue::Boolean(*b))),
            TokenKind::Identifier(s) => Ok(Expression::Identifier(s.clone())),
            _ => Err(format!("Unexpected token {:?} at {}:{}", token.kind, token.line, token.col)),
        }
    }
}
