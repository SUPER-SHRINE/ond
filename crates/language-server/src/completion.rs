use std::collections::{BTreeMap, BTreeSet};

use compiler::ir::ast;
#[path = "completion_expression.rs"]
mod expression;
use compiler::source::Span;
use serde_json::{Value, json};

use crate::protocol::{Position, lsp_offset, mask_block_comments};
use crate::symbol_index::{IndexedSymbol, IndexedSymbolKind, ProjectSymbolIndex};

const MAX_COMPLETION_INFERRED_POINTER_DEPTH: usize = 16;

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
        "package", "import", "const", "var", "type", "func", "struct", "return", "trap", "defer",
        "break", "continue", "if", "else", "for", "true", "false", "nil", "as",
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
        ("u64", "64-bit unsigned integer"),
        ("i64", "64-bit signed integer"),
        ("f64", "64-bit floating-point number"),
        (
            "q16",
            "32-bit signed fixed-point number with 16 fractional bits",
        ),
        (
            "q32",
            "64-bit signed fixed-point number with 32 fractional bits",
        ),
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
            Self::Operator(operator) => operator.signature.params.first(),
        }
    }
}

pub fn completion_text_with_placeholder(
    text: &str,
    line: u32,
    character: u32,
) -> Option<(String, &'static str)> {
    let offset = lsp_offset(text, &Position { line, character }).ok()?;

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

pub(crate) fn completion_text_with_identifier_call(
    text: &str,
    line: u32,
    character: u32,
) -> Option<String> {
    let offset = lsp_offset(text, &Position { line, character }).ok()?;
    let masked = mask_block_comments(text);
    let code = masked.as_deref().unwrap_or(text);
    let before = code[..offset]
        .rsplit_once('\n')
        .map_or(&code[..offset], |(_, line)| line);
    let after = code[offset..].split('\n').next()?;
    let is_identifier_char = |ch: char| ch == '_' || ch.is_alphanumeric();
    let prefix = before.trim_end_matches(is_identifier_char);
    let identifier = &before[prefix.len()..];
    let first = identifier.chars().next()?;
    if !(first == '_' || first.is_alphabetic()) {
        return None;
    }

    let prefix = prefix.trim_end();
    if !prefix.is_empty() && !prefix.ends_with(['{', ';']) {
        return None;
    }
    let suffix = after.trim_start_matches(is_identifier_char);
    let remainder = suffix.trim_start();
    if !remainder.is_empty() && !remainder.starts_with(['}', ';']) && !remainder.starts_with("//") {
        return None;
    }

    // A call is a valid expression statement and introduces no local binding.
    let mut completed = text.to_string();
    completed.insert_str(offset + after.len() - suffix.len(), "()");
    Some(completed)
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
        ast::Stmt::InlineAssembly(statement) => statement
            .bindings
            .iter()
            .find_map(|binding| find_completion_selector_in_expr(&binding.value, marker, function)),
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
    resolve_completion_struct_fields_for_type(package, &ty, 0, 0)
}

pub fn resolve_project_completion_struct_fields(
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
    function: Option<CompletionScope<'_>>,
    base: &ast::Expr,
) -> Option<Vec<ast::Field>> {
    let ty = resolve_project_completion_expr_type(project, package, file, function, base)?;
    let (fields, _, _, external) = resolve_project_completion_struct_type(project, ty, 0, 0)?;
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
    let exact_receiver = project_completion_method_receiver(project, &resolved)?;
    let exact_receiver_key = (
        exact_receiver.0.logical_path.clone(),
        exact_receiver.1.clone(),
    );
    let mut receivers = vec![(exact_receiver, false)];
    if let Some(resolved) =
        project_completion_implicit_dereference_type(project, resolved.clone(), 0)
        && let Some(receiver) = project_completion_method_receiver(project, &resolved)
        && (receiver.0.logical_path != exact_receiver_key.0 || receiver.1 != exact_receiver_key.1)
    {
        receivers.push((receiver, true));
    }
    let addressable =
        project_completion_expr_is_addressable(project, package, file, function, base);
    let mut methods = Vec::new();
    for ((method_package, name, pointer_depth, external), exact_only) in receivers {
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
                    && if exact_only {
                        receiver_pointer == pointer_depth
                    } else {
                        completion_receiver_matches(pointer_depth, receiver_pointer, addressable)
                    }
                    && (!external || completion_name_is_exported(&method.name.name))
                {
                    methods.push(method_completion_item(method));
                }
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
    let exact_receiver = completion_exact_named_receiver(&ty)?;
    let mut receivers = vec![(exact_receiver.0.to_string(), exact_receiver.1, false)];
    if let Some(ty) = completion_implicit_dereference_type(package, &ty, 0)
        && let Some(receiver) = completion_exact_named_receiver(&ty)
        && receiver.0 != exact_receiver.0
    {
        receivers.push((receiver.0.to_string(), receiver.1, true));
    }
    let addressable = completion_expr_is_addressable(package, function, base);
    let mut methods = Vec::new();
    for (name, pointer_depth, exact_only) in receivers {
        for file in &package.files {
            for decl in &file.decls {
                let ast::TopLevelDecl::Func(method) = decl else {
                    continue;
                };
                let Some(receiver) = &method.receiver else {
                    continue;
                };
                if completion_exact_named_receiver(&receiver.ty).is_some_and(
                    |(receiver_name, receiver_pointer)| {
                        receiver_name == name
                            && if exact_only {
                                receiver_pointer == pointer_depth
                            } else {
                                completion_receiver_matches(
                                    pointer_depth,
                                    receiver_pointer,
                                    addressable,
                                )
                            }
                    },
                ) {
                    methods.push(method_completion_item(method));
                }
            }
        }
    }
    Some(methods)
}

fn completion_exact_named_receiver(ty: &ast::Type) -> Option<(&str, usize)> {
    let (path, pointer_depth) = completion_receiver_path(ty, 0)?;
    (path.segments.len() == 1).then(|| (path.segments[0].name.as_str(), pointer_depth))
}

fn completion_receiver_path(ty: &ast::Type, pointer_depth: usize) -> Option<(&ast::Path, usize)> {
    match ty {
        ast::Type::Named(path) => Some((path, pointer_depth)),
        ast::Type::Apply { base, .. } => Some((base, pointer_depth)),
        ast::Type::Pointer(inner, _) => completion_receiver_path(inner, pointer_depth + 1),
        _ => None,
    }
}

fn completion_pointer_depth(ty: &ast::Type) -> usize {
    let mut depth = 0;
    let mut ty = ty;
    while let ast::Type::Pointer(inner, _) = ty {
        depth += 1;
        ty = inner;
    }
    depth
}

fn completion_receiver_matches(
    actual_pointer_depth: usize,
    declared_pointer_depth: usize,
    addressable: bool,
) -> bool {
    declared_pointer_depth == actual_pointer_depth
        || actual_pointer_depth == declared_pointer_depth + 1
        || (addressable && declared_pointer_depth == actual_pointer_depth + 1)
}

fn completion_implicit_dereference_type(
    package: &ast::Package,
    ty: &ast::Type,
    depth: usize,
) -> Option<ast::Type> {
    if depth > 16 {
        return None;
    }
    match ty {
        ast::Type::Pointer(inner, _) => Some((**inner).clone()),
        ast::Type::Named(path) if path.segments.len() == 1 => {
            let (ty, _) = completion_type_declaration(package, &path.segments[0].name)?;
            completion_implicit_dereference_type(package, ty, depth + 1)
        }
        ast::Type::Apply {
            base, arguments, ..
        } if base.segments.len() == 1 => {
            let (spec, _) = completion_type_spec(package, &base.segments[0].name)?;
            let ty = substitute_completion_type_arguments(spec, arguments)?;
            completion_implicit_dereference_type(package, &ty, depth + 1)
        }
        _ => None,
    }
}

fn completion_expr_is_addressable(
    package: &ast::Package,
    function: Option<CompletionScope<'_>>,
    expr: &ast::Expr,
) -> bool {
    match expr {
        ast::Expr::Name(_)
        | ast::Expr::Unary {
            op: ast::UnaryOp::Deref,
            ..
        } => true,
        ast::Expr::Selector { base, .. } | ast::Expr::Index { base, .. } => {
            resolve_completion_expr_type(package, function, base).is_some_and(|ty| {
                matches!(ty, ast::Type::Pointer(_, _))
                    || completion_implicit_dereference_type(package, &ty, 0).is_some()
            }) || completion_expr_is_addressable(package, function, base)
        }
        _ => false,
    }
}

pub(crate) fn project_completion_expr_is_addressable(
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
    function: Option<CompletionScope<'_>>,
    expr: &ast::Expr,
) -> bool {
    project_completion_expr_is_addressable_with(
        project,
        package,
        file,
        ProjectExprLookup::Scope(function),
        expr,
    )
}

fn project_completion_expr_is_addressable_with(
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
    lookup: ProjectExprLookup<'_, '_>,
    expr: &ast::Expr,
) -> bool {
    match expr {
        ast::Expr::Name(_)
        | ast::Expr::Unary {
            op: ast::UnaryOp::Deref,
            ..
        } => true,
        ast::Expr::Selector { base, .. } | ast::Expr::Index { base, .. } => {
            resolve_project_completion_expr_type_with(project, package, file, lookup, base)
                .is_some_and(|resolved| {
                    matches!(resolved.ty, ast::Type::Pointer(_, _))
                        || project_completion_implicit_dereference_type(
                            project,
                            resolved.clone(),
                            0,
                        )
                        .is_some()
                })
                || project_completion_expr_is_addressable_with(project, package, file, lookup, base)
        }
        _ => false,
    }
}

fn method_completion_item(method: &ast::FuncDecl) -> Value {
    let detail = completion_function_signature(method);
    completion_item(&method.name.name, 2, Some(&detail), Some("method"))
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

type ProjectLocalTypes<'a> = BTreeMap<String, ProjectLocalBinding<'a>>;

struct ProjectLocalBinding<'a> {
    scope_start: usize,
    ty: Option<ProjectCompletionType<'a>>,
}

#[derive(Clone, Copy)]
enum ProjectExprLookup<'a, 'b> {
    Scope(Option<CompletionScope<'b>>),
    Initializer(&'b ProjectLocalTypes<'a>),
}

impl<'a> ProjectExprLookup<'a, '_> {
    fn value(
        self,
        project: &'a ast::Project,
        package: &'a ast::Package,
        file: &'a ast::File,
        name: &ast::Ident,
    ) -> Option<Option<ProjectCompletionType<'a>>> {
        match self {
            Self::Scope(function) => project_completion_value_lookup(
                project,
                package,
                file,
                function,
                &name.name,
                name.span.start,
            ),
            Self::Initializer(types) => match types.get(&name.name) {
                Some(binding) => Some(binding.ty.clone()),
                None => project_completion_package_value_type(project, package, &name.name, false)
                    .map(Some),
            },
        }
    }
}

fn project_completion_method_receiver<'a>(
    project: &'a ast::Project,
    resolved: &ProjectCompletionType<'a>,
) -> Option<(&'a ast::Package, String, usize, bool)> {
    let (path, pointer_depth) = completion_receiver_path(&resolved.ty, 0)?;
    match path.segments.as_slice() {
        [name] => Some((
            resolved.package,
            name.name.clone(),
            pointer_depth,
            resolved.external,
        )),
        [alias, name] => Some((
            completion_imported_package(project, resolved.file, &alias.name)?,
            name.name.clone(),
            pointer_depth,
            true,
        )),
        _ => None,
    }
}

