use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use compiler::ir::ast;
use compiler::source::{SourceDb, Span};
use serde_json::{Value, json};

use crate::protocol::{Position, lsp_offset};
use crate::symbol_index::{IndexedSymbol, IndexedSymbolKind, ProjectSymbolIndex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportPathCompletion {
    prefix: String,
    replace_start: usize,
    replace_end: usize,
}

pub fn import_path_completion_context(
    text: &str,
    line: u32,
    character: u32,
) -> Option<ImportPathCompletion> {
    let offset = lsp_offset(text, &Position { line, character }).ok()?;
    let bytes = text.as_bytes();
    let mut index = 0;
    let mut pending_import = false;
    let mut import_string_start = None;

    while index < offset {
        match bytes[index] {
            b'\n' => {
                pending_import = false;
                index += 1;
            }
            byte if byte.is_ascii_whitespace() => index += 1,
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index += 2;
                while index < offset && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index += 2;
                while index < offset {
                    if bytes[index] == b'\n' {
                        pending_import = false;
                    }
                    if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                        index += 2;
                        break;
                    }
                    index += 1;
                }
            }
            b'"' => {
                let is_import = pending_import;
                let content_start = index + 1;
                index += 1;
                let mut closed = false;
                while index < offset {
                    match bytes[index] {
                        b'\\' => index = (index + 2).min(offset),
                        b'"' => {
                            index += 1;
                            closed = true;
                            break;
                        }
                        b'\n' => break,
                        _ => index += 1,
                    }
                }
                if is_import && !closed && index == offset {
                    import_string_start = Some(content_start);
                }
                pending_import = false;
            }
            b'`' => {
                pending_import = false;
                index += 1;
                while index < offset && bytes[index] != b'`' {
                    index += 1;
                }
                if index < offset {
                    index += 1;
                }
            }
            byte if byte.is_ascii_alphabetic() || byte == b'_' => {
                let start = index;
                index += 1;
                while index < offset
                    && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
                {
                    index += 1;
                }
                pending_import = &text[start..index] == "import";
            }
            _ => {
                pending_import = false;
                index += text[index..].chars().next()?.len_utf8();
            }
        }
    }

    let replace_start = import_string_start?;
    let mut replace_end = offset;
    while replace_end < bytes.len() {
        match bytes[replace_end] {
            b'\\' => replace_end = (replace_end + 2).min(bytes.len()),
            b'"' | b'\n' | b'\r' => break,
            _ => replace_end += text[replace_end..].chars().next()?.len_utf8(),
        }
    }
    Some(ImportPathCompletion {
        prefix: text[replace_start..offset].to_string(),
        replace_start,
        replace_end,
    })
}

pub fn import_path_completion_items<'a>(
    text: &str,
    context: &ImportPathCompletion,
    paths: impl IntoIterator<Item = &'a str>,
) -> Vec<Value> {
    let start = completion_position(text, context.replace_start);
    let end = completion_position(text, context.replace_end);
    paths
        .into_iter()
        .filter(|path| path.starts_with(&context.prefix))
        .map(|path| {
            json!({
                "label": path,
                "kind": 9,
                "detail": "package",
                "filterText": path,
                "textEdit": {
                    "range": {
                        "start": { "line": start.0, "character": start.1 },
                        "end": { "line": end.0, "character": end.1 }
                    },
                    "newText": path
                },
                "labelDetails": {
                    "description": "import path"
                }
            })
        })
        .collect()
}

fn completion_position(text: &str, offset: usize) -> (u32, u32) {
    let prefix = &text[..offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() as u32;
    let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
    let character = text[line_start..offset].encode_utf16().count() as u32;
    (line, character)
}

pub fn keyword_completion_items() -> Vec<Value> {
    [
        "package",
        "import",
        "const",
        "var",
        "type",
        "func",
        "struct",
        "interface",
        "return",
        "trap",
        "defer",
        "break",
        "continue",
        "if",
        "else",
        "for",
        "true",
        "false",
        "nil",
        "as",
    ]
    .into_iter()
    .map(|label| completion_item(label, 14, Some("keyword"), None))
    .collect()
}

pub fn append_builtin_type_completion_items(items: &mut Vec<Value>) {
    [
        ("u8", "8-bit unsigned integer"),
        ("u16", "16-bit unsigned integer"),
        ("u32", "32-bit unsigned integer"),
        ("i8", "8-bit signed integer"),
        ("i16", "16-bit signed integer"),
        ("i32", "32-bit signed integer"),
        ("f32", "32-bit floating-point number"),
        ("bool", "boolean"),
    ]
    .into_iter()
    .for_each(|(name, description)| {
        items.push(completion_item(
            name,
            7,
            Some("built-in type"),
            Some(description),
        ));
    });
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BuiltinInfo {
    pub(crate) name: &'static str,
    pub(crate) signature: &'static str,
    pub(crate) description: &'static str,
    pub(crate) syntax: bool,
}

pub(crate) const BUILTINS: &[BuiltinInfo] = &[
    BuiltinInfo {
        name: "alloc",
        signature: "alloc[T](count: u32) -> *T",
        description: "allocate heap memory",
        syntax: false,
    },
    BuiltinInfo {
        name: "free",
        signature: "free(pointer: *T)",
        description: "release heap memory",
        syntax: false,
    },
    BuiltinInfo {
        name: "len",
        signature: "len(array: [N]T) -> u32",
        description: "get a compile-time array length",
        syntax: false,
    },
    BuiltinInfo {
        name: "load32",
        signature: "load32(address: u32 | *T) -> u32",
        description: "read a 32-bit value from memory",
        syntax: false,
    },
    BuiltinInfo {
        name: "store32",
        signature: "store32(address: u32 | *T, value: u32)",
        description: "write a 32-bit value to memory",
        syntax: false,
    },
    BuiltinInfo {
        name: "thisFile",
        signature: "thisFile() -> *u8",
        description: "get the current logical source path",
        syntax: false,
    },
    BuiltinInfo {
        name: "thisLine",
        signature: "thisLine() -> u32",
        description: "get the current one-based source line",
        syntax: false,
    },
    BuiltinInfo {
        name: "new",
        signature: "new(T) -> *T",
        description: "allocate zero-initialized heap memory",
        syntax: true,
    },
];

pub(crate) fn builtin_info(name: &str) -> Option<BuiltinInfo> {
    BUILTINS
        .iter()
        .find(|builtin| builtin.name == name)
        .copied()
}

pub fn append_builtin_function_completion_items(items: &mut Vec<Value>) {
    BUILTINS.iter().for_each(|builtin| {
        let detail = if builtin.syntax {
            "built-in syntax"
        } else {
            "built-in function"
        };
        items.push(completion_function_item(
            builtin.name,
            builtin.signature,
            &format!("{detail}: {}", builtin.description),
        ));
    });
}

const COMPLETION_FIELD_MARKER: &str = "__ond_completion_field";
const COMPLETION_VALUE_MARKER: &str = "__ond_completion_value";

#[derive(Clone, Copy)]
pub(crate) enum CompletionScope<'a> {
    Function(&'a ast::FuncDecl),
    Operator(&'a ast::OperatorDecl),
}

impl<'a> CompletionScope<'a> {
    pub(crate) fn type_params(self) -> &'a [ast::Ident] {
        match self {
            Self::Function(function) => &function.type_params,
            Self::Operator(operator) => &operator.type_params,
        }
    }

    pub(crate) fn params(self) -> &'a [ast::Field] {
        match self {
            Self::Function(function) => &function.signature.params,
            Self::Operator(operator) => &operator.signature.params,
        }
    }

    pub(crate) fn body(self) -> &'a ast::Block {
        match self {
            Self::Function(function) => &function.body,
            Self::Operator(operator) => &operator.body,
        }
    }

    pub(crate) fn receiver(self) -> Option<&'a ast::Field> {
        match self {
            Self::Function(function) => function.receiver.as_deref(),
            Self::Operator(_) => None,
        }
    }
}

pub fn completion_text_with_placeholder(
    text: &str,
    line: u32,
    character: u32,
) -> Option<(String, &'static str)> {
    let mut sources = SourceDb::default();
    let file_id = sources.add_file(PathBuf::new(), text.to_string());
    let offset = sources
        .file(file_id)
        .offset_at_line_col(line as usize + 1, character as usize + 1);
    if offset > text.len() || !text.is_char_boundary(offset) {
        return None;
    }

    let bytes = text.as_bytes();
    let mut member_start = offset;
    while member_start > 0 && is_identifier_byte(bytes[member_start - 1]) {
        member_start -= 1;
    }

    let mut member_end = offset;
    while member_end < bytes.len() && is_identifier_byte(bytes[member_end]) {
        member_end += 1;
    }

    let mut dot = member_start;
    while dot > 0 && bytes[dot - 1].is_ascii_whitespace() {
        dot -= 1;
    }
    if dot == 0 || bytes[dot - 1] != b'.' {
        return None;
    }

    let mut completed = text.to_string();
    completed.replace_range(member_start..member_end, COMPLETION_FIELD_MARKER);
    let marker_end = member_start + COMPLETION_FIELD_MARKER.len();
    let line_end = completed[marker_end..]
        .find(['\r', '\n'])
        .map(|relative| marker_end + relative)
        .unwrap_or(completed.len());
    let suffix = completed[marker_end..line_end].trim();
    if suffix.is_empty() || suffix.starts_with('}') {
        completed.insert_str(marker_end, "()");
    } else if incomplete_assignment_operator(suffix) {
        completed.insert_str(line_end, " 0");
    }
    Some((completed, COMPLETION_FIELD_MARKER))
}

