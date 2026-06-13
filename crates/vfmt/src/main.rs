use std::env;
use std::fs;
use vre_compiler::lexer::Lexer;
use vre_compiler::parser::Parser;

mod formatter;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: vfmt <file.vya> [--check]");
        std::process::exit(1);
    }

    let filepath = &args[1];
    let check_only = args.get(2).map(|s| s == "--check").unwrap_or(false);

    let source = match fs::read_to_string(filepath) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file {}: {}", filepath, e);
            std::process::exit(1);
        }
    };

    let lexer = Lexer::new(&source);
    let mut parser = Parser::new(lexer);
    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error parsing {}: {:?}", filepath, e);
            std::process::exit(1);
        }
    };

    let formatted = formatter::format_program(&program);

    if check_only {
        if formatted != source {
            println!("File {} is not formatted correctly.", filepath);
            std::process::exit(1);
        } else {
            println!("File {} is formatted correctly.", filepath);
        }
    } else {
        if formatted != source {
            if let Err(e) = fs::write(filepath, formatted) {
                eprintln!("Error writing formatted file {}: {}", filepath, e);
                std::process::exit(1);
            }
            println!("Formatted {}", filepath);
        } else {
            println!("{} is already formatted.", filepath);
        }
    }
}
