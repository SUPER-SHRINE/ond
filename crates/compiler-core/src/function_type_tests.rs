//! Names in function types are optional documentation; declaration names are mandatory.
use crate::{ir::ast, source::FileId, test_support::compile};

#[test]
fn ast_preserves_optional_names_and_type_spans() {
    let source = "package main\ntype Callback func(i32, label: *u8, [2]i32, func(bool) -> bool,) -> (i32, bool,)\n";
    let file = crate::parse_source_file(FileId(0), source).unwrap();
    let ast::TopLevelDecl::Type(decl) = &file.decls[0] else {
        panic!("expected type")
    };
    let ast::Type::Func { signature, .. } = &decl.specs[0].ty else {
        panic!("expected function type")
    };
    assert_eq!(signature.params.len(), 4);
    assert!(signature.params[0].name.is_none());
    assert_eq!(signature.params[1].name.as_ref().unwrap().name, "label");
    assert!(signature.params[2].name.is_none());
    assert_eq!(
        &source[signature.params[0].span.start..signature.params[0].span.end],
        "i32"
    );
    assert_eq!(
        &source[signature.params[1].span.start..signature.params[1].span.end],
        "label: *u8"
    );
    assert_eq!(signature.results.len(), 2);
}

#[test]
fn named_unnamed_and_mixed_signatures_have_identical_types() {
    compile("package main\nfunc main() { var a: func(i32, bool) -> i32; var b: func(x: i32, bool) -> i32; var c: func(i32, flag: bool) -> i32; var d: func(x: i32, x: bool) -> i32; a = b; b = c; c = d; }\n", None).unwrap();
    compile("package main\nfunc main() { var a: func(*u8, [2]i32, struct { X: bool; }, func(i32) -> bool); var b: func(p: *u8, a: [2]i32, s: struct { X: bool; }, cb: func(n: i32) -> bool); a = b; }\n", None).unwrap();
}

#[test]
fn callback_arguments_returns_and_indirect_calls_reach_valid_mir() {
    compile("package main\nfunc identity(x: i32) -> i32 { return x; }\nfunc choose() -> func(i32) -> i32 { return identity; }\nfunc apply(f: func(i32) -> i32, x: i32) -> i32 { return f(x); }\nfunc main() { var callback: func(i32) -> i32 = choose(); var x = apply(callback, 7); }\n", None).unwrap();
    compile("package main\nimport \"shared\"\nvar callback: func(shared.T) -> shared.T\nfunc main() {}\n", Some("package shared\ntype T i32\n")).unwrap();
}

#[test]
fn type_parameter_names_are_not_bindings_and_declarations_still_need_names() {
    for (source, message, span) in [
        ("func f(i32) {}", "builtin type name `i32`", "i32"),
        (
            "func f(x: i32, bool) {}",
            "builtin type name `bool`",
            "bool",
        ),
        ("func f(*u8) {}", "expected identifier", "*"),
        (
            "func f() { var cb: func(label: i32); var x = label; }",
            "unknown identifier `label`",
            "label",
        ),
        (
            "func f() { var cb: func(i32,); var x = cb(1, 2); }",
            "function expects 1 arguments, found 2",
            "cb(1, 2)",
        ),
        (
            "func f() { var cb: func(i32); cb(true); }",
            "boolean literal requires bool type",
            "true",
        ),
        ("func f() { var cb: func(: i32); }", "expected type", ":"),
        (
            "func f() { var cb: func(i32 bool); }",
            "expected `)`",
            "bool",
        ),
    ] {
        crate::test_support::reject(
            &format!("package main\n{source}\nfunc main() {{}}\n"),
            None,
            message,
            span,
        );
    }
    compile(
        "package main\nfunc f(_: i32) {}\nfunc main() { f(1); }\n",
        None,
    )
    .unwrap();
}