pub fn completion_text_with_value_placeholder(
    text: &str,
    line: u32,
    character: u32,
) -> Option<String> {
    let offset = lsp_offset(text, &Position { line, character }).ok()?;
    if offset > text.len() || !text.is_char_boundary(offset) {
        return None;
    }
    let previous = text[..offset]
        .chars()
        .rev()
        .find(|ch| !ch.is_whitespace())?;
    if !matches!(
        previous,
        '=' | '+'
            | '-'
            | '*'
            | '/'
            | '%'
            | '&'
            | '|'
            | '^'
            | '!'
            | '<'
            | '>'
            | ','
            | '('
            | '['
            | ':'
    ) {
        return None;
    }
    let mut completed = text.to_string();
    completed.insert_str(offset, COMPLETION_VALUE_MARKER);
    Some(completed)
}

fn incomplete_assignment_operator(suffix: &str) -> bool {
    matches!(
        suffix,
        "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "&^=" | "<<=" | ">>="
    )
}

pub fn completion_qualifier(text: &str, line: u32, character: u32) -> Option<String> {
    let offset = lsp_offset(text, &Position { line, character }).ok()?;
    let bytes = text.as_bytes();
    let mut member_start = offset;
    while member_start > 0 && is_identifier_byte(bytes[member_start - 1]) {
        member_start -= 1;
    }
    while member_start > 0 && bytes[member_start - 1].is_ascii_whitespace() {
        member_start -= 1;
    }
    if member_start == 0 || bytes[member_start - 1] != b'.' {
        return None;
    }
    let mut base_end = member_start - 1;
    while base_end > 0 && bytes[base_end - 1].is_ascii_whitespace() {
        base_end -= 1;
    }
    let mut base_start = base_end;
    while base_start > 0 && is_identifier_byte(bytes[base_start - 1]) {
        base_start -= 1;
    }
    (base_start < base_end).then(|| text[base_start..base_end].to_string())
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

pub fn find_completion_selector<'a>(
    file: &'a ast::File,
    marker: &str,
) -> Option<(ast::Expr, Option<CompletionScope<'a>>)> {
    for decl in &file.decls {
        let result = match decl {
            ast::TopLevelDecl::Const(decl) => decl
                .specs
                .iter()
                .flat_map(|spec| &spec.values)
                .find_map(|expr| find_completion_selector_in_expr(expr, marker, None)),
            ast::TopLevelDecl::Var(decl) => decl
                .specs
                .iter()
                .flat_map(|spec| &spec.values)
                .find_map(|expr| find_completion_selector_in_expr(expr, marker, None)),
            ast::TopLevelDecl::Type(_) => None,
            ast::TopLevelDecl::Func(decl) => find_completion_selector_in_block(
                &decl.body,
                marker,
                Some(CompletionScope::Function(decl)),
            ),
            ast::TopLevelDecl::Operator(decl) => find_completion_selector_in_block(
                &decl.body,
                marker,
                Some(CompletionScope::Operator(decl)),
            ),
        };
        if result.is_some() {
            return result;
        }
    }
    None
}

fn find_completion_selector_in_block<'a>(
    block: &'a ast::Block,
    marker: &str,
    function: Option<CompletionScope<'a>>,
) -> Option<(ast::Expr, Option<CompletionScope<'a>>)> {
    block
        .statements
        .iter()
        .find_map(|stmt| find_completion_selector_in_stmt(stmt, marker, function))
}

fn find_completion_selector_in_stmt<'a>(
    stmt: &'a ast::Stmt,
    marker: &str,
    function: Option<CompletionScope<'a>>,
) -> Option<(ast::Expr, Option<CompletionScope<'a>>)> {
    match stmt {
        ast::Stmt::Const(decl) => decl
            .specs
            .iter()
            .flat_map(|spec| &spec.values)
            .find_map(|expr| find_completion_selector_in_expr(expr, marker, function)),
        ast::Stmt::Var(decl) => decl
            .specs
            .iter()
            .flat_map(|spec| &spec.values)
            .find_map(|expr| find_completion_selector_in_expr(expr, marker, function)),
        ast::Stmt::ShortVar(decl) => decl
            .values
            .iter()
            .find_map(|expr| find_completion_selector_in_expr(expr, marker, function)),
        ast::Stmt::Assign(stmt) => stmt
            .targets
            .iter()
            .chain(&stmt.values)
            .find_map(|expr| find_completion_selector_in_expr(expr, marker, function)),
        ast::Stmt::Expr(stmt) => find_completion_selector_in_expr(&stmt.expr, marker, function),
        ast::Stmt::Return(stmt) => stmt
            .values
            .iter()
            .find_map(|expr| find_completion_selector_in_expr(expr, marker, function)),
        ast::Stmt::Defer(stmt) => find_completion_selector_in_block(&stmt.block, marker, function),
        ast::Stmt::Break(_) | ast::Stmt::Continue(_) | ast::Stmt::Trap { .. } => None,
        ast::Stmt::Block(block) => find_completion_selector_in_block(block, marker, function),
        ast::Stmt::If(stmt) => stmt
            .init
            .as_deref()
            .and_then(|stmt| find_completion_selector_in_stmt(stmt, marker, function))
            .or_else(|| find_completion_selector_in_expr(&stmt.condition, marker, function))
            .or_else(|| find_completion_selector_in_block(&stmt.then_block, marker, function))
            .or_else(|| {
                stmt.else_branch
                    .as_deref()
                    .and_then(|stmt| find_completion_selector_in_stmt(stmt, marker, function))
            }),
        ast::Stmt::For(stmt) => {
            let in_kind = match &stmt.kind {
                ast::ForKind::Infinite => None,
                ast::ForKind::While(condition) => {
                    find_completion_selector_in_expr(condition, marker, function)
                }
                ast::ForKind::ThreeClause {
                    init,
                    condition,
                    post,
                } => init
                    .as_deref()
                    .and_then(|stmt| find_completion_selector_in_stmt(stmt, marker, function))
                    .or_else(|| {
                        condition.as_ref().and_then(|condition| {
                            find_completion_selector_in_expr(condition, marker, function)
                        })
                    })
                    .or_else(|| {
                        post.as_deref().and_then(|stmt| {
                            find_completion_selector_in_stmt(stmt, marker, function)
                        })
                    }),
            };
            in_kind.or_else(|| find_completion_selector_in_block(&stmt.body, marker, function))
        }
    }
}

fn find_completion_selector_in_expr<'a>(
    expr: &'a ast::Expr,
    marker: &str,
    function: Option<CompletionScope<'a>>,
) -> Option<(ast::Expr, Option<CompletionScope<'a>>)> {
    match expr {
        ast::Expr::Selector { base, field, .. } => {
            if field.name == marker {
                Some((base.as_ref().clone(), function))
            } else {
                find_completion_selector_in_expr(base, marker, function)
            }
        }
        ast::Expr::Name(path)
            if path.segments.len() >= 2
                && path
                    .segments
                    .last()
                    .map(|field| field.name == marker)
                    .unwrap_or(false) =>
        {
            let segments = path.segments[..path.segments.len() - 1].to_vec();
            let span = Span::new(
                path.span.file,
                segments.first()?.span.start,
                segments.last()?.span.end,
            );
            Some((ast::Expr::Name(ast::Path { segments, span }), function))
        }
        ast::Expr::Unary { expr, .. } => find_completion_selector_in_expr(expr, marker, function),
        ast::Expr::Binary { lhs, rhs, .. } => {
            find_completion_selector_in_expr(lhs, marker, function)
                .or_else(|| find_completion_selector_in_expr(rhs, marker, function))
        }
        ast::Expr::Call { callee, args, .. } => {
            find_completion_selector_in_expr(callee, marker, function).or_else(|| {
                args.iter()
                    .find_map(|arg| find_completion_selector_in_expr(arg, marker, function))
            })
        }
        ast::Expr::TypeApply { base, .. } => {
            find_completion_selector_in_expr(base, marker, function)
        }
        ast::Expr::Index { base, index, .. } => {
            find_completion_selector_in_expr(base, marker, function)
                .or_else(|| find_completion_selector_in_expr(index, marker, function))
        }
        ast::Expr::Cast { expr, .. } => find_completion_selector_in_expr(expr, marker, function),
        ast::Expr::Composite { elements, .. } => elements.iter().find_map(|element| {
            element
                .key
                .as_ref()
                .and_then(|key| find_completion_selector_in_expr(key, marker, function))
                .or_else(|| find_completion_selector_in_expr(&element.value, marker, function))
        }),
        ast::Expr::Literal(_)
        | ast::Expr::Name(_)
        | ast::Expr::New { .. }
        | ast::Expr::LayoutQuery { .. } => None,
    }
}

