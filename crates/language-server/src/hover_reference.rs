use std::collections::BTreeMap;

use compiler::ir::ast;
use compiler::source::Span;

use crate::completion::CompletionScope;

#[derive(Debug, Clone, Copy)]
pub struct ResolvedReference {
    pub target: ResolvedTarget,
    pub reference: Span,
}

#[derive(Debug, Clone, Copy)]
pub enum ResolvedTarget {
    Declaration(Span),
    Package(usize),
}

#[derive(Debug, Clone)]
pub struct LocalValueReference {
    pub name: String,
    pub reference: Span,
    pub ty: Option<ast::Type>,
}

#[derive(Debug, Clone)]
pub struct ResolvedFieldReference {
    pub declaration: Span,
    pub reference: Span,
    pub name: String,
    pub ty: ast::Type,
}

#[derive(Debug, Clone)]
pub struct ResolvedMethodReference {
    pub declaration: Span,
    pub reference: Span,
    pub name: String,
    pub signature: ast::TypeSignature,
}

pub fn resolve_interface_method_reference(
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
    offset: usize,
) -> Option<ResolvedMethodReference> {
    let (ty, field) = match find_reference(file, offset)? {
        Reference::Name {
            path,
            segment: 1,
            function,
            type_position: false,
        } if path.segments.len() == 2 => {
            let base = &path.segments[0];
            let ty = type_for_name(package, function, &base.name, offset).or_else(|| {
                crate::completion::project_value_type_for_name(
                    project, package, file, &base.name, offset,
                )
            })?;
            (ty, &path.segments[1])
        }
        Reference::Field {
            base,
            field,
            function,
        } => (
            resolve_expr_type(project, package, file, function, base, offset)?,
            field,
        ),
        _ => return None,
    };
    let method = interface_method_in_project(project, package, file, &ty, &field.name, 0, false)?;
    Some(ResolvedMethodReference {
        declaration: method.name.span,
        reference: field.span,
        name: field.name.clone(),
        signature: method.signature,
    })
}

#[allow(clippy::too_many_arguments)]
fn interface_method_in_project(
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
    ty: &ast::Type,
    name: &str,
    depth: usize,
    external: bool,
) -> Option<ast::InterfaceMethod> {
    if depth > 16 {
        return None;
    }
    match ty {
        ast::Type::Interface { methods, .. } => methods
            .iter()
            .find(|method| method.name.name == name && (!external || is_exported(name)))
            .cloned(),
        ast::Type::Named(path) if path.segments.len() == 1 => {
            let ty = find_type(package, &path.segments[0].name)?;
            interface_method_in_project(project, package, file, ty, name, depth + 1, external)
        }
        ast::Type::Named(path) if path.segments.len() == 2 => {
            let import_path = import_path_for_alias(file, &path.segments[0].name)?;
            let imported = project
                .packages
                .iter()
                .find(|candidate| candidate.logical_path == import_path)?;
            let ty = find_type(imported, &path.segments[1].name)?;
            let imported_file = imported.files.first()?;
            interface_method_in_project(project, imported, imported_file, ty, name, depth + 1, true)
        }
        ast::Type::Apply {
            base, arguments, ..
        } if base.segments.len() == 1 => {
            let spec = find_type_spec(package, &base.segments[0].name)?;
            let ty = substitute_type_spec(spec, arguments)?;
            interface_method_in_project(project, package, file, &ty, name, depth + 1, external)
        }
        ast::Type::Apply {
            base, arguments, ..
        } if base.segments.len() == 2 => {
            let import_path = import_path_for_alias(file, &base.segments[0].name)?;
            let imported = project
                .packages
                .iter()
                .find(|candidate| candidate.logical_path == import_path)?;
            let spec = find_type_spec(imported, &base.segments[1].name)?;
            let ty = substitute_type_spec(spec, arguments)?;
            let imported_file = imported.files.first()?;
            interface_method_in_project(
                project,
                imported,
                imported_file,
                &ty,
                name,
                depth + 1,
                true,
            )
        }
        _ => None,
    }
}

pub fn resolve_field_reference(
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
    offset: usize,
) -> Option<ResolvedFieldReference> {
    let (ty, field) = match find_reference(file, offset)? {
        Reference::Name {
            path,
            segment: 1,
            function,
            type_position: false,
        } if path.segments.len() == 2 => {
            let base = &path.segments[0];
            let ty = type_for_name(package, function, &base.name, offset).or_else(|| {
                crate::completion::project_value_type_for_name(
                    project, package, file, &base.name, offset,
                )
            })?;
            (ty, &path.segments[1])
        }
        Reference::Field {
            base,
            field,
            function,
        } => (
            resolve_expr_type(project, package, file, function, base, offset)?,
            field,
        ),
        Reference::CompositeField { ty, field } => (ty.clone(), field),
        _ => return None,
    };
    let declaration = resolve_composite_field(project, package, file, &ty, &field.name, 0)?;
    let ty = field_type_in_project(project, package, file, &ty, &field.name, 0)?;
    Some(ResolvedFieldReference {
        declaration,
        reference: field.span,
        name: field.name.clone(),
        ty,
    })
}

pub fn resolve_local_value_reference(
    file: &ast::File,
    offset: usize,
) -> Option<LocalValueReference> {
    if let Some((_, _, name, _)) = find_short_declaration(file, offset) {
        return Some(LocalValueReference {
            name: name.name.clone(),
            reference: name.span,
            ty: None,
        });
    }
    if let Some((function, name)) = find_local_declaration(file, offset) {
        return Some(LocalValueReference {
            name: name.name.clone(),
            reference: name.span,
            ty: local_binding_info(Some(function), &name.name, offset)
                .and_then(|binding| binding.ty),
        });
    }
    let Reference::Name {
        path,
        segment,
        function: Some(function),
        type_position: false,
    } = find_reference(file, offset)?
    else {
        return None;
    };
    let name = path.segments.get(segment)?;
    let binding = local_binding_info(Some(function), &name.name, offset)?;
    Some(LocalValueReference {
        name: name.name.clone(),
        reference: name.span,
        ty: binding.ty,
    })
}