fn project_completion_implicit_dereference_type<'a>(
    project: &'a ast::Project,
    resolved: ProjectCompletionType<'a>,
    depth: usize,
) -> Option<ProjectCompletionType<'a>> {
    if depth > 16 {
        return None;
    }
    match resolved.ty {
        ast::Type::Pointer(inner, _) => Some(ProjectCompletionType {
            ty: *inner,
            ..resolved
        }),
        ast::Type::Named(path) => match path.segments.as_slice() {
            [name] => {
                let (ty, declaring_file) =
                    completion_type_declaration(resolved.package, &name.name)?;
                project_completion_implicit_dereference_type(
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
                let (ty, _) = completion_type_declaration(imported, &name.name)?;
                let ty = qualify_project_value_type(ty.clone(), &alias.name, imported);
                project_completion_implicit_dereference_type(
                    project,
                    ProjectCompletionType { ty, ..resolved },
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
                project_completion_implicit_dereference_type(
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
                let (spec, _) = completion_type_spec(imported, &name.name)?;
                let ty = substitute_imported_completion_type_arguments(
                    spec,
                    &arguments,
                    &alias.name,
                    imported,
                )?;
                project_completion_implicit_dereference_type(
                    project,
                    ProjectCompletionType { ty, ..resolved },
                    depth + 1,
                )
            }
            _ => None,
        },
        _ => None,
    }
}

fn substitute_imported_completion_type_arguments(
    spec: &ast::TypeSpec,
    arguments: &[ast::Type],
    alias: &str,
    declaring_package: &ast::Package,
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
    let ty = qualify_project_type_spec(&spec.ty, alias, declaring_package, &spec.type_params);
    Some(substitute_completion_type(&ty, &substitutions))
}

fn qualify_project_type_spec(
    ty: &ast::Type,
    alias: &str,
    declaring_package: &ast::Package,
    type_params: &[ast::Ident],
) -> ast::Type {
    match ty {
        ast::Type::Named(path)
            if path.segments.len() == 1
                && type_params
                    .iter()
                    .any(|parameter| parameter.name == path.segments[0].name) =>
        {
            ty.clone()
        }
        ast::Type::Pointer(inner, span) => ast::Type::Pointer(
            Box::new(qualify_project_type_spec(
                inner,
                alias,
                declaring_package,
                type_params,
            )),
            *span,
        ),
        ast::Type::Array { len, element, span } => ast::Type::Array {
            len: len.clone(),
            element: Box::new(qualify_project_type_spec(
                element,
                alias,
                declaring_package,
                type_params,
            )),
            span: *span,
        },
        ast::Type::Apply {
            base,
            arguments,
            span,
        } if base.segments.len() == 1 => {
            let mut base = base.clone();
            base.segments.insert(
                0,
                ast::Ident {
                    name: alias.to_string(),
                    span: base.span,
                },
            );
            ast::Type::Apply {
                base,
                arguments: arguments
                    .iter()
                    .map(|argument| {
                        qualify_project_type_spec(argument, alias, declaring_package, type_params)
                    })
                    .collect(),
                span: *span,
            }
        }
        _ => qualify_project_value_type(ty.clone(), alias, declaring_package),
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
    resolve_project_completion_expr_type_with(
        project,
        package,
        file,
        ProjectExprLookup::Scope(function),
        expr,
    )
}

fn resolve_project_completion_expr_type_with<'a>(
    project: &'a ast::Project,
    package: &'a ast::Package,
    file: &'a ast::File,
    lookup: ProjectExprLookup<'a, '_>,
    expr: &ast::Expr,
) -> Option<ProjectCompletionType<'a>> {
    let typed = |ty| ProjectCompletionType {
        ty,
        package,
        file,
        external: false,
    };
    match expr {
        ast::Expr::Literal(ast::Literal::Int(_, span)) => Some(typed(primitive_type("i32", *span))),
        ast::Expr::Literal(ast::Literal::Float(_, span)) => {
            Some(typed(primitive_type("f32", *span)))
        }
        ast::Expr::Literal(ast::Literal::Bool(_, span)) => {
            Some(typed(primitive_type("bool", *span)))
        }
        ast::Expr::LayoutQuery { span, .. } => Some(typed(primitive_type("u32", *span))),
        ast::Expr::Name(path) if !path.segments.is_empty() => {
            if let [name] = path.segments.as_slice() {
                if let Some(ty) = lookup.value(project, package, file, name) {
                    return ty;
                }
                let (function, declaring_file) = project_completion_function(package, &name.name)?;
                if !function.type_params.is_empty() {
                    return None;
                }
                let signature = &function.signature;
                return Some(ProjectCompletionType {
                    ty: ast::Type::Func {
                        signature: ast::TypeSignature {
                            params: signature
                                .params
                                .iter()
                                .map(|param| ast::TypeParameter {
                                    name: Some(param.name.clone()),
                                    ty: param.ty.clone(),
                                    span: param.span,
                                })
                                .collect(),
                            results: signature.results.clone(),
                            span: signature.span,
                        },
                        span: signature.span,
                    },
                    file: declaring_file,
                    package,
                    external: false,
                });
            }
            let (mut resolved, fields) =
                match lookup.value(project, package, file, &path.segments[0]) {
                    Some(ty) => (ty?, &path.segments[1..]),
                    None => {
                        let [alias, name, rest @ ..] = path.segments.as_slice() else {
                            return None;
                        };
                        (
                            project_completion_imported_value(project, file, alias, name)?,
                            rest,
                        )
                    }
                };
            for segment in fields {
                let (fields, field_package, field_file, external) =
                    resolve_project_completion_struct_type(project, resolved, 0, 0)?;
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
            if let ast::Expr::Name(path) = base.as_ref()
                && let [alias] = path.segments.as_slice()
                && lookup.value(project, package, file, alias).is_none()
                && let Some(value) = project_completion_imported_value(project, file, alias, field)
            {
                return Some(value);
            }
            let base =
                resolve_project_completion_expr_type_with(project, package, file, lookup, base)?;
            let (fields, field_package, field_file, external) =
                resolve_project_completion_struct_type(project, base, 0, 0)?;
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
        ast::Expr::Cast { ty, .. } | ast::Expr::Composite { ty, .. } => Some(typed(ty.clone())),
        ast::Expr::New { ty, span } => Some(typed(ast::Type::Pointer(Box::new(ty.clone()), *span))),
        ast::Expr::Call { callee, .. } => {
            project_completion_call_result(project, package, file, lookup, callee)
        }
        ast::Expr::Unary {
            op: ast::UnaryOp::Deref,
            expr,
            ..
        } => {
            let mut resolved =
                resolve_project_completion_expr_type_with(project, package, file, lookup, expr)?;
            let ast::Type::Pointer(inner, _) = resolved.ty else {
                return None;
            };
            resolved.ty = *inner;
            Some(resolved)
        }
        ast::Expr::Unary {
            op: ast::UnaryOp::AddrOf,
            expr,
            span,
        } => {
            let mut resolved =
                resolve_project_completion_expr_type_with(project, package, file, lookup, expr)?;
            if completion_pointer_depth(&resolved.ty) >= MAX_COMPLETION_INFERRED_POINTER_DEPTH {
                return None;
            }
            resolved.ty = ast::Type::Pointer(Box::new(resolved.ty), *span);
            Some(resolved)
        }
        ast::Expr::Unary { .. } | ast::Expr::Binary { .. } => {
            expression::operator_type(expr, |operand| {
                resolve_project_completion_expr_type_with(project, package, file, lookup, operand)
                    .map(|resolved| resolved.ty)
            })
            .map(typed)
        }
        ast::Expr::Index { base, .. } | ast::Expr::TypeApply { base, .. } => {
            // The parser can represent a named index as a single type argument.
            if let ast::Expr::TypeApply { arguments, .. } = expr
                && !matches!(arguments.as_slice(), [ast::Type::Named(_)])
            {
                return None;
            }
            let mut resolved =
                resolve_project_completion_expr_type_with(project, package, file, lookup, base)?;
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
    project_completion_value_lookup(project, package, file, function, name, limit).flatten()
}

fn project_completion_value_lookup<'a>(
    project: &'a ast::Project,
    package: &'a ast::Package,
    file: &'a ast::File,
    function: Option<CompletionScope<'_>>,
    name: &str,
    limit: usize,
) -> Option<Option<ProjectCompletionType<'a>>> {
    if let Some(function) = function {
        let mut locals = BTreeMap::new();
        let scope_start = function.body().span.start;
        if let Some(receiver) = function.receiver() {
            locals.insert(
                receiver.name.name.clone(),
                ProjectLocalBinding {
                    scope_start,
                    ty: Some(ProjectCompletionType {
                        ty: receiver.ty.clone(),
                        package,
                        file,
                        external: false,
                    }),
                },
            );
        }
        for param in function.params() {
            locals.insert(
                param.name.name.clone(),
                ProjectLocalBinding {
                    scope_start,
                    ty: Some(ProjectCompletionType {
                        ty: param.ty.clone(),
                        package,
                        file,
                        external: false,
                    }),
                },
            );
        }
        collect_project_completion_local_types(
            project,
            package,
            file,
            &function.body().statements,
            scope_start,
            limit,
            &mut locals,
        );
        if let Some(binding) = locals.get(name) {
            return Some(binding.ty.clone());
        }
    }

    project_completion_package_value_type(project, package, name, true).map(Some)
}

fn project_completion_package_value_type<'a>(
    project: &'a ast::Project,
    package: &'a ast::Package,
    name: &str,
    infer: bool,
) -> Option<ProjectCompletionType<'a>> {
    package.files.iter().find_map(|candidate_file| {
        let resolve = |names: &[ast::Ident], ty: Option<&ast::Type>, values: &[ast::Expr]| {
            let index = names.iter().position(|candidate| candidate.name == name)?;
            if let Some(ty) = ty {
                Some(ProjectCompletionType {
                    ty: ty.clone(),
                    package,
                    file: candidate_file,
                    external: false,
                })
            } else if infer {
                project_completion_type_from_known_expr(
                    project,
                    package,
                    candidate_file,
                    values.get(index)?,
                    &BTreeMap::new(),
                )
            } else {
                None
            }
        };
        candidate_file.decls.iter().find_map(|decl| match decl {
            ast::TopLevelDecl::Var(decl) => decl
                .specs
                .iter()
                .find_map(|spec| resolve(&spec.names, spec.ty.as_ref(), &spec.values)),
            ast::TopLevelDecl::Const(decl) => decl
                .specs
                .iter()
                .find_map(|spec| resolve(&spec.names, spec.ty.as_ref(), &spec.values)),
            _ => None,
        })
    })
}

fn project_completion_imported_value<'a>(
    project: &'a ast::Project,
    file: &'a ast::File,
    alias: &ast::Ident,
    name: &ast::Ident,
) -> Option<ProjectCompletionType<'a>> {
    if !completion_name_is_exported(&name.name) {
        return None;
    }
    let package = completion_imported_package(project, file, &alias.name)?;
    let value = project_completion_package_value_type(project, package, &name.name, false)?;
    Some(ProjectCompletionType {
        external: true,
        ..value
    })
}

