//! Source-to-MIR regression tests for mandatory constant evaluation.
use crate::{ir::hir, test_support::compile};

fn value(declarations: &str, expression: &str) -> hir::Expr {
    let result = compile(
        &format!("package main\n{declarations}\nconst Result = {expression}\nfunc main() {{}}\n"),
        None,
    )
    .unwrap();
    result
        .hir
        .packages
        .iter()
        .flat_map(|p| &p.items)
        .find_map(|item| match item {
            hir::Item::Const(item) if item.names.iter().any(|name| name == "Result") => {
                Some(item.values[0].clone())
            }
            _ => None,
        })
        .unwrap()
}

fn integer(declarations: &str, expression: &str, expected: u64) {
    assert!(
        matches!(value(declarations, expression).kind, hir::ExprKind::Integer(actual) if actual == expected)
    );
}

#[test]
fn exact_integer_intermediates_are_not_limited_to_i128() {
    let product = std::iter::repeat_n("2000000000", 6)
        .collect::<Vec<_>>()
        .join(" * ");
    integer("", &format!("({product}) / ({product})"), 1);
    integer("const X: u8 = (250 + 10) - 10", "X", 250);
    integer("", "(-2147483648) % -1", 0);
}

#[test]
fn shifts_use_lhs_width_and_exact_left_shift() {
    integer("const X: u8 = 1 << 8", "X", 1);
    integer("const X: u16 = 3 << 33", "X", 6);
    integer("const X: i8 = -8 >> 9", "X", (-4i64) as u64);
    integer("", "1 << 4294967295 >> 31", 1);
}

#[test]
fn nested_aggregate_constants_and_implicit_zeros() {
    let declarations = "type S struct { xs: [3]u8; label: [3]u8; p: *u8; }\nconst A = S{xs: [3]u8{1: 5 + 2}, label: \"abc\"}";
    integer(declarations, "A.xs[1]", 7);
    integer(declarations, "A.xs[0]", 0);
    integer(declarations, "len(A.label)", 3);
    integer(declarations, "A.label[2]", 99);
    assert!(matches!(
        value(declarations, "A.p").kind,
        hir::ExprKind::Nil
    ));
    assert!(matches!(
        value(declarations, "A.p == nil").kind,
        hir::ExprKind::Bool(true)
    ));
    integer(
        "const Huge = [4294967295]u8{4294967294 as u32: 42}",
        "Huge[4294967294 as u32]",
        42,
    );
    integer(
        "const Keys = [1]i32{2}\nconst A = [3]u8{Keys[0]: 9}",
        "A[2]",
        9,
    );
}

#[test]
fn aggregate_constants_reach_mir_as_independent_values() {
    let result = compile("package main\ntype S struct { x: i32; }\nconst C = [2]S{S{x: 7}}\nfunc main() {}\nfunc result() -> i32 { var a = C; a[0].x = 9; var b = C; return b[0].x; }\n", None).unwrap();
    assert_eq!(
        crate::memory_test_vm::execute(&result.mir, "main.result", &[]).unwrap(),
        vec![7]
    );
}

#[test]
fn casts_and_indices_emit_the_checked_exact_result() {
    let result = compile("package main\nfunc main() {}\nfunc result() -> i32 { var a = [2]i32{4, 7}; var x = ((2000000000 * 2) / 2000000000) as i32; return a[((2000000000 * 2) / 2000000000) - 1] + x; }\n", None).unwrap();
    assert_eq!(
        crate::memory_test_vm::execute(&result.mir, "main.result", &[]).unwrap(),
        vec![9]
    );
}

#[test]
fn invalid_constants_produce_source_diagnostics() {
    for declaration in [
        "const X: u8 = 128 << 1",
        "const X = 1 << (4294967295 + 1)",
        "const X = 1 / 0",
        "const X = (-2147483648) / -1",
        "const X = [1]u8{250 + 6}",
        "const X = false && f()",
        "const X = [1]bool{f()}",
        "const X = new(u8)",
        "const X: *u8 = 1 as *u8",
        "const X = nil",
    ] {
        let error = compile(&format!("package main\n{declaration}\nfunc f() -> bool {{ return true; }}\nfunc main() {{}}\n"), None).unwrap_err();
        assert!(!error.diagnostics().is_empty(), "{declaration}");
        assert!(
            !format!("{error:?}").contains("internal MIR"),
            "{declaration}: {error:?}"
        );
    }
}

#[test]
fn float_operations_keep_binary32_and_canonical_nan() {
    assert!(matches!(
        value("", "16777216.0 + 1.0 - 16777216.0").kind,
        hir::ExprKind::Float32(0)
    ));
    assert!(matches!(
        value("", "-(0.0 / 0.0)").kind,
        hir::ExprKind::Float32(0x7fc00000)
    ));
    assert!(matches!(
        value("", "-0.0").kind,
        hir::ExprKind::Float32(0x80000000)
    ));
    assert!(matches!(
        value("", "1.0 / 0.0").kind,
        hir::ExprKind::Float32(0x7f800000)
    ));
}
