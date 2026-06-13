use vre_compiler::ast::*;

pub fn format_program(program: &Program) -> String {
    let mut out = String::new();
    
    for import in &program.imports {
        out.push_str(&format!("import \"{}\"", import.path));
        if let Some(alias) = &import.alias {
            out.push_str(&format!(" as {}", alias));
        }
        out.push_str(";\n");
    }
    
    if !program.imports.is_empty() && (!program.structs.is_empty() || !program.classes.is_empty() || !program.functions.is_empty()) {
        out.push('\n');
    }

    for (i, struct_decl) in program.structs.iter().enumerate() {
        out.push_str(&format_stmt(struct_decl, 0));
        if i < program.structs.len() - 1 || !program.classes.is_empty() || !program.functions.is_empty() {
            out.push('\n');
        }
    }

    for (i, class_decl) in program.classes.iter().enumerate() {
        out.push_str(&format_stmt(class_decl, 0));
        if i < program.classes.len() - 1 || !program.functions.is_empty() {
            out.push('\n');
        }
    }

    for (i, func) in program.functions.iter().enumerate() {
        out.push_str(&format_function(func, 0));
        if i < program.functions.len() - 1 {
            out.push('\n');
            out.push('\n');
        } else {
            out.push('\n');
        }
    }

    out
}

fn indent(level: usize) -> String {
    "    ".repeat(level)
}

fn format_type(t: &Type) -> String {
    match t {
        Type::Int32 => "Int32".to_string(),
        Type::Int64 => "Int64".to_string(),
        Type::Float32 => "Float32".to_string(),
        Type::Float64 => "Float64".to_string(),
        Type::Bool => "Bool".to_string(),
        Type::String => "String".to_string(),
        Type::Any => "Any".to_string(),
        Type::Struct(name) | Type::Class(name) => name.clone(),
        Type::Array(inner) => format!("Array<{}>", format_type(inner)),
        Type::Dict(k, v) => format!("Dict<{}, {}>", format_type(k), format_type(v)),
        Type::Function(params, ret) => {
            let p_str: Vec<String> = params.iter().map(format_type).collect();
            format!("fn({}) -> {}", p_str.join(", "), format_type(ret))
        }
    }
}

fn format_function(f: &Function, level: usize) -> String {
    let mut out = indent(level);
    if f.is_exported {
        out.push_str("export ");
    }
    out.push_str(&format!("fn {}(", f.name));
    
    let params: Vec<String> = f.params.iter().map(|(n, t)| {
        if let Some(typ) = t {
            format!("{}: {}", n, format_type(typ))
        } else {
            n.clone()
        }
    }).collect();
    
    out.push_str(&params.join(", "));
    out.push(')');
    
    if let Some(ret) = &f.return_type {
        out.push_str(&format!(" -> {}", format_type(ret)));
    }
    
    out.push_str(" {\n");
    for stmt in &f.body {
        out.push_str(&format_stmt(stmt, level + 1));
        out.push('\n');
    }
    out.push_str(&indent(level));
    out.push('}');
    out
}

fn format_block(block: &Block, level: usize) -> String {
    if block.is_empty() {
        return "{}".to_string();
    }
    let mut out = "{\n".to_string();
    for stmt in block {
        out.push_str(&format_stmt(stmt, level + 1));
        out.push('\n');
    }
    out.push_str(&indent(level));
    out.push('}');
    out
}

