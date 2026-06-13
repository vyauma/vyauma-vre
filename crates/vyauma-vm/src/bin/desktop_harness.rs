use std::fs;
use vyauma_frontend::lexer::Lexer;
use vyauma_frontend::parser::Parser;
use vyauma_vm::compiler::Compiler;

fn main() {
    println!("--- Desktop Framework Execution Harness ---");
    let source = match fs::read_to_string("E:/Vyauma/desktop_app/src/main.vym") {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to read main.vym: {}", e);
            return;
        }
    };
    
    let lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();
    
    let mut parser = Parser::new(tokens);
    let ast = match parser.parse() {
        Ok(a) => a,
        Err(e) => {
            println!("Parser Error: {}", e);
            return;
        }
    };
    
    println!("AST Generated Successfully: {:#?}", ast);
}
