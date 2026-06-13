use std::fs;
use vyauma_frontend::lexer::Lexer;
use vyauma_frontend::parser::Parser;
use vyauma_vm::compiler::Compiler;

fn main() {
    println!("--- WebAssembly Compilation Harness ---");
    let source = match fs::read_to_string("E:/Vyauma/wasm_app/src/main.vym") {
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
    
    println!("AST Generated Successfully.");
    println!("Attempting Lowering (AST -> IR) for LLVM/WASM target...");
    
    panic!("Compiler Architect Gap: Missing Vyauma Intermediate Representation (IR). Cannot target WASM without an intermediate optimization layer.");
}
