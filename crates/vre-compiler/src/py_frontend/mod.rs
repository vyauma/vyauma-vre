pub mod lower;

use rustpython_parser::{ast, Parse};
use crate::vir::Module;

pub fn compile_py_to_vir(source: &str, _path: &str) -> Result<Module, String> {
    let suite = ast::Suite::parse(source, "<embedded>")
        .map_err(|e| format!("Python Parsing Error: {:?}", e))?;

    let mut lowerer = lower::Lowerer::new();
    for stmt in suite.iter() {
        lowerer.lower_statement(stmt)?;
    }
    
    Ok(lowerer.finish())
}