pub fn resolve_completion_struct_fields(
    package: &ast::Package,
    function: Option<CompletionScope<'_>>,
    base: &ast::Expr,
) -> Option<Vec<ast::Field>> {
    let ty = resolve_completion_expr_type(package, function, base)?;
    resolve_completion_struct_fields_for_type(package, &ty, 0)
}

pub fn resolve_project_completion_struct_fields(
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
    function: Option<CompletionScope<'_>>,
    base: &ast::Expr,
) -> Option<Vec<ast::Field>> {
    let ty = resolve_project_completion_expr_type(project, package, file, function, base)?;
    let (fields, _, _, external) = resolve_project_completion_struct_type(project, ty, 0)?;
    Some(
        fields
            .into_iter()
            .filter(|field| !external || completion_name_is_exported(&field.name.name))
            .collect(),
    )
}

pub fn resolve_project_completion_methods(
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
    function: Option<CompletionScope<'_>>,
    base: &ast::Expr,
) -> Option<Vec<Value>> {
    let resolved = resolve_project_completion_expr_type(project, package, file, function, base)?;
    let interface = resolve_project_completion_interface_type(project, resolved.clone(), 0);
    let (path, pointer) = completion_receiver_path(&resolved.ty, false)?;
    let (method_package, name, external) = match path.segments.as_slice() {
        [name] => (resolved.package, name.name.as_str(), resolved.external),
        [alias, name] => (
            completion_imported_package(project, resolved.file, &alias.name)?,
            name.name.as_str(),
            true,
        ),
        _ => return None,
    };
    let mut methods = Vec::new();
    if !pointer && let Some((required, interface_external)) = interface {
        methods.extend(
            required
                .iter()
                .filter(|method| {
                    !interface_external || completion_name_is_exported(&method.name.name)
                })
                .map(interface_method_completion_item),
        );
    }
    for candidate_file in &method_package.files {
        for decl in &candidate_file.decls {
            let ast::TopLevelDecl::Func(method) = decl else {
                continue;
            };
            let Some(receiver) = &method.receiver else {
                continue;
            };
            let Some((receiver_name, receiver_pointer)) =
                completion_exact_named_receiver(&receiver.ty)
            else {
                continue;
            };
            if receiver_name == name
                && receiver_pointer == pointer
                && (!external || completion_name_is_exported(&method.name.name))
            {
                methods.push(method_completion_item(method));
            }
        }
    }
    Some(methods)
}

pub fn resolve_completion_methods(
    package: &ast::Package,
    function: Option<CompletionScope<'_>>,
    base: &ast::Expr,
) -> Option<Vec<Value>> {
    let ty = resolve_completion_expr_type(package, function, base)?;
    let (name, pointer) = completion_exact_named_receiver(&ty)?;
    let mut methods = Vec::new();
    if !pointer && let Some(required) = resolve_completion_interface_type(package, &ty, 0) {
        methods.extend(required.iter().map(interface_method_completion_item));
    }
    for file in &package.files {
        for decl in &file.decls {
            let ast::TopLevelDecl::Func(method) = decl else {
                continue;
            };
            let Some(receiver) = &method.receiver else {
                continue;
            };
            if completion_exact_named_receiver(&receiver.ty) == Some((name, pointer)) {
                methods.push(method_completion_item(method));
            }
        }
    }
    Some(methods)
}

fn completion_exact_named_receiver(ty: &ast::Type) -> Option<(&str, bool)> {
    let (path, pointer) = completion_receiver_path(ty, false)?;
    (path.segments.len() == 1).then(|| (path.segments[0].name.as_str(), pointer))
}

fn completion_receiver_path(ty: &ast::Type, pointer: bool) -> Option<(&ast::Path, bool)> {
    match ty {
        ast::Type::Named(path) => Some((path, pointer)),
        ast::Type::Apply { base, .. } => Some((base, pointer)),
        ast::Type::Pointer(inner, _) => completion_receiver_path(inner, true),
        _ => None,
    }
}

fn method_completion_item(method: &ast::FuncDecl) -> Value {
    let detail = completion_function_signature(method);
    completion_item(&method.name.name, 2, Some(&detail), Some("method"))
}

fn interface_method_completion_item(method: &ast::InterfaceMethod) -> Value {
    let parameters = method
        .signature
        .params
        .iter()
        .map(|parameter| completion_type_name(&parameter.ty))
        .collect::<Vec<_>>()
        .join(", ");
    let returns = match method.signature.results.as_slice() {
        [] => String::new(),
        [result] => format!(" -> {}", completion_type_name(result)),
        results => format!(
            " -> ({})",
            results
                .iter()
                .map(completion_type_name)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };
    let detail = format!("{}({parameters}){returns}", method.name.name);
    completion_item(
        &method.name.name,
        2,
        Some(&detail),
        Some("interface method"),
    )
}

fn resolve_completion_interface_type(
    package: &ast::Package,
    ty: &ast::Type,
    depth: usize,
) -> Option<Vec<ast::InterfaceMethod>> {
    if depth > 16 {
        return None;
    }
    match ty {
        ast::Type::Interface { methods, .. } => Some(methods.clone()),
        ast::Type::Named(path) if path.segments.len() == 1 => {
            let (ty, _) = completion_type_declaration(package, &path.segments[0].name)?;
            resolve_completion_interface_type(package, ty, depth + 1)
        }
        ast::Type::Apply {
            base, arguments, ..
        } if base.segments.len() == 1 => {
            let (spec, _) = completion_type_spec(package, &base.segments[0].name)?;
            let ty = substitute_completion_type_arguments(spec, arguments)?;
            resolve_completion_interface_type(package, &ty, depth + 1)
        }
        _ => None,
    }
}

pub fn project_completion_base_has_known_type(
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
    function: Option<CompletionScope<'_>>,
    base: &ast::Expr,
) -> bool {
    resolve_project_completion_expr_type(project, package, file, function, base).is_some()
}

#[derive(Clone)]
struct ProjectCompletionType<'a> {
    ty: ast::Type,
    package: &'a ast::Package,
    file: &'a ast::File,
    external: bool,
}

