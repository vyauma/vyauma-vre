import re

# Patch lib.rs
with open('crates/vre-compiler/src/lib.rs', 'r', encoding='utf-8') as f:
    content = f.read()

content = content.replace('let program = parse_and_resolve(source, base_path, &mut visited)?;', 'let mut program = parse_and_resolve(source, base_path, &mut visited)?;')
content = content.replace('checker.check_program(&program)', 'checker.check_program(&mut program)')

with open('crates/vre-compiler/src/lib.rs', 'w', encoding='utf-8') as f:
    f.write(content)

# Patch compiler.rs
with open('crates/vre-compiler/src/compiler.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Make sure we import Type
if 'use crate::ast::Type;' not in content:
    content = content.replace('use crate::ast::{Program, Function, Stmt, Expr, BinaryOperator};', 'use crate::ast::{Program, Function, Stmt, Expr, BinaryOperator, Type};')

binary_op_pattern = r'Expr::BinaryOp\(left, op, right\) => \{.*?\n\s+\}'

binary_op_replacement = '''Expr::BinaryOp(left, op, right, expr_type) => {
                self.compile_expression(*left)?;
                self.compile_expression(*right)?;
                let ty = expr_type.unwrap_or(Type::Float64);
                match op {
                    BinaryOperator::Add => match ty {
                        Type::Int32 => self.emit_opcode(OpCode::AddI32),
                        Type::Int64 => self.emit_opcode(OpCode::AddI64),
                        Type::Float32 => self.emit_opcode(OpCode::AddF32),
                        _ => self.emit_opcode(OpCode::AddF64),
                    },
                    BinaryOperator::Subtract => match ty {
                        Type::Int32 => self.emit_opcode(OpCode::SubI32),
                        Type::Int64 => self.emit_opcode(OpCode::SubI64),
                        Type::Float32 => self.emit_opcode(OpCode::SubF32),
                        _ => self.emit_opcode(OpCode::SubF64),
                    },
                    BinaryOperator::Multiply => match ty {
                        Type::Int32 => self.emit_opcode(OpCode::MulI32),
                        Type::Int64 => self.emit_opcode(OpCode::MulI64),
                        Type::Float32 => self.emit_opcode(OpCode::MulF32),
                        _ => self.emit_opcode(OpCode::MulF64),
                    },
                    BinaryOperator::Divide => match ty {
                        Type::Int32 => self.emit_opcode(OpCode::DivI32),
                        Type::Int64 => self.emit_opcode(OpCode::DivI64),
                        Type::Float32 => self.emit_opcode(OpCode::DivF32),
                        _ => self.emit_opcode(OpCode::DivF64),
                    },
                    BinaryOperator::Equals => match ty {
                        Type::Int32 => self.emit_opcode(OpCode::EqualI32),
                        Type::Int64 => self.emit_opcode(OpCode::EqualI64),
                        Type::Float32 => self.emit_opcode(OpCode::EqualF32),
                        Type::String => self.emit_opcode(OpCode::EqualStr),
                        _ => self.emit_opcode(OpCode::EqualF64),
                    },
                    BinaryOperator::NotEquals => match ty {
                        Type::Int32 => self.emit_opcode(OpCode::NotEqualI32),
                        Type::Int64 => self.emit_opcode(OpCode::NotEqualI64),
                        Type::Float32 => self.emit_opcode(OpCode::NotEqualF32),
                        Type::String => self.emit_opcode(OpCode::NotEqualStr),
                        _ => self.emit_opcode(OpCode::NotEqualF64),
                    },
                    BinaryOperator::LessThan => match ty {
                        Type::Int32 => self.emit_opcode(OpCode::LessI32),
                        Type::Int64 => self.emit_opcode(OpCode::LessI64),
                        Type::Float32 => self.emit_opcode(OpCode::LessF32),
                        _ => self.emit_opcode(OpCode::LessF64),
                    },
                    BinaryOperator::LessThanOrEq => match ty {
                        Type::Int32 => self.emit_opcode(OpCode::LessEqualI32),
                        Type::Int64 => self.emit_opcode(OpCode::LessEqualI64),
                        Type::Float32 => self.emit_opcode(OpCode::LessEqualF32),
                        _ => self.emit_opcode(OpCode::LessEqualF64),
                    },
                    BinaryOperator::GreaterThan => match ty {
                        Type::Int32 => self.emit_opcode(OpCode::GreaterI32),
                        Type::Int64 => self.emit_opcode(OpCode::GreaterI64),
                        Type::Float32 => self.emit_opcode(OpCode::GreaterF32),
                        _ => self.emit_opcode(OpCode::GreaterF64),
                    },
                    BinaryOperator::GreaterThanOrEq => match ty {
                        Type::Int32 => self.emit_opcode(OpCode::GreaterEqualI32),
                        Type::Int64 => self.emit_opcode(OpCode::GreaterEqualI64),
                        Type::Float32 => self.emit_opcode(OpCode::GreaterEqualF32),
                        _ => self.emit_opcode(OpCode::GreaterEqualF64),
                    },
                }
            }'''

content = re.sub(binary_op_pattern, binary_op_replacement, content, flags=re.DOTALL)

with open('crates/vre-compiler/src/compiler.rs', 'w', encoding='utf-8') as f:
    f.write(content)
