use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use compiler::ir::ast;
use compiler::source::{SourceDb, Span};
use serde_json::{Value, json};

use crate::hover_reference::{ResolvedTarget, resolve_reference};
use crate::protocol::{Position, lsp_offset};
use crate::symbols::span_to_range;
use crate::util::uri_from_path;

#[derive(Debug, Clone, Copy)]
struct SymbolAt {
    target: Span,
    occurrence: Span,
}

pub fn prepare_rename(
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
    sources: &SourceDb,
    position: &Position,
    editable_root: Option<&Path>,
) -> Option<Value> {
    let symbol = symbol_at(project, package, file, sources, position)?;
    if !target_is_editable(symbol.target, sources, editable_root) {
        return None;
    }
    let placeholder = sources
        .file(symbol.occurrence.file)
        .slice(symbol.occurrence);
    if placeholder == "_" {
        return None;
    }
    Some(json!({
        "range": span_to_range(symbol.occurrence, sources),
        "placeholder": placeholder
    }))
}

pub fn references(
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
    sources: &SourceDb,
    position: &Position,
    include_declaration: bool,
) -> Value {
    let Some(symbol) = symbol_at(project, package, file, sources, position) else {
        return Value::Array(Vec::new());
    };
    Value::Array(
        occurrences(project, symbol.target)
            .into_iter()
            .filter(|span| include_declaration || *span != symbol.target)
            .filter_map(|span| {
                let uri = uri_from_path(sources.file(span.file).path()).ok()?;
                Some(json!({
                    "uri": uri,
                    "range": span_to_range(span, sources)
                }))
            })
            .collect(),
    )
}

pub fn rename(
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
    sources: &SourceDb,
    position: &Position,
    editable_root: Option<&Path>,
    new_name: &str,
) -> Result<Value, String> {
    if !valid_identifier(new_name) || new_name == "_" {
        return Err(format!(
            "`{new_name}` is not a valid Ond identifier for rename"
        ));
    }
    let symbol = symbol_at(project, package, file, sources, position)
        .ok_or_else(|| "the selected position is not a renameable symbol".to_string())?;
    if !target_is_editable(symbol.target, sources, editable_root) {
        return Err("symbols declared outside the current project cannot be renamed".to_string());
    }

    let old_name = sources.file(symbol.target.file).slice(symbol.target);
    if old_name == "_" {
        return Err("the blank identifier cannot be renamed".to_string());
    }
    if old_name == new_name {
        return Ok(json!({ "changes": {} }));
    }

    reject_direct_conflict(project, sources, symbol.target, new_name)?;
    let occurrences = occurrences(project, symbol.target);
    if !is_exported(new_name) && has_cross_package_reference(project, symbol.target, &occurrences) {
        return Err(format!(
            "`{new_name}` would not be exported, but the symbol is referenced from another package"
        ));
    }

    let mut changes = BTreeMap::<String, Vec<Value>>::new();
    for span in occurrences {
        let path = sources.file(span.file).path();
        if editable_root.is_some_and(|root| !path.starts_with(root)) {
            return Err(format!(
                "rename would modify a source outside the current project: {}",
                path.display()
            ));
        }
        let uri = uri_from_path(path)?;
        changes.entry(uri).or_default().push(json!({
            "range": span_to_range(span, sources),
            "newText": new_name
        }));
    }
    Ok(json!({ "changes": changes }))
}

fn has_cross_package_reference(project: &ast::Project, target: Span, occurrences: &[Span]) -> bool {
    let owner = project
        .packages
        .iter()
        .position(|package| package.files.iter().any(|file| file.file_id == target.file));
    let Some(owner) = owner else {
        return false;
    };
    occurrences.iter().any(|occurrence| {
        project
            .packages
            .iter()
            .position(|package| {
                package
                    .files
                    .iter()
                    .any(|file| file.file_id == occurrence.file)
            })
            .is_some_and(|package| package != owner)
    })
}

