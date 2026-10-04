//! Definition-time lexical name validation for generic bodies.
//!
//! Generic bodies are otherwise typed only after specialization.  Lexical
//! mistakes do not depend on type arguments, so report them at the definition
//! even when no specialization is requested.

use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::ir::ast;
use std::collections::{BTreeMap, BTreeSet};

const BUILTINS: &[&str] = &[
    "alloc", "free", "len", "load32", "store32", "thisFile", "thisLine",
];

pub(super) fn validate_generic_body_names(project: &ast::Project) -> Diagnostics {
    let package_values = project
        .packages
        .iter()
        .map(|package| {
            let values = package
                .files
                .iter()
                .flat_map(|file| &file.decls)
                .flat_map(top_level_value_names)
                .collect::<BTreeSet<_>>();
            (package.logical_path.as_str(), values)
        })
        .collect::<BTreeMap<_, _>>();
    let mut diagnostics = Diagnostics::new();

    for package in &project.packages {
        let values = &package_values[package.logical_path.as_str()];
        for file in &package.files {
            let imports = file
                .imports
                .iter()
                .filter_map(|import| {
                    let logical = crate::string_literal::import_path(&import.path).ok()?;
                    Some(
                        import
                            .alias
                            .as_ref()
                            .map(|alias| alias.name.clone())
                            .unwrap_or_else(|| {
                                logical.rsplit('/').next().unwrap_or_default().to_string()
                            }),
                    )
                })
                .collect::<BTreeSet<_>>();
            for declaration in &file.decls {
                match declaration {
                    ast::TopLevelDecl::Func(function)
                        if !function.type_params.is_empty()
                            || function
                                .receiver
                                .as_deref()
                                .and_then(super::types::generic_method_receiver)
                                .is_some() =>
                    {
                        let mut validator = NameValidator::new(values, &imports, &mut diagnostics);
                        if let Some(receiver) = function.receiver.as_deref() {
                            validator.declare(&receiver.name);
                        }
                        for parameter in &function.signature.params {
                            validator.declare(&parameter.name);
                        }
                        validator.block(&function.body);
                    }
                    ast::TopLevelDecl::Operator(operator) if !operator.type_params.is_empty() => {
                        let mut validator = NameValidator::new(values, &imports, &mut diagnostics);
                        for parameter in &operator.signature.params {
                            validator.declare(&parameter.name);
                        }
                        validator.block(&operator.body);
                    }
                    _ => {}
                }
            }
        }
    }
    diagnostics
}

fn top_level_value_names(declaration: &ast::TopLevelDecl) -> Vec<String> {
    match declaration {
        ast::TopLevelDecl::Const(declaration) => declaration
            .specs
            .iter()
            .flat_map(|spec| &spec.names)
            .map(|name| name.name.clone())
            .collect(),
        ast::TopLevelDecl::Var(declaration) => declaration
            .specs
            .iter()
            .flat_map(|spec| &spec.names)
            .map(|name| name.name.clone())
            .collect(),
        ast::TopLevelDecl::Func(function) if function.receiver.is_none() => {
            vec![function.name.name.clone()]
        }
        _ => Vec::new(),
    }
}

struct NameValidator<'a> {
    package_values: &'a BTreeSet<String>,
    imports: &'a BTreeSet<String>,
    scopes: Vec<BTreeSet<String>>,
    diagnostics: &'a mut Diagnostics,
}

impl<'a> NameValidator<'a> {
    fn new(
        package_values: &'a BTreeSet<String>,
        imports: &'a BTreeSet<String>,
        diagnostics: &'a mut Diagnostics,
    ) -> Self {
        Self {
            package_values,
            imports,
            scopes: vec![BTreeSet::new()],
            diagnostics,
        }
    }

    fn declare(&mut self, name: &ast::Ident) {
        if name.name != "_" {
            self.scopes
                .last_mut()
                .expect("name validator always has a scope")
                .insert(name.name.clone());
        }
    }

    fn known(&self, name: &str) -> bool {
        name == "_"
            || BUILTINS.contains(&name)
            || self.package_values.contains(name)
            || self.imports.contains(name)
            || self.scopes.iter().rev().any(|scope| scope.contains(name))
    }

