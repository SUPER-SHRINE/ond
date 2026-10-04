use compiler::ir::ast;
use compiler::source::{SourceDb, Span};
use serde_json::{Value, json};

use crate::hover_reference::{ResolvedTarget, resolve_reference};
use crate::protocol::{Position, lsp_offset};
use crate::symbols::span_to_range;
use crate::util::uri_from_path;

pub fn definition_for_file(
    file: &ast::File,
    sources: &SourceDb,
    position: &Position,
) -> Option<Value> {
    let package = ast::Package {
        logical_path: ".".to_string(),
        files: vec![file.clone()],
    };
    let project = ast::Project {
        packages: vec![package.clone()],
    };
    definition_for_project(&project, &package, file, sources, position)
}

pub fn definition_for_project(
    project: &ast::Project,
    package: &ast::Package,
    file: &ast::File,
    sources: &SourceDb,
    position: &Position,
) -> Option<Value> {
    let source = sources.file(file.file_id);
    let offset = lsp_offset(source.text(), position).ok()?;

    if let Some(origin) = import_string_at(file, sources, offset) {
        let import = file.imports.iter().find(|import| import.span == origin.1)?;
        let imported = project
            .packages
            .iter()
            .find(|candidate| candidate.logical_path == import.path.trim_matches('"'))?;
        let target = package_target(imported, sources)?;
        return location_link(origin.0, target.0, target.1, sources);
    }

    if let Some((selection, full)) = declaration_at(file, sources, offset) {
        return location_link(selection, selection, full, sources);
    }

    let resolved = resolve_reference(project, package, file, offset)?;
    match resolved.target {
        ResolvedTarget::Declaration(selection) => {
            let full = declaration_full_span(project, selection).unwrap_or(selection);
            location_link(resolved.reference, selection, full, sources)
        }
        ResolvedTarget::Package(index) => {
            let (selection, full) = package_target(project.packages.get(index)?, sources)?;
            location_link(resolved.reference, selection, full, sources)
        }
    }
}

fn location_link(origin: Span, target: Span, full: Span, sources: &SourceDb) -> Option<Value> {
    let uri = uri_from_path(sources.file(target.file).path()).ok()?;
    Some(json!([{
        "originSelectionRange": span_to_range(origin, sources),
        "targetUri": uri,
        "targetRange": span_to_range(full, sources),
        "targetSelectionRange": span_to_range(target, sources)
    }]))
}

fn package_target(package: &ast::Package, sources: &SourceDb) -> Option<(Span, Span)> {
    let file = package.files.first()?;
    let source = sources.file(file.file_id);
    let relative = source.slice(file.package.span).rfind(&file.package.name)?;
    let start = file.package.span.start + relative;
    Some((
        Span::new(file.file_id, start, start + file.package.name.len()),
        file.package.span,
    ))
}

fn import_string_at(file: &ast::File, sources: &SourceDb, offset: usize) -> Option<(Span, Span)> {
    let source = sources.file(file.file_id);
    file.imports.iter().find_map(|import| {
        if !(import.span.start <= offset && offset < import.span.end) {
            return None;
        }
        let text = source.slice(import.span);
        let start = text.find('"')? + import.span.start;
        let end = text[start - import.span.start + 1..].find('"')? + start + 2;
        (start <= offset && offset < end)
            .then_some((Span::new(file.file_id, start, end), import.span))
    })
}

fn declaration_at(file: &ast::File, sources: &SourceDb, offset: usize) -> Option<(Span, Span)> {
    let source = sources.file(file.file_id);
    let package_start =
        file.package.span.start + source.slice(file.package.span).rfind(&file.package.name)?;
    let package_name = Span::new(
        file.file_id,
        package_start,
        package_start + file.package.name.len(),
    );
    if contains(package_name, offset) {
        return Some((package_name, file.package.span));
    }
    file.decls
        .iter()
        .find_map(|decl| declaration_in_top(decl, offset))
}