fn collect_project_completion_local_types<'a>(
    project: &'a ast::Project,
    package: &'a ast::Package,
    file: &'a ast::File,
    statements: &[ast::Stmt],
    scope_start: usize,
    limit: usize,
    types: &mut ProjectLocalTypes<'a>,
) {
    for stmt in statements {
        if stmt_span(stmt).start > limit {
            break;
        }
        match stmt {
            ast::Stmt::Var(decl) => {
                for spec in &decl.specs {
                    if declaration_visible_at(spec.span, &spec.values, limit) {
                        collect_project_completion_bindings(
                            project,
                            package,
                            file,
                            scope_start,
                            ProjectBindingDeclaration {
                                names: &spec.names,
                                ty: spec.ty.as_ref(),
                                values: &spec.values,
                                reuse_existing: false,
                            },
                            types,
                        );
                    }
                }
            }
            ast::Stmt::Const(decl) => {
                for spec in &decl.specs {
                    if declaration_visible_at(spec.span, &spec.values, limit) {
                        collect_project_completion_bindings(
                            project,
                            package,
                            file,
                            scope_start,
                            ProjectBindingDeclaration {
                                names: &spec.names,
                                ty: spec.ty.as_ref(),
                                values: &spec.values,
                                reuse_existing: false,
                            },
                            types,
                        );
                    }
                }
            }
            ast::Stmt::ShortVar(decl) if declaration_visible_at(decl.span, &decl.values, limit) => {
                collect_project_completion_bindings(
                    project,
                    package,
                    file,
                    scope_start,
                    ProjectBindingDeclaration {
                        names: &decl.names,
                        ty: None,
                        values: &decl.values,
                        reuse_existing: true,
                    },
                    types,
                );
            }
            ast::Stmt::Block(block) if limit <= block.span.end => {
                collect_project_completion_local_types(
                    project,
                    package,
                    file,
                    &block.statements,
                    block.span.start,
                    limit,
                    types,
                )
            }
            ast::Stmt::Defer(stmt) if limit <= stmt.block.span.end => {
                collect_project_completion_local_types(
                    project,
                    package,
                    file,
                    &stmt.block.statements,
                    stmt.block.span.start,
                    limit,
                    types,
                )
            }
            ast::Stmt::If(stmt) if limit <= stmt.span.end => {
                if let Some(init) = &stmt.init {
                    collect_project_completion_local_types(
                        project,
                        package,
                        file,
                        std::slice::from_ref(init),
                        stmt.span.start,
                        limit,
                        types,
                    );
                }
                if limit <= stmt.then_block.span.end {
                    collect_project_completion_local_types(
                        project,
                        package,
                        file,
                        &stmt.then_block.statements,
                        stmt.then_block.span.start,
                        limit,
                        types,
                    );
                } else if let Some(other) = &stmt.else_branch {
                    collect_project_completion_local_types(
                        project,
                        package,
                        file,
                        std::slice::from_ref(other),
                        stmt_span(other).start,
                        limit,
                        types,
                    );
                }
            }
            ast::Stmt::For(stmt) if limit <= stmt.span.end => {
                if let ast::ForKind::ThreeClause {
                    init: Some(init), ..
                } = &stmt.kind
                {
                    collect_project_completion_local_types(
                        project,
                        package,
                        file,
                        std::slice::from_ref(init),
                        stmt.span.start,
                        limit,
                        types,
                    );
                }
                collect_project_completion_local_types(
                    project,
                    package,
                    file,
                    &stmt.body.statements,
                    stmt.body.span.start,
                    limit,
                    types,
                );
            }
            _ => {}
        }
    }
}