fn resolve_project_completion_interface_type(
    project: &ast::Project,
    resolved: ProjectCompletionType<'_>,
    depth: usize,
) -> Option<(Vec<ast::InterfaceMethod>, bool)> {
    if depth > 16 {
        return None;
    }
    match resolved.ty {
        ast::Type::Interface { methods, .. } => Some((methods, resolved.external)),
        ast::Type::Named(path) => match path.segments.as_slice() {
            [name] => {
                let (ty, declaring_file) =
                    completion_type_declaration(resolved.package, &name.name)?;
                resolve_project_completion_interface_type(
                    project,
                    ProjectCompletionType {
                        ty: ty.clone(),
                        file: declaring_file,
                        ..resolved
                    },
                    depth + 1,
                )
            }
            [alias, name] => {
                let imported = completion_imported_package(project, resolved.file, &alias.name)?;
                let (ty, declaring_file) = completion_type_declaration(imported, &name.name)?;
                resolve_project_completion_interface_type(
                    project,
                    ProjectCompletionType {
                        ty: ty.clone(),
                        package: imported,
                        file: declaring_file,
                        external: true,
                    },
                    depth + 1,
                )
            }
            _ => None,
        },
        ast::Type::Apply {
            base, arguments, ..
        } => match base.segments.as_slice() {
            [name] => {
                let (spec, declaring_file) = completion_type_spec(resolved.package, &name.name)?;
                let ty = substitute_completion_type_arguments(spec, &arguments)?;
                resolve_project_completion_interface_type(
                    project,
                    ProjectCompletionType {
                        ty,
                        file: declaring_file,
                        ..resolved
                    },
                    depth + 1,
                )
            }
            [alias, name] => {
                let imported = completion_imported_package(project, resolved.file, &alias.name)?;
                let (spec, declaring_file) = completion_type_spec(imported, &name.name)?;
                let ty = substitute_completion_type_arguments(spec, &arguments)?;
                resolve_project_completion_interface_type(
                    project,
                    ProjectCompletionType {
                        ty,
                        package: imported,
                        file: declaring_file,
                        external: true,
                    },
                    depth + 1,
                )
            }
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn project_value_type_for_name(
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
    name: &str,
    offset: usize,
) -> Option<ast::Type> {
    let function = file.decls.iter().find_map(|decl| match decl {
        ast::TopLevelDecl::Func(function)
            if function.span.start <= offset && offset <= function.span.end =>
        {
            Some(CompletionScope::Function(function))
        }
        ast::TopLevelDecl::Operator(operator)
            if operator.span.start <= offset && offset <= operator.span.end =>
        {
            Some(CompletionScope::Operator(operator))
        }
        _ => None,
    });
    let resolved =
        project_completion_type_for_name(project, package, file, function, name, offset)?;
    if !resolved.external {
        return Some(resolved.ty);
    }
    let alias = file.imports.iter().find_map(|import| {
        let logical_path = compiler::decode_import_path(&import.path).ok()?;
        if logical_path != resolved.package.logical_path {
            return None;
        }
        import
            .alias
            .as_ref()
            .map(|alias| alias.name.clone())
            .or_else(|| logical_path.rsplit('/').next().map(str::to_string))
    })?;
    Some(qualify_project_value_type(
        resolved.ty,
        &alias,
        resolved.package,
    ))
}

fn qualify_project_value_type(
    ty: ast::Type,
    alias: &str,
    declaring_package: &ast::Package,
) -> ast::Type {
    match ty {
        ast::Type::Named(path)
            if path.segments.len() == 1
                && completion_type_spec(declaring_package, &path.segments[0].name).is_some() =>
        {
            let span = path.span;
            ast::Type::Named(ast::Path {
                segments: vec![
                    ast::Ident {
                        name: alias.to_string(),
                        span,
                    },
                    path.segments[0].clone(),
                ],
                span,
            })
        }
        ast::Type::Apply {
            mut base,
            arguments,
            span,
        } if base.segments.len() == 1 => {
            base.segments.insert(
                0,
                ast::Ident {
                    name: alias.to_string(),
                    span: base.span,
                },
            );
            ast::Type::Apply {
                base,
                arguments,
                span,
            }
        }
        ast::Type::Pointer(inner, span) => ast::Type::Pointer(
            Box::new(qualify_project_value_type(*inner, alias, declaring_package)),
            span,
        ),
        ast::Type::Array { len, element, span } => ast::Type::Array {
            len,
            element: Box::new(qualify_project_value_type(
                *element,
                alias,
                declaring_package,
            )),
            span,
        },
        other => other,
    }
}

fn resolve_project_completion_expr_type<'a>(
    project: &'a ast::Project,
    package: &'a ast::Package,
    file: &'a ast::File,
    function: Option<CompletionScope<'_>>,
    expr: &ast::Expr,
) -> Option<ProjectCompletionType<'a>> {
    match expr {
        ast::Expr::Name(path) if !path.segments.is_empty() => {
            let mut resolved = project_completion_type_for_name(
                project,
                package,
                file,
                function,
                &path.segments[0].name,
                expr.span().start,
            )?;
            for segment in &path.segments[1..] {
                let (fields, field_package, field_file, external) =
                    resolve_project_completion_struct_type(project, resolved, 0)?;
                let field = fields
                    .into_iter()
                    .find(|candidate| candidate.name.name == segment.name)?;
                resolved = ProjectCompletionType {
                    ty: field.ty,
                    package: field_package,
                    file: field_file,
                    external,
                };
            }
            Some(resolved)
        }
        ast::Expr::Selector { base, field, .. } => {
            let base =
                resolve_project_completion_expr_type(project, package, file, function, base)?;
            let (fields, field_package, field_file, external) =
                resolve_project_completion_struct_type(project, base, 0)?;
            let field = fields
                .into_iter()
                .find(|candidate| candidate.name.name == field.name)?;
            Some(ProjectCompletionType {
                ty: field.ty,
                package: field_package,
                file: field_file,
                external,
            })
        }
        ast::Expr::Cast { ty, .. } | ast::Expr::Composite { ty, .. } => {
            Some(ProjectCompletionType {
                ty: ty.clone(),
                package,
                file,
                external: false,
            })
        }
        ast::Expr::New { ty, span } => Some(ProjectCompletionType {
            ty: ast::Type::Pointer(Box::new(ty.clone()), *span),
            package,
            file,
            external: false,
        }),
        ast::Expr::Call { callee, .. } => project_completion_call_result(
            project,
            package,
            file,
            function,
            callee,
            expr.span().start,
        ),
        ast::Expr::Unary {
            op: ast::UnaryOp::Deref,
            expr,
            ..
        } => {
            let mut resolved =
                resolve_project_completion_expr_type(project, package, file, function, expr)?;
            let ast::Type::Pointer(inner, _) = resolved.ty else {
                return None;
            };
            resolved.ty = *inner;
            Some(resolved)
        }
        ast::Expr::Index { base, .. } => {
            let mut resolved =
                resolve_project_completion_expr_type(project, package, file, function, base)?;
            resolved.ty = match resolved.ty {
                ast::Type::Array { element, .. } | ast::Type::Pointer(element, _) => *element,
                _ => return None,
            };
            Some(resolved)
        }
        _ => None,
    }
}

fn project_completion_type_for_name<'a>(
    project: &'a ast::Project,
    package: &'a ast::Package,
    file: &'a ast::File,
    function: Option<CompletionScope<'_>>,
    name: &str,
    limit: usize,
) -> Option<ProjectCompletionType<'a>> {
    if let Some(function) = function {
        let mut locals = BTreeMap::new();
        if let Some(receiver) = function.receiver() {
            locals.insert(
                receiver.name.name.clone(),
                ProjectCompletionType {
                    ty: receiver.ty.clone(),
                    package,
                    file,
                    external: false,
                },
            );
        }
        for param in function.params() {
            locals.insert(
                param.name.name.clone(),
                ProjectCompletionType {
                    ty: param.ty.clone(),
                    package,
                    file,
                    external: false,
                },
            );
        }
        collect_project_completion_local_types(
            project,
            package,
            file,
            &function.body().statements,
            limit,
            &mut locals,
        );
        if let Some(ty) = locals.get(name) {
            return Some(ty.clone());
        }
    }

    package.files.iter().find_map(|candidate_file| {
        candidate_file.decls.iter().find_map(|decl| {
            let ast::TopLevelDecl::Var(decl) = decl else {
                return None;
            };
            decl.specs.iter().find_map(|spec| {
                spec.names
                    .iter()
                    .position(|candidate| candidate.name == name)
                    .and_then(|index| {
                        spec.ty
                            .as_ref()
                            .map(|ty| ProjectCompletionType {
                                ty: ty.clone(),
                                package,
                                file: candidate_file,
                                external: false,
                            })
                            .or_else(|| {
                                spec.values.get(index).and_then(|value| {
                                    project_completion_type_from_known_expr(
                                        project,
                                        package,
                                        candidate_file,
                                        value,
                                        &BTreeMap::new(),
                                    )
                                })
                            })
                    })
            })
        })
    })
}

fn collect_project_completion_local_types<'a>(
    project: &'a ast::Project,
    package: &'a ast::Package,
    file: &'a ast::File,
    statements: &[ast::Stmt],
    limit: usize,
    types: &mut BTreeMap<String, ProjectCompletionType<'a>>,
) {
    for stmt in statements {
        if stmt_span(stmt).start > limit {
            break;
        }
        match stmt {
            ast::Stmt::Var(decl) => {
                for spec in &decl.specs {
                    if spec.span.start > limit {
                        continue;
                    }
                    for (index, name) in spec.names.iter().enumerate() {
                        let ty = spec
                            .ty
                            .as_ref()
                            .map(|ty| ProjectCompletionType {
                                ty: ty.clone(),
                                package,
                                file,
                                external: false,
                            })
                            .or_else(|| {
                                spec.values.get(index).and_then(|value| {
                                    project_completion_type_from_known_expr(
                                        project, package, file, value, types,
                                    )
                                })
                            });
                        if let Some(ty) = ty {
                            types.insert(name.name.clone(), ty);
                        }
                    }
                }
            }
            ast::Stmt::Const(decl) => {
                for spec in &decl.specs {
                    if spec.span.start > limit {
                        continue;
                    }
                    for (index, name) in spec.names.iter().enumerate() {
                        let ty = spec
                            .ty
                            .as_ref()
                            .map(|ty| ProjectCompletionType {
                                ty: ty.clone(),
                                package,
                                file,
                                external: false,
                            })
                            .or_else(|| {
                                spec.values.get(index).and_then(|value| {
                                    project_completion_type_from_known_expr(
                                        project, package, file, value, types,
                                    )
                                })
                            });
                        if let Some(ty) = ty {
                            types.insert(name.name.clone(), ty);
                        }
                    }
                }
            }
            ast::Stmt::ShortVar(decl) if decl.span.start <= limit => {
                for (name, value) in decl.names.iter().zip(&decl.values) {
                    if let Some(ty) = project_completion_type_from_known_expr(
                        project, package, file, value, types,
                    ) {
                        types.insert(name.name.clone(), ty);
                    }
                }
            }
            ast::Stmt::Block(block) => collect_project_completion_local_types(
                project,
                package,
                file,
                &block.statements,
                limit,
                types,
            ),
            ast::Stmt::Defer(stmt) => collect_project_completion_local_types(
                project,
                package,
                file,
                &stmt.block.statements,
                limit,
                types,
            ),
            ast::Stmt::If(stmt) => {
                if let Some(init) = &stmt.init {
                    collect_project_completion_local_types(
                        project,
                        package,
                        file,
                        std::slice::from_ref(init),
                        limit,
                        types,
                    );
                }
                collect_project_completion_local_types(
                    project,
                    package,
                    file,
                    &stmt.then_block.statements,
                    limit,
                    types,
                );
                if let Some(other) = &stmt.else_branch {
                    collect_project_completion_local_types(
                        project,
                        package,
                        file,
                        std::slice::from_ref(other),
                        limit,
                        types,
                    );
                }
            }
            ast::Stmt::For(stmt) => {
                if let ast::ForKind::ThreeClause {
                    init: Some(init), ..
                } = &stmt.kind
                {
                    collect_project_completion_local_types(
                        project,
                        package,
                        file,
                        std::slice::from_ref(init),
                        limit,
                        types,
                    );
                }
                collect_project_completion_local_types(
                    project,
                    package,
                    file,
                    &stmt.body.statements,
                    limit,
                    types,
                );
            }
            _ => {}
        }
    }
}

