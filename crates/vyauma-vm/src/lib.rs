pub mod opcodes;
pub mod value;
pub mod heap;
pub mod chunk;
pub mod compiler;
pub mod vm;
pub mod stdlib;

#[cfg(test)]
mod tests {
    use super::*;
    use value::{Value, Function};
    use heap::{Heap, ObjectType};
    use opcodes::OpCode;
    use std::rc::Rc;
    use vyauma_frontend::ast::{Statement, Expression, LiteralValue, VariableDecl};

    #[test]
    fn test_math_execution() {
        let ast = vec![
            Statement::Variable(VariableDecl {
                name: "x".to_string(),
                value: Expression::Literal(LiteralValue::Integer(5)),
            }),
            Statement::Variable(VariableDecl {
                name: "y".to_string(),
                value: Expression::Literal(LiteralValue::Integer(10)),
            }),
            Statement::Return(vyauma_frontend::ast::ReturnStmt {
                value: Some(Expression::Call {
                    callee: Box::new(Expression::Identifier("native_add".to_string())),
                    args: vec![
                        Expression::Identifier("x".to_string()),
                        Expression::Identifier("y".to_string()),
                    ],
                    named_args: std::collections::HashMap::new(),
                })
            })
        ];

        let mut heap = Heap::new();
        let main_fn = compiler::Compiler::compile_function("main".to_string(), vec![], ast, &mut heap);
        
        let mut vm = vm::VM::new(heap);
        vm.define_native("native_add", 2, |_heap, args| {
            if let (Value::Int(a), Value::Int(b)) = (&args[0], &args[1]) {
                Ok(Value::Int(a + b))
            } else {
                Ok(Value::Null)
            }
        });

        let result = vm.interpret(main_fn).unwrap();
        assert_eq!(result, Value::Int(15));
    }

    #[test]
    fn test_hello_world() {
        let ast = vec![
            Statement::Return(vyauma_frontend::ast::ReturnStmt {
                value: Some(Expression::Call {
                    callee: Box::new(Expression::Identifier("print".to_string())),
                    args: vec![Expression::Literal(LiteralValue::String("Hello World".to_string()))],
                    named_args: std::collections::HashMap::new(),
                })
            })
        ];

        let mut heap = Heap::new();
        let main_fn = compiler::Compiler::compile_function("main".to_string(), vec![], ast, &mut heap);
        
        let mut vm = vm::VM::new(heap);
        vm.define_native("print", 1, |_heap, args| {
            Ok(args[0].clone())
        });

        let result = vm.interpret(main_fn).unwrap();
        
        // Match the string inside the heap
        if let Value::HeapRef(handle) = result {
            if let ObjectType::String(s) = &vm.heap.get(handle).obj_type {
                assert_eq!(s, "Hello World");
                return;
            }
        }
        panic!("Did not return the correct heap string");
    }

    #[test]
    fn test_gc_stress_allocation() {
        let mut heap = Heap::new();
        // Set threshold extremely low to force constant sweeping
        heap.gc_threshold = 10; 
        
        let mut vm = vm::VM::new(heap);
        
        // Allocate 10,000 strings into local stack slots
        for i in 0..10_000 {
            // Allocate string
            let handle = vm.heap.allocate(ObjectType::String(format!("String {}", i)));
            vm.stack.push(Value::HeapRef(handle)); // Push to stack (Root)
            
            // Pop the old one to orphan it and reclaim memory
            if i > 0 {
                vm.stack.remove(0); 
            }
            
            if vm.heap.allocated_objects > vm.heap.gc_threshold {
                let swept = vm.collect_garbage();
                assert!(swept > 0);
            }
        }

        // Run one final GC to clean up any remaining objects below the 1000 threshold
        vm.collect_garbage();

        // The heap should only have 1 active string left, though free_slots will be high
        assert!(vm.heap.allocated_objects < 20); 
    }

    #[test]
    fn test_stdlib_json_and_fs() {
        let ast = vec![
            Statement::Variable(VariableDecl {
                name: "data".to_string(),
                value: Expression::Call {
                    callee: Box::new(Expression::MemberAccess {
                        object: Box::new(Expression::Identifier("json".to_string())),
                        member: "parse".to_string(),
                    }),
                    args: vec![Expression::Literal(LiteralValue::String("{\"name\": \"Manvirr\", \"age\": 30}".to_string()))],
                    named_args: std::collections::HashMap::new(),
                }
            }),
            Statement::Return(vyauma_frontend::ast::ReturnStmt {
                value: Some(Expression::Call {
                    callee: Box::new(Expression::MemberAccess {
                        object: Box::new(Expression::Identifier("json".to_string())),
                        member: "stringify".to_string(),
                    }),
                    args: vec![Expression::Identifier("data".to_string())],
                    named_args: std::collections::HashMap::new(),
                })
            })
        ];

        let mut heap = Heap::new();
        let main_fn = compiler::Compiler::compile_function("main".to_string(), vec![], ast, &mut heap);
        
        let mut vm = vm::VM::new(heap);
        crate::stdlib::register_all(&mut vm);

        let result = vm.interpret(main_fn).unwrap();
        
        if let Value::HeapRef(handle) = result {
            if let ObjectType::String(s) = &vm.heap.get(handle).obj_type {
                // The pretty stringified output
                assert!(s.contains("\"name\": \"Manvirr\""));
                assert!(s.contains("\"age\": 30"));
                return;
            }
        }
        panic!("Failed to stringify JSON");
    }
}
