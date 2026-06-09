pub mod lexer;
pub mod ast;
pub mod parser;
pub mod compiler;

use crate::ast::Program;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::compiler::Compiler;
use std::path::{Path, PathBuf};
use std::collections::HashSet;

pub use crate::compiler::CompiledProgram;

pub fn compile(source: &str, base_path: Option<&Path>) -> Result<CompiledProgram, String> {
    let mut visited = HashSet::new();
    let program = parse_and_resolve(source, base_path, &mut visited)?;
    
    let compiler = Compiler::new();
    let compiled = compiler.compile(program)?;
    
    Ok(compiled)
}

fn parse_and_resolve(
    source: &str, 
    base_path: Option<&Path>, 
    visited: &mut HashSet<PathBuf>
) -> Result<Program, String> {
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program()?;

    let mut merged_functions = program.functions.clone();
    let mut merged_structs = program.structs.clone();

    for import_path_str in &program.imports {
        if let Some(base) = base_path {
            let import_path = base.join(import_path_str);
            let canonical = match std::fs::canonicalize(&import_path) {
                Ok(p) => p,
                Err(_) => import_path.clone(),
            };

            if visited.contains(&canonical) {
                continue;
            }
            visited.insert(canonical.clone());

            let imported_source = std::fs::read_to_string(&import_path)
                .map_err(|e| format!("Failed to read imported file {}: {}", import_path.display(), e))?;

            let new_base = import_path.parent();
            let imported_program = parse_and_resolve(&imported_source, new_base, visited)?;

            merged_functions.extend(imported_program.functions);
            merged_structs.extend(imported_program.structs);
        } else {
            return Err(format!("Cannot resolve import '{}' without a base path", import_path_str));
        }
    }

    Ok(Program {
        imports: vec![],
        functions: merged_functions,
        structs: merged_structs,
    })
}