fn declaration_in_top(decl: &ast::TopLevelDecl, offset: usize) -> Option<(Span, Span)> {
    match decl {
        ast::TopLevelDecl::Const(decl) => decl.specs.iter().find_map(|spec| {
            ident_in(&spec.names, spec.span, offset).or_else(|| {
                spec.ty
                    .as_ref()
                    .and_then(|ty| declaration_in_type(ty, offset))
            })
        }),
        ast::TopLevelDecl::Var(decl) => decl.specs.iter().find_map(|spec| {
            ident_in(&spec.names, spec.span, offset).or_else(|| {
                spec.ty
                    .as_ref()
                    .and_then(|ty| declaration_in_type(ty, offset))
            })
        }),
        ast::TopLevelDecl::Type(decl) => decl.specs.iter().find_map(|spec| {
            contains(spec.name.span, offset)
                .then_some((spec.name.span, spec.span))
                .or_else(|| declaration_in_type(&spec.ty, offset))
        }),
        ast::TopLevelDecl::Func(function) => contains(function.name.span, offset)
            .then_some((function.name.span, function.span))
            .or_else(|| {
                function.receiver.as_ref().and_then(|receiver| {
                    contains(receiver.name.span, offset)
                        .then_some((receiver.name.span, receiver.span))
                        .or_else(|| declaration_in_type(&receiver.ty, offset))
                })
            })
            .or_else(|| {
                function.signature.params.iter().find_map(|field| {
                    contains(field.name.span, offset)
                        .then_some((field.name.span, field.span))
                        .or_else(|| declaration_in_type(&field.ty, offset))
                })
            })
            .or_else(|| declaration_in_block(&function.body, offset)),
        ast::TopLevelDecl::Operator(_) => None,
    }
}

fn declaration_in_block(block: &ast::Block, offset: usize) -> Option<(Span, Span)> {
    block.statements.iter().find_map(|stmt| match stmt {
        ast::Stmt::Const(decl) => decl
            .specs
            .iter()
            .find_map(|spec| ident_in(&spec.names, spec.span, offset)),
        ast::Stmt::Var(decl) => decl
            .specs
            .iter()
            .find_map(|spec| ident_in(&spec.names, spec.span, offset)),
        ast::Stmt::ShortVar(_) => None,
        ast::Stmt::Block(block) => declaration_in_block(block, offset),
        ast::Stmt::Defer(stmt) => declaration_in_block(&stmt.block, offset),
        ast::Stmt::If(stmt) => stmt
            .init
            .as_deref()
            .and_then(|stmt| declaration_in_stmt(stmt, offset))
            .or_else(|| declaration_in_block(&stmt.then_block, offset))
            .or_else(|| {
                stmt.else_branch
                    .as_deref()
                    .and_then(|stmt| declaration_in_stmt(stmt, offset))
            }),
        ast::Stmt::For(stmt) => {
            let header = if let ast::ForKind::ThreeClause { init, post, .. } = &stmt.kind {
                init.as_deref()
                    .and_then(|stmt| declaration_in_stmt(stmt, offset))
                    .or_else(|| {
                        post.as_deref()
                            .and_then(|stmt| declaration_in_stmt(stmt, offset))
                    })
            } else {
                None
            };
            header.or_else(|| declaration_in_block(&stmt.body, offset))
        }
        _ => None,
    })
}

fn declaration_in_stmt(stmt: &ast::Stmt, offset: usize) -> Option<(Span, Span)> {
    match stmt {
        ast::Stmt::Const(decl) => decl
            .specs
            .iter()
            .find_map(|spec| ident_in(&spec.names, spec.span, offset)),
        ast::Stmt::Var(decl) => decl
            .specs
            .iter()
            .find_map(|spec| ident_in(&spec.names, spec.span, offset)),
        ast::Stmt::ShortVar(_) => None,
        ast::Stmt::Block(block) => declaration_in_block(block, offset),
        ast::Stmt::Defer(defer) => declaration_in_block(&defer.block, offset),
        ast::Stmt::If(stmt) => declaration_in_block(&stmt.then_block, offset),
        ast::Stmt::For(stmt) => declaration_in_block(&stmt.body, offset),
        _ => None,
    }
}

