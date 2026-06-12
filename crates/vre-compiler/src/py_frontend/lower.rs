use crate::vir::{Module, Function, BasicBlock, Instruction, Value, BlockId};
use rustpython_parser::ast::{Stmt, Expr, Constant, Operator, CmpOp};

pub struct Lowerer {
    pub module: Module,
    pub current_block: BlockId,
    pub blocks: Vec<BasicBlock>,
    pub next_value: Value,
}

impl Lowerer {
    pub fn new() -> Self {
        let mut lowerer = Self {
            module: Module { functions: Vec::new() },
            current_block: 0,

            blocks: Vec::new(),
            next_value: 0,
        };
        lowerer.current_block = lowerer.new_block();
        lowerer
    }

    pub fn finish(mut self) -> Module {
        self.add_inst(Instruction::Return(None));
        
        let main_func = Function {
            name: "@main".to_string(),
            params: vec![],
            blocks: self.blocks,
            entry_block: 0,
        };
        
        self.module.functions.push(main_func);
        self.module
    }

    pub fn new_block(&mut self) -> BlockId {
        let id = self.blocks.len();
        self.blocks.push(BasicBlock { id, instructions: Vec::new() });
        id
    }

    pub fn add_inst(&mut self, inst: Instruction) -> Value {
        let val = self.next_value;
        self.next_value += 1;
        self.blocks[self.current_block].instructions.push((val, inst));
        val
    }