    fn block(&mut self, block: &ast::Block) {
        self.scopes.push(BTreeSet::new());
        for statement in &block.statements {
            self.statement(statement);
        }
        self.scopes.pop();
    }

    fn statement(&mut self, statement: &ast::Stmt) {
        match statement {
            ast::Stmt::Const(declaration) => {
                for spec in &declaration.specs {
                    for value in &spec.values {
                        self.expr(value);
                    }
                    for name in &spec.names {
                        self.declare(name);
                    }
                }
            }
            ast::Stmt::Var(declaration) => {
                for spec in &declaration.specs {
                    for value in &spec.values {
                        self.expr(value);
                    }
                    for name in &spec.names {
                        self.declare(name);
                    }
                }
            }
            ast::Stmt::ShortVar(declaration) => {
                for value in &declaration.values {
                    self.expr(value);
                }
                for name in &declaration.names {
                    self.declare(name);
                }
            }
            ast::Stmt::Assign(assignment) => {
                for target in &assignment.targets {
                    self.expr(target);
                }
                for value in &assignment.values {
                    self.expr(value);
                }
            }
            ast::Stmt::Expr(statement) => self.expr(&statement.expr),
            ast::Stmt::Return(statement) => {
                for value in &statement.values {
                    self.expr(value);
                }
            }
            ast::Stmt::Defer(statement) => self.block(&statement.block),
            ast::Stmt::Block(block) => self.block(block),
            ast::Stmt::If(statement) => {
                self.scopes.push(BTreeSet::new());
                if let Some(init) = statement.init.as_deref() {
                    self.statement(init);
                }
                self.expr(&statement.condition);
                self.block(&statement.then_block);
                if let Some(branch) = statement.else_branch.as_deref() {
                    self.statement(branch);
                }
                self.scopes.pop();
            }
            ast::Stmt::For(statement) => {
                self.scopes.push(BTreeSet::new());
                match &statement.kind {
                    ast::ForKind::Infinite => {}
                    ast::ForKind::While(condition) => self.expr(condition),
                    ast::ForKind::ThreeClause {
                        init,
                        condition,
                        post,
                    } => {
                        if let Some(init) = init.as_deref() {
                            self.statement(init);
                        }
                        if let Some(condition) = condition {
                            self.expr(condition);
                        }
                        if let Some(post) = post.as_deref() {
                            self.statement(post);
                        }
                    }
                }
                self.block(&statement.body);
                self.scopes.pop();
            }
            ast::Stmt::Trap { .. } | ast::Stmt::Break(_) | ast::Stmt::Continue(_) => {}
        }
    }

    fn expr(&mut self, expression: &ast::Expr) {
        match expression {
            ast::Expr::Name(path) => {
                if let Some(name) = path.segments.first()
                    && !self.known(&name.name)
                {
                    self.diagnostics.push(Diagnostic::error(
                        name.span,
                        format!("unknown identifier `{}`", name.name),
                    ));
                }
            }
            ast::Expr::Unary { expr, .. } => self.expr(expr),
            ast::Expr::Binary { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            ast::Expr::Call { callee, args, .. } => {
                self.expr(callee);
                for argument in args {
                    self.expr(argument);
                }
            }
            ast::Expr::TypeApply { base, .. } => self.expr(base),
            ast::Expr::Selector { base, .. } => self.expr(base),
            ast::Expr::Index { base, index, .. } => {
                self.expr(base);
                self.expr(index);
            }
            ast::Expr::Cast { expr, .. } => self.expr(expr),
            ast::Expr::Composite { elements, .. } => {
                for element in elements {
                    // A bare keyed name can be a struct field rather than a value.
                    if !matches!(element.key, Some(ast::Expr::Name(_)))
                        && let Some(key) = &element.key
                    {
                        self.expr(key);
                    }
                    self.expr(&element.value);
                }
            }
            ast::Expr::Literal(_) | ast::Expr::LayoutQuery { .. } | ast::Expr::New { .. } => {}
        }
    }
}