fn declaration_in_type(ty: &ast::Type, offset: usize) -> Option<(Span, Span)> {
    match ty {
        ast::Type::Apply { arguments, .. } => arguments
            .iter()
            .find_map(|argument| declaration_in_type(argument, offset)),
        ast::Type::Resolved(_, _) => None,
        ast::Type::Pointer(inner, _) => declaration_in_type(inner, offset),
        ast::Type::Array { element, .. } => declaration_in_type(element, offset),
        ast::Type::Struct { fields, .. } => fields.iter().find_map(|field| {
            contains(field.name.span, offset)
                .then_some((field.name.span, field.span))
                .or_else(|| declaration_in_type(&field.ty, offset))
        }),
        ast::Type::Interface { methods, .. } => methods.iter().find_map(|method| {
            contains(method.name.span, offset)
                .then_some((method.name.span, method.span))
                .or_else(|| {
                    method.signature.params.iter().find_map(|parameter| {
                        parameter
                            .name
                            .as_ref()
                            .and_then(|name| {
                                contains(name.span, offset).then_some((name.span, parameter.span))
                            })
                            .or_else(|| declaration_in_type(&parameter.ty, offset))
                    })
                })
                .or_else(|| {
                    method
                        .signature
                        .results
                        .iter()
                        .find_map(|ty| declaration_in_type(ty, offset))
                })
        }),
        ast::Type::Func { signature, .. } => signature.params.iter().find_map(|field| {
            field
                .name
                .as_ref()
                .and_then(|name| contains(name.span, offset).then_some((name.span, field.span)))
                .or_else(|| declaration_in_type(&field.ty, offset))
        }),
        ast::Type::Named(_) => None,
    }
}

fn ident_in(names: &[ast::Ident], full: Span, offset: usize) -> Option<(Span, Span)> {
    names
        .iter()
        .find(|name| contains(name.span, offset))
        .map(|name| (name.span, full))
}

fn declaration_full_span(project: &ast::Project, selection: Span) -> Option<Span> {
    project
        .packages
        .iter()
        .flat_map(|package| &package.files)
        .find_map(|file| {
            if file.file_id != selection.file {
                return None;
            }
            file.decls
                .iter()
                .find_map(|decl| full_in_top(decl, selection))
        })
}

fn full_in_top(decl: &ast::TopLevelDecl, selection: Span) -> Option<Span> {
    match decl {
        ast::TopLevelDecl::Const(decl) => decl.specs.iter().find_map(|s| {
            s.names
                .iter()
                .any(|n| n.span == selection)
                .then_some(s.span)
        }),
        ast::TopLevelDecl::Var(decl) => decl.specs.iter().find_map(|s| {
            s.names
                .iter()
                .any(|n| n.span == selection)
                .then_some(s.span)
        }),
        ast::TopLevelDecl::Type(decl) => decl.specs.iter().find_map(|s| {
            (s.name.span == selection)
                .then_some(s.span)
                .or_else(|| full_in_type(&s.ty, selection))
        }),
        ast::TopLevelDecl::Func(f) => (f.name.span == selection)
            .then_some(f.span)
            .or_else(|| {
                f.receiver
                    .as_ref()
                    .filter(|receiver| receiver.name.span == selection)
                    .map(|receiver| receiver.span)
            })
            .or_else(|| {
                f.signature
                    .params
                    .iter()
                    .find(|p| p.name.span == selection)
                    .map(|p| p.span)
            })
            .or_else(|| full_in_block(&f.body, selection)),
        ast::TopLevelDecl::Operator(_) => None,
    }
}

fn full_in_block(block: &ast::Block, selection: Span) -> Option<Span> {
    block.statements.iter().find_map(|stmt| match stmt {
        ast::Stmt::Const(d) => d.specs.iter().find_map(|s| {
            s.names
                .iter()
                .any(|n| n.span == selection)
                .then_some(s.span)
        }),
        ast::Stmt::Var(d) => d.specs.iter().find_map(|s| {
            s.names
                .iter()
                .any(|n| n.span == selection)
                .then_some(s.span)
        }),
        ast::Stmt::ShortVar(d) => d
            .names
            .iter()
            .any(|n| n.span == selection)
            .then_some(d.span),
        ast::Stmt::Block(b) => full_in_block(b, selection),
        ast::Stmt::If(s) => s
            .init
            .as_deref()
            .and_then(|x| full_in_stmt(x, selection))
            .or_else(|| full_in_block(&s.then_block, selection))
            .or_else(|| {
                s.else_branch
                    .as_deref()
                    .and_then(|x| full_in_stmt(x, selection))
            }),
        ast::Stmt::For(s) => {
            let header = if let ast::ForKind::ThreeClause { init, post, .. } = &s.kind {
                init.as_deref()
                    .and_then(|stmt| full_in_stmt(stmt, selection))
                    .or_else(|| {
                        post.as_deref()
                            .and_then(|stmt| full_in_stmt(stmt, selection))
                    })
            } else {
                None
            };
            header.or_else(|| full_in_block(&s.body, selection))
        }
        _ => None,
    })
}