pub fn resolve_reference(
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
    offset: usize,
) -> Option<ResolvedReference> {
    if let Some((function, decl, name, scope_depth)) = find_short_declaration(file, offset) {
        let target = local_binding_info(Some(function), &name.name, decl.span.start)
            .filter(|binding| binding.scope_depth == scope_depth)
            .map_or(name.span, |binding| binding.declaration);
        return Some(ResolvedReference {
            target: ResolvedTarget::Declaration(target),
            reference: name.span,
        });
    }
    let reference = find_reference(file, offset)?;
    match reference {
        Reference::Name {
            path,
            segment,
            function,
            type_position,
        } => resolve_name(
            project,
            package,
            file,
            function,
            path,
            segment,
            offset,
            type_position,
        ),
        Reference::Field {
            base,
            field,
            function,
        } => {
            let ty = resolve_expr_type(project, package, file, function, base, offset)?;
            let declaration = resolve_composite_field(project, package, file, &ty, &field.name, 0)
                .or_else(|| resolve_method(project, package, file, &ty, &field.name))?;
            Some(ResolvedReference {
                target: ResolvedTarget::Declaration(declaration),
                reference: field.span,
            })
        }
        Reference::CompositeField { ty, field } => {
            let declaration = resolve_composite_field(project, package, file, ty, &field.name, 0)?;
            Some(ResolvedReference {
                target: ResolvedTarget::Declaration(declaration),
                reference: field.span,
            })
        }
    }
}

fn resolve_method(
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
    ty: &ast::Type,
    name: &str,
) -> Option<Span> {
    let (path, pointer) = method_receiver_path(ty, false)?;
    match path.segments.as_slice() {
        [base] => method_in_package(package, &base.name, pointer, name, false),
        [alias, base] => {
            if !is_exported(&base.name) || !is_exported(name) {
                return None;
            }
            let import_path = import_path_for_alias(file, &alias.name)?;
            let imported = project
                .packages
                .iter()
                .find(|candidate| candidate.logical_path == import_path)?;
            method_in_package(imported, &base.name, pointer, name, true)
        }
        _ => None,
    }
}

fn method_receiver_path(ty: &ast::Type, pointer: bool) -> Option<(&ast::Path, bool)> {
    match ty {
        ast::Type::Named(path) => Some((path, pointer)),
        ast::Type::Apply { base, .. } => Some((base, pointer)),
        ast::Type::Pointer(inner, _) => method_receiver_path(inner, true),
        _ => None,
    }
}

fn method_in_package(
    package: &ast::Package,
    receiver_name: &str,
    pointer: bool,
    method_name: &str,
    external: bool,
) -> Option<Span> {
    for file in &package.files {
        for decl in &file.decls {
            if let ast::TopLevelDecl::Func(method) = decl
                && let Some(receiver) = &method.receiver
                && exact_receiver_name(&receiver.ty) == Some((receiver_name, pointer))
                && method.name.name == method_name
                && (!external || is_exported(method_name))
            {
                return Some(method.name.span);
            }
        }
    }
    if pointer {
        return None;
    }
    let ast::Type::Interface { methods, .. } = find_type(package, receiver_name)? else {
        return None;
    };
    methods
        .iter()
        .find(|method| method.name.name == method_name && (!external || is_exported(method_name)))
        .map(|method| method.name.span)
}

fn exact_receiver_name(ty: &ast::Type) -> Option<(&str, bool)> {
    match ty {
        ast::Type::Named(path) if path.segments.len() == 1 => Some((&path.segments[0].name, false)),
        ast::Type::Apply { base, .. } if base.segments.len() == 1 => {
            Some((&base.segments[0].name, false))
        }
        ast::Type::Pointer(inner, _) => match inner.as_ref() {
            ast::Type::Named(path) if path.segments.len() == 1 => {
                Some((&path.segments[0].name, true))
            }
            ast::Type::Apply { base, .. } if base.segments.len() == 1 => {
                Some((&base.segments[0].name, true))
            }
            _ => None,
        },
        _ => None,
    }
}

fn find_short_declaration(
    file: &ast::File,
    offset: usize,
) -> Option<(CompletionScope<'_>, &ast::ShortVarDecl, &ast::Ident, usize)> {
    file.decls.iter().find_map(|decl| {
        let scope = match decl {
            ast::TopLevelDecl::Func(function) => CompletionScope::Function(function),
            ast::TopLevelDecl::Operator(operator) => CompletionScope::Operator(operator),
            _ => return None,
        };
        find_short_in_block(scope.body(), scope, offset, 0)
    })
}

fn find_local_declaration(
    file: &ast::File,
    offset: usize,
) -> Option<(CompletionScope<'_>, &ast::Ident)> {
    file.decls.iter().find_map(|decl| {
        let scope = match decl {
            ast::TopLevelDecl::Func(function) => CompletionScope::Function(function),
            ast::TopLevelDecl::Operator(operator) => CompletionScope::Operator(operator),
            _ => return None,
        };
        if let Some(receiver) = scope.receiver()
            && contains(receiver.name.span, offset)
        {
            return Some((scope, &receiver.name));
        }
        if let Some(parameter) = scope
            .params()
            .iter()
            .find(|parameter| contains(parameter.name.span, offset))
        {
            return Some((scope, &parameter.name));
        }
        find_declared_name_in_block(scope.body(), offset).map(|name| (scope, name))
    })
}

fn find_declared_name_in_block(block: &ast::Block, offset: usize) -> Option<&ast::Ident> {
    block
        .statements
        .iter()
        .find_map(|statement| match statement {
            ast::Stmt::Const(decl) => decl
                .specs
                .iter()
                .flat_map(|spec| &spec.names)
                .find(|name| contains(name.span, offset)),
            ast::Stmt::Var(decl) => decl
                .specs
                .iter()
                .flat_map(|spec| &spec.names)
                .find(|name| contains(name.span, offset)),
            ast::Stmt::ShortVar(decl) => decl.names.iter().find(|name| contains(name.span, offset)),
            ast::Stmt::Block(block) => find_declared_name_in_block(block, offset),
            ast::Stmt::Defer(defer) => find_declared_name_in_block(&defer.block, offset),
            ast::Stmt::If(statement) => statement
                .init
                .as_deref()
                .and_then(|init| find_declared_name_in_stmt(init, offset))
                .or_else(|| find_declared_name_in_block(&statement.then_block, offset))
                .or_else(|| {
                    statement
                        .else_branch
                        .as_deref()
                        .and_then(|branch| find_declared_name_in_stmt(branch, offset))
                }),
            ast::Stmt::For(statement) => {
                let header = match &statement.kind {
                    ast::ForKind::ThreeClause { init, post, .. } => init
                        .as_deref()
                        .and_then(|init| find_declared_name_in_stmt(init, offset))
                        .or_else(|| {
                            post.as_deref()
                                .and_then(|post| find_declared_name_in_stmt(post, offset))
                        }),
                    _ => None,
                };
                header.or_else(|| find_declared_name_in_block(&statement.body, offset))
            }
            _ => None,
        })
}

