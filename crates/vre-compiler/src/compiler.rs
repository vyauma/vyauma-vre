use std::collections::HashMap;
use vre_core::bytecode::opcode::OpCode;
use vre_core::vm::value::Value;

use crate::ast::{Type, Program, Stmt, Expr, BinaryOperator, Block, Function};

#[derive(Debug)]
pub struct CompiledProgram {
    pub instructions: Vec<u8>,
    pub constants: Vec<Value>,
    pub native_imports: Vec<String>,
    /// Maps user-defined function names to their bytecode entry addresses
    pub function_table: HashMap<String, u32>,
}

pub struct Compiler {
    instructions: Vec<u8>,
    constants: Vec<Value>,
    
    // Function name -> start address
    functions: HashMap<String, u32>,
    
    // (patch_offset, function_name, arg_count)
    unresolved_calls: Vec<(usize, String, u8)>,

    // Currently compiled function context
    locals: HashMap<String, u16>,
    local_count: u16,

    // Function parameter names for named arguments reordering
    function_signatures: HashMap<String, Vec<String>>,

    // For single-level closure upvalue resolution
    outer_locals: Option<HashMap<String, u16>>,
    current_upvalues: Vec<(String, u16)>, // (name, outer_local_idx)
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            instructions: Vec::new(),
            constants: Vec::new(),
            functions: HashMap::new(),
            unresolved_calls: Vec::new(),
            locals: HashMap::new(),
            local_count: 0,
            function_signatures: HashMap::new(),
            outer_locals: None,
            current_upvalues: Vec::new(),
        }
    }

    pub fn compile(mut self, program: Program) -> Result<CompiledProgram, String> {
        // Pre-pass to populate function signatures for named arguments
        for func in &program.functions {
            let param_names = func.params.iter().map(|p| p.0.clone()).collect();
            self.function_signatures.insert(func.name.clone(), param_names);
        }

        // Emit jump to main at the start
        // We will emit Call <addr> <0>; Halt.
        // Call opcode: 1 byte + 4 bytes target + 2 bytes local_count = 7 bytes.
        self.emit_opcode(OpCode::Call);
        let main_call_target_offset = self.instructions.len();
        self.emit_u32(0); // placeholder for main address
        self.emit_u16(256); // give main 256 locals as a hack
        self.emit_opcode(OpCode::Halt);

        for func in program.functions {
            self.compile_function(func)?;
        }

        // Patch main call
        if let Some(&main_addr) = self.functions.get("main") {
            self.patch_u32(main_call_target_offset, main_addr);
        } else {
            return Err("No main function found".to_string());
        }

        let mut native_imports = Vec::new();

        // Patch other calls
        for (offset, name, arg_count) in self.unresolved_calls.clone() {
            if let Some(&addr) = self.functions.get(&name) {
                self.patch_u32(offset, addr);
            } else if name == "print" {
                // Builtin handled specially during compile_expression, but if they fell through here it's an error
                return Err("Cannot patch builtin".to_string());
            } else {
                // It's a Native FFI import!
                let native_idx = match native_imports.iter().position(|x| x == &name) {
                    Some(idx) => idx as u16,
                    None => {
                        let idx = native_imports.len() as u16;
                        native_imports.push(name.clone());
                        idx
                    }
                };

                // Rewrite OpCode::Call to OpCode::CallNative (offset - 1 is the opcode byte)
                self.instructions[offset - 1] = OpCode::CallNative as u8;
                
                // Write native_idx (2 bytes) (Big-endian)
                self.instructions[offset] = ((native_idx >> 8) & 0xFF) as u8;
                self.instructions[offset + 1] = (native_idx & 0xFF) as u8;
                
                // Write arg_count (1 byte)
                self.instructions[offset + 2] = arg_count;
                
                // The remaining 3 bytes of the original 6-byte Call operand space are left as 0 padding
            }
        }

        Ok(CompiledProgram {
            instructions: self.instructions,
            constants: self.constants,
            native_imports,
            function_table: self.functions.clone(),
        })
    }

    fn compile_function(&mut self, func: Function) -> Result<(), String> {
        let start_addr = self.instructions.len() as u32;
        self.functions.insert(func.name.clone(), start_addr);

        self.locals.clear();
        self.local_count = 0;

        // Register parameters as locals
        for param in &func.params {
            self.locals.insert(param.0.clone(), self.local_count);
            self.local_count += 1;
        }

        // Calculate total locals used in this function (params + let decls)
        // For simplicity, we just dynamically allocate locals as we see them.
        // Wait, the Call instruction needs the `local_count` before we start executing!
        // But the Call instruction is emitted by the *caller*, not the *callee*.
        // This is a design issue: how does the caller know how many locals the callee needs?
        // In our VM, Call takes `local_count` as an operand. So the caller needs to know it.
        // This means we need a 2-pass compilation:
        // Pass 1: find all functions and their local counts.
        // Pass 2: compile bodies.
        
        // Actually, we can just over-allocate or patch the local count later.
        // But since we are compiling functions sequentially, a caller might call a function defined later.
        // Let's change our `unresolved_calls` to also store the offset for the local_count, and patch it!
        // Or, simpler: just use a fixed local count for all functions for now, like 256. 
        // No, let's do a quick pass to count locals.
        let _total_locals = func.params.len() as u16 + count_locals(&func.body);
        // The caller must emit the local_count. To patch it later, we must store the total_locals in a map.
        // We will maintain `function_locals: HashMap<String, u16>`.

        // Pop arguments into locals
        // The last argument pushed is at the top of the stack, so it corresponds to the last parameter.
        for param in func.params.iter().rev() {
            let idx = *self.locals.get(&param.0).unwrap();
            self.emit_opcode(OpCode::StoreLocal);
            self.emit_u16(idx);
        }

        self.compile_block(func.body)?;

        // Ensure functions always return a value
        let idx = self.add_constant(Value::Float64(0.0));
        self.emit_opcode(OpCode::Push);
        self.emit_u16(idx);
        self.emit_opcode(OpCode::Return);

        Ok(())
    }

    fn compile_block(&mut self, block: Block) -> Result<(), String> {
        for stmt in block {
            self.compile_statement(stmt)?;
        }
        Ok(())
    }

    fn compile_statement(&mut self, stmt: Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Let(name, _, expr) => {
                self.compile_expression(expr)?;
                let local_idx = self.local_count;
                self.locals.insert(name.clone(), local_idx);
                self.local_count += 1;
                self.emit_opcode(OpCode::StoreLocal);
                self.emit_u16(local_idx as u16);
            }
            Stmt::LetMut(name, _, expr) => {
                self.compile_expression(expr)?;
                let local_idx = self.local_count;
                self.locals.insert(name.clone(), local_idx);
                self.local_count += 1;
                self.emit_opcode(OpCode::StoreLocal);
                self.emit_u16(local_idx as u16);
            }
            Stmt::Assign(name, expr) => {
                self.compile_expression(expr)?;
                if let Some(&idx) = self.locals.get(&name) {
                    self.emit_opcode(OpCode::StoreLocal);
                    self.emit_u16(idx);
                } else if let Some(upvalue_idx) = self.resolve_upvalue(&name) {
                    self.emit_opcode(OpCode::StoreUpvalue);
                    self.emit_u16(upvalue_idx);
                } else {
                    return Err(format!("Undefined variable: {}", name));
                }
            }
            Stmt::AssignIndex(name, index_expr, value_expr) => {
                if let Some(&idx) = self.locals.get(&name) {
                    // Push Ref
                    self.emit_opcode(OpCode::LoadLocal);
                    self.emit_u16(idx);
                    
                    // Push index
                    self.compile_expression(index_expr)?;
                    
                    // Push value
                    self.compile_expression(value_expr)?;
                    
                    // StoreElement pops value, index, Ref
                    self.emit_opcode(OpCode::StoreElement);
                } else {
                    return Err(format!("Undefined variable: {}", name));
                }
            }
            Stmt::AssignProperty(obj_expr, prop_name, value_expr) => {
                self.compile_expression(*obj_expr)?;
                self.compile_expression(value_expr)?;
                let prop_idx = self.add_constant(Value::String(prop_name));
                self.emit_opcode(OpCode::StoreProperty);
                self.emit_u16(prop_idx);
            }
            Stmt::StructDecl(..) | Stmt::ClassDecl(..) => {
                // Duck-typed, no runtime representation needed
            }
            Stmt::Expr(expr) => {
                match expr {
                    Expr::Number(_) | Expr::StringLiteral(_) => {
                        // Optimization: Skip emitting Push and Pop for pure literals
                    }
                    _ => {
                        self.compile_expression(expr)?;
                        self.emit_opcode(OpCode::Pop); // discard result
                    }
                }
            }
            Stmt::Return(opt_expr) => {
                if let Some(expr) = opt_expr {
                    self.compile_expression(expr)?;
                } else {
                    // return 0 if no expr
                    let idx = self.add_constant(Value::Float64(0.0));
                    self.emit_opcode(OpCode::Push);
                    self.emit_u16(idx);
                }
                self.emit_opcode(OpCode::Return);
            }
            Stmt::Throw(expr) => {
                self.compile_expression(expr)?;
                self.emit_opcode(OpCode::Throw);
            }
            Stmt::Yield => {
                // Suspend the current coroutine task and re-enqueue it
                self.emit_opcode(OpCode::Yield);
            }
            Stmt::TryCatch(try_block, catch_param, catch_block) => {
                self.emit_opcode(OpCode::TryStart);
                let try_start_offset = self.instructions.len();
                self.emit_u32(0); // placeholder for catch block
                
                self.compile_block(try_block)?;
                self.emit_opcode(OpCode::TryEnd);
                
                self.emit_opcode(OpCode::Jump);
                let jump_end_offset = self.instructions.len();
                self.emit_u32(0); // placeholder for end of catch block
                
                // Catch block
                let catch_addr = self.instructions.len() as u32;
                self.patch_u32(try_start_offset, catch_addr);
                
                // Catch param needs to be stored in locals
                let local_idx = if let Some(&idx) = self.locals.get(&catch_param) {
                    idx as u8
                } else {
                    let idx = self.local_count;
                    self.locals.insert(catch_param.clone(), idx);
                    self.local_count += 1;
                    idx as u8
                };
                self.emit_opcode(OpCode::StoreLocal);
                self.emit_u16(local_idx as u16);
                
                self.compile_block(catch_block)?;
                
                // End of catch block
                let end_addr = self.instructions.len() as u32;
                self.patch_u32(jump_end_offset, end_addr);
            }
            Stmt::If(cond, cons, alt) => {
                self.compile_expression(cond)?;
                
                // We emit a conditional jump to the alternative/end
                // Our jump logic: evaluate cond. If true, continue.
                // Wait, we have JumpIf. It jumps if TRUE.
                // It's easier to jump if FALSE to the alternative block. But we don't have JumpIfNot.
                // We could emit: JumpIf <consequence>, Jump <alternative>
                
                // JumpIf <cons>
                self.emit_opcode(OpCode::JumpIf);
                let jump_if_offset = self.instructions.len();
                self.emit_u32(0); // placeholder for consequence

                // Jump <alt or end>
                self.emit_opcode(OpCode::Jump);
                let jump_alt_offset = self.instructions.len();
                self.emit_u32(0); // placeholder for alternative

                // Consequence block
                let cons_addr = self.instructions.len() as u32;
                self.patch_u32(jump_if_offset, cons_addr);
                self.compile_block(cons)?;

                // Jump <end>
                self.emit_opcode(OpCode::Jump);
                let jump_end_offset = self.instructions.len();
                self.emit_u32(0);

                // Alternative block
                let alt_addr = self.instructions.len() as u32;
                self.patch_u32(jump_alt_offset, alt_addr);

                if let Some(alt_block) = alt {
                    self.compile_block(alt_block)?;
                }

                // End
                let end_addr = self.instructions.len() as u32;
                self.patch_u32(jump_end_offset, end_addr);
            }
            Stmt::While(cond, body) => {
                let start_addr = self.instructions.len() as u32;

                self.compile_expression(cond)?;
                
                // JumpIf body
                self.emit_opcode(OpCode::JumpIf);
                let jump_if_offset = self.instructions.len();
                self.emit_u32(0); // placeholder for body

                // Jump end
                self.emit_opcode(OpCode::Jump);
                let jump_end_offset = self.instructions.len();
                self.emit_u32(0); // placeholder for end

                // Body
                let body_addr = self.instructions.len() as u32;
                self.patch_u32(jump_if_offset, body_addr);
                self.compile_block(body)?;

                // Jump back to start
                self.emit_opcode(OpCode::Jump);
                self.emit_u32(start_addr);

                // End
                let end_addr = self.instructions.len() as u32;
                self.patch_u32(jump_end_offset, end_addr);
            }
            Stmt::For(init, cond, inc, body) => {
                self.compile_statement(*init)?;

                let start_addr = self.instructions.len() as u32;

                self.compile_expression(cond)?;
                
                // JumpIf body
                self.emit_opcode(OpCode::JumpIf);
                let jump_if_offset = self.instructions.len();
                self.emit_u32(0); // placeholder for body

                // Jump end
                self.emit_opcode(OpCode::Jump);
                let jump_end_offset = self.instructions.len();
                self.emit_u32(0); // placeholder for end

                // Body
                let body_addr = self.instructions.len() as u32;
                self.patch_u32(jump_if_offset, body_addr);
                self.compile_block(body)?;

                // Increment
                self.compile_statement(*inc)?;

                // Jump back to start
                self.emit_opcode(OpCode::Jump);
                self.emit_u32(start_addr);

                // End
                let end_addr = self.instructions.len() as u32;
                self.patch_u32(jump_end_offset, end_addr);
            }
        }
        Ok(())
    }

    fn compile_expression(&mut self, expr: Expr) -> Result<(), String> {
        match expr {
            Expr::StringLiteral(s) => {
                let idx = self.add_constant(Value::String(s));
                self.emit_opcode(OpCode::Push);
                self.emit_u16(idx);
            }
            Expr::Number(val) => {
                let idx = self.add_constant(Value::Float64(val as f64));
                self.emit_opcode(OpCode::Push);
                self.emit_u16(idx);
            }
            Expr::Boolean(b) => {
                let idx = self.add_constant(Value::Bool(b));
                self.emit_opcode(OpCode::Push);
                self.emit_u16(idx);
            }
            Expr::Identifier(name, expr_type) => {
                if let Some(&idx) = self.locals.get(&name) {
                    let op = match expr_type.unwrap_or(Type::Any) {
                        Type::Int32 => OpCode::LoadLocalI32,
                        Type::Int64 => OpCode::LoadLocalI64,
                        Type::Float32 => OpCode::LoadLocalF32,
                        Type::Float64 => OpCode::LoadLocalF64,
                        Type::String => OpCode::LoadLocalStr,
                        _ => OpCode::LoadLocal,
                    };
                    self.emit_opcode(op);
                    self.emit_u16(idx);
                } else if let Some(upvalue_idx) = self.resolve_upvalue(&name) {
                    self.emit_opcode(OpCode::LoadUpvalue);
                    self.emit_u16(upvalue_idx);
                } else {
                    return Err(format!("Undefined variable: {}", name));
                }
            }
            Expr::BinaryOp(left, op, right, opt_ty) => {
                let left_ty = match &*left {
                    Expr::Number(_) => Type::Float64,
                    Expr::StringLiteral(_) => Type::String,
                    Expr::Boolean(_) => Type::Bool,
                    Expr::Identifier(_, t) | Expr::BinaryOp(_, _, _, t) | Expr::Call(_, _, t) | Expr::MethodCall(_, _, _, t) | Expr::PropertyAccess(_, _, t) | Expr::NamedCall(_, _, t) | Expr::NamedMethodCall(_, _, _, t) | Expr::IndexAccess(_, _, t) => t.clone().unwrap_or(Type::Float64),
                    Expr::NewClass(n, _) | Expr::NamedNewClass(n, _) => Type::Class(n.clone()),
                    Expr::StructInit(n, _) => Type::Struct(n.clone()),
                    _ => Type::Float64,
                };
                
                // For Add/Sub/Mul/Div, the operation type is the result type (opt_ty)
                let math_ty = opt_ty.clone().unwrap_or(Type::Float64);
                
                // For Comparisons, the operation type is the operand's type
                let cmp_ty = if left_ty == Type::Any { Type::Float64 } else { left_ty };

                self.compile_expression(*left)?;
                self.compile_expression(*right)?;
                match op {
                    BinaryOperator::Add => match math_ty {
                        Type::Int32 => self.emit_opcode(OpCode::AddI32),
                        Type::Int64 => self.emit_opcode(OpCode::AddI64),
                        Type::Float32 => self.emit_opcode(OpCode::AddF32),
                        Type::String => self.emit_opcode(OpCode::AddStr),
                        _ => self.emit_opcode(OpCode::AddF64),
                    },
                    BinaryOperator::Subtract => match math_ty {
                        Type::Int32 => self.emit_opcode(OpCode::SubI32),
                        Type::Int64 => self.emit_opcode(OpCode::SubI64),
                        Type::Float32 => self.emit_opcode(OpCode::SubF32),
                        _ => self.emit_opcode(OpCode::SubF64),
                    },
                    BinaryOperator::Multiply => match math_ty {
                        Type::Int32 => self.emit_opcode(OpCode::MulI32),
                        Type::Int64 => self.emit_opcode(OpCode::MulI64),
                        Type::Float32 => self.emit_opcode(OpCode::MulF32),
                        _ => self.emit_opcode(OpCode::MulF64),
                    },
                    BinaryOperator::Divide => match math_ty {
                        Type::Int32 => self.emit_opcode(OpCode::DivI32),
                        Type::Int64 => self.emit_opcode(OpCode::DivI64),
                        Type::Float32 => self.emit_opcode(OpCode::DivF32),
                        _ => self.emit_opcode(OpCode::DivF64),
                    },
                    BinaryOperator::Equals => match cmp_ty {
                        Type::Int32 => self.emit_opcode(OpCode::EqualI32),
                        Type::Int64 => self.emit_opcode(OpCode::EqualI64),
                        Type::Float32 => self.emit_opcode(OpCode::EqualF32),
                        Type::String => self.emit_opcode(OpCode::EqualStr),
                        Type::Bool => self.emit_opcode(OpCode::EqualBool),
                        _ => self.emit_opcode(OpCode::EqualF64),
                    },
                    BinaryOperator::NotEquals => match cmp_ty {
                        Type::Int32 => self.emit_opcode(OpCode::NotEqualI32),
                        Type::Int64 => self.emit_opcode(OpCode::NotEqualI64),
                        Type::Float32 => self.emit_opcode(OpCode::NotEqualF32),
                        Type::String => self.emit_opcode(OpCode::NotEqualStr),
                        Type::Bool => self.emit_opcode(OpCode::NotEqualBool),
                        _ => self.emit_opcode(OpCode::NotEqualF64),
                    },
                    BinaryOperator::LessThan => match cmp_ty {
                        Type::Int32 => self.emit_opcode(OpCode::LessI32),
                        Type::Int64 => self.emit_opcode(OpCode::LessI64),
                        Type::Float32 => self.emit_opcode(OpCode::LessF32),
                        _ => self.emit_opcode(OpCode::LessF64),
                    },
                    BinaryOperator::LessThanOrEq => match cmp_ty {
                        Type::Int32 => self.emit_opcode(OpCode::LessEqualI32),
                        Type::Int64 => self.emit_opcode(OpCode::LessEqualI64),
                        Type::Float32 => self.emit_opcode(OpCode::LessEqualF32),
                        _ => self.emit_opcode(OpCode::LessEqualF64),
                    },
                    BinaryOperator::GreaterThan => match cmp_ty {
                        Type::Int32 => self.emit_opcode(OpCode::GreaterI32),
                        Type::Int64 => self.emit_opcode(OpCode::GreaterI64),
                        Type::Float32 => self.emit_opcode(OpCode::GreaterF32),
                        _ => self.emit_opcode(OpCode::GreaterF64),
                    },
                    BinaryOperator::GreaterThanOrEq => match cmp_ty {
                        Type::Int32 => self.emit_opcode(OpCode::GreaterEqualI32),
                        Type::Int64 => self.emit_opcode(OpCode::GreaterEqualI64),
                        Type::Float32 => self.emit_opcode(OpCode::GreaterEqualF32),
                        _ => self.emit_opcode(OpCode::GreaterEqualF64),
                    },
                    BinaryOperator::And => self.emit_opcode(OpCode::AndBool),
                    BinaryOperator::Or => self.emit_opcode(OpCode::OrBool),
                }
            }
            Expr::Call(name, args, _) => {
                match name.as_str() {
                    "print" => {
                        self.compile_expression(args[0].clone())?;
                        self.emit_opcode(OpCode::Syscall);
                        self.emit_u8(0x01);
                        let idx = self.add_constant(Value::Float64(0.0));
                        self.emit_opcode(OpCode::Push);
                        self.emit_u16(idx);
                    }
                    "read_char" => {
                        self.emit_opcode(OpCode::Syscall);
                        self.emit_u8(0x02);
                    }
                    "read" => {
                        self.compile_expression(args[0].clone())?;
                        self.compile_expression(args[1].clone())?;
                        self.emit_opcode(OpCode::Syscall);
                        self.emit_u8(0x03);
                    }
                    "write" => {
                        self.compile_expression(args[0].clone())?;
                        self.compile_expression(args[1].clone())?;
                        self.emit_opcode(OpCode::Syscall);
                        self.emit_u8(0x04);
                    }
                    "close" => {
                        self.compile_expression(args[0].clone())?;
                        self.emit_opcode(OpCode::Syscall);
                        self.emit_u8(0x05);
                    }
                    "file_open" => {
                        self.compile_expression(args[0].clone())?;
                        self.emit_opcode(OpCode::Syscall);
                        self.emit_u8(0x10);
                    }
                    "sleep" => {
                        self.compile_expression(args[0].clone())?;
                        self.emit_opcode(OpCode::Syscall);
                        self.emit_u8(0x06);
                    }
                    "sleep_async" => {
                        self.compile_expression(args[0].clone())?;
                        self.emit_opcode(OpCode::Syscall);
                        self.emit_u8(0x08);
                    }
                    "await" => {
                        self.compile_expression(args[0].clone())?;
                        self.emit_opcode(OpCode::Await);
                    }
                    "spawn" => {
                        if let Expr::Identifier(func_name, _) = &args[0] {
                            self.emit_opcode(OpCode::Spawn);
                            self.unresolved_calls.push((self.instructions.len(), func_name.clone(), 0));
                            self.emit_u32(0);
                        } else {
                            return Err("spawn expects a function identifier".to_string());
                        }
                    }
                    "gc" => {
                        self.emit_opcode(OpCode::Syscall);
                        self.emit_u8(0x07);
                    }
                    "net_connect" => {
                        self.compile_expression(args[0].clone())?;
                        self.compile_expression(args[1].clone())?;
                        self.emit_opcode(OpCode::Syscall);
                        self.emit_u8(0x20);
                    }
                    "net_set_nonblocking" => {
                        self.compile_expression(args[0].clone())?;
                        self.compile_expression(args[1].clone())?;
                        self.emit_opcode(OpCode::Syscall);
                        self.emit_u8(0x23);
                    }
                    "net_listen" => {
                        self.compile_expression(args[0].clone())?;
                        self.emit_opcode(OpCode::Syscall);
                        self.emit_u8(0x21);
                    }
                    "net_accept" => {
                        self.compile_expression(args[0].clone())?;
                        self.emit_opcode(OpCode::Syscall);
                        self.emit_u8(0x22);
                    }
                    "net_poll" => {
                        self.emit_opcode(OpCode::Syscall);
                        self.emit_u8(0x24);
                    }
                    "net_read" => {
                        self.compile_expression(args[0].clone())?;
                        self.compile_expression(args[1].clone())?;
                        self.emit_opcode(OpCode::Syscall);
                        self.emit_u8(0x25);
                    }
                    "net_write" => {
                        self.compile_expression(args[0].clone())?;
                        self.compile_expression(args[1].clone())?;
                        self.emit_opcode(OpCode::Syscall);
                        self.emit_u8(0x26);
                    }
                    "net_close" => {
                        self.compile_expression(args[0].clone())?;
                        self.emit_opcode(OpCode::Syscall);
                        self.emit_u8(0x27);
                    }
                    "string_to_bytes" => {
                        self.compile_expression(args[0].clone())?;
                        self.emit_opcode(OpCode::Syscall);
                        self.emit_u8(0x30);
                    }
                    "bytes_to_string" => {
                        self.compile_expression(args[0].clone())?;
                        self.emit_opcode(OpCode::Syscall);
                        self.emit_u8(0x31);
                    }

                    _ => {
                        // User-defined function
                        let arg_count = args.len() as u8;
                        for arg in args {
                            self.compile_expression(arg)?;
                        }
                        self.emit_opcode(OpCode::Call);
                        self.unresolved_calls.push((self.instructions.len(), name.clone(), arg_count));
                        self.emit_u32(0);
                        self.emit_u16(256);
                    }
                }
            }
            Expr::ArrayLiteral(elements) => {
                let size = elements.len() as f64;
                let size_idx = self.add_constant(Value::Float64(size));
                self.emit_opcode(OpCode::Push);
                self.emit_u16(size_idx);
                self.emit_opcode(OpCode::NewArray);
                
                for (i, elem) in elements.into_iter().enumerate() {
                    self.emit_opcode(OpCode::Dup); // Dup the Ref
                    
                    let idx = self.add_constant(Value::Float64(i as f64));
                    self.emit_opcode(OpCode::Push);
                    self.emit_u16(idx);
                    
                    self.compile_expression(elem)?;
                    self.emit_opcode(OpCode::StoreElement);
                }
            }
            Expr::IndexAccess(array_expr, index_expr, _) => {
                self.compile_expression(*array_expr)?; // pushes Ref
                self.compile_expression(*index_expr)?; // pushes index
                self.emit_opcode(OpCode::LoadElement);
            }
            Expr::StructInit(_, fields) => {
                let count = fields.len() as f64;
                for (key, val_expr) in fields {
                    let key_idx = self.add_constant(Value::String(key));
                    self.emit_opcode(OpCode::Push);
                    self.emit_u16(key_idx);
                    self.compile_expression(val_expr)?;
                }
                let count_idx = self.add_constant(Value::Float64(count));
                self.emit_opcode(OpCode::Push);
                self.emit_u16(count_idx);
                self.emit_opcode(OpCode::NewStruct);
            }
            Expr::DictLiteral(elements) => {
                let count = elements.len() as f64;
                for (key_expr, val_expr) in elements {
                    self.compile_expression(key_expr)?;
                    self.compile_expression(val_expr)?;
                }
                let count_idx = self.add_constant(Value::Float64(count));
                self.emit_opcode(OpCode::Push);
                self.emit_u16(count_idx);
                self.emit_opcode(OpCode::NewDict);
            }
            Expr::PropertyAccess(obj_expr, prop_name, _) => {
                self.compile_expression(*obj_expr)?;
                let prop_idx = self.add_constant(Value::String(prop_name));
                self.emit_opcode(OpCode::LoadProperty);
                self.emit_u16(prop_idx);
            }
            Expr::NewClass(_, _) | Expr::MethodCall(_, _, _, _) | Expr::CallDynamic(_, _, _) => {
                unreachable!("Type checker transforms these into StructInit and Call or VIR builder handles it")
            }
            Expr::NamedCall(name, args, _) => {
                let param_names = match self.function_signatures.get(&name) {
                    Some(params) => params.clone(),
                    None => return Err(format!("Cannot use named arguments for unknown or builtin function {}", name)),
                };

                let mut reordered_args = vec![None; param_names.len()];
                let mut positional_index = 0;

                for arg in args {
                    if let Some(arg_name) = arg.name {
                        if let Some(idx) = param_names.iter().position(|p| p == &arg_name) {
                            reordered_args[idx] = Some(arg.value);
                        } else {
                            return Err(format!("Function {} has no parameter named {}", name, arg_name));
                        }
                    } else {
                        while positional_index < reordered_args.len() && reordered_args[positional_index].is_some() {
                            positional_index += 1;
                        }
                        if positional_index < reordered_args.len() {
                            reordered_args[positional_index] = Some(arg.value);
                            positional_index += 1;
                        } else {
                            return Err(format!("Too many positional arguments for function {}", name));
                        }
                    }
                }

                let arg_count = reordered_args.len() as u8;
                for (i, arg_opt) in reordered_args.into_iter().enumerate() {
                    match arg_opt {
                        Some(expr) => self.compile_expression(expr)?,
                        None => return Err(format!("Missing argument '{}' for function {}", param_names[i], name)),
                    }
                }

                self.emit_opcode(OpCode::Call);
                self.unresolved_calls.push((self.instructions.len(), name.clone(), arg_count));
                self.emit_u32(0);
                self.emit_u16(256);
            }
            Expr::NamedMethodCall(_, _, _, _) | Expr::NamedNewClass(_, _) => {
                return Err("v2 AST nodes not yet supported in bytecode compiler".into());
            }
            Expr::Closure { params, return_type: _, body } => {
                if self.outer_locals.is_some() {
                    return Err("Nested closures (more than 1 level) are not yet supported".into());
                }

                self.emit_opcode(OpCode::Jump);
                let jump_over_offset = self.instructions.len();
                self.emit_u32(0); // placeholder

                let closure_start = self.instructions.len() as u32;

                let saved_locals = self.locals.clone();
                let saved_local_count = self.local_count;

                self.outer_locals = Some(saved_locals.clone());
                self.current_upvalues.clear();

                self.locals.clear();
                self.local_count = 0;

                for param in params.iter() {
                    self.locals.insert(param.0.clone(), self.local_count);
                    self.local_count += 1;
                }

                for param in params.iter().rev() {
                    let idx = *self.locals.get(&param.0).unwrap();
                    self.emit_opcode(OpCode::StoreLocal);
                    self.emit_u16(idx);
                }

                self.compile_block(body)?;

                let null_idx = self.add_constant(Value::Null);
                self.emit_opcode(OpCode::Push);
                self.emit_u16(null_idx);
                self.emit_opcode(OpCode::Return);

                self.locals = saved_locals;
                self.local_count = saved_local_count;
                self.outer_locals = None;

                let closure_end = self.instructions.len() as u32;
                self.patch_u32(jump_over_offset, closure_end);

                let upvalues = self.current_upvalues.clone();
                for (_, outer_idx) in &upvalues {
                    self.emit_opcode(OpCode::LoadLocal);
                    self.emit_u16(*outer_idx);
                    self.emit_opcode(OpCode::BoxValue);
                }

                self.emit_opcode(OpCode::NewClosure);
                self.emit_u32(closure_start);
                self.emit_u16(upvalues.len() as u16);
            }
        }
        Ok(())
    }

    fn resolve_upvalue(&mut self, name: &str) -> Option<u16> {
        if let Some(outer) = &self.outer_locals {
            if let Some(&outer_idx) = outer.get(name) {
                if let Some(pos) = self.current_upvalues.iter().position(|(n, _)| n == name) {
                    return Some(pos as u16);
                }
                let pos = self.current_upvalues.len() as u16;
                self.current_upvalues.push((name.to_string(), outer_idx));
                return Some(pos);
            }
        }
        None
    }

    fn add_constant(&mut self, val: Value) -> u16 {
        for (i, c) in self.constants.iter().enumerate() {
            if c == &val {
                return i as u16;
            }
        }
        let idx = self.constants.len() as u16;
        self.constants.push(val);
        idx
    }

    fn emit_opcode(&mut self, op: OpCode) {
        self.instructions.push(op as u8);
    }

    fn emit_u8(&mut self, val: u8) {
        self.instructions.push(val);
    }

    fn emit_u16(&mut self, val: u16) {
        self.instructions.extend_from_slice(&val.to_be_bytes());
    }

    fn emit_u32(&mut self, val: u32) {
        self.instructions.extend_from_slice(&val.to_be_bytes());
    }

    fn patch_u32(&mut self, offset: usize, val: u32) {
        let bytes = val.to_be_bytes();
        self.instructions[offset] = bytes[0];
        self.instructions[offset + 1] = bytes[1];
        self.instructions[offset + 2] = bytes[2];
        self.instructions[offset + 3] = bytes[3];
    }
}

fn count_locals(block: &Block) -> u16 {
    let mut count = 0;
    for stmt in block {
        match stmt {
            Stmt::Let(..) => count += 1,
            Stmt::If(_, cons, alt) => {
                count += count_locals(cons);
                if let Some(a) = alt {
                    count += count_locals(a);
                }
            }
            Stmt::While(_, body) => count += count_locals(body),
            Stmt::For(init, _, _, body) => {
                if let Stmt::Let(..) = **init { count += 1; }
                count += count_locals(body);
            }
            Stmt::TryCatch(try_block, _, catch_block) => {
                count += count_locals(try_block);
                count += count_locals(catch_block);
                count += 1; // For the catch param
            }
            _ => {}
        }
    }
    count


}
