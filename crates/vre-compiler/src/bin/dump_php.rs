use php_parser_rs::parser;

fn main() {
    let source = "<?php require 'module_name'; export function foo() {} ?>";
    let ast = parser::parse(source.as_bytes()).unwrap();
    println!("{:#?}", ast);
}
