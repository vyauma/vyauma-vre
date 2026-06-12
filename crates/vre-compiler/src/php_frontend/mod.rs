pub mod lower;

use crate::vir::Module;
use php_parser_rs::parse;

pub fn compile_php_to_vir(source: &str, _path: &str) -> Result<Module, String> {
    let ast = parse(source.as_bytes()).map_err(|e| format!("PHP Parse Error: {:?}", e))?;
    
    let lowerer = lower::Lowerer::new();
    lowerer.lower_program(&ast)
}
