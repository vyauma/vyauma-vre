use rustpython_parser::parse;

fn main() {
    let source = "import os\nfrom math import sqrt\ndef foo(): pass\n";
    let ast = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<string>").unwrap();
    println!("{:#?}", ast);
}