fn is_exported(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|character| character.is_ascii_uppercase())
}

fn symbol_at(
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
    sources: &SourceDb,
    position: &Position,
) -> Option<SymbolAt> {
    let offset = lsp_offset(sources.file(file.file_id).text(), position).ok()?;
    if let Some(resolved) = resolve_reference(project, package, file, offset)
        && let ResolvedTarget::Declaration(target) = resolved.target
        && resolved.reference.start <= offset
        && offset < resolved.reference.end
    {
        return Some(SymbolAt {
            target,
            occurrence: resolved.reference,
        });
    }

    declaration_records(project)
        .into_iter()
        .find(|record| {
            record.span.file == file.file_id
                && record.span.start <= offset
                && offset < record.span.end
        })
        .map(|record| SymbolAt {
            target: record.span,
            occurrence: record.span,
        })
}

fn occurrences(project: &ast::Project, target: Span) -> Vec<Span> {
    let declarations = declaration_records(project)
        .into_iter()
        .map(|record| record.span)
        .collect::<HashSet<_>>();
    let mut result = HashSet::new();
    for package in &project.packages {
        for file in &package.files {
            for span in identifier_spans(file) {
                let resolved_target = resolve_reference(project, package, file, span.start)
                    .and_then(|resolved| (resolved.reference == span).then_some(resolved.target));
                let matches = match resolved_target {
                    Some(ResolvedTarget::Declaration(declaration)) => declaration == target,
                    Some(ResolvedTarget::Package(_)) => false,
                    None => declarations.contains(&span) && span == target,
                };
                if matches {
                    result.insert(span);
                }
            }
        }
    }
    let mut result = result.into_iter().collect::<Vec<_>>();
    result.sort_by_key(|span| (span.file, span.start, span.end));
    result
}

fn target_is_editable(target: Span, sources: &SourceDb, editable_root: Option<&Path>) -> bool {
    let path = sources.file(target.file).path();
    editable_root.is_none_or(|root| path.starts_with(root))
}