fn declaration_visible_at(span: Span, values: &[ast::Expr], offset: usize) -> bool {
    span.start <= offset
        && (span.end <= offset
            || values
                .first()
                .is_none_or(|value| offset < value.span().start))
}

struct ProjectBindingDeclaration<'a> {
    names: &'a [ast::Ident],
    ty: Option<&'a ast::Type>,
    values: &'a [ast::Expr],
    reuse_existing: bool,
}

fn collect_project_completion_bindings<'a>(
    project: &'a ast::Project,
    package: &'a ast::Package,
    file: &'a ast::File,
    scope_start: usize,
    declaration: ProjectBindingDeclaration<'_>,
    types: &mut ProjectLocalTypes<'a>,
) {
    let bindings = declaration
        .names
        .iter()
        .enumerate()
        .filter(|(_, name)| {
            name.name != "_"
                && !(declaration.reuse_existing
                    && types
                        .get(&name.name)
                        .is_some_and(|binding| binding.scope_start == scope_start))
        })
        .map(|(index, name)| {
            let ty = declaration
                .ty
                .map(|ty| ProjectCompletionType {
                    ty: ty.clone(),
                    package,
                    file,
                    external: false,
                })
                .or_else(|| {
                    project_completion_type_from_known_expr(
                        project,
                        package,
                        file,
                        declaration.values.get(index)?,
                        types,
                    )
                });
            (name.name.clone(), ProjectLocalBinding { scope_start, ty })
        })
        .collect::<Vec<_>>();
    types.extend(bindings);
}

