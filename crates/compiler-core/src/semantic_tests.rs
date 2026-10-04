use crate::test_support::compile;
use crate::{
    ir::hir,
    semantic::{TypeId, TypeKind},
};

#[test]
fn constants_resolve_across_packages_and_lexical_scopes() {
    let compilation = compile("package main\nimport \"shared\"\nconst Width = shared.Base + 2\ntype Buffer [Width * 2]u8\nfunc main() {}\nfunc value() -> i32 {\nconst x = Width * 3\n{ const x = 1\nvar y = x\n}\nreturn x\n}\n", Some("package shared\nconst Base = 4\nvar Global: i32\nfunc Function() {}\n")).unwrap();
    let root = compilation
        .hir
        .packages
        .iter()
        .find(|p| p.logical_path == ".")
        .unwrap();
    let file = root.files[0].file_id;
    let global = compilation
        .hir
        .resolve_symbol(root.id, file, Some("shared"), "Global")
        .unwrap();
    assert_eq!(global.kind, hir::SymbolKind::Global);
    assert_eq!(
        compilation
            .hir
            .resolve_symbol(root.id, file, Some("shared"), "Function")
            .unwrap()
            .kind,
        hir::SymbolKind::Function
    );
    let buffer = compilation
        .hir
        .resolve_symbol(root.id, file, None, "Buffer")
        .unwrap();
    assert_eq!(buffer.canonical_name, "main.Buffer");
    let buffer_type = root
        .items
        .iter()
        .find_map(|item| match item {
            hir::Item::Type(item) => Some(item.ty),
            _ => None,
        })
        .unwrap();
    assert!(
        matches!(compilation.hir.types.underlying_kind(buffer_type), Some(TypeKind::Array { length: 12, element }) if *element == TypeId::U8)
    );
    let function = root
        .items
        .iter()
        .find_map(|item| match item {
            hir::Item::Func(f) if f.name == "value" => Some(f),
            _ => None,
        })
        .unwrap();
    let hir::Body::Typed(body) = &function.body;
    assert!(
        matches!(body.block.statements.last(), Some(hir::Stmt::Return { values: hir::ValueList::Expressions(values), .. }) if matches!(values[0].kind, hir::ExprKind::Integer(18)))
    );
    crate::mir::validate_project(&compilation.mir).unwrap();
}

#[test]
fn semantic_errors_are_not_pending_bodies() {
    for (body, expected) in [
        ("var x: i32 = true", "bool"),
        ("return missing", "return value count"),
        ("var x = missing", "unknown identifier"),
        ("missing = 1", "unknown identifier"),
        ("const x = 1 / 0", "division by zero"),
        ("const x: u8 = 256", "out of range"),
        ("const x = 1\nx = 2", "constant"),
        ("if 1 {\n}", "bool"),
        ("break", "enclosing loop"),
        ("var x = true + false", "binary operand"),
    ] {
        let error =
            compile(&format!("package main\nfunc main() {{\n{body}\n}}\n"), None).unwrap_err();
        assert!(error.to_string().contains(expected), "{body}: {error}");
        assert!(!error.diagnostics()[0].primary.span.is_synthetic());
    }
}

#[test]
fn reports_cycles_and_private_imports() {
    let error = compile(
        "package main\nconst N = -1\ntype Buffer [N]u8\nfunc main() {}\n",
        None,
    )
    .unwrap_err();
    assert!(error.to_string().contains("array length"));
    let error = compile(
        "package main\nconst A = B\nconst B = A\nfunc main() {}\n",
        None,
    )
    .unwrap_err();
    assert!(error.to_string().contains("cycle"));
    let error = compile(
        "package main\nimport \"shared\"\nconst X = shared.private\nfunc main() {}\n",
        Some("package shared\nconst private = 1\n"),
    )
    .unwrap_err();
    assert!(error.to_string().contains("not exported"));
}

#[test]
fn invalid_body_returns_a_source_located_error() {
    let error = compile("package main\nfunc main() { missing()\n}\n", None).unwrap_err();
    assert!(error.to_string().contains("unknown identifier"));
    assert!(!error.diagnostics()[0].primary.span.is_synthetic());
}

#[test]
fn one_body_reports_multiple_independent_statement_errors() {
    let source =
        "package main\nfunc main() {\nvar first: i32 = true\nfirst = 1\nvar second: bool = 1\n}\n";
    let error = compile(source, None).expect_err("both invalid declarations must be rejected");
    let diagnostics = error.diagnostics();
    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("boolean literal")),
        "{diagnostics:?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("integer literal")),
        "{diagnostics:?}"
    );
}

#[test]
fn unused_generic_bodies_report_all_unknown_local_names_at_definition() {
    let source = r#"package main
type Box[T] struct { value: T }
func (box: *Box[T]) Broken(value: T) {
    list.value = value
    box.value = item
}
func main() {}
"#;
    let error = compile(source, None).expect_err("unused generic bodies must be name checked");
    let diagnostics = error.diagnostics();
    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.message == "unknown identifier `list`"
            && &source[diagnostic.primary.span.start..diagnostic.primary.span.end] == "list"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.message == "unknown identifier `item`"
            && &source[diagnostic.primary.span.start..diagnostic.primary.span.end] == "item"
    }));
}

#[test]
fn generic_definition_and_specialization_errors_are_reported_together() {
    let source = r#"package main
type List[T] interface { Get(index: u32) -> *T; Set(index: u32, value: T); }
type listImpl[T] struct { data: *T }
func (l: *listImpl[T]) Get(index: u32) -> *T { return &list.data[index] }
func (l: *listImpl[T]) Set(index: u32, item: T) { list.data[index] = value }
func NewList[T]() -> List[T] {
    list := new(listImpl[T])
    return list
}
func main() { _ = NewList[u32]() }
"#;
    let error = compile(source, None).expect_err("all independent generic errors must be reported");
    let diagnostics = error.diagnostics();
    assert_eq!(diagnostics.len(), 4, "{diagnostics:?}");
    for name in ["list", "value"] {
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message == format!("unknown identifier `{name}`")),
            "{diagnostics:?}"
        );
    }
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.message == "unknown identifier `list`")
            .count(),
        2,
        "{diagnostics:?}"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("pointer-to-interface conversion")),
        "{diagnostics:?}"
    );
}