fn project_completion_type_from_known_expr<'a>(
    project: &'a ast::Project,
    package: &'a ast::Package,
    file: &'a ast::File,
    expr: &ast::Expr,
    types: &BTreeMap<String, ProjectCompletionType<'a>>,
) -> Option<ProjectCompletionType<'a>> {
    match expr {
        ast::Expr::Literal(ast::Literal::Int(_, span)) => Some(ProjectCompletionType {
            ty: primitive_type("i32", *span),
            package,
            file,
            external: false,
        }),
        ast::Expr::Literal(ast::Literal::Float(_, span)) => Some(ProjectCompletionType {
            ty: primitive_type("f32", *span),
            package,
            file,
            external: false,
        }),
        ast::Expr::Literal(ast::Literal::Bool(_, span)) => Some(ProjectCompletionType {
            ty: primitive_type("bool", *span),
            package,
            file,
            external: false,
        }),
        ast::Expr::Cast { ty, .. } | ast::Expr::Composite { ty, .. } => {
            Some(ProjectCompletionType {
                ty: ty.clone(),
                package,
                file,
                external: false,
            })
        }
        ast::Expr::New { ty, span } => Some(ProjectCompletionType {
            ty: ast::Type::Pointer(Box::new(ty.clone()), *span),
            package,
            file,
            external: false,
        }),
        ast::Expr::Name(path) if path.segments.len() == 1 => {
            types.get(&path.segments[0].name).cloned()
        }
        ast::Expr::Call { callee, .. } => {
            project_completion_call_result(project, package, file, None, callee, expr.span().start)
        }
        _ => None,
    }
}

fn primitive_type(name: &str, span: Span) -> ast::Type {
    ast::Type::Named(ast::Path {
        segments: vec![ast::Ident {
            name: name.to_string(),
            span,
        }],
        span,
    })
}

fn project_completion_call_result<'a>(
    project: &'a ast::Project,
    package: &'a ast::Package,
    file: &'a ast::File,
    function: Option<CompletionScope<'_>>,
    callee: &ast::Expr,
    limit: usize,
) -> Option<ProjectCompletionType<'a>> {
    if let ast::Expr::Selector { base, field, .. } = callee {
        return project_completion_method_result(
            project, package, file, function, base, field, limit,
        );
    }
    if let ast::Expr::Name(path) = callee
        && let [base, method] = path.segments.as_slice()
        && project_completion_type_for_name(project, package, file, function, &base.name, limit)
            .is_some()
    {
        let base = ast::Expr::Name(ast::Path {
            segments: vec![base.clone()],
            span: base.span,
        });
        return project_completion_method_result(
            project, package, file, function, &base, method, limit,
        );
    }
    let (path, type_arguments) = match callee {
        ast::Expr::Name(path) => (path, None),
        ast::Expr::TypeApply {
            base, arguments, ..
        } => {
            let ast::Expr::Name(path) = base.as_ref() else {
                return None;
            };
            (path, Some(arguments.as_slice()))
        }
        _ => return None,
    };
    match path.segments.as_slice() {
        [name] => package.files.iter().find_map(|candidate_file| {
            candidate_file.decls.iter().find_map(|decl| {
                let ast::TopLevelDecl::Func(decl) = decl else {
                    return None;
                };
                if decl.receiver.is_some() || decl.name.name != name.name {
                    return None;
                }
                Some(ProjectCompletionType {
                    ty: completion_function_result_type(decl, type_arguments)?,
                    package,
                    file: candidate_file,
                    external: false,
                })
            })
        }),
        [alias, name] if completion_name_is_exported(&name.name) => {
            let imported = completion_imported_package(project, file, &alias.name)?;
            imported.files.iter().find_map(|candidate_file| {
                candidate_file.decls.iter().find_map(|decl| {
                    let ast::TopLevelDecl::Func(decl) = decl else {
                        return None;
                    };
                    if decl.receiver.is_some() || decl.name.name != name.name {
                        return None;
                    }
                    Some(ProjectCompletionType {
                        ty: completion_function_result_type(decl, type_arguments)?,
                        package: imported,
                        file: candidate_file,
                        external: true,
                    })
                })
            })
        }
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn project_completion_method_result<'a>(
    project: &'a ast::Project,
    package: &'a ast::Package,
    file: &'a ast::File,
    function: Option<CompletionScope<'_>>,
    base: &ast::Expr,
    method_name: &ast::Ident,
    _limit: usize,
) -> Option<ProjectCompletionType<'a>> {
    let receiver = resolve_project_completion_expr_type(project, package, file, function, base)?;
    let (path, pointer, arguments) = completion_receiver_application(&receiver.ty, false)?;
    let (method_package, receiver_name, alias) = match path.segments.as_slice() {
        [name] => (receiver.package, name.name.as_str(), None),
        [alias, name] => (
            completion_imported_package(project, receiver.file, &alias.name)?,
            name.name.as_str(),
            Some(alias.name.as_str()),
        ),
        _ => return None,
    };
    let (method, declaring_file) = method_package.files.iter().find_map(|candidate_file| {
        candidate_file.decls.iter().find_map(|decl| {
            let ast::TopLevelDecl::Func(method) = decl else {
                return None;
            };
            let receiver = method.receiver.as_ref()?;
            (completion_exact_named_receiver(&receiver.ty) == Some((receiver_name, pointer))
                && method.name.name == method_name.name)
                .then_some((method, candidate_file))
        })
    })?;
    let mut result = method.signature.results.first()?.clone();
    if let Some(alias) = alias {
        result = qualify_project_value_type(result, alias, method_package);
    }
    result =
        substitute_completion_method_receiver(result, &method.receiver.as_ref()?.ty, arguments)?;
    if alias.is_some() {
        Some(ProjectCompletionType {
            ty: result,
            package: receiver.package,
            file: receiver.file,
            external: receiver.external,
        })
    } else {
        Some(ProjectCompletionType {
            ty: result,
            package: method_package,
            file: declaring_file,
            external: receiver.external,
        })
    }
}

fn completion_receiver_application(
    ty: &ast::Type,
    pointer: bool,
) -> Option<(&ast::Path, bool, &[ast::Type])> {
    match ty {
        ast::Type::Named(path) => Some((path, pointer, &[])),
        ast::Type::Apply {
            base, arguments, ..
        } => Some((base, pointer, arguments)),
        ast::Type::Pointer(inner, _) => completion_receiver_application(inner, true),
        _ => None,
    }
}

fn substitute_completion_method_receiver(
    result: ast::Type,
    declared_receiver: &ast::Type,
    arguments: &[ast::Type],
) -> Option<ast::Type> {
    let (_, _, parameters) = completion_receiver_application(declared_receiver, false)?;
    if parameters.len() != arguments.len() {
        return None;
    }
    let substitutions = parameters
        .iter()
        .zip(arguments)
        .map(|(parameter, argument)| {
            let ast::Type::Named(path) = parameter else {
                return None;
            };
            let [name] = path.segments.as_slice() else {
                return None;
            };
            Some((name.name.clone(), argument.clone()))
        })
        .collect::<Option<BTreeMap<_, _>>>()?;
    Some(substitute_completion_type(&result, &substitutions))
}

fn completion_function_result_type(
    function: &ast::FuncDecl,
    type_arguments: Option<&[ast::Type]>,
) -> Option<ast::Type> {
    let result = function.signature.results.first()?;
    let Some(arguments) = type_arguments else {
        return Some(result.clone());
    };
    if function.type_params.len() != arguments.len() {
        return None;
    }
    let substitutions = function
        .type_params
        .iter()
        .zip(arguments)
        .map(|(parameter, argument)| (parameter.name.clone(), argument.clone()))
        .collect::<BTreeMap<_, _>>();
    Some(substitute_completion_type(result, &substitutions))
}

