pub mod lexer;
pub mod ast;
pub mod parser;
pub mod resolver;

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_code(code: &str) -> Result<Vec<ast::Statement>, String> {
        let mut lexer = lexer::Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = parser::Parser::new(tokens);
        parser.parse()
    }

    #[test]
    fn test_hello_world() {
        let code = r#"
            fn main() {
                print("Hello, World");
            }
        "#;
        let ast = parse_code(code).unwrap();
        assert_eq!(ast.len(), 1);
    }

    #[test]
    fn test_structs_and_functions() {
        let code = r#"
            struct User {
                name: string,
                age: int
            }

            fn greet(name: string) {
                print(name);
            }

            fn main() {
                let user = User(
                    name="Manvirr",
                    age=30
                );
                greet(user.name);
            }
        "#;
        
        let ast = parse_code(code).unwrap();
        assert_eq!(ast.len(), 3); // struct User, fn greet, fn main
    }

    #[test]
    fn test_imports() {
        let code = r#"
            import math;
            import utils.validation;
        "#;
        let ast = parse_code(code).unwrap();
        assert_eq!(ast.len(), 2);
    }

    #[test]
    fn test_resolver_duplicate_validation() {
        let code = r#"
            fn test() {}
            fn test() {}
        "#;
        let ast = parse_code(code).unwrap();
        let mut resolver = resolver::Resolver::new();
        let res = resolver.resolve("main", ast);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err()[0], "Duplicate function 'test' in module 'main'");
    }
}
