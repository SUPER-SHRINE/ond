use crate::{ir::hir, memory_test_vm::execute, test_support::compile};

fn result(source: &str) -> Vec<u64> {
    let compilation = compile(source, None).unwrap();
    crate::mir::validate_project(&compilation.mir).unwrap();
    assert!(compilation.hir.symbols.iter().all(|s| s.name != "_"));
    assert!(
        compilation
            .mir
            .packages
            .iter()
            .flat_map(|p| &p.globals)
            .all(|g| !g.symbol.ends_with("._"))
    );
    execute(&compilation.mir, "main.result", &[]).unwrap()
}

#[test]
fn discards_preserve_calls_and_multi_value_positions() {
    assert_eq!(
        result(
            r#"package main
var Log: i32
func mark(v: i32) -> i32 { Log = Log * 10 + v; return v }
func pair() -> (i32, i32) { return mark(1), mark(2) }
func main() {}
func result() -> (i32, i32) {
    var _, a = pair()
    _, b := pair()
    a, _ = pair()
    var _, _: i32 = mark(3), mark(4)
    return a + b, Log
}
"#
        ),
        [3, 12121234]
    );
}

#[test]
fn discarded_globals_still_require_compile_time_values() {
    crate::test_support::reject(
        "package main\nvar _ = mark(1)\nfunc mark(v: i32) -> i32 { return v; }\nfunc main() {}\n",
        None,
        "compile-time value",
        "mark(1)",
    );
}

#[test]
fn blank_parameters_keep_argument_slots_but_no_binding() {
    assert_eq!(
        result(
            r#"package main
func choose(_: i32, value: i32, _: bool) -> i32 { return value }
func main() {}
func result() -> i32 { return choose(10, 42, true) }
"#
        ),
        [42]
    );
}

#[test]
fn short_declarations_reuse_only_current_scope_variables() {
    assert_eq!(
        result(
            r#"package main
func main() {}
func result() -> (i32, i32) {
    a := 1
    { a, _ := 2, true; a, b := 3, 4; _ = b }
    a, b := 5, 6
    const _, _ = 1, true
    const _: *u8 = nil
    var _: func() = nil
    return a, b
}
"#
        ),
        [5, 6]
    );
}

#[test]
fn discard_never_suppresses_evaluation_faults() {
    let c = compile(
        "package main\nfunc main() {}\nfunc result() { var z: i32; _ = 1 / z }\n",
        None,
    )
    .unwrap();
    assert_eq!(
        execute(&c.mir, "main.result", &[]).unwrap_err(),
        "division by zero"
    );
}

#[test]
fn blank_local_declarations_do_not_allocate_local_slots() {
    let c = compile(
        "package main\nfunc main() { var _, _ = 1, true; _, a := 2, 3; _ = a }\n",
        None,
    )
    .unwrap();
    let hir::Item::Func(f) = &c.hir.packages[0].items[0] else {
        panic!()
    };
    let hir::Body::Typed(body) = &f.body;
    assert_eq!(body.locals.len(), 1);
    assert_eq!(body.locals[0].name, "a");
}

