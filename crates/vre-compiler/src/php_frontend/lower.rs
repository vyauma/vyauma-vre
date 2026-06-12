use php_parser_rs::parser::ast::{Program, Statement, Expression, literals::Literal, variables::Variable};
use crate::vir::{Module, Function, Instruction, Value, BasicBlock};

pub struct Lowerer {
    module: Module,
    blocks: Vec<BasicBlock>,
    current_block: usize,
    next_value: Value,
}

impl Lowerer {
    pub fn new() -> Self {
        Self {
            module: Module { functions: Vec::new() },
            blocks: Vec::new(),
            current_block: 0,
            next_value: 0,
        }
    }

    fn next_val(&mut self) -> Value {
        let val = self.next_value;
        self.next_value += 1;
        val
    }

    fn new_block(&mut self) -> usize {
        let id = self.blocks.len();
        self.blocks.push(BasicBlock { id, instructions: Vec::new() });
        id
    }

    fn add_inst(&mut self, inst: Instruction) -> Value {
        let val = self.next_val();
        self.blocks[self.current_block].instructions.push((val, inst));
        val
    }

    pub fn lower_program(mut self, program: &Program) -> Result<Module, String> {
        let mut main_func = Function {
            name: "@main".to_string(),
            params: Vec::new(),
            blocks: Vec::new(),
            entry_block: 0,
        };
        
        self.current_block = self.new_block();

        for stmt in program {
            self.lower_statement(stmt)?;
        }
        
        // Ensure implicit return
        self.add_inst(Instruction::Return(None));
        main_func.blocks = std::mem::take(&mut self.blocks);
        
        self.module.functions.push(main_func);

        Ok(self.module)
    }

    fn lower_statement(&mut self, stmt: &Statement) -> Result<(), String> {
        match stmt {
            Statement::Expression(expr_stmt) => {
                let _val = self.lower_expression(&expr_stmt.expression)?;
            }
            Statement::Return(ret_stmt) => {
                let val = if let Some(expr) = &ret_stmt.value {
                    Some(self.lower_expression(expr)?)
                } else {
                    None
                };
                self.add_inst(Instruction::Return(val));
            }
            Statement::Function(func) => {
                let old_blocks = std::mem::take(&mut self.blocks);
                let old_current = self.current_block;
                let old_next = self.next_value;
                
                self.current_block = self.new_block();
                self.next_value = 0;
                
                let mut params = Vec::new();
                for param in func.parameters.iter() {
                    let param_name = String::from_utf8_lossy(&param.name.name).to_string();
                    params.push(param_name);
                }
                
                for stmt in &func.body.statements {
                    self.lower_statement(stmt)?;
                }
                
                self.add_inst(Instruction::Return(None));
                
                let func_name = String::from_utf8_lossy(&func.name.value).to_string();
                let new_func = Function {
                    name: func_name,
                    params,
                    blocks: std::mem::take(&mut self.blocks),
                    entry_block: 0,
                };
                self.module.functions.push(new_func);
                
                self.blocks = old_blocks;
                self.current_block = old_current;
                self.next_value = old_next;
            }
            Statement::While(while_stmt) => {
                let cond_block = self.new_block();
                let body_block = self.new_block();
                let end_block = self.new_block();

                self.add_inst(Instruction::Branch(cond_block));

                self.current_block = cond_block;
                let cond_val = self.lower_expression(&while_stmt.condition)?;
                self.add_inst(Instruction::CondBranch(cond_val, body_block, end_block));

                self.current_block = body_block;
                match &while_stmt.body {
                    php_parser_rs::parser::ast::loops::WhileStatementBody::Statement { statement } => {
                        self.lower_statement(statement)?;
                    }
                    php_parser_rs::parser::ast::loops::WhileStatementBody::Block { statements, .. } => {
                        for stmt in statements {
                            self.lower_statement(stmt)?;
                        }
                    }
                }
                self.add_inst(Instruction::Branch(cond_block));

                self.current_block = end_block;
            }
            Statement::If(if_stmt) => {
                let then_block = self.new_block();
                let end_block = self.new_block();
                let mut else_block = end_block;

                let has_else = match &if_stmt.body {
                    php_parser_rs::parser::ast::control_flow::IfStatementBody::Statement { r#else, .. } => r#else.is_some(),
                    php_parser_rs::parser::ast::control_flow::IfStatementBody::Block { r#else, .. } => r#else.is_some(),
                };

                if has_else {
                    else_block = self.new_block();
                }

                let cond_val = self.lower_expression(&if_stmt.condition)?;
                self.add_inst(Instruction::CondBranch(cond_val, then_block, else_block));

                self.current_block = then_block;
                match &if_stmt.body {
                    php_parser_rs::parser::ast::control_flow::IfStatementBody::Statement { statement, r#else, .. } => {
                        self.lower_statement(statement)?;
                        self.add_inst(Instruction::Branch(end_block));
                        if let Some(else_stmt) = r#else {
                            self.current_block = else_block;
                            self.lower_statement(&else_stmt.statement)?;
                            self.add_inst(Instruction::Branch(end_block));
                        }
                    }
                    php_parser_rs::parser::ast::control_flow::IfStatementBody::Block { statements, r#else, .. } => {
                        for stmt in statements {
                            self.lower_statement(stmt)?;
                        }
                        self.add_inst(Instruction::Branch(end_block));
                        if let Some(else_stmt) = r#else {
                            self.current_block = else_block;
                            for stmt in &else_stmt.statements {
                                self.lower_statement(stmt)?;
                            }
                            self.add_inst(Instruction::Branch(end_block));
                        }
                    }
                }
                
                self.current_block = end_block;
            }
            Statement::Block(php_parser_rs::parser::ast::BlockStatement { statements, .. }) => {
                for stmt in statements {
                    self.lower_statement(stmt)?;
                }
            }
            Statement::Echo(echo_stmt) => {
                for expr in &echo_stmt.values {
                    let val = self.lower_expression(expr)?;
                    self.add_inst(Instruction::Call("ffi_console_print".to_string(), vec![val]));
                }
            }
            Statement::Noop(_) | Statement::InlineHtml(_) | Statement::ClosingTag(_) | Statement::FullOpeningTag(_) => {}
            _ => return Err(format!("Unsupported PHP statement: {:?}", stmt)),
        }
        Ok(())
    }