fn find_declared_name_in_stmt(statement: &ast::Stmt, offset: usize) -> Option<&ast::Ident> {
    match statement {
        ast::Stmt::Const(decl) => decl
            .specs
            .iter()
            .flat_map(|spec| &spec.names)
            .find(|name| contains(name.span, offset)),
        ast::Stmt::Var(decl) => decl
            .specs
            .iter()
            .flat_map(|spec| &spec.names)
            .find(|name| contains(name.span, offset)),
        ast::Stmt::ShortVar(decl) => decl.names.iter().find(|name| contains(name.span, offset)),
        ast::Stmt::Block(block) => find_declared_name_in_block(block, offset),
        _ => None,
    }
}

fn find_short_in_block<'a>(
    block: &'a ast::Block,
    function: CompletionScope<'a>,
    offset: usize,
    scope_depth: usize,
) -> Option<(
    CompletionScope<'a>,
    &'a ast::ShortVarDecl,
    &'a ast::Ident,
    usize,
)> {
    block.statements.iter().find_map(|stmt| match stmt {
        ast::Stmt::ShortVar(decl) => decl
            .names
            .iter()
            .find(|name| contains(name.span, offset))
            .map(|name| (function, decl, name, scope_depth)),
        ast::Stmt::Block(block) => find_short_in_block(block, function, offset, scope_depth + 1),
        ast::Stmt::Defer(stmt) => {
            find_short_in_block(&stmt.block, function, offset, scope_depth + 1)
        }
        ast::Stmt::If(stmt) => stmt
            .init
            .as_deref()
            .and_then(|stmt| find_short_in_stmt(stmt, function, offset, scope_depth + 1))
            .or_else(|| find_short_in_block(&stmt.then_block, function, offset, scope_depth + 2))
            .or_else(|| {
                stmt.else_branch
                    .as_deref()
                    .and_then(|stmt| find_short_in_stmt(stmt, function, offset, scope_depth + 1))
            }),
        ast::Stmt::For(stmt) => {
            let header = if let ast::ForKind::ThreeClause { init, post, .. } = &stmt.kind {
                init.as_deref()
                    .and_then(|stmt| find_short_in_stmt(stmt, function, offset, scope_depth + 1))
                    .or_else(|| {
                        post.as_deref().and_then(|stmt| {
                            find_short_in_stmt(stmt, function, offset, scope_depth + 1)
                        })
                    })
            } else {
                None
            };
            header.or_else(|| find_short_in_block(&stmt.body, function, offset, scope_depth + 2))
        }
        _ => None,
    })
}

fn find_short_in_stmt<'a>(
    stmt: &'a ast::Stmt,
    function: CompletionScope<'a>,
    offset: usize,
    scope_depth: usize,
) -> Option<(
    CompletionScope<'a>,
    &'a ast::ShortVarDecl,
    &'a ast::Ident,
    usize,
)> {
    match stmt {
        ast::Stmt::ShortVar(decl) => decl
            .names
            .iter()
            .find(|name| contains(name.span, offset))
            .map(|name| (function, decl, name, scope_depth)),
        ast::Stmt::Block(block) => find_short_in_block(block, function, offset, scope_depth + 1),
        ast::Stmt::Defer(defer) => {
            find_short_in_block(&defer.block, function, offset, scope_depth + 1)
        }
        _ => None,
    }
}

enum Reference<'a> {
    Name {
        path: &'a ast::Path,
        segment: usize,
        function: Option<CompletionScope<'a>>,
        type_position: bool,
    },
    Field {
        base: &'a ast::Expr,
        field: &'a ast::Ident,
        function: Option<CompletionScope<'a>>,
    },
    CompositeField {
        ty: &'a ast::Type,
        field: &'a ast::Ident,
    },
}

fn resolve_name(
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
    function: Option<CompletionScope<'_>>,
    path: &ast::Path,
    segment: usize,
    offset: usize,
    type_position: bool,
) -> Option<ResolvedReference> {
    let ident = path.segments.get(segment)?;
    if path.segments.len() == 1 {
        if !type_position && let Some(binding) = local_binding_info(function, &ident.name, offset) {
            return Some(ResolvedReference {
                target: ResolvedTarget::Declaration(binding.declaration),
                reference: ident.span,
            });
        }
        if type_position
            && let Some(parameter) = function
                .into_iter()
                .flat_map(CompletionScope::type_params)
                .find(|parameter| parameter.name == ident.name)
        {
            return Some(ResolvedReference {
                target: ResolvedTarget::Declaration(parameter.span),
                reference: ident.span,
            });
        }
        let declaration = if type_position {
            type_declaration(package, &ident.name)
        } else {
            top_level_declaration(package, &ident.name)
        }?;
        return Some(ResolvedReference {
            target: ResolvedTarget::Declaration(declaration),
            reference: ident.span,
        });
    }

    if path.segments.len() != 2 {
        return None;
    }

    let base = &path.segments[0];
    if segment == 0 {
        if let Some(binding) = local_binding_info(function, &base.name, offset) {
            return Some(ResolvedReference {
                target: ResolvedTarget::Declaration(binding.declaration),
                reference: base.span,
            });
        }
        let import_path = import_path_for_alias(file, &base.name)?;
        let package_index = project
            .packages
            .iter()
            .position(|candidate| candidate.logical_path == import_path)?;
        return Some(ResolvedReference {
            target: ResolvedTarget::Package(package_index),
            reference: base.span,
        });
    }

    if segment != 1 {
        return None;
    }

    if !has_local_binding(function, &base.name, offset)
        && let Some(import_path) = import_path_for_alias(file, &base.name)
    {
        let imported = project
            .packages
            .iter()
            .find(|candidate| candidate.logical_path == import_path)?;
        let declaration = if type_position {
            type_declaration(imported, &ident.name)
        } else {
            top_level_declaration(imported, &ident.name)
        }?;
        if !is_exported(&ident.name) {
            return None;
        }
        return Some(ResolvedReference {
            target: ResolvedTarget::Declaration(declaration),
            reference: ident.span,
        });
    }

    let ty = type_for_name(package, function, &base.name, offset).or_else(|| {
        crate::completion::project_value_type_for_name(project, package, file, &base.name, offset)
    })?;
    let declaration = resolve_composite_field(project, package, file, &ty, &ident.name, 0)
        .or_else(|| resolve_method(project, package, file, &ty, &ident.name))?;
    Some(ResolvedReference {
        target: ResolvedTarget::Declaration(declaration),
        reference: ident.span,
    })
}

fn type_declaration(package: &ast::Package, name: &str) -> Option<Span> {
    package.files.iter().find_map(|file| {
        file.decls.iter().find_map(|decl| {
            let ast::TopLevelDecl::Type(decl) = decl else {
                return None;
            };
            decl.specs
                .iter()
                .find(|spec| spec.name.name == name)
                .map(|spec| spec.name.span)
        })
    })
}

