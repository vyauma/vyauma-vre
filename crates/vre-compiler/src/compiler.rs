use std::collections::HashMap;
use vre_core::bytecode::opcode::OpCode;
use vre_core::vm::value::Value;

use crate::ast::{Type, Program, Stmt, Expr, BinaryOperator, Block, Function};

#[derive(Debug)]
pub struct CompiledProgram {
    pub instructions: Vec<u8>,
    pub constants: Vec<Value>,
    pub native_imports: Vec<String>,
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
        }
    }

    pub fn compile(mut self, program: Program) -> Result<CompiledProgram, String> {
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

        // Ensure functions always return
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
                let idx = self.local_count;
                self.locals.insert(name, idx);
                self.local_count += 1;
                self.emit_opcode(OpCode::StoreLocal);
                self.emit_u16(idx);
            }
            Stmt::Assign(name, expr) => {
                self.compile_expression(expr)?;
                if let Some(&idx) = self.locals.get(&name) {
                    self.emit_opcode(OpCode::StoreLocal);
                    self.emit_u16(idx);
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
            Stmt::StructDecl(_, _) | Stmt::ClassDecl(_, _, _) => {
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
                } else {
                    return Err(format!("Undefined variable: {}", name));
                }
            }
            Expr::BinaryOp(left, op, right, expr_type) => {
                self.compile_expression(*left)?;
                self.compile_expression(*right)?;
                let ty = expr_type.unwrap_or(Type::Float64);
                match op {
                    BinaryOperator::Add => match ty {
                        Type::Int32 => self.emit_opcode(OpCode::AddI32),
                        Type::Int64 => self.emit_opcode(OpCode::AddI64),
                        Type::Float32 => self.emit_opcode(OpCode::AddF32),
                        _ => self.emit_opcode(OpCode::AddF64),
                    },
                    BinaryOperator::Subtract => match ty {
                        Type::Int32 => self.emit_opcode(OpCode::SubI32),
                        Type::Int64 => self.emit_opcode(OpCode::SubI64),
                        Type::Float32 => self.emit_opcode(OpCode::SubF32),
                        _ => self.emit_opcode(OpCode::SubF64),
                    },
                    BinaryOperator::Multiply => match ty {
                        Type::Int32 => self.emit_opcode(OpCode::MulI32),
                        Type::Int64 => self.emit_opcode(OpCode::MulI64),
                        Type::Float32 => self.emit_opcode(OpCode::MulF32),
                        _ => self.emit_opcode(OpCode::MulF64),
                    },
                    BinaryOperator::Divide => match ty {
                        Type::Int32 => self.emit_opcode(OpCode::DivI32),
                        Type::Int64 => self.emit_opcode(OpCode::DivI64),
                        Type::Float32 => self.emit_opcode(OpCode::DivF32),
                        _ => self.emit_opcode(OpCode::DivF64),
                    },
                    BinaryOperator::Equals => match ty {
                        Type::Int32 => self.emit_opcode(OpCode::EqualI32),
                        Type::Int64 => self.emit_opcode(OpCode::EqualI64),
                        Type::Float32 => self.emit_opcode(OpCode::EqualF32),
                        Type::String => self.emit_opcode(OpCode::EqualStr),
                        _ => self.emit_opcode(OpCode::EqualF64),
                    },
                    BinaryOperator::NotEquals => match ty {
                        Type::Int32 => self.emit_opcode(OpCode::NotEqualI32),
                        Type::Int64 => self.emit_opcode(OpCode::NotEqualI64),
                        Type::Float32 => self.emit_opcode(OpCode::NotEqualF32),
                        Type::String => self.emit_opcode(OpCode::NotEqualStr),
                        _ => self.emit_opcode(OpCode::NotEqualF64),
                    },
                    BinaryOperator::LessThan => match ty {
                        Type::Int32 => self.emit_opcode(OpCode::LessI32),
                        Type::Int64 => self.emit_opcode(OpCode::LessI64),
                        Type::Float32 => self.emit_opcode(OpCode::LessF32),
                        _ => self.emit_opcode(OpCode::LessF64),
                    },
                    BinaryOperator::LessThanOrEq => match ty {
                        Type::Int32 => self.emit_opcode(OpCode::LessEqualI32),
                        Type::Int64 => self.emit_opcode(OpCode::LessEqualI64),
                        Type::Float32 => self.emit_opcode(OpCode::LessEqualF32),
                        _ => self.emit_opcode(OpCode::LessEqualF64),
                    },
                    BinaryOperator::GreaterThan => match ty {
                        Type::Int32 => self.emit_opcode(OpCode::GreaterI32),
                        Type::Int64 => self.emit_opcode(OpCode::GreaterI64),
                        Type::Float32 => self.emit_opcode(OpCode::GreaterF32),
                        _ => self.emit_opcode(OpCode::GreaterF64),
                    },
                    BinaryOperator::GreaterThanOrEq => match ty {
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
            Expr::IndexAccess(array_expr, index_expr) => {
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
                self.emit_opcode(OpCode::NewStruct);
            }
            Expr::PropertyAccess(obj_expr, prop_name, _) => {
                self.compile_expression(*obj_expr)?;
                let prop_idx = self.add_constant(Value::String(prop_name));
                self.emit_opcode(OpCode::LoadProperty);
                self.emit_u16(prop_idx);
            }
            Expr::NewClass(_, _) | Expr::MethodCall(_, _, _, _) => {
                unreachable!("Type checker transforms these into StructInit and Call")
            }
        }
        Ok(())
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
