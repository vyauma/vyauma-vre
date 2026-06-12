use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

fn main() {
    let source = r#"
        import "module_name";
        import { foo } from "other_module";
        export function baz() {}
        export { foo };
    "#;
    let allocator = Allocator::default();
    let source_type = SourceType::default().with_module(true).with_typescript(true);
    let ret = Parser::new(&allocator, source, source_type).parse();
    
    println!("{:#?}", ret.program);
}
