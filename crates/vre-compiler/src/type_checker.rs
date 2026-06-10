use std::collections::{HashMap, HashSet};
use crate::ast::{Program, Stmt, Expr, Function, Type, BinaryOperator, Block};

#[derive(Debug, Clone, PartialEq)]
pub enum TypeError {
    UndefinedVariable(String),
    TypeMismatch { expected: String, found: String },
    UnsupportedOperation(String),
    UndefinedFunction(String),
    InvalidArguments(String),
    UndefinedStruct(String),
    UndefinedClass(String),
    UnknownProperty(String),
    MissingField(String),
    ExtraField(String),
    InvalidIndexType(String),
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeError::UndefinedVariable(name) => write!(f, "Undefined variable: {}", name),
            TypeError::TypeMismatch { expected, found } => write!(f, "Type mismatch: expected {}, found {}", expected, found),
            TypeError::UnsupportedOperation(op) => write!(f, "Unsupported operation: {}", op),
            TypeError::UndefinedFunction(name) => write!(f, "Undefined function: {}", name),
            TypeError::InvalidArguments(msg) => write!(f, "Invalid arguments: {}", msg),
            TypeError::UndefinedStruct(name) => write!(f, "Undefined struct: {}", name),
            TypeError::UndefinedClass(name) => write!(f, "Undefined class: {}", name),
            TypeError::UnknownProperty(name) => write!(f, "Unknown property: {}", name),
            TypeError::MissingField(name) => write!(f, "Missing field: {}", name),
            TypeError::ExtraField(name) => write!(f, "Extra field: {}", name),
            TypeError::InvalidIndexType(msg) => write!(f, "Invalid index type: {}", msg),
        }
    }
}

pub struct TypeEnvironment {
    scopes: Vec<HashMap<String, Type>>,
    functions: HashMap<String, FunctionSignature>,
    pub structs: HashMap<String, HashMap<String, Type>>,
    pub classes: HashMap<String, ClassSignature>,
}

#[derive(Clone)]
pub struct ClassSignature {
    pub fields: HashMap<String, Type>,
    pub field_order: Vec<String>,
    pub methods: HashMap<String, FunctionSignature>,
}

#[derive(Clone)]
struct FunctionSignature {
    params: Vec<Type>,
    return_type: Type,
}

impl TypeEnvironment {
    pub fn new() -> Self {
        TypeEnvironment {
            scopes: vec![HashMap::new()],
            functions: HashMap::new(),
            structs: HashMap::new(),
            classes: HashMap::new(),
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn declare_var(&mut self, name: &str, ty: Type) {
        let last = self.scopes.len() - 1;
        self.scopes[last].insert(name.to_string(), ty);
    }

    pub fn get_var_type(&self, name: &str) -> Result<Type, TypeError> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Ok(ty.clone());
            }
        }
        Err(TypeError::UndefinedVariable(name.to_string()))
    }
    
    pub fn register_function(&mut self, name: &str, params: Vec<Type>, return_type: Type) {
        self.functions.insert(name.to_string(), FunctionSignature { params, return_type });
    }
    
    pub fn register_struct(&mut self, name: &str, fields: HashMap<String, Type>) {
        self.structs.insert(name.to_string(), fields);
    }
    
    pub fn register_class(&mut self, name: &str, fields: Vec<(String, Type)>, methods: HashMap<String, FunctionSignature>) {
        let mut field_map = HashMap::new();
        let mut field_order = Vec::new();
        for (k, v) in fields {
            field_map.insert(k.clone(), v);
            field_order.push(k);
        }
        self.classes.insert(name.to_string(), ClassSignature { fields: field_map, field_order, methods });
    }
}

