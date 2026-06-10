#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    Boolean(bool),
    Identifier(String, Option<Type>),
    StringLiteral(String),
    BinaryOp(Box<Expr>, BinaryOperator, Box<Expr>, Option<Type>),
    Call(String, Vec<Expr>, Option<Type>),
    ArrayLiteral(Vec<Expr>),
    IndexAccess(Box<Expr>, Box<Expr>),
    StructInit(String, Vec<(String, Expr)>),
    PropertyAccess(Box<Expr>, String, Option<Type>),
    DictLiteral(Vec<(Expr, Expr)>),
    NewClass(String, Vec<Expr>),
    MethodCall(Box<Expr>, String, Vec<Expr>, Option<Type>),
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
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int32,
    Int64,
    Float32,
    Float64,
    Bool,
    String,
    Any,
    Struct(String),
    Array(Box<Type>),
    Dict(Box<Type>, Box<Type>),
    Class(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let(String, Option<Type>, Expr),
    Assign(String, Expr),
    AssignIndex(String, Expr, Expr),
    Expr(Expr),
    If(Expr, Block, Option<Block>),
    While(Expr, Block),
    Return(Option<Expr>),
    StructDecl(String, Vec<(String, Option<Type>)>, bool),
    AssignProperty(Box<Expr>, String, Expr),
    TryCatch(Block, String, Block),
    Throw(Expr),
    For(Box<Stmt>, Expr, Box<Stmt>, Block),
    ClassDecl(String, Vec<(String, Option<Type>)>, Vec<Function>, bool),
}

pub type Block = Vec<Stmt>;

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<(String, Option<Type>)>,
    pub return_type: Option<Type>,
    pub body: Block,
    pub is_exported: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportDecl {
    /// Relative file path, e.g. "utils" or "math/vec"
    pub path: String,
    /// Optional alias; if None, the stem of `path` is used as the namespace
    pub alias: Option<String>,
}

impl ImportDecl {
    /// Returns the namespace string used for name-mangling (e.g. "vec" for "math/vec").
    pub fn namespace(&self) -> String {
        if let Some(alias) = &self.alias {
            alias.clone()
        } else {
            // Take the last path component, strip extension
            let stem = self.path
                .split('/')
                .last()
                .unwrap_or(&self.path);
            let stem = stem.trim_end_matches(".vya").trim_end_matches(".vym");
            stem.to_string()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub imports: Vec<ImportDecl>,
    pub functions: Vec<Function>,
    pub structs: Vec<Stmt>,
    pub classes: Vec<Stmt>,
}