fn format_stmt(stmt: &Stmt, level: usize) -> String {
    let ind = indent(level);
    match stmt {
        Stmt::Let(name, t, expr) => {
            let type_str = t.as_ref().map(|typ| format!(": {}", format_type(typ))).unwrap_or_default();
            format!("{}let {}{} = {};", ind, name, type_str, format_expr(expr))
        }
        Stmt::LetMut(name, t, expr) => {
            let type_str = t.as_ref().map(|typ| format!(": {}", format_type(typ))).unwrap_or_default();
            format!("{}let mut {}{} = {};", ind, name, type_str, format_expr(expr))
        }
        Stmt::Assign(name, expr) => {
            format!("{}{} = {};", ind, name, format_expr(expr))
        }
        Stmt::AssignIndex(name, idx, expr) => {
            format!("{}{}[{}] = {};", ind, name, format_expr(idx), format_expr(expr))
        }
        Stmt::AssignProperty(obj, prop, expr) => {
            format!("{}{}.{} = {};", ind, format_expr(obj), prop, format_expr(expr))
        }
        Stmt::Expr(expr) => {
            format!("{}{};", ind, format_expr(expr))
        }
        Stmt::Return(expr) => {
            if let Some(e) = expr {
                format!("{}return {};", ind, format_expr(e))
            } else {
                format!("{}return;", ind)
            }
        }
        Stmt::Throw(expr) => {
            format!("{}throw {};", ind, format_expr(expr))
        }
        Stmt::Yield => {
            format!("{}yield;", ind)
        }
        Stmt::If(cond, then_b, else_b) => {
            let mut out = format!("{}if {} {}", ind, format_expr(cond), format_block(then_b, level));
            if let Some(eb) = else_b {
                out.push_str(" else ");
                out.push_str(&format_block(eb, level));
            }
            out
        }
        Stmt::While(cond, body) => {
            format!("{}while {} {}", ind, format_expr(cond), format_block(body, level))
        }
        Stmt::For(init, cond, inc, body) => {
            // For loop format is slightly tricky because statements have trailing semicolons
            let init_str = format_stmt(init, 0);
            let inc_str = format_stmt(inc, 0);
            format!("{}for ({} {} {}) {}", ind, init_str, format_expr(cond), inc_str.trim_end_matches(';'), format_block(body, level))
        }
        Stmt::StructDecl(name, fields, exported) => {
            let mut out = ind.clone();
            if *exported { out.push_str("export "); }
            out.push_str(&format!("struct {} {{\n", name));
            for (i, (fname, ftype)) in fields.iter().enumerate() {
                let t_str = ftype.as_ref().map(|t| format!(": {}", format_type(t))).unwrap_or_default();
                out.push_str(&format!("{}    {}{}", ind, fname, t_str));
                if i < fields.len() - 1 {
                    out.push_str(",\n");
                } else {
                    out.push('\n');
                }
            }
            out.push_str(&ind);
            out.push_str("}\n");
            out
        }
        Stmt::ClassDecl(name, fields, methods, exported) => {
            let mut out = ind.clone();
            if *exported { out.push_str("export "); }
            out.push_str(&format!("class {} {{\n", name));
            for (i, (fname, ftype)) in fields.iter().enumerate() {
                let t_str = ftype.as_ref().map(|t| format!(": {}", format_type(t))).unwrap_or_default();
                out.push_str(&format!("{}    {}{}", ind, fname, t_str));
                if i < fields.len() - 1 || !methods.is_empty() {
                    out.push_str(",\n");
                } else {
                    out.push('\n');
                }
            }
            for func in methods {
                out.push_str(&format_function(func, level + 1));
                out.push('\n');
            }
            out.push_str(&ind);
            out.push_str("}\n");
            out
        }
        Stmt::TryCatch(try_b, err_name, catch_b) => {
            format!("{}try {} catch ({}) {}", ind, format_block(try_b, level), err_name, format_block(catch_b, level))
        }
    }
}