fn full_in_stmt(stmt: &ast::Stmt, selection: Span) -> Option<Span> {
    match stmt {
        ast::Stmt::Const(d) => d.specs.iter().find_map(|s| {
            s.names
                .iter()
                .any(|n| n.span == selection)
                .then_some(s.span)
        }),
        ast::Stmt::Var(d) => d.specs.iter().find_map(|s| {
            s.names
                .iter()
                .any(|n| n.span == selection)
                .then_some(s.span)
        }),
        ast::Stmt::ShortVar(d) => d
            .names
            .iter()
            .any(|n| n.span == selection)
            .then_some(d.span),
        ast::Stmt::Block(b) => full_in_block(b, selection),
        _ => None,
    }
}

fn full_in_type(ty: &ast::Type, selection: Span) -> Option<Span> {
    match ty {
        ast::Type::Apply { arguments, .. } => arguments
            .iter()
            .find_map(|argument| full_in_type(argument, selection)),
        ast::Type::Resolved(_, _) => None,
        ast::Type::Pointer(inner, _) => full_in_type(inner, selection),
        ast::Type::Array { element, .. } => full_in_type(element, selection),
        ast::Type::Struct { fields, .. } => fields.iter().find_map(|f| {
            (f.name.span == selection)
                .then_some(f.span)
                .or_else(|| full_in_type(&f.ty, selection))
        }),
        ast::Type::Interface { methods, .. } => methods.iter().find_map(|method| {
            (method.name.span == selection)
                .then_some(method.span)
                .or_else(|| {
                    method.signature.params.iter().find_map(|parameter| {
                        parameter
                            .name
                            .as_ref()
                            .is_some_and(|name| name.span == selection)
                            .then_some(parameter.span)
                            .or_else(|| full_in_type(&parameter.ty, selection))
                    })
                })
                .or_else(|| {
                    method
                        .signature
                        .results
                        .iter()
                        .find_map(|ty| full_in_type(ty, selection))
                })
        }),
        ast::Type::Func { signature, .. } => signature
            .params
            .iter()
            .find(|f| f.name.as_ref().is_some_and(|name| name.span == selection))
            .map(|f| f.span),
        ast::Type::Named(_) => None,
    }
}

