use std::fs;
use vyauma_frontend::lexer::Lexer;
use vyauma_frontend::parser::Parser;

fn main() {
    let source = match fs::read_to_string("e:/Vyauma/taskflow-api/src/main.vym") {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to read main.vym: {}", e);
            return;
        }
    };

    println!("Lexing taskflow-api/main.vym...");
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();
    println!("Lexer succeeded.");

    println!("Parsing main.vym...");
    let mut parser = Parser::new(tokens);
    let ast = match parser.parse() {
        Ok(a) => a,
        Err(e) => {
            println!("Parser Failure: {:?}", e);
            return;
        }
    };
    println!("Parser succeeded.");
    println!("AST: {:#?}", ast);
}
