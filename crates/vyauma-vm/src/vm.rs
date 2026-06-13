use crate::value::{Value, Function, NativeFunction};
use crate::heap::{Heap, ObjectType, StructInstanceData};
use crate::opcodes::OpCode;
use std::collections::HashMap;
use std::rc::Rc;

pub struct CallFrame {
    pub function: Rc<Function>,
    pub ip: usize,
    pub slots: usize,
}

impl CallFrame {
    pub fn read_byte(&mut self) -> u8 {
        let byte = self.function.chunk.code[self.ip];
        self.ip += 1;
        byte
    }

    pub fn read_constant(&mut self) -> Value {
        let idx = self.read_byte() as usize;
        self.function.chunk.constants[idx].clone()
    }
}

pub struct VM {
    pub stack: Vec<Value>,
    pub frames: Vec<CallFrame>,
    pub globals: HashMap<String, Value>,
    pub heap: Heap,
}

impl VM {
    pub fn new(heap: Heap) -> Self {
        VM {
            stack: Vec::new(),
            frames: Vec::new(),
            globals: HashMap::new(),
            heap,
        }
    }

    pub fn collect_garbage(&mut self) -> usize {
        self.heap.clear_marks();
        for val in &self.stack {
            self.heap.mark_value(val);
        }
        for val in self.globals.values() {
            self.heap.mark_value(val);
        }
        // Constants in functions on the stack
        for frame in &self.frames {
            for constant in &frame.function.chunk.constants {
                self.heap.mark_value(constant);
            }
        }
        self.heap.sweep()
    }

    pub fn define_native(&mut self, name: &str, arity: usize, func: fn(&mut Heap, &[Value]) -> Result<Value, String>) {
        let native = Value::NativeFunction(Rc::new(NativeFunction {
            name: name.to_string(),
            arity,
            func,
        }));
        self.globals.insert(name.to_string(), native);
    }

    pub fn register_module(&mut self, module: crate::stdlib::NativeModule) {
        let mut fields = HashMap::new();
        for (name, val) in module.functions {
            fields.insert(name, val);
        }
        
        let handle = self.heap.allocate(ObjectType::StructInstance(StructInstanceData {
            name: module.name.clone(),
            fields,
        }));
        
        self.globals.insert(module.name.clone(), Value::HeapRef(handle));
    }