fn top_level_declaration(package: &ast::Package, name: &str) -> Option<Span> {
    package.files.iter().find_map(|file| {
        file.decls.iter().find_map(|decl| match decl {
            ast::TopLevelDecl::Const(decl) => decl
                .specs
                .iter()
                .flat_map(|spec| &spec.names)
                .find(|ident| ident.name == name)
                .map(|ident| ident.span),
            ast::TopLevelDecl::Var(decl) => decl
                .specs
                .iter()
                .flat_map(|spec| &spec.names)
                .find(|ident| ident.name == name)
                .map(|ident| ident.span),
            ast::TopLevelDecl::Type(decl) => decl
                .specs
                .iter()
                .find(|spec| spec.name.name == name)
                .map(|spec| spec.name.span),
            ast::TopLevelDecl::Func(decl) => {
                (decl.receiver.is_none() && decl.name.name == name).then_some(decl.name.span)
            }
            ast::TopLevelDecl::Operator(_) => None,
        })
    })
}

fn find_reference(file: &ast::File, offset: usize) -> Option<Reference<'_>> {
    for decl in &file.decls {
        let found = match decl {
            ast::TopLevelDecl::Const(decl) => decl.specs.iter().find_map(|spec| {
                spec.ty
                    .as_ref()
                    .and_then(|ty| find_in_type(ty, offset, None))
                    .or_else(|| {
                        spec.values
                            .iter()
                            .find_map(|expr| find_in_expr(expr, offset, None))
                    })
            }),
            ast::TopLevelDecl::Var(decl) => decl.specs.iter().find_map(|spec| {
                spec.ty
                    .as_ref()
                    .and_then(|ty| find_in_type(ty, offset, None))
                    .or_else(|| {
                        spec.values
                            .iter()
                            .find_map(|expr| find_in_expr(expr, offset, None))
                    })
            }),
            ast::TopLevelDecl::Type(decl) => decl
                .specs
                .iter()
                .find_map(|spec| find_in_type(&spec.ty, offset, None)),
            ast::TopLevelDecl::Func(function) => {
                let scope = CompletionScope::Function(function);
                function
                    .receiver
                    .as_ref()
                    .and_then(|receiver| find_in_type(&receiver.ty, offset, Some(scope)))
                    .or_else(|| {
                        function
                            .signature
                            .params
                            .iter()
                            .find_map(|field| find_in_type(&field.ty, offset, Some(scope)))
                    })
                    .or_else(|| {
                        function
                            .signature
                            .results
                            .iter()
                            .find_map(|ty| find_in_type(ty, offset, Some(scope)))
                    })
                    .or_else(|| find_in_block(&function.body, offset, scope))
            }
            ast::TopLevelDecl::Operator(operator) => {
                let scope = CompletionScope::Operator(operator);
                operator
                    .signature
                    .params
                    .iter()
                    .find_map(|field| find_in_type(&field.ty, offset, Some(scope)))
                    .or_else(|| {
                        operator
                            .signature
                            .results
                            .iter()
                            .find_map(|ty| find_in_type(ty, offset, Some(scope)))
                    })
                    .or_else(|| find_in_block(&operator.body, offset, scope))
            }
        };
        if found.is_some() {
            return found;
        }
    }
    None
}

fn find_in_block<'a>(
    block: &'a ast::Block,
    offset: usize,
    function: CompletionScope<'a>,
) -> Option<Reference<'a>> {
    block
        .statements
        .iter()
        .find_map(|stmt| find_in_stmt(stmt, offset, function))
}

fn find_in_stmt<'a>(
    stmt: &'a ast::Stmt,
    offset: usize,
    function: CompletionScope<'a>,
) -> Option<Reference<'a>> {
    let context = Some(function);
    match stmt {
        ast::Stmt::Const(decl) => decl.specs.iter().find_map(|spec| {
            spec.ty
                .as_ref()
                .and_then(|ty| find_in_type(ty, offset, context))
                .or_else(|| {
                    spec.values
                        .iter()
                        .find_map(|expr| find_in_expr(expr, offset, context))
                })
        }),
        ast::Stmt::Var(decl) => decl.specs.iter().find_map(|spec| {
            spec.ty
                .as_ref()
                .and_then(|ty| find_in_type(ty, offset, context))
                .or_else(|| {
                    spec.values
                        .iter()
                        .find_map(|expr| find_in_expr(expr, offset, context))
                })
        }),
        ast::Stmt::ShortVar(decl) => decl
            .values
            .iter()
            .find_map(|expr| find_in_expr(expr, offset, context)),
        ast::Stmt::Assign(stmt) => stmt
            .targets
            .iter()
            .chain(&stmt.values)
            .find_map(|expr| find_in_expr(expr, offset, context)),
        ast::Stmt::Expr(stmt) => find_in_expr(&stmt.expr, offset, context),
        ast::Stmt::Return(stmt) => stmt
            .values
            .iter()
            .find_map(|expr| find_in_expr(expr, offset, context)),
        ast::Stmt::Defer(stmt) => find_in_block(&stmt.block, offset, function),
        ast::Stmt::Block(block) => find_in_block(block, offset, function),
        ast::Stmt::If(stmt) => stmt
            .init
            .as_deref()
            .and_then(|stmt| find_in_stmt(stmt, offset, function))
            .or_else(|| find_in_expr(&stmt.condition, offset, context))
            .or_else(|| find_in_block(&stmt.then_block, offset, function))
            .or_else(|| {
                stmt.else_branch
                    .as_deref()
                    .and_then(|stmt| find_in_stmt(stmt, offset, function))
            }),
        ast::Stmt::For(stmt) => {
            let header = match &stmt.kind {
                ast::ForKind::Infinite => None,
                ast::ForKind::While(expr) => find_in_expr(expr, offset, context),
                ast::ForKind::ThreeClause {
                    init,
                    condition,
                    post,
                } => init
                    .as_deref()
                    .and_then(|stmt| find_in_stmt(stmt, offset, function))
                    .or_else(|| {
                        condition
                            .as_ref()
                            .and_then(|expr| find_in_expr(expr, offset, context))
                    })
                    .or_else(|| {
                        post.as_deref()
                            .and_then(|stmt| find_in_stmt(stmt, offset, function))
                    }),
            };
            header.or_else(|| find_in_block(&stmt.body, offset, function))
        }
        ast::Stmt::Break(_) | ast::Stmt::Continue(_) | ast::Stmt::Trap { .. } => None,
    }
}