pub struct TypeChecker {
    env: TypeEnvironment,
}

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker {
            env: TypeEnvironment::new(),
        }
    }

    pub fn check_program(&mut self, program: &mut Program) -> Result<(), TypeError> {
        // First pass: Register all functions
        for func in &mut program.functions {
            let mut param_types = Vec::new();
            for param in &func.params {
                param_types.push(param.1.clone().unwrap_or(Type::Any));
            }
            let return_type = func.return_type.clone().unwrap_or(Type::Any);
            self.env.register_function(&func.name, param_types, return_type);
        }

        // Register structs
        for stmt in &program.structs {
            if let Stmt::StructDecl(name, fields, _is_exported) = stmt {
                let mut field_types = HashMap::new();
                for (field_name, opt_type) in fields {
                    field_types.insert(field_name.clone(), opt_type.clone().unwrap_or(Type::Any));
                }
                self.env.register_struct(name, field_types);
            }
        }

        // Register classes and move methods to functions
        let classes = std::mem::take(&mut program.classes);
        for class_decl in classes {
            if let Stmt::ClassDecl(name, fields, methods, is_exported) = class_decl {
                let mut class_fields = Vec::new();
                for (field_name, opt_type) in &fields {
                    class_fields.push((field_name.clone(), opt_type.clone().unwrap_or(Type::Any)));
                }
                let mut method_sigs = HashMap::new();
                for method in &methods {
                    let mut param_types = vec![Type::Class(name.clone())]; // implicit self
                    for param in &method.params {
                        param_types.push(param.1.clone().unwrap_or(Type::Any));
                    }
                    let return_type = method.return_type.clone().unwrap_or(Type::Any);
                    method_sigs.insert(method.name.clone(), FunctionSignature { params: param_types, return_type });
                }
                self.env.register_class(&name, class_fields, method_sigs);
                
                // Move methods to global functions with mangled names
                for mut method in methods {
                    method.name = format!("{}_{}", name, method.name);
                    method.params.insert(0, ("self".to_string(), Some(Type::Class(name.clone()))));
                    
                    let mut param_types = Vec::new();
                    for param in &method.params {
                        param_types.push(param.1.clone().unwrap_or(Type::Any));
                    }
                    let return_type = method.return_type.clone().unwrap_or(Type::Any);
                    self.env.register_function(&method.name, param_types, return_type);
                    
                    program.functions.push(method);
                }
                program.classes.push(Stmt::ClassDecl(name, fields, Vec::new(), is_exported));
            }
        }

        // Second pass: Check function bodies
        for func in &mut program.functions {
            self.check_function(func)?;
        }

        Ok(())
    }

    fn check_function(&mut self, func: &mut Function) -> Result<(), TypeError> {
        self.env.push_scope();

        for param in &func.params {
            let ty = param.1.clone().unwrap_or(Type::Any);
            self.env.declare_var(&param.0, ty);
        }

        let expected_return_type = func.return_type.clone().unwrap_or(Type::Any);

        self.check_block(&mut func.body, &expected_return_type)?;

        self.env.pop_scope();
        Ok(())
    }

    fn check_block(&mut self, block: &mut Block, expected_return: &Type) -> Result<(), TypeError> {
        for stmt in block.iter_mut() {
            self.check_statement(stmt, expected_return)?;
        }
        Ok(())
    }

    fn check_statement(&mut self, stmt: &mut Stmt, expected_return: &Type) -> Result<(), TypeError> {
        match stmt {
            Stmt::Let(name, type_annotation, expr) => {
                let expr_type = self.get_expr_type(expr)?;
                let final_type = match type_annotation {
                    Some(annotated_ty) => {
                        if !self.is_compatible(annotated_ty, &expr_type, expr) {
                            return Err(TypeError::TypeMismatch {
                                expected: format!("{:?}", annotated_ty),
                                found: format!("{:?}", expr_type),
                            });
                        }
                        annotated_ty.clone()
                    }
                    None => expr_type,
                };
                self.env.declare_var(name, final_type);
            }
            Stmt::Assign(name, expr) => {
                let var_type = self.env.get_var_type(name)?;
                let expr_type = self.get_expr_type(expr)?;
                if !self.is_compatible(&var_type, &expr_type, expr) {
                    return Err(TypeError::TypeMismatch {
                        expected: format!("{:?}", var_type),
                        found: format!("{:?}", expr_type),
                    });
                }
            }
            Stmt::AssignIndex(name, index_expr, rhs_expr) => {
                let var_ty = self.env.get_var_type(name)?;
                let index_ty = self.get_expr_type(index_expr)?;
                let rhs_ty = self.get_expr_type(rhs_expr)?;
                
                match var_ty {
                    Type::Array(elem_ty) => {
                        let is_idx_num = matches!(*index_expr, Expr::Number(_));
                        if !is_idx_num && index_ty != Type::Int32 && index_ty != Type::Int64 && index_ty != Type::Any {
                            return Err(TypeError::InvalidIndexType(format!("Array index must be Int32 or Int64, found {:?}", index_ty)));
                        }
                        let is_rhs_num = matches!(*rhs_expr, Expr::Number(_)) && matches!(*elem_ty, Type::Int32 | Type::Int64 | Type::Float32 | Type::Float64);
                        if !is_rhs_num && *elem_ty != Type::Any && rhs_ty != Type::Any && *elem_ty != rhs_ty {
                            return Err(TypeError::TypeMismatch {
                                expected: format!("{:?}", elem_ty),
                                found: format!("{:?}", rhs_ty),
                            });
                        }
                    }
                    Type::Dict(k_ty, v_ty) => {
                        let is_idx_num = matches!(*index_expr, Expr::Number(_)) && matches!(*k_ty, Type::Int32 | Type::Int64 | Type::Float32 | Type::Float64);
                        if !is_idx_num && *k_ty != Type::Any && index_ty != Type::Any && *k_ty != index_ty {
                            return Err(TypeError::InvalidIndexType(format!("Dict key expected {:?}, found {:?}", k_ty, index_ty)));
                        }
                        let is_rhs_num = matches!(*rhs_expr, Expr::Number(_)) && matches!(*v_ty, Type::Int32 | Type::Int64 | Type::Float32 | Type::Float64);
                        if !is_rhs_num && *v_ty != Type::Any && rhs_ty != Type::Any && *v_ty != rhs_ty {
                            return Err(TypeError::TypeMismatch {
                                expected: format!("{:?}", v_ty),
                                found: format!("{:?}", rhs_ty),
                            });
                        }
                    }
                    Type::Any => {}
                    _ => return Err(TypeError::UnsupportedOperation(format!("Cannot index into {:?}", var_ty))),
                }
            }
            Stmt::Return(opt_expr) => {
                let ret_ty = match opt_expr {
                    Some(expr) => self.get_expr_type(expr)?,
                    None => Type::Any,
                };
                if expected_return != &Type::Any && ret_ty != Type::Any && expected_return != &ret_ty {
                    return Err(TypeError::TypeMismatch {
                        expected: format!("{:?}", expected_return),
                        found: format!("{:?}", ret_ty),
                    });
                }
            }
            Stmt::Expr(expr) => {
                self.get_expr_type(expr)?;
            }
            Stmt::If(cond, cons, alt) => {
                let cond_ty = self.get_expr_type(cond)?;
                if cond_ty != Type::Bool && cond_ty != Type::Any {
                    return Err(TypeError::TypeMismatch {
                        expected: "Bool".to_string(),
                        found: format!("{:?}", cond_ty),
                    });
                }
                
                self.env.push_scope();
                self.check_block(cons, expected_return)?;
                self.env.pop_scope();

                if let Some(alt_block) = alt {
                    self.env.push_scope();
                    self.check_block(alt_block, expected_return)?;
                    self.env.pop_scope();
                }
            }
            Stmt::While(cond, body) => {
                let cond_ty = self.get_expr_type(cond)?;
                let is_numeric = matches!(cond, Expr::Number(_)) && matches!(cond_ty, Type::Int32 | Type::Int64 | Type::Float32 | Type::Float64);
                if !is_numeric && cond_ty != Type::Bool && cond_ty != Type::Any {
                    return Err(TypeError::TypeMismatch {
                        expected: "Bool".to_string(),
                        found: format!("{:?}", cond_ty),
                    });
                }
                self.check_block(body, expected_return)?;
            }
            Stmt::For(init, cond, inc, body) => {
                self.env.push_scope();
                self.check_statement(init, expected_return)?;
                
                let cond_ty = self.get_expr_type(cond)?;
                let is_numeric = matches!(cond, Expr::Number(_)) && matches!(cond_ty, Type::Int32 | Type::Int64 | Type::Float32 | Type::Float64);
                if !is_numeric && cond_ty != Type::Bool && cond_ty != Type::Any {
                    return Err(TypeError::TypeMismatch {
                        expected: "Bool".to_string(),
                        found: format!("{:?}", cond_ty),
                    });
                }
                
                self.check_statement(inc, expected_return)?;
                self.check_block(body, expected_return)?;
                self.env.pop_scope();
            }
            Stmt::TryCatch(try_block, catch_var, catch_block) => {
                self.check_block(try_block, expected_return)?;
                self.env.push_scope();
                self.env.declare_var(catch_var, Type::String); // Defaulting to String for thrown errors
                self.check_block(catch_block, expected_return)?;
                self.env.pop_scope();
            }
            Stmt::Throw(expr) => {
                let _ty = self.get_expr_type(expr)?;
                // Allow throwing any type for now
            }
            Stmt::StructDecl(_, _, _) => {}
            Stmt::AssignProperty(obj_expr, prop_name, rhs) => {
                let obj_ty = self.get_expr_type(obj_expr)?;
                let rhs_ty = self.get_expr_type(rhs)?;
                
                if let Type::Struct(struct_name) = &obj_ty {
                    if let Some(struct_fields) = self.env.structs.get(struct_name).cloned() {
                        if let Some(expected_ty) = struct_fields.get(prop_name) {
                            let is_numeric_literal = matches!(rhs, Expr::Number(_)) && matches!(expected_ty, Type::Int32 | Type::Int64 | Type::Float32 | Type::Float64);
                            if !is_numeric_literal && expected_ty != &Type::Any && rhs_ty != Type::Any && expected_ty != &rhs_ty {
                                return Err(TypeError::TypeMismatch {
                                    expected: format!("{:?}", expected_ty),
                                    found: format!("{:?}", rhs_ty),
                                });
                            }
                        } else {
                            return Err(TypeError::UnknownProperty(prop_name.clone()));
                        }
                    } else {
                        return Err(TypeError::UndefinedStruct(struct_name.clone()));
                    }
                } else if let Type::Class(class_name) = &obj_ty {
                    if let Some(class_def) = self.env.classes.get(class_name).cloned() {
                        if let Some(expected_ty) = class_def.fields.get(prop_name) {
                            let is_numeric_literal = matches!(rhs, Expr::Number(_)) && matches!(expected_ty, Type::Int32 | Type::Int64 | Type::Float32 | Type::Float64);
                            if !is_numeric_literal && expected_ty != &Type::Any && rhs_ty != Type::Any && expected_ty != &rhs_ty {
                                return Err(TypeError::TypeMismatch {
                                    expected: format!("{:?}", expected_ty),
                                    found: format!("{:?}", rhs_ty),
                                });
                            }
                        } else {
                            return Err(TypeError::UnknownProperty(prop_name.clone()));
                        }
                    } else {
                        return Err(TypeError::UndefinedClass(class_name.clone()));
                    }
                } else if obj_ty != Type::Any {
                    return Err(TypeError::UnsupportedOperation(format!("Cannot assign property to {:?}", obj_ty)));
                }
            }
            // For other statements, we just accept them for now
            _ => {}
        }
        Ok(())
    }

    fn is_compatible(&self, expected: &Type, found: &Type, expr: &Expr) -> bool {
        if expected == &Type::Any || found == &Type::Any { return true; }
        if expected == found { return true; }
        
        match (expected, found, expr) {
            (Type::Struct(n1), Type::Class(n2), _) if n1 == n2 => true,
            (Type::Class(n1), Type::Struct(n2), _) if n1 == n2 => true,
            (Type::Int32 | Type::Int64 | Type::Float32 | Type::Float64, _, Expr::Number(_)) => true,
            (Type::Bool, _, Expr::Boolean(_)) => true,
            (Type::Array(e_expected), Type::Array(e_found), Expr::ArrayLiteral(elems)) => {
                if e_expected == e_found { return true; }
                if elems.is_empty() { return true; }
                if matches!(**e_expected, Type::Int32 | Type::Int64 | Type::Float32 | Type::Float64) && **e_found == Type::Float64 {
                    elems.iter().all(|el| matches!(el, Expr::Number(_)))
                } else {
                    false
                }
            }
            (Type::Dict(k_expected, v_expected), Type::Dict(k_found, v_found), Expr::DictLiteral(elems)) => {
                let k_compat = if k_expected == k_found { true } else if matches!(**k_expected, Type::Int32 | Type::Int64 | Type::Float32 | Type::Float64) && **k_found == Type::Float64 {
                    elems.iter().all(|(k, _)| matches!(k, Expr::Number(_)))
                } else { false };
                
                let v_compat = if v_expected == v_found { true } else if matches!(**v_expected, Type::Int32 | Type::Int64 | Type::Float32 | Type::Float64) && **v_found == Type::Float64 {
                    elems.iter().all(|(_, v)| matches!(v, Expr::Number(_)))
                } else { false };
                
                k_compat && v_compat
            }
            _ => false
        }
    }

    fn get_expr_type(&mut self, expr: &mut Expr) -> Result<Type, TypeError> {
        match expr {
            Expr::Number(_) => Ok(Type::Float64), // Default number literal is Float64
            Expr::Boolean(_) => Ok(Type::Bool),
            Expr::StringLiteral(_) => Ok(Type::String),
            Expr::Identifier(name, ref mut expr_type) => {
                // "true" and "false" are bools, but handled by lexer usually?
                // Let's rely on environment for idents
                let ty = self.env.get_var_type(name)?;
                *expr_type = Some(ty.clone());
                Ok(ty)
            }
            Expr::BinaryOp(left, op, right, ref mut expr_type) => {
                let l_ty = self.get_expr_type(left)?;
                let r_ty = self.get_expr_type(right)?;
                
                // String concatenation: if either side is String, the Add is string concat
                if *op == BinaryOperator::Add && (l_ty == Type::String || r_ty == Type::String) {
                    *expr_type = Some(Type::String);
                    return Ok(Type::String);
                }
                
                if l_ty == Type::Any || r_ty == Type::Any {
                    return Ok(Type::Any);
                }

                match op {
                    BinaryOperator::Add | BinaryOperator::Subtract | BinaryOperator::Multiply | BinaryOperator::Divide => {
                        // For strict checking, we require exact match
                        if l_ty != r_ty {
                            return Err(TypeError::TypeMismatch {
                                expected: format!("{:?}", l_ty),
                                found: format!("{:?}", r_ty),
                            });
                        }

                        match l_ty {
                            Type::Int32 | Type::Int64 | Type::Float32 | Type::Float64 => { *expr_type = Some(l_ty.clone()); Ok(l_ty) }
                            Type::String if *op == BinaryOperator::Add => { *expr_type = Some(Type::String); Ok(Type::String) }
                            _ => Err(TypeError::UnsupportedOperation(format!("Cannot perform math on {:?}", l_ty))),
                        }

                    }
                    BinaryOperator::Equals | BinaryOperator::NotEquals | BinaryOperator::LessThan | BinaryOperator::GreaterThan | BinaryOperator::LessThanOrEq | BinaryOperator::GreaterThanOrEq => {
                        *expr_type = Some(Type::Bool);
                        Ok(Type::Bool)
                    }
                    BinaryOperator::And | BinaryOperator::Or => {
                        if l_ty != Type::Bool && l_ty != Type::Any {
                            return Err(TypeError::TypeMismatch {
                                expected: "Bool".to_string(),
                                found: format!("{:?}", l_ty),
                            });
                        }
                        if r_ty != Type::Bool && r_ty != Type::Any {
                            return Err(TypeError::TypeMismatch {
                                expected: "Bool".to_string(),
                                found: format!("{:?}", r_ty),
                            });
                        }
                        *expr_type = Some(Type::Bool);
                        Ok(Type::Bool)
                    }
                }
            }
            Expr::Call(name, args, ref mut expr_type) => {
                // Simple call check
                if let Some(sig) = self.env.functions.get(name).cloned() {
                    if sig.params.len() != args.len() {
                        return Err(TypeError::InvalidArguments(format!("Expected {} args, got {}", sig.params.len(), args.len())));
                    }
                    for (i, arg) in args.iter_mut().enumerate() {
                        let arg_ty = self.get_expr_type(arg)?;
                        if !self.is_compatible(&sig.params[i], &arg_ty, arg) {
                            return Err(TypeError::TypeMismatch {
                                expected: format!("{:?}", sig.params[i]),
                                found: format!("{:?}", arg_ty),
                            });
                        }
                    }
                    *expr_type = Some(sig.return_type.clone());
                    return Ok(sig.return_type);
                }
                
                for arg in args.iter_mut() {
                    let _ = self.get_expr_type(arg);
                }
                
                // Allow FFI or built-in calls that we don't know about by returning Any
                *expr_type = Some(Type::Any);
                Ok(Type::Any)
            }
            Expr::NewClass(_, _) => {
                let (name, mut args) = match std::mem::replace(expr, Expr::Number(0.0)) {
                    Expr::NewClass(n, a) => (n, a),
                    _ => unreachable!(),
                };
                if let Some(class_def) = self.env.classes.get(&name).cloned() {
                    if class_def.field_order.len() != args.len() {
                        return Err(TypeError::InvalidArguments(format!("Class {} expected {} args, got {}", name, class_def.field_order.len(), args.len())));
                    }
                    for (i, arg) in args.iter_mut().enumerate() {
                        let arg_ty = self.get_expr_type(arg)?;
                        let field_name = &class_def.field_order[i];
                        let expected_ty = &class_def.fields[field_name];
                        let is_numeric_literal = matches!(arg, Expr::Number(_)) && matches!(expected_ty, Type::Int32 | Type::Int64 | Type::Float32 | Type::Float64);
                        if !is_numeric_literal && expected_ty != &Type::Any && arg_ty != Type::Any && expected_ty != &arg_ty {
                            return Err(TypeError::TypeMismatch {
                                expected: format!("{:?}", expected_ty),
                                found: format!("{:?}", arg_ty),
                            });
                        }
                    }
                    
                    let mut fields = Vec::new();
                    for (i, arg) in args.into_iter().enumerate() {
                        fields.push((class_def.field_order[i].clone(), arg));
                    }
                    *expr = Expr::StructInit(name.clone(), fields);
                    
                    Ok(Type::Class(name.clone()))
                } else {
                    Err(TypeError::UndefinedStruct(name.clone()))
                }
            }
            Expr::StructInit(name, fields) => {
                if let Some(struct_def) = self.env.structs.get(name).cloned() {
                    let mut provided_fields = HashSet::new();
                    for (field_name, field_expr) in fields {
                        provided_fields.insert(field_name.clone());
                        let expr_ty = self.get_expr_type(field_expr)?;
                        if let Some(expected_ty) = struct_def.get(field_name) {
                            let is_numeric_literal = matches!(field_expr, Expr::Number(_)) && matches!(expected_ty, Type::Int32 | Type::Int64 | Type::Float32 | Type::Float64);
                            if !is_numeric_literal && expected_ty != &Type::Any && expr_ty != Type::Any && expected_ty != &expr_ty {
                                return Err(TypeError::TypeMismatch {
                                    expected: format!("{:?}", expected_ty),
                                    found: format!("{:?}", expr_ty),
                                });
                            }
                        } else {
                            return Err(TypeError::ExtraField(field_name.clone()));
                        }
                    }
                    for expected_field in struct_def.keys() {
                        if !provided_fields.contains(expected_field) {
                            return Err(TypeError::MissingField(expected_field.clone()));
                        }
                    }
                    let struct_ty = Type::Struct(name.clone());
                    Ok(struct_ty)
                } else {
                    Err(TypeError::UndefinedStruct(name.clone()))
                }
            }
            Expr::PropertyAccess(obj_expr, prop_name, ref mut expr_type) => {
                let obj_ty = self.get_expr_type(obj_expr)?;
                let class_name = match &obj_ty {
                    Type::Class(name) => Some(name.clone()),
                    Type::Struct(name) => {
                        if self.env.classes.contains_key(name) {
                            Some(name.clone())
                        } else {
                            None
                        }
                    },
                    _ => None,
                };

                if let Some(c_name) = class_name {
                    if let Some(class_def) = self.env.classes.get(&c_name).cloned() {
                        if let Some(prop_ty) = class_def.fields.get(prop_name) {
                            *expr_type = Some(prop_ty.clone());
                            Ok(prop_ty.clone())
                        } else {
                            Err(TypeError::UnknownProperty(prop_name.clone()))
                        }
                    } else {
                        Err(TypeError::UndefinedClass(c_name))
                    }
                } else if let Type::Struct(struct_name) = obj_ty {
                    if let Some(struct_fields) = self.env.structs.get(&struct_name).cloned() {
                        if let Some(prop_ty) = struct_fields.get(prop_name) {
                            *expr_type = Some(prop_ty.clone());
                            Ok(prop_ty.clone())
                        } else {
                            Err(TypeError::UnknownProperty(prop_name.clone()))
                        }
                    } else {
                        Err(TypeError::UndefinedStruct(struct_name.clone()))
                    }
                } else if obj_ty == Type::Any {
                    *expr_type = Some(Type::Any);
                    Ok(Type::Any)
                } else {
                    Err(TypeError::UnsupportedOperation(format!("Cannot access property on {:?}", obj_ty)))
                }
            }
            Expr::MethodCall(_, _, _, _) => {
                let (obj_expr, method_name, mut args, mut opt_type) = match std::mem::replace(expr, Expr::Number(0.0)) {
                    Expr::MethodCall(o, m, a, t) => (o, m, a, t),
                    _ => unreachable!(),
                };
                
                let obj_ty = self.get_expr_type(&mut obj_expr.clone())?;
                let class_name = match &obj_ty {
                    Type::Class(name) => Some(name.clone()),
                    Type::Struct(name) => {
                        if self.env.classes.contains_key(name) {
                            Some(name.clone())
                        } else {
                            None
                        }
                    },
                    _ => None,
                };
                
                if let Some(class_name) = class_name {
                    if let Some(class_def) = self.env.classes.get(&class_name).cloned() {
                        if let Some(sig) = class_def.methods.get(&method_name) {
                            let expected_args = sig.params.len() - 1;
                            if expected_args != args.len() {
                                return Err(TypeError::InvalidArguments(format!("Expected {} args, got {}", expected_args, args.len())));
                            }
                            for (i, arg) in args.iter_mut().enumerate() {
                                let arg_ty = self.get_expr_type(arg)?;
                                if !self.is_compatible(&sig.params[i + 1], &arg_ty, arg) {
                                    return Err(TypeError::TypeMismatch {
                                        expected: format!("{:?}", sig.params[i + 1]),
                                        found: format!("{:?}", arg_ty),
                                    });
                                }
                            }
                            
                            opt_type = Some(sig.return_type.clone());
                            let mangled_name = format!("{}_{}", class_name, method_name);
                            
                            // Insert obj_expr as the first argument (self)
                            args.insert(0, *obj_expr);
                            *expr = Expr::Call(mangled_name, args, opt_type);
                            
                            return Ok(sig.return_type.clone());
                        } else {
                            return Err(TypeError::UndefinedFunction(method_name.clone()));
                        }
                    } else {
                        return Err(TypeError::UndefinedClass(class_name.clone()));
                    }
                } else if obj_ty == Type::Any {
                    opt_type = Some(Type::Any);
                    *expr = Expr::Call(method_name, args, opt_type);
                    Ok(Type::Any)
                } else {
                    return Err(TypeError::UnsupportedOperation(format!("Cannot call method on {:?}", obj_ty)));
                }
            }
            Expr::ArrayLiteral(elements) => {
                if elements.is_empty() {
                    Ok(Type::Array(Box::new(Type::Any)))
                } else {
                    let mut elem_ty = None;
                    for el in elements {
                        let ty = self.get_expr_type(el)?;
                        if let Some(ref current_ty) = elem_ty {
                            let is_numeric_literal = matches!(el, Expr::Number(_)) && matches!(current_ty, Type::Int32 | Type::Int64 | Type::Float32 | Type::Float64);
                            if !is_numeric_literal && current_ty != &ty && current_ty != &Type::Any && ty != Type::Any {
                                return Err(TypeError::TypeMismatch {
                                    expected: format!("{:?}", current_ty),
                                    found: format!("{:?}", ty),
                                });
                            }
                        } else {
                            elem_ty = Some(ty);
                        }
                    }
                    Ok(Type::Array(Box::new(elem_ty.unwrap_or(Type::Any))))
                }
            }
            Expr::DictLiteral(elements) => {
                if elements.is_empty() {
                    Ok(Type::Dict(Box::new(Type::Any), Box::new(Type::Any)))
                } else {
                    let mut key_ty = None;
                    let mut val_ty = None;
                    for (k, v) in elements {
                        let kt = self.get_expr_type(k)?;
                        let vt = self.get_expr_type(v)?;
                        
                        if let Some(ref current_k) = key_ty {
                            let is_numeric_literal = matches!(k, Expr::Number(_)) && matches!(current_k, Type::Int32 | Type::Int64 | Type::Float32 | Type::Float64);
                            if !is_numeric_literal && current_k != &kt && current_k != &Type::Any && kt != Type::Any {
                                return Err(TypeError::TypeMismatch {
                                    expected: format!("{:?}", current_k),
                                    found: format!("{:?}", kt),
                                });
                            }
                        } else {
                            key_ty = Some(kt);
                        }
                        
                        if let Some(ref current_v) = val_ty {
                            let is_numeric_literal = matches!(v, Expr::Number(_)) && matches!(current_v, Type::Int32 | Type::Int64 | Type::Float32 | Type::Float64);
                            if !is_numeric_literal && current_v != &vt && current_v != &Type::Any && vt != Type::Any {
                                return Err(TypeError::TypeMismatch {
                                    expected: format!("{:?}", current_v),
                                    found: format!("{:?}", vt),
                                });
                            }
                        } else {
                            val_ty = Some(vt);
                        }
                    }
                    Ok(Type::Dict(Box::new(key_ty.unwrap_or(Type::Any)), Box::new(val_ty.unwrap_or(Type::Any))))
                }
            }
            Expr::IndexAccess(base_expr, index_expr) => {
                let base_ty = self.get_expr_type(base_expr)?;
                let index_ty = self.get_expr_type(index_expr)?;
                
                match base_ty {
                    Type::Array(elem_ty) => {
                        let is_numeric_literal = matches!(**index_expr, Expr::Number(_));
                        if !is_numeric_literal && index_ty != Type::Int32 && index_ty != Type::Int64 && index_ty != Type::Any {
                            return Err(TypeError::InvalidIndexType(format!("Array index must be Int32 or Int64, found {:?}", index_ty)));
                        }
                        Ok(*elem_ty)
                    }
                    Type::Dict(k_ty, v_ty) => {
                        let is_numeric_literal = matches!(**index_expr, Expr::Number(_)) && matches!(*k_ty, Type::Int32 | Type::Int64 | Type::Float32 | Type::Float64);
                        if !is_numeric_literal && *k_ty != Type::Any && index_ty != Type::Any && *k_ty != index_ty {
                            return Err(TypeError::InvalidIndexType(format!("Dict key expected {:?}, found {:?}", k_ty, index_ty)));
                        }
                        Ok(*v_ty)
                    }
                    Type::Any => Ok(Type::Any),
                    _ => Err(TypeError::UnsupportedOperation(format!("Cannot index into {:?}", base_ty))),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use crate::lexer::Lexer;

    fn check(source: &str) -> Result<(), TypeError> {
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let mut program = parser.parse_program().unwrap();
        let mut checker = TypeChecker::new();
        checker.check_program(&mut program)
    }

    #[test]
    fn test_valid_types() {
        let source = r#"
            fn add(x: Int32, y: Int32) -> Int32 {
                return x + y;
            }
            fn main() {
                let a: Int32 = 10;
                let b: Int32 = 20;
                let c: Int32 = add(a, b);
            }
        "#;
        assert_eq!(check(source), Ok(()));
    }

    #[test]
    fn test_logical_operators() {
        let source = r#"
            fn main() {
                let a: Bool = 1.0 < 2.0 && 2.0 < 3.0;
                let b: Bool = 1.0 < 2.0 || 2.0 < 3.0;
            }
        "#;
        assert_eq!(check(source), Ok(()));
    }

    #[test]
    fn test_for_loop() {
        let source = r#"
            fn main() {
                for let i = 0; i < 10; i = i + 1 {
                    let a: Float64 = i;
                }
            }
        "#;
        assert_eq!(check(source), Ok(()));
    }

    #[test]
    fn test_try_catch() {
        let source = r#"
            fn main() {
                try {
                    throw "Error";
                } catch (err) {
                    let msg: String = err;
                }
            }
        "#;
        assert_eq!(check(source), Ok(()));
    }

    #[test]
    fn test_type_mismatch_assignment() {
        let source = r#"
            fn main() {
                let a: Int32 = "hello";
            }
        "#;
        let res = check(source);
        assert!(res.is_err());
    }

    #[test]
    fn test_type_mismatch_return() {
        let source = r#"
            fn add(x: Int32, y: Int32) -> String {
                return x + y;
            }
        "#;
        let res = check(source);
        assert!(res.is_err());
    }

    #[test]
    fn test_struct_init_valid() {
        let source = r#"
            struct Point { x: Int32, y: Int32 }
            fn main() {
                let p: Point = new Point { x: 10, y: 20 };
            }
        "#;
        assert_eq!(check(source), Ok(()));
    }

    #[test]
    fn test_struct_init_missing_field() {
        let source = r#"
            struct Point { x: Int32, y: Int32 }
            fn main() {
                let p: Point = new Point { x: 10 };
            }
        "#;
        assert!(check(source).is_err());
    }

    #[test]
    fn test_struct_init_type_mismatch() {
        let source = r#"
            struct Point { x: Int32, y: Int32 }
            fn main() {
                let p: Point = new Point { x: 10, y: "hello" };
            }
        "#;
        assert!(check(source).is_err());
    }

    #[test]
    fn test_struct_property_access() {
        let source = r#"
            struct Point { x: Int32, y: Int32 }
            fn get_x(p: Point) -> Int32 {
                return p.x;
            }
        "#;
        assert_eq!(check(source), Ok(()));
    }

    #[test]
    fn test_array_valid() {
        let source = r#"
            fn main() {
                let a: Array<Int32> = [1, 2, 3];
                let b: Int32 = a[0];
                a[1] = 10;
            }
        "#;
        assert_eq!(check(source), Ok(()));
    }

    #[test]
    fn test_array_invalid_type() {
        let source = r#"
            fn main() {
                let a: Array<Int32> = [1, "two", 3];
            }
        "#;
        assert!(check(source).is_err());
    }

    #[test]
    fn test_dict_valid() {
        let source = r#"
            fn main() {
                let d: Dict<String, Int32> = { "a": 1, "b": 2 };
                let val: Int32 = d["a"];
                d["b"] = 10;
            }
        "#;
        assert_eq!(check(source), Ok(()));
    }

    #[test]
    fn test_dict_invalid_key_type() {
        let source = r#"
            fn main() {
                let d: Dict<String, Int32> = { "a": 1 };
                let val: Int32 = d[10];
            }
        "#;
        assert!(check(source).is_err());
    }
}