    pub fn interpret(&mut self, function: Function) -> Result<Value, String> {
        let frame = CallFrame {
            function: Rc::new(function),
            ip: 0,
            slots: self.stack.len(),
        };
        self.frames.push(frame);

        self.run()
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn pop(&mut self) -> Value {
        self.stack.pop().expect("Stack underflow")
    }

    fn peek(&self, distance: usize) -> &Value {
        &self.stack[self.stack.len() - 1 - distance]
    }

    pub fn run(&mut self) -> Result<Value, String> {
        loop {
            if self.heap.allocated_objects >= self.heap.gc_threshold {
                self.collect_garbage();
            }

            let instruction: OpCode = {
                let frame = self.frames.last_mut().expect("No call frame");
                frame.read_byte().into()
            };

            match instruction {
                OpCode::PushConstant => {
                    let constant = {
                        let frame = self.frames.last_mut().unwrap();
                        frame.read_constant()
                    };
                    self.push(constant);
                }
                OpCode::PushTrue => self.push(Value::Bool(true)),
                OpCode::PushFalse => self.push(Value::Bool(false)),
                OpCode::PushNull => self.push(Value::Null),
                OpCode::Pop => {
                    self.pop();
                }
                OpCode::Dup => {
                    let top = self.peek(0).clone();
                    self.push(top);
                }
                OpCode::LoadLocal => {
                    let (slot, slots) = {
                        let frame = self.frames.last_mut().unwrap();
                        (frame.read_byte() as usize, frame.slots)
                    };
                    let value = self.stack[slots + slot].clone();
                    self.push(value);
                }
                OpCode::StoreLocal => {
                    let (slot, slots) = {
                        let frame = self.frames.last_mut().unwrap();
                        (frame.read_byte() as usize, frame.slots)
                    };
                    self.stack[slots + slot] = self.peek(0).clone();
                }
                OpCode::LoadGlobal => {
                    let name_val = {
                        let frame = self.frames.last_mut().unwrap();
                        frame.read_constant()
                    };
                    if let Value::HeapRef(handle) = name_val {
                        let obj = self.heap.get(handle);
                        if let ObjectType::String(name) = &obj.obj_type {
                            if let Some(val) = self.globals.get(name) {
                                let cloned_val = val.clone();
                                self.push(cloned_val);
                            } else {
                                return Err(format!("Undefined global '{}'", name));
                            }
                        } else {
                            return Err("Global name must be string".into());
                        }
                    }
                }
                OpCode::StoreGlobal => {
                    let name_val = {
                        let frame = self.frames.last_mut().unwrap();
                        frame.read_constant()
                    };
                    if let Value::HeapRef(handle) = name_val {
                        let obj = self.heap.get(handle);
                        if let ObjectType::String(name) = &obj.obj_type {
                            self.globals.insert(name.clone(), self.peek(0).clone());
                        }
                    }
                }
                OpCode::LoadField => {
                    let name_val = {
                        let frame = self.frames.last_mut().unwrap();
                        frame.read_constant()
                    };
                    if let Value::HeapRef(name_handle) = name_val {
                        let field_name = if let ObjectType::String(n) = &self.heap.get(name_handle).obj_type {
                            n.clone()
                        } else {
                            return Err("Field name must be string".into());
                        };

                        let instance_val = self.pop();
                        if let Value::HeapRef(inst_handle) = instance_val {
                            let obj = self.heap.get(inst_handle);
                            if let ObjectType::StructInstance(inst) = &obj.obj_type {
                                if let Some(field_val) = inst.fields.get(&field_name) {
                                    let cloned = field_val.clone();
                                    self.push(cloned);
                                } else {
                                    return Err(format!("Undefined property '{}'", field_name));
                                }
                            } else {
                                return Err("Only structs have fields".into());
                            }
                        } else {
                            return Err("Only structs have fields".into());
                        }
                    }
                }
                OpCode::StoreField => {
                    let name_val = {
                        let frame = self.frames.last_mut().unwrap();
                        frame.read_constant()
                    };
                    if let Value::HeapRef(name_handle) = name_val {
                        let field_name = if let ObjectType::String(n) = &self.heap.get(name_handle).obj_type {
                            n.clone()
                        } else {
                            return Err("Field name must be string".into());
                        };

                        let value = self.pop();
                        let instance_val = self.pop();
                        if let Value::HeapRef(inst_handle) = instance_val {
                            let obj = self.heap.get_mut(inst_handle);
                            if let ObjectType::StructInstance(inst) = &mut obj.obj_type {
                                inst.fields.insert(field_name, value.clone());
                                self.push(value);
                            } else {
                                return Err("Only structs have fields".into());
                            }
                        } else {
                            return Err("Only structs have fields".into());
                        }
                    }
                }
                OpCode::Call => {
                    let arg_count = {
                        let frame = self.frames.last_mut().unwrap();
                        frame.read_byte() as usize
                    };
                    let callee = self.peek(arg_count).clone();
                    
                    match callee {
                        Value::NativeFunction(native) => {
                            let mut args = Vec::new();
                            for _ in 0..arg_count {
                                args.push(self.pop());
                            }
                            args.reverse();
                            self.pop(); // pop native fn
                            let result = (native.func)(&mut self.heap, &args)?;
                            self.push(result);
                        }
                        Value::Function(func) => {
                            if arg_count != func.arity {
                                return Err(format!("Expected {} arguments but got {}", func.arity, arg_count));
                            }
                            let frame = CallFrame {
                                function: func.clone(),
                                ip: 0,
                                slots: self.stack.len() - arg_count - 1,
                            };
                            self.frames.push(frame);
                        }
                        Value::HeapRef(handle) => {
                            let obj = self.heap.get(handle);
                            if let ObjectType::String(struct_name) = &obj.obj_type {
                                let name_copy = struct_name.clone();
                                let mut fields = HashMap::new();
                                for _ in 0..(arg_count / 2) {
                                    let field_name_val = self.pop();
                                    let field_val = self.pop();
                                    if let Value::HeapRef(field_handle) = field_name_val {
                                        if let ObjectType::String(field_name) = &self.heap.get(field_handle).obj_type {
                                            fields.insert(field_name.clone(), field_val);
                                        }
                                    }
                                }
                                self.pop(); // pop the struct name Ref
                                
                                let inst_handle = self.heap.allocate(ObjectType::StructInstance(StructInstanceData {
                                    name: name_copy,
                                    fields,
                                }));
                                self.push(Value::HeapRef(inst_handle));
                            } else {
                                return Err("Can only call functions and structs".into());
                            }
                        }
                        _ => return Err("Can only call functions and structs".into()),
                    }
                }
                OpCode::Return => {
                    let result = self.pop();
                    let frame = self.frames.pop().unwrap();
                    if self.frames.is_empty() {
                        return Ok(result);
                    }
                    while self.stack.len() > frame.slots {
                        self.pop();
                    }
                    self.push(result);
                }
                OpCode::Add => {
                    let b = self.pop();
                    let a = self.pop();
                    match (a, b) {
                        (Value::Int(a), Value::Int(b)) => self.push(Value::Int(a + b)),
                        (Value::Float(a), Value::Float(b)) => self.push(Value::Float(a + b)),
                        _ => return Err("Operands must be two numbers".into()),
                    }
                }
                OpCode::Halt => {
                    return Ok(Value::Null);
                }
                _ => return Err("Unimplemented OpCode".into()),
            }
        }
    }
}
