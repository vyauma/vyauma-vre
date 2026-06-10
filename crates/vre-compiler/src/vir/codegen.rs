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
            native_imports: Vec::new(),
        }
    }
    
    pub fn generate(mut self, module: Module) -> Result<CompiledProgram, String> {
        self.emit_opcode(OpCode::Call);
        let main_call_target_offset = self.instructions.len();
        self.emit_u32(0);
        self.emit_u16(256);
        self.emit_opcode(OpCode::Halt);
        
        for func in module.functions {
            self.compile_function(func)?;
        }
        
        if let Some(&main_addr) = self.functions.get("main") {
            self.patch_u32(main_call_target_offset, main_addr);
        } else {
            return Err("No main function found".to_string());
        }
        
        for (offset, name, arg_count) in self.unresolved_calls {
            if let Some(&addr) = self.functions.get(&name) {
                let mut bytes = addr.to_le_bytes().to_vec();
                bytes.extend_from_slice(&(256u16).to_le_bytes()); // Hack: local count
                for i in 0..6 {
                    self.instructions[offset + i] = bytes[i];
                }
            } else {
                let mut native_idx = self.native_imports.iter().position(|n| n == &name);
                if native_idx.is_none() {
                    self.native_imports.push(name.clone());
                    native_idx = Some(self.native_imports.len() - 1);
                }
                
                let import_idx = native_idx.unwrap() as u16;
                let original_opcode_offset = offset - 1;
                self.instructions[original_opcode_offset] = OpCode::CallNative as u8;
                
                let bytes = import_idx.to_le_bytes();
                self.instructions[offset] = bytes[0];
                self.instructions[offset + 1] = bytes[1];
                self.instructions[offset + 2] = arg_count;
                // Next 3 bytes are unused (padded with NOP)
                self.instructions[offset + 3] = OpCode::Nop as u8;
                self.instructions[offset + 4] = OpCode::Nop as u8;
                self.instructions[offset + 5] = OpCode::Nop as u8;
            }
        }
        
        Ok(CompiledProgram {
            instructions: self.instructions,
            constants: self.constants,
            native_imports: self.native_imports,
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
        for param in func.params {
            ctx.locals.insert(param, ctx.local_count);
            ctx.local_count += 1;
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
            self.patch_u32(offset + 4, alt_addr);
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
    
    fn emit_u16(&mut self, v: u16) {
        self.instructions.extend_from_slice(&v.to_le_bytes());
    }
    
    fn emit_u32(&mut self, v: u32) {
        self.instructions.extend_from_slice(&v.to_le_bytes());
    }
    
    fn patch_u32(&mut self, offset: usize, value: u32) {
        let bytes = value.to_le_bytes();
        self.instructions[offset] = bytes[0];
        self.instructions[offset + 1] = bytes[1];
        self.instructions[offset + 2] = bytes[2];
        self.instructions[offset + 3] = bytes[3];
    }
}
