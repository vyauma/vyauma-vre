use crate::vir::{Module, Function, BasicBlock, Instruction, Value};
use oxc_ast::ast::{Program, Statement, Declaration, Expression, BinaryOperator, ImportDeclaration, ExportNamedDeclaration};
use std::collections::{HashMap, HashSet};

/// A scope in the lexical scope chain.
#[derive(Debug, Clone)]
struct Scope {
    /// Variable name -> whether it has been boxed (captured by an inner closure)
    vars: HashMap<String, bool>,
}

impl Scope {
    fn new() -> Self {
        Scope { vars: HashMap::new() }
    }
    fn define(&mut self, name: &str) {
        self.vars.entry(name.to_string()).or_insert(false);
    }
    fn has(&self, name: &str) -> bool {
        self.vars.contains_key(name)
    }
}

pub struct Lowerer {
    module: Module,
    blocks: Vec<BasicBlock>,
    current_block: usize,
    next_value: Value,
    /// Maps captured variable name -> upvalue index in current closure.
    upvalue_map: HashMap<String, usize>,
    /// Maps captured name -> the VIR Value holding the Box reference in outer scope.
    captured_boxes: HashMap<String, Value>,
    /// Lexical scope stack (innermost last)
    scopes: Vec<Scope>,
    /// Whether we are currently inside a closure
    inside_closure: bool,
}

impl Lowerer {
    pub fn new() -> Self {
        Self {
            module: Module { functions: Vec::new() },
            blocks: Vec::new(),
            current_block: 0,
            next_value: 0,
            upvalue_map: HashMap::new(),
            captured_boxes: HashMap::new(),
            scopes: Vec::new(),
            inside_closure: false,
        }
    }

    fn next_val(&mut self) -> Value {
        let val = self.next_value;
        self.next_value += 1;
        val
    }

    fn new_block(&mut self) -> usize {
        let id = self.blocks.len();
        self.blocks.push(BasicBlock { id, instructions: Vec::new() });
        id
    }

    fn add_inst(&mut self, inst: Instruction) -> Value {
        let val = self.next_val();
        self.blocks[self.current_block].instructions.push((val, inst));
        val
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    fn pop_scope(&mut self) -> Option<Scope> {
        self.scopes.pop()
    }

    fn define_var(&mut self, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.define(name);
        }
    }

    /// Check if a variable is defined in any enclosing scope.
    fn find_in_outer_scopes(&self, name: &str) -> bool {
        if self.scopes.len() < 2 { return false; }
        for depth in 1..self.scopes.len() {
            let scope_idx = self.scopes.len() - 1 - depth;
            if self.scopes[scope_idx].has(name) {
                return true;
            }
        }
        false
    }

    fn is_local_var(&self, name: &str) -> bool {
        self.scopes.iter().any(|s| s.has(name)) || self.upvalue_map.contains_key(name)
    }

    pub fn lower_program(mut self, program: &Program<'_>) -> Result<Module, String> {
        self.blocks = vec![BasicBlock { id: 0, instructions: Vec::new() }];
        self.current_block = 0;
        self.next_value = 0;
        self.push_scope();

        for stmt in &program.body {
            self.lower_stmt(stmt)?;
        }

        self.add_inst(Instruction::Return(None));
        self.pop_scope();

        let main_func = Function {
            name: "@main".to_string(),
            params: Vec::new(),
            entry_block: 0,
            blocks: self.blocks.clone(),
        };
        self.module.functions.push(main_func);

        Ok(self.module)
    }