    fn lower_expression(&mut self, expr: &Expression) -> Result<Value, String> {
        match expr {
            Expression::Literal(Literal::String(s)) => {
                let val_str = String::from_utf8_lossy(&s.value).to_string();
                let inst = Instruction::LoadConstString(val_str);
                Ok(self.add_inst(inst))
            }
            Expression::Literal(Literal::Integer(i)) => {
                let val_str = String::from_utf8_lossy(&i.value);
                let val: f64 = val_str.parse().unwrap_or(0.0);
                let inst = Instruction::LoadConstNumber(val);
                Ok(self.add_inst(inst))
            }
            Expression::Literal(Literal::Float(f)) => {
                let val_str = String::from_utf8_lossy(&f.value);
                let val: f64 = val_str.parse().unwrap_or(0.0);
                let inst = Instruction::LoadConstNumber(val);
                Ok(self.add_inst(inst))
            }
            Expression::Variable(Variable::SimpleVariable(var)) => {
                let var_name = String::from_utf8_lossy(&var.name).to_string();
                let inst = Instruction::LoadVar(var_name);
                Ok(self.add_inst(inst))
            }
            Expression::AssignmentOperation(php_parser_rs::parser::ast::operators::AssignmentOperationExpression::Assign { left, right, .. }) => {
                let right_val = self.lower_expression(right)?;
                
                if let Expression::Variable(Variable::SimpleVariable(var)) = left.as_ref() {
                    let var_name = String::from_utf8_lossy(&var.name).to_string();
                    self.add_inst(Instruction::StoreVar(var_name, right_val));
                    // PHP assignments evaluate to the assigned value
                    Ok(right_val)
                } else {
                    Err(format!("Unsupported assignment target: {:?}", left))
                }
            }
            Expression::FunctionCall(call_expr) => {
                if let Expression::Identifier(php_parser_rs::parser::ast::identifiers::Identifier::SimpleIdentifier(ident)) = call_expr.target.as_ref() {
                    let raw_name = String::from_utf8_lossy(&ident.value).to_string();
                    if raw_name == "vyauma_export" {
                        if call_expr.arguments.arguments.len() != 2 {
                            return Err("vyauma_export requires exactly 2 arguments (name, value)".to_string());
                        }
                        let name_arg = match &call_expr.arguments.arguments[0] {
                            php_parser_rs::parser::ast::arguments::Argument::Positional(p) => &p.value,
                            _ => return Err("Named arguments not supported in vyauma_export".to_string()),
                        };
                        let val_arg = match &call_expr.arguments.arguments[1] {
                            php_parser_rs::parser::ast::arguments::Argument::Positional(p) => &p.value,
                            _ => return Err("Named arguments not supported in vyauma_export".to_string()),
                        };
                        
                        let name_str = if let Expression::Literal(Literal::String(s)) = name_arg {
                            String::from_utf8_lossy(&s.value).to_string()
                        } else {
                            return Err("vyauma_export first argument must be a string literal".to_string());
                        };
                        
                        let val = self.lower_expression(val_arg)?;
                        let inst = Instruction::ExportValue(name_str, val);
                        return Ok(self.add_inst(inst));
                    }

                    // Idiomatic PHP name → FFI mapping
                    let func_name = match raw_name.as_str() {
                        // Database operations
                        "vre_db_insert"  => "ffi_db_insert".to_string(),
                        "vre_db_find"    => "ffi_db_find".to_string(),
                        "vre_db_delete"  => "ffi_db_delete".to_string(),
                        // Filesystem operations
                        "vre_fs_read"    => "ffi_fs_read_file".to_string(),
                        "vre_fs_write"   => "ffi_fs_write_file".to_string(),
                        "vre_fs_exists"  => "ffi_fs_exists".to_string(),
                        "vre_fs_delete"  => "ffi_fs_delete".to_string(),
                        // Concurrency
                        "vre_sleep"      => "ffi_task_sleep".to_string(),
                        "vre_spawn"      => "ffi_task_spawn".to_string(),
                        // Pass all other names through as-is (including direct ffi_* calls)
                        _                => raw_name,
                    };

                    let mut args = Vec::new();
                    for arg in call_expr.arguments.iter() {
                        match arg {
                            php_parser_rs::parser::ast::arguments::Argument::Positional(pos_arg) => {
                                args.push(self.lower_expression(&pos_arg.value)?);
                            }
                            _ => return Err("Named arguments not yet supported".to_string()),
                        }
                    }
                    Ok(self.add_inst(Instruction::Call(func_name, args)))
                } else {
                    Err("Dynamic function calls not yet supported".to_string())
                }
            }
            Expression::ArithmeticOperation(php_parser_rs::parser::ast::operators::ArithmeticOperationExpression::Addition { left, right, .. }) => {
                let left_val = self.lower_expression(left)?;
                let right_val = self.lower_expression(right)?;
                Ok(self.add_inst(Instruction::Add(left_val, right_val)))
            }
            Expression::ComparisonOperation(php_parser_rs::parser::ast::operators::ComparisonOperationExpression::LessThan { left, right, .. }) => {
                let left_val = self.lower_expression(left)?;
                let right_val = self.lower_expression(right)?;
                Ok(self.add_inst(Instruction::Lt(left_val, right_val)))
            }
            Expression::ShortArray(php_parser_rs::parser::ast::ShortArrayExpression { items, .. }) => {
                let mut vals = Vec::new();
                for item in items.inner.iter() {
                    match item {
                        php_parser_rs::parser::ast::ArrayItem::Value { value } => {
                            vals.push(self.lower_expression(value)?);
                        }
                        _ => return Err("Unsupported array item type".to_string()),
                    }
                }
                Ok(self.add_inst(Instruction::ArrayLiteral(vals)))
            }
            Expression::Array(php_parser_rs::parser::ast::ArrayExpression { items, .. }) => {
                let mut vals = Vec::new();
                for item in items.inner.iter() {
                    match item {
                        php_parser_rs::parser::ast::ArrayItem::Value { value } => {
                            vals.push(self.lower_expression(value)?);
                        }
                        _ => return Err("Unsupported array item type".to_string()),
                    }
                }
                Ok(self.add_inst(Instruction::ArrayLiteral(vals)))
            }
            Expression::Require(req_expr) => {
                if let Expression::Literal(Literal::String(s)) = req_expr.path.as_ref() {
                    let source = String::from_utf8_lossy(&s.value).to_string();
                    let mod_val = self.add_inst(Instruction::ImportModule(source));
                    Ok(mod_val)
                } else {
                    Err("Dynamic require is not supported".to_string())
                }
            }
            Expression::RequireOnce(req_expr) => {
                if let Expression::Literal(Literal::String(s)) = req_expr.path.as_ref() {
                    let source = String::from_utf8_lossy(&s.value).to_string();
                    let mod_val = self.add_inst(Instruction::ImportModule(source));
                    Ok(mod_val)
                } else {
                    Err("Dynamic require_once is not supported".to_string())
                }
            }
            Expression::Include(req_expr) => {
                if let Expression::Literal(Literal::String(s)) = req_expr.path.as_ref() {
                    let source = String::from_utf8_lossy(&s.value).to_string();
                    let mod_val = self.add_inst(Instruction::ImportModule(source));
                    Ok(mod_val)
                } else {
                    Err("Dynamic include is not supported".to_string())
                }
            }
            Expression::IncludeOnce(req_expr) => {
                if let Expression::Literal(Literal::String(s)) = req_expr.path.as_ref() {
                    let source = String::from_utf8_lossy(&s.value).to_string();
                    let mod_val = self.add_inst(Instruction::ImportModule(source));
                    Ok(mod_val)
                } else {
                    Err("Dynamic include_once is not supported".to_string())
                }
            }
            _ => Err(format!("Unsupported PHP expression: {:?}", expr)),
        }
    }
}
