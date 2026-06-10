import re

with open('crates/vre-compiler/src/type_checker.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Make check_program mutable
content = content.replace('pub fn check_program(&mut self, program: &Program)', 'pub fn check_program(&mut self, program: &mut Program)')
content = content.replace('fn check_function(&mut self, func: &Function)', 'fn check_function(&mut self, func: &mut Function)')
content = content.replace('fn check_block(&mut self, block: &Block', 'fn check_block(&mut self, block: &mut Block')
content = content.replace('fn check_statement(&mut self, stmt: &Stmt', 'fn check_statement(&mut self, stmt: &mut Stmt')
content = content.replace('fn get_expr_type(&mut self, expr: &Expr)', 'fn get_expr_type(&mut self, expr: &mut Expr)')

# In check_block:
content = content.replace('for stmt in block {', 'for stmt in block.iter_mut() {')

# In get_expr_type:
content = content.replace('Expr::BinaryOp(left, op, right)', 'Expr::BinaryOp(left, op, right, ref mut expr_type)')

assign_type_str = '''
                        match l_ty {
                            Type::Int32 | Type::Int64 | Type::Float32 | Type::Float64 => { *expr_type = Some(l_ty.clone()); Ok(l_ty) }
                            Type::String if *op == BinaryOperator::Add => { *expr_type = Some(Type::String); Ok(Type::String) }
                            _ => Err(TypeError::UnsupportedOperation(format!("Cannot perform math on {:?}", l_ty))),
                        }
'''
content = content.replace('''                        match l_ty {
                            Type::Int32 | Type::Int64 | Type::Float32 | Type::Float64 => Ok(l_ty),
                            Type::String if *op == BinaryOperator::Add => Ok(Type::String),
                            _ => Err(TypeError::UnsupportedOperation(format!("Cannot perform math on {:?}", l_ty))),
                        }''', assign_type_str)

bool_type_str = '''                    BinaryOperator::Equals | BinaryOperator::NotEquals | BinaryOperator::LessThan | BinaryOperator::GreaterThan | BinaryOperator::LessThanOrEq | BinaryOperator::GreaterThanOrEq => {
                        *expr_type = Some(l_ty);
                        Ok(Type::Bool)
                    }'''
content = content.replace('''                    BinaryOperator::Equals | BinaryOperator::NotEquals | BinaryOperator::LessThan | BinaryOperator::GreaterThan | BinaryOperator::LessThanOrEq | BinaryOperator::GreaterThanOrEq => {
                        Ok(Type::Bool)
                    }''', bool_type_str)

# In test:
content = content.replace('let program = parser.parse_program().unwrap();', 'let mut program = parser.parse_program().unwrap();')
content = content.replace('checker.check_program(&program)', 'checker.check_program(&mut program)')

with open('crates/vre-compiler/src/type_checker.rs', 'w', encoding='utf-8') as f:
    f.write(content)
