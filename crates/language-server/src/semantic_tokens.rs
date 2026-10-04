use std::collections::{BTreeMap, BTreeSet, HashMap};

use compiler::ir::ast;
use compiler::source::Span;

use crate::completion::{BuiltinInfo, builtin_info};
use crate::hover_reference::{ResolvedTarget, resolve_reference};

const NAMESPACE_TOKEN_TYPE: u32 = 0;
const TYPE_TOKEN_TYPE: u32 = 1;
const FUNCTION_TOKEN_TYPE: u32 = 2;
const PROPERTY_TOKEN_TYPE: u32 = 3;
const VARIABLE_TOKEN_TYPE: u32 = 4;
const RECEIVER_TOKEN_TYPE: u32 = 5;
const METHOD_TOKEN_TYPE: u32 = 6;
const PARAMETER_TOKEN_TYPE: u32 = 7;
const TYPE_PARAMETER_TOKEN_TYPE: u32 = 8;
const READONLY_TOKEN_MODIFIER: u32 = 1;
const DECLARATION_TOKEN_MODIFIER: u32 = 1 << 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Identifier,
    String,
    Dot,
    Other,
}

#[derive(Debug)]
struct Token<'a> {
    kind: TokenKind,
    text: &'a str,
    start: usize,
    end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SemanticToken {
    line: u32,
    start: u32,
    length: u32,
    token_type: u32,
    token_modifiers: u32,
}

pub fn package_semantic_tokens(text: &str) -> Vec<u32> {
    semantic_tokens(text, None, None)
}

pub fn project_semantic_tokens(
    text: &str,
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
) -> Vec<u32> {
    semantic_tokens(text, Some((project, package, file)), None)
}

pub fn indexed_semantic_tokens(text: &str, project: &ast::Project) -> Vec<u32> {
    let styles = indexed_styles(project);
    semantic_tokens(text, None, Some(&styles))
}

fn semantic_tokens(
    text: &str,
    context: Option<(&ast::Project, &ast::Package, &ast::File)>,
    indexed: Option<&HashMap<String, Option<SemanticStyle>>>,
) -> Vec<u32> {
    let tokens = lex(text);
    let mut package_names = BTreeSet::new();
    let mut highlighted = BTreeMap::new();

    for window in tokens.windows(2) {
        if window[0].kind == TokenKind::Identifier
            && window[0].text == "package"
            && window[1].kind == TokenKind::Identifier
        {
            highlighted.insert(window[1].start, (SemanticStyle::Namespace, false));
        }
        if window[0].kind == TokenKind::Identifier
            && window[0].text == "import"
            && window[1].kind == TokenKind::String
        {
            if let Some(name) = imported_package_name(window[1].text) {
                package_names.insert(name);
            }
        }
    }

    for window in tokens.windows(2) {
        if window[0].kind == TokenKind::Identifier
            && package_names.contains(window[0].text)
            && window[1].kind == TokenKind::Dot
        {
            highlighted.insert(window[0].start, (SemanticStyle::Namespace, false));
        }
    }

    if let Some((project, package, file)) = context {
        let declarations = declaration_styles(project);
        for token in &tokens {
            if token.kind != TokenKind::Identifier {
                continue;
            }
            let token_span = Span::new(file.file_id, token.start, token.end);
            let style = declarations
                .get(&token_span)
                .copied()
                .map(|style| (style, true))
                .or_else(|| {
                    let resolved = resolve_reference(project, package, file, token.start)?;
                    if resolved.reference != token_span {
                        return None;
                    }
                    match resolved.target {
                        ResolvedTarget::Declaration(target) => declarations
                            .get(&target)
                            .copied()
                            .map(|style| (style, false)),
                        ResolvedTarget::Package(_) => Some((SemanticStyle::Namespace, false)),
                    }
                });
            if let Some(style) = style {
                highlighted.insert(token.start, style);
            }
        }
    } else if let Some(indexed) = indexed {
        for token in &tokens {
            if token.kind == TokenKind::Identifier
                && let Some(Some(style)) = indexed.get(token.text)
            {
                highlighted.insert(token.start, (*style, false));
            }
        }
    }

    let semantic_tokens = tokens
        .iter()
        .filter_map(|token| {
            let (style, declaration) = highlighted.get(&token.start)?;
            let (line, start) = lsp_position(text, token.start);
            Some(SemanticToken {
                line,
                start,
                length: text[token.start..token.end]
                    .encode_utf16()
                    .count()
                    .try_into()
                    .unwrap_or(u32::MAX),
                token_type: style.token_type(),
                token_modifiers: style.token_modifiers()
                    | if *declaration {
                        DECLARATION_TOKEN_MODIFIER
                    } else {
                        0
                    },
            })
        })
        .collect::<Vec<_>>();

    encode_semantic_tokens(&semantic_tokens)
}