fn valid_identifier(name: &str) -> bool {
    if name.is_empty() || name.contains(['\r', '\n']) {
        return false;
    }
    let source = format!("package rename_check\nvar {name}: i32\n");
    let mut sources = SourceDb::default();
    let file = sources.add_file("rename-check.ond".into(), source.clone());
    compiler::parse_source_file(file, &source).is_ok()
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum DeclarationScope {
    Package(String),
    Local(Span),
    Fields(Span),
    Methods(String),
}

#[derive(Debug, Clone)]
struct DeclarationRecord {
    span: Span,
    name: String,
    scope: DeclarationScope,
    short: bool,
}

fn reject_direct_conflict(
    project: &ast::Project,
    sources: &SourceDb,
    target: Span,
    new_name: &str,
) -> Result<(), String> {
    let records = declaration_records(project);
    let Some(target_record) = records.iter().find(|record| record.span == target) else {
        return Ok(());
    };
    if records.iter().any(|record| {
        record.span != target
            && record.scope == target_record.scope
            && record.name == new_name
            && (!record.short || short_declaration_introduces(project, record.span))
    }) {
        let path = sources.file(target.file).path();
        return Err(format!(
            "renaming `{}` to `{new_name}` would conflict with another declaration in {}",
            target_record.name,
            path.display()
        ));
    }
    Ok(())
}

fn short_declaration_introduces(project: &ast::Project, span: Span) -> bool {
    project.packages.iter().any(|package| {
        package.files.iter().any(|file| {
            file.file_id == span.file
                && resolve_reference(project, package, file, span.start).is_some_and(|resolved| {
                    resolved.reference == span
                        && matches!(resolved.target, ResolvedTarget::Declaration(target) if target == span)
                })
        })
    })
}

fn declaration_records(project: &ast::Project) -> Vec<DeclarationRecord> {
    let mut records = Vec::new();
    for package in &project.packages {
        let package_scope = DeclarationScope::Package(package.logical_path.clone());
        for file in &package.files {
            for decl in &file.decls {
                collect_top_declarations(decl, &package_scope, &mut records);
            }
        }
    }
    records
}

fn collect_top_declarations(
    decl: &ast::TopLevelDecl,
    package_scope: &DeclarationScope,
    records: &mut Vec<DeclarationRecord>,
) {
    match decl {
        ast::TopLevelDecl::Const(decl) => collect_const_declarations(decl, package_scope, records),
        ast::TopLevelDecl::Var(decl) => collect_var_declarations(decl, package_scope, records),
        ast::TopLevelDecl::Type(decl) => {
            for spec in &decl.specs {
                push_declaration(&spec.name, package_scope, false, records);
                collect_type_declarations(&spec.ty, records);
            }
        }
        ast::TopLevelDecl::Func(function) => {
            let method_scope = function.receiver.as_ref().and_then(|receiver| {
                receiver_type_name(&receiver.ty).map(|name| {
                    let package = match package_scope {
                        DeclarationScope::Package(package) => package.as_str(),
                        _ => unreachable!("top-level declaration has package scope"),
                    };
                    DeclarationScope::Methods(format!("{package}.{name}"))
                })
            });
            push_declaration(
                &function.name,
                method_scope.as_ref().unwrap_or(package_scope),
                false,
                records,
            );
            let local_scope = DeclarationScope::Local(function.span);
            for parameter in &function.type_params {
                push_declaration(parameter, &local_scope, false, records);
            }
            if let Some(receiver) = &function.receiver {
                push_declaration(&receiver.name, &local_scope, false, records);
                collect_type_declarations(&receiver.ty, records);
            }
            for param in &function.signature.params {
                push_declaration(&param.name, &local_scope, false, records);
                collect_type_declarations(&param.ty, records);
            }
            for result in &function.signature.results {
                collect_type_declarations(result, records);
            }
            collect_block_declarations(&function.body, &local_scope, records);
        }
        ast::TopLevelDecl::Operator(operator) => {
            let local_scope = DeclarationScope::Local(operator.span);
            for parameter in &operator.type_params {
                push_declaration(parameter, &local_scope, false, records);
            }
            for parameter in &operator.signature.params {
                push_declaration(&parameter.name, &local_scope, false, records);
                collect_type_declarations(&parameter.ty, records);
            }
            for result in &operator.signature.results {
                collect_type_declarations(result, records);
            }
            collect_block_declarations(&operator.body, &local_scope, records);
        }
    }
}

fn receiver_type_name(ty: &ast::Type) -> Option<&str> {
    match ty {
        ast::Type::Named(path) if path.segments.len() == 1 => Some(&path.segments[0].name),
        ast::Type::Apply { base, .. } if base.segments.len() == 1 => Some(&base.segments[0].name),
        ast::Type::Pointer(inner, _) => match inner.as_ref() {
            ast::Type::Named(path) if path.segments.len() == 1 => Some(&path.segments[0].name),
            ast::Type::Apply { base, .. } if base.segments.len() == 1 => {
                Some(&base.segments[0].name)
            }
            _ => None,
        },
        _ => None,
    }
}

fn collect_const_declarations(
    decl: &ast::ConstDecl,
    scope: &DeclarationScope,
    records: &mut Vec<DeclarationRecord>,
) {
    for spec in &decl.specs {
        for name in &spec.names {
            push_declaration(name, scope, false, records);
        }
        if let Some(ty) = &spec.ty {
            collect_type_declarations(ty, records);
        }
    }
}

fn collect_var_declarations(
    decl: &ast::VarDecl,
    scope: &DeclarationScope,
    records: &mut Vec<DeclarationRecord>,
) {
    for spec in &decl.specs {
        for name in &spec.names {
            push_declaration(name, scope, false, records);
        }
        if let Some(ty) = &spec.ty {
            collect_type_declarations(ty, records);
        }
    }
}

fn collect_block_declarations(
    block: &ast::Block,
    scope: &DeclarationScope,
    records: &mut Vec<DeclarationRecord>,
) {
    for stmt in &block.statements {
        collect_stmt_declarations(stmt, scope, records);
    }
}

fn collect_stmt_declarations(
    stmt: &ast::Stmt,
    scope: &DeclarationScope,
    records: &mut Vec<DeclarationRecord>,
) {
    match stmt {
        ast::Stmt::Const(decl) => collect_const_declarations(decl, scope, records),
        ast::Stmt::Var(decl) => collect_var_declarations(decl, scope, records),
        ast::Stmt::ShortVar(decl) => {
            for name in &decl.names {
                push_declaration(name, scope, true, records);
            }
        }
        ast::Stmt::Block(block) => {
            let nested = DeclarationScope::Local(block.span);
            collect_block_declarations(block, &nested, records);
        }
        ast::Stmt::Defer(stmt) => {
            let nested = DeclarationScope::Local(stmt.block.span);
            collect_block_declarations(&stmt.block, &nested, records);
        }
        ast::Stmt::If(stmt) => {
            if let Some(init) = stmt.init.as_deref() {
                let init_scope = DeclarationScope::Local(stmt.span);
                collect_stmt_declarations(init, &init_scope, records);
            }
            let then_scope = DeclarationScope::Local(stmt.then_block.span);
            collect_block_declarations(&stmt.then_block, &then_scope, records);
            if let Some(other) = stmt.else_branch.as_deref() {
                let else_scope = DeclarationScope::Local(stmt.span);
                collect_stmt_declarations(other, &else_scope, records);
            }
        }
        ast::Stmt::For(stmt) => {
            if let ast::ForKind::ThreeClause { init, post, .. } = &stmt.kind {
                let header_scope = DeclarationScope::Local(stmt.span);
                if let Some(init) = init.as_deref() {
                    collect_stmt_declarations(init, &header_scope, records);
                }
                if let Some(post) = post.as_deref() {
                    collect_stmt_declarations(post, &header_scope, records);
                }
            }
            let body_scope = DeclarationScope::Local(stmt.body.span);
            collect_block_declarations(&stmt.body, &body_scope, records);
        }
        ast::Stmt::Assign(_)
        | ast::Stmt::Expr(_)
        | ast::Stmt::Return(_)
        | ast::Stmt::Break(_)
        | ast::Stmt::Continue(_)
        | ast::Stmt::Trap { .. } => {}
    }
}

fn collect_type_declarations(ty: &ast::Type, records: &mut Vec<DeclarationRecord>) {
    match ty {
        ast::Type::Named(_) => {}
        ast::Type::Apply { arguments, .. } => {
            for argument in arguments {
                collect_type_declarations(argument, records);
            }
        }
        ast::Type::Resolved(_, _) => {}
        ast::Type::Pointer(inner, _) => collect_type_declarations(inner, records),
        ast::Type::Array { element, .. } => collect_type_declarations(element, records),
        ast::Type::Struct { fields, span } => {
            let scope = DeclarationScope::Fields(*span);
            for field in fields {
                push_declaration(&field.name, &scope, false, records);
                collect_type_declarations(&field.ty, records);
            }
        }
        ast::Type::Interface { methods, span } => {
            let scope = DeclarationScope::Fields(*span);
            for method in methods {
                push_declaration(&method.name, &scope, false, records);
                for parameter in &method.signature.params {
                    collect_type_declarations(&parameter.ty, records);
                }
                for result in &method.signature.results {
                    collect_type_declarations(result, records);
                }
            }
        }
        ast::Type::Func { signature, .. } => {
            for param in &signature.params {
                collect_type_declarations(&param.ty, records);
            }
            for result in &signature.results {
                collect_type_declarations(result, records);
            }
        }
    }
}

fn push_declaration(
    ident: &ast::Ident,
    scope: &DeclarationScope,
    short: bool,
    records: &mut Vec<DeclarationRecord>,
) {
    if ident.name != "_" {
        records.push(DeclarationRecord {
            span: ident.span,
            name: ident.name.clone(),
            scope: scope.clone(),
            short,
        });
    }
}

fn identifier_spans(file: &ast::File) -> Vec<Span> {
    let mut spans = Vec::new();
    for import in &file.imports {
        if let Some(alias) = &import.alias {
            spans.push(alias.span);
        }
    }
    for decl in &file.decls {
        collect_top_identifier_spans(decl, &mut spans);
    }
    spans
}

fn collect_top_identifier_spans(decl: &ast::TopLevelDecl, spans: &mut Vec<Span>) {
    match decl {
        ast::TopLevelDecl::Const(decl) => collect_const_identifier_spans(decl, spans),
        ast::TopLevelDecl::Var(decl) => collect_var_identifier_spans(decl, spans),
        ast::TopLevelDecl::Type(decl) => {
            for spec in &decl.specs {
                spans.push(spec.name.span);
                collect_type_identifier_spans(&spec.ty, spans);
            }
        }
        ast::TopLevelDecl::Func(function) => {
            spans.push(function.name.span);
            spans.extend(function.type_params.iter().map(|parameter| parameter.span));
            if let Some(receiver) = &function.receiver {
                spans.push(receiver.name.span);
                collect_type_identifier_spans(&receiver.ty, spans);
            }
            for param in &function.signature.params {
                spans.push(param.name.span);
                collect_type_identifier_spans(&param.ty, spans);
            }
            for result in &function.signature.results {
                collect_type_identifier_spans(result, spans);
            }
            collect_block_identifier_spans(&function.body, spans);
        }
        ast::TopLevelDecl::Operator(operator) => {
            spans.extend(operator.type_params.iter().map(|parameter| parameter.span));
            for parameter in &operator.signature.params {
                spans.push(parameter.name.span);
                collect_type_identifier_spans(&parameter.ty, spans);
            }
            for result in &operator.signature.results {
                collect_type_identifier_spans(result, spans);
            }
            collect_block_identifier_spans(&operator.body, spans);
        }
    }
}

fn collect_const_identifier_spans(decl: &ast::ConstDecl, spans: &mut Vec<Span>) {
    for spec in &decl.specs {
        spans.extend(spec.names.iter().map(|name| name.span));
        if let Some(ty) = &spec.ty {
            collect_type_identifier_spans(ty, spans);
        }
        for value in &spec.values {
            collect_expr_identifier_spans(value, spans);
        }
    }
}

fn collect_var_identifier_spans(decl: &ast::VarDecl, spans: &mut Vec<Span>) {
    for spec in &decl.specs {
        spans.extend(spec.names.iter().map(|name| name.span));
        if let Some(ty) = &spec.ty {
            collect_type_identifier_spans(ty, spans);
        }
        for value in &spec.values {
            collect_expr_identifier_spans(value, spans);
        }
    }
}

fn collect_block_identifier_spans(block: &ast::Block, spans: &mut Vec<Span>) {
    for stmt in &block.statements {
        collect_stmt_identifier_spans(stmt, spans);
    }
}

fn collect_stmt_identifier_spans(stmt: &ast::Stmt, spans: &mut Vec<Span>) {
    match stmt {
        ast::Stmt::Const(decl) => collect_const_identifier_spans(decl, spans),
        ast::Stmt::Var(decl) => collect_var_identifier_spans(decl, spans),
        ast::Stmt::ShortVar(decl) => {
            spans.extend(decl.names.iter().map(|name| name.span));
            for value in &decl.values {
                collect_expr_identifier_spans(value, spans);
            }
        }
        ast::Stmt::Assign(stmt) => {
            for expr in stmt.targets.iter().chain(&stmt.values) {
                collect_expr_identifier_spans(expr, spans);
            }
        }
        ast::Stmt::Expr(stmt) => collect_expr_identifier_spans(&stmt.expr, spans),
        ast::Stmt::Return(stmt) => {
            for value in &stmt.values {
                collect_expr_identifier_spans(value, spans);
            }
        }
        ast::Stmt::Block(block) => collect_block_identifier_spans(block, spans),
        ast::Stmt::Defer(stmt) => collect_block_identifier_spans(&stmt.block, spans),
        ast::Stmt::If(stmt) => {
            if let Some(init) = stmt.init.as_deref() {
                collect_stmt_identifier_spans(init, spans);
            }
            collect_expr_identifier_spans(&stmt.condition, spans);
            collect_block_identifier_spans(&stmt.then_block, spans);
            if let Some(other) = stmt.else_branch.as_deref() {
                collect_stmt_identifier_spans(other, spans);
            }
        }
        ast::Stmt::For(stmt) => {
            match &stmt.kind {
                ast::ForKind::Infinite => {}
                ast::ForKind::While(condition) => collect_expr_identifier_spans(condition, spans),
                ast::ForKind::ThreeClause {
                    init,
                    condition,
                    post,
                } => {
                    if let Some(init) = init.as_deref() {
                        collect_stmt_identifier_spans(init, spans);
                    }
                    if let Some(condition) = condition {
                        collect_expr_identifier_spans(condition, spans);
                    }
                    if let Some(post) = post.as_deref() {
                        collect_stmt_identifier_spans(post, spans);
                    }
                }
            }
            collect_block_identifier_spans(&stmt.body, spans);
        }
        ast::Stmt::Break(_) | ast::Stmt::Continue(_) | ast::Stmt::Trap { .. } => {}
    }
}

fn collect_type_identifier_spans(ty: &ast::Type, spans: &mut Vec<Span>) {
    match ty {
        ast::Type::Named(path) => spans.extend(path.segments.iter().map(|ident| ident.span)),
        ast::Type::Apply {
            base, arguments, ..
        } => {
            spans.extend(base.segments.iter().map(|ident| ident.span));
            for argument in arguments {
                collect_type_identifier_spans(argument, spans);
            }
        }
        ast::Type::Resolved(_, _) => {}
        ast::Type::Pointer(inner, _) => collect_type_identifier_spans(inner, spans),
        ast::Type::Array { len, element, .. } => {
            if let Some(len) = len {
                collect_expr_identifier_spans(len, spans);
            }
            collect_type_identifier_spans(element, spans);
        }
        ast::Type::Struct { fields, .. } => {
            for field in fields {
                spans.push(field.name.span);
                collect_type_identifier_spans(&field.ty, spans);
            }
        }
        ast::Type::Interface { methods, .. } => {
            for method in methods {
                spans.push(method.name.span);
                for parameter in &method.signature.params {
                    if let Some(name) = &parameter.name {
                        spans.push(name.span);
                    }
                    collect_type_identifier_spans(&parameter.ty, spans);
                }
                for result in &method.signature.results {
                    collect_type_identifier_spans(result, spans);
                }
            }
        }
        ast::Type::Func { signature, .. } => {
            for param in &signature.params {
                if let Some(name) = &param.name {
                    spans.push(name.span);
                }
                collect_type_identifier_spans(&param.ty, spans);
            }
            for result in &signature.results {
                collect_type_identifier_spans(result, spans);
            }
        }
    }
}

fn collect_expr_identifier_spans(expr: &ast::Expr, spans: &mut Vec<Span>) {
    match expr {
        ast::Expr::Literal(_) => {}
        ast::Expr::Name(path) => spans.extend(path.segments.iter().map(|ident| ident.span)),
        ast::Expr::Unary { expr, .. } => collect_expr_identifier_spans(expr, spans),
        ast::Expr::Binary { lhs, rhs, .. } => {
            collect_expr_identifier_spans(lhs, spans);
            collect_expr_identifier_spans(rhs, spans);
        }
        ast::Expr::Call { callee, args, .. } => {
            collect_expr_identifier_spans(callee, spans);
            for arg in args {
                collect_expr_identifier_spans(arg, spans);
            }
        }
        ast::Expr::TypeApply {
            base, arguments, ..
        } => {
            collect_expr_identifier_spans(base, spans);
            for ty in arguments {
                collect_type_identifier_spans(ty, spans);
            }
        }
        ast::Expr::Selector { base, field, .. } => {
            collect_expr_identifier_spans(base, spans);
            spans.push(field.span);
        }
        ast::Expr::Index { base, index, .. } => {
            collect_expr_identifier_spans(base, spans);
            collect_expr_identifier_spans(index, spans);
        }
        ast::Expr::Cast { expr, ty, .. } => {
            collect_expr_identifier_spans(expr, spans);
            collect_type_identifier_spans(ty, spans);
        }
        ast::Expr::Composite { ty, elements, .. } => {
            collect_type_identifier_spans(ty, spans);
            for element in elements {
                if let Some(key) = &element.key {
                    collect_expr_identifier_spans(key, spans);
                }
                collect_expr_identifier_spans(&element.value, spans);
            }
        }
        ast::Expr::New { ty, .. } => collect_type_identifier_spans(ty, spans),
        ast::Expr::LayoutQuery { ty, .. } => collect_type_identifier_spans(ty, spans),
    }
}

#[cfg(test)]
mod tests {
    use compiler::ir::ast;
    use compiler::source::SourceDb;

    use super::{prepare_rename, references, rename};
    use crate::protocol::Position;

    fn position(text: &str, needle: &str, occurrence: usize) -> Position {
        let offset = text.match_indices(needle).nth(occurrence).unwrap().0;
        let prefix = &text[..offset];
        let line = prefix.bytes().filter(|byte| *byte == b'\n').count() as u32;
        let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
        let character = text[line_start..offset].encode_utf16().count() as u32;
        Position { line, character }
    }

    fn fixture(text: &str) -> (SourceDb, ast::Project, ast::Package, ast::File) {
        let mut sources = SourceDb::default();
        let path = std::env::current_dir().unwrap().join("rename.ond");
        let file_id = sources.add_file(path, text.to_string());
        let file = compiler::parse_source_file(file_id, text).unwrap();
        let package = ast::Package {
            logical_path: ".".to_string(),
            files: vec![file.clone()],
        };
        let project = ast::Project {
            packages: vec![package.clone()],
        };
        (sources, project, package, file)
    }

    #[test]
    fn renames_only_references_to_the_selected_local() {
        let text = "package main\nfunc run(value: i32) -> i32 {\n    result := value\n    {\n        value := 2\n        result = result + value\n    }\n    return result + value\n}\n";
        let (sources, project, package, file) = fixture(text);
        let edit = rename(
            &project,
            &package,
            &file,
            &sources,
            &position(text, "value", 0),
            None,
            "input",
        )
        .unwrap();
        let edits = edit["changes"]
            .as_object()
            .unwrap()
            .values()
            .next()
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(edits.len(), 3, "{edit}");
    }

    #[test]
    fn renames_struct_field_access_and_composite_keys() {
        let text = "package main\ntype Item struct { Value: i32 }\nfunc run(item: Item) -> i32 {\n    other := Item{Value: 1}\n    return item.Value + other.Value\n}\n";
        let (sources, project, package, file) = fixture(text);
        let edit = rename(
            &project,
            &package,
            &file,
            &sources,
            &position(text, "Value", 0),
            None,
            "Amount",
        )
        .unwrap();
        let edits = edit["changes"]
            .as_object()
            .unwrap()
            .values()
            .next()
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(edits.len(), 4, "{edit}");
    }

    #[test]
    fn renames_types_type_parameters_and_values_through_operator_scopes() {
        let text = r#"package main
type Box[T] struct { Value: T }

operator[T] +(left: Box[T], right: Box[T]) -> Box[T] {
    result := Box[T]{Value: left.Value}
    {
        right := left
        _ = right.Value
    }
    result.Value = result.Value
    return Box[T]{Value: right.Value}
}

func main() {}
"#;
        let (sources, project, package, file) = fixture(text);

        for (needle, occurrence, new_name, expected_edits) in [
            ("Box", 0, "Container", 6),
            ("T", 2, "Element", 6),
            ("left", 0, "lhs", 3),
            ("right", 0, "rhs", 2),
            ("result", 0, "sum", 3),
            ("Value", 0, "Item", 8),
        ] {
            let edit = rename(
                &project,
                &package,
                &file,
                &sources,
                &position(text, needle, occurrence),
                None,
                new_name,
            )
            .unwrap_or_else(|error| panic!("failed to rename {needle}: {error}"));
            let edits = edit["changes"]
                .as_object()
                .unwrap()
                .values()
                .next()
                .unwrap()
                .as_array()
                .unwrap();
            assert_eq!(edits.len(), expected_edits, "rename {needle}: {edit}");
            assert!(
                edits.iter().all(|edit| edit["newText"] == new_name),
                "rename {needle}: {edit}"
            );
        }
    }

    #[test]
    fn rejects_invalid_names_and_direct_conflicts() {
        let text =
            "package main\nfunc run(first: i32, second: i32) -> i32 { return first + second }\n";
        let (sources, project, package, file) = fixture(text);
        let selected = position(text, "first", 0);
        assert!(rename(&project, &package, &file, &sources, &selected, None, "if").is_err());
        assert!(
            rename(
                &project, &package, &file, &sources, &selected, None, "second"
            )
            .is_err()
        );
    }

    #[test]
    fn prepares_and_lists_references() {
        let text = "package main\nconst Count: i32 = 1\nfunc run() -> i32 { return Count }\n";
        let (sources, project, package, file) = fixture(text);
        let selected = position(text, "Count", 1);
        let prepared =
            prepare_rename(&project, &package, &file, &sources, &selected, None).unwrap();
        assert_eq!(prepared["placeholder"], "Count");
        let locations = references(&project, &package, &file, &sources, &selected, true);
        assert_eq!(locations.as_array().unwrap().len(), 2, "{locations}");
    }

    #[test]
    fn preserves_export_visibility_for_cross_package_references() {
        let shared_text = "package shared\nfunc Shared() {}\n";
        let main_text = "package main\nimport \"shared\"\nfunc main() { shared.Shared() }\n";
        let mut sources = SourceDb::default();
        let root = std::env::current_dir().unwrap();
        let shared_id = sources.add_file(root.join("shared.ond"), shared_text.to_string());
        let main_id = sources.add_file(root.join("main.ond"), main_text.to_string());
        let shared_file = compiler::parse_source_file(shared_id, shared_text).unwrap();
        let main_file = compiler::parse_source_file(main_id, main_text).unwrap();
        let shared_package = ast::Package {
            logical_path: "shared".to_string(),
            files: vec![shared_file.clone()],
        };
        let main_package = ast::Package {
            logical_path: ".".to_string(),
            files: vec![main_file],
        };
        let project = ast::Project {
            packages: vec![main_package, shared_package.clone()],
        };
        let selected = position(shared_text, "Shared", 0);

        assert!(
            rename(
                &project,
                &shared_package,
                &shared_file,
                &sources,
                &selected,
                None,
                "hidden",
            )
            .is_err()
        );
        let edit = rename(
            &project,
            &shared_package,
            &shared_file,
            &sources,
            &selected,
            None,
            "Updated",
        )
        .unwrap();
        assert_eq!(
            edit["changes"]
                .as_object()
                .unwrap()
                .values()
                .map(|edits| edits.as_array().unwrap().len())
                .sum::<usize>(),
            2,
            "{edit}"
        );
    }
}