fn project_completion_type_from_known_expr<'a>(
    project: &'a ast::Project,
    package: &'a ast::Package,
    file: &'a ast::File,
    expr: &ast::Expr,
    types: &ProjectLocalTypes<'a>,
) -> Option<ProjectCompletionType<'a>> {
    resolve_project_completion_expr_type_with(
        project,
        package,
        file,
        ProjectExprLookup::Initializer(types),
        expr,
    )
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
    lookup: ProjectExprLookup<'a, '_>,
    callee: &ast::Expr,
) -> Option<ProjectCompletionType<'a>> {
    let method_result = match callee {
        ast::Expr::Selector { base, field, .. } => {
            project_completion_method_result(project, package, file, lookup, base, field)
        }
        ast::Expr::Name(path)
            if path.segments.len() >= 2
                && lookup
                    .value(project, package, file, &path.segments[0])
                    .is_some() =>
        {
            let (method, receiver) = path.segments.split_last()?;
            let base = ast::Expr::Name(ast::Path {
                segments: receiver.to_vec(),
                span: path.span,
            });
            project_completion_method_result(project, package, file, lookup, &base, method)
        }
        _ => None,
    };
    if method_result.is_some() {
        return method_result;
    }
    if let Some(mut target) =
        resolve_project_completion_expr_type_with(project, package, file, lookup, callee)
    {
        let ast::Type::Func { signature, .. } = &target.ty else {
            return None;
        };
        target.ty = signature.results.first()?.clone();
        return Some(target);
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
        [name] if lookup.value(project, package, file, name).is_none() => {
            let (function, declaring_file) = project_completion_function(package, &name.name)?;
            Some(ProjectCompletionType {
                ty: completion_function_result_type(function, type_arguments)?,
                package,
                file: declaring_file,
                external: false,
            })
        }
        [alias, name]
            if completion_name_is_exported(&name.name)
                && lookup.value(project, package, file, alias).is_none() =>
        {
            let imported = completion_imported_package(project, file, &alias.name)?;
            let (function, declaring_file) = project_completion_function(imported, &name.name)?;
            Some(ProjectCompletionType {
                ty: completion_function_result_type(function, type_arguments)?,
                package: imported,
                file: declaring_file,
                external: true,
            })
        }
        _ => None,
    }
}