    fn lower_variable_declaration(&mut self, var_decl: &oxc_ast::ast::VariableDeclaration<'_>) -> Result<(), String> {
        for decl in &var_decl.declarations {
            let name = decl.id.get_binding_identifier()
                .map(|id| id.name.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            self.define_var(&name);
            if let Some(init) = &decl.init {
                let val = self.lower_expr(init)?;
                self.add_inst(Instruction::StoreVar(name.clone(), val));
            }
        }
        Ok(())
    }

    fn lower_stmt(&mut self, stmt: &Statement<'_>) -> Result<(), String> {
        match stmt {
            Statement::VariableDeclaration(var_decl) => {
                self.lower_variable_declaration(var_decl)?;
            }
            Statement::FunctionDeclaration(func_decl) => {
                self.lower_function_declaration(func_decl)?;
            }
            Statement::ExpressionStatement(expr_stmt) => {
                self.lower_expr(&expr_stmt.expression)?;
            }
            Statement::ReturnStatement(ret_stmt) => {
                let val = if let Some(arg) = &ret_stmt.argument {
                    Some(self.lower_expr(arg)?)
                } else {
                    None
                };
                self.add_inst(Instruction::Return(val));
            }
            Statement::IfStatement(if_stmt) => {
                let cond_val = self.lower_expr(&if_stmt.test)?;
                let cons_block = self.new_block();
                let alt_block = if if_stmt.alternate.is_some() { Some(self.new_block()) } else { None };
                let merge_block = self.new_block();

                self.add_inst(Instruction::CondBranch(cond_val, cons_block, alt_block.unwrap_or(merge_block)));

                self.current_block = cons_block;
                self.lower_stmt(&if_stmt.consequent)?;
                self.add_inst(Instruction::Branch(merge_block));

                if let Some(alt) = &if_stmt.alternate {
                    self.current_block = alt_block.unwrap();
                    self.lower_stmt(alt)?;
                    self.add_inst(Instruction::Branch(merge_block));
                }

                self.current_block = merge_block;
            }
            Statement::WhileStatement(while_stmt) => {
                let cond_block = self.new_block();
                let body_block = self.new_block();
                let end_block = self.new_block();

                self.add_inst(Instruction::Branch(cond_block));
                self.current_block = cond_block;
                let cond_val = self.lower_expr(&while_stmt.test)?;
                self.add_inst(Instruction::CondBranch(cond_val, body_block, end_block));

                self.current_block = body_block;
                self.lower_stmt(&while_stmt.body)?;
                self.add_inst(Instruction::Branch(cond_block));

                self.current_block = end_block;
            }
            Statement::ForStatement(for_stmt) => {
                if let Some(init) = &for_stmt.init {
                    if let oxc_ast::ast::ForStatementInit::VariableDeclaration(var_decl) = init {
                        self.lower_variable_declaration(var_decl)?;
                    } else if let Some(expr) = init.as_expression() {
                        self.lower_expr(expr)?;
                    }
                }

                let cond_block = self.new_block();
                let body_block = self.new_block();
                let end_block = self.new_block();

                self.add_inst(Instruction::Branch(cond_block));
                self.current_block = cond_block;
                if let Some(test) = &for_stmt.test {
                    let cond_val = self.lower_expr(test)?;
                    self.add_inst(Instruction::CondBranch(cond_val, body_block, end_block));
                } else {
                    self.add_inst(Instruction::Branch(body_block));
                }

                self.current_block = body_block;
                self.lower_stmt(&for_stmt.body)?;

                if let Some(update) = &for_stmt.update {
                    self.lower_expr(update)?;
                }
                self.add_inst(Instruction::Branch(cond_block));

                self.current_block = end_block;
            }
            Statement::BlockStatement(block_stmt) => {
                self.push_scope();
                for s in &block_stmt.body {
                    self.lower_stmt(s)?;
                }
                self.pop_scope();
            }
            Statement::ImportDeclaration(import_decl) => {
                let source = import_decl.source.value.to_string();
                let mod_val = self.add_inst(Instruction::ImportModule(source));

                if let Some(specifiers) = &import_decl.specifiers {
                    for spec in specifiers {
                        if let oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(ispec) = spec {
                            let imported_name = ispec.imported.name().to_string();
                            let local_name = ispec.local.name.to_string();
                            
                            let prop_val = self.add_inst(Instruction::PropertyAccess(mod_val, imported_name));
                            self.define_var(&local_name);
                            self.add_inst(Instruction::StoreVar(local_name, prop_val));
                        }
                    }
                }
            }
            Statement::ExportNamedDeclaration(export_decl) => {
                // If it has a declaration (e.g., `export function foo() {}`)
                if let Some(decl) = &export_decl.declaration {
                    match decl {
                        Declaration::FunctionDeclaration(func_decl) => {
                            self.lower_function_declaration(func_decl)?;
                            // Export it
                            if let Some(id) = &func_decl.id {
                                let name = id.name.to_string();
                                let val = self.add_inst(Instruction::LoadVar(name.clone()));
                                self.add_inst(Instruction::ExportValue(name, val));
                            }
                        }
                        Declaration::VariableDeclaration(var_decl) => {
                            self.lower_variable_declaration(var_decl)?;
                            for d in &var_decl.declarations {
                                if let Some(id) = d.id.get_binding_identifier() {
                                    let name = id.name.to_string();
                                    let val = self.add_inst(Instruction::LoadVar(name.clone()));
                                    self.add_inst(Instruction::ExportValue(name, val));
                                }
                            }
                        }
                        _ => {}
                    }
                }

                // If it exports specific named specifiers (e.g., `export { foo };`)
                for spec in &export_decl.specifiers {
                    let local_name = spec.local.name().to_string();
                    let exported_name = spec.exported.name().to_string();
                    let val = self.add_inst(Instruction::LoadVar(local_name));
                    self.add_inst(Instruction::ExportValue(exported_name, val));
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn lower_function_declaration(&mut self, func_decl: &oxc_ast::ast::Function<'_>) -> Result<(), String> {
        let name = func_decl.id.as_ref()
            .map(|id| id.name.to_string())
            .unwrap_or_else(|| "anonymous".to_string());

        // Save current state
        let old_blocks = std::mem::take(&mut self.blocks);
        let old_current_block = self.current_block;
        let old_next_value = self.next_value;
        let old_upvalue_map = std::mem::take(&mut self.upvalue_map);
        let old_captured_boxes = std::mem::take(&mut self.captured_boxes);

        // Start new function
        self.blocks = vec![BasicBlock { id: 0, instructions: Vec::new() }];
        self.current_block = 0;
        self.next_value = 0;
        self.push_scope();

        let mut params = Vec::new();
        for param in &func_decl.params.items {
            if let Some(ident) = param.pattern.get_binding_identifier() {
                let p = ident.name.to_string();
                self.define_var(&p);
                params.push(p);
            }
        }

        if let Some(body) = &func_decl.body {
            for s in &body.statements {
                self.lower_stmt(s)?;
            }
        }

        // Ensure return
        if let Some(last_block) = self.blocks.last() {
            let has_ret = last_block.instructions.last()
                .map_or(false, |(_, inst)| matches!(inst, Instruction::Return(_)));
            if !has_ret {
                self.add_inst(Instruction::Return(None));
            }
        }

        self.pop_scope();

        let func = Function {
            name: name.clone(),
            params,
            entry_block: 0,
            blocks: std::mem::take(&mut self.blocks),
        };
        self.module.functions.push(func);

        // Restore state
        self.blocks = old_blocks;
        self.current_block = old_current_block;
        self.next_value = old_next_value;
        self.upvalue_map = old_upvalue_map;
        self.captured_boxes = old_captured_boxes;
        
        let closure_val = self.add_inst(Instruction::NewClosure(name.clone(), vec![]));
        self.define_var(&name);
        self.add_inst(Instruction::StoreVar(name, closure_val));

        Ok(())
    }

    fn lower_expr(&mut self, expr: &Expression<'_>) -> Result<Value, String> {
        match expr {
            Expression::NumericLiteral(lit) => {
                Ok(self.add_inst(Instruction::LoadConstNumber(lit.value)))
            }
            Expression::StringLiteral(lit) => {
                Ok(self.add_inst(Instruction::LoadConstString(lit.value.to_string())))
            }
            Expression::BooleanLiteral(lit) => {
                Ok(self.add_inst(Instruction::LoadConstBool(lit.value)))
            }
            Expression::Identifier(ident) => {
                let name = ident.name.to_string();
                if let Some(&upval_idx) = self.upvalue_map.get(&name) {
                    return Ok(self.add_inst(Instruction::LoadUpvalue(upval_idx)));
                }
                Ok(self.add_inst(Instruction::LoadVar(name)))
            }
            Expression::BinaryExpression(bin_expr) => {
                let left = self.lower_expr(&bin_expr.left)?;
                let right = self.lower_expr(&bin_expr.right)?;
                let inst = match bin_expr.operator {
                    BinaryOperator::Addition => Instruction::Add(left, right),
                    BinaryOperator::Subtraction => Instruction::Sub(left, right),
                    BinaryOperator::Multiplication => Instruction::Mul(left, right),
                    BinaryOperator::Division => Instruction::Div(left, right),
                    BinaryOperator::LessThan => Instruction::Lt(left, right),
                    BinaryOperator::LessEqualThan => Instruction::Lte(left, right),
                    BinaryOperator::GreaterThan => Instruction::Gt(left, right),
                    BinaryOperator::GreaterEqualThan => Instruction::Gte(left, right),
                    BinaryOperator::Equality => Instruction::Eq(left, right),
                    BinaryOperator::Inequality => Instruction::NotEq(left, right),
                    _ => return Err(format!("Unsupported binary operator: {:?}", bin_expr.operator)),
                };
                Ok(self.add_inst(inst))
            }
            Expression::ArrayExpression(arr_expr) => {
                let mut elements = Vec::new();
                for el in &arr_expr.elements {
                    if let Some(expr) = el.as_expression() {
                        elements.push(self.lower_expr(expr)?);
                    } else {
                        return Err("Spread in arrays not yet supported".to_string());
                    }
                }
                Ok(self.add_inst(Instruction::ArrayLiteral(elements)))
            }
            Expression::ObjectExpression(obj_expr) => {
                let mut fields = Vec::new();
                for prop in &obj_expr.properties {
                    if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(obj_prop) = prop {
                        let key_name = match &obj_prop.key {
                            oxc_ast::ast::PropertyKey::Identifier(ident) => ident.name.to_string(),
                            oxc_ast::ast::PropertyKey::StringLiteral(str_lit) => str_lit.value.to_string(),
                            _ => return Err("Unsupported object property key".to_string()),
                        };
                        let val = self.lower_expr(&obj_prop.value)?;
                        fields.push((key_name, val));
                    } else {
                        return Err("Unsupported object property kind".to_string());
                    }
                }
                Ok(self.add_inst(Instruction::StructInit(String::new(), fields)))
            }
            Expression::StaticMemberExpression(member_expr) => {
                let obj = self.lower_expr(&member_expr.object)?;
                let prop_name = member_expr.property.name.to_string();
                Ok(self.add_inst(Instruction::PropertyAccess(obj, prop_name)))
            }
            Expression::ComputedMemberExpression(member_expr) => {
                let obj = self.lower_expr(&member_expr.object)?;
                let expr = self.lower_expr(&member_expr.expression)?;
                Ok(self.add_inst(Instruction::IndexAccess(obj, expr)))
            }
            Expression::AssignmentExpression(assign_expr) => {
                let right = self.lower_expr(&assign_expr.right)?;
                if assign_expr.operator != oxc_ast::ast::AssignmentOperator::Assign {
                    return Err(format!("Unsupported assignment operator: {:?}", assign_expr.operator));
                }

                match &assign_expr.left {
                    oxc_ast::ast::AssignmentTarget::AssignmentTargetIdentifier(ident) => {
                        let name = ident.name.to_string();
                        if let Some(&upval_idx) = self.upvalue_map.get(&name) {
                            self.add_inst(Instruction::StoreUpvalue(upval_idx, right));
                            return Ok(right);
                        }
                        self.add_inst(Instruction::StoreVar(name, right));
                        Ok(right)
                    }
                    oxc_ast::ast::AssignmentTarget::StaticMemberExpression(member_expr) => {
                        let obj = self.lower_expr(&member_expr.object)?;
                        let prop_name = member_expr.property.name.to_string();
                        self.add_inst(Instruction::AssignProperty(obj, prop_name, right));
                        Ok(right)
                    }
                    oxc_ast::ast::AssignmentTarget::ComputedMemberExpression(member_expr) => {
                        let obj = self.lower_expr(&member_expr.object)?;
                        let expr = self.lower_expr(&member_expr.expression)?;
                        self.add_inst(Instruction::AssignIndex(obj, expr, right));
                        Ok(right)
                    }
                    _ => Err("Unsupported assignment target".to_string()),
                }
            }
            Expression::ArrowFunctionExpression(arrow_func) => {
                // ArrowFunctionBody is always a FunctionBody block in OXC
                self.lower_function_expression(
                    None,
                    &arrow_func.params.items,
                    Some(arrow_func.body.statements.as_slice()),
                    None,
                )
            }
            Expression::FunctionExpression(func_expr) => {
                let name = func_expr.id.as_ref().map(|id| id.name.to_string());
                let body_stmts = func_expr.body.as_ref().map(|b| b.statements.as_slice());
                self.lower_function_expression(
                    name.as_deref(),
                    &func_expr.params.items,
                    body_stmts,
                    None,
                )
            }
            Expression::CallExpression(call_expr) => {
                let mut args = Vec::new();
                for arg in &call_expr.arguments {
                    if let Some(expr) = arg.as_expression() {
                        args.push(self.lower_expr(expr)?);
                    }
                }
                match &call_expr.callee {
                    Expression::Identifier(ident) => {
                        let name = ident.name.to_string();
                        if name == "vre_spawn" {
                            if args.len() != 1 {
                                return Err("vre_spawn expects exactly 1 argument (the closure)".to_string());
                            }
                            let callee_val = args.pop().unwrap();
                            Ok(self.add_inst(Instruction::SpawnDynamicTask(callee_val)))
                        } else if self.is_local_var(&name) {
                            let callee_val = self.lower_expr(&call_expr.callee)?;
                            Ok(self.add_inst(Instruction::CallDynamic(callee_val, args)))
                        } else {
                            Ok(self.add_inst(Instruction::Call(name, args)))
                        }
                    }
                    Expression::StaticMemberExpression(member_expr) => {
                        let obj = self.lower_expr(&member_expr.object)?;
                        let prop_name = member_expr.property.name.to_string();
                        Ok(self.add_inst(Instruction::MethodCall(obj, prop_name, args)))
                    }
                    _ => {
                        // Dynamic/closure call
                        let callee_val = self.lower_expr(&call_expr.callee)?;
                        Ok(self.add_inst(Instruction::CallDynamic(callee_val, args)))
                    }
                }
            }
            _ => Err("Unsupported expression".to_string()),
        }
    }

    // ── Free variable collection ─────────────────────────────────────────────

    fn collect_free_vars_expr<'a>(
        &self, expr: &Expression<'a>,
        local_params: &HashSet<String>,
        found: &mut HashSet<String>,
    ) {
        match expr {
            Expression::Identifier(ident) => {
                let name = ident.name.to_string();
                if !local_params.contains(&name) { found.insert(name); }
            }
            Expression::BinaryExpression(b) => {
                self.collect_free_vars_expr(&b.left, local_params, found);
                self.collect_free_vars_expr(&b.right, local_params, found);
            }
            Expression::AssignmentExpression(a) => {
                self.collect_free_vars_expr(&a.right, local_params, found);
                // The left side may be an identifier — extract it
                if let oxc_ast::ast::AssignmentTarget::AssignmentTargetIdentifier(ident) = &a.left {
                    let name = ident.name.to_string();
                    if !local_params.contains(&name) { found.insert(name); }
                }
            }
            Expression::CallExpression(c) => {
                self.collect_free_vars_expr(&c.callee, local_params, found);
                for arg in &c.arguments {
                    if let Some(e) = arg.as_expression() {
                        self.collect_free_vars_expr(e, local_params, found);
                    }
                }
            }
            Expression::StaticMemberExpression(m) => {
                self.collect_free_vars_expr(&m.object, local_params, found);
            }
            Expression::ComputedMemberExpression(m) => {
                self.collect_free_vars_expr(&m.object, local_params, found);
                self.collect_free_vars_expr(&m.expression, local_params, found);
            }
            _ => {}
        }
    }

    fn collect_free_vars_stmt<'a>(
        &self, stmt: &Statement<'a>,
        local_params: &HashSet<String>,
        found: &mut HashSet<String>,
    ) {
        match stmt {
            Statement::ExpressionStatement(e) =>
                self.collect_free_vars_expr(&e.expression, local_params, found),
            Statement::ReturnStatement(r) => {
                if let Some(arg) = &r.argument {
                    self.collect_free_vars_expr(arg, local_params, found);
                }
            }
            Statement::VariableDeclaration(v) => {
                for decl in &v.declarations {
                    if let Some(init) = &decl.init {
                        self.collect_free_vars_expr(init, local_params, found);
                    }
                }
            }
            Statement::BlockStatement(b) => {
                for s in &b.body { self.collect_free_vars_stmt(s, local_params, found); }
            }
            Statement::IfStatement(i) => {
                self.collect_free_vars_expr(&i.test, local_params, found);
                self.collect_free_vars_stmt(&i.consequent, local_params, found);
                if let Some(alt) = &i.alternate {
                    self.collect_free_vars_stmt(alt, local_params, found);
                }
            }
            Statement::WhileStatement(w) => {
                self.collect_free_vars_expr(&w.test, local_params, found);
                self.collect_free_vars_stmt(&w.body, local_params, found);
            }
            _ => {}
        }
    }

    // ── Function expression / closure lowering ────────────────────────────────

    fn lower_function_expression<'a>(
        &mut self,
        name: Option<&str>,
        params: &[oxc_ast::ast::FormalParameter<'a>],
        body_stmts: Option<&[Statement<'a>]>,
        arrow_body_exprs: Option<&[&Expression<'a>]>,
    ) -> Result<Value, String> {
        // 1. Collect free variables referenced inside the closure body
        let inner_params: HashSet<String> = params.iter()
            .filter_map(|p| p.pattern.get_binding_identifier().map(|id| id.name.to_string()))
            .collect();

        let mut free_vars: HashSet<String> = HashSet::new();
        if let Some(stmts) = body_stmts {
            for s in stmts {
                self.collect_free_vars_stmt(s, &inner_params, &mut free_vars);
            }
        }
        if let Some(exprs) = arrow_body_exprs {
            for e in exprs {
                self.collect_free_vars_expr(e, &inner_params, &mut free_vars);
            }
        }

        // 2. Determine which free vars are from an enclosing scope
        let captured_names: Vec<String> = free_vars.into_iter()
            .filter(|n| self.is_local_var(n) || self.captured_boxes.contains_key(n))
            .collect();

        // 3. Box any un-boxed captured variables in the current scope
        let mut cap_box_vals: Vec<(String, Value)> = Vec::new();
        for cap_name in &captured_names {
            if let Some(&existing_box) = self.captured_boxes.get(cap_name) {
                cap_box_vals.push((cap_name.clone(), existing_box));
            } else {
                let loaded = self.add_inst(Instruction::LoadVar(cap_name.clone()));
                let boxed  = self.add_inst(Instruction::BoxValue(loaded));
                self.captured_boxes.insert(cap_name.clone(), boxed);
                cap_box_vals.push((cap_name.clone(), boxed));
            }
        }

        // 4. Save outer state, start inner function compilation
        let old_blocks        = std::mem::take(&mut self.blocks);
        let old_current_block = self.current_block;
        let old_next_value    = self.next_value;
        let old_upvalue_map   = std::mem::take(&mut self.upvalue_map);
        let old_cap_boxes     = std::mem::take(&mut self.captured_boxes);
        let old_inside        = self.inside_closure;

        let mut inner_upvalue_map: HashMap<String, usize> = HashMap::new();
        for (idx, (cap_name, _)) in cap_box_vals.iter().enumerate() {
            inner_upvalue_map.insert(cap_name.clone(), idx);
        }

        self.blocks        = vec![BasicBlock { id: 0, instructions: Vec::new() }];
        self.current_block = 0;
        self.next_value    = 0;
        self.upvalue_map   = inner_upvalue_map;
        self.captured_boxes = HashMap::new();
        self.inside_closure = true;
        self.push_scope();

        let mut param_names = Vec::new();
        for param in params {
            if let Some(ident) = param.pattern.get_binding_identifier() {
                let p = ident.name.to_string();
                self.define_var(&p);
                param_names.push(p);
            }
        }

        if let Some(stmts) = body_stmts {
            for s in stmts { self.lower_stmt(s)?; }
        }
        if let Some(exprs) = arrow_body_exprs {
            for e in exprs {
                let v = self.lower_expr(e)?;
                self.add_inst(Instruction::Return(Some(v)));
            }
        }

        // Ensure trailing return
        if let Some(last_block) = self.blocks.last() {
            let has_ret = last_block.instructions.last()
                .map_or(false, |(_, inst)| matches!(inst, Instruction::Return(_)));
            if !has_ret { self.add_inst(Instruction::Return(None)); }
        }

        self.pop_scope();

        let func_name = name.map(|n| n.to_string())
            .unwrap_or_else(|| format!("__closure_{}", self.module.functions.len()));

        let func = Function {
            name: func_name.clone(),
            params: param_names,
            entry_block: 0,
            blocks: std::mem::take(&mut self.blocks),
        };
        self.module.functions.push(func);

        // 5. Restore outer state
        self.blocks         = old_blocks;
        self.current_block  = old_current_block;
        self.next_value     = old_next_value;
        self.upvalue_map    = old_upvalue_map;
        self.captured_boxes = old_cap_boxes;
        self.inside_closure = old_inside;

        // 6. Emit NewClosure with box references
        let cap_vals: Vec<Value> = cap_box_vals.iter().map(|(_, v)| *v).collect();
        Ok(self.add_inst(Instruction::NewClosure(func_name, cap_vals)))
    }
}