fn resolve_project_completion_struct_type<'a>(
    project: &'a ast::Project,
    resolved: ProjectCompletionType<'a>,
    depth: usize,
) -> Option<(Vec<ast::Field>, &'a ast::Package, &'a ast::File, bool)> {
    if depth > 16 {
        return None;
    }
    match resolved.ty {
        ast::Type::Pointer(inner, _) => resolve_project_completion_struct_type(
            project,
            ProjectCompletionType {
                ty: *inner,
                ..resolved
            },
            depth + 1,
        ),
        ast::Type::Struct { fields, .. } => {
            Some((fields, resolved.package, resolved.file, resolved.external))
        }
        ast::Type::Named(path) => match path.segments.as_slice() {
            [name] => {
                let (ty, declaring_file) =
                    completion_type_declaration(resolved.package, &name.name)?;
                resolve_project_completion_struct_type(
                    project,
                    ProjectCompletionType {
                        ty: ty.clone(),
                        file: declaring_file,
                        ..resolved
                    },
                    depth + 1,
                )
            }
            [alias, name] => {
                let imported = completion_imported_package(project, resolved.file, &alias.name)?;
                let (ty, declaring_file) = completion_type_declaration(imported, &name.name)?;
                resolve_project_completion_struct_type(
                    project,
                    ProjectCompletionType {
                        ty: ty.clone(),
                        package: imported,
                        file: declaring_file,
                        external: true,
                    },
                    depth + 1,
                )
            }
            _ => None,
        },
        ast::Type::Apply {
            base, arguments, ..
        } => match base.segments.as_slice() {
            [name] => {
                let (spec, declaring_file) = completion_type_spec(resolved.package, &name.name)?;
                let ty = substitute_completion_type_arguments(spec, &arguments)?;
                resolve_project_completion_struct_type(
                    project,
                    ProjectCompletionType {
                        ty,
                        file: declaring_file,
                        ..resolved
                    },
                    depth + 1,
                )
            }
            [alias, name] => {
                let imported = completion_imported_package(project, resolved.file, &alias.name)?;
                let (spec, declaring_file) = completion_type_spec(imported, &name.name)?;
                let ty = substitute_completion_type_arguments(spec, &arguments)?;
                resolve_project_completion_struct_type(
                    project,
                    ProjectCompletionType {
                        ty,
                        package: imported,
                        file: declaring_file,
                        external: true,
                    },
                    depth + 1,
                )
            }
            _ => None,
        },
        _ => None,
    }
}

fn completion_type_spec<'a>(
    package: &'a ast::Package,
    name: &str,
) -> Option<(&'a ast::TypeSpec, &'a ast::File)> {
    package.files.iter().find_map(|file| {
        file.decls.iter().find_map(|decl| {
            let ast::TopLevelDecl::Type(decl) = decl else {
                return None;
            };
            decl.specs
                .iter()
                .find(|spec| spec.name.name == name)
                .map(|spec| (spec, file))
        })
    })
}

fn completion_type_declaration<'a>(
    package: &'a ast::Package,
    name: &str,
) -> Option<(&'a ast::Type, &'a ast::File)> {
    completion_type_spec(package, name).map(|(spec, file)| (&spec.ty, file))
}

fn substitute_completion_type_arguments(
    spec: &ast::TypeSpec,
    arguments: &[ast::Type],
) -> Option<ast::Type> {
    if spec.type_params.len() != arguments.len() {
        return None;
    }
    let substitutions = spec
        .type_params
        .iter()
        .zip(arguments)
        .map(|(parameter, argument)| (parameter.name.clone(), argument.clone()))
        .collect::<BTreeMap<_, _>>();
    Some(substitute_completion_type(&spec.ty, &substitutions))
}

fn substitute_completion_type(
    ty: &ast::Type,
    substitutions: &BTreeMap<String, ast::Type>,
) -> ast::Type {
    match ty {
        ast::Type::Named(path) if path.segments.len() == 1 => substitutions
            .get(&path.segments[0].name)
            .cloned()
            .unwrap_or_else(|| ty.clone()),
        ast::Type::Named(_) | ast::Type::Resolved(_, _) => ty.clone(),
        ast::Type::Apply {
            base,
            arguments,
            span,
        } => ast::Type::Apply {
            base: base.clone(),
            arguments: arguments
                .iter()
                .map(|argument| substitute_completion_type(argument, substitutions))
                .collect(),
            span: *span,
        },
        ast::Type::Pointer(inner, span) => ast::Type::Pointer(
            Box::new(substitute_completion_type(inner, substitutions)),
            *span,
        ),
        ast::Type::Array { len, element, span } => ast::Type::Array {
            len: len.clone(),
            element: Box::new(substitute_completion_type(element, substitutions)),
            span: *span,
        },
        ast::Type::Struct { fields, span } => ast::Type::Struct {
            fields: fields
                .iter()
                .map(|field| ast::Field {
                    name: field.name.clone(),
                    ty: substitute_completion_type(&field.ty, substitutions),
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
                    signature: ast::TypeSignature {
                        params: method
                            .signature
                            .params
                            .iter()
                            .map(|parameter| ast::TypeParameter {
                                name: parameter.name.clone(),
                                ty: substitute_completion_type(&parameter.ty, substitutions),
                                span: parameter.span,
                            })
                            .collect(),
                        results: method
                            .signature
                            .results
                            .iter()
                            .map(|result| substitute_completion_type(result, substitutions))
                            .collect(),
                        span: method.signature.span,
                    },
                    span: method.span,
                })
                .collect(),
            span: *span,
        },
        ast::Type::Func { signature, span } => ast::Type::Func {
            signature: ast::TypeSignature {
                params: signature
                    .params
                    .iter()
                    .map(|parameter| ast::TypeParameter {
                        name: parameter.name.clone(),
                        ty: substitute_completion_type(&parameter.ty, substitutions),
                        span: parameter.span,
                    })
                    .collect(),
                results: signature
                    .results
                    .iter()
                    .map(|result| substitute_completion_type(result, substitutions))
                    .collect(),
                span: signature.span,
            },
            span: *span,
        },
    }
}

fn completion_imported_package<'a>(
    project: &'a ast::Project,
    file: &ast::File,
    alias: &str,
) -> Option<&'a ast::Package> {
    let logical_path = file.imports.iter().find_map(|import| {
        let logical_path = compiler::decode_import_path(&import.path).ok()?;
        let import_name = import
            .alias
            .as_ref()
            .map(|name| name.name.as_str())
            .or_else(|| logical_path.rsplit('/').next());
        (import_name == Some(alias)).then_some(logical_path)
    })?;
    project
        .packages
        .iter()
        .find(|package| package.logical_path == logical_path)
}

fn completion_name_is_exported(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|character| character.is_ascii_uppercase())
}

pub fn package_member_completion_items(
    index: &ProjectSymbolIndex,
    file: &ast::File,
    base: &ast::Expr,
) -> Option<Vec<Value>> {
    let ast::Expr::Name(path) = base else {
        return None;
    };
    if path.segments.len() != 1 {
        return None;
    }

    package_member_completion_items_for_alias(index, file, &path.segments[0].name)
}

pub fn package_member_completion_items_for_alias(
    index: &ProjectSymbolIndex,
    file: &ast::File,
    alias: &str,
) -> Option<Vec<Value>> {
    let import_path = file.imports.iter().find_map(|import| {
        let logical_path = compiler::decode_import_path(&import.path).ok()?;
        let import_name = import
            .alias
            .as_ref()
            .map(|name| name.name.as_str())
            .or_else(|| logical_path.rsplit('/').next());
        (import_name == Some(alias)).then_some(logical_path)
    })?;
    Some(
        index
            .package_symbols(&import_path)
            .iter()
            .filter(|symbol| symbol.exported)
            .map(|symbol| indexed_symbol_completion_item(symbol, true))
            .collect(),
    )
}

pub fn completion_function_signature(function: &ast::FuncDecl) -> String {
    let params = function
        .signature
        .params
        .iter()
        .map(|param| format!("{}: {}", param.name.name, completion_type_name(&param.ty)))
        .collect::<Vec<_>>()
        .join(", ");
    let mut signature = format!("{}({params})", function.name.name);
    match function.signature.results.as_slice() {
        [] => {}
        [result] => signature.push_str(&format!(" -> {}", completion_type_name(result))),
        results => {
            let results = results
                .iter()
                .map(completion_type_name)
                .collect::<Vec<_>>()
                .join(", ");
            signature.push_str(&format!(" -> ({results})"));
        }
    }
    signature
}