    pub fn lower_statement(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Expr(expr_stmt) => {
                let _val = self.lower_expression(&expr_stmt.value)?;
            }
            Stmt::Assign(assign_stmt) => {
                let val = self.lower_expression(&assign_stmt.value)?;
                for target in &assign_stmt.targets {
                    match target {
                        Expr::Name(name_expr) => {
                            self.add_inst(Instruction::StoreVar(name_expr.id.to_string(), val));
                        }
                        Expr::Subscript(sub_expr) => {
                            let array_val = self.lower_expression(&sub_expr.value)?;
                            let index_val = self.lower_expression(&sub_expr.slice)?;
                            self.add_inst(Instruction::AssignIndex(array_val, index_val, val));
                        }
                        _ => return Err(format!("Unsupported assignment target: {:?}", target)),
                    }
                }
            }
            Stmt::If(if_stmt) => {
                let test_val = self.lower_expression(&if_stmt.test)?;
                let then_block = self.new_block();
                let else_block = if if_stmt.orelse.is_empty() { None } else { Some(self.new_block()) };
                let merge_block = self.new_block();

                self.add_inst(Instruction::CondBranch(
                    test_val,
                    then_block,
                    else_block.unwrap_or(merge_block),
                ));

                self.current_block = then_block;
                for s in &if_stmt.body {
                    self.lower_statement(s)?;
                }
                self.add_inst(Instruction::Branch(merge_block));

                if let Some(elb) = else_block {
                    self.current_block = elb;
                    for s in &if_stmt.orelse {
                        self.lower_statement(s)?;
                    }
                    self.add_inst(Instruction::Branch(merge_block));
                }

                self.current_block = merge_block;
            }
            Stmt::While(while_stmt) => {
                let cond_block = self.new_block();
                let body_block = self.new_block();
                let end_block = self.new_block();

                self.add_inst(Instruction::Branch(cond_block));
                self.current_block = cond_block;

                let test_val = self.lower_expression(&while_stmt.test)?;
                self.add_inst(Instruction::CondBranch(test_val, body_block, end_block));

                self.current_block = body_block;
                for s in &while_stmt.body {
                    self.lower_statement(s)?;
                }
                self.add_inst(Instruction::Branch(cond_block));

                self.current_block = end_block;
            }
            Stmt::FunctionDef(func_def) => {
                let old_blocks = std::mem::take(&mut self.blocks);
                let old_current = self.current_block;
                let old_next = self.next_value;
                
                self.current_block = self.new_block();
                self.next_value = 0;
                
                let mut params = Vec::new();
                // arguments is Box<Arguments<R>>
                // We map python positional args (func_def.args.args)
                for arg in &func_def.args.args {
                    // ArgWithDefault has an .as_arg() which returns &Arg<R>
                    // Arg<R> has .arg which is an Identifier
                    params.push(arg.as_arg().arg.to_string());
                }
                
                for s in &func_def.body {
                    self.lower_statement(s)?;
                }
                
                self.add_inst(Instruction::Return(None));
                
                let new_func = Function {
                    name: func_def.name.to_string(),
                    params,
                    blocks: std::mem::take(&mut self.blocks),
                    entry_block: 0,
                };
                self.module.functions.push(new_func);
                
                self.blocks = old_blocks;
                self.current_block = old_current;
                self.next_value = old_next;
            }
            Stmt::Return(ret_stmt) => {
                let val = if let Some(expr) = &ret_stmt.value {
                    Some(self.lower_expression(expr)?)
                } else {
                    None
                };
                self.add_inst(Instruction::Return(val));
            }
            Stmt::Import(import_stmt) => {
                for alias in &import_stmt.names {
                    let name = alias.name.to_string();
                    let asname = alias.asname.as_ref().map(|id| id.to_string()).unwrap_or(name.clone());
                    let mod_val = self.add_inst(Instruction::ImportModule(name.clone()));
                    self.add_inst(Instruction::StoreVar(asname, mod_val));
                }
            }
            Stmt::ImportFrom(import_from) => {
                let module_name = import_from.module.as_ref().map(|id| id.to_string()).unwrap_or("".to_string());
                let mod_val = self.add_inst(Instruction::ImportModule(module_name.clone()));
                for alias in &import_from.names {
                    let name = alias.name.to_string();
                    let asname = alias.asname.as_ref().map(|id| id.to_string()).unwrap_or(name.clone());
                    let prop_val = self.add_inst(Instruction::PropertyAccess(mod_val, name));
                    self.add_inst(Instruction::StoreVar(asname, prop_val));
                }
            }
            _ => return Err(format!("Unsupported python statement: {:?}", stmt)),
        }
        Ok(())
    }

    pub fn lower_expression(&mut self, expr: &Expr) -> Result<Value, String> {
        match expr {
            Expr::Constant(const_expr) => {
                match &const_expr.value {
                    Constant::Int(i) => {
                        // In malachite-bigint, we can get u64 or string.
                        let num = i.to_string().parse::<f64>().unwrap_or(0.0);
                        Ok(self.add_inst(Instruction::LoadConstNumber(num)))
                    }
                    Constant::Float(f) => {
                        Ok(self.add_inst(Instruction::LoadConstNumber(*f)))
                    }
                    Constant::Str(s) => {
                        Ok(self.add_inst(Instruction::LoadConstString(s.clone())))
                    }
                    Constant::Bool(b) => {
                        Ok(self.add_inst(Instruction::LoadConstBool(*b)))
                    }
                    _ => Err(format!("Unsupported constant: {:?}", const_expr.value))
                }
            }
            Expr::Name(name_expr) => {
                Ok(self.add_inst(Instruction::LoadVar(name_expr.id.to_string())))
            }
            Expr::Call(call_expr) => {
                if let Expr::Name(name_expr) = &*call_expr.func {
                    if name_expr.id.as_str() == "vyauma_export" {
                        if call_expr.args.len() != 2 {
                            return Err("vyauma_export requires exactly 2 arguments (name, value)".to_string());
                        }
                        let name_str = if let Expr::Constant(c) = &call_expr.args[0] {
                            if let Constant::Str(s) = &c.value {
                                s.clone()
                            } else {
                                return Err("vyauma_export first argument must be a string".to_string());
                            }
                        } else {
                            return Err("vyauma_export first argument must be a string literal".to_string());
                        };
                        
                        let val = self.lower_expression(&call_expr.args[1])?;
                        return Ok(self.add_inst(Instruction::ExportValue(name_str, val)));
                    }

                    // Idiomatic Python name → FFI mapping
                    let native_name: Option<&str> = match name_expr.id.as_str() {
                        "print"          => Some("ffi_console_println"),
                        // Database operations
                        "vre_db_insert"  => Some("ffi_db_insert"),
                        "vre_db_find"    => Some("ffi_db_find"),
                        "vre_db_delete"  => Some("ffi_db_delete"),
                        // Filesystem operations
                        "vre_fs_read"    => Some("ffi_fs_read_file"),
                        "vre_fs_write"   => Some("ffi_fs_write_file"),
                        "vre_fs_exists"  => Some("ffi_fs_exists"),
                        "vre_fs_delete"  => Some("ffi_fs_delete"),
                        // Concurrency
                        "vre_sleep"      => Some("ffi_task_sleep"),
                        "vre_spawn"      => Some("ffi_task_spawn"),
                        _ => None,
                    };
                    if let Some(ffi_name) = native_name {
                        let mut args = Vec::new();
                        for arg in &call_expr.args {
                            args.push(self.lower_expression(arg)?);
                        }
                        return Ok(self.add_inst(Instruction::Call(ffi_name.to_string(), args)));
                    }
                }
                
                // Not supported dynamic calls easily without CallDynamic. We will assume static function names for now.
                // Or maybe we can just panic for now if it's not a Name.
                if let Expr::Name(name_expr) = &*call_expr.func {
                    let mut args = Vec::new();
                    for arg in &call_expr.args {
                        args.push(self.lower_expression(arg)?);
                    }
                    Ok(self.add_inst(Instruction::Call(name_expr.id.to_string(), args)))
                } else {
                    Err("Dynamic calls not supported yet".to_string())
                }
            }
            Expr::BinOp(bin_op) => {
                let left = self.lower_expression(&bin_op.left)?;
                let right = self.lower_expression(&bin_op.right)?;
                match bin_op.op {
                    Operator::Add => Ok(self.add_inst(Instruction::Add(left, right))),
                    Operator::Sub => Ok(self.add_inst(Instruction::Sub(left, right))),
                    Operator::Mult => Ok(self.add_inst(Instruction::Mul(left, right))),
                    Operator::Div => Ok(self.add_inst(Instruction::Div(left, right))),
                    _ => Err(format!("Unsupported bin op: {:?}", bin_op.op)),
                }
            }
            Expr::Compare(cmp) => {
                if cmp.ops.len() != 1 || cmp.comparators.len() != 1 {
                    return Err("Only single comparisons are supported".to_string());
                }
                let left = self.lower_expression(&cmp.left)?;
                let right = self.lower_expression(&cmp.comparators[0])?;
                match cmp.ops[0] {
                    CmpOp::Lt => Ok(self.add_inst(Instruction::Lt(left, right))),
                    CmpOp::Gt => Ok(self.add_inst(Instruction::Gt(left, right))),
                    CmpOp::Eq => Ok(self.add_inst(Instruction::Eq(left, right))),
                    CmpOp::NotEq => Ok(self.add_inst(Instruction::NotEq(left, right))),
                    _ => Err(format!("Unsupported cmp op: {:?}", cmp.ops[0])),
                }
            }
            Expr::List(list_expr) => {
                let mut elts = Vec::new();
                for elt in &list_expr.elts {
                    elts.push(self.lower_expression(elt)?);
                }
                Ok(self.add_inst(Instruction::ArrayLiteral(elts)))
            }
            Expr::Tuple(tuple_expr) => {
                let mut elts = Vec::new();
                for elt in &tuple_expr.elts {
                    elts.push(self.lower_expression(elt)?);
                }
                Ok(self.add_inst(Instruction::ArrayLiteral(elts)))
            }
            Expr::Dict(dict_expr) => {
                let mut pairs = Vec::new();
                for (key_opt, val_expr) in dict_expr.keys.iter().zip(dict_expr.values.iter()) {
                    if let Some(key_expr) = key_opt {
                        if let Expr::Constant(const_expr) = key_expr {
                            if let Constant::Str(s) = &const_expr.value {
                                let v = self.lower_expression(val_expr)?;
                                pairs.push((s.clone(), v));
                            } else {
                                return Err("Dict keys must be strings".to_string());
                            }
                        } else {
                            return Err("Dict keys must be string constants".to_string());
                        }
                    } else {
                        return Err("**kwargs in dict literal not supported".to_string());
                    }
                }
                Ok(self.add_inst(Instruction::StructInit("".to_string(), pairs)))
            }
            Expr::Subscript(sub_expr) => {
                let array_val = self.lower_expression(&sub_expr.value)?;
                let index_val = self.lower_expression(&sub_expr.slice)?;
                Ok(self.add_inst(Instruction::IndexAccess(array_val, index_val)))
            }
            Expr::Attribute(attr_expr) => {
                let obj_val = self.lower_expression(&attr_expr.value)?;
                Ok(self.add_inst(Instruction::PropertyAccess(obj_val, attr_expr.attr.to_string())))
            }
            _ => Err(format!("Unsupported python expression: {:?}", expr)),
        }
    }
}
