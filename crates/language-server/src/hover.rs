use compiler::ir::ast;
use compiler::source::{SourceDb, SourceFile, Span};
use serde_json::{Value, json};

use crate::completion::{completion_type_name, project_value_type_for_name};
use crate::hover_reference::{ResolvedTarget, resolve_local_value_reference, resolve_reference};
use crate::hover_reference::{resolve_field_reference, resolve_interface_method_reference};
use crate::protocol::{Position, lsp_offset};
use crate::semantic_tokens::builtin_call_at;

pub fn hover_for_file(file: &ast::File, source: &SourceFile, position: &Position) -> Option<Value> {
    let mut sources = SourceDb::default();
    sources.add_file(source.path().to_path_buf(), source.text().to_string());
    let package = ast::Package {
        logical_path: ".".to_string(),
        files: vec![file.clone()],
    };
    let project = ast::Project {
        packages: vec![package.clone()],
    };
    hover_for_project(&project, &package, file, &sources, position)
}

pub fn hover_for_project(
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
    sources: &SourceDb,
    position: &Position,
) -> Option<Value> {
    let source = sources.file(file.file_id);
    let offset = lsp_offset(source.text(), position).ok()?;
    if let Some(target) = find_target(file, source, offset) {
        if let Some(hover) = render_top_level_variable_hover(
            project,
            package,
            file,
            &target,
            source,
            target.name_span(),
            source,
        ) {
            return Some(hover);
        }
        return Some(render_hover(&target, source, target.name_span(), source));
    }

    if let Some(reference) = resolve_local_value_reference(file, offset) {
        let ty = reference.ty.or_else(|| {
            project_value_type_for_name(project, package, file, &reference.name, offset)
        });
        if let Some(ty) = ty {
            return Some(render_value_hover(
                &reference.name,
                &ty,
                reference.reference,
                source,
            ));
        }
    }

    if let Some(field) = resolve_field_reference(project, package, file, offset) {
        let declaration_file = project
            .packages
            .iter()
            .flat_map(|package| &package.files)
            .find(|file| file.file_id == field.declaration.file)?;
        let declaration_source = sources.file(field.declaration.file);
        let target = find_target(
            declaration_file,
            declaration_source,
            field.declaration.start,
        )?;
        let mut value = format!(
            "```ond\n{}: {}\n```",
            field.name,
            completion_type_name(&field.ty)
        );
        if let Some(comment) = documentation_for(&target, declaration_source) {
            value.push_str("\n\n");
            value.push_str(&comment);
        }
        return Some(json!({
            "contents": { "kind": "markdown", "value": value },
            "range": {
                "start": lsp_position(source, field.reference.start),
                "end": lsp_position(source, field.reference.end)
            }
        }));
    }

    if let Some(method) = resolve_interface_method_reference(project, package, file, offset) {
        let declaration_file = project
            .packages
            .iter()
            .flat_map(|package| &package.files)
            .find(|file| file.file_id == method.declaration.file)?;
        let declaration_source = sources.file(method.declaration.file);
        let target = find_target(
            declaration_file,
            declaration_source,
            method.declaration.start,
        )?;
        let parameters = method
            .signature
            .params
            .iter()
            .map(|parameter| {
                parameter.name.as_ref().map_or_else(
                    || completion_type_name(&parameter.ty),
                    |name| format!("{}: {}", name.name, completion_type_name(&parameter.ty)),
                )
            })
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
        let mut value = format!("```ond\n{}({parameters}){returns}\n```", method.name);
        if let Some(comment) = documentation_for(&target, declaration_source) {
            value.push_str("\n\n");
            value.push_str(&comment);
        }
        return Some(json!({
            "contents": { "kind": "markdown", "value": value },
            "range": {
                "start": lsp_position(source, method.reference.start),
                "end": lsp_position(source, method.reference.end)
            }
        }));
    }

    let Some(resolved) = resolve_reference(project, package, file, offset) else {
        let (builtin, start, end) = builtin_call_at(source.text(), offset)?;
        return Some(render_builtin_hover(
            builtin.signature,
            builtin.description,
            Span::new(file.file_id, start, end),
            source,
        ));
    };
    match resolved.target {
        ResolvedTarget::Declaration(declaration) => {
            let declaration_file = package
                .files
                .iter()
                .find(|candidate| candidate.file_id == declaration.file)
                .or_else(|| {
                    project
                        .packages
                        .iter()
                        .flat_map(|package| &package.files)
                        .find(|candidate| candidate.file_id == declaration.file)
                })?;
            let declaration_source = sources.file(declaration.file);
            let target = find_target(declaration_file, declaration_source, declaration.start)?;
            let declaration_package = project.packages.iter().find(|candidate| {
                candidate
                    .files
                    .iter()
                    .any(|file| file.file_id == declaration.file)
            })?;
            if let Some(hover) = render_top_level_variable_hover(
                project,
                declaration_package,
                declaration_file,
                &target,
                declaration_source,
                resolved.reference,
                source,
            ) {
                return Some(hover);
            }
            Some(render_hover(
                &target,
                declaration_source,
                resolved.reference,
                source,
            ))
        }
        ResolvedTarget::Package(package_index) => {
            let imported = project.packages.get(package_index)?;
            let (target, declaration_source) = package_hover_target(imported, sources)?;
            Some(render_hover(
                &target,
                declaration_source,
                resolved.reference,
                source,
            ))
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_top_level_variable_hover(
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
    target: &HoverTarget,
    declaration_source: &SourceFile,
    range: Span,
    range_source: &SourceFile,
) -> Option<Value> {
    if !matches!(target.kind, TargetKind::Var) || !target.signature.starts_with("var ") {
        return None;
    }
    if target
        .signature
        .split_once('=')
        .map_or(target.signature.as_str(), |(declaration, _)| declaration)
        .contains(':')
    {
        return None;
    }
    let name = declaration_source.slice(target.name_span());
    let ty = project_value_type_for_name(project, package, file, name, target.name_start)?;
    let mut value = format!("```ond\nvar {name}: {}\n```", completion_type_name(&ty));
    if let Some(comment) = documentation_for(target, declaration_source) {
        value.push_str("\n\n");
        value.push_str(&comment);
    }
    Some(json!({
        "contents": {
            "kind": "markdown",
            "value": value
        },
        "range": {
            "start": lsp_position(range_source, range.start),
            "end": lsp_position(range_source, range.end)
        }
    }))
}

fn render_value_hover(name: &str, ty: &ast::Type, range: Span, source: &SourceFile) -> Value {
    json!({
        "contents": {
            "kind": "markdown",
            "value": format!("```ond\n{name}: {}\n```", completion_type_name(ty))
        },
        "range": {
            "start": lsp_position(source, range.start),
            "end": lsp_position(source, range.end)
        }
    })
}

fn render_builtin_hover(
    signature: &str,
    description: &str,
    range: Span,
    source: &SourceFile,
) -> Value {
    json!({
        "contents": {
            "kind": "markdown",
            "value": format!("```ond\n{signature}\n```\n\n{description}")
        },
        "range": {
            "start": lsp_position(source, range.start),
            "end": lsp_position(source, range.end)
        }
    })
}

fn package_hover_target<'a>(
    package: &ast::Package,
    sources: &'a SourceDb,
) -> Option<(HoverTarget, &'a SourceFile)> {
    let mut fallback = None;
    for file in &package.files {
        let source = sources.file(file.file_id);
        let offset = package_name_start(file, source);
        let target = find_target(file, source, offset)?;
        if documentation_for(&target, source).is_some() {
            return Some((target, source));
        }
        if fallback.is_none() {
            fallback = Some((target, source));
        }
    }
    fallback
}

fn render_hover(
    target: &HoverTarget,
    declaration_source: &SourceFile,
    range: Span,
    range_source: &SourceFile,
) -> Value {
    let comment = documentation_for(target, declaration_source);
    let mut value = format!("```ond\n{}\n```", target.signature);
    if let Some(comment) = comment {
        value.push_str("\n\n");
        value.push_str(&comment);
    }

    let range_start = lsp_position(range_source, range.start);
    let range_end = lsp_position(range_source, range.end);
    json!({
        "contents": {
            "kind": "markdown",
            "value": value
        },
        "range": {
            "start": range_start,
            "end": range_end
        }
    })
}

#[derive(Debug, Clone, Copy)]
enum TargetKind {
    Package,
    Func,
    Var,
    Const,
    StructType,
    AliasType,
    Field,
}

#[derive(Debug)]
struct HoverTarget {
    kind: TargetKind,
    declaration_start: usize,
    declaration_end: usize,
    name_start: usize,
    name_end: usize,
    signature: String,
}

impl HoverTarget {
    fn name_span(&self) -> Span {
        Span::new(compiler::source::FileId(0), self.name_start, self.name_end)
    }
}

fn find_target(file: &ast::File, source: &SourceFile, offset: usize) -> Option<HoverTarget> {
    let package_name_start = package_name_start(file, source);
    let package_name_end = package_name_start + file.package.name.len();
    if contains(package_name_start, package_name_end, offset) {
        return Some(HoverTarget {
            kind: TargetKind::Package,
            declaration_start: file.package.span.start,
            declaration_end: file.package.span.end,
            name_start: package_name_start,
            name_end: package_name_end,
            signature: format!("package {}", file.package.name),
        });
    }

    for decl in &file.decls {
        match decl {
            ast::TopLevelDecl::Const(decl) => {
                for spec in &decl.specs {
                    for name in &spec.names {
                        if contains(name.span.start, name.span.end, offset) {
                            return Some(target_for_name(
                                TargetKind::Const,
                                spec_comment_start(decl.span.start, spec.span.start, source),
                                spec.span.end,
                                name.span.start,
                                name.span.end,
                                format_spec_signature("const", spec, source),
                            ));
                        }
                    }
                }
            }
            ast::TopLevelDecl::Var(decl) => {
                for spec in &decl.specs {
                    for name in &spec.names {
                        if contains(name.span.start, name.span.end, offset) {
                            return Some(target_for_name(
                                TargetKind::Var,
                                spec_comment_start(decl.span.start, spec.span.start, source),
                                spec.span.end,
                                name.span.start,
                                name.span.end,
                                format_spec_signature("var", spec, source),
                            ));
                        }
                    }
                }
            }
            ast::TopLevelDecl::Type(decl) => {
                for spec in &decl.specs {
                    if contains(spec.name.span.start, spec.name.span.end, offset) {
                        let kind = if matches!(spec.ty, ast::Type::Struct { .. }) {
                            TargetKind::StructType
                        } else {
                            TargetKind::AliasType
                        };
                        return Some(target_for_name(
                            kind,
                            spec_comment_start(decl.span.start, spec.span.start, source),
                            spec.span.end,
                            spec.name.span.start,
                            spec.name.span.end,
                            format!("type {} {}", spec.name.name, format_type(&spec.ty, source)),
                        ));
                    }

                    if let ast::Type::Struct { fields, .. } = &spec.ty {
                        for field in fields {
                            if contains(field.name.span.start, field.name.span.end, offset) {
                                return Some(target_for_name(
                                    TargetKind::Field,
                                    field.span.start,
                                    field.span.end,
                                    field.name.span.start,
                                    field.name.span.end,
                                    format!(
                                        "{}: {}",
                                        field.name.name,
                                        format_type(&field.ty, source)
                                    ),
                                ));
                            }
                        }
                    }
                    if let ast::Type::Interface { methods, .. } = &spec.ty {
                        for method in methods {
                            if contains(method.name.span.start, method.name.span.end, offset) {
                                return Some(target_for_name(
                                    TargetKind::Func,
                                    method.span.start,
                                    method.span.end,
                                    method.name.span.start,
                                    method.name.span.end,
                                    source.slice(method.span).trim().to_string(),
                                ));
                            }
                        }
                    }
                }
            }
            ast::TopLevelDecl::Func(decl) => {
                if let Some(receiver) = &decl.receiver
                    && contains(receiver.name.span.start, receiver.name.span.end, offset)
                {
                    return Some(target_for_name(
                        TargetKind::Var,
                        receiver.span.start,
                        receiver.span.end,
                        receiver.name.span.start,
                        receiver.name.span.end,
                        format!(
                            "{}: {}",
                            receiver.name.name,
                            format_type(&receiver.ty, source)
                        ),
                    ));
                }
                if contains(decl.name.span.start, decl.name.span.end, offset) {
                    let end = decl.signature.span.end;
                    return Some(target_for_name(
                        TargetKind::Func,
                        decl.span.start,
                        end,
                        decl.name.span.start,
                        decl.name.span.end,
                        source
                            .slice(compiler::source::Span::new(
                                decl.name.span.file,
                                decl.span.start,
                                end,
                            ))
                            .trim()
                            .to_string(),
                    ));
                }
            }
            ast::TopLevelDecl::Operator(_) => {}
        }
    }

    None
}

fn target_for_name(
    kind: TargetKind,
    declaration_start: usize,
    declaration_end: usize,
    name_start: usize,
    name_end: usize,
    signature: String,
) -> HoverTarget {
    HoverTarget {
        kind,
        declaration_start,
        declaration_end,
        name_start,
        name_end,
        signature,
    }
}

fn documentation_for(target: &HoverTarget, source: &SourceFile) -> Option<String> {
    let leading = leading_comment(source.text(), target.declaration_start);
    let raw = leading.or_else(|| {
        if matches!(
            target.kind,
            TargetKind::Var | TargetKind::Const | TargetKind::Field | TargetKind::AliasType
        ) {
            trailing_comment(
                source.text(),
                target.declaration_start,
                target.declaration_end,
            )
        } else {
            None
        }
    })?;
    Some(markdown_comment(&raw))
}

fn format_spec_signature<T: SpecSpan>(keyword: &str, spec: &T, source: &SourceFile) -> String {
    format!("{} {}", keyword, source.slice(spec.span()).trim())
}

trait SpecSpan {
    fn span(&self) -> compiler::source::Span;
}

impl SpecSpan for ast::VarSpec {
    fn span(&self) -> compiler::source::Span {
        self.span
    }
}

impl SpecSpan for ast::ConstSpec {
    fn span(&self) -> compiler::source::Span {
        self.span
    }
}

fn format_type(ty: &ast::Type, source: &SourceFile) -> String {
    match ty {
        ast::Type::Named(path) => source.slice(path.span).to_string(),
        ast::Type::Apply { span, .. } | ast::Type::Resolved(_, span) => {
            source.slice(*span).to_string()
        }
        ast::Type::Pointer(inner, _) => format!("*{}", format_type(inner, source)),
        ast::Type::Array { span, .. } => source.slice(*span).trim().to_string(),
        ast::Type::Struct { fields, .. } => {
            let fields = fields
                .iter()
                .map(|field| format!("{}: {}", field.name.name, format_type(&field.ty, source)))
                .collect::<Vec<_>>()
                .join("; ");
            format!("struct {{ {} }}", fields)
        }
        ast::Type::Interface { span, .. } => source.slice(*span).trim().to_string(),
        ast::Type::Func { span, .. } => source.slice(*span).trim().to_string(),
    }
}

fn contains(start: usize, end: usize, offset: usize) -> bool {
    start <= offset && offset < end
}

fn package_name_start(file: &ast::File, source: &SourceFile) -> usize {
    file.package.span.start
        + source
            .slice(file.package.span)
            .find(&file.package.name)
            .unwrap_or(0)
}

fn spec_comment_start(declaration_start: usize, spec_start: usize, source: &SourceFile) -> usize {
    if line_start(source.text(), declaration_start) == line_start(source.text(), spec_start) {
        declaration_start
    } else {
        spec_start
    }
}

fn leading_comment(text: &str, declaration_start: usize) -> Option<String> {
    let current_line_start = line_start(text, declaration_start);
    let same_line_prefix = text[current_line_start..declaration_start].trim();
    if !same_line_prefix.is_empty() {
        if same_line_prefix.starts_with("/*") && same_line_prefix.ends_with("*/") {
            return Some(normalize_block_comment(same_line_prefix));
        }
        return None;
    }

    let previous_end = current_line_start.saturating_sub(1);
    if previous_end == 0 && current_line_start == 0 {
        return None;
    }
    let previous_end = if text.as_bytes().get(previous_end) == Some(&b'\n') {
        previous_end
    } else {
        previous_end + 1
    };
    let previous_start = line_start(text, previous_end);
    let previous_line = text[previous_start..previous_end]
        .trim_end_matches(['\r', '\n'])
        .trim_end();
    if previous_line.trim().is_empty() {
        return None;
    }

    if previous_line.trim_start().starts_with("//") {
        let mut start = previous_start;
        loop {
            let line_end = start;
            if line_end == 0 {
                break;
            }
            let end = line_end.saturating_sub(1);
            let candidate_start = line_start(text, end);
            let candidate = text[candidate_start..line_end]
                .trim_end_matches(['\r', '\n'])
                .trim_end();
            if !candidate.trim_start().starts_with("//") {
                break;
            }
            start = candidate_start;
        }
        return Some(normalize_line_comments(&text[start..previous_end]));
    }

    if previous_line.trim_end().ends_with("*/") {
        let block_end = previous_end.min(text.len());
        let Some(block_start) = text[..block_end].rfind("/*") else {
            return None;
        };
        let block_line_start = line_start(text, block_start);
        if !text[block_line_start..block_start].trim().is_empty() {
            return None;
        }
        return Some(normalize_block_comment(
            &text[block_start..block_end.min(text.len())].replace("\r\n", "\n"),
        ));
    }

    None
}

fn trailing_comment(
    text: &str,
    declaration_start: usize,
    declaration_end: usize,
) -> Option<String> {
    let start = line_start(text, declaration_start);
    let end = text[start..]
        .find('\n')
        .map(|offset| start + offset)
        .unwrap_or(text.len());
    let bytes = text.as_bytes();
    let mut index = start;
    let mut state = LineState::Code;
    while index + 1 < end {
        match state {
            LineState::Code => match bytes[index] {
                b'"' => state = LineState::String,
                b'`' => state = LineState::RawString,
                b'/' if bytes[index + 1] == b'/' && index >= declaration_end => {
                    return Some(text[index + 2..end].trim().to_string());
                }
                _ => {}
            },
            LineState::String => {
                if bytes[index] == b'\\' {
                    index += 1;
                } else if bytes[index] == b'"' {
                    state = LineState::Code;
                }
            }
            LineState::RawString => {
                if bytes[index] == b'`' {
                    state = LineState::Code;
                }
            }
        }
        index += 1;
    }
    None
}

#[derive(Debug, Clone, Copy)]
enum LineState {
    Code,
    String,
    RawString,
}

fn line_start(text: &str, offset: usize) -> usize {
    text[..offset.min(text.len())]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0)
}

fn lsp_position(source: &SourceFile, offset: usize) -> Value {
    let (line, byte_column) = source.line_col(offset);
    let line_start = offset.saturating_sub(byte_column.saturating_sub(1));
    let character = source.text()[line_start..offset].encode_utf16().count();
    json!({
        "line": line.saturating_sub(1),
        "character": character
    })
}

fn normalize_line_comments(text: &str) -> String {
    text.lines()
        .map(|line| {
            let line = line.trim_start();
            let line = line.strip_prefix("//").unwrap_or(line);
            line.strip_prefix(' ').unwrap_or(line).trim_end()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_block_comment(text: &str) -> String {
    let text = text.trim();
    let body = text
        .strip_prefix("/*")
        .and_then(|body| body.strip_suffix("*/"))
        .unwrap_or(text);
    body.lines()
        .map(|line| {
            let line = line.trim_start();
            let line = line.strip_prefix('*').unwrap_or(line);
            line.strip_prefix(' ').unwrap_or(line).trim_end()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn markdown_comment(comment: &str) -> String {
    comment.split('\n').collect::<Vec<_>>().join("  \n")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use compiler::ir::ast;
    use compiler::parse_source_file;
    use compiler::source::SourceDb;
    use serde_json::Value;

    use super::{hover_for_file, hover_for_project};
    use crate::protocol::Position;

    fn hover_json(text: &str, line: u32, character: u32) -> Option<Value> {
        let mut sources = SourceDb::default();
        let file_id = sources.add_file(PathBuf::from("test.ond"), text.to_string());
        let file = parse_source_file(file_id, text).expect("source should parse");
        hover_for_file(&file, sources.file(file_id), &Position { line, character })
    }

    #[test]
    fn formats_function_types_with_optional_parameter_names() {
        let text = "package main\nvar callback: func(i32, flag: bool) -> i32\n";
        assert!(hover(text, 1, 5).contains("func(i32, flag: bool) -> i32"));
    }

    #[test]
    fn resolves_method_and_receiver_references() {
        let text = "package main\ntype Device struct {}\nfunc (device: *Device) Read() -> i32 { _ = device; return 1; }\nfunc main() { var device: *Device; _ = device.Read(); }\n";
        let method = hover_at(text, "Read", 1).expect("method hover");
        assert!(
            method["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("func (device: *Device) Read() -> i32")
        );
        let receiver = hover_at(text, "device", 1).expect("receiver hover");
        assert!(
            receiver["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("device: *Device")
        );
    }

    #[test]
    fn resolves_method_hover_for_an_inferred_generic_receiver() {
        let text = "package main\ntype List[T] struct { length: u32 }\nfunc (list: *List[T]) push(value: T) {}\nfunc NewList[T]() -> *List[T] { return new(List[T]) }\nfunc f() { l := NewList[u32](); l.push(1) }\n";

        let method = hover_at(text, "push", 1).expect("generic method hover");
        assert!(
            method["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("func (list: *List[T]) push(value: T)")
        );
    }

    #[test]
    fn resolves_generic_interface_method_hover_with_the_applied_type() {
        let text = "package main\ntype List[T] interface { Get(index: u32) -> T; }\nfunc use(values: List[u16]) { _ = values.Get(0) }\n";

        let method = hover_at(text, "Get", 1).expect("generic interface method hover");
        let value = method["contents"]["value"].as_str().unwrap();
        assert!(value.contains("Get(index: u32) -> u16"), "{value}");
    }

    #[test]
    fn resolves_imported_method_hover_for_an_inferred_generic_receiver() {
        let library_text = "package model\ntype Box[T] struct {}\nfunc (box: *Box[T]) Read() -> T { trap \"unused\" }\nfunc NewBox[T]() -> *Box[T] { return new(Box[T]) }\n";
        let usage_text = "package main\nimport \"model\"\nfunc f() { box := model.NewBox[u32](); _ = box.Read() }\n";
        let mut sources = SourceDb::default();
        let library_id = sources.add_file(PathBuf::from("model/model.ond"), library_text.into());
        let usage_id = sources.add_file(PathBuf::from("main.ond"), usage_text.into());
        let library = parse_source_file(library_id, library_text).unwrap();
        let usage = parse_source_file(usage_id, usage_text).unwrap();
        let main_package = ast::Package {
            logical_path: ".".into(),
            files: vec![usage.clone()],
        };
        let project = ast::Project {
            packages: vec![
                main_package.clone(),
                ast::Package {
                    logical_path: "model".into(),
                    files: vec![library],
                },
            ],
        };
        let offset = usage_text.rfind("Read").unwrap();
        let position = Position {
            line: 2,
            character: usage_text[usage_text[..offset].rfind('\n').unwrap() + 1..offset]
                .encode_utf16()
                .count() as u32,
        };

        let method = hover_for_project(&project, &main_package, &usage, &sources, &position)
            .expect("imported generic method hover");
        assert!(
            method["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("func (box: *Box[T]) Read() -> T")
        );
    }

    #[test]
    fn resolves_generic_struct_field_hover_with_the_applied_type() {
        let text = "package main\ntype Box[T] struct { value: T // stored value\n}\nfunc NewBox[T]() -> *Box[T] { return new(Box[T]) }\nfunc f() { box := NewBox[u32](); _ = box.value }\n";

        let field = hover_at(text, "value", 2).expect("generic field hover");
        let value = field["contents"]["value"].as_str().unwrap();
        assert!(value.contains("value: u32"));
        assert!(value.contains("stored value"));
    }

    #[test]
    fn resolves_imported_generic_struct_field_hover() {
        let library_text = "package model\ntype Box[T] struct { Value: T // exported value\n}\nfunc NewBox[T]() -> *Box[T] { return new(Box[T]) }\n";
        let usage_text = "package main\nimport \"model\"\nfunc f() { box := model.NewBox[u16](); _ = box.Value }\n";
        let mut sources = SourceDb::default();
        let library_id = sources.add_file(PathBuf::from("model/model.ond"), library_text.into());
        let usage_id = sources.add_file(PathBuf::from("main.ond"), usage_text.into());
        let library = parse_source_file(library_id, library_text).unwrap();
        let usage = parse_source_file(usage_id, usage_text).unwrap();
        let main_package = ast::Package {
            logical_path: ".".into(),
            files: vec![usage.clone()],
        };
        let project = ast::Project {
            packages: vec![
                main_package.clone(),
                ast::Package {
                    logical_path: "model".into(),
                    files: vec![library],
                },
            ],
        };
        let offset = usage_text.rfind("Value").unwrap();
        let position = Position {
            line: 2,
            character: usage_text[usage_text[..offset].rfind('\n').unwrap() + 1..offset]
                .encode_utf16()
                .count() as u32,
        };

        let field = hover_for_project(&project, &main_package, &usage, &sources, &position)
            .expect("imported generic field hover");
        let value = field["contents"]["value"].as_str().unwrap();
        assert!(value.contains("Value: u16"));
        assert!(value.contains("exported value"));
    }

    #[test]
    fn displays_declared_and_inferred_local_variable_types() {
        let text = "package main\ntype List[T] struct {}\nfunc NewList[T]() -> *List[T] { return new(List[T]) }\nfunc f(parameter: u16) { var explicit: u32; inferred := NewList[u32](); _ = parameter; _ = explicit; _ = inferred }\n";

        let parameter = hover_at(text, "parameter", 1).expect("parameter type hover");
        assert!(
            parameter["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("parameter: u16")
        );
        let explicit = hover_at(text, "explicit", 1).expect("explicit local type hover");
        assert!(
            explicit["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("explicit: u32")
        );
        let inferred = hover_at(text, "inferred", 1).expect("inferred local type hover");
        assert!(
            inferred["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("inferred: *List[u32]")
        );
    }

    #[test]
    fn displays_operator_parameter_types_at_declarations_and_uses() {
        let text = "package main\ntype List[T] struct {}\noperator[T] [](list: *List[T], index: u32) -> *T { _ = index; trap \"unused\" }\n";

        for occurrence in [0, 1] {
            let index = hover_at(text, "index", occurrence).expect("operator parameter hover");
            assert!(
                index["contents"]["value"]
                    .as_str()
                    .unwrap()
                    .contains("index: u32")
            );
        }
    }

    #[test]
    fn displays_inferred_top_level_variable_types_and_documentation() {
        let text = "package main\n// current count\nvar count = 1\nfunc f() { _ = count }\n";

        let count = hover_at(text, "count", 1).expect("top-level variable type hover");
        let value = count["contents"]["value"].as_str().unwrap();
        assert!(value.contains("var count: i32"));
        assert!(value.contains("current count"));
    }

    fn hover(text: &str, line: u32, character: u32) -> String {
        let value = hover_json(text, line, character)
            .unwrap_or_else(|| panic!("hover should exist at line {line}, character {character}"));
        value["contents"]["value"]
            .as_str()
            .expect("hover value should be a string")
            .to_string()
    }

    fn no_hover(text: &str, line: u32, character: u32) {
        assert!(
            hover_json(text, line, character).is_none(),
            "hover should be absent at line {line}, character {character}"
        );
    }

    fn hover_at(text: &str, needle: &str, occurrence: usize) -> Option<Value> {
        let offset = text
            .match_indices(needle)
            .nth(occurrence)
            .map(|(offset, _)| offset)
            .expect("needle should exist");
        let before = &text[..offset];
        let line = before.bytes().filter(|byte| *byte == b'\n').count() as u32;
        let line_start = before.rfind('\n').map(|index| index + 1).unwrap_or(0);
        let character = text[line_start..offset].encode_utf16().count() as u32;
        hover_json(text, line, character)
    }

    #[test]
    fn preserves_multiline_line_comment_breaks() {
        let value = hover(
            "package main\n// 一行目\n// 二行目\nfunc LoadStage(path: string) -> u32 {}\n",
            3,
            5,
        );
        assert!(value.contains("func LoadStage(path: string) -> u32"));
        assert!(value.contains("一行目  \n二行目"));
    }

    #[test]
    fn supports_allowed_trailing_comments_and_rejects_struct_type_trailing_comments() {
        let text = "package main\nconst value: u32 = 1 // const doc\nvar current: u32 // var doc\ntype Example struct {\n    member: u32 // field doc\n} // struct type should not use this\ntype Alias u16 // alias doc\n";
        assert!(hover(text, 1, 7).contains("const doc"));
        assert!(hover(text, 2, 5).contains("var doc"));
        assert!(hover(text, 4, 5).contains("field doc"));
        assert!(!hover(text, 3, 5).contains("struct type should not use this"));
        assert!(hover(text, 6, 5).contains("alias doc"));
    }

    #[test]
    fn preceding_comment_takes_priority_over_trailing_comment() {
        let text = "package main\n// leading doc\nconst value: u32 = 1 // trailing doc\n";
        let value = hover(text, 2, 7);
        assert!(value.contains("leading doc"));
        assert!(!value.contains("trailing doc"));
    }

    #[test]
    fn supports_package_and_struct_type_preceding_comments() {
        let text = "// package doc\npackage main\n// struct doc\ntype Example struct {}\n";
        let package = hover(text, 1, 8);
        assert!(package.contains("package doc"));
        assert!(package.contains("package main"));

        let structure = hover(text, 3, 6);
        assert!(structure.contains("struct doc"));
        assert!(structure.contains("type Example struct"));
    }

    #[test]
    fn rejects_comments_gaps_and_non_identifier_positions() {
        let text = "package main\n// separated doc\n\nconst value: u32 = 1\n";
        no_hover(text, 1, 4);
        no_hover(text, 3, 0);
        no_hover(text, 3, 13);
        assert!(!hover(text, 3, 7).contains("separated doc"));

        let package = "// package doc\npackage main\n";
        no_hover(package, 1, 0);
        no_hover(package, 0, 4);
    }

    #[test]
    fn handles_crlf_multiline_japanese_comments_and_hover_range() {
        let text = "package main\r\n// 一行目\r\n// 二行目\r\nconst value: u32 = 1\r\n";
        let value = hover_json(text, 3, 7).expect("hover should exist");
        let markdown = value["contents"]["value"]
            .as_str()
            .expect("hover value should be a string");
        assert!(markdown.contains("一行目  \n二行目"));
        assert_eq!(value["range"]["start"]["line"], 3);
        assert_eq!(value["range"]["start"]["character"], 6);
        assert_eq!(value["range"]["end"]["character"], 11);
    }

    #[test]
    fn does_not_treat_slashes_inside_string_literals_as_trailing_comments() {
        let text = "package main\nconst url: string = \"https://example.test\"\n";
        let value = hover(text, 1, 7);
        assert!(value.contains("https://example.test"));
        assert!(value.ends_with("\n```"));
    }

    #[test]
    fn supports_preceding_comments_for_each_declaration_kind() {
        let text = "package main\n// func doc\nfunc load() {}\n// var doc\nvar current: u32\n// const doc\nconst limit: u32 = 1\ntype Example struct {\n    // field doc\n    member: u32\n}\n";
        assert!(hover(text, 2, 5).contains("func doc"));
        assert!(hover(text, 4, 5).contains("var doc"));
        assert!(hover(text, 6, 6).contains("const doc"));
        assert!(hover(text, 9, 5).contains("field doc"));
    }

    #[test]
    fn normalizes_common_multiline_block_comments() {
        let text = "package main\n/*\n * 一行目\n * 二行目\n */\ntype Example u32\n";
        let value = hover(text, 5, 6);
        assert!(value.contains("一行目  \n二行目"));
    }

    #[test]
    fn handles_block_comments_and_utf16_hover_positions() {
        let text = "package main\n/* 😀 block doc */ const value: u32 = 1\n";
        let character = text[..text.find("value").unwrap()].encode_utf16().count() as u32;
        let value = hover(
            text,
            1,
            character - text[..text.find('\n').unwrap() + 1].encode_utf16().count() as u32,
        );
        assert!(value.contains("block doc"));
        assert!(value.contains("const value: u32 = 1"));
    }

    #[test]
    fn resolves_top_level_function_variable_constant_and_type_references() {
        let text = "package main\n// function doc\nfunc load() -> u32 {\n    return Limit\n}\n// const doc\nconst Limit: u32 = 1\n// var doc\nvar Current: u32\n// type doc\ntype Count u32\nfunc main() {\n    var value: Count = load()\n    Current = value + Limit\n}\n";

        let function = hover_at(text, "load", 1).expect("function reference hover");
        assert!(
            function["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("function doc")
        );
        let constant = hover_at(text, "Limit", 2).expect("const reference hover");
        assert!(
            constant["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("const doc")
        );
        let variable = hover_at(text, "Current", 1).expect("var reference hover");
        assert!(
            variable["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("var doc")
        );
        let ty = hover_at(text, "Count", 1).expect("type reference hover");
        assert!(
            ty["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("type doc")
        );
    }

    #[test]
    fn resolves_struct_field_access_to_the_base_struct() {
        let text = "package main\n// first type\ntype First struct {\n    value: u32 // first field\n}\n// second type\ntype Second struct {\n    value: u16 // second field\n}\nfunc read(first: First, second: Second) -> u32 {\n    var a: u32 = first.value\n    var b: u16 = second.value\n    return a + (b as u32)\n}\n";
        let first = hover_at(text, "value", 2).expect("first field hover");
        let first = first["contents"]["value"].as_str().unwrap();
        assert!(first.contains("value: u32"));
        assert!(first.contains("first field"));
        assert!(!first.contains("second field"));

        let second = hover_at(text, "value", 3).expect("second field hover");
        let second = second["contents"]["value"].as_str().unwrap();
        assert!(second.contains("value: u16"));
        assert!(second.contains("second field"));
        assert!(!second.contains("first field"));
    }

    #[test]
    fn displays_types_for_names_that_shadow_top_level_declarations() {
        let text = "package main\n// top value\nvar value: u32\nfunc withParam(value: u16) -> u16 {\n    return value\n}\nfunc withLocal() -> u16 {\n    var value: u16 = 1\n    return value\n}\nfunc withInferredLocal() -> u32 {\n    var value = 1\n    return value\n}\n";
        assert!(
            hover_at(text, "value", 2).unwrap()["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("value: u16")
        );
        assert!(
            hover_at(text, "value", 4).unwrap()["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("value: u16")
        );
        assert!(
            hover_at(text, "value", 6).unwrap()["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("value: i32")
        );
    }

    #[test]
    fn inner_block_binding_does_not_shadow_a_later_outer_reference() {
        let text = "package main\nvar value: u32 // top doc\nfunc read() -> u32 {\n    {\n        var value: u16\n        value = 1\n    }\n    return value\n}\n";
        let value = hover_at(text, "value", 3).expect("top-level reference after inner block");
        assert!(
            value["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("top doc")
        );
    }

    #[test]
    fn sibling_branch_binding_does_not_leak_to_other_or_outer_scopes() {
        let text = "package main\nvar value: u32 // top doc\nfunc read(flag: bool) -> u32 {\n    if flag {\n        var value: u16\n        value = 1\n    } else {\n        value = 2\n    }\n    return value\n}\n";
        let sibling = hover_at(text, "value", 3).expect("reference in sibling branch");
        assert!(
            sibling["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("top doc")
        );
        let outer = hover_at(text, "value", 4).expect("reference after if");
        assert!(
            outer["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("top doc")
        );
    }

    #[test]
    fn if_and_for_init_bindings_are_limited_to_their_statements() {
        let if_text = "package main\nvar value: u32 // top doc\nfunc read() -> u32 {\n    if value := 1; true {\n        return value\n    }\n    return value\n}\n";
        assert!(
            hover_at(if_text, "value", 2).unwrap()["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("value: i32")
        );
        let after_if = hover_at(if_text, "value", 3).expect("top-level value after if");
        assert!(
            after_if["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("top doc")
        );

        let for_text = "package main\nvar value: u32 // top doc\nfunc read() -> u32 {\n    for value := 1; value < 2; value = value + 1 {\n        return value\n    }\n    return value\n}\n";
        assert!(
            hover_at(for_text, "value", 2).unwrap()["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("value: i32")
        );
        assert!(
            hover_at(for_text, "value", 5).unwrap()["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("value: i32")
        );
        let after_for = hover_at(for_text, "value", 6).expect("top-level value after for");
        assert!(
            after_for["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("top doc")
        );
    }

    #[test]
    fn field_type_resolution_uses_the_current_lexical_scope() {
        let text = "package main\ntype Outer struct {\n    value: u32 // outer field\n}\ntype Inner struct {\n    value: u16 // inner field\n}\nvar item: Outer\nfunc read() -> u32 {\n    {\n        var item: Inner\n        item.value = 1\n    }\n    return item.value\n}\n";
        let inner = hover_at(text, "value", 2).expect("inner field reference");
        assert!(
            inner["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("inner field")
        );
        let outer = hover_at(text, "value", 3).expect("outer field after inner block");
        assert!(
            outer["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("outer field")
        );
    }

    #[test]
    fn explicitly_typed_local_const_can_resolve_field_access() {
        let text = "package main\ntype Item struct {\n    value: u32 // item field\n}\nfunc read() -> u32 {\n    const item: Item = Item{value: 1}\n    return item.value\n}\n";
        let field = hover_at(text, "value", 2).expect("local const field hover");
        assert!(
            field["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("item field")
        );
    }

    #[test]
    fn reference_hover_ignores_comments_strings_and_keeps_utf16_range() {
        let text = "package main\n// target doc\nconst Target: u32 = 1\nfunc main() {\n    // Target\n    var text: [11]u8 = `😀 Target`\n    var value: u32 = len(\"😀\") + Target\n}\n";
        assert!(hover_at(text, "Target", 1).is_none());
        assert!(hover_at(text, "Target", 2).is_none());
        let value = hover_at(text, "Target", 3).expect("UTF-16 reference hover");
        let start = value["range"]["start"]["character"].as_u64().unwrap();
        let line = text.lines().nth(6).unwrap();
        let expected = line[..line.rfind("Target").unwrap()].encode_utf16().count() as u64;
        assert_eq!(start, expected);
        assert!(
            value["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("target doc")
        );
    }

    #[test]
    fn resolves_same_package_symbol_from_another_file() {
        let declaration_text = "package main\n// shared doc\nconst Shared: u32 = 1\n";
        let usage_text = "package main\nfunc main() -> u32 {\n    return Shared\n}\n";
        let mut sources = SourceDb::default();
        let declaration_id = sources.add_file(PathBuf::from("decl.ond"), declaration_text.into());
        let usage_id = sources.add_file(PathBuf::from("use.ond"), usage_text.into());
        let declaration = parse_source_file(declaration_id, declaration_text).unwrap();
        let usage = parse_source_file(usage_id, usage_text).unwrap();
        let package = ast::Package {
            logical_path: ".".into(),
            files: vec![declaration, usage.clone()],
        };
        let project = ast::Project {
            packages: vec![package.clone()],
        };
        let offset = usage_text.rfind("Shared").unwrap();
        let position = Position {
            line: 2,
            character: usage_text[usage_text[..offset].rfind('\n').unwrap() + 1..offset]
                .encode_utf16()
                .count() as u32,
        };
        let value = hover_for_project(&project, &package, &usage, &sources, &position)
            .expect("cross-file hover");
        assert!(
            value["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("shared doc")
        );
    }

    #[test]
    fn resolves_exported_symbol_through_package_qualifier() {
        let library_text = "package lib\n// exported doc\nconst Shared: u32 = 1\n";
        let usage_text =
            "package main\nimport \"lib\"\nfunc main() -> u32 {\n    return lib.Shared\n}\n";
        let mut sources = SourceDb::default();
        let library_id = sources.add_file(PathBuf::from("lib/lib.ond"), library_text.into());
        let usage_id = sources.add_file(PathBuf::from("main.ond"), usage_text.into());
        let library = parse_source_file(library_id, library_text).unwrap();
        let usage = parse_source_file(usage_id, usage_text).unwrap();
        let main_package = ast::Package {
            logical_path: ".".into(),
            files: vec![usage.clone()],
        };
        let library_package = ast::Package {
            logical_path: "lib".into(),
            files: vec![library],
        };
        let project = ast::Project {
            packages: vec![main_package.clone(), library_package],
        };
        let offset = usage_text.rfind("Shared").unwrap();
        let line_start = usage_text[..offset].rfind('\n').unwrap() + 1;
        let position = Position {
            line: 3,
            character: usage_text[line_start..offset].encode_utf16().count() as u32,
        };
        let value = hover_for_project(&project, &main_package, &usage, &sources, &position)
            .expect("qualified reference hover");
        assert!(
            value["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("exported doc")
        );
    }

    #[test]
    fn resolves_imported_package_qualifier_and_prefers_commented_package_clause() {
        let plain_text = "package diag\nfunc Other() {}\n";
        let documented_text = "// diagnostic package\n// reports test results\npackage diag\nfunc Pass(code: u32) {}\n";
        let usage_text =
            "package main\nimport \"tools/diag\"\nfunc main() {\n    diag.Pass(0)\n}\n";
        let mut sources = SourceDb::default();
        let plain_id = sources.add_file(PathBuf::from("tools/diag/a.ond"), plain_text.into());
        let documented_id =
            sources.add_file(PathBuf::from("tools/diag/b.ond"), documented_text.into());
        let usage_id = sources.add_file(PathBuf::from("main.ond"), usage_text.into());
        let plain = parse_source_file(plain_id, plain_text).unwrap();
        let documented = parse_source_file(documented_id, documented_text).unwrap();
        let usage = parse_source_file(usage_id, usage_text).unwrap();
        let main_package = ast::Package {
            logical_path: ".".into(),
            files: vec![usage.clone()],
        };
        let imported_package = ast::Package {
            logical_path: "tools/diag".into(),
            files: vec![plain, documented],
        };
        let project = ast::Project {
            packages: vec![main_package.clone(), imported_package],
        };
        let offset = usage_text.rfind("diag.Pass").unwrap();
        let line_start = usage_text[..offset].rfind('\n').unwrap() + 1;
        let position = Position {
            line: 3,
            character: usage_text[line_start..offset].encode_utf16().count() as u32,
        };
        let value = hover_for_project(&project, &main_package, &usage, &sources, &position)
            .expect("package qualifier hover");
        let markdown = value["contents"]["value"].as_str().unwrap();
        assert!(markdown.contains("package diag"));
        assert!(markdown.contains("diagnostic package  \nreports test results"));
        assert_eq!(value["range"]["start"]["character"], 4);
        assert_eq!(value["range"]["end"]["character"], 8);
    }

    #[test]
    fn package_qualifier_requires_an_import_and_respects_local_shadowing() {
        let unimported = "package main\nfunc main() {\n    diag.Pass(0)\n}\n";
        assert!(hover_at(unimported, "diag", 0).is_none());

        let shadowed = "package main\nimport \"diag\"\ntype Local struct {\n    Pass: u32\n}\nfunc main(diag: Local) {\n    diag.Pass = 1\n}\n";
        assert!(
            hover_at(shadowed, "diag", 1).unwrap()["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("diag: Local")
        );
    }

    #[test]
    fn resolves_composite_literal_field_keys() {
        let text = "package main\ntype Pair struct {\n    Left: u32 // left field\n    Right: u32 // right field\n}\nfunc make() -> Pair {\n    return Pair{Left: 40 as u32, Right: 2 as u32}\n}\n";
        let left = hover_at(text, "Left", 1).expect("Left key hover");
        let left = left["contents"]["value"].as_str().unwrap();
        assert!(left.contains("Left: u32"));
        assert!(left.contains("left field"));

        let right = hover_at(text, "Right", 1).expect("Right key hover");
        let right = right["contents"]["value"].as_str().unwrap();
        assert!(right.contains("Right: u32"));
        assert!(right.contains("right field"));
    }

    #[test]
    fn composite_keys_use_their_immediate_struct_type() {
        let text = "package main\ntype First struct { Value: u32 // first value\n}\ntype Second struct { Value: u16 // second value\n}\ntype Outer struct { Child: Second // child field\n}\nfunc make() {\n    var first: First = First{Value: 1}\n    var outer: Outer = Outer{Child: Second{Value: 2 as u16}}\n}\n";
        let first = hover_at(text, "Value", 2).expect("First.Value key");
        let first = first["contents"]["value"].as_str().unwrap();
        assert!(first.contains("Value: u32"));
        assert!(first.contains("first value"));

        let child = hover_at(text, "Child", 1).expect("Outer.Child key");
        assert!(
            child["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("child field")
        );

        let second = hover_at(text, "Value", 3).expect("nested Second.Value key");
        let second = second["contents"]["value"].as_str().unwrap();
        assert!(second.contains("Value: u16"));
        assert!(second.contains("second value"));
        assert!(!second.contains("first value"));
    }

    #[test]
    fn positional_elements_and_regular_names_are_not_composite_keys() {
        let text = "package main\nconst Left: u32 = 1 // const left\ntype Pair struct { Left: u32 // field left\n}\nfunc make() {\n    var positional: [1]u32 = [1]u32{Left}\n    var regular: u32 = Left\n}\n";
        let positional = hover_at(text, "Left", 2).expect("positional expression hover");
        let positional = positional["contents"]["value"].as_str().unwrap();
        assert!(positional.contains("const Left"));
        assert!(positional.contains("const left"));
        assert!(!positional.contains("field left"));

        let regular = hover_at(text, "Left", 3).expect("regular name hover");
        assert!(
            regular["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("const left")
        );
    }

    #[test]
    fn composite_key_hover_range_uses_utf16_columns() {
        let text = "package main\ntype Pair struct { Left: u32 // field doc\n}\nfunc make() {\n    var pair: Pair = Pair{Left: (\"😀\" == \"😀\") as u32}\n}\n";
        let value = hover_at(text, "Left", 1).expect("UTF-16 composite key hover");
        let line = text.lines().nth(4).unwrap();
        let expected = line[..line.rfind("Left").unwrap()].encode_utf16().count() as u64;
        assert_eq!(value["range"]["start"]["character"], expected);
        assert_eq!(value["range"]["end"]["character"], expected + 4);
    }

    #[test]
    fn resolves_imported_struct_composite_field_key() {
        let library_text = "package model\ntype Pair struct {\n    Left: u32 // imported left\n}\n";
        let usage_text = "package main\nimport \"model\"\nfunc make() -> model.Pair {\n    return model.Pair{Left: 1}\n}\n";
        let mut sources = SourceDb::default();
        let library_id = sources.add_file(PathBuf::from("model/pair.ond"), library_text.into());
        let usage_id = sources.add_file(PathBuf::from("main.ond"), usage_text.into());
        let library = parse_source_file(library_id, library_text).unwrap();
        let usage = parse_source_file(usage_id, usage_text).unwrap();
        let main_package = ast::Package {
            logical_path: ".".into(),
            files: vec![usage.clone()],
        };
        let model_package = ast::Package {
            logical_path: "model".into(),
            files: vec![library],
        };
        let project = ast::Project {
            packages: vec![main_package.clone(), model_package],
        };
        let offset = usage_text.rfind("Left").unwrap();
        let line_start = usage_text[..offset].rfind('\n').unwrap() + 1;
        let position = Position {
            line: 3,
            character: usage_text[line_start..offset].encode_utf16().count() as u32,
        };
        let value = hover_for_project(&project, &main_package, &usage, &sources, &position)
            .expect("imported composite key hover");
        let markdown = value["contents"]["value"].as_str().unwrap();
        assert!(markdown.contains("Left: u32"));
        assert!(markdown.contains("imported left"));
    }

    #[test]
    fn declaration_hover_still_works_after_reference_resolution() {
        let text = "package main\n// declaration doc\nfunc target() {}\n";
        let value = hover_at(text, "target", 0).expect("declaration hover");
        assert!(
            value["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("declaration doc")
        );
    }

    #[test]
    fn source_location_builtins_have_hover_and_respect_source_context() {
        let text = "package main\nfunc main(){var file=thisFile();var line=thisLine()}\n";
        let file = hover_at(text, "thisFile", 0).expect("thisFile hover");
        let line = hover_at(text, "thisLine", 0).expect("thisLine hover");
        assert!(
            file["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("thisFile() -> *u8")
        );
        assert!(
            line["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("thisLine() -> u32")
        );

        assert!(
            hover_at(
                "package main\nfunc main(){var text=\"thisLine()\"}\n",
                "thisLine",
                0
            )
            .is_none()
        );
    }
}