fn resolve_completion_expr_type(
    package: &ast::Package,
    function: Option<CompletionScope<'_>>,
    expr: &ast::Expr,
) -> Option<ast::Type> {
    match expr {
        ast::Expr::Name(path) if path.segments.len() == 1 => {
            completion_type_for_name(package, function, &path.segments[0].name, expr.span().start)
        }
        ast::Expr::Selector { base, field, .. } => {
            let base_ty = resolve_completion_expr_type(package, function, base)?;
            let fields = resolve_completion_struct_fields_for_type(package, &base_ty, 0)?;
            fields
                .into_iter()
                .find(|candidate| candidate.name.name == field.name)
                .map(|field| field.ty)
        }
        ast::Expr::Cast { ty, .. } | ast::Expr::Composite { ty, .. } => Some(ty.clone()),
        ast::Expr::New { ty, span } => Some(ast::Type::Pointer(Box::new(ty.clone()), *span)),
        ast::Expr::Call { callee, .. } => completion_call_result(package, callee),
        _ => None,
    }
}

fn completion_type_for_name(
    package: &ast::Package,
    function: Option<CompletionScope<'_>>,
    name: &str,
    limit: usize,
) -> Option<ast::Type> {
    if let Some(function) = function {
        let mut locals = BTreeMap::new();
        if let Some(receiver) = function.receiver() {
            locals.insert(receiver.name.name.clone(), receiver.ty.clone());
        }
        for param in function.params() {
            locals.insert(param.name.name.clone(), param.ty.clone());
        }
        collect_completion_local_types(package, &function.body().statements, limit, &mut locals);
        if let Some(ty) = locals.get(name) {
            return Some(ty.clone());
        }
    }

    for file in &package.files {
        for decl in &file.decls {
            let ast::TopLevelDecl::Var(decl) = decl else {
                continue;
            };
            for spec in &decl.specs {
                for (index, declared_name) in spec.names.iter().enumerate() {
                    if declared_name.name != name {
                        continue;
                    }
                    if let Some(ty) = &spec.ty {
                        return Some(ty.clone());
                    }
                    if let Some(value) = spec.values.get(index) {
                        if let Some(ty) =
                            completion_type_from_known_expr(package, value, &BTreeMap::new())
                        {
                            return Some(ty);
                        }
                    }
                }
            }
        }
    }
    None
}

fn collect_completion_local_types(
    package: &ast::Package,
    statements: &[ast::Stmt],
    limit: usize,
    types: &mut BTreeMap<String, ast::Type>,
) {
    for stmt in statements {
        if stmt_span(stmt).start > limit {
            break;
        }
        match stmt {
            ast::Stmt::Var(decl) => {
                for spec in &decl.specs {
                    if spec.span.start > limit {
                        continue;
                    }
                    for (index, name) in spec.names.iter().enumerate() {
                        if let Some(ty) = &spec.ty {
                            types.insert(name.name.clone(), ty.clone());
                        } else if let Some(value) = spec.values.get(index) {
                            if let Some(ty) = completion_type_from_known_expr(package, value, types)
                            {
                                types.insert(name.name.clone(), ty);
                            }
                        }
                    }
                }
            }
            ast::Stmt::ShortVar(decl) => {
                if decl.span.start <= limit {
                    for (name, value) in decl.names.iter().zip(&decl.values) {
                        if let Some(ty) = completion_type_from_known_expr(package, value, types) {
                            types.insert(name.name.clone(), ty);
                        }
                    }
                }
            }
            ast::Stmt::Block(block) => {
                collect_completion_local_types(package, &block.statements, limit, types);
            }
            ast::Stmt::Defer(stmt) => {
                collect_completion_local_types(package, &stmt.block.statements, limit, types);
            }
            ast::Stmt::If(stmt) => {
                if let Some(init) = &stmt.init {
                    collect_completion_local_types(
                        package,
                        std::slice::from_ref(init),
                        limit,
                        types,
                    );
                }
                collect_completion_local_types(package, &stmt.then_block.statements, limit, types);
                if let Some(else_branch) = &stmt.else_branch {
                    collect_completion_local_types(
                        package,
                        std::slice::from_ref(else_branch),
                        limit,
                        types,
                    );
                }
            }
            ast::Stmt::For(stmt) => {
                if let ast::ForKind::ThreeClause {
                    init: Some(init), ..
                } = &stmt.kind
                {
                    collect_completion_local_types(
                        package,
                        std::slice::from_ref(init),
                        limit,
                        types,
                    );
                }
                collect_completion_local_types(package, &stmt.body.statements, limit, types);
            }
            _ => {}
        }
    }
}

fn completion_type_from_known_expr(
    package: &ast::Package,
    expr: &ast::Expr,
    types: &BTreeMap<String, ast::Type>,
) -> Option<ast::Type> {
    match expr {
        ast::Expr::Cast { ty, .. } | ast::Expr::Composite { ty, .. } => Some(ty.clone()),
        ast::Expr::New { ty, span } => Some(ast::Type::Pointer(Box::new(ty.clone()), *span)),
        ast::Expr::Name(path) if path.segments.len() == 1 => {
            types.get(&path.segments[0].name).cloned()
        }
        ast::Expr::Call { callee, .. } => completion_call_result(package, callee),
        _ => None,
    }
}

fn completion_call_result(package: &ast::Package, callee: &ast::Expr) -> Option<ast::Type> {
    let (name, type_arguments) = match callee {
        ast::Expr::Name(path) if path.segments.len() == 1 => (&path.segments[0].name, None),
        ast::Expr::TypeApply {
            base, arguments, ..
        } => {
            let ast::Expr::Name(path) = base.as_ref() else {
                return None;
            };
            if path.segments.len() != 1 {
                return None;
            }
            (&path.segments[0].name, Some(arguments.as_slice()))
        }
        _ => return None,
    };
    package.files.iter().find_map(|file| {
        file.decls.iter().find_map(|decl| {
            let ast::TopLevelDecl::Func(function) = decl else {
                return None;
            };
            if function.receiver.is_some() || function.name.name != *name {
                return None;
            }
            completion_function_result_type(function, type_arguments)
        })
    })
}

fn resolve_completion_struct_fields_for_type(
    package: &ast::Package,
    ty: &ast::Type,
    depth: usize,
) -> Option<Vec<ast::Field>> {
    if depth > 16 {
        return None;
    }
    match ty {
        ast::Type::Struct { fields, .. } => Some(fields.clone()),
        ast::Type::Pointer(inner, _) => {
            resolve_completion_struct_fields_for_type(package, inner, depth + 1)
        }
        ast::Type::Named(path) if path.segments.len() == 1 => package
            .files
            .iter()
            .flat_map(|file| &file.decls)
            .find_map(|decl| {
                let ast::TopLevelDecl::Type(type_decl) = decl else {
                    return None;
                };
                type_decl
                    .specs
                    .iter()
                    .find(|spec| spec.name.name == path.segments[0].name)
                    .and_then(|spec| {
                        resolve_completion_struct_fields_for_type(package, &spec.ty, depth + 1)
                    })
            }),
        ast::Type::Apply {
            base, arguments, ..
        } if base.segments.len() == 1 => {
            let (spec, _) = completion_type_spec(package, &base.segments[0].name)?;
            let ty = substitute_completion_type_arguments(spec, arguments)?;
            resolve_completion_struct_fields_for_type(package, &ty, depth + 1)
        }
        _ => None,
    }
}

pub fn struct_field_completion_items(fields: &[ast::Field]) -> Vec<Value> {
    fields
        .iter()
        .map(|field| {
            let detail = format!("{}: {}", field.name.name, completion_type_name(&field.ty));
            completion_item(&field.name.name, 5, Some(&detail), Some("struct field"))
        })
        .collect()
}

