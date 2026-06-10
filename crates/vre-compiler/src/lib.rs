pub mod lexer;
pub mod lexer_indent;
pub mod ast;
pub mod parser;
pub mod parser_indent;
pub mod compiler;
pub mod type_checker;
pub mod optimizer;
pub mod vir;
use crate::ast::{Program, Expr, Stmt};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::compiler::Compiler;
use crate::type_checker::TypeChecker;
use std::path::{Path, PathBuf};
use std::collections::HashSet;

pub use crate::compiler::CompiledProgram;

pub fn compile(source: &str, path_str: &str, base_path: Option<&Path>) -> Result<CompiledProgram, String> {
    let mut visited = HashSet::new();
    let mut program = parse_and_resolve(source, path_str, base_path, &mut visited)?;
    
    // Static Type Checking Pass
    let mut checker = TypeChecker::new();
    checker.check_program(&mut program).map_err(|e| format!("Type Error: {}", e))?;
    
    // AST Optimization Pass
    let mut optimizer = optimizer::AstOptimizer::new();
    optimizer.optimize(&mut program);
    
    let compiler = Compiler::new();
    let compiled = compiler.compile(program)?;
    
    Ok(compiled)
}

fn parse_and_resolve(
    source: &str,
    path_str: &str,
    base_path: Option<&Path>,
    visited: &mut HashSet<PathBuf>,
) -> Result<Program, String> {
    let is_vym = path_str.ends_with(".vym");

    let program = if is_vym {
        let lexer = crate::lexer_indent::LexerIndent::new(source);
        let mut parser = crate::parser_indent::ParserIndent::new(lexer);
        parser.parse_program()?
    } else {
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        parser.parse_program()?
    };

    let mut merged_functions = program.functions.clone();
    let mut merged_structs = program.structs.clone();
    let mut merged_classes = program.classes.clone();

    for import_decl in &program.imports {
        let is_std = import_decl.path.starts_with("std/");
        let is_relative = import_decl.path.starts_with("./") || import_decl.path.starts_with("../");
        
        let import_path = if is_std {
            let std_root = std::env::var("VRE_STD_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("std"));
            let stripped = import_decl.path.strip_prefix("std/").unwrap();
            std_root.join(resolve_import_path(stripped))
        } else if is_relative {
            if let Some(base) = base_path {
                base.join(resolve_import_path(&import_decl.path))
            } else {
                return Err(format!(
                    "Cannot resolve relative import '{}' without a base path",
                    import_decl.path
                ));
            }
        } else {
            // Package resolution from vyauma_modules
            let mut p = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            p.push("vyauma_modules");
            p.push(&import_decl.path);
            p.push("index.vym");
            p
        };

        let canonical = match vre_core::pal::get_pal().canonicalize(&import_path) {
            Ok(p) => p,
            Err(_) => import_path.clone(),
        };

        if visited.contains(&canonical) {
            continue;
        }
        visited.insert(canonical.clone());

        let imported_source = vre_core::pal::get_pal()
            .read_to_string(&import_path)
            .map_err(|e| format!("Failed to read imported file '{}': {}", import_path.display(), e))?;

        let new_base = import_path.parent();
        let import_path_str_full = import_path.to_string_lossy().to_string();
        let mut imported_program =
            parse_and_resolve(&imported_source, &import_path_str_full, new_base, visited)?;

        // Apply name-mangling using the import's namespace
        let namespace = import_decl.namespace();
        mangle_program(&mut imported_program, &namespace);

        merged_functions.extend(imported_program.functions);
        merged_structs.extend(imported_program.structs);
        merged_classes.extend(imported_program.classes);
    }

    Ok(Program {
        imports: vec![],
        functions: merged_functions,
        structs: merged_structs,
        classes: merged_classes,
    })
}

/// Resolves an import path to a filename with extension.
/// Bare paths (e.g. "utils") get ".vya" appended; paths already ending in
/// ".vya" or ".vym" are returned as-is.
fn resolve_import_path(path: &str) -> String {
    if path.ends_with(".vya") || path.ends_with(".vym") {
        return path.to_string();
    }
    format!("{}.vya", path)
}

/// Name-mangling pass: prefixes all function names in `program` with
/// `namespace__` and rewrites all intra-module `Expr::Call` references
/// so they continue to point to the now-renamed functions.
/// Non-exported functions are mangled with `__private_namespace__` to prevent external access.
fn mangle_program(program: &mut Program, namespace: &str) {
    // Map of local function name -> is_exported
    let local_fns: std::collections::HashMap<String, bool> = program
        .functions
        .iter()
        .map(|f| (f.name.clone(), f.is_exported))
        .collect();

    for func in &mut program.functions {
        if func.is_exported {
            func.name = format!("{}__{}", namespace, func.name);
        } else {
            func.name = format!("__private_{}__{}", namespace, func.name);
        }
        for stmt in &mut func.body {
            mangle_stmt(stmt, namespace, &local_fns);
        }
    }
}

fn mangle_stmt(stmt: &mut Stmt, namespace: &str, local_fns: &std::collections::HashMap<String, bool>) {
    match stmt {
        Stmt::Let(_, _, expr) => mangle_expr(expr, namespace, local_fns),
        Stmt::Assign(_, expr) => mangle_expr(expr, namespace, local_fns),
        Stmt::AssignIndex(_, idx, val) => {
            mangle_expr(idx, namespace, local_fns);
            mangle_expr(val, namespace, local_fns);
        }
        Stmt::AssignProperty(obj, _, val) => {
            mangle_expr(obj, namespace, local_fns);
            mangle_expr(val, namespace, local_fns);
        }
        Stmt::Expr(expr) => mangle_expr(expr, namespace, local_fns),
        Stmt::Return(Some(expr)) => mangle_expr(expr, namespace, local_fns),
        Stmt::Throw(expr) => mangle_expr(expr, namespace, local_fns),
        Stmt::If(cond, cons, alt) => {
            mangle_expr(cond, namespace, local_fns);
            for s in cons {
                mangle_stmt(s, namespace, local_fns);
            }
            if let Some(alt_block) = alt {
                for s in alt_block {
                    mangle_stmt(s, namespace, local_fns);
                }
            }
        }
        Stmt::While(cond, body) => {
            mangle_expr(cond, namespace, local_fns);
            for s in body {
                mangle_stmt(s, namespace, local_fns);
            }
        }
        Stmt::For(init, cond, inc, body) => {
            mangle_stmt(init, namespace, local_fns);
            mangle_expr(cond, namespace, local_fns);
            mangle_stmt(inc, namespace, local_fns);
            for s in body {
                mangle_stmt(s, namespace, local_fns);
            }
        }
        Stmt::TryCatch(try_block, _, catch_block) => {
            for s in try_block {
                mangle_stmt(s, namespace, local_fns);
            }
            for s in catch_block {
                mangle_stmt(s, namespace, local_fns);
            }
        }
        Stmt::Return(None) | Stmt::StructDecl(..) | Stmt::ClassDecl(..) => {}
    }
}

fn mangle_expr(expr: &mut Expr, namespace: &str, local_fns: &std::collections::HashMap<String, bool>) {
    match expr {
        Expr::Call(name, args, _) => {
            if let Some(&is_exported) = local_fns.get(name) {
                if is_exported {
                    *name = format!("{}__{}", namespace, name);
                } else {
                    *name = format!("__private_{}__{}", namespace, name);
                }
            }
            for arg in args {
                mangle_expr(arg, namespace, local_fns);
            }
        }
        Expr::BinaryOp(left, _, right, _) => {
            mangle_expr(left, namespace, local_fns);
            mangle_expr(right, namespace, local_fns);
        }
        Expr::ArrayLiteral(elems) => {
            for e in elems {
                mangle_expr(e, namespace, local_fns);
            }
        }
        Expr::DictLiteral(pairs) => {
            for (k, v) in pairs {
                mangle_expr(k, namespace, local_fns);
                mangle_expr(v, namespace, local_fns);
            }
        }
        Expr::IndexAccess(arr, idx) => {
            mangle_expr(arr, namespace, local_fns);
            mangle_expr(idx, namespace, local_fns);
        }
        Expr::StructInit(_, fields) => {
            for (_, val) in fields {
                mangle_expr(val, namespace, local_fns);
            }
        }
        Expr::PropertyAccess(obj, _, _) => mangle_expr(obj, namespace, local_fns),
        Expr::MethodCall(obj, _, args, _) => {
            mangle_expr(obj, namespace, local_fns);
            for a in args {
                mangle_expr(a, namespace, local_fns);
            }
        }
        Expr::NewClass(_, args) => {
            for a in args {
                mangle_expr(a, namespace, local_fns);
            }
        }
        Expr::Number(_) | Expr::Boolean(_) | Expr::StringLiteral(_) | Expr::Identifier(_, _) => {}
    }
}
