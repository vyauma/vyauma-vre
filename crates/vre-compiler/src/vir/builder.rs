use super::{BlockId, Instruction, Value, Function, BasicBlock, Module};
use crate::ast::{self, Stmt, Expr};
use std::collections::HashMap;

pub struct VirBuilder {
    blocks: Vec<BasicBlock>,
    current_block: BlockId,
    next_value: Value,
    
    // For break/continue
    loop_targets: Vec<(BlockId, BlockId)>, // (continue_target, break_target)
}

impl VirBuilder {
    pub fn new() -> Self {
        let initial_block = BasicBlock {
            id: 0,
            instructions: Vec::new(),
        };
        Self {
            blocks: vec![initial_block],
            current_block: 0,
            next_value: 0,
            loop_targets: Vec::new(),
        }
    }
    
    pub fn build_module(program: &ast::Program) -> Module {
        let mut functions = Vec::new();
        for func in &program.functions {
            let mut builder = VirBuilder::new();
            let vir_func = builder.build_function(func);
            functions.push(vir_func);
        }
        Module { functions }
    }
    
    pub fn build_function(&mut self, func: &ast::Function) -> Function {
        let params: Vec<String> = func.params.iter().map(|param| param.0.clone()).collect();
        
        for stmt in &func.body {
            self.build_stmt(stmt);
        }
        
        // Ensure the last block ends with a return if it hasn't already
        if let Some(last_block) = self.blocks.last() {
            let has_ret = last_block.instructions.last().map_or(false, |(_, inst)| {
                matches!(inst, Instruction::Return(_))
            });
            if !has_ret {
                let r = self.add_inst(Instruction::Return(None));
            }
        }
        
        Function {
            name: func.name.clone(),
            params,
            blocks: self.blocks.clone(),
            entry_block: 0,
        }
    }
    
    fn next_val(&mut self) -> Value {
        let val = self.next_value;
        self.next_value += 1;
        val
    }
    
    fn new_block(&mut self) -> BlockId {
        let id = self.blocks.len();
        self.blocks.push(BasicBlock {
            id,
            instructions: Vec::new(),
        });
        id
    }
    
    fn switch_to_block(&mut self, id: BlockId) {
        self.current_block = id;
    }
    
    fn add_inst(&mut self, inst: Instruction) -> Value {
        let val = self.next_val();
        self.blocks[self.current_block].instructions.push((val, inst));
        val
    }
    