fn contains(span: Span, offset: usize) -> bool {
    span.start <= offset && offset < span.end
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use compiler::source::SourceDb;
    use serde_json::Value;

    use super::{definition_for_file, definition_for_project};
    use crate::protocol::Position;

    fn position(text: &str, needle: &str, occurrence: usize) -> Position {
        let offset = text
            .match_indices(needle)
            .nth(occurrence)
            .expect("needle occurrence")
            .0;
        let prefix = &text[..offset];
        let line = prefix.bytes().filter(|byte| *byte == b'\n').count() as u32;
        let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
        Position {
            line,
            character: text[line_start..offset].encode_utf16().count() as u32,
        }
    }

    fn link(value: Value) -> Value {
        value.as_array().expect("location link array")[0].clone()
    }

    fn test_path(name: &str) -> PathBuf {
        std::env::current_dir()
            .expect("current directory")
            .join(name)
    }

    #[test]
    fn resolves_local_shadowing_and_parameter_bindings() {
        let text = "package main\nfunc run(value: u32) {\n    var outer: u32 = value\n    {\n        var outer: u32 = 2 as u32\n        outer = outer\n    }\n    outer = value\n}\n";
        let mut sources = SourceDb::default();
        let id = sources.add_file(test_path("locals.ond"), text.to_string());
        let file = compiler::parse_source_file(id, text).expect("parse local test");

        let inner =
            link(definition_for_file(&file, &sources, &position(text, "outer", 2)).unwrap());
        assert_eq!(inner["targetSelectionRange"]["start"]["line"], 4);

        let outer =
            link(definition_for_file(&file, &sources, &position(text, "outer", 4)).unwrap());
        assert_eq!(outer["targetSelectionRange"]["start"]["line"], 2);

        let parameter =
            link(definition_for_file(&file, &sources, &position(text, "value", 2)).unwrap());
        assert_eq!(parameter["targetSelectionRange"]["start"]["line"], 1);
    }

    #[test]
    fn resolves_method_and_receiver_definitions() {
        let text = "package main\ntype Device struct {}\nfunc (device: *Device) Read() -> i32 { _ = device; return 1; }\nfunc main() { var device: *Device; _ = device.Read(); }\n";
        let mut sources = SourceDb::default();
        let id = sources.add_file(test_path("methods.ond"), text.to_string());
        let file = compiler::parse_source_file(id, text).expect("parse method test");

        let method =
            link(definition_for_file(&file, &sources, &position(text, "Read", 1)).unwrap());
        assert_eq!(method["targetSelectionRange"]["start"]["line"], 2);

        let receiver =
            link(definition_for_file(&file, &sources, &position(text, "device", 1)).unwrap());
        assert_eq!(receiver["targetSelectionRange"]["start"]["line"], 2);
    }

    #[test]
    fn resolves_type_names_in_unnamed_function_type_parameters() {
        let text =
            "package main\ntype Value i32\nvar callback: func(Value, label: Value) -> Value\n";
        let mut sources = SourceDb::default();
        let id = sources.add_file(test_path("function-types.ond"), text.to_string());
        let file = compiler::parse_source_file(id, text).unwrap();
        for occurrence in 1..=3 {
            let target = link(
                definition_for_file(&file, &sources, &position(text, "Value", occurrence)).unwrap(),
            );
            assert_eq!(target["targetSelectionRange"]["start"]["line"], 1);
        }
    }

    #[test]
    fn short_declaration_reuses_only_names_from_the_same_scope() {
        let text = "package main\nfunc run() {\n    first := 1 as u32\n    first, second := first, 2 as u32\n    {\n        first, third := second, 3 as u32\n        first = third\n    }\n}\n";
        let mut sources = SourceDb::default();
        let id = sources.add_file(test_path("short-vars.ond"), text.to_string());
        let file = compiler::parse_source_file(id, text).expect("parse short-var test");

        let reused =
            link(definition_for_file(&file, &sources, &position(text, "first", 1)).unwrap());
        assert_eq!(reused["targetSelectionRange"]["start"]["line"], 2);

        let new_in_nested_scope =
            link(definition_for_file(&file, &sources, &position(text, "first", 4)).unwrap());
        assert_eq!(
            new_in_nested_scope["targetSelectionRange"]["start"]["line"],
            5
        );
    }

    #[test]
    fn resolves_struct_field_and_composite_key() {
        let text = "package main\ntype Pair struct {\n    Value: u32\n}\nfunc run() {\n    var pair: Pair = Pair{Value: 1 as u32}\n    pair.Value = 2 as u32\n}\n";
        let mut sources = SourceDb::default();
        let id = sources.add_file(test_path("fields.ond"), text.to_string());
        let file = compiler::parse_source_file(id, text).expect("parse field test");
        for occurrence in [1, 2] {
            let target = link(
                definition_for_file(&file, &sources, &position(text, "Value", occurrence)).unwrap(),
            );
            assert_eq!(target["targetSelectionRange"]["start"]["line"], 2);
        }
    }

    #[test]
    fn resolves_import_string_qualifier_and_exported_member() {
        let main_text = "package main\nimport \"shared\"\nfunc main() {\n    shared.Run()\n}\n";
        let shared_text = "package shared\nfunc Run() {}\n";
        let mut sources = SourceDb::default();
        let main_id = sources.add_file(test_path("main.ond"), main_text.to_string());
        let shared_id = sources.add_file(test_path("shared.ond"), shared_text.to_string());
        let main = compiler::parse_source_file(main_id, main_text).expect("parse main");
        let shared = compiler::parse_source_file(shared_id, shared_text).expect("parse shared");
        let main_package = compiler::ir::ast::Package {
            logical_path: "main".into(),
            files: vec![main.clone()],
        };
        let shared_package = compiler::ir::ast::Package {
            logical_path: "shared".into(),
            files: vec![shared],
        };
        let project = compiler::ir::ast::Project {
            packages: vec![main_package.clone(), shared_package],
        };

        for (needle, occurrence, target_line) in
            [("\"shared\"", 0, 0), ("shared", 1, 0), ("Run", 0, 1)]
        {
            let target = link(
                definition_for_project(
                    &project,
                    &main_package,
                    &main,
                    &sources,
                    &position(main_text, needle, occurrence),
                )
                .unwrap(),
            );
            assert_eq!(target["targetSelectionRange"]["start"]["line"], target_line);
        }
    }

    #[test]
    fn resolves_field_on_a_value_with_an_imported_type() {
        let main_text = "package main\nimport \"shared\"\nfunc use(value: shared.Pair) {\n    value.Value = 1 as u32\n}\n";
        let shared_text = "package shared\ntype Pair struct {\n    Value: u32\n}\n";
        let mut sources = SourceDb::default();
        let main_id = sources.add_file(test_path("imported-field-main.ond"), main_text.to_string());
        let shared_id = sources.add_file(
            test_path("imported-field-shared.ond"),
            shared_text.to_string(),
        );
        let main = compiler::parse_source_file(main_id, main_text).expect("parse main");
        let shared = compiler::parse_source_file(shared_id, shared_text).expect("parse shared");
        let main_package = compiler::ir::ast::Package {
            logical_path: "main".into(),
            files: vec![main.clone()],
        };
        let shared_package = compiler::ir::ast::Package {
            logical_path: "shared".into(),
            files: vec![shared],
        };
        let project = compiler::ir::ast::Project {
            packages: vec![main_package.clone(), shared_package],
        };

        let target = link(
            definition_for_project(
                &project,
                &main_package,
                &main,
                &sources,
                &position(main_text, "Value", 0),
            )
            .unwrap(),
        );
        assert_eq!(target["targetSelectionRange"]["start"]["line"], 2);
        assert!(
            target["targetUri"]
                .as_str()
                .unwrap()
                .contains("imported-field-shared.ond")
        );
    }

    #[test]
    fn resolves_field_on_an_imported_function_result() {
        let main_text =
            "package main\nimport \"shared\"\nfunc use() {\n    _ = shared.NewPair().Value\n}\n";
        let shared_text = "package shared\ntype Pair struct {\n    Value: u32\n}\nfunc NewPair() -> Pair {\n    return Pair{Value: 1 as u32}\n}\n";
        let mut sources = SourceDb::default();
        let main_id =
            sources.add_file(test_path("imported-result-main.ond"), main_text.to_string());
        let shared_id = sources.add_file(
            test_path("imported-result-shared.ond"),
            shared_text.to_string(),
        );
        let main = compiler::parse_source_file(main_id, main_text).expect("parse main");
        let shared = compiler::parse_source_file(shared_id, shared_text).expect("parse shared");
        let main_package = compiler::ir::ast::Package {
            logical_path: "main".into(),
            files: vec![main.clone()],
        };
        let shared_package = compiler::ir::ast::Package {
            logical_path: "shared".into(),
            files: vec![shared],
        };
        let project = compiler::ir::ast::Project {
            packages: vec![main_package.clone(), shared_package],
        };

        let target = link(
            definition_for_project(
                &project,
                &main_package,
                &main,
                &sources,
                &position(main_text, "Value", 0),
            )
            .unwrap(),
        );
        assert_eq!(target["targetSelectionRange"]["start"]["line"], 2);
    }

    #[test]
    fn produces_utf16_ranges() {
        let text = "package main\nfunc run() {\n    var value: u32 = 1 as u32\n    /*\u{1f600}*/ value = value\n}\n";
        let mut sources = SourceDb::default();
        let id = sources.add_file(test_path("unicode.ond"), text.to_string());
        let file = compiler::parse_source_file(id, text).expect("parse unicode test");
        let target =
            link(definition_for_file(&file, &sources, &position(text, "value", 1)).unwrap());
        assert_eq!(target["originSelectionRange"]["start"]["character"], 11);
    }
}