fn find_in_expr<'a>(
    expr: &'a ast::Expr,
    offset: usize,
    function: Option<CompletionScope<'a>>,
) -> Option<Reference<'a>> {
    match expr {
        ast::Expr::Name(path) => path
            .segments
            .iter()
            .position(|ident| contains(ident.span, offset))
            .map(|segment| Reference::Name {
                path,
                segment,
                function,
                type_position: false,
            }),
        ast::Expr::Selector { base, field, .. } => {
            if contains(field.span, offset) {
                Some(Reference::Field {
                    base,
                    field,
                    function,
                })
            } else {
                find_in_expr(base, offset, function)
            }
        }
        ast::Expr::Unary { expr, .. } => find_in_expr(expr, offset, function),
        ast::Expr::Binary { lhs, rhs, .. } => {
            find_in_expr(lhs, offset, function).or_else(|| find_in_expr(rhs, offset, function))
        }
        ast::Expr::Call { callee, args, .. } => {
            find_in_expr(callee, offset, function).or_else(|| {
                args.iter()
                    .find_map(|arg| find_in_expr(arg, offset, function))
            })
        }
        ast::Expr::TypeApply {
            base, arguments, ..
        } => find_in_expr(base, offset, function).or_else(|| {
            arguments
                .iter()
                .find_map(|ty| find_in_type(ty, offset, function))
        }),
        ast::Expr::Index { base, index, .. } => {
            find_in_expr(base, offset, function).or_else(|| find_in_expr(index, offset, function))
        }
        ast::Expr::Cast { expr, ty, .. } => {
            find_in_expr(expr, offset, function).or_else(|| find_in_type(ty, offset, function))
        }
        ast::Expr::Composite { ty, elements, .. } => {
            find_in_type(ty, offset, function).or_else(|| {
                elements.iter().find_map(|element| {
                    if let Some(ast::Expr::Name(path)) = element.key.as_ref() {
                        if path.segments.len() == 1 && contains(path.segments[0].span, offset) {
                            return Some(Reference::CompositeField {
                                ty,
                                field: &path.segments[0],
                            });
                        }
                    }
                    element
                        .key
                        .as_ref()
                        .and_then(|key| find_in_expr(key, offset, function))
                        .or_else(|| find_in_expr(&element.value, offset, function))
                })
            })
        }
        ast::Expr::New { ty, .. } => find_in_type(ty, offset, function),
        ast::Expr::LayoutQuery { ty, .. } => find_in_type(ty, offset, function),
        ast::Expr::Literal(_) => None,
    }
}

fn resolve_composite_field(
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
    ty: &ast::Type,
    name: &str,
    depth: usize,
) -> Option<Span> {
    if depth > 16 {
        return None;
    }
    match ty {
        ast::Type::Pointer(inner, _) => {
            resolve_composite_field(project, package, file, inner, name, depth + 1)
        }
        ast::Type::Struct { .. } => resolve_field(package, ty, name, depth),
        ast::Type::Named(path) if path.segments.len() == 1 => {
            resolve_field(package, ty, name, depth)
        }
        ast::Type::Named(path) if path.segments.len() == 2 => {
            if !is_exported(&path.segments[1].name) || !is_exported(name) {
                return None;
            }
            let import_path = import_path_for_alias(file, &path.segments[0].name)?;
            let imported = project
                .packages
                .iter()
                .find(|candidate| candidate.logical_path == import_path)?;
            let imported_ty = find_type(imported, &path.segments[1].name)?;
            resolve_field(imported, imported_ty, name, depth + 1)
        }
        ast::Type::Apply {
            base, arguments, ..
        } if base.segments.len() == 1 => {
            let spec = find_type_spec(package, &base.segments[0].name)?;
            let ty = substitute_type_spec(spec, arguments)?;
            resolve_composite_field(project, package, file, &ty, name, depth + 1)
        }
        ast::Type::Apply {
            base, arguments, ..
        } if base.segments.len() == 2 => {
            if !is_exported(&base.segments[1].name) || !is_exported(name) {
                return None;
            }
            let import_path = import_path_for_alias(file, &base.segments[0].name)?;
            let imported = project
                .packages
                .iter()
                .find(|candidate| candidate.logical_path == import_path)?;
            let spec = find_type_spec(imported, &base.segments[1].name)?;
            let ty = substitute_type_spec(spec, arguments)?;
            let imported_file = imported.files.first()?;
            resolve_composite_field(project, imported, imported_file, &ty, name, depth + 1)
        }
        _ => None,
    }
}

fn find_in_type<'a>(
    ty: &'a ast::Type,
    offset: usize,
    function: Option<CompletionScope<'a>>,
) -> Option<Reference<'a>> {
    match ty {
        ast::Type::Apply {
            base, arguments, ..
        } => base
            .segments
            .iter()
            .position(|ident| contains(ident.span, offset))
            .map(|segment| Reference::Name {
                path: base,
                segment,
                function,
                type_position: true,
            })
            .or_else(|| {
                arguments
                    .iter()
                    .find_map(|argument| find_in_type(argument, offset, function))
            }),
        ast::Type::Resolved(_, _) => None,
        ast::Type::Named(path) => path
            .segments
            .iter()
            .position(|ident| contains(ident.span, offset))
            .map(|segment| Reference::Name {
                path,
                segment,
                function,
                type_position: true,
            }),
        ast::Type::Pointer(inner, _) => find_in_type(inner, offset, function),
        ast::Type::Array { len, element, .. } => len
            .as_ref()
            .and_then(|len| find_in_expr(len, offset, function))
            .or_else(|| find_in_type(element, offset, function)),
        ast::Type::Struct { fields, .. } => fields
            .iter()
            .find_map(|field| find_in_type(&field.ty, offset, function)),
        ast::Type::Interface { methods, .. } => methods.iter().find_map(|method| {
            method
                .signature
                .params
                .iter()
                .find_map(|parameter| find_in_type(&parameter.ty, offset, function))
                .or_else(|| {
                    method
                        .signature
                        .results
                        .iter()
                        .find_map(|ty| find_in_type(ty, offset, function))
                })
        }),
        ast::Type::Func { signature, .. } => signature
            .params
            .iter()
            .find_map(|field| find_in_type(&field.ty, offset, function))
            .or_else(|| {
                signature
                    .results
                    .iter()
                    .find_map(|ty| find_in_type(ty, offset, function))
            }),
    }
}

