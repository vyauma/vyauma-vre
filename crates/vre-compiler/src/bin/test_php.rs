fn main() {
    let code = "<?php echo \"hello\"; ?>";
    match php_parser_rs::parse(code.as_bytes()) {
        Ok(ast) => println!("{:#?}", ast),
        Err(e) => println!("Error: {:?}", e),
    }
}
