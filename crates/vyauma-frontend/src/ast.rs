use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub modules: HashMap<String, Module>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub path: String,
    pub ast: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Import(ImportStmt),
    Struct(StructDecl),
    Function(FunctionDecl),
    Variable(VariableDecl),
    Return(ReturnStmt),
    Expression(Expression),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportStmt {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<Parameter>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<String>,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub param_type: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariableDecl {
    pub name: String,
    pub value: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStmt {
    pub value: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(LiteralValue),
    Identifier(String),
    Call {
        callee: Box<Expression>,
        args: Vec<Expression>,
        named_args: HashMap<String, Expression>,
    },
    MemberAccess {
        object: Box<Expression>,
        member: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}