fn resolve_expr_type(
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
    function: Option<CompletionScope<'_>>,
    expr: &ast::Expr,
    limit: usize,
) -> Option<ast::Type> {
    match expr {
        ast::Expr::Name(path) if path.segments.len() == 1 => {
            type_for_name(package, function, &path.segments[0].name, limit).or_else(|| {
                crate::completion::project_value_type_for_name(
                    project,
                    package,
                    file,
                    &path.segments[0].name,
                    limit,
                )
            })
        }
        ast::Expr::Name(path) if path.segments.len() == 2 => {
            let base = type_for_name(package, function, &path.segments[0].name, limit)?;
            field_type_in_project(project, package, file, &base, &path.segments[1].name, 0)
        }
        ast::Expr::Selector { base, field, .. } => {
            let base = resolve_expr_type(project, package, file, function, base, limit)?;
            field_type_in_project(project, package, file, &base, &field.name, 0)
        }
        ast::Expr::Unary {
            op: ast::UnaryOp::Deref,
            expr,
            ..
        } => match resolve_expr_type(project, package, file, function, expr, limit)? {
            ast::Type::Pointer(inner, _) => Some(*inner),
            _ => None,
        },
        ast::Expr::Index { base, .. } => {
            match resolve_expr_type(project, package, file, function, base, limit)? {
                ast::Type::Array { element, .. } => Some(*element),
                ast::Type::Pointer(inner, _) => Some(*inner),
                _ => None,
            }
        }
        ast::Expr::Cast { ty, .. } | ast::Expr::Composite { ty, .. } => Some(ty.clone()),
        ast::Expr::New { ty, span } => Some(ast::Type::Pointer(Box::new(ty.clone()), *span)),
        ast::Expr::Call { callee, .. } => {
            call_result_type(project, package, file, function, callee, limit)
        }
        _ => None,
    }
}

fn call_result_type(
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
    function: Option<CompletionScope<'_>>,
    callee: &ast::Expr,
    limit: usize,
) -> Option<ast::Type> {
    let ast::Expr::Name(path) = callee else {
        return None;
    };
    if path.segments.len() == 2 {
        let alias = &path.segments[0];
        let name = &path.segments[1];
        if has_local_binding(function, &alias.name, limit) || !is_exported(&name.name) {
            return None;
        }
        let import_path = import_path_for_alias(file, &alias.name)?;
        let imported = project
            .packages
            .iter()
            .find(|candidate| candidate.logical_path == import_path)?;
        let result = imported.files.iter().find_map(|file| {
            file.decls.iter().find_map(|decl| {
                let ast::TopLevelDecl::Func(decl) = decl else {
                    return None;
                };
                (decl.receiver.is_none() && decl.name.name == name.name)
                    .then(|| decl.signature.results.first().cloned())
                    .flatten()
            })
        })?;
        return Some(qualify_imported_type(result, alias));
    }
    if path.segments.len() != 1 || has_local_binding(function, &path.segments[0].name, limit) {
        return None;
    }
    let result = package.files.iter().find_map(|file| {
        file.decls.iter().find_map(|decl| {
            let ast::TopLevelDecl::Func(decl) = decl else {
                return None;
            };
            (decl.receiver.is_none() && decl.name.name == path.segments[0].name)
                .then(|| decl.signature.results.first().cloned())
                .flatten()
        })
    });
    if result.is_some() || package_declares_value(package, &path.segments[0].name) {
        return result;
    }
    builtin_result_type(&path.segments[0])
}

fn package_declares_value(package: &ast::Package, name: &str) -> bool {
    package.files.iter().any(|file| {
        file.decls.iter().any(|decl| match decl {
            ast::TopLevelDecl::Const(decl) => decl
                .specs
                .iter()
                .any(|spec| spec.names.iter().any(|candidate| candidate.name == name)),
            ast::TopLevelDecl::Var(decl) => decl
                .specs
                .iter()
                .any(|spec| spec.names.iter().any(|candidate| candidate.name == name)),
            ast::TopLevelDecl::Func(decl) => decl.receiver.is_none() && decl.name.name == name,
            ast::TopLevelDecl::Type(_) => false,
            ast::TopLevelDecl::Operator(_) => false,
        })
    })
}

fn builtin_result_type(name: &ast::Ident) -> Option<ast::Type> {
    let primitive = |text: &str| {
        ast::Type::Named(ast::Path {
            segments: vec![ast::Ident {
                name: text.to_string(),
                span: name.span,
            }],
            span: name.span,
        })
    };
    match name.name.as_str() {
        "thisFile" => Some(ast::Type::Pointer(Box::new(primitive("u8")), name.span)),
        "thisLine" => Some(primitive("u32")),
        _ => None,
    }
}

fn qualify_imported_type(ty: ast::Type, alias: &ast::Ident) -> ast::Type {
    match ty {
        ast::Type::Named(path)
            if path.segments.len() == 1
                && matches!(
                    path.segments[0].name.as_str(),
                    "bool" | "u8" | "u16" | "u32" | "i8" | "i16" | "i32" | "f32"
                ) =>
        {
            ast::Type::Named(path)
        }
        ast::Type::Named(path) if path.segments.len() == 1 => ast::Type::Named(ast::Path {
            span: path.span,
            segments: vec![alias.clone(), path.segments[0].clone()],
        }),
        ast::Type::Pointer(inner, span) => {
            ast::Type::Pointer(Box::new(qualify_imported_type(*inner, alias)), span)
        }
        ast::Type::Array { len, element, span } => ast::Type::Array {
            len,
            element: Box::new(qualify_imported_type(*element, alias)),
            span,
        },
        ast::Type::Apply {
            mut base,
            arguments,
            span,
        } if base.segments.len() == 1 => {
            base.segments.insert(0, alias.clone());
            ast::Type::Apply {
                base,
                arguments,
                span,
            }
        }
        other => other,
    }
}

fn type_for_name(
    package: &ast::Package,
    function: Option<CompletionScope<'_>>,
    name: &str,
    limit: usize,
) -> Option<ast::Type> {
    if has_local_binding(function, name, limit) {
        return local_binding(function, name, limit);
    }
    package.files.iter().find_map(|file| {
        file.decls.iter().find_map(|decl| match decl {
            ast::TopLevelDecl::Var(decl) => decl.specs.iter().find_map(|spec| {
                spec.names
                    .iter()
                    .position(|ident| ident.name == name)
                    .and_then(|index| {
                        declared_type(
                            spec.ty.as_ref(),
                            spec.values.get(index),
                            &VisibleLocals::new(),
                        )
                    })
            }),
            _ => None,
        })
    })
}

fn local_binding(
    function: Option<CompletionScope<'_>>,
    name: &str,
    limit: usize,
) -> Option<ast::Type> {
    visible_locals(function?, limit)
        .remove(name)
        .and_then(|binding| binding.ty)
}

fn local_binding_info(
    function: Option<CompletionScope<'_>>,
    name: &str,
    limit: usize,
) -> Option<LocalBinding> {
    visible_locals(function?, limit).remove(name)
}

fn has_local_binding(function: Option<CompletionScope<'_>>, name: &str, limit: usize) -> bool {
    function.is_some_and(|function| visible_locals(function, limit).contains_key(name))
}

#[derive(Debug, Clone)]
struct LocalBinding {
    ty: Option<ast::Type>,
    declaration: Span,
    scope_depth: usize,
}

type VisibleLocals = BTreeMap<String, LocalBinding>;

