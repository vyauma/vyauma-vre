use crate::chunk::Chunk;
use crate::opcodes::OpCode;
use crate::value::{Value, Function};
use std::rc::Rc;
use vyauma_frontend::ast::{Statement, Expression, LiteralValue};
use crate::heap::{Heap, ObjectType};

pub struct Compiler<'a> {
    pub chunk: Chunk,
    pub locals: Vec<String>,
    pub heap: &'a mut Heap,
}

impl<'a> Compiler<'a> {
    pub fn new(heap: &'a mut Heap) -> Self {
        Compiler {
            chunk: Chunk::new(),
            locals: Vec::new(),
            heap,
        }
    }

    pub fn compile_function(name: String, params: Vec<String>, body: Vec<Statement>, heap: &mut Heap) -> Function {
        let mut compiler = Compiler::new(heap);
        for param in &params {
            compiler.locals.push(param.clone());
        }
        for stmt in body {
            compiler.compile_statement(&stmt);
        }
        compiler.chunk.write_opcode(OpCode::PushNull);
        compiler.chunk.write_opcode(OpCode::Return);
        
        Function {
            name,
            arity: params.len(),
            chunk: compiler.chunk,
        }
    }

    pub fn compile_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Variable(var_decl) => {
                self.compile_expression(&var_decl.value);
                self.locals.push(var_decl.name.clone());
            }
            Statement::Expression(expr) => {
                self.compile_expression(expr);
                self.chunk.write_opcode(OpCode::Pop);
            }
            Statement::Return(ret_stmt) => {
                if let Some(expr) = &ret_stmt.value {
                    self.compile_expression(expr);
                } else {
                    self.chunk.write_opcode(OpCode::PushNull);
                }
                self.chunk.write_opcode(OpCode::Return);
            }
            _ => {}
        }
    }

    pub fn compile_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::Literal(lit) => match lit {
                LiteralValue::Integer(i) => {
                    let idx = self.chunk.add_constant(Value::Int(*i));
                    self.chunk.write_opcode(OpCode::PushConstant);
                    self.chunk.write(idx as u8);
                }
                LiteralValue::Float(f) => {
                    let idx = self.chunk.add_constant(Value::Float(*f));
                    self.chunk.write_opcode(OpCode::PushConstant);
                    self.chunk.write(idx as u8);
                }
                LiteralValue::String(s) => {
                    let handle = self.heap.allocate(ObjectType::String(s.clone()));
                    let idx = self.chunk.add_constant(Value::HeapRef(handle));
                    self.chunk.write_opcode(OpCode::PushConstant);
                    self.chunk.write(idx as u8);
                }
                LiteralValue::Boolean(b) => {
                    if *b {
                        self.chunk.write_opcode(OpCode::PushTrue);
                    } else {
                        self.chunk.write_opcode(OpCode::PushFalse);
                    }
                }
            },
            Expression::Identifier(name) => {
                if let Some(idx) = self.resolve_local(name) {
                    self.chunk.write_opcode(OpCode::LoadLocal);
                    self.chunk.write(idx as u8);
                } else {
                    let handle = self.heap.allocate(ObjectType::String(name.clone()));
                    let const_idx = self.chunk.add_constant(Value::HeapRef(handle));
                    self.chunk.write_opcode(OpCode::LoadGlobal);
                    self.chunk.write(const_idx as u8);
                }
            }
            Expression::MemberAccess { object, member } => {
                self.compile_expression(object);
                let handle = self.heap.allocate(ObjectType::String(member.clone()));
                let const_idx = self.chunk.add_constant(Value::HeapRef(handle));
                self.chunk.write_opcode(OpCode::LoadField);
                self.chunk.write(const_idx as u8);
            }
            Expression::Call { callee, args, named_args } => {
                self.compile_expression(callee); 
                for arg in args {
                    self.compile_expression(arg);
                }
                for (name, expr) in named_args {
                    self.compile_expression(expr);
                    let handle = self.heap.allocate(ObjectType::String(name.clone()));
                    let name_idx = self.chunk.add_constant(Value::HeapRef(handle));
                    self.chunk.write_opcode(OpCode::PushConstant);
                    self.chunk.write(name_idx as u8);
                }
                
                self.chunk.write_opcode(OpCode::Call);
                self.chunk.write((args.len() + (named_args.len() * 2)) as u8); 
            }
        }
    }

    fn resolve_local(&self, name: &str) -> Option<usize> {
        for (i, local) in self.locals.iter().enumerate().rev() {
            if local == name {
                return Some(i);
            }
        }
        None
    }
}
