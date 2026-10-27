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
    loaded: &crate::project::LoadedProject,
    project: &ast::Project,
) -> Diagnostics {
    let names = |types: bool, normal_only: bool| {
        project
            .packages
            .iter()
            .map(|package| {
                let names = package
                    .files
                    .iter()
                    .filter(|file| {
                        !normal_only
                            || !crate::project::is_test_source(
                                loaded.sources.file(file.file_id).path(),
                            )
                    })
                    .flat_map(|file| &file.decls)
                    .flat_map(|decl| {
                        if types {
                            match decl {
                                ast::TopLevelDecl::Type(decl) => decl
                                    .specs
                                    .iter()
                                    .map(|spec| spec.name.name.clone())
                                    .collect(),
                                _ => Vec::new(),
                            }
                        } else {
                            top_level_value_names(decl)
                        }
                    })
                    .collect::<BTreeSet<_>>();
                (package.logical_path.clone(), names)
            })
            .collect::<BTreeMap<_, _>>()
    };
    let all_values = names(false, false);
    let normal_values = names(false, true);
    let all_types = names(true, false);
    let normal_types = names(true, true);
    let hidden_names = project
        .packages
        .iter()
        .map(|package| {
            let path = &package.logical_path;
            let hidden_types = all_types[path]
                .difference(&normal_types[path])
                .cloned()
                .collect::<BTreeSet<_>>();
            let mut hidden = all_values[path]
                .difference(&normal_values[path])
                .cloned()
                .collect::<BTreeSet<_>>();
            hidden.extend(hidden_types.iter().cloned());
            (path.as_str(), (hidden, hidden_types))
        })
        .collect::<BTreeMap<_, _>>();
    let empty = BTreeSet::new();
    let mut diagnostics = Diagnostics::new();
    for package in &project.packages {
        for file in &package.files {
            let role_context = RoleNameContext::new(&loaded.sources, project, package, file);
            let test = crate::project::is_test_source(loaded.sources.file(file.file_id).path());
            let values = if test {
                &all_values[&package.logical_path]
            } else {
                &normal_values[&package.logical_path]
            };
            let hidden_types = if test {
                &empty
            } else {
                &hidden_names[package.logical_path.as_str()].1
            };
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
            let mut generic_functions = package
                .files
                .iter()
                .flat_map(|file| &file.decls)
                .filter_map(|decl| match decl {
                    ast::TopLevelDecl::Func(function)
                        if function.receiver.is_none() && !function.type_params.is_empty() =>
                    {
                        Some(vec![function.name.name.clone()])
                    }
                    _ => None,
                })
                .collect::<BTreeSet<_>>();
            for import in &file.imports {
                let Ok(logical) = crate::string_literal::import_path(&import.path) else {
                    continue;
                };
                let alias = import
                    .alias
                    .as_ref()
                    .map(|alias| alias.name.clone())
                    .unwrap_or_else(|| logical.rsplit('/').next().unwrap_or_default().into());
                if let Some(imported) = project
                    .packages
                    .iter()
                    .find(|package| package.logical_path == logical)
                {
                    for decl in imported.files.iter().flat_map(|file| &file.decls) {
                        if let ast::TopLevelDecl::Func(function) = decl
                            && function.receiver.is_none()
                            && !function.type_params.is_empty()
                        {
                            generic_functions
                                .insert(vec![alias.clone(), function.name.name.clone()]);
                        }
                    }
                }
            }
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
                            &generic_functions,
                            &role_context,
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
                            validator.declare(&receiver.name, Binding::Variable);
                            validator.ty(&receiver.ty);
                        }
                        for result in &function.signature.results {
                            validator.ty(result);
                        }
                        for parameter in &function.signature.params {
                            validator.ty(&parameter.ty);
                            validator.declare(&parameter.name, Binding::Variable);
                        }
                        validator.body(&function.body);
                    }
                    ast::TopLevelDecl::Operator(operator) if !operator.type_params.is_empty() => {
                        let mut validator = NameValidator::new(
                            values,
                            &imports,
                            hidden_types,
                            &imported_names,
                            &generic_functions,
                            &role_context,
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
                            validator.declare(&parameter.name, Binding::Variable);
                        }
                        validator.body(&operator.body);
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
                                &generic_functions,
                                &role_context,
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Binding {
    Variable,
    Constant,
}

struct RoleNameContext<'a> {
    sources: &'a crate::source::SourceDb,
    declarations: BTreeMap<String, Vec<crate::source::Span>>,
}
impl<'a> RoleNameContext<'a> {
    fn new(
        sources: &'a crate::source::SourceDb,
        project: &ast::Project,
        package: &ast::Package,
        file: &ast::File,
    ) -> Self {
        let mut context = Self {
            sources,
            declarations: BTreeMap::new(),
        };
        context.add_package(package, "");
        for import in &file.imports {
            let Ok(path) = crate::decode_import_path(&import.path) else {
                continue;
            };
            let alias = import
                .alias
                .as_ref()
                .map(|alias| alias.name.clone())
                .unwrap_or_else(|| path.rsplit('/').next().unwrap_or_default().into());
            context
                .declarations
                .entry(alias.clone())
                .or_default()
                .push(import.span);
            if let Some(package) = project
                .packages
                .iter()
                .find(|package| package.logical_path == path)
            {
                context.add_package(package, &format!("{alias}."));
            }
        }
        context
    }
    fn add_package(&mut self, package: &ast::Package, prefix: &str) {
        for declaration in package.files.iter().flat_map(|file| &file.decls) {
            let names = if let ast::TopLevelDecl::Type(decl) = declaration {
                decl.specs
                    .iter()
                    .map(|spec| spec.name.name.clone())
                    .collect()
            } else {
                top_level_value_names(declaration)
            };
            for name in names {
                self.declarations
                    .entry(format!("{prefix}{name}"))
                    .or_default()
                    .push(crate::ond::declaration_span(declaration));
            }
        }
    }
    fn visible(&self, name: &str, caller: crate::source::Span) -> bool {
        self.declarations.get(name).is_none_or(|declarations| {
            declarations
                .iter()
                .any(|declaration| crate::roles::visible(self.sources, *declaration, caller))
        })
    }
}

struct NameValidator<'a> {
    role_context: &'a RoleNameContext<'a>,
    package_values: &'a BTreeSet<String>,
    imports: &'a BTreeSet<String>,
    hidden_types: &'a BTreeSet<String>,
    imported_names: &'a BTreeMap<String, BTreeSet<String>>,
    generic_functions: &'a BTreeSet<Vec<String>>,
    type_parameters: BTreeSet<String>,
    scopes: Vec<BTreeMap<String, Binding>>,
    diagnostics: &'a mut Diagnostics,
}

