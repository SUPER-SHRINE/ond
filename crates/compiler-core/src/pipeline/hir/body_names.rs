//! Definition-time lexical name validation for generic bodies.
//!
//! Generic bodies are otherwise typed only after specialization.  Lexical
//! mistakes do not depend on type arguments, so report them at the definition
//! even when no specialization is requested.

use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::ir::ast;
use std::collections::{BTreeMap, BTreeSet};

const BUILTINS: &[&str] = &["alloc", "free", "len", "thisFile", "thisLine"];

pub(super) fn validate_generic_body_names(
    project: &ast::Project,
    all_sources: &ast::Project,
) -> Diagnostics {
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
    let package_types = project
        .packages
        .iter()
        .map(|package| {
            let names = package
                .files
                .iter()
                .flat_map(|file| &file.decls)
                .filter_map(|decl| match decl {
                    ast::TopLevelDecl::Type(decl) => Some(&decl.specs),
                    _ => None,
                })
                .flatten()
                .map(|spec| spec.name.name.clone())
                .collect::<BTreeSet<_>>();
            (package.logical_path.as_str(), names)
        })
        .collect::<BTreeMap<_, _>>();
    // Only names removed by the test visibility context need additional checks.
    // Ordinary unresolved generic types retain their existing specialization rules.
    let hidden_names = all_sources
        .packages
        .iter()
        .map(|package| {
            let all_values = package
                .files
                .iter()
                .flat_map(|file| &file.decls)
                .flat_map(top_level_value_names)
                .collect::<BTreeSet<_>>();
            let all_types = package
                .files
                .iter()
                .flat_map(|file| &file.decls)
                .filter_map(|decl| match decl {
                    ast::TopLevelDecl::Type(decl) => Some(&decl.specs),
                    _ => None,
                })
                .flatten()
                .map(|spec| spec.name.name.clone())
                .collect::<BTreeSet<_>>();
            let hidden_types = all_types
                .difference(&package_types[package.logical_path.as_str()])
                .cloned()
                .collect::<BTreeSet<_>>();
            let mut hidden = all_values
                .difference(&package_values[package.logical_path.as_str()])
                .cloned()
                .collect::<BTreeSet<_>>();
            hidden.extend(hidden_types.iter().cloned());
            (package.logical_path.as_str(), (hidden, hidden_types))
        })
        .collect::<BTreeMap<_, _>>();
    let mut diagnostics = Diagnostics::new();

    for package in &project.packages {
        let values = &package_values[package.logical_path.as_str()];
        let hidden_types = &hidden_names[package.logical_path.as_str()].1;
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
            let imported_names = file
                .imports
                .iter()
                .filter_map(|import| {
                    let logical = crate::string_literal::import_path(&import.path).ok()?;
                    let alias = import
                        .alias
                        .as_ref()
                        .map(|name| name.name.clone())
                        .unwrap_or_else(|| {
                            logical.rsplit('/').next().unwrap_or_default().to_string()
                        });
                    let names = hidden_names.get(logical.as_str())?.0.clone();
                    Some((alias, names))
                })
                .collect::<BTreeMap<String, BTreeSet<String>>>();
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
                        let mut validator = NameValidator::new(
                            values,
                            &imports,
                            hidden_types,
                            &imported_names,
                            &mut diagnostics,
                        );
                        validator
                            .type_parameters
                            .extend(function.type_params.iter().map(|p| p.name.clone()));
                        if let Some(receiver) = function.receiver.as_deref() {
                            if let Some(generic) = super::types::generic_method_receiver(receiver) {
                                validator
                                    .type_parameters
                                    .extend(generic.parameters.iter().map(|p| p.name.clone()));
                            }
                            validator.declare(&receiver.name);
                            validator.ty(&receiver.ty);
                        }
                        for result in &function.signature.results {
                            validator.ty(result);
                        }
                        for parameter in &function.signature.params {
                            validator.ty(&parameter.ty);
                            validator.declare(&parameter.name);
                        }
                        validator.block(&function.body);
                    }
                    ast::TopLevelDecl::Operator(operator) if !operator.type_params.is_empty() => {
                        let mut validator = NameValidator::new(
                            values,
                            &imports,
                            hidden_types,
                            &imported_names,
                            &mut diagnostics,
                        );
                        validator
                            .type_parameters
                            .extend(operator.type_params.iter().map(|p| p.name.clone()));
                        for result in &operator.signature.results {
                            validator.ty(result);
                        }
                        for parameter in &operator.signature.params {
                            validator.ty(&parameter.ty);
                            validator.declare(&parameter.name);
                        }
                        validator.block(&operator.body);
                    }
                    ast::TopLevelDecl::Type(declaration) => {
                        for spec in declaration
                            .specs
                            .iter()
                            .filter(|spec| !spec.type_params.is_empty())
                        {
                            let mut validator = NameValidator::new(
                                values,
                                &imports,
                                hidden_types,
                                &imported_names,
                                &mut diagnostics,
                            );
                            validator
                                .type_parameters
                                .extend(spec.type_params.iter().map(|p| p.name.clone()));
                            validator.ty(&spec.ty);
                        }
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
    hidden_types: &'a BTreeSet<String>,
    imported_names: &'a BTreeMap<String, BTreeSet<String>>,
    type_parameters: BTreeSet<String>,
    scopes: Vec<BTreeSet<String>>,
    diagnostics: &'a mut Diagnostics,
}

impl<'a> NameValidator<'a> {
    fn new(
        package_values: &'a BTreeSet<String>,
        imports: &'a BTreeSet<String>,
        hidden_types: &'a BTreeSet<String>,
        imported_names: &'a BTreeMap<String, BTreeSet<String>>,
        diagnostics: &'a mut Diagnostics,
    ) -> Self {
        Self {
            package_values,
            imports,
            hidden_types,
            imported_names,
            type_parameters: BTreeSet::new(),
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
                    if let Some(ty) = &spec.ty {
                        self.ty(ty);
                    }
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
                    if let Some(ty) = &spec.ty {
                        self.ty(ty);
                    }
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

    fn imported_member(&mut self, alias: &ast::Ident, name: &ast::Ident) {
        if self.scopes.iter().any(|scope| scope.contains(&alias.name)) {
            return;
        }
        if let Some(names) = self.imported_names.get(&alias.name)
            && names.contains(&name.name)
        {
            self.diagnostics.push(Diagnostic::error(
                name.span,
                format!(
                    "unknown or unexported identifier `{}.{}`",
                    alias.name, name.name
                ),
            ));
        }
    }

    fn type_path(&mut self, path: &ast::Path) {
        match path.segments.as_slice() {
            [name]
                if self.hidden_types.contains(&name.name)
                    && !self.type_parameters.contains(&name.name) =>
            {
                self.diagnostics.push(Diagnostic::error(
                    name.span,
                    format!("unknown type `{}`", name.name),
                ));
            }
            [alias, name] => self.imported_member(alias, name),
            _ => {}
        }
    }

    fn ty(&mut self, ty: &ast::Type) {
        match ty {
            ast::Type::Named(path) => self.type_path(path),
            ast::Type::Apply {
                base, arguments, ..
            } => {
                self.type_path(base);
                for ty in arguments {
                    self.ty(ty);
                }
            }
            ast::Type::Pointer(inner, _) => self.ty(inner),
            ast::Type::Array { len, element, .. } => {
                if let Some(len) = len {
                    self.expr(len);
                }
                self.ty(element);
            }
            ast::Type::Struct { fields, .. } => {
                for field in fields {
                    self.ty(&field.ty);
                }
            }
            ast::Type::Interface { methods, .. } => {
                for method in methods {
                    for parameter in &method.signature.params {
                        self.ty(&parameter.ty);
                    }
                    for ty in &method.signature.results {
                        self.ty(ty);
                    }
                }
            }
            ast::Type::Func { signature, .. } => {
                for parameter in &signature.params {
                    self.ty(&parameter.ty);
                }
                for ty in &signature.results {
                    self.ty(ty);
                }
            }
            ast::Type::Resolved(..) => {}
        }
    }

    fn expr(&mut self, expression: &ast::Expr) {
        match expression {
            ast::Expr::Name(path) => {
                if let [alias, name] = path.segments.as_slice() {
                    self.imported_member(alias, name);
                }
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
            ast::Expr::TypeApply {
                base, arguments, ..
            } => {
                self.expr(base);
                for ty in arguments {
                    self.ty(ty);
                }
            }
            ast::Expr::Selector { base, field, .. } => {
                if let ast::Expr::Name(path) = base.as_ref()
                    && let [alias] = path.segments.as_slice()
                {
                    self.imported_member(alias, field);
                }
                self.expr(base);
            }
            ast::Expr::Index { base, index, .. } => {
                self.expr(base);
                self.expr(index);
            }
            ast::Expr::Cast { expr, ty, .. } => {
                self.expr(expr);
                self.ty(ty);
            }
            ast::Expr::Composite { ty, elements, .. } => {
                self.ty(ty);
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
            ast::Expr::LayoutQuery { ty, .. } | ast::Expr::New { ty, .. } => self.ty(ty),
            ast::Expr::Literal(_) => {}
        }
    }
}
