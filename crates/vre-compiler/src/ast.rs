#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    Identifier(String),
    StringLiteral(String),
    BinaryOp(Box<Expr>, BinaryOperator, Box<Expr>),
    Call(String, Vec<Expr>),
    ArrayLiteral(Vec<Expr>),
    IndexAccess(Box<Expr>, Box<Expr>),
    StructInit(String, Vec<(String, Expr)>),
    PropertyAccess(Box<Expr>, String),
    DictLiteral(Vec<(Expr, Expr)>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equals,
    NotEquals,
    LessThan,
    GreaterThan,
    LessThanOrEq,
    GreaterThanOrEq,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let(String, Expr),
    Assign(String, Expr),
    AssignIndex(String, Expr, Expr),
    Expr(Expr),
    If(Expr, Block, Option<Block>),
    While(Expr, Block),
    Return(Option<Expr>),
    StructDecl(String, Vec<String>),
    AssignProperty(Box<Expr>, String, Expr),
    TryCatch(Block, String, Block),
    Throw(Expr),
}

pub type Block = Vec<Stmt>;

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<String>,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub imports: Vec<String>,
    pub functions: Vec<Function>,
    pub structs: Vec<Stmt>,
}