fn project_completion_function<'a>(
    package: &'a ast::Package,
    name: &str,
) -> Option<(&'a ast::FuncDecl, &'a ast::File)> {
    package.files.iter().find_map(|file| {
        file.decls.iter().find_map(|decl| match decl {
            ast::TopLevelDecl::Func(function)
                if function.receiver.is_none() && function.name.name == name =>
            {
                Some((function, file))
            }
            _ => None,
        })
    })
}

fn project_completion_method_result<'a>(
    project: &'a ast::Project,
    package: &'a ast::Package,
    file: &'a ast::File,
    lookup: ProjectExprLookup<'a, '_>,
    base: &ast::Expr,
    method_name: &ast::Ident,
) -> Option<ProjectCompletionType<'a>> {
    let receiver = resolve_project_completion_expr_type_with(project, package, file, lookup, base)?;
    let addressable =
        project_completion_expr_is_addressable_with(project, package, file, lookup, base);
    project_completion_method_result_for_receiver(
        project,
        receiver.clone(),
        method_name,
        addressable,
        false,
    )
    .or_else(|| {
        let receiver = project_completion_implicit_dereference_type(project, receiver, 0)?;
        project_completion_method_result_for_receiver(
            project,
            receiver,
            method_name,
            addressable,
            true,
        )
    })
}