#[test]
fn invalid_blank_uses_and_short_declarations_are_rejected() {
    for (body, message, span) in [
        (
            "_ := 1",
            "short declaration requires at least one new name",
            "_ := 1",
        ),
        (
            "_, _ := 1, 2",
            "short declaration requires at least one new name",
            "_, _ := 1, 2",
        ),
        (
            "a := 1; a, _ := 2, 3",
            "short declaration requires at least one new name",
            "a, _ := 2, 3",
        ),
        (
            "a := 1; a, a, b := 2, 3, 4",
            "duplicate name in short declaration",
            "a",
        ),
        ("a, a := 1, 2", "duplicate name in short declaration", "a"),
        (
            "var _ = 1; var a = _",
            "blank identifier `_` cannot be used as a value",
            "_",
        ),
        (
            "const _ = 1; var a = _",
            "blank identifier `_` cannot be used as a value",
            "_",
        ),
        ("var _: u8 = 256", "integer literal out of range", "256"),
        (
            "const _: i32 = missing()",
            "constant initializer requires a compile-time value",
            "missing()",
        ),
        (
            "_ = nil",
            "requires an explicit pointer or function type context",
            "nil",
        ),
        ("_, _ = 1", "assignment value count mismatch", "_, _ = 1"),
        (
            "var _, _ = 1",
            "declaration value count mismatch",
            "_, _ = 1",
        ),
        (
            "const _, _ = 1",
            "constant declaration value count mismatch",
            "_, _ = 1",
        ),
        (
            "const a = 1; a, b := 2, 3",
            "cannot assign to a constant",
            "a, b := 2, 3",
        ),
    ] {
        let source = format!("package main\nfunc main() {{ {body} }}\n");
        crate::test_support::reject(&source, None, message, span);
    }
    for (declaration, message, span) in [
        ("var _: u8 = 256", "integer literal out of range", "256"),
        (
            "const _ = missing()",
            "constant initializer requires a compile-time value",
            "missing()",
        ),
        (
            "const _, _ = 1",
            "constant declaration value count mismatch",
            "_, _ = 1",
        ),
        (
            "var _, _ = 1",
            "declaration value count mismatch",
            "_, _ = 1",
        ),
    ] {
        crate::test_support::reject(
            &format!("package main\n{declaration}\nfunc main() {{}}\n"),
            None,
            message,
            span,
        );
    }
    crate::test_support::reject(
        "package main\nfunc f(_: i32) -> i32 { return _ }\nfunc main() {}\n",
        None,
        "blank identifier `_` cannot be used as a value",
        "_",
    );
}

#[test]
fn delimiters_empty_statements_and_trailing_call_comma_are_supported() {
    assert_eq!(
        result(
            r#"package main
const (A = 1)
var (B: i32)
type (S struct { Value: i32 })
func id(x: i32) -> i32 { return x }
func main() { ;; }
func result() -> i32 {
    ; var (x = A); const (C = 2)
    for ; x < 3; x = x + 1 { ; }
    for ;; { break }
    if ; x == 3 { x = x + C }
    return id(x,)
}
"#
        ),
        [5]
    );
}

#[test]
fn call_statements_are_allowed_in_control_flow_clauses() {
    assert_eq!(
        result(
            r#"package main
var N: i32
func step() { N = N + 1 }
func main() {}
func result() -> i32 {
    if step(); N == 1 { step() }
    for step(); N < 5; step() {}
    return N
}
"#
        ),
        [5]
    );
}

#[test]
fn non_call_expression_statements_and_missing_separators_are_rejected() {
    for (body, message, span) in [
        (
            "1 + 2",
            "expression statement must be a function call",
            "1 + 2",
        ),
        (
            "true",
            "expression statement must be a function call",
            "true",
        ),
        (
            "new(i32)",
            "expression statement must be a function call",
            "new(i32)",
        ),
        (
            "f() + 1",
            "expression statement must be a function call",
            "f() + 1",
        ),
        (
            "f() as i32",
            "expression statement must be a function call",
            "f() as i32",
        ),
        ("if 1; true {}", "expected simple statement", "1"),
        ("for 1; true; f() {}", "expected simple statement", "1"),
        (
            "for ; true; 1 {}",
            "expression statement must be a function call",
            "1",
        ),
        ("var a = 1 var b = 2", "expected `;` or newline", "var"),
        ("f() f()", "expected `;` or newline", "f"),
        ("for var a = 1; true; f() {}", "expected expression", "var"),
        ("f(,)", "expected expression", ","),
        ("f(1,,)", "expected expression", ","),
    ] {
        let source =
            format!("package main\nfunc f() -> i32 {{ return 1 }}\nfunc main() {{ {body} }}\n");
        crate::test_support::reject(&source, None, message, span);
    }
}