impl<'a> NameValidator<'a> {
    fn new(
        package_values: &'a BTreeSet<String>,
        imports: &'a BTreeSet<String>,
        hidden_types: &'a BTreeSet<String>,
        imported_names: &'a BTreeMap<String, BTreeSet<String>>,
        generic_functions: &'a BTreeSet<Vec<String>>,
        role_context: &'a RoleNameContext<'a>,
        diagnostics: &'a mut Diagnostics,
    ) -> Self {
        Self {
            role_context,
            package_values,
            imports,
            hidden_types,
            imported_names,
            generic_functions,
            type_parameters: BTreeSet::new(),
            scopes: vec![BTreeMap::new()],
            diagnostics,
        }
    }

    fn declare(&mut self, name: &ast::Ident, binding: Binding) {
        if name.name != "_" {
            let scope = self
                .scopes
                .last_mut()
                .expect("name validator always has a scope");
            if let std::collections::btree_map::Entry::Vacant(entry) =
                scope.entry(name.name.clone())
            {
                entry.insert(binding);
            } else {
                self.diagnostics.push(Diagnostic::error(
                    name.span,
                    format!("duplicate local declaration `{}`", name.name),
                ));
            }
        }
    }

    fn known(&self, name: &str, span: crate::source::Span) -> bool {
        name == "_"
            || BUILTINS.contains(&name)
            || (self.package_values.contains(name) && self.role_context.visible(name, span))
            || (self.imports.contains(name) && self.role_context.visible(name, span))
            || self
                .scopes
                .iter()
                .rev()
                .any(|scope| scope.contains_key(name))
    }

