use std::fs;
use vyauma_frontend::lexer::Lexer;
use vyauma_frontend::parser::Parser;
use vyauma_vm::compiler::Compiler;

fn main() {
    println!("--- VWF Execution Harness ---");
    let source = fs::read_to_string("E:/Vyauma/myapp/src/main.vym").expect("Failed to read main.vym");
    
    let mut lexer = Lexer::new(&source);
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