    fn build_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let(name, _type_hint, expr) | Stmt::LetMut(name, _type_hint, expr) => {
                let val = self.build_expr(expr);
                self.add_inst(Instruction::StoreVar(name.clone(), val));
            }
            Stmt::Assign(name, expr) => {
                let val = self.build_expr(expr);
                self.add_inst(Instruction::StoreVar(name.clone(), val));
            }
            Stmt::AssignIndex(name, idx_expr, val_expr) => {
                let arr = self.add_inst(Instruction::LoadVar(name.clone()));
                let idx = self.build_expr(idx_expr);
                let val = self.build_expr(val_expr);
                self.add_inst(Instruction::AssignIndex(arr, idx, val));
            }
            Stmt::AssignProperty(obj_expr, prop, val_expr) => {
                let obj = self.build_expr(obj_expr);
                let val = self.build_expr(val_expr);
                self.add_inst(Instruction::AssignProperty(obj, prop.clone(), val));
            }
            Stmt::Expr(expr) => {
                self.build_expr(expr);
            }
            Stmt::Return(opt_expr) => {
                let val = opt_expr.as_ref().map(|e| self.build_expr(e));
                self.add_inst(Instruction::Return(val));
            }
            Stmt::Throw(expr) => {
                let val = self.build_expr(expr);
                self.add_inst(Instruction::Throw(val));
            }
            Stmt::If(cond, cons, alt) => {
                let cond_val = self.build_expr(cond);
                
                let cons_block = self.new_block();
                let alt_block = self.new_block();
                let merge_block = self.new_block();
                
                if alt.is_some() {
                    self.add_inst(Instruction::CondBranch(cond_val, cons_block, alt_block));
                } else {
                    self.add_inst(Instruction::CondBranch(cond_val, cons_block, merge_block));
                }
                
                self.switch_to_block(cons_block);
                for s in cons { self.build_stmt(s); }
                self.add_inst(Instruction::Branch(merge_block));
                
                if let Some(alt_stmts) = alt {
                    self.switch_to_block(alt_block);
                    for s in alt_stmts { self.build_stmt(s); }
                    self.add_inst(Instruction::Branch(merge_block));
                }
                
                self.switch_to_block(merge_block);
            }
            Stmt::While(cond, body) => {
                let cond_block = self.new_block();
                let body_block = self.new_block();
                let merge_block = self.new_block();
                
                self.add_inst(Instruction::Branch(cond_block));
                
                self.switch_to_block(cond_block);
                let cond_val = self.build_expr(cond);
                self.add_inst(Instruction::CondBranch(cond_val, body_block, merge_block));
                
                self.switch_to_block(body_block);
                self.loop_targets.push((cond_block, merge_block));
                for s in body { self.build_stmt(s); }
                self.loop_targets.pop();
                self.add_inst(Instruction::Branch(cond_block));
                
                self.switch_to_block(merge_block);
            }
            Stmt::For(init, cond, inc, body) => {
                self.build_stmt(init);
                
                let cond_block = self.new_block();
                let body_block = self.new_block();
                let inc_block = self.new_block();
                let merge_block = self.new_block();
                
                self.add_inst(Instruction::Branch(cond_block));
                
                self.switch_to_block(cond_block);
                let cond_val = self.build_expr(cond);
                self.add_inst(Instruction::CondBranch(cond_val, body_block, merge_block));
                
                self.switch_to_block(body_block);
                self.loop_targets.push((inc_block, merge_block));
                for s in body { self.build_stmt(s); }
                self.loop_targets.pop();
                self.add_inst(Instruction::Branch(inc_block));
                
                self.switch_to_block(inc_block);
                self.build_stmt(inc);
                self.add_inst(Instruction::Branch(cond_block));
                
                self.switch_to_block(merge_block);
            }
            Stmt::TryCatch(try_block, err_name, catch_block) => {
                let catch_target = self.new_block();
                let merge_block = self.new_block();
                
                self.add_inst(Instruction::SetupTry(catch_target));
                for s in try_block { self.build_stmt(s); }
                self.add_inst(Instruction::PopTry);
                self.add_inst(Instruction::Branch(merge_block));
                
                self.switch_to_block(catch_target);
                // In catch block, error value is top of stack in actual bytecode,
                // but in VIR we can simulate it with a specific instruction if we want.
                // For simplicity, we just store var here.
                // TODO: how to get the caught error value in VIR?
                // For now, let's add an instruction `GetException`
                // Let's assume we can emit something like LoadVar("__exception__") 
                // Wait, let's update vir/mod.rs later if needed.
                for s in catch_block { self.build_stmt(s); }
                self.add_inst(Instruction::Branch(merge_block));
                
                self.switch_to_block(merge_block);
            }
            Stmt::StructDecl(..) | Stmt::ClassDecl(..) => {} // Declarations are handled globally
            Stmt::Yield => {
                // Emit a Yield instruction in VIR — suspends the coroutine task
                self.add_inst(Instruction::Yield);
            }
        }
    }
    
    fn build_expr(&mut self, expr: &Expr) -> Value {
        match expr {
            Expr::Number(n) => self.add_inst(Instruction::LoadConstNumber(*n)),
            Expr::Boolean(b) => self.add_inst(Instruction::LoadConstBool(*b)),
            Expr::StringLiteral(s) => self.add_inst(Instruction::LoadConstString(s.clone())),
            Expr::Identifier(name, ty) => {
                if let Some(crate::ast::Type::Function(_, _)) = ty {
                    self.add_inst(Instruction::NewClosure(name.clone(), vec![]))
                } else {
                    self.add_inst(Instruction::LoadVar(name.clone()))
                }
            }
            Expr::BinaryOp(left, op, right, opt_ty) => {
                let l = self.build_expr(left);
                let r = self.build_expr(right);
                
                let left_ty = match &**left {
                    Expr::StringLiteral(_) => crate::ast::Type::String,
                    Expr::Boolean(_) => crate::ast::Type::Bool,
                    Expr::Identifier(_, t) | Expr::BinaryOp(_, _, _, t) | Expr::Call(_, _, t) | Expr::MethodCall(_, _, _, t) | Expr::PropertyAccess(_, _, t) | Expr::NamedCall(_, _, t) | Expr::NamedMethodCall(_, _, _, t) | Expr::IndexAccess(_, _, t) => t.clone().unwrap_or(crate::ast::Type::Float64),
                    _ => crate::ast::Type::Float64,
                };
                let math_ty = opt_ty.clone().unwrap_or(crate::ast::Type::Float64);
                let cmp_ty = if left_ty == crate::ast::Type::Any { crate::ast::Type::Float64 } else { left_ty };
                
                match op {
                    ast::BinaryOperator::Add => {
                        if math_ty == crate::ast::Type::String {
                            self.add_inst(Instruction::AddStr(l, r))
                        } else {
                            self.add_inst(Instruction::Add(l, r))
                        }
                    },
                    ast::BinaryOperator::Subtract => self.add_inst(Instruction::Sub(l, r)),
                    ast::BinaryOperator::Multiply => self.add_inst(Instruction::Mul(l, r)),
                    ast::BinaryOperator::Divide => self.add_inst(Instruction::Div(l, r)),

                    ast::BinaryOperator::Equals => {
                        if cmp_ty == crate::ast::Type::String {
                            self.add_inst(Instruction::EqStr(l, r))
                        } else {
                            self.add_inst(Instruction::Eq(l, r))
                        }
                    },
                    ast::BinaryOperator::NotEquals => {
                        if cmp_ty == crate::ast::Type::String {
                            self.add_inst(Instruction::NotEqStr(l, r))
                        } else {
                            self.add_inst(Instruction::NotEq(l, r))
                        }
                    },
                    ast::BinaryOperator::LessThan => self.add_inst(Instruction::Lt(l, r)),
                    ast::BinaryOperator::LessThanOrEq => self.add_inst(Instruction::Lte(l, r)),
                    ast::BinaryOperator::GreaterThan => self.add_inst(Instruction::Gt(l, r)),
                    ast::BinaryOperator::GreaterThanOrEq => self.add_inst(Instruction::Gte(l, r)),
                    ast::BinaryOperator::And => self.add_inst(Instruction::And(l, r)),
                    ast::BinaryOperator::Or => self.add_inst(Instruction::Or(l, r)),
                }
            }
            Expr::Call(name, args, _) => {
                let arg_vals: Vec<Value> = args.iter().map(|a| self.build_expr(a)).collect();
                let syscall_code = match name.as_str() {
                    "print" => Some(0x01),
                    "read_char" => Some(0x02),
                    "read" => Some(0x03),
                    "write" => Some(0x04),
                    "close" => Some(0x05),
                    "file_open" => Some(0x10),
                    "sleep" => Some(0x06),
                    "sleep_async" => Some(0x08),
                    "gc" => Some(0x07),
                    "net_connect" => Some(0x20),
                    "net_listen" => Some(0x21),
                    "net_accept" => Some(0x22),
                    "net_set_nonblocking" => Some(0x23),
                    "net_poll" => Some(0x24),
                    "net_read" => Some(0x25),
                    "net_write" => Some(0x26),
                    "net_close" => Some(0x27),
                    "string_to_bytes" => Some(0x30),
                    "bytes_to_string" => Some(0x31),
                    _ => None,
                };
                
                if let Some(code) = syscall_code {
                    self.add_inst(Instruction::Syscall(code, arg_vals))
                } else {
                    self.add_inst(Instruction::Call(name.clone(), arg_vals))
                }
            }
            Expr::CallDynamic(callee, args, _) => {
                let callee_val = self.build_expr(callee);
                let arg_vals: Vec<Value> = args.iter().map(|a| self.build_expr(a)).collect();
                self.add_inst(Instruction::CallDynamic(callee_val, arg_vals))
            }
            Expr::MethodCall(obj, name, args, _) => {
                let obj_val = self.build_expr(obj);
                let arg_vals: Vec<Value> = args.iter().map(|a| self.build_expr(a)).collect();
                self.add_inst(Instruction::MethodCall(obj_val, name.clone(), arg_vals))
            }
            Expr::ArrayLiteral(elems) => {
                let elem_vals: Vec<Value> = elems.iter().map(|e| self.build_expr(e)).collect();
                self.add_inst(Instruction::ArrayLiteral(elem_vals))
            }
            Expr::DictLiteral(pairs) => {
                let pair_vals: Vec<(Value, Value)> = pairs.iter().map(|(k, v)| (self.build_expr(k), self.build_expr(v))).collect();
                self.add_inst(Instruction::DictLiteral(pair_vals))
            }
            Expr::StructInit(name, fields) => {
                let field_vals: Vec<(String, Value)> = fields.iter().map(|(k, v)| (k.clone(), self.build_expr(v))).collect();
                self.add_inst(Instruction::StructInit(name.clone(), field_vals))
            }
            Expr::NewClass(name, args) => {
                let arg_vals: Vec<Value> = args.iter().map(|a| self.build_expr(a)).collect();
                self.add_inst(Instruction::NewClass(name.clone(), arg_vals))
            }
            Expr::IndexAccess(arr, idx, _) => {
                let arr_val = self.build_expr(arr);
                let idx_val = self.build_expr(idx);
                self.add_inst(Instruction::IndexAccess(arr_val, idx_val))
            }
            Expr::PropertyAccess(obj, prop, _) => {
                let obj_val = self.build_expr(obj);
                self.add_inst(Instruction::PropertyAccess(obj_val, prop.clone()))
            }
            Expr::NamedCall(name, args, _) => {
                let arg_vals: Vec<Value> = args.iter().map(|a| self.build_expr(&a.value)).collect();
                self.add_inst(Instruction::Call(name.clone(), arg_vals))
            }
            Expr::NamedMethodCall(obj, name, args, _) => {
                let obj_val = self.build_expr(obj);
                let arg_vals: Vec<Value> = args.iter().map(|a| self.build_expr(&a.value)).collect();
                self.add_inst(Instruction::MethodCall(obj_val, name.clone(), arg_vals))
            }
            Expr::NamedNewClass(name, args) => {
                let arg_vals: Vec<Value> = args.iter().map(|a| self.build_expr(&a.value)).collect();
                self.add_inst(Instruction::NewClass(name.clone(), arg_vals))
            }
            Expr::Closure { .. } => {
                // TODO: Implement lambda lifting.
                // A closure requires taking the block, generating a separate VirBuilder::build_function pass, 
                // injecting it into the Module, and then emitting a NewClosure(func_name, captured_vars) instruction.
                // For now, we will emit a placeholder.
                let captured_vars = vec![];
                self.add_inst(Instruction::NewClosure("anon_closure".to_string(), captured_vars))
            }
        }
    }
}