    fn generic_function(&self, expression: &ast::Expr) -> bool {
        let ast::Expr::Name(path) = expression else {
            return false;
        };
        if matches!(path.segments.as_slice(), [name] if name.name=="alloc") {
            return true;
        }
        !path.segments.first().is_some_and(|name| {
            self.scopes
                .iter()
                .any(|scope| scope.contains_key(&name.name))
        }) && self.generic_functions.contains(
            &path
                .segments
                .iter()
                .map(|name| name.name.clone())
                .collect::<Vec<_>>(),
        )
    }

    fn block(&mut self, block: &ast::Block) {
        self.scopes.push(BTreeMap::new());
        self.body(block);
        self.scopes.pop();
    }

    fn body(&mut self, block: &ast::Block) {
        for statement in &block.statements {
            self.statement(statement);
        }
    }

    fn statement(&mut self, statement: &ast::Stmt) {
        match statement {
            ast::Stmt::InlineAssembly(statement) => {
                for diagnostic in crate::inline_asm::validate_source(statement) {
                    self.diagnostics.push(diagnostic);
                }
                for binding in &statement.bindings {
                    self.expr(&binding.value);
                }
            }
            ast::Stmt::Const(declaration) => {
                for spec in &declaration.specs {
                    if let Some(ty) = &spec.ty {
                        self.ty(ty);
                    }
                    for value in &spec.values {
                        self.expr(value);
                    }
                    for name in &spec.names {
                        self.declare(name, Binding::Constant);
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
                        self.declare(name, Binding::Variable);
                    }
                }
            }
            ast::Stmt::ShortVar(declaration) => {
                for value in &declaration.values {
                    self.expr(value);
                }
                let scope = self.scopes.last().expect("short declaration has a scope");
                let mut seen = BTreeSet::new();
                let mut new_names = Vec::new();
                for name in declaration.names.iter().filter(|name| name.name != "_") {
                    if !seen.insert(&name.name) {
                        self.diagnostics.push(Diagnostic::error(
                            name.span,
                            "duplicate name in short declaration",
                        ));
                    } else if scope.get(&name.name) == Some(&Binding::Constant) {
                        self.diagnostics
                            .push(Diagnostic::error(name.span, "cannot assign to a constant"));
                    } else if !scope.contains_key(&name.name) {
                        new_names.push(name);
                    }
                }
                if new_names.is_empty() {
                    self.diagnostics.push(Diagnostic::error(
                        declaration.span,
                        "short declaration requires at least one new name",
                    ));
                }
                for name in new_names {
                    self.declare(name, Binding::Variable);
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
                self.scopes.push(BTreeMap::new());
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
                self.scopes.push(BTreeMap::new());
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
        if self
            .scopes
            .iter()
            .any(|scope| scope.contains_key(&alias.name))
        {
            return;
        }
        if self
            .imported_names
            .get(&alias.name)
            .is_some_and(|names| names.contains(&name.name))
            || !self
                .role_context
                .visible(&format!("{}.{}", alias.name, name.name), name.span)
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
                if (self.hidden_types.contains(&name.name)
                    || !self.role_context.visible(&name.name, name.span))
                    && !self.type_parameters.contains(&name.name) =>
            {
                self.diagnostics.push(Diagnostic::error(
                    name.span,
                    format!("unknown type `{}`", name.name),
                ));
            }
            [alias, name] => {
                if !self.role_context.visible(&alias.name, alias.span) {
                    self.diagnostics.push(Diagnostic::error(
                        alias.span,
                        format!("import alias `{}` is not visible here", alias.name),
                    ));
                } else {
                    self.imported_member(alias, name);
                }
            }
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
                    && !self.known(&name.name, name.span)
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
                if !self.generic_function(base)
                    && let [argument] = arguments.as_slice()
                    && let Some(index) = super::body::type_argument_index(argument)
                {
                    self.expr(&index);
                } else {
                    for ty in arguments {
                        self.ty(ty);
                    }
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
                if self.generic_function(base)
                    && let Some(ty) = super::body::index_type_argument(index)
                {
                    self.ty(&ty);
                } else {
                    self.expr(index);
                }
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