pub(crate) fn builtin_call_at(text: &str, offset: usize) -> Option<(BuiltinInfo, usize, usize)> {
    let tokens = lex(text);
    tokens.windows(2).find_map(|window| {
        let name = &window[0];
        let open = &window[1];
        if name.kind == TokenKind::Identifier
            && name.start <= offset
            && offset < name.end
            && open.kind == TokenKind::Other
            && open.text == "("
        {
            Some((builtin_info(name.text)?, name.start, name.end))
        } else {
            None
        }
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SemanticStyle {
    Namespace,
    Type,
    TypeParameter,
    Function,
    Method,
    Parameter,
    Variable,
    Constant,
    Field,
    Receiver,
}

impl SemanticStyle {
    fn token_type(self) -> u32 {
        match self {
            Self::Namespace => NAMESPACE_TOKEN_TYPE,
            Self::Type => TYPE_TOKEN_TYPE,
            Self::TypeParameter => TYPE_PARAMETER_TOKEN_TYPE,
            Self::Function => FUNCTION_TOKEN_TYPE,
            Self::Method => METHOD_TOKEN_TYPE,
            Self::Parameter => PARAMETER_TOKEN_TYPE,
            Self::Variable => VARIABLE_TOKEN_TYPE,
            Self::Field => PROPERTY_TOKEN_TYPE,
            Self::Constant => VARIABLE_TOKEN_TYPE,
            Self::Receiver => RECEIVER_TOKEN_TYPE,
        }
    }

    fn token_modifiers(self) -> u32 {
        match self {
            Self::Constant => READONLY_TOKEN_MODIFIER,
            _ => 0,
        }
    }
}

fn declaration_styles(project: &ast::Project) -> HashMap<Span, SemanticStyle> {
    let mut declarations = HashMap::new();
    for package in &project.packages {
        for file in &package.files {
            for decl in &file.decls {
                collect_top_level_decl(decl, &mut declarations);
            }
        }
    }
    declarations
}

fn indexed_styles(project: &ast::Project) -> HashMap<String, Option<SemanticStyle>> {
    let mut styles = HashMap::new();
    for package in &project.packages {
        for file in &package.files {
            for decl in &file.decls {
                collect_indexed_top_level_decl(decl, &mut styles);
            }
        }
    }
    styles
}

fn collect_indexed_top_level_decl(
    decl: &ast::TopLevelDecl,
    styles: &mut HashMap<String, Option<SemanticStyle>>,
) {
    match decl {
        ast::TopLevelDecl::Const(decl) => collect_indexed_const_decl(decl, styles),
        ast::TopLevelDecl::Var(decl) => {
            for spec in &decl.specs {
                if let Some(ty) = &spec.ty {
                    collect_indexed_type(ty, styles);
                }
            }
        }
        ast::TopLevelDecl::Type(decl) => {
            for spec in &decl.specs {
                insert_indexed_style(styles, &spec.name.name, SemanticStyle::Type);
                collect_indexed_type(&spec.ty, styles);
            }
        }
        ast::TopLevelDecl::Func(decl) => {
            insert_indexed_style(styles, &decl.name.name, SemanticStyle::Function);
            if let Some(receiver) = &decl.receiver {
                insert_indexed_style(styles, &receiver.name.name, SemanticStyle::Receiver);
                collect_indexed_type(&receiver.ty, styles);
            }
            for param in &decl.signature.params {
                collect_indexed_type(&param.ty, styles);
            }
            for result in &decl.signature.results {
                collect_indexed_type(result, styles);
            }
            collect_indexed_block(&decl.body, styles);
        }
        ast::TopLevelDecl::Operator(_) => {}
    }
}

fn collect_indexed_const_decl(
    decl: &ast::ConstDecl,
    styles: &mut HashMap<String, Option<SemanticStyle>>,
) {
    for spec in &decl.specs {
        for name in &spec.names {
            insert_indexed_style(styles, &name.name, SemanticStyle::Constant);
        }
        if let Some(ty) = &spec.ty {
            collect_indexed_type(ty, styles);
        }
    }
}

fn collect_indexed_type(ty: &ast::Type, styles: &mut HashMap<String, Option<SemanticStyle>>) {
    match ty {
        ast::Type::Named(_) => {}
        ast::Type::Apply { arguments, .. } => {
            for argument in arguments {
                collect_indexed_type(argument, styles);
            }
        }
        ast::Type::Resolved(_, _) => {}
        ast::Type::Pointer(inner, _) => collect_indexed_type(inner, styles),
        ast::Type::Array { element, .. } => collect_indexed_type(element, styles),
        ast::Type::Struct { fields, .. } => {
            for field in fields {
                insert_indexed_style(styles, &field.name.name, SemanticStyle::Field);
                collect_indexed_type(&field.ty, styles);
            }
        }
        ast::Type::Interface { methods, .. } => {
            for method in methods {
                insert_indexed_style(styles, &method.name.name, SemanticStyle::Function);
                for parameter in &method.signature.params {
                    collect_indexed_type(&parameter.ty, styles);
                }
                for result in &method.signature.results {
                    collect_indexed_type(result, styles);
                }
            }
        }
        ast::Type::Func { signature, .. } => {
            for param in &signature.params {
                collect_indexed_type(&param.ty, styles);
            }
            for result in &signature.results {
                collect_indexed_type(result, styles);
            }
        }
    }
}

fn collect_indexed_block(block: &ast::Block, styles: &mut HashMap<String, Option<SemanticStyle>>) {
    for stmt in &block.statements {
        match stmt {
            ast::Stmt::Const(decl) => collect_indexed_const_decl(decl, styles),
            ast::Stmt::Var(decl) => {
                for spec in &decl.specs {
                    if let Some(ty) = &spec.ty {
                        collect_indexed_type(ty, styles);
                    }
                }
            }
            ast::Stmt::Block(block) => collect_indexed_block(block, styles),
            ast::Stmt::Defer(stmt) => collect_indexed_block(&stmt.block, styles),
            ast::Stmt::If(stmt) => {
                if let Some(init) = stmt.init.as_deref() {
                    collect_indexed_stmt(init, styles);
                }
                collect_indexed_block(&stmt.then_block, styles);
                if let Some(other) = stmt.else_branch.as_deref() {
                    collect_indexed_stmt(other, styles);
                }
            }
            ast::Stmt::For(stmt) => {
                if let ast::ForKind::ThreeClause { init, post, .. } = &stmt.kind {
                    if let Some(init) = init.as_deref() {
                        collect_indexed_stmt(init, styles);
                    }
                    if let Some(post) = post.as_deref() {
                        collect_indexed_stmt(post, styles);
                    }
                }
                collect_indexed_block(&stmt.body, styles);
            }
            _ => {}
        }
    }
}

fn collect_indexed_stmt(stmt: &ast::Stmt, styles: &mut HashMap<String, Option<SemanticStyle>>) {
    match stmt {
        ast::Stmt::Const(decl) => collect_indexed_const_decl(decl, styles),
        ast::Stmt::Var(decl) => {
            for spec in &decl.specs {
                if let Some(ty) = &spec.ty {
                    collect_indexed_type(ty, styles);
                }
            }
        }
        ast::Stmt::Block(block) => collect_indexed_block(block, styles),
        ast::Stmt::Defer(defer) => collect_indexed_block(&defer.block, styles),
        ast::Stmt::If(stmt) => {
            collect_indexed_block(&stmt.then_block, styles);
            if let Some(other) = stmt.else_branch.as_deref() {
                collect_indexed_stmt(other, styles);
            }
        }
        ast::Stmt::For(stmt) => collect_indexed_block(&stmt.body, styles),
        _ => {}
    }
}

fn insert_indexed_style(
    styles: &mut HashMap<String, Option<SemanticStyle>>,
    name: &str,
    style: SemanticStyle,
) {
    styles
        .entry(name.to_string())
        .and_modify(|current| {
            if *current != Some(style) {
                *current = None;
            }
        })
        .or_insert(Some(style));
}

fn collect_top_level_decl(
    decl: &ast::TopLevelDecl,
    declarations: &mut HashMap<Span, SemanticStyle>,
) {
    match decl {
        ast::TopLevelDecl::Const(decl) => collect_const_decl(decl, declarations),
        ast::TopLevelDecl::Var(decl) => {
            for spec in &decl.specs {
                for name in &spec.names {
                    declarations.insert(name.span, SemanticStyle::Variable);
                }
                if let Some(ty) = &spec.ty {
                    collect_type(ty, declarations, &BTreeSet::new());
                }
            }
        }
        ast::TopLevelDecl::Type(decl) => {
            for spec in &decl.specs {
                declarations.insert(spec.name.span, SemanticStyle::Type);
                let type_parameters =
                    collect_type_parameter_declarations(&spec.type_params, declarations);
                collect_type(&spec.ty, declarations, &type_parameters);
            }
        }
        ast::TopLevelDecl::Func(decl) => {
            declarations.insert(
                decl.name.span,
                if decl.receiver.is_some() {
                    SemanticStyle::Method
                } else {
                    SemanticStyle::Function
                },
            );
            let mut type_parameters =
                collect_type_parameter_declarations(&decl.type_params, declarations);
            if let Some(receiver) = &decl.receiver {
                collect_receiver_type_parameters(&receiver.ty, declarations, &mut type_parameters);
                declarations.insert(receiver.name.span, SemanticStyle::Receiver);
                collect_type(&receiver.ty, declarations, &type_parameters);
            }
            for param in &decl.signature.params {
                declarations.insert(param.name.span, SemanticStyle::Parameter);
                collect_type(&param.ty, declarations, &type_parameters);
            }
            for result in &decl.signature.results {
                collect_type(result, declarations, &type_parameters);
            }
            collect_block(&decl.body, declarations, &type_parameters);
        }
        ast::TopLevelDecl::Operator(decl) => {
            let type_parameters =
                collect_type_parameter_declarations(&decl.type_params, declarations);
            for param in &decl.signature.params {
                declarations.insert(param.name.span, SemanticStyle::Parameter);
                collect_type(&param.ty, declarations, &type_parameters);
            }
            for result in &decl.signature.results {
                collect_type(result, declarations, &type_parameters);
            }
            collect_block(&decl.body, declarations, &type_parameters);
        }
    }
}

fn collect_receiver_type_parameters(
    receiver: &ast::Type,
    declarations: &mut HashMap<Span, SemanticStyle>,
    type_parameters: &mut BTreeSet<String>,
) {
    let receiver = match receiver {
        ast::Type::Pointer(inner, _) => inner.as_ref(),
        receiver => receiver,
    };
    let ast::Type::Apply { arguments, .. } = receiver else {
        return;
    };
    for argument in arguments {
        let ast::Type::Named(path) = argument else {
            continue;
        };
        let [name] = path.segments.as_slice() else {
            continue;
        };
        declarations.insert(name.span, SemanticStyle::TypeParameter);
        type_parameters.insert(name.name.clone());
    }
}

fn collect_type_parameter_declarations(
    parameters: &[ast::Ident],
    declarations: &mut HashMap<Span, SemanticStyle>,
) -> BTreeSet<String> {
    parameters
        .iter()
        .map(|parameter| {
            declarations.insert(parameter.span, SemanticStyle::TypeParameter);
            parameter.name.clone()
        })
        .collect()
}

fn collect_const_decl(decl: &ast::ConstDecl, declarations: &mut HashMap<Span, SemanticStyle>) {
    for spec in &decl.specs {
        for name in &spec.names {
            declarations.insert(name.span, SemanticStyle::Constant);
        }
        if let Some(ty) = &spec.ty {
            collect_type(ty, declarations, &BTreeSet::new());
        }
    }
}

fn collect_type(
    ty: &ast::Type,
    declarations: &mut HashMap<Span, SemanticStyle>,
    type_parameters: &BTreeSet<String>,
) {
    match ty {
        ast::Type::Named(path) => {
            for segment in &path.segments {
                if type_parameters.contains(&segment.name) {
                    declarations.insert(segment.span, SemanticStyle::TypeParameter);
                }
            }
        }
        ast::Type::Apply {
            base, arguments, ..
        } => {
            for segment in &base.segments {
                if type_parameters.contains(&segment.name) {
                    declarations.insert(segment.span, SemanticStyle::TypeParameter);
                }
            }
            for argument in arguments {
                collect_type(argument, declarations, type_parameters);
            }
        }
        ast::Type::Resolved(_, _) => {}
        ast::Type::Pointer(inner, _) => collect_type(inner, declarations, type_parameters),
        ast::Type::Array { element, .. } => collect_type(element, declarations, type_parameters),
        ast::Type::Struct { fields, .. } => {
            for field in fields {
                declarations.insert(field.name.span, SemanticStyle::Field);
                collect_type(&field.ty, declarations, type_parameters);
            }
        }
        ast::Type::Interface { methods, .. } => {
            for method in methods {
                declarations.insert(method.name.span, SemanticStyle::Method);
                for parameter in &method.signature.params {
                    if let Some(name) = &parameter.name {
                        declarations.insert(name.span, SemanticStyle::Parameter);
                    }
                    collect_type(&parameter.ty, declarations, type_parameters);
                }
                for result in &method.signature.results {
                    collect_type(result, declarations, type_parameters);
                }
            }
        }
        ast::Type::Func { signature, .. } => {
            for param in &signature.params {
                if let Some(name) = &param.name {
                    declarations.insert(name.span, SemanticStyle::Parameter);
                }
                collect_type(&param.ty, declarations, type_parameters);
            }
            for result in &signature.results {
                collect_type(result, declarations, type_parameters);
            }
        }
    }
}

fn collect_block(
    block: &ast::Block,
    declarations: &mut HashMap<Span, SemanticStyle>,
    type_parameters: &BTreeSet<String>,
) {
    for stmt in &block.statements {
        collect_stmt(stmt, declarations, type_parameters);
    }
}

fn collect_stmt(
    stmt: &ast::Stmt,
    declarations: &mut HashMap<Span, SemanticStyle>,
    type_parameters: &BTreeSet<String>,
) {
    match stmt {
        ast::Stmt::Const(decl) => collect_const_decl(decl, declarations),
        ast::Stmt::Var(decl) => {
            for spec in &decl.specs {
                for name in &spec.names {
                    declarations.insert(name.span, SemanticStyle::Variable);
                }
                if let Some(ty) = &spec.ty {
                    collect_type(ty, declarations, type_parameters);
                }
            }
        }
        ast::Stmt::ShortVar(decl) => {
            for name in &decl.names {
                declarations.insert(name.span, SemanticStyle::Variable);
            }
        }
        ast::Stmt::Block(block) => collect_block(block, declarations, type_parameters),
        ast::Stmt::Defer(stmt) => collect_block(&stmt.block, declarations, type_parameters),
        ast::Stmt::If(stmt) => {
            if let Some(init) = stmt.init.as_deref() {
                collect_stmt(init, declarations, type_parameters);
            }
            collect_block(&stmt.then_block, declarations, type_parameters);
            if let Some(other) = stmt.else_branch.as_deref() {
                collect_stmt(other, declarations, type_parameters);
            }
        }
        ast::Stmt::For(stmt) => {
            if let ast::ForKind::ThreeClause { init, post, .. } = &stmt.kind {
                if let Some(init) = init.as_deref() {
                    collect_stmt(init, declarations, type_parameters);
                }
                if let Some(post) = post.as_deref() {
                    collect_stmt(post, declarations, type_parameters);
                }
            }
            collect_block(&stmt.body, declarations, type_parameters);
        }
        ast::Stmt::Assign(_)
        | ast::Stmt::Expr(_)
        | ast::Stmt::Return(_)
        | ast::Stmt::Break(_)
        | ast::Stmt::Continue(_)
        | ast::Stmt::Trap { .. } => {}
    }
}

fn imported_package_name(literal: &str) -> Option<&str> {
    let path = literal.strip_prefix('"')?.strip_suffix('"')?;
    path.rsplit('/').find(|component| !component.is_empty())
}

fn encode_semantic_tokens(tokens: &[SemanticToken]) -> Vec<u32> {
    let mut data = Vec::with_capacity(tokens.len() * 5);
    let mut previous_line = 0;
    let mut previous_start = 0;

    for token in tokens {
        let delta_line = token.line - previous_line;
        let delta_start = if delta_line == 0 {
            token.start - previous_start
        } else {
            token.start
        };
        data.extend([
            delta_line,
            delta_start,
            token.length,
            token.token_type,
            token.token_modifiers,
        ]);
        previous_line = token.line;
        previous_start = token.start;
    }

    data
}

fn lsp_position(text: &str, offset: usize) -> (u32, u32) {
    let prefix = &text[..offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() as u32;
    let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
    let character = text[line_start..offset].encode_utf16().count() as u32;
    (line, character)
}

fn lex(text: &str) -> Vec<Token<'_>> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index += 2;
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index += 2;
                while index < bytes.len() {
                    if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                        index += 2;
                        break;
                    }
                    index += 1;
                }
            }
            b'"' => {
                let start = index;
                index += 1;
                while index < bytes.len() {
                    if bytes[index] == b'\\' {
                        index = (index + 2).min(bytes.len());
                    } else if bytes[index] == b'"' {
                        index += 1;
                        break;
                    } else {
                        index += 1;
                    }
                }
                tokens.push(Token {
                    kind: TokenKind::String,
                    text: &text[start..index],
                    start,
                    end: index,
                });
            }
            b'`' => {
                let start = index;
                index += 1;
                while index < bytes.len() && bytes[index] != b'`' {
                    index += 1;
                }
                index = (index + 1).min(bytes.len());
                tokens.push(Token {
                    kind: TokenKind::String,
                    text: &text[start..index],
                    start,
                    end: index,
                });
            }
            b'.' => {
                tokens.push(Token {
                    kind: TokenKind::Dot,
                    text: &text[index..index + 1],
                    start: index,
                    end: index + 1,
                });
                index += 1;
            }
            byte if byte.is_ascii_alphabetic() || byte == b'_' => {
                let start = index;
                index += 1;
                while index < bytes.len()
                    && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
                {
                    index += 1;
                }
                tokens.push(Token {
                    kind: TokenKind::Identifier,
                    text: &text[start..index],
                    start,
                    end: index,
                });
            }
            byte if byte.is_ascii_whitespace() => index += 1,
            _ => {
                let start = index;
                let width = text[index..].chars().next().map_or(1, char::len_utf8);
                index += width;
                tokens.push(Token {
                    kind: TokenKind::Other,
                    text: &text[start..index],
                    start,
                    end: index,
                });
            }
        }
    }

    tokens
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use compiler::ir::ast;
    use compiler::source::SourceDb;

    use super::{indexed_semantic_tokens, package_semantic_tokens, project_semantic_tokens};

    #[test]
    fn highlights_declared_and_imported_package_names() {
        let source = "package main\n\nimport \"shared\"\n\nfunc run() {\n    shared.Read()\n}\n";
        assert_eq!(
            package_semantic_tokens(source),
            vec![0, 8, 4, 0, 0, 5, 4, 6, 0, 0]
        );
    }

    #[test]
    fn does_not_highlight_struct_receivers_or_comments() {
        let source = "package main\nimport \"shared\"\nfunc run() {\n    value.Field\n    // shared.Read()\n}\n";
        assert_eq!(package_semantic_tokens(source), vec![0, 8, 4, 0, 0]);
    }

    #[test]
    fn uses_utf16_columns_after_unicode_text() {
        let source = "/* 日本語 */ package main\nimport \"dir/shared\"\nshared.Read()\n";
        assert_eq!(
            package_semantic_tokens(source),
            vec![0, 18, 4, 0, 0, 2, 0, 6, 0, 0]
        );
    }

    #[test]
    fn classifies_type_constant_function_and_field_declarations_and_references() {
        let source = "package main\ntype Item struct { Value: u32; }\nconst Limit: u32 = 1\nfunc Read(item: Item) -> u32 { return item.Value + Limit; }\nfunc main() { Read(Item{Value: Limit}); }\n";
        let (project, package, file) = single_file_project(source);
        let decoded = decode(
            source,
            &project_semantic_tokens(source, &project, &package, &file),
        );

        assert!(decoded.contains(&("Item", 1, 2)), "{decoded:?}");
        assert!(decoded.contains(&("Limit", 4, 3)), "{decoded:?}");
        assert!(decoded.contains(&("Read", 2, 2)), "{decoded:?}");
        assert!(decoded.contains(&("Read", 2, 0)), "{decoded:?}");
        assert!(decoded.contains(&("Value", 3, 2)), "{decoded:?}");
        assert!(decoded.contains(&("Value", 3, 0)), "{decoded:?}");
        assert!(
            decoded
                .iter()
                .filter(|(name, token_type, _)| name == &"Item" && *token_type == 1)
                .count()
                >= 3
        );
        assert!(
            decoded
                .iter()
                .filter(|(name, token_type, _)| name == &"Limit" && *token_type == 4)
                .count()
                >= 3
        );
        assert!(
            decoded
                .iter()
                .filter(|(name, token_type, _)| name == &"Read" && *token_type == 2)
                .count()
                >= 2
        );
        assert!(
            decoded
                .iter()
                .filter(|(name, token_type, _)| name == &"Value" && *token_type == 3)
                .count()
                >= 3
        );
    }

    #[test]
    fn classifies_method_receiver_declaration_and_references_separately() {
        let source = "package main\ntype Counter struct {}\nfunc (counter: *Counter) Reset() { counter.Reset() }\nfunc main() {}\n";
        let (project, package, file) = single_file_project(source);
        let decoded = decode(
            source,
            &project_semantic_tokens(source, &project, &package, &file),
        );
        assert_eq!(
            decoded
                .iter()
                .filter(|(name, token_type, _)| name == &"counter" && *token_type == 5)
                .count(),
            2,
            "{decoded:?}"
        );
        assert!(decoded.contains(&("Reset", 6, 2)), "{decoded:?}");
        assert!(decoded.contains(&("Reset", 6, 0)), "{decoded:?}");
    }

    #[test]
    fn classifies_generic_function_declaration_and_call_separately() {
        let source = "package main\nfunc NewList[T](value: T) -> T { return value }\nfunc main() { value := NewList[u32](1); _ = value }\n";
        let (project, package, file) = single_file_project(source);
        let decoded = decode(
            source,
            &project_semantic_tokens(source, &project, &package, &file),
        );

        assert!(decoded.contains(&("NewList", 2, 2)), "{decoded:?}");
        assert!(decoded.contains(&("NewList", 2, 0)), "{decoded:?}");
    }

    #[test]
    fn highlights_alloc_type_arguments_as_type_parameters() {
        let source =
            "package main\nfunc Allocate[T](count: u32) -> *T { return alloc[T](count) }\n";
        let (project, package, file) = single_file_project(source);
        let decoded = decode(
            source,
            &project_semantic_tokens(source, &project, &package, &file),
        );

        assert!(decoded.contains(&("T", 8, 2)), "{decoded:?}");
        assert!(decoded.contains(&("T", 8, 0)), "{decoded:?}");
    }

    #[test]
    fn distinguishes_methods_parameters_variables_and_type_parameters() {
        let source = "package main\ntype Box[T] struct { value: T }\nfunc (box: *Box[T]) Read(fallback: T) -> T { local := fallback; _ = box; return local }\noperator[T] [](box: *Box[T], index: u32) -> *T { _ = index; return &box.value }\n";
        let (project, package, file) = single_file_project(source);
        let decoded = decode(
            source,
            &project_semantic_tokens(source, &project, &package, &file),
        );

        assert!(decoded.contains(&("Read", 6, 2)), "{decoded:?}");
        assert!(decoded.contains(&("fallback", 7, 0)), "{decoded:?}");
        assert!(decoded.contains(&("index", 7, 0)), "{decoded:?}");
        assert!(decoded.contains(&("local", 4, 0)), "{decoded:?}");
        assert!(
            decoded
                .iter()
                .filter(|(name, token_type, _)| name == &"T" && *token_type == 8)
                .count()
                >= 7,
            "{decoded:?}"
        );
    }

    #[test]
    fn leaves_source_location_builtins_to_textmate_highlighting() {
        let source = "package main\nfunc main(){var file=thisFile();var line=thisLine()}\n";
        let (project, package, file) = single_file_project(source);
        let decoded = decode(
            source,
            &project_semantic_tokens(source, &project, &package, &file),
        );

        assert!(!decoded.iter().any(|(name, _, _)| name == &"thisFile"));
        assert!(!decoded.iter().any(|(name, _, _)| name == &"thisLine"));
    }

    #[test]
    fn classifies_imported_members_by_their_declarations() {
        let main = "package main\nimport \"shared\"\nfunc main() { shared.Run(); var value: shared.Item; value = shared.Limit; }\n";
        let shared = "package shared\nfunc Run() {}\nconst Limit: u32 = 1\ntype Item u32\n";
        let mut sources = SourceDb::default();
        let main_id = sources.add_file(PathBuf::from("main.ond"), main.to_string());
        let shared_id = sources.add_file(PathBuf::from("shared.ond"), shared.to_string());
        let main_file = compiler::parse_source_file(main_id, main).unwrap();
        let shared_file = compiler::parse_source_file(shared_id, shared).unwrap();
        let main_package = ast::Package {
            logical_path: ".".to_string(),
            files: vec![main_file.clone()],
        };
        let project = ast::Project {
            packages: vec![
                main_package.clone(),
                ast::Package {
                    logical_path: "shared".to_string(),
                    files: vec![shared_file],
                },
            ],
        };
        let decoded = decode(
            main,
            &project_semantic_tokens(main, &project, &main_package, &main_file),
        );

        assert!(decoded.contains(&("shared", 0, 0)), "{decoded:?}");
        assert!(decoded.contains(&("Run", 2, 0)), "{decoded:?}");
        assert!(decoded.contains(&("Item", 1, 0)), "{decoded:?}");
        assert!(decoded.contains(&("Limit", 4, 1)), "{decoded:?}");
    }

    #[test]
    fn retains_indexed_classifications_for_incomplete_source() {
        let valid = "package main\ntype Item struct { Value: u32; }\nconst Limit: u32 = 1\nfunc Read(item: Item) -> u32 { return item.Value + Limit; }\n";
        let incomplete = "package main\ntype Item struct { Value: u32; }\nconst Limit: u32 = 1\nfunc Read(item: Item) -> u32 { return item.Value + Limit\n";
        let (project, _, _) = single_file_project(valid);
        let decoded = decode(incomplete, &indexed_semantic_tokens(incomplete, &project));

        assert!(decoded.contains(&("Item", 1, 0)), "{decoded:?}");
        assert!(decoded.contains(&("Limit", 4, 1)), "{decoded:?}");
        assert!(decoded.contains(&("Read", 2, 0)), "{decoded:?}");
        assert!(decoded.contains(&("Value", 3, 0)), "{decoded:?}");
    }

    fn single_file_project(source: &str) -> (ast::Project, ast::Package, ast::File) {
        let mut sources = SourceDb::default();
        let file_id = sources.add_file(PathBuf::from("main.ond"), source.to_string());
        let file = compiler::parse_source_file(file_id, source).unwrap();
        let package = ast::Package {
            logical_path: ".".to_string(),
            files: vec![file.clone()],
        };
        let project = ast::Project {
            packages: vec![package.clone()],
        };
        (project, package, file)
    }

    fn decode<'a>(source: &'a str, data: &[u32]) -> Vec<(&'a str, u32, u32)> {
        let lines = source.lines().collect::<Vec<_>>();
        let mut line = 0;
        let mut start = 0;
        data.chunks_exact(5)
            .map(|token| {
                line += token[0];
                start = if token[0] == 0 {
                    start + token[1]
                } else {
                    token[1]
                };
                let text = &lines[line as usize][start as usize..(start + token[2]) as usize];
                (text, token[3], token[4])
            })
            .collect()
    }
}
