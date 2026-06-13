use crate::ast::*;
use crate::lexer::{Lexer, Token, TokenKind};

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: Token,
    peek_token: Token,
    current_span: String,
}

impl<'a> Parser<'a> {
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let current_token = lexer.next_token();
        let current_span = lexer.current_span();
        let peek_token = lexer.next_token();
        Parser {
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

    fn expect_peek(&mut self, token: TokenKind) -> Result<(), String> {
        if self.peek_token.kind == token {
            self.next_token();
            Ok(())
        } else {
            Err(self.span_err(&format!("Expected {:?}, got {:?}", token, self.peek_token)))
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, String> {
        let mut imports = Vec::new();
        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let mut classes = Vec::new();

        while self.current_token.kind != TokenKind::Eof {
            if self.current_token.kind == TokenKind::Import {
                self.next_token();
                let path = match &self.current_token.kind {
                    TokenKind::String(path) => path.clone(),
                    _ => return Err("Expected string literal after import".to_string()),
                };
                // Optional: `as alias`
                let alias = if self.peek_token.kind == TokenKind::As {
                    self.next_token(); // consume 'as'
                    self.next_token(); // move to alias identifier
                    match &self.current_token.kind {
                        TokenKind::Identifier(id) => Some(id.clone()),
                        _ => return Err("Expected identifier after 'as'".to_string()),
                    }
                } else {
                    None
                };
                if self.peek_token.kind == TokenKind::Semicolon {
                    self.next_token();
                }
                imports.push(crate::ast::ImportDecl { path, alias });
            } else if self.current_token.kind == TokenKind::Fn {
                functions.push(self.parse_function()?);
            } else if self.current_token.kind == TokenKind::Struct {
                structs.push(self.parse_struct_decl()?);
            } else if self.current_token.kind == TokenKind::Class {
                classes.push(self.parse_class_decl()?);
            } else if self.current_token.kind == TokenKind::Export {
                self.next_token();
                if self.current_token.kind == TokenKind::Fn {
                    let mut func = self.parse_function()?;
                    func.is_exported = true;
                    functions.push(func);
                } else if self.current_token.kind == TokenKind::Struct {
                    let mut s = self.parse_struct_decl()?;
                    if let Stmt::StructDecl(_, _, ref mut exp) = s {
                        *exp = true;
                    }
                    structs.push(s);
                } else if self.current_token.kind == TokenKind::Class {
                    let mut c = self.parse_class_decl()?;
                    if let Stmt::ClassDecl(_, _, _, ref mut exp) = c {
                        *exp = true;
                    }
                    classes.push(c);
                } else {
                    return Err(format!("Unexpected token after export: {:?}", self.current_token));
                }
            } else {
                return Err(format!("Unexpected token at top level: {:?}", self.current_token));
            }
            self.next_token();
        }

        Ok(Program { imports, functions, structs, classes })
    }

    fn parse_type(&mut self) -> Result<Type, String> {
        match &self.current_token.kind {
            TokenKind::Identifier(id) => match id.as_str() {
                "Int32" => Ok(Type::Int32),
                "Int64" => Ok(Type::Int64),
                "Float32" => Ok(Type::Float32),
                "Float64" => Ok(Type::Float64),
                "Bool" => Ok(Type::Bool),
                "String" => Ok(Type::String),
                "Any" => Ok(Type::Any),
                "Array" => {
                    if self.peek_token.kind == TokenKind::LessThan {
                        self.next_token(); // move to '<'
                        self.next_token(); // move to inner type
                        let elem_ty = self.parse_type()?;
                        self.expect_peek(TokenKind::GreaterThan)?;
                        Ok(Type::Array(Box::new(elem_ty)))
                    } else {
                        Ok(Type::Array(Box::new(Type::Any)))
                    }
                },
                "Dict" => {
                    if self.peek_token.kind == TokenKind::LessThan {
                        self.next_token(); // move to '<'
                        self.next_token(); // move to key type
                        let key_ty = self.parse_type()?;
                        self.expect_peek(TokenKind::Comma)?;
                        self.next_token(); // move to val type
                        let val_ty = self.parse_type()?;
                        self.expect_peek(TokenKind::GreaterThan)?;
                        Ok(Type::Dict(Box::new(key_ty), Box::new(val_ty)))
                    } else {
                        Ok(Type::Dict(Box::new(Type::Any), Box::new(Type::Any)))
                    }
                },
                _ => Ok(Type::Struct(id.clone())),
            },
            TokenKind::LBracket => {
                self.next_token(); // move past '['
                let elem_ty = self.parse_type()?;
                self.expect_peek(TokenKind::RBracket)?;
                Ok(Type::Array(Box::new(elem_ty)))
            },
            _ => Err("Expected type identifier".to_string()),
        }
    }

    fn parse_struct_decl(&mut self) -> Result<Stmt, String> {
        // Current is 'struct'
        self.next_token();
        let name = match &self.current_token.kind {
            TokenKind::Identifier(id) => id.clone(),
            _ => return Err("Expected struct name".to_string()),
        };
        self.expect_peek(TokenKind::LBrace)?;
        
        let mut fields = Vec::new();
        if self.peek_token.kind != TokenKind::RBrace {
            self.next_token();
            let field_name = match &self.current_token.kind {
                TokenKind::Identifier(id) => id.clone(),
                _ => return Err("Expected field name".to_string()),
            };

            let mut field_type = None;
            if self.peek_token.kind == TokenKind::Colon {
                self.next_token(); // comma
                self.next_token(); // id
                field_type = Some(self.parse_type()?);
            }
            fields.push((field_name, field_type));

            while self.peek_token.kind == TokenKind::Comma {
                self.next_token(); // comma
                self.next_token(); // id
                let field_name = match &self.current_token.kind {
                    TokenKind::Identifier(id) => id.clone(),
                    _ => return Err("Expected field name after comma".to_string()),
                };

                let mut field_type = None;
                if self.peek_token.kind == TokenKind::Colon {
                    self.next_token(); // comma
                    self.next_token(); // id
                    field_type = Some(self.parse_type()?);
                }
                fields.push((field_name, field_type));
            }
        }
        self.expect_peek(TokenKind::RBrace)?;
        Ok(Stmt::StructDecl(name, fields, false))
    }

    fn parse_class_decl(&mut self) -> Result<Stmt, String> {
        let name = match &self.peek_token.kind {
            TokenKind::Identifier(id) => id.clone(),
            _ => return Err(format!("Expected class name, got {:?}", self.peek_token)),
        };
        self.next_token();

        self.expect_peek(TokenKind::LBrace)?;
        
        let mut fields = Vec::new();
        let mut methods = Vec::new();

        self.next_token(); // Move into class body

        while self.current_token.kind != TokenKind::RBrace && self.current_token.kind != TokenKind::Eof {
            if self.current_token.kind == TokenKind::Fn {
                methods.push(self.parse_function()?);
            } else if let TokenKind::Identifier(id) = &self.current_token.kind {
                let field_name = id.clone();
                let mut field_type = None;
                if self.peek_token.kind == TokenKind::Colon {
                    self.next_token();
                    self.next_token();
                    field_type = Some(self.parse_type()?);
                }
                fields.push((field_name, field_type));

                if self.peek_token.kind == TokenKind::Comma || self.peek_token.kind == TokenKind::Semicolon {
                    self.next_token();
                }
            } else {
                return Err(format!("Expected field or method in class, got {:?}", self.current_token));
            }
            self.next_token();
        }

        if self.current_token.kind != TokenKind::RBrace {
            return Err("Expected '}' at end of class declaration".to_string());
        }

        Ok(Stmt::ClassDecl(name, fields, methods, false))
    }

    fn parse_function(&mut self) -> Result<Function, String> {
        // Current is 'fn'
        
        // Name
        let name = match &self.peek_token.kind {
            TokenKind::Identifier(id) => id.clone(),
            _ => return Err(format!("Expected function name, got {:?}", self.peek_token)),
        };
        self.next_token();

        self.expect_peek(TokenKind::LParen)?;

        let mut params = Vec::new();
        if self.peek_token.kind != TokenKind::RParen {
            self.next_token();
            let param_name = match &self.current_token.kind {
                TokenKind::Identifier(id) => id.clone(),
                _ => return Err("Expected parameter name".to_string()),
            };
            
            let mut param_type = None;
            if self.peek_token.kind == TokenKind::Colon {
                self.next_token(); // Move to colon
                self.next_token(); // Move to type identifier
                param_type = Some(self.parse_type()?);
            }
            params.push((param_name, param_type));

            while self.peek_token.kind == TokenKind::Comma {
                self.next_token(); // consume comma
                self.next_token(); // move to id
                let param_name = match &self.current_token.kind {
                    TokenKind::Identifier(id) => id.clone(),
                    _ => return Err("Expected parameter name after comma".to_string()),
                };
                
                let mut param_type = None;
                if self.peek_token.kind == TokenKind::Colon {
                    self.next_token(); // Move to colon
                    self.next_token(); // Move to type identifier
                    param_type = Some(self.parse_type()?);
                }
                params.push((param_name, param_type));
            }
        }
        self.expect_peek(TokenKind::RParen)?;
        
        let mut return_type = None;
        if self.peek_token.kind == TokenKind::ReturnArrow {
            self.next_token(); // Move to ->
            self.next_token(); // Move to type identifier
            return_type = Some(self.parse_type()?);
        }
        
        self.expect_peek(TokenKind::LBrace)?;

        let body = self.parse_block()?;

        Ok(Function {
            name,
            params,
            return_type,
            body,
            is_exported: false,
        })
    }

    fn parse_block(&mut self) -> Result<Block, String> {
        self.next_token(); // Move past '{'
        let mut stmts = Vec::new();

        while self.current_token.kind != TokenKind::RBrace && self.current_token.kind != TokenKind::Eof {
            stmts.push(self.parse_statement()?);
            self.next_token();
        }

        if self.current_token.kind != TokenKind::RBrace {
            return Err("Expected '}' at end of block".to_string());
        }

        Ok(stmts)
    }

    fn parse_statement(&mut self) -> Result<Stmt, String> {
        match &self.current_token.kind {
            TokenKind::Let => self.parse_let_statement(),
            TokenKind::Return => self.parse_return_statement(),
            TokenKind::If => self.parse_if_statement(),
            TokenKind::While => self.parse_while_statement(),
            TokenKind::For => self.parse_for_statement(),
            TokenKind::Try => self.parse_try_catch_statement(),
            TokenKind::Throw => self.parse_throw_statement(),
            TokenKind::Yield => {
                // `yield;` — suspend current task
                if self.peek_token.kind == TokenKind::Semicolon {
                    self.next_token();
                }
                Ok(Stmt::Yield)
            }
            TokenKind::Identifier(_) if self.peek_token.kind == TokenKind::Assign => self.parse_assign_statement(),
            TokenKind::Identifier(_) if self.peek_token.kind == TokenKind::LBracket => self.parse_assign_index_statement(),
            _ => {
                let expr = self.parse_expression(Precedence::Lowest)?;
                
                if self.peek_token.kind == TokenKind::Assign {
                    self.next_token(); // move to '='
                    self.next_token(); // move to rhs
                    let rhs = self.parse_expression(Precedence::Lowest)?;
                    if self.peek_token.kind == TokenKind::Semicolon {
                        self.next_token();
                    }
                    match expr {
                        Expr::Identifier(name, _) => Ok(Stmt::Assign(name, rhs)),
                        Expr::PropertyAccess(obj, prop, _) => Ok(Stmt::AssignProperty(obj, prop, rhs)),
                        Expr::IndexAccess(arr, idx, _) => {
                            if let Expr::Identifier(name, _) = *arr {
                                Ok(Stmt::AssignIndex(name, *idx, rhs))
                            } else {
                                Err("Invalid array assignment target".to_string())
                            }
                        }
                        _ => Err("Invalid assignment target".to_string()),
                    }
                } else {
                    if self.peek_token.kind == TokenKind::Semicolon {
                        self.next_token();
                    }
                    Ok(Stmt::Expr(expr))
                }
            }
        }
    }

    fn parse_let_statement(&mut self) -> Result<Stmt, String> {
        let mut is_mut = false;
        if self.peek_token.kind == TokenKind::Mut {
            self.next_token();
            is_mut = true;
        }

        let name = match &self.peek_token.kind {
            TokenKind::Identifier(id) => id.clone(),
            _ => return Err("Expected identifier after 'let'".to_string()),
        };
        self.next_token();

        let mut type_annotation = None;
        if self.peek_token.kind == TokenKind::Colon {
            self.next_token(); // Move to colon
            self.next_token(); // Move to type identifier
            type_annotation = Some(self.parse_type()?);
        }

        self.expect_peek(TokenKind::Assign)?;
        self.next_token(); // move past '='

        let expr = self.parse_expression(Precedence::Lowest)?;
        
        if self.peek_token.kind == TokenKind::Semicolon {
            self.next_token();
        }

        if is_mut {
            Ok(Stmt::LetMut(name, type_annotation, expr))
        } else {
            Ok(Stmt::Let(name, type_annotation, expr))
        }
    }

    fn parse_try_catch_statement(&mut self) -> Result<Stmt, String> {
        self.expect_peek(TokenKind::LBrace)?; 
        let try_block = self.parse_block()?;

        self.expect_peek(TokenKind::Catch)?;
        self.expect_peek(TokenKind::LParen)?;
        
        self.next_token();
        let catch_param = match &self.current_token.kind {
            TokenKind::Identifier(id) => id.clone(),
            _ => return Err("Expected identifier for catch parameter".to_string()),
        };
        
        self.expect_peek(TokenKind::RParen)?;
        self.expect_peek(TokenKind::LBrace)?;
        
        let catch_block = self.parse_block()?;
        
        Ok(Stmt::TryCatch(try_block, catch_param, catch_block))
    }

    fn parse_throw_statement(&mut self) -> Result<Stmt, String> {
        self.next_token(); 
        
        let expr = self.parse_expression(Precedence::Lowest)?;
        
        if self.peek_token.kind == TokenKind::Semicolon {
            self.next_token();
        }
        
        Ok(Stmt::Throw(expr))
    }

    fn parse_assign_statement(&mut self) -> Result<Stmt, String> {
        let name = match &self.current_token.kind {
            TokenKind::Identifier(id) => id.clone(),
            _ => unreachable!(),
        };

        self.expect_peek(TokenKind::Assign)?;
        self.next_token();

        let expr = self.parse_expression(Precedence::Lowest)?;

        if self.peek_token.kind == TokenKind::Semicolon {
            self.next_token();
        }

        Ok(Stmt::Assign(name, expr))
    }

    fn parse_assign_index_statement(&mut self) -> Result<Stmt, String> {
        let name = match &self.current_token.kind {
            TokenKind::Identifier(id) => id.clone(),
            _ => unreachable!(),
        };

        self.expect_peek(TokenKind::LBracket)?; // consume '['
        self.next_token(); // move to index expr
        
        let index = self.parse_expression(Precedence::Lowest)?;
        
        self.expect_peek(TokenKind::RBracket)?;
        self.expect_peek(TokenKind::Assign)?;
        self.next_token();

        let value = self.parse_expression(Precedence::Lowest)?;

        if self.peek_token.kind == TokenKind::Semicolon {
            self.next_token();
        }

        Ok(Stmt::AssignIndex(name, index, value))
    }

    fn parse_return_statement(&mut self) -> Result<Stmt, String> {
        if self.peek_token.kind == TokenKind::Semicolon {
            self.next_token();
            Ok(Stmt::Return(None))
        } else {
            self.next_token();
            let expr = self.parse_expression(Precedence::Lowest)?;
            if self.peek_token.kind == TokenKind::Semicolon {
                self.next_token();
            }
            Ok(Stmt::Return(Some(expr)))
        }
    }

    fn parse_if_statement(&mut self) -> Result<Stmt, String> {
        self.next_token(); // move past 'if'
        let condition = self.parse_expression(Precedence::Lowest)?;
        
        self.expect_peek(TokenKind::LBrace)?;
        let consequence = self.parse_block()?;

        let mut alternative = None;
        if self.peek_token.kind == TokenKind::Else {
            self.next_token();
            self.expect_peek(TokenKind::LBrace)?;
            alternative = Some(self.parse_block()?);
        }

        Ok(Stmt::If(condition, consequence, alternative))
    }

    fn parse_while_statement(&mut self) -> Result<Stmt, String> {
        self.next_token(); // move past 'while'
        let condition = self.parse_expression(Precedence::Lowest)?;

        self.expect_peek(TokenKind::LBrace)?;
        let body = self.parse_block()?;

        Ok(Stmt::While(condition, body))
    }

    fn parse_for_statement(&mut self) -> Result<Stmt, String> {
        self.next_token(); // move past 'for'
        
        let init = self.parse_statement()?; 
        self.next_token(); // move past the semicolon that ended the init statement
        
        let condition = self.parse_expression(Precedence::Lowest)?;
        self.expect_peek(TokenKind::Semicolon)?; 
        
        self.next_token(); // move past the semicolon to the increment statement
        let increment = self.parse_statement()?;
        
        self.expect_peek(TokenKind::LBrace)?;
        let body = self.parse_block()?;

        Ok(Stmt::For(Box::new(init), condition, Box::new(increment), body))
    }

    fn parse_expression(&mut self, precedence: Precedence) -> Result<Expr, String> {
        let mut left = self.parse_prefix()?;

        while self.peek_token.kind != TokenKind::Semicolon && precedence < self.peek_precedence() {
            self.next_token();
            left = self.parse_infix(left)?;
        }

        Ok(left)
    }

    fn parse_prefix(&mut self) -> Result<Expr, String> {
        match &self.current_token.kind {
            TokenKind::Identifier(id) => {
                let id = id.clone();
                if self.peek_token.kind == TokenKind::DoubleColon {
                    // namespace::function_name(...)
                    self.next_token(); // consume '::'
                    self.next_token(); // move to function name
                    let func_name = match &self.current_token.kind {
                        TokenKind::Identifier(fname) => fname.clone(),
                        _ => return Err("Expected function name after '::'".to_string()),
                    };
                    // Mangle immediately: namespace__func_name
                    let mangled = format!("{}__{}" , id, func_name);
                    self.parse_call_expression(mangled)
                } else if self.peek_token.kind == TokenKind::LParen {
                    self.parse_call_expression(id)
                } else {
                    Ok(Expr::Identifier(id, None))
                }
            }
            TokenKind::Number(val) => Ok(Expr::Number(*val)),
            TokenKind::Boolean(b) => Ok(Expr::Boolean(*b)),
            TokenKind::String(s) => Ok(Expr::StringLiteral(s.clone())),
            TokenKind::LParen => {
                self.next_token();
                let expr = self.parse_expression(Precedence::Lowest)?;
                self.expect_peek(TokenKind::RParen)?;
                Ok(expr)
            }
            TokenKind::LBracket => {
                self.parse_array_literal()
            }
            TokenKind::LBrace => {
                self.parse_dict_literal()
            }
            TokenKind::New => {
                self.parse_new_expression()
            }
            TokenKind::At => {
                self.parse_closure()
            }
            _ => Err(format!("No prefix parse function for {:?}", self.current_token)),
        }
    }

    fn parse_dict_literal(&mut self) -> Result<Expr, String> {
        let mut elements = Vec::new();
        if self.peek_token.kind != TokenKind::RBrace {
            self.next_token(); // Move to first key
            let key = self.parse_expression(Precedence::Lowest)?;
            
            self.expect_peek(TokenKind::Colon)?;
            self.next_token(); // Move to value
            
            let val = self.parse_expression(Precedence::Lowest)?;
            elements.push((key, val));

            while self.peek_token.kind == TokenKind::Comma {
                self.next_token(); // consume comma
                self.next_token(); // move to key
                let key = self.parse_expression(Precedence::Lowest)?;
                
                self.expect_peek(TokenKind::Colon)?;
                self.next_token(); // Move to value
                let val = self.parse_expression(Precedence::Lowest)?;
                elements.push((key, val));
            }
        }
        self.expect_peek(TokenKind::RBrace)?;
        Ok(Expr::DictLiteral(elements))
    }

    fn parse_closure(&mut self) -> Result<Expr, String> {
        self.next_token(); // Move past '@'
        
        self.expect_peek(TokenKind::LParen)?;
        
        let mut params = Vec::new();
        if self.peek_token.kind != TokenKind::RParen {
            self.next_token();
            let param_name = match &self.current_token.kind {
                TokenKind::Identifier(id) => id.clone(),
                _ => return Err("Expected parameter name in closure".to_string()),
            };
            
            let mut param_type = None;
            if self.peek_token.kind == TokenKind::Colon {
                self.next_token();
                self.next_token();
                param_type = Some(self.parse_type()?);
            }
            params.push((param_name, param_type));

            while self.peek_token.kind == TokenKind::Comma {
                self.next_token(); // consume comma
                self.next_token(); // move to id
                let param_name = match &self.current_token.kind {
                    TokenKind::Identifier(id) => id.clone(),
                    _ => return Err("Expected parameter name after comma".to_string()),
                };
                
                let mut param_type = None;
                if self.peek_token.kind == TokenKind::Colon {
                    self.next_token();
                    self.next_token();
                    param_type = Some(self.parse_type()?);
                }
                params.push((param_name, param_type));
            }
        }
        self.expect_peek(TokenKind::RParen)?;
        
        let mut return_type = None;
        if self.peek_token.kind == TokenKind::ReturnArrow {
            self.next_token();
            self.next_token();
            return_type = Some(self.parse_type()?);
        }
        
        self.expect_peek(TokenKind::LBrace)?;
        let body = self.parse_block()?;
        
        Ok(Expr::Closure { params, return_type, body })
    }

    fn parse_new_expression(&mut self) -> Result<Expr, String> {
        self.next_token(); // Move past 'new'
        let name = match &self.current_token.kind {
            TokenKind::Identifier(id) => id.clone(),
            _ => return Err("Expected type name after new".to_string()),
        };
        
        if self.peek_token.kind == TokenKind::LParen {
            self.next_token(); // move to '('
            self.next_token(); // move inside '('
            
            let mut args = Vec::new();
            let mut has_named = false;
            if self.current_token.kind != TokenKind::RParen {
                let arg = self.parse_argument()?;
                if arg.name.is_some() { has_named = true; }
                args.push(arg);

                while self.peek_token.kind == TokenKind::Comma {
                    self.next_token(); // consume comma
                    self.next_token(); // move to next expr
                    let arg = self.parse_argument()?;
                    if arg.name.is_some() { has_named = true; }
                    args.push(arg);
                }
            }
            if self.peek_token.kind == TokenKind::RParen {
                self.next_token(); // consume ')'
            } else if self.current_token.kind != TokenKind::RParen {
                return Err("Expected ')'".to_string());
            }
            
            if has_named {
                Ok(Expr::NamedNewClass(name, args))
            } else {
                let exprs = args.into_iter().map(|a| a.value).collect();
                Ok(Expr::NewClass(name, exprs))
            }
        } else {
            self.expect_peek(TokenKind::LBrace)?;
            let mut fields = Vec::new();

            if self.peek_token.kind != TokenKind::RBrace {
                self.next_token(); // Move to first key
                let key = match &self.current_token.kind {
                    TokenKind::Identifier(id) => id.clone(),
                    _ => return Err("Expected field name".to_string()),
                };
                self.expect_peek(TokenKind::Colon)?;
                self.next_token(); // Move to value
                let val = self.parse_expression(Precedence::Lowest)?;
                fields.push((key, val));

                while self.peek_token.kind == TokenKind::Comma {
                    self.next_token(); // consume comma
                    self.next_token(); // move to key
                    let key = match &self.current_token.kind {
                        TokenKind::Identifier(id) => id.clone(),
                        _ => return Err("Expected field name".to_string()),
                    };
                    self.expect_peek(TokenKind::Colon)?;
                    self.next_token();
                    let val = self.parse_expression(Precedence::Lowest)?;
                    fields.push((key, val));
                }
            }
            self.expect_peek(TokenKind::RBrace)?;

            Ok(Expr::StructInit(name, fields))
        }
    }

    fn parse_array_literal(&mut self) -> Result<Expr, String> {
        let mut elements = Vec::new();
        if self.peek_token.kind != TokenKind::RBracket {
            self.next_token();
            elements.push(self.parse_expression(Precedence::Lowest)?);

            while self.peek_token.kind == TokenKind::Comma {
                self.next_token(); // consume comma
                self.next_token();
                elements.push(self.parse_expression(Precedence::Lowest)?);
            }
        }
        self.expect_peek(TokenKind::RBracket)?;
        Ok(Expr::ArrayLiteral(elements))
    }

    fn parse_argument(&mut self) -> Result<Argument, String> {
        if let TokenKind::Identifier(id) = &self.current_token.kind {
            if self.peek_token.kind == TokenKind::Assign {
                let name = id.clone();
                self.next_token(); // move to '='
                self.next_token(); // move past '='
                let value = self.parse_expression(Precedence::Lowest)?;
                return Ok(Argument { name: Some(name), value });
            }
        }
        let value = self.parse_expression(Precedence::Lowest)?;
        Ok(Argument { name: None, value })
    }

    fn parse_call_expression(&mut self, func_name: String) -> Result<Expr, String> {
        self.expect_peek(TokenKind::LParen)?; // Should be '('
        
        let mut args = Vec::new();
        let mut has_named = false;
        if self.peek_token.kind != TokenKind::RParen {
            self.next_token();
            let arg = self.parse_argument()?;
            if arg.name.is_some() { has_named = true; }
            args.push(arg);

            while self.peek_token.kind == TokenKind::Comma {
                self.next_token(); // consume comma
                self.next_token(); // move to next expr
                let arg = self.parse_argument()?;
                if arg.name.is_some() { has_named = true; }
                args.push(arg);
            }
        }
        self.expect_peek(TokenKind::RParen)?;

        if has_named {
            Ok(Expr::NamedCall(func_name, args, None))
        } else {
            let exprs = args.into_iter().map(|a| a.value).collect();
            Ok(Expr::Call(func_name, exprs, None))
        }
    }

    fn parse_infix(&mut self, left: Expr) -> Result<Expr, String> {
        let operator = match &self.current_token.kind {
            TokenKind::Plus => BinaryOperator::Add,
            TokenKind::Minus => BinaryOperator::Subtract,
            TokenKind::Star => BinaryOperator::Multiply,
            TokenKind::Slash => BinaryOperator::Divide,
            TokenKind::Equals => BinaryOperator::Equals,
            TokenKind::NotEquals => BinaryOperator::NotEquals,
            TokenKind::LessThan => BinaryOperator::LessThan,
            TokenKind::GreaterThan => BinaryOperator::GreaterThan,
            TokenKind::LessThanOrEq => BinaryOperator::LessThanOrEq,
            TokenKind::GreaterThanOrEq => BinaryOperator::GreaterThanOrEq,
            TokenKind::And => BinaryOperator::And,
            TokenKind::Or => BinaryOperator::Or,
            TokenKind::LBracket => return self.parse_index_access(left),
            TokenKind::Dot => return self.parse_property_access(left),
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
        self.expect_peek(TokenKind::RBracket)?;
        Ok(Expr::IndexAccess(Box::new(left), Box::new(index), None))
    }

    fn parse_property_access(&mut self, left: Expr) -> Result<Expr, String> {
        self.next_token(); // current token is now the property identifier
        let prop = match &self.current_token.kind {
            TokenKind::Identifier(id) => id.clone(),
            _ => return Err("Expected property name after '.'".to_string()),
        };

        if self.peek_token.kind == TokenKind::LParen {
            self.expect_peek(TokenKind::LParen)?; // move to '('
            
            let mut args = Vec::new();
            let mut has_named = false;
            if self.peek_token.kind != TokenKind::RParen {
                self.next_token(); // move to first arg
                let arg = self.parse_argument()?;
                if arg.name.is_some() { has_named = true; }
                args.push(arg);

                while self.peek_token.kind == TokenKind::Comma {
                    self.next_token(); // consume comma
                    self.next_token(); // move to next expr
                    let arg = self.parse_argument()?;
                    if arg.name.is_some() { has_named = true; }
                    args.push(arg);
                }
            }
            self.expect_peek(TokenKind::RParen)?;
            
            if has_named {
                Ok(Expr::NamedMethodCall(Box::new(left), prop, args, None))
            } else {
                let exprs = args.into_iter().map(|a| a.value).collect();
                Ok(Expr::MethodCall(Box::new(left), prop, exprs, None))
            }
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
    match &token.kind {
        TokenKind::Or => Precedence::LogicalOr,
        TokenKind::And => Precedence::LogicalAnd,
        TokenKind::Equals | TokenKind::NotEquals => Precedence::Equals,
        TokenKind::LessThan | TokenKind::GreaterThan | TokenKind::LessThanOrEq | TokenKind::GreaterThanOrEq => Precedence::LessGreater,
        TokenKind::Plus | TokenKind::Minus => Precedence::Sum,
        TokenKind::Star | TokenKind::Slash => Precedence::Product,
        TokenKind::LParen | TokenKind::LBracket | TokenKind::Dot => Precedence::Call,
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