fn project_completion_method_result_for_receiver<'a>(
    project: &'a ast::Project,
    receiver: ProjectCompletionType<'a>,
    method_name: &ast::Ident,
    addressable: bool,
    exact_only: bool,
) -> Option<ProjectCompletionType<'a>> {
    let (path, pointer_depth, arguments) = completion_receiver_application(&receiver.ty, 0)?;
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
            (completion_exact_named_receiver(&receiver.ty).is_some_and(
                |(candidate_name, receiver_pointer)| {
                    candidate_name == receiver_name
                        && if exact_only {
                            receiver_pointer == pointer_depth
                        } else {
                            completion_receiver_matches(
                                pointer_depth,
                                receiver_pointer,
                                addressable,
                            )
                        }
                },
            ) && method.name.name == method_name.name)
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
    pointer_depth: usize,
) -> Option<(&ast::Path, usize, &[ast::Type])> {
    match ty {
        ast::Type::Named(path) => Some((path, pointer_depth, &[])),
        ast::Type::Apply {
            base, arguments, ..
        } => Some((base, pointer_depth, arguments)),
        ast::Type::Pointer(inner, _) => completion_receiver_application(inner, pointer_depth + 1),
        _ => None,
    }
}

fn substitute_completion_method_receiver(
    result: ast::Type,
    declared_receiver: &ast::Type,
    arguments: &[ast::Type],
) -> Option<ast::Type> {
    let (_, _, parameters) = completion_receiver_application(declared_receiver, 0)?;
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
    pointer_depth: usize,
) -> Option<(Vec<ast::Field>, &'a ast::Package, &'a ast::File, bool)> {
    if depth > 16 || pointer_depth > 1 {
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
            pointer_depth + 1,
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
                    pointer_depth,
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
                    pointer_depth,
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
                    pointer_depth,
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
                    pointer_depth,
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

pub(crate) fn completion_imported_package<'a>(
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
            let fields = resolve_completion_struct_fields_for_type(package, &base_ty, 0, 0)?;
            fields
                .into_iter()
                .find(|candidate| candidate.name.name == field.name)
                .map(|field| field.ty)
        }
        ast::Expr::Cast { ty, .. } | ast::Expr::Composite { ty, .. } => Some(ty.clone()),
        ast::Expr::New { ty, span } => Some(ast::Type::Pointer(Box::new(ty.clone()), *span)),
        ast::Expr::Unary {
            op: ast::UnaryOp::AddrOf,
            expr,
            span,
        } => Some(ast::Type::Pointer(
            Box::new(resolve_completion_expr_type(package, function, expr)?),
            *span,
        )),
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
        ast::Expr::Unary {
            op: ast::UnaryOp::AddrOf,
            expr,
            span,
        } => {
            let ty = completion_type_from_known_expr(package, expr, types)?;
            (completion_pointer_depth(&ty) < MAX_COMPLETION_INFERRED_POINTER_DEPTH)
                .then(|| ast::Type::Pointer(Box::new(ty), *span))
        }
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
    pointer_depth: usize,
) -> Option<Vec<ast::Field>> {
    if depth > 16 || pointer_depth > 1 {
        return None;
    }
    match ty {
        ast::Type::Struct { fields, .. } => Some(fields.clone()),
        ast::Type::Pointer(inner, _) => {
            resolve_completion_struct_fields_for_type(package, inner, depth + 1, pointer_depth + 1)
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
                        resolve_completion_struct_fields_for_type(
                            package,
                            &spec.ty,
                            depth + 1,
                            pointer_depth,
                        )
                    })
            }),
        ast::Type::Apply {
            base, arguments, ..
        } if base.segments.len() == 1 => {
            let (spec, _) = completion_type_spec(package, &base.segments[0].name)?;
            let ty = substitute_completion_type_arguments(spec, arguments)?;
            resolve_completion_struct_fields_for_type(package, &ty, depth + 1, pointer_depth)
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
        ast::Stmt::InlineAssembly(statement) => statement.span,
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
    fn asm_header_selector_completion_visits_ordinary_input_expression() {
        let text =
            "package main\nfunc main(){asm(r1<-object.__ond_completion_member){ADD r1,r1,r0,0}}\n";
        let file = compiler::parse_source_file(compiler::source::FileId(0), text).unwrap();
        let compiler::ir::ast::TopLevelDecl::Func(function) = &file.decls[0] else {
            panic!("function");
        };
        assert!(
            super::find_completion_selector_in_stmt(
                &function.body.statements[0],
                "__ond_completion_member",
                None
            )
            .is_some()
        );
    }

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
    fn repairs_member_at_utf16_position_after_non_bmp_comment() {
        let prefix = "    /* 𐐀𐐀𐐀 .x */ value.";
        let source = format!("{prefix}member");
        let (repaired, marker) = super::completion_text_with_placeholder(
            &source,
            0,
            prefix.encode_utf16().count() as u32,
        )
        .unwrap();
        assert_eq!(repaired, format!("{prefix}{marker}()"));
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