fn format_expr(expr: &Expr) -> String {
    match expr {
        Expr::Number(n) => n.to_string(),
        Expr::Boolean(b) => b.to_string(),
        Expr::Identifier(s, _) => s.clone(),
        Expr::StringLiteral(s) => format!("\"{}\"", s), // In a real formatter we'd escape this
        Expr::BinaryOp(l, op, r, _) => {
            let op_str = match op {
                BinaryOperator::Add => "+",
                BinaryOperator::Subtract => "-",
                BinaryOperator::Multiply => "*",
                BinaryOperator::Divide => "/",
                BinaryOperator::Equals => "==",
                BinaryOperator::NotEquals => "!=",
                BinaryOperator::LessThan => "<",
                BinaryOperator::GreaterThan => ">",
                BinaryOperator::LessThanOrEq => "<=",
                BinaryOperator::GreaterThanOrEq => ">=",
                BinaryOperator::And => "&&",
                BinaryOperator::Or => "||",
            };
            format!("{} {} {}", format_expr(l), op_str, format_expr(r))
        }
        Expr::Call(name, args, _) => {
            let arg_strs: Vec<String> = args.iter().map(format_expr).collect();
            format!("{}({})", name, arg_strs.join(", "))
        }
        Expr::CallDynamic(func, args, _) => {
            let arg_strs: Vec<String> = args.iter().map(format_expr).collect();
            format!("{}({})", format_expr(func), arg_strs.join(", "))
        }
        Expr::MethodCall(obj, name, args, _) => {
            let arg_strs: Vec<String> = args.iter().map(format_expr).collect();
            format!("{}.{}({})", format_expr(obj), name, arg_strs.join(", "))
        }
        Expr::NamedCall(name, args, _) => {
            let arg_strs: Vec<String> = args.iter().map(|a| {
                if let Some(n) = &a.name {
                    format!("{} = {}", n, format_expr(&a.value))
                } else {
                    format_expr(&a.value)
                }
            }).collect();
            format!("{}({})", name, arg_strs.join(", "))
        }
        Expr::NamedMethodCall(obj, name, args, _) => {
            let arg_strs: Vec<String> = args.iter().map(|a| {
                if let Some(n) = &a.name {
                    format!("{} = {}", n, format_expr(&a.value))
                } else {
                    format_expr(&a.value)
                }
            }).collect();
            format!("{}.{}({})", format_expr(obj), name, arg_strs.join(", "))
        }
        Expr::NamedNewClass(name, args) => {
            let arg_strs: Vec<String> = args.iter().map(|a| {
                if let Some(n) = &a.name {
                    format!("{} = {}", n, format_expr(&a.value))
                } else {
                    format_expr(&a.value)
                }
            }).collect();
            format!("new {}({})", name, arg_strs.join(", "))
        }
        Expr::ArrayLiteral(elements) => {
            let el_strs: Vec<String> = elements.iter().map(format_expr).collect();
            format!("[{}]", el_strs.join(", "))
        }
        Expr::DictLiteral(pairs) => {
            let mut out = "{\n".to_string();
            for (i, (k, v)) in pairs.iter().enumerate() {
                out.push_str(&format!("    {}: {}", format_expr(k), format_expr(v)));
                if i < pairs.len() - 1 {
                    out.push_str(",\n");
                } else {
                    out.push('\n');
                }
            }
            out.push('}');
            out
        }
        Expr::IndexAccess(obj, idx, _) => {
            format!("{}[{}]", format_expr(obj), format_expr(idx))
        }
        Expr::PropertyAccess(obj, prop, _) => {
            format!("{}.{}", format_expr(obj), prop)
        }
        Expr::StructInit(name, fields) => {
            let mut out = format!("{} {{\n", name);
            for (i, (fname, fexpr)) in fields.iter().enumerate() {
                out.push_str(&format!("    {}: {}", fname, format_expr(fexpr)));
                if i < fields.len() - 1 {
                    out.push_str(",\n");
                } else {
                    out.push('\n');
                }
            }
            out.push('}');
            out
        }
        Expr::NewClass(name, args) => {
            let arg_strs: Vec<String> = args.iter().map(format_expr).collect();
            format!("new {}({})", name, arg_strs.join(", "))
        }
        Expr::Closure { params, return_type, body } => {
            let p_str: Vec<String> = params.iter().map(|(n, t)| {
                if let Some(typ) = t {
                    format!("{}: {}", n, format_type(typ))
                } else {
                    n.clone()
                }
            }).collect();
            let ret_str = return_type.as_ref().map(|t| format!(" -> {}", format_type(t))).unwrap_or_default();
            format!("|{}|{} {}", p_str.join(", "), ret_str, format_block(body, 0))
        }
    }
}
