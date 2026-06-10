use crate::ast::{Program, Stmt, Expr, BinaryOperator, Block, Function};

pub struct AstOptimizer;

impl AstOptimizer {
    pub fn new() -> Self {
        AstOptimizer
    }

    pub fn optimize(&mut self, program: &mut Program) {
        for func in &mut program.functions {
            self.optimize_function(func);
        }
        
        // Also optimize methods inside classes
        for class_decl in &mut program.classes {
            if let Stmt::ClassDecl(_, _, methods) = class_decl {
                for method in methods {
                    self.optimize_function(method);
                }
            }
        }
    }

    fn optimize_function(&mut self, func: &mut Function) {
        self.optimize_block(&mut func.body);
    }

    fn optimize_block(&mut self, block: &mut Block) {
        // First pass: optimize all statements
        for stmt in block.iter_mut() {
            self.optimize_statement(stmt);
        }

        // Second pass: flatten dead branches by splicing
        let mut optimized_block = Vec::new();
        for mut stmt in std::mem::take(block) {
            match stmt {
                Stmt::If(Expr::Number(n), mut cons, alt) => {
                    if n != 0.0 {
                        // Always true, keep cons, drop alt
                        optimized_block.append(&mut cons);
                    } else if let Some(mut alt_block) = alt {
                        // Always false, drop cons, keep alt
                        optimized_block.append(&mut alt_block);
                    }
                }
                Stmt::While(Expr::Number(n), _) if n == 0.0 => {
                    // Always false, drop the entire while loop
                }
                _ => {
                    optimized_block.push(stmt);
                }
            }
        }
        *block = optimized_block;
    }

    fn optimize_statement(&mut self, stmt: &mut Stmt) {
        match stmt {
            Stmt::Let(_, _, expr) => self.optimize_expression(expr),
            Stmt::Assign(_, expr) => self.optimize_expression(expr),
            Stmt::AssignIndex(_, index_expr, value_expr) => {
                self.optimize_expression(index_expr);
                self.optimize_expression(value_expr);
            }
            Stmt::AssignProperty(obj_expr, _, value_expr) => {
                self.optimize_expression(obj_expr);
                self.optimize_expression(value_expr);
            }
            Stmt::If(cond, cons, alt) => {
                self.optimize_expression(cond);
                self.optimize_block(cons);
                if let Some(alt_block) = alt {
                    self.optimize_block(alt_block);
                }
            }
            Stmt::While(cond, body) => {
                self.optimize_expression(cond);
                self.optimize_block(body);
            }
            Stmt::For(init, cond, inc, body) => {
                self.optimize_statement(init);
                self.optimize_expression(cond);
                self.optimize_statement(inc);
                self.optimize_block(body);
            }
            Stmt::Return(opt_expr) => {
                if let Some(expr) = opt_expr {
                    self.optimize_expression(expr);
                }
            }
            Stmt::Throw(expr) => self.optimize_expression(expr),
            Stmt::TryCatch(try_block, _, catch_block) => {
                self.optimize_block(try_block);
                self.optimize_block(catch_block);
            }
            Stmt::Expr(expr) => self.optimize_expression(expr),
            Stmt::StructDecl(_, _) | Stmt::ClassDecl(_, _, _) => {}
        }
    }

    fn optimize_expression(&mut self, expr: &mut Expr) {
        match expr {
            Expr::BinaryOp(left, op, right, _) => {
                self.optimize_expression(left);
                self.optimize_expression(right);

                // Constant folding
                if let (Expr::Number(l), Expr::Number(r)) = (&**left, &**right) {
                    let folded = match op {
                        BinaryOperator::Add => Some(l + r),
                        BinaryOperator::Subtract => Some(l - r),
                        BinaryOperator::Multiply => Some(l * r),
                        BinaryOperator::Divide => {
                            if *r != 0.0 { Some(l / r) } else { None }
                        }
                        BinaryOperator::Equals => Some(if l == r { 1.0 } else { 0.0 }),
                        BinaryOperator::NotEquals => Some(if l != r { 1.0 } else { 0.0 }),
                        BinaryOperator::LessThan => Some(if l < r { 1.0 } else { 0.0 }),
                        BinaryOperator::LessThanOrEq => Some(if l <= r { 1.0 } else { 0.0 }),
                        BinaryOperator::GreaterThan => Some(if l > r { 1.0 } else { 0.0 }),
                        BinaryOperator::GreaterThanOrEq => Some(if l >= r { 1.0 } else { 0.0 }),
                        BinaryOperator::And => Some(if *l != 0.0 && *r != 0.0 { 1.0 } else { 0.0 }),
                        BinaryOperator::Or => Some(if *l != 0.0 || *r != 0.0 { 1.0 } else { 0.0 }),
                    };

                    if let Some(val) = folded {
                        *expr = Expr::Number(val);
                    }
                }
            }
            Expr::ArrayLiteral(elements) => {
                for elem in elements {
                    self.optimize_expression(elem);
                }
            }
            Expr::DictLiteral(elements) => {
                for (k, v) in elements {
                    self.optimize_expression(k);
                    self.optimize_expression(v);
                }
            }
            Expr::IndexAccess(arr, idx) => {
                self.optimize_expression(arr);
                self.optimize_expression(idx);
            }
            Expr::StructInit(_, fields) => {
                for (_, val) in fields {
                    self.optimize_expression(val);
                }
            }
            Expr::PropertyAccess(obj, _, _) => {
                self.optimize_expression(obj);
            }
            Expr::MethodCall(obj, _, args, _) => {
                self.optimize_expression(obj);
                for arg in args {
                    self.optimize_expression(arg);
                }
            }
            Expr::NewClass(_, args) => {
                for arg in args {
                    self.optimize_expression(arg);
                }
            }
            Expr::Call(_, args, _) => {
                for arg in args {
                    self.optimize_expression(arg);
                }
            }
            Expr::Identifier(_, _) | Expr::Number(_) | Expr::StringLiteral(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Type;

    #[test]
    fn test_constant_folding() {
        let mut optimizer = AstOptimizer::new();
        // 1 + 2 * 3
        let mut expr = Expr::BinaryOp(
            Box::new(Expr::Number(1.0)),
            BinaryOperator::Add,
            Box::new(Expr::BinaryOp(
                Box::new(Expr::Number(2.0)),
                BinaryOperator::Multiply,
                Box::new(Expr::Number(3.0)),
                None,
            )),
            None,
        );

        optimizer.optimize_expression(&mut expr);

        assert_eq!(expr, Expr::Number(7.0));
    }

    #[test]
    fn test_dead_code_elimination() {
        let mut optimizer = AstOptimizer::new();
        // if (0) { a = 1; } else { a = 2; }
        let mut block = vec![Stmt::If(
            Expr::Number(0.0),
            vec![Stmt::Assign("a".to_string(), Expr::Number(1.0))],
            Some(vec![Stmt::Assign("a".to_string(), Expr::Number(2.0))]),
        )];

        optimizer.optimize_block(&mut block);

        assert_eq!(block.len(), 1);
        assert_eq!(block[0], Stmt::Assign("a".to_string(), Expr::Number(2.0)));
    }
}
