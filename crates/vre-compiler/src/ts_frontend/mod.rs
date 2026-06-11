pub mod lower;
pub mod tests;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

pub fn parse_ts<'a>(source_text: &'a str, file_name: &'a str) -> Result<oxc_ast::ast::Program<'a>, String> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(file_name).unwrap_or_default().with_typescript(true);
    let ret = Parser::new(&allocator, source_text, source_type).parse();
    if !ret.errors.is_empty() {
        return Err(format!("Parse errors: {:?}", ret.errors));
    }
    unimplemented!()
}

pub fn compile_ts_to_vir(source_text: &str, file_name: &str) -> Result<crate::vir::Module, String> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(file_name).unwrap_or_default().with_typescript(true);
    let ret = Parser::new(&allocator, source_text, source_type).parse();
    
    if !ret.errors.is_empty() {
        return Err(format!("Parse errors: {:?}", ret.errors));
    }

    let mut lowerer = lower::Lowerer::new();
    lowerer.lower_program(&ret.program)
}