fn visible_locals(function: CompletionScope<'_>, offset: usize) -> VisibleLocals {
    let mut locals = VisibleLocals::new();
    if let Some(receiver) = function.receiver()
        && receiver.name.name != "_"
    {
        locals.insert(
            receiver.name.name.clone(),
            LocalBinding {
                ty: Some(receiver.ty.clone()),
                declaration: receiver.name.span,
                scope_depth: 0,
            },
        );
    }
    for param in function.params() {
        if param.name.name != "_" {
            locals.insert(
                param.name.name.clone(),
                LocalBinding {
                    ty: Some(param.ty.clone()),
                    declaration: param.name.span,
                    scope_depth: 0,
                },
            );
        }
    }
    collect_visible_in_block(function.body(), offset, 0, &mut locals);
    locals
}

fn collect_visible_in_block(
    block: &ast::Block,
    offset: usize,
    scope_depth: usize,
    locals: &mut VisibleLocals,
) {
    if !contains_offset(block.span, offset) {
        return;
    }
    for stmt in &block.statements {
        let span = stmt_span(stmt);
        if span.start >= offset {
            break;
        }
        if contains_offset(span, offset) {
            collect_visible_in_current_stmt(stmt, offset, scope_depth, locals);
            break;
        }
        add_direct_bindings(stmt, scope_depth, locals);
    }
}

fn collect_visible_in_current_stmt(
    stmt: &ast::Stmt,
    offset: usize,
    scope_depth: usize,
    locals: &mut VisibleLocals,
) {
    match stmt {
        ast::Stmt::Block(block) => collect_visible_in_block(block, offset, scope_depth + 1, locals),
        ast::Stmt::Defer(stmt) => {
            collect_visible_in_block(&stmt.block, offset, scope_depth + 1, locals)
        }
        ast::Stmt::If(stmt) => {
            if let Some(init) = stmt.init.as_deref() {
                if contains_offset(stmt_span(init), offset) {
                    return;
                }
                if stmt_span(init).end <= offset {
                    add_direct_bindings(init, scope_depth + 1, locals);
                }
            }
            if contains_offset(stmt.then_block.span, offset) {
                collect_visible_in_block(&stmt.then_block, offset, scope_depth + 2, locals);
            } else if let Some(other) = stmt.else_branch.as_deref() {
                if contains_offset(stmt_span(other), offset) {
                    collect_visible_in_current_stmt(other, offset, scope_depth + 1, locals);
                }
            }
        }
        ast::Stmt::For(stmt) => {
            if let ast::ForKind::ThreeClause { init, post, .. } = &stmt.kind {
                if let Some(init) = init.as_deref() {
                    if contains_offset(stmt_span(init), offset) {
                        return;
                    }
                    if stmt_span(init).end <= offset {
                        add_direct_bindings(init, scope_depth + 1, locals);
                    }
                }
                if post
                    .as_deref()
                    .is_some_and(|post| contains_offset(stmt_span(post), offset))
                {
                    return;
                }
            }
            if contains_offset(stmt.body.span, offset) {
                collect_visible_in_block(&stmt.body, offset, scope_depth + 2, locals);
            }
        }
        _ => {}
    }
}

fn add_direct_bindings(stmt: &ast::Stmt, scope_depth: usize, locals: &mut VisibleLocals) {
    match stmt {
        ast::Stmt::Const(decl) => {
            for spec in &decl.specs {
                for (index, name) in spec.names.iter().enumerate() {
                    let ty = declared_type(spec.ty.as_ref(), spec.values.get(index), locals);
                    insert_binding(locals, name, ty, scope_depth, false);
                }
            }
        }
        ast::Stmt::Var(decl) => {
            for spec in &decl.specs {
                for (index, name) in spec.names.iter().enumerate() {
                    let ty = declared_type(spec.ty.as_ref(), spec.values.get(index), locals);
                    insert_binding(locals, name, ty, scope_depth, false);
                }
            }
        }
        ast::Stmt::ShortVar(decl) => {
            for (name, value) in decl.names.iter().zip(&decl.values) {
                let ty = known_expr_type(value, locals);
                insert_binding(locals, name, ty, scope_depth, true);
            }
        }
        _ => {}
    }
}

fn declared_type(
    ty: Option<&ast::Type>,
    value: Option<&ast::Expr>,
    locals: &VisibleLocals,
) -> Option<ast::Type> {
    ty.cloned()
        .or_else(|| value.and_then(|value| known_expr_type(value, locals)))
}

fn known_expr_type(expr: &ast::Expr, locals: &VisibleLocals) -> Option<ast::Type> {
    match expr {
        ast::Expr::Cast { ty, .. } | ast::Expr::Composite { ty, .. } => Some(ty.clone()),
        ast::Expr::New { ty, span } => Some(ast::Type::Pointer(Box::new(ty.clone()), *span)),
        ast::Expr::Name(path) if path.segments.len() == 1 => locals
            .get(&path.segments[0].name)
            .and_then(|binding| binding.ty.clone()),
        _ => None,
    }
}

fn insert_binding(
    locals: &mut VisibleLocals,
    name: &ast::Ident,
    ty: Option<ast::Type>,
    scope_depth: usize,
    reuse_in_scope: bool,
) {
    if name.name != "_" {
        if reuse_in_scope
            && locals
                .get(&name.name)
                .is_some_and(|binding| binding.scope_depth == scope_depth)
        {
            return;
        }
        locals.insert(
            name.name.clone(),
            LocalBinding {
                ty,
                declaration: name.span,
                scope_depth,
            },
        );
    }
}

fn contains_offset(span: Span, offset: usize) -> bool {
    span.start <= offset && offset <= span.end
}

fn resolve_field(package: &ast::Package, ty: &ast::Type, name: &str, depth: usize) -> Option<Span> {
    if depth > 16 {
        return None;
    }
    match ty {
        ast::Type::Pointer(inner, _) => resolve_field(package, inner, name, depth + 1),
        ast::Type::Struct { fields, .. } => fields
            .iter()
            .find(|field| field.name.name == name)
            .map(|field| field.name.span),
        ast::Type::Named(path) if path.segments.len() == 1 => {
            find_type(package, &path.segments[0].name)
                .and_then(|ty| resolve_field(package, ty, name, depth + 1))
        }
        _ => None,
    }
}

