use super::{Module, Function, Instruction, BlockId, Value};
use crate::compiler::CompiledProgram;
use vre_core::bytecode::opcode::OpCode;
use vre_core::vm::value::Value as VmValue;
use std::collections::HashMap;

pub struct VirCodegen {
    instructions: Vec<u8>,
    constants: Vec<VmValue>,
    
    functions: HashMap<String, u32>,
    unresolved_calls: Vec<(usize, String, u8)>,
    /// (patch_offset, function_name) — resolved like Call but emits Spawn
    unresolved_spawns: Vec<(usize, String)>,
    
    native_imports: Vec<String>,
}

struct FuncContext {
    locals: HashMap<String, u16>,
    registers: HashMap<Value, u16>, // VIR Value -> Local index
    local_count: u16,
    block_offsets: HashMap<BlockId, u32>,
    unresolved_branches: Vec<(usize, BlockId)>, // offset to patch, target block
    unresolved_cond_branches: Vec<(usize, BlockId, BlockId)>, // offset, cons, alt
    unresolved_tries: Vec<(usize, BlockId)>, // offset, catch block
}

impl VirCodegen {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            constants: Vec::new(),
            functions: HashMap::new(),
            unresolved_calls: Vec::new(),
            unresolved_spawns: Vec::new(),
            native_imports: Vec::new(),
        }
    }
    
    pub fn generate(mut self, module: Module) -> Result<CompiledProgram, String> {
        self.emit_opcode(OpCode::Call);
        let main_call_target_offset = self.instructions.len();
        self.emit_u32(0);
        self.emit_u16(256);
        self.emit_opcode(OpCode::Return);
        
        for func in module.functions {
            self.compile_function(func)?;
        }
        
        if let Some(&main_addr) = self.functions.get("@main").or_else(|| self.functions.get("main")) {
            self.patch_u32(main_call_target_offset, main_addr);
        } else {
            return Err("No main function found".to_string());
        }
        
        println!("FUNCTIONS MAP: {:?}", self.functions);
        let unresolved_calls = std::mem::take(&mut self.unresolved_calls);
        for (offset, name, arg_count) in unresolved_calls {
            if let Some(&addr) = self.functions.get(&name) {
                // Patch address (u32)
                self.patch_u32(offset, addr);
                // For regular Call, also patch local_count (u16) — arg_count==0xFF signals NewClosure (addr-only)
                if arg_count != 0xFF {
                    let local_bytes = (256u16).to_be_bytes();
                    self.instructions[offset + 4] = local_bytes[0];
                    self.instructions[offset + 5] = local_bytes[1];
                }
            } else {
                // Must be a native import — only valid for regular Call opcode (arg_count != 0xFF)
                if arg_count == 0xFF {
                    return Err(format!("NewClosure references unknown function '{}'", name));
                }
                let mut native_idx = self.native_imports.iter().position(|n| n == &name);
                if native_idx.is_none() {
                    self.native_imports.push(name.clone());
                    native_idx = Some(self.native_imports.len() - 1);
                }

                let import_idx = native_idx.unwrap() as u16;
                let original_opcode_offset = offset - 1;
                self.instructions[original_opcode_offset] = OpCode::CallNative as u8;

                let bytes = import_idx.to_be_bytes();
                self.instructions[offset] = bytes[0];
                self.instructions[offset + 1] = bytes[1];
                self.instructions[offset + 2] = arg_count;
                self.instructions[offset + 3] = OpCode::Nop as u8;
                self.instructions[offset + 4] = OpCode::Nop as u8;
                self.instructions[offset + 5] = OpCode::Nop as u8;
            }
        }

        // Link SpawnTask targets (must refer to user-defined functions)
        let unresolved_spawns = std::mem::take(&mut self.unresolved_spawns);
        for (offset, name) in unresolved_spawns {
            if let Some(&addr) = self.functions.get(&name) {
                self.patch_u32(offset, addr);
            } else {
                return Err(format!("vre_spawn: function '{}' not found — only user-defined functions can be spawned", name));
            }
        }
        
        Ok(CompiledProgram {
            instructions: self.instructions,
            constants: self.constants,
            native_imports: self.native_imports,
            function_table: self.functions.clone(),
        })
    }
    
    fn compile_function(&mut self, func: Function) -> Result<(), String> {
        let start_addr = self.instructions.len() as u32;
        self.functions.insert(func.name.clone(), start_addr);
        
        let mut ctx = FuncContext {
            locals: HashMap::new(),
            registers: HashMap::new(),
            local_count: 0,
            block_offsets: HashMap::new(),
            unresolved_branches: Vec::new(),
            unresolved_cond_branches: Vec::new(),
            unresolved_tries: Vec::new(),
        };
        
        // Define params
        for param in func.params.iter() {
            ctx.locals.insert(param.clone(), ctx.local_count);
            ctx.local_count += 1;
        }

        // Pop arguments into locals
        for param in func.params.iter().rev() {
            let idx = *ctx.locals.get(param).unwrap();
            self.emit_opcode(OpCode::StoreLocal);
            self.emit_u16(idx);
        }
        
        // Sort blocks by ID or sequential structure
        // Since we emit linearly, we just emit them in order
        for block in func.blocks {
            ctx.block_offsets.insert(block.id, self.instructions.len() as u32);
            for (val, inst) in block.instructions {
                self.compile_instruction(val, &inst, &mut ctx)?;
            }
        }
        
        // Patch branches
        for (offset, target_block) in ctx.unresolved_branches {
            let target_addr = *ctx.block_offsets.get(&target_block).unwrap();
            self.patch_u32(offset, target_addr);
        }
        for (offset, cons_block, alt_block) in ctx.unresolved_cond_branches {
            let cons_addr = *ctx.block_offsets.get(&cons_block).unwrap();
            let alt_addr = *ctx.block_offsets.get(&alt_block).unwrap();
            self.patch_u32(offset, cons_addr);
            self.patch_u32(offset + 5, alt_addr);
        }
        for (offset, catch_block) in ctx.unresolved_tries {
            let catch_addr = *ctx.block_offsets.get(&catch_block).unwrap();
            self.patch_u32(offset, catch_addr);
        }
        
        Ok(())
    }
    
    fn compile_instruction(&mut self, val: Value, inst: &Instruction, ctx: &mut FuncContext) -> Result<(), String> {
        match inst {
            Instruction::LoadConstNumber(n) => {
                let idx = self.add_constant(VmValue::Float64(*n));
                self.emit_opcode(OpCode::Push);
                self.emit_u16(idx);
                self.store_val(val, ctx);
            }
            Instruction::LoadConstBool(b) => {
                let idx = self.add_constant(VmValue::Bool(*b));
                self.emit_opcode(OpCode::Push);
                self.emit_u16(idx);
                self.store_val(val, ctx);
            }
            Instruction::LoadConstString(s) => {
                let idx = self.add_constant(VmValue::String(s.clone()));
                self.emit_opcode(OpCode::Push);
                self.emit_u16(idx);
                self.store_val(val, ctx);
            }
            Instruction::LoadNull => {
                let idx = self.add_constant(VmValue::Null);
                self.emit_opcode(OpCode::Push);
                self.emit_u16(idx);
                self.store_val(val, ctx);
            }
            Instruction::LoadVar(name) => {
                let local_idx = *ctx.locals.get(name).unwrap_or(&0); // Handle unmapped gracefully for now
                self.emit_opcode(OpCode::LoadLocal);
                self.emit_u16(local_idx);
                self.store_val(val, ctx);
            }
            Instruction::StoreVar(name, src_val) => {
                self.load_val(*src_val, ctx);
                let local_idx = if let Some(&idx) = ctx.locals.get(name) {
                    idx
                } else {
                    let idx = ctx.local_count;
                    ctx.locals.insert(name.clone(), idx);
                    ctx.local_count += 1;
                    idx
                };
                self.emit_opcode(OpCode::StoreLocal);
                self.emit_u16(local_idx);
            }
            Instruction::Add(l, r) => {
                self.load_val(*l, ctx);
                self.load_val(*r, ctx);
                self.emit_opcode(OpCode::AddF64);
                self.store_val(val, ctx);
            }
            Instruction::AddStr(l, r) => {
                self.load_val(*l, ctx);
                self.load_val(*r, ctx);
                self.emit_opcode(OpCode::AddStr);
                self.store_val(val, ctx);
            }
            Instruction::Sub(l, r) => {
                self.load_val(*l, ctx);
                self.load_val(*r, ctx);
                self.emit_opcode(OpCode::SubF64);
                self.store_val(val, ctx);
            }
            Instruction::Mul(l, r) => {
                self.load_val(*l, ctx);
                self.load_val(*r, ctx);
                self.emit_opcode(OpCode::MulF64);
                self.store_val(val, ctx);
            }
            Instruction::Div(l, r) => {
                self.load_val(*l, ctx);
                self.load_val(*r, ctx);
                self.emit_opcode(OpCode::DivF64);
                self.store_val(val, ctx);
            }
            Instruction::Eq(l, r) => {
                self.load_val(*l, ctx);
                self.load_val(*r, ctx);
                self.emit_opcode(OpCode::EqualF64);
                self.store_val(val, ctx);
            }
            Instruction::EqStr(l, r) => {
                self.load_val(*l, ctx);
                self.load_val(*r, ctx);
                self.emit_opcode(OpCode::EqualStr);
                self.store_val(val, ctx);
            }
            Instruction::NotEq(l, r) => {
                self.load_val(*l, ctx);
                self.load_val(*r, ctx);
                self.emit_opcode(OpCode::NotEqualF64);
                self.store_val(val, ctx);
            }
            Instruction::NotEqStr(l, r) => {
                self.load_val(*l, ctx);
                self.load_val(*r, ctx);
                self.emit_opcode(OpCode::NotEqualStr);
                self.store_val(val, ctx);
            }
            Instruction::Lt(l, r) => {
                self.load_val(*l, ctx);
                self.load_val(*r, ctx);
                self.emit_opcode(OpCode::LessF64);
                self.store_val(val, ctx);
            }
            Instruction::Lte(l, r) => {
                self.load_val(*l, ctx);
                self.load_val(*r, ctx);
                self.emit_opcode(OpCode::LessEqualF64);
                self.store_val(val, ctx);
            }
            Instruction::Gt(l, r) => {
                self.load_val(*l, ctx);
                self.load_val(*r, ctx);
                self.emit_opcode(OpCode::GreaterF64);
                self.store_val(val, ctx);
            }
            Instruction::Gte(l, r) => {
                self.load_val(*l, ctx);
                self.load_val(*r, ctx);
                self.emit_opcode(OpCode::GreaterEqualF64);
                self.store_val(val, ctx);
            }
            Instruction::Return(opt_val) => {
                if let Some(v) = opt_val {
                    self.load_val(*v, ctx);
                } else {
                    let idx = self.add_constant(VmValue::Null);
                    self.emit_opcode(OpCode::Push);
                    self.emit_u16(idx);
                }
                self.emit_opcode(OpCode::Return);
            }
            Instruction::Call(name, args) => {
                for arg in args {
                    self.load_val(*arg, ctx);
                }
                self.emit_opcode(OpCode::Call);
                let call_offset = self.instructions.len();
                self.emit_u32(0); // Placeholder for address
                self.emit_u16(0); // Placeholder for local count
                self.unresolved_calls.push((call_offset, name.clone(), args.len() as u8));
                self.store_val(val, ctx);
            }
            Instruction::Branch(target_block) => {
                self.emit_opcode(OpCode::Jump);
                let offset = self.instructions.len();
                self.emit_u32(0); // Placeholder
                ctx.unresolved_branches.push((offset, *target_block));
            }
            Instruction::CondBranch(cond_val, true_block, false_block) => {
                self.load_val(*cond_val, ctx);
                
                self.emit_opcode(OpCode::JumpIf);
                let cons_offset = self.instructions.len();
                self.emit_u32(0);
                
                self.emit_opcode(OpCode::Jump);
                let alt_offset = self.instructions.len();
                self.emit_u32(0);
                
                ctx.unresolved_cond_branches.push((cons_offset, *true_block, *false_block));
            }
            Instruction::ArrayLiteral(elements) => {
                for el in elements {
                    self.load_val(*el, ctx);
                }
                let len_idx = self.add_constant(VmValue::Float64(elements.len() as f64));
                self.emit_opcode(OpCode::Push);
                self.emit_u16(len_idx);
                self.emit_opcode(OpCode::NewArray);
                self.store_val(val, ctx);
            }
            Instruction::StructInit(_, fields) => {
                for (key, val_ref) in fields {
                    let key_idx = self.add_constant(VmValue::String(key.clone()));
                    self.emit_opcode(OpCode::Push);
                    self.emit_u16(key_idx);
                    
                    self.load_val(*val_ref, ctx);
                }
                let count_idx = self.add_constant(VmValue::Float64(fields.len() as f64));
                self.emit_opcode(OpCode::Push);
                self.emit_u16(count_idx);
                self.emit_opcode(OpCode::NewStruct);
                self.store_val(val, ctx);
            }
            Instruction::IndexAccess(obj, idx) => {
                self.load_val(*obj, ctx);
                self.load_val(*idx, ctx);
                self.emit_opcode(OpCode::LoadElement);
                self.store_val(val, ctx);
            }
            Instruction::PropertyAccess(obj, prop) => {
                self.load_val(*obj, ctx);
                let prop_idx = self.add_constant(VmValue::String(prop.clone()));
                self.emit_opcode(OpCode::LoadProperty);
                self.emit_u16(prop_idx);
                self.store_val(val, ctx);
            }
            Instruction::AssignIndex(obj, idx, src_val) => {
                self.load_val(*obj, ctx);
                self.load_val(*idx, ctx);
                self.load_val(*src_val, ctx);
                self.emit_opcode(OpCode::StoreElement);
            }
            Instruction::AssignProperty(obj, prop, src_val) => {
                self.load_val(*obj, ctx);
                self.load_val(*src_val, ctx);
                let prop_idx = self.add_constant(VmValue::String(prop.clone()));
                self.emit_opcode(OpCode::StoreProperty);
                self.emit_u16(prop_idx);
            }
            Instruction::NewClosure(func_name, cap_box_vals) => {
                // Push each captured box value (they are already on heap as References)
                for cap in cap_box_vals {
                    self.load_val(*cap, ctx);
                }
                self.emit_opcode(OpCode::NewClosure);
                // func addr placeholder — resolved after all functions compiled
                let call_offset = self.instructions.len();
                self.emit_u32(0);
                self.emit_u16(cap_box_vals.len() as u16);
                // 0xFF sentinel tells linker to only patch u32 addr, not overwrite the upvalue count u16
                self.unresolved_calls.push((call_offset, func_name.clone(), 0xFF));
                self.store_val(val, ctx);
            }
            Instruction::LoadUpvalue(idx) => {
                self.emit_opcode(OpCode::LoadUpvalue);
                self.emit_u16(*idx as u16);
                self.store_val(val, ctx);
            }
            Instruction::StoreUpvalue(idx, src_val) => {
                self.load_val(*src_val, ctx);
                self.emit_opcode(OpCode::StoreUpvalue);
                self.emit_u16(*idx as u16);
            }
            Instruction::BoxValue(src_val) => {
                self.load_val(*src_val, ctx);
                self.emit_opcode(OpCode::BoxValue);
                self.store_val(val, ctx);
            }
            Instruction::LoadBox(src_val) => {
                self.load_val(*src_val, ctx);
                self.emit_opcode(OpCode::LoadBox);
                self.store_val(val, ctx);
            }
            Instruction::StoreBox(box_val, src_val) => {
                self.load_val(*box_val, ctx);
                self.load_val(*src_val, ctx);
                self.emit_opcode(OpCode::StoreBox);
            }
            Instruction::CallDynamic(callee, args) => {
                self.load_val(*callee, ctx);
                for arg in args {
                    self.load_val(*arg, ctx);
                }
                self.emit_opcode(OpCode::CallDynamic);
                self.emit_u16(args.len() as u16);
                self.emit_u16(256u16); // local count (generous default)
                self.store_val(val, ctx);
            }
            Instruction::SpawnTask(name, args) => {
                // Push args onto the stack so the spawned task can read them
                for arg in args {
                    self.load_val(*arg, ctx);
                }
                // Emit Spawn opcode with placeholder address (resolved at link time)
                self.emit_opcode(OpCode::Spawn);
                let spawn_offset = self.instructions.len();
                self.emit_u32(0); // placeholder IP — patched after all functions are compiled
                self.unresolved_spawns.push((spawn_offset, name.clone()));
                // Spawn pushes the task_id onto the stack — store it
                self.store_val(val, ctx);
            }
            Instruction::SpawnDynamicTask(callee) => {
                self.load_val(*callee, ctx);
                self.emit_opcode(OpCode::SpawnDynamic);
                self.store_val(val, ctx);
            }
            Instruction::ImportModule(path) => {
                // Emit ImportModule opcode with a constant-pool string operand for the path
                let path_idx = self.add_constant(VmValue::String(path.clone()));
                self.emit_opcode(OpCode::ImportModule);
                self.emit_u16(path_idx);
                self.store_val(val, ctx);
            }
            Instruction::ExportValue(name, _src_val) => {
                // Push the name as a string constant, then load the value and emit ExportValue
                // The value is expected to already be on the stack from _src_val's register
                if let Some(&reg_idx) = ctx.registers.get(_src_val) {
                    self.emit_opcode(OpCode::LoadLocal);
                    self.emit_u16(reg_idx);
                }
                let name_idx = self.add_constant(VmValue::String(name.clone()));
                self.emit_opcode(OpCode::ExportValue);
                self.emit_u16(name_idx);
            }
            Instruction::Rem(l, r) => {
                self.load_val(*l, ctx);
                self.load_val(*r, ctx);
                self.emit_opcode(OpCode::ModF64);
                self.store_val(val, ctx);
            }
            Instruction::Not(v) => {
                self.load_val(*v, ctx);
                self.emit_opcode(OpCode::NotBool);
                self.store_val(val, ctx);
            }
            Instruction::And(l, r) => {
                self.load_val(*l, ctx);
                self.load_val(*r, ctx);
                self.emit_opcode(OpCode::AndBool);
                self.store_val(val, ctx);
            }
            Instruction::Or(l, r) => {
                self.load_val(*l, ctx);
                self.load_val(*r, ctx);
                self.emit_opcode(OpCode::OrBool);
                self.store_val(val, ctx);
            }
            Instruction::DictLiteral(pairs) => {
                for (k, v_ref) in pairs {
                    self.load_val(*k, ctx);
                    self.load_val(*v_ref, ctx);
                }
                let count_idx = self.add_constant(VmValue::Float64(pairs.len() as f64));
                self.emit_opcode(OpCode::Push);
                self.emit_u16(count_idx);
                self.emit_opcode(OpCode::NewStruct);
                self.store_val(val, ctx);
            }
            Instruction::MethodCall(obj, name, args) => {
                self.load_val(*obj, ctx);
                for arg in args {
                    self.load_val(*arg, ctx);
                }
                let name_idx = self.add_constant(VmValue::String(name.clone()));
                self.emit_opcode(OpCode::CallMethod);
                self.emit_u16(name_idx);
                self.emit_u16(args.len() as u16);
                self.store_val(val, ctx);
            }
            Instruction::NewClass(name, args) => {
                for arg in args {
                    self.load_val(*arg, ctx);
                }
                let name_idx = self.add_constant(VmValue::String(name.clone()));
                self.emit_opcode(OpCode::NewClass);
                self.emit_u16(name_idx);
                self.emit_u16(args.len() as u16);
                self.store_val(val, ctx);
            }
            Instruction::Syscall(code, args) => {
                for arg in args {
                    self.load_val(*arg, ctx);
                }
                self.emit_opcode(OpCode::Syscall);
                self.emit_u8(*code);
                
                // If it's a print syscall, legacy compiler expects to push 0.0 afterwards.
                if *code == 0x01 {
                    let idx = self.add_constant(VmValue::Float64(0.0));
                    self.emit_opcode(OpCode::Push);
                    self.emit_u16(idx);
                }
                self.store_val(val, ctx);
            }
            Instruction::Throw(v) => {
                self.load_val(*v, ctx);
                self.emit_opcode(OpCode::Throw);
            }
            Instruction::SetupTry(catch_block) => {
                self.emit_opcode(OpCode::TryStart);
                let offset = self.instructions.len();
                self.emit_u32(0); // target placeholder
                ctx.unresolved_tries.push((offset, *catch_block));
            }
            Instruction::PopTry => {
                self.emit_opcode(OpCode::TryEnd);
            }
            Instruction::PropertyAccess(obj, prop) => {
                self.load_val(*obj, ctx);
                let prop_idx = self.add_constant(VmValue::String(prop.clone()));
                self.emit_opcode(OpCode::Push);
                self.emit_u16(prop_idx);
                self.emit_opcode(OpCode::LoadElement);
                self.store_val(val, ctx);
            }
            Instruction::IndexAccess(arr, idx) => {
                self.load_val(*arr, ctx);
                self.load_val(*idx, ctx);
                self.emit_opcode(OpCode::LoadElement);
                self.store_val(val, ctx);
            }
            Instruction::AssignProperty(obj, prop, src_val) => {
                self.load_val(*obj, ctx);
                let prop_idx = self.add_constant(VmValue::String(prop.clone()));
                self.emit_opcode(OpCode::Push);
                self.emit_u16(prop_idx);
                self.load_val(*src_val, ctx);
                self.emit_opcode(OpCode::StoreElement);
            }
            Instruction::AssignIndex(obj, idx, src_val) => {
                self.load_val(*obj, ctx);
                self.load_val(*idx, ctx);
                self.load_val(*src_val, ctx);
                self.emit_opcode(OpCode::StoreElement);
            }
            Instruction::StructInit(name, fields) => {
                for (k, v_ref) in fields {
                    let k_idx = self.add_constant(VmValue::String(k.clone()));
                    self.emit_opcode(OpCode::Push);
                    self.emit_u16(k_idx);
                    self.load_val(*v_ref, ctx);
                }
                let count_idx = self.add_constant(VmValue::Float64(fields.len() as f64));
                self.emit_opcode(OpCode::Push);
                self.emit_u16(count_idx);
                self.emit_opcode(OpCode::NewStruct);
                self.store_val(val, ctx);
            }
            Instruction::ArrayLiteral(elems) => {
                for elem in elems {
                    self.load_val(*elem, ctx);
                }
                let count_idx = self.add_constant(VmValue::Float64(elems.len() as f64));
                self.emit_opcode(OpCode::Push);
                self.emit_u16(count_idx);
                self.emit_opcode(OpCode::NewArray);
                self.store_val(val, ctx);
            }
            _ => {
                // Not fully implemented for all yet.
            }
        }
        Ok(())
    }
    
    fn load_val(&mut self, val: Value, ctx: &mut FuncContext) {
        if let Some(&idx) = ctx.registers.get(&val) {
            self.emit_opcode(OpCode::LoadLocal);
            self.emit_u16(idx);
        }
    }
    
    fn store_val(&mut self, val: Value, ctx: &mut FuncContext) {
        let idx = ctx.local_count;
        ctx.registers.insert(val, idx);
        ctx.local_count += 1;
        self.emit_opcode(OpCode::StoreLocal);
        self.emit_u16(idx);
    }
    
    fn add_constant(&mut self, value: VmValue) -> u16 {
        for (i, c) in self.constants.iter().enumerate() {
            if c == &value {
                return i as u16;
            }
        }
        self.constants.push(value);
        (self.constants.len() - 1) as u16
    }
    
    fn emit_opcode(&mut self, op: OpCode) {
        self.instructions.push(op as u8);
    }
    
    fn emit_u8(&mut self, v: u8) {
        self.instructions.push(v);
    }
    
    fn emit_u16(&mut self, v: u16) {
        self.instructions.extend_from_slice(&v.to_be_bytes());
    }
    
    fn emit_u32(&mut self, v: u32) {
        self.instructions.extend_from_slice(&v.to_be_bytes());
    }
    
    fn patch_u32(&mut self, offset: usize, value: u32) {
        let bytes = value.to_be_bytes();
        self.instructions[offset] = bytes[0];
        self.instructions[offset + 1] = bytes[1];
        self.instructions[offset + 2] = bytes[2];
        self.instructions[offset + 3] = bytes[3];
    }
}
