use crate::ast::*;
use crate::lexer_indent::{LexerIndent, Token};

pub struct ParserIndent<'a> {
    lexer: LexerIndent<'a>,
    current_token: Token,
    peek_token: Token,
    current_span: String,
}

impl<'a> ParserIndent<'a> {
    pub fn new(mut lexer: LexerIndent<'a>) -> Self {
        let current_token = lexer.next_token();
        let current_span = lexer.current_span();
        let peek_token = lexer.next_token();
        ParserIndent {
            lexer,
            current_token,
            peek_token,
            current_span,
        }
    }

    fn next_token(&mut self) {
        self.current_token = self.peek_token.clone();
        self.peek_token = self.lexer.next_token();
        self.current_span = self.lexer.current_span();
    }

    /// Prefixes a message with the current source span.
    fn span_err(&self, msg: &str) -> String {
        format!("[{}] {}", self.current_span, msg)
    }

    fn expect_peek(&mut self, token: Token) -> Result<(), String> {
        if self.peek_token == token {
            self.next_token();
            Ok(())
        } else {
            Err(self.span_err(&format!("Expected {:?}, got {:?}", token, self.peek_token)))
        }
    }
    
    fn skip_newlines(&mut self) {
        while self.current_token == Token::Newline {
            self.next_token();
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, String> {
        let mut imports = Vec::new();
        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let mut classes = Vec::new();

        self.skip_newlines();

        while self.current_token != Token::Eof {
            if self.current_token == Token::Import {
                self.next_token();
                let path = match &self.current_token {
                    Token::String(path) => path.clone(),
                    _ => return Err("Expected string literal after import".to_string()),
                };
                let alias = if self.peek_token == Token::As {
                    self.next_token(); // consume 'as'
                    self.next_token(); // move to alias identifier
                    match &self.current_token {
                        Token::Identifier(id) => Some(id.clone()),
                        _ => return Err("Expected identifier after 'as'".to_string()),
                    }
                } else {
                    None
                };
                if self.peek_token == Token::Newline {
                    self.next_token();
                }
                imports.push(crate::ast::ImportDecl { path, alias });
            } else if self.current_token == Token::Fn {
                functions.push(self.parse_function()?);
            } else if self.current_token == Token::Struct {
                structs.push(self.parse_struct_decl()?);
            } else if self.current_token == Token::Class {
                classes.push(self.parse_class_decl()?);
            } else {
                return Err(format!("Unexpected token at top level: {:?}", self.current_token));
            }
            self.next_token();
            self.skip_newlines();
        }

        Ok(Program { imports, functions, structs, classes })
    }

    fn parse_type(&mut self) -> Result<Type, String> {
        match &self.current_token {
            Token::Identifier(id) => match id.as_str() {
                "Int32" => Ok(Type::Int32),
                "Int64" => Ok(Type::Int64),
                "Float32" => Ok(Type::Float32),
                "Float64" => Ok(Type::Float64),
                "Bool" => Ok(Type::Bool),
                "String" => Ok(Type::String),
                "Any" => Ok(Type::Any),
                "Array" => {
                    if self.peek_token == Token::LessThan {
                        self.next_token(); // move to '<'
                        self.next_token(); // move to inner type
                        let elem_ty = self.parse_type()?;
                        self.expect_peek(Token::GreaterThan)?;
                        Ok(Type::Array(Box::new(elem_ty)))
                    } else {
                        Ok(Type::Array(Box::new(Type::Any)))
                    }
                },
                "Dict" => {
                    if self.peek_token == Token::LessThan {
                        self.next_token(); // move to '<'
                        self.next_token(); // move to key type
                        let key_ty = self.parse_type()?;
                        self.expect_peek(Token::Comma)?;
                        self.next_token(); // move to val type
                        let val_ty = self.parse_type()?;
                        self.expect_peek(Token::GreaterThan)?;
                        Ok(Type::Dict(Box::new(key_ty), Box::new(val_ty)))
                    } else {
                        Ok(Type::Dict(Box::new(Type::Any), Box::new(Type::Any)))
                    }
                },
                _ => Ok(Type::Struct(id.clone())),
            },
            Token::LBracket => {
                self.next_token(); // move past '['
                let elem_ty = self.parse_type()?;
                self.expect_peek(Token::RBracket)?;
                Ok(Type::Array(Box::new(elem_ty)))
            },
            _ => Err("Expected type identifier".to_string()),
        }
    }

    fn parse_struct_decl(&mut self) -> Result<Stmt, String> {
        // Current is 'struct'
        self.next_token();
        let name = match &self.current_token {
            Token::Identifier(id) => id.clone(),
            _ => return Err("Expected struct name".to_string()),
        };
        self.expect_peek(Token::Colon)?;
        self.expect_peek(Token::Newline)?;
        self.expect_peek(Token::Indent)?;
        
        let mut fields = Vec::new();
        if self.peek_token != Token::Dedent {
            self.next_token();
            let field_name = match &self.current_token {
                Token::Identifier(id) => id.clone(),
                _ => return Err("Expected field name".to_string()),
            };

            let mut field_type = None;
            if self.peek_token == Token::Colon {
                self.next_token(); // colon
                self.next_token(); // id
                field_type = Some(self.parse_type()?);
            }
            fields.push((field_name, field_type));

            while self.peek_token == Token::Newline {
                self.next_token(); // newline
                if self.peek_token == Token::Dedent {
                    break;
                }
                self.next_token(); // move to id
                let field_name = match &self.current_token {
                    Token::Identifier(id) => id.clone(),
                    _ => return Err("Expected field name after newline".to_string()),
                };

                let mut field_type = None;
                if self.peek_token == Token::Colon {
                    self.next_token(); // colon
                    self.next_token(); // id
                    field_type = Some(self.parse_type()?);
                }
                fields.push((field_name, field_type));
            }
        }
        self.expect_peek(Token::Dedent)?;
        Ok(Stmt::StructDecl(name, fields))
    }

    fn parse_class_decl(&mut self) -> Result<Stmt, String> {
        self.next_token(); // move past 'class'
        let name = match &self.current_token {
            Token::Identifier(id) => id.clone(),
            _ => return Err(format!("Expected class name, got {:?}", self.current_token)),
        };

        self.expect_peek(Token::Colon)?;
        self.expect_peek(Token::Newline)?;
        self.expect_peek(Token::Indent)?;
        
        let mut fields = Vec::new();
        let mut methods = Vec::new();

        self.next_token(); // move into block

        while self.current_token != Token::Dedent && self.current_token != Token::Eof {
            if self.current_token == Token::Newline {
                self.next_token();
                continue;
            }

            if self.current_token == Token::Fn {
                methods.push(self.parse_function()?);
            } else if let Token::Identifier(id) = &self.current_token {
                let field_name = id.clone();
                let mut field_type = None;
                if self.peek_token == Token::Colon {
                    self.next_token(); // move to colon
                    self.next_token(); // move to type id
                    field_type = Some(self.parse_type()?);
                }
                fields.push((field_name, field_type));
            } else {
                return Err(format!("Expected field or method in class, got {:?}", self.current_token));
            }
            self.next_token(); // Move to next field/method or Newline
        }

        if self.current_token != Token::Dedent {
            return Err("Expected Dedent at end of class declaration".to_string());
        }

        Ok(Stmt::ClassDecl(name, fields, methods))
    }

    fn parse_function(&mut self) -> Result<Function, String> {
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
            let param_name = match &self.current_token {
                Token::Identifier(id) => id.clone(),
                _ => return Err("Expected parameter name".to_string()),
            };
            
            let mut param_type = None;
            if self.peek_token == Token::Colon {
                self.next_token(); // Move to colon
                self.next_token(); // Move to type identifier
                param_type = Some(self.parse_type()?);
            }
            params.push((param_name, param_type));

            while self.peek_token == Token::Comma {
                self.next_token(); // consume comma
                self.next_token(); // move to id
                let param_name = match &self.current_token {
                    Token::Identifier(id) => id.clone(),
                    _ => return Err("Expected parameter name after comma".to_string()),
                };
                
                let mut param_type = None;
                if self.peek_token == Token::Colon {
                    self.next_token(); // Move to colon
                    self.next_token(); // Move to type identifier
                    param_type = Some(self.parse_type()?);
                }
                params.push((param_name, param_type));
            }
        }
        self.expect_peek(Token::RParen)?;
        
        let mut return_type = None;
        if self.peek_token == Token::ReturnArrow {
            self.next_token(); // Move to ->
            self.next_token(); // Move to type identifier
            return_type = Some(self.parse_type()?);
        }
        
        self.expect_peek(Token::Colon)?;
        self.expect_peek(Token::Newline)?;
        self.expect_peek(Token::Indent)?;

        let body = self.parse_block()?;

        Ok(Function {
            name,
            params,
            return_type,
            body,
        })
    }

    fn parse_block(&mut self) -> Result<Block, String> {
        // Assumes current token is Indent
        self.next_token(); // Move past Indent
        let mut stmts = Vec::new();

        while self.current_token != Token::Dedent && self.current_token != Token::Eof {
            if self.current_token == Token::Newline {
                self.next_token();
                continue;
            }
            stmts.push(self.parse_statement()?);
            self.next_token();
        }

        if self.current_token != Token::Dedent {
            return Err("Expected Dedent at end of block".to_string());
        }

        Ok(stmts)
    }

    fn parse_statement(&mut self) -> Result<Stmt, String> {
        match self.current_token {
            Token::Let => self.parse_let_statement(),
            Token::Return => self.parse_return_statement(),
            Token::If => self.parse_if_statement(),
            Token::While => self.parse_while_statement(),
            Token::For => self.parse_for_statement(),
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
                    if self.peek_token == Token::Newline || self.peek_token == Token::Semicolon {
                        self.next_token();
                    }
                    match expr {
                        Expr::Identifier(name, _) => Ok(Stmt::Assign(name, rhs)),
                        Expr::PropertyAccess(obj, prop, _) => Ok(Stmt::AssignProperty(obj, prop, rhs)),
                        Expr::IndexAccess(arr, idx) => {
                            if let Expr::Identifier(name, _) = *arr {
                                Ok(Stmt::AssignIndex(name, *idx, rhs))
                            } else {
                                Err("Invalid array assignment target".to_string())
                            }
                        }
                        _ => Err("Invalid assignment target".to_string()),
                    }
                } else {
                    if self.peek_token == Token::Newline || self.peek_token == Token::Semicolon {
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

        let mut type_annotation = None;
        if self.peek_token == Token::Colon {
            self.next_token(); // Move to colon
            self.next_token(); // Move to type identifier
            type_annotation = Some(self.parse_type()?);
        }

        self.expect_peek(Token::Assign)?;
        self.next_token(); // move past '='

        let expr = self.parse_expression(Precedence::Lowest)?;
        
        if self.peek_token == Token::Newline || self.peek_token == Token::Semicolon {
            self.next_token();
        }

        Ok(Stmt::Let(name, type_annotation, expr))
    }

    fn parse_try_catch_statement(&mut self) -> Result<Stmt, String> {
        self.expect_peek(Token::Colon)?; 
        self.expect_peek(Token::Newline)?; 
        self.expect_peek(Token::Indent)?; 
        let try_block = self.parse_block()?;

        // After parse_block, current is Dedent, so peek might be Catch if on same line, but typically it's Newline then Catch.
        // Actually, in Python-style, catch is dedented to match try.
        if self.peek_token == Token::Newline {
            self.next_token();
        }
        self.expect_peek(Token::Catch)?;
        
        // Vyauma Python-style syntax: `except Exception as e:` or `catch (err):`
        // Let's use `catch (err):` to match lexer tokens
        self.expect_peek(Token::LParen)?;
        
        self.next_token();
        let catch_param = match &self.current_token {
            Token::Identifier(id) => id.clone(),
            _ => return Err("Expected identifier for catch parameter".to_string()),
        };
        
        self.expect_peek(Token::RParen)?;
        self.expect_peek(Token::Colon)?;
        self.expect_peek(Token::Newline)?;
        self.expect_peek(Token::Indent)?;
        
        let catch_block = self.parse_block()?;
        
        Ok(Stmt::TryCatch(try_block, catch_param, catch_block))
    }

    fn parse_throw_statement(&mut self) -> Result<Stmt, String> {
        self.next_token(); 
        
        let expr = self.parse_expression(Precedence::Lowest)?;
        
        if self.peek_token == Token::Newline || self.peek_token == Token::Semicolon {
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

        if self.peek_token == Token::Newline || self.peek_token == Token::Semicolon {
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

        if self.peek_token == Token::Newline || self.peek_token == Token::Semicolon {
            self.next_token();
        }

        Ok(Stmt::AssignIndex(name, index, value))
    }

    fn parse_return_statement(&mut self) -> Result<Stmt, String> {
        if self.peek_token == Token::Newline || self.peek_token == Token::Semicolon {
            self.next_token();
            Ok(Stmt::Return(None))
        } else {
            self.next_token();
            let expr = self.parse_expression(Precedence::Lowest)?;
            if self.peek_token == Token::Newline || self.peek_token == Token::Semicolon {
                self.next_token();
            }
            Ok(Stmt::Return(Some(expr)))
        }
    }

    fn parse_if_statement(&mut self) -> Result<Stmt, String> {
        self.next_token(); // move past 'if'
        let condition = self.parse_expression(Precedence::Lowest)?;
        
        self.expect_peek(Token::Colon)?;
        self.expect_peek(Token::Newline)?;
        self.expect_peek(Token::Indent)?;
        let consequence = self.parse_block()?;

        let mut alternative = None;
        if self.peek_token == Token::Newline {
            self.next_token();
        }
        if self.peek_token == Token::Else {
            self.next_token();
            self.expect_peek(Token::Colon)?;
            self.expect_peek(Token::Newline)?;
            self.expect_peek(Token::Indent)?;
            alternative = Some(self.parse_block()?);
        }

        Ok(Stmt::If(condition, consequence, alternative))
    }

    fn parse_while_statement(&mut self) -> Result<Stmt, String> {
        self.next_token(); // move past 'while'
        let condition = self.parse_expression(Precedence::Lowest)?;

        self.expect_peek(Token::Colon)?;
        self.expect_peek(Token::Newline)?;
        self.expect_peek(Token::Indent)?;
        let body = self.parse_block()?;

        Ok(Stmt::While(condition, body))
    }

    fn parse_for_statement(&mut self) -> Result<Stmt, String> {
        self.next_token(); // move past 'for'
        
        let init = self.parse_statement()?; 
        if self.current_token == Token::Semicolon {
            self.next_token(); // move to condition
        } else {
            return Err("Expected ';' after for loop init".to_string());
        }
        
        let condition = self.parse_expression(Precedence::Lowest)?;
        self.expect_peek(Token::Semicolon)?; 
        
        self.next_token(); // move to increment
        let increment = self.parse_statement()?;
        
        if self.peek_token == Token::Colon {
            self.expect_peek(Token::Colon)?;
        } else if self.current_token == Token::Colon {
            // Already at colon
        } else {
            return Err("Expected ':' after for loop header".to_string());
        }
        
        self.expect_peek(Token::Newline)?;
        self.expect_peek(Token::Indent)?;
        
        let body = self.parse_block()?;

        Ok(Stmt::For(Box::new(init), condition, Box::new(increment), body))
    }

    fn parse_expression(&mut self, precedence: Precedence) -> Result<Expr, String> {
        let mut left = self.parse_prefix()?;

        while self.peek_token != Token::Newline && self.peek_token != Token::Semicolon && self.peek_token != Token::Eof && precedence < self.peek_precedence() {
            self.next_token();
            left = self.parse_infix(left)?;
        }

        Ok(left)
    }

    fn parse_prefix(&mut self) -> Result<Expr, String> {
        match &self.current_token {
            Token::Identifier(id) => {
                let id = id.clone();
                if self.peek_token == Token::DoubleColon {
                    // namespace::function_name(...)
                    self.next_token(); // consume '::'
                    self.next_token(); // move to function name
                    let func_name = match &self.current_token {
                        Token::Identifier(fname) => fname.clone(),
                        _ => return Err("Expected function name after '::'".to_string()),
                    };
                    let mangled = format!("{}__{}" , id, func_name);
                    self.parse_call_expression(mangled)
                } else if self.peek_token == Token::LParen {
                    self.parse_call_expression(id)
                } else {
                    Ok(Expr::Identifier(id, None))
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
                self.parse_new_expression()
            }
            _ => Err(format!("No prefix parse function for {:?}", self.current_token)),
        }
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

    fn parse_new_expression(&mut self) -> Result<Expr, String> {
        self.next_token(); // Move past 'new'
        let name = match &self.current_token {
            Token::Identifier(id) => id.clone(),
            _ => return Err("Expected type name after new".to_string()),
        };
        
        if self.peek_token == Token::LParen {
            self.next_token(); // move to '('
            self.next_token(); // move inside '('
            
            let mut args = Vec::new();
            if self.current_token != Token::RParen {
                args.push(self.parse_expression(Precedence::Lowest)?);

                while self.peek_token == Token::Comma {
                    self.next_token(); // consume comma
                    self.next_token(); // move to next expr
                    args.push(self.parse_expression(Precedence::Lowest)?);
                }
            }
            if self.peek_token == Token::RParen {
                self.next_token(); // consume ')'
            } else if self.current_token != Token::RParen {
                return Err("Expected ')'".to_string());
            }
            
            Ok(Expr::NewClass(name, args))
        } else {
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

        Ok(Expr::Call(func_name, args, None))
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
            Token::And => BinaryOperator::And,
            Token::Or => BinaryOperator::Or,
            Token::LBracket => return self.parse_index_access(left),
            Token::Dot => return self.parse_property_access(left),
            _ => return Err(format!("Unknown infix operator: {:?}", self.current_token)),
        };

        let precedence = self.current_precedence();
        self.next_token();
        let right = self.parse_expression(precedence)?;

        Ok(Expr::BinaryOp(Box::new(left), operator, Box::new(right), None))
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

        if self.peek_token == Token::LParen {
            self.next_token(); // move to '('
            self.next_token(); // move inside '('
            
            let mut args = Vec::new();
            if self.current_token != Token::RParen {
                args.push(self.parse_expression(Precedence::Lowest)?);

                while self.peek_token == Token::Comma {
                    self.next_token(); // consume comma
                    self.next_token(); // move to next expr
                    args.push(self.parse_expression(Precedence::Lowest)?);
                }
            }
            if self.peek_token == Token::RParen {
                self.next_token(); // move to ')'
            } else if self.current_token != Token::RParen {
                return Err("Expected ')' after method arguments".to_string());
            }
            
            Ok(Expr::MethodCall(Box::new(left), prop, args, None))
        } else {
            Ok(Expr::PropertyAccess(Box::new(left), prop, None))
        }
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
    LogicalOr,
    LogicalAnd,
    Equals,
    LessGreater,
    Sum,
    Product,
    Call,
}

fn token_precedence(token: &Token) -> Precedence {
    match token {
        Token::Or => Precedence::LogicalOr,
        Token::And => Precedence::LogicalAnd,
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
    fn test_parse_program_indent() {
        let input = r#"
fn main():
    let x: Int32 = 10
    let y = x + 5 * 2
    if y > 15:
        print(y)
    else:
        return 0
"#;
        let lexer = LexerIndent::new(input);
        let mut parser = ParserIndent::new(lexer);
        let program = parser.parse_program().unwrap();

        assert_eq!(program.functions.len(), 1);
        let main_fn = &program.functions[0];
        assert_eq!(main_fn.name, "main");
        assert_eq!(main_fn.body.len(), 3); // let, let, if
    }

    #[test]
    fn test_parse_for_loop_indent() {
        let input = r#"
fn run_loop():
    for let i = 0; i < 10; i = i + 1:
        print(i)
"#;
        let lexer = LexerIndent::new(input);
        let mut parser = ParserIndent::new(lexer);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.functions.len(), 1);
        let body = &program.functions[0].body;
        assert_eq!(body.len(), 1);
        match &body[0] {
            Stmt::For(_, _, _, _) => {}
            _ => panic!("Expected For statement"),
        }
    }

    #[test]
    fn test_parse_class_decl_indent() {
        let input = r#"
class Person:
    name: String
    age: Int32
    
    fn greet():
        print("hello")
"#;
        let lexer = LexerIndent::new(input);
        let mut parser = ParserIndent::new(lexer);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.classes.len(), 1);
        match &program.classes[0] {
            Stmt::ClassDecl(name, fields, methods) => {
                assert_eq!(name, "Person");
                assert_eq!(fields.len(), 2);
                assert_eq!(methods.len(), 1);
            }
            _ => panic!("Expected ClassDecl"),
        }
    }
}