pub(crate) fn completion_type_name(ty: &ast::Type) -> String {
    match ty {
        ast::Type::Named(path) => path
            .segments
            .iter()
            .map(|segment| segment.name.as_str())
            .collect::<Vec<_>>()
            .join("."),
        ast::Type::Apply {
            base, arguments, ..
        } => format!(
            "{}[{}]",
            base.segments
                .iter()
                .map(|segment| segment.name.as_str())
                .collect::<Vec<_>>()
                .join("."),
            arguments
                .iter()
                .map(completion_type_name)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ast::Type::Resolved(_, _) => "<resolved>".to_string(),
        ast::Type::Pointer(inner, _) => format!("*{}", completion_type_name(inner)),
        ast::Type::Array { element, .. } => format!("[...]{}", completion_type_name(element)),
        ast::Type::Struct { .. } => "struct".to_string(),
        ast::Type::Interface { .. } => "interface".to_string(),
        ast::Type::Func { .. } => "func".to_string(),
    }
}

pub fn append_import_completion_items(items: &mut Vec<Value>, file: &ast::File) {
    for import in &file.imports {
        let alias = import
            .alias
            .as_ref()
            .map(|alias| alias.name.as_str())
            .unwrap_or_else(|| {
                import
                    .path
                    .trim_matches('"')
                    .rsplit('/')
                    .next()
                    .unwrap_or_default()
            });
        if !alias.is_empty() {
            items.push(completion_item(
                alias,
                9,
                Some("import"),
                Some(import.path.trim_matches('"')),
            ));
        }
    }
}

pub fn append_top_level_completion_items(items: &mut Vec<Value>, file: &ast::File) {
    for decl in &file.decls {
        match decl {
            ast::TopLevelDecl::Const(decl) => {
                for spec in &decl.specs {
                    for name in &spec.names {
                        items.push(completion_item(&name.name, 21, Some("const"), None));
                    }
                }
            }
            ast::TopLevelDecl::Var(decl) => {
                for spec in &decl.specs {
                    for name in &spec.names {
                        items.push(completion_item(&name.name, 6, Some("var"), None));
                    }
                }
            }
            ast::TopLevelDecl::Type(decl) => {
                for spec in &decl.specs {
                    items.push(completion_item(&spec.name.name, 7, Some("type"), None));
                }
            }
            ast::TopLevelDecl::Func(decl) => {
                let signature = completion_function_signature(decl);
                items.push(completion_function_item(
                    &decl.name.name,
                    &signature,
                    "function",
                ));
            }
            ast::TopLevelDecl::Operator(_) => {}
        }
    }
}

pub fn append_top_level_variable_completion_items(items: &mut Vec<Value>, file: &ast::File) {
    for decl in &file.decls {
        let ast::TopLevelDecl::Var(decl) = decl else {
            continue;
        };
        for spec in &decl.specs {
            for name in &spec.names {
                items.push(completion_item(&name.name, 6, Some("var"), None));
            }
        }
    }
}

pub fn append_indexed_symbol_completion_items(
    items: &mut Vec<Value>,
    index: &ProjectSymbolIndex,
    package: &str,
) {
    items.extend(
        index
            .package_symbols(package)
            .iter()
            .map(|symbol| indexed_symbol_completion_item(symbol, false)),
    );
}

fn indexed_symbol_completion_item(symbol: &IndexedSymbol, imported: bool) -> Value {
    let description = match (symbol.kind, imported) {
        (IndexedSymbolKind::Function, false) => "function",
        (IndexedSymbolKind::Function, true) => "package function",
        (IndexedSymbolKind::Constant, false) => "const",
        (IndexedSymbolKind::Constant, true) => "package constant",
        (IndexedSymbolKind::Type, false) => "type",
        (IndexedSymbolKind::Type, true) => "package type",
    };
    match symbol.kind {
        IndexedSymbolKind::Function => {
            completion_function_item(&symbol.name, &symbol.detail, description)
        }
        IndexedSymbolKind::Constant => {
            completion_item(&symbol.name, 21, Some(&symbol.detail), Some(description))
        }
        IndexedSymbolKind::Type => {
            completion_item(&symbol.name, 7, Some(&symbol.detail), Some(description))
        }
    }
}

pub fn append_local_completion_items(items: &mut Vec<Value>, file: &ast::File, offset: usize) {
    for decl in &file.decls {
        let scope = match decl {
            ast::TopLevelDecl::Func(function) => CompletionScope::Function(function),
            ast::TopLevelDecl::Operator(operator) => CompletionScope::Operator(operator),
            _ => continue,
        };
        if offset < scope.body().span.start || offset > scope.body().span.end {
            continue;
        }

        for param in scope.params() {
            items.push(completion_item(&param.name.name, 6, Some("param"), None));
        }
        if let Some(receiver) = scope.receiver() {
            items.push(completion_item(
                &receiver.name.name,
                6,
                Some("receiver"),
                None,
            ));
        }
        append_stmt_completion_items(items, &scope.body().statements, offset);
    }
}

fn append_stmt_completion_items(items: &mut Vec<Value>, statements: &[ast::Stmt], offset: usize) {
    for stmt in statements {
        if stmt_span(stmt).start > offset {
            break;
        }
        match stmt {
            ast::Stmt::Const(decl) => {
                for spec in &decl.specs {
                    if spec.span.start <= offset {
                        for name in &spec.names {
                            items.push(completion_item(&name.name, 21, Some("const"), None));
                        }
                    }
                }
            }
            ast::Stmt::Var(decl) => {
                for spec in &decl.specs {
                    if spec.span.start <= offset {
                        for name in &spec.names {
                            items.push(completion_item(&name.name, 6, Some("var"), None));
                        }
                    }
                }
            }
            ast::Stmt::ShortVar(decl) => {
                if decl.span.start <= offset {
                    for name in &decl.names {
                        items.push(completion_item(&name.name, 6, Some("var"), None));
                    }
                }
            }
            ast::Stmt::Block(block) => {
                if offset >= block.span.start && offset <= block.span.end {
                    append_stmt_completion_items(items, &block.statements, offset);
                }
            }
            ast::Stmt::Defer(stmt) => {
                if offset >= stmt.block.span.start && offset <= stmt.block.span.end {
                    append_stmt_completion_items(items, &stmt.block.statements, offset);
                }
            }
            ast::Stmt::If(stmt) => {
                if let Some(init) = &stmt.init {
                    append_stmt_completion_items(items, std::slice::from_ref(init), offset);
                }
                if offset >= stmt.then_block.span.start && offset <= stmt.then_block.span.end {
                    append_stmt_completion_items(items, &stmt.then_block.statements, offset);
                }
                if let Some(else_branch) = &stmt.else_branch {
                    append_stmt_completion_items(items, std::slice::from_ref(else_branch), offset);
                }
            }
            ast::Stmt::For(stmt) => {
                if offset >= stmt.body.span.start && offset <= stmt.body.span.end {
                    append_stmt_completion_items(items, &stmt.body.statements, offset);
                }
            }
            _ => {}
        }
    }
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

pub fn dedupe_completion_items(items: Vec<Value>) -> Result<Vec<Value>, String> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();
    for item in items {
        let label = item
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let kind = item.get("kind").and_then(Value::as_u64).unwrap_or_default();
        let key = (label.to_string(), kind);
        if seen.insert(key) {
            deduped.push(item);
        }
    }
    Ok(deduped)
}

fn completion_item(
    label: &str,
    kind: u32,
    detail: Option<&str>,
    description: Option<&str>,
) -> Value {
    json!({
        "label": label,
        "kind": kind,
        "detail": detail,
        "insertText": label,
        "labelDetails": {
            "description": description
        }
    })
}

fn completion_function_item(label: &str, signature: &str, description: &str) -> Value {
    completion_item(label, 3, Some(signature), Some(description))
}

#[cfg(test)]
mod tests {
    use super::{
        append_builtin_function_completion_items, completion_qualifier,
        completion_text_with_value_placeholder, import_path_completion_context,
        import_path_completion_items,
    };

    #[test]
    fn extracts_the_qualifier_before_a_member_completion() {
        assert_eq!(
            completion_qualifier("    graphic.Present()", 0, 12).as_deref(),
            Some("graphic")
        );
        assert_eq!(
            completion_qualifier("    graphic.Pre", 0, 15).as_deref(),
            Some("graphic")
        );
        assert!(completion_qualifier("    Present()", 0, 7).is_none());
    }

    #[test]
    fn repairs_an_expression_hole_after_operators() {
        for source in ["value + ", "value == ", "call(", "items[index] = "] {
            let repaired = completion_text_with_value_placeholder(
                source,
                0,
                source.encode_utf16().count() as u32,
            )
            .unwrap();
            assert!(repaired.ends_with("__ond_completion_value"), "{repaired}");
        }
        assert!(completion_text_with_value_placeholder("value ", 0, 6).is_none());
    }

    #[test]
    fn source_location_builtins_include_signatures_in_completion() {
        let mut items = Vec::new();
        append_builtin_function_completion_items(&mut items);
        for (name, signature) in [
            ("thisFile", "thisFile() -> *u8"),
            ("thisLine", "thisLine() -> u32"),
        ] {
            let item = items
                .iter()
                .find(|item| item["label"] == name)
                .unwrap_or_else(|| panic!("missing {name}: {items:?}"));
            assert_eq!(item["detail"], signature);
        }
    }

    #[test]
    fn detects_import_strings_and_replaces_the_whole_path() {
        let text = "package main\nimport \"tools/old\"\n";
        let context = import_path_completion_context(text, 1, 12).unwrap();
        assert_eq!(context.prefix, "tool");
        let items =
            import_path_completion_items(text, &context, ["tools/diag", "tools/log", "shared"]);

        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["label"], "tools/diag");
        assert_eq!(items[0]["textEdit"]["newText"], "tools/diag");
        assert_eq!(items[0]["textEdit"]["range"]["start"]["character"], 8);
        assert_eq!(items[0]["textEdit"]["range"]["end"]["character"], 17);
    }

    #[test]
    fn detects_an_unterminated_empty_import_but_not_comments_or_other_strings() {
        assert!(import_path_completion_context("import \"", 0, 8).is_some());
        assert!(import_path_completion_context("// import \"tools", 0, 16).is_none());
        assert!(import_path_completion_context("var path = \"tools", 0, 17).is_none());
    }
}
