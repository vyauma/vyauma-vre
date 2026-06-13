use crate::ast::*;
use std::collections::{HashSet, HashMap};

pub struct Resolver {
    pub program: Program,
}

impl Resolver {
    pub fn new() -> Self {
        Resolver {
            program: Program { modules: HashMap::new() },
        }
    }

    pub fn resolve(&mut self, module_name: &str, ast: Vec<Statement>) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        let mut funcs = HashSet::new();
        let mut structs = HashSet::new();
        
        for stmt in &ast {
            match stmt {
                Statement::Function(f) => {
                    if !funcs.insert(f.name.clone()) {
                        errors.push(format!("Duplicate function '{}' in module '{}'", f.name, module_name));
                    }
                }
                Statement::Struct(s) => {
                    if !structs.insert(s.name.clone()) {
                        errors.push(format!("Duplicate struct '{}' in module '{}'", s.name, module_name));
                    }
                }
                Statement::Import(i) => {
                    // Just simulate checking for missing imports, we won't do full I/O mapping here 
                    // since we aren't hooking it up to a real filesystem yet in the tests.
                }
                _ => {}
            }
        }

        self.program.modules.insert(module_name.to_string(), Module {
            path: module_name.to_string(),
            ast,
        });

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
