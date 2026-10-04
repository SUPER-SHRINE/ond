use compiler::ir::ast;
use compiler::source::{SourceDb, Span};
use serde_json::{Value, json};

pub fn document_symbols_for_file(file: &ast::File, sources: &SourceDb) -> Value {
    Value::Array(
        file.decls
            .iter()
            .flat_map(|decl| symbols_for_top_level_decl(decl, sources))
            .collect(),
    )
}

fn symbols_for_top_level_decl(decl: &ast::TopLevelDecl, sources: &SourceDb) -> Vec<Value> {
    match decl {
        ast::TopLevelDecl::Const(decl) => decl
            .specs
            .iter()
            .flat_map(|spec| {
                spec.names
                    .iter()
                    .map(|name| make_document_symbol(&name.name, 14, name.span, spec.span, sources))
            })
            .collect(),
        ast::TopLevelDecl::Var(decl) => decl
            .specs
            .iter()
            .flat_map(|spec| {
                spec.names
                    .iter()
                    .map(|name| make_document_symbol(&name.name, 13, name.span, spec.span, sources))
            })
            .collect(),
        ast::TopLevelDecl::Type(decl) => decl
            .specs
            .iter()
            .map(|spec| {
                make_document_symbol(&spec.name.name, 5, spec.name.span, spec.span, sources)
            })
            .collect(),
        ast::TopLevelDecl::Func(decl) => vec![make_document_symbol(
            &decl.name.name,
            12,
            decl.name.span,
            decl.span,
            sources,
        )],
        ast::TopLevelDecl::Operator(_) => Vec::new(),
    }
}

fn make_document_symbol(
    name: &str,
    kind: u32,
    selection_span: Span,
    full_span: Span,
    sources: &SourceDb,
) -> Value {
    json!({
        "name": name,
        "kind": kind,
        "range": span_to_range(full_span, sources),
        "selectionRange": span_to_range(selection_span, sources)
    })
}

pub fn to_lsp_diagnostic(diagnostic: &compiler::Diagnostic, sources: &SourceDb) -> Value {
    let range = if diagnostic.primary.span.is_synthetic() {
        zero_range()
    } else {
        span_to_range(diagnostic.primary.span, sources)
    };

    let related_information = diagnostic
        .related
        .iter()
        .filter(|related| !related.span.is_synthetic())
        .map(|related| {
            let file = sources.file(related.span.file);
            json!({
                "location": {
                    "uri": crate::util::uri_from_path(file.path()).unwrap_or_default(),
                    "range": span_to_range(related.span, sources)
                },
                "message": related.message
            })
        })
        .collect::<Vec<_>>();

    json!({
        "range": range,
        "severity": 1,
        "source": "ond-lsp",
        "message": diagnostic.message,
        "relatedInformation": related_information
    })
}

pub fn span_to_range(span: Span, sources: &SourceDb) -> Value {
    let file = sources.file(span.file);
    let (start_line, start_byte_col) = file.line_col(span.start);
    let (end_line, end_byte_col) = file.line_col(span.end);
    let start_line_offset = span.start.saturating_sub(start_byte_col.saturating_sub(1));
    let end_line_offset = span.end.saturating_sub(end_byte_col.saturating_sub(1));
    let start_col = file.text()[start_line_offset..span.start]
        .encode_utf16()
        .count();
    let end_col = file.text()[end_line_offset..span.end]
        .encode_utf16()
        .count();

    json!({
        "start": {
            "line": start_line.saturating_sub(1),
            "character": start_col
        },
        "end": {
            "line": end_line.saturating_sub(1),
            "character": end_col
        }
    })
}

fn zero_range() -> Value {
    json!({
        "start": {
            "line": 0,
            "character": 0
        },
        "end": {
            "line": 0,
            "character": 0
        }
    })
}

#[cfg(test)]
mod tests {
    use compiler::Diagnostic;
    use compiler::source::{SourceDb, Span};

    use super::to_lsp_diagnostic;

    #[test]
    fn diagnostics_include_related_source_locations() {
        let mut sources = SourceDb::default();
        let root = std::env::temp_dir().join("ond-lsp-related-diagnostic");
        let use_file = sources.add_file(root.join("main.ond"), "use".into());
        let definition_file = sources.add_file(root.join("generic.ond"), "definition".into());
        let diagnostic = Diagnostic::error(Span::new(use_file, 0, 3), "invalid instance")
            .with_related(
                Span::new(definition_file, 0, 10),
                "generic definition failed here",
            );

        let value = to_lsp_diagnostic(&diagnostic, &sources);
        assert_eq!(value["range"]["end"]["character"], 3);
        assert_eq!(
            value["relatedInformation"][0]["message"],
            "generic definition failed here"
        );
        assert!(
            value["relatedInformation"][0]["location"]["uri"]
                .as_str()
                .unwrap()
                .ends_with("generic.ond")
        );
    }
}