fn field_type_in_project(
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
    ty: &ast::Type,
    name: &str,
    depth: usize,
) -> Option<ast::Type> {
    if depth > 16 {
        return None;
    }
    match ty {
        ast::Type::Pointer(inner, _) => {
            field_type_in_project(project, package, file, inner, name, depth + 1)
        }
        ast::Type::Struct { fields, .. } => fields
            .iter()
            .find(|field| field.name.name == name)
            .map(|field| field.ty.clone()),
        ast::Type::Named(path) if path.segments.len() == 1 => {
            find_type(package, &path.segments[0].name)
                .and_then(|ty| field_type_in_project(project, package, file, ty, name, depth + 1))
        }
        ast::Type::Named(path) if path.segments.len() == 2 => {
            let import_path = import_path_for_alias(file, &path.segments[0].name)?;
            let imported = project
                .packages
                .iter()
                .find(|candidate| candidate.logical_path == import_path)?;
            let imported_ty = find_type(imported, &path.segments[1].name)?;
            // Once the qualified type is entered, its unqualified nested types belong to the
            // imported package. Any file in that package carries the relevant imports.
            let imported_file = imported.files.first()?;
            let ty = field_type_in_project(
                project,
                imported,
                imported_file,
                imported_ty,
                name,
                depth + 1,
            )?;
            Some(qualify_imported_type(ty, &path.segments[0]))
        }
        ast::Type::Apply {
            base, arguments, ..
        } if base.segments.len() == 1 => {
            let spec = find_type_spec(package, &base.segments[0].name)?;
            let ty = substitute_type_spec(spec, arguments)?;
            field_type_in_project(project, package, file, &ty, name, depth + 1)
        }
        ast::Type::Apply {
            base, arguments, ..
        } if base.segments.len() == 2 => {
            let import_path = import_path_for_alias(file, &base.segments[0].name)?;
            let imported = project
                .packages
                .iter()
                .find(|candidate| candidate.logical_path == import_path)?;
            let spec = find_type_spec(imported, &base.segments[1].name)?;
            let ty = substitute_type_spec(spec, arguments)?;
            let imported_file = imported.files.first()?;
            let ty = field_type_in_project(project, imported, imported_file, &ty, name, depth + 1)?;
            Some(qualify_imported_type(ty, &base.segments[0]))
        }
        _ => None,
    }
}

fn find_type_spec<'a>(package: &'a ast::Package, name: &str) -> Option<&'a ast::TypeSpec> {
    package.files.iter().find_map(|file| {
        file.decls.iter().find_map(|decl| {
            let ast::TopLevelDecl::Type(decl) = decl else {
                return None;
            };
            decl.specs.iter().find(|spec| spec.name.name == name)
        })
    })
}

fn substitute_type_spec(spec: &ast::TypeSpec, arguments: &[ast::Type]) -> Option<ast::Type> {
    if spec.type_params.len() != arguments.len() {
        return None;
    }
    let substitutions = spec
        .type_params
        .iter()
        .zip(arguments)
        .map(|(parameter, argument)| (parameter.name.as_str(), argument))
        .collect::<BTreeMap<_, _>>();
    Some(substitute_type(&spec.ty, &substitutions))
}

fn substitute_type(ty: &ast::Type, substitutions: &BTreeMap<&str, &ast::Type>) -> ast::Type {
    match ty {
        ast::Type::Named(path) if path.segments.len() == 1 => substitutions
            .get(path.segments[0].name.as_str())
            .map_or_else(|| ty.clone(), |ty| (*ty).clone()),
        ast::Type::Named(_) | ast::Type::Resolved(_, _) => ty.clone(),
        ast::Type::Apply {
            base,
            arguments,
            span,
        } => ast::Type::Apply {
            base: base.clone(),
            arguments: arguments
                .iter()
                .map(|argument| substitute_type(argument, substitutions))
                .collect(),
            span: *span,
        },
        ast::Type::Pointer(inner, span) => {
            ast::Type::Pointer(Box::new(substitute_type(inner, substitutions)), *span)
        }
        ast::Type::Array { len, element, span } => ast::Type::Array {
            len: len.clone(),
            element: Box::new(substitute_type(element, substitutions)),
            span: *span,
        },
        ast::Type::Struct { fields, span } => ast::Type::Struct {
            fields: fields
                .iter()
                .map(|field| ast::Field {
                    name: field.name.clone(),
                    ty: substitute_type(&field.ty, substitutions),
                    span: field.span,
                })
                .collect(),
            span: *span,
        },
        ast::Type::Interface { methods, span } => ast::Type::Interface {
            methods: methods
                .iter()
                .map(|method| ast::InterfaceMethod {
                    name: method.name.clone(),
                    signature: substitute_type_signature(&method.signature, substitutions),
                    span: method.span,
                })
                .collect(),
            span: *span,
        },
        ast::Type::Func { signature, span } => ast::Type::Func {
            signature: substitute_type_signature(signature, substitutions),
            span: *span,
        },
    }
}

fn substitute_type_signature(
    signature: &ast::TypeSignature,
    substitutions: &BTreeMap<&str, &ast::Type>,
) -> ast::TypeSignature {
    ast::TypeSignature {
        params: signature
            .params
            .iter()
            .map(|parameter| ast::TypeParameter {
                name: parameter.name.clone(),
                ty: substitute_type(&parameter.ty, substitutions),
                span: parameter.span,
            })
            .collect(),
        results: signature
            .results
            .iter()
            .map(|result| substitute_type(result, substitutions))
            .collect(),
        span: signature.span,
    }
}

fn find_type<'a>(package: &'a ast::Package, name: &str) -> Option<&'a ast::Type> {
    package.files.iter().find_map(|file| {
        file.decls.iter().find_map(|decl| {
            let ast::TopLevelDecl::Type(decl) = decl else {
                return None;
            };
            decl.specs
                .iter()
                .find(|spec| spec.name.name == name)
                .map(|spec| &spec.ty)
        })
    })
}

fn import_path_for_alias<'a>(file: &'a ast::File, alias: &str) -> Option<&'a str> {
    file.imports.iter().find_map(|import| {
        let path = import.path.trim_matches('"');
        let import_name = import
            .alias
            .as_ref()
            .map(|name| name.name.as_str())
            .or_else(|| path.rsplit('/').next());
        (import_name == Some(alias)).then_some(path)
    })
}

fn is_exported(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
}

fn contains(span: Span, offset: usize) -> bool {
    span.start <= offset && offset < span.end
}

fn stmt_span(stmt: &ast::Stmt) -> Span {
    match stmt {
        ast::Stmt::Const(decl) => decl.span,
        ast::Stmt::Var(decl) => decl.span,
        ast::Stmt::ShortVar(decl) => decl.span,
        ast::Stmt::Assign(stmt) => stmt.span,
        ast::Stmt::Expr(stmt) => stmt.span,
        ast::Stmt::Return(stmt) => stmt.span,
        ast::Stmt::Defer(stmt) => stmt.span,
        ast::Stmt::Trap { span, .. } => *span,
        ast::Stmt::Break(span) | ast::Stmt::Continue(span) => *span,
        ast::Stmt::Block(block) => block.span,
        ast::Stmt::If(stmt) => stmt.span,
        ast::Stmt::For(stmt) => stmt.span,
    }
}
