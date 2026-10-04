//! Inferred array literals become ordinary fixed-size arrays before MIR.
use crate::{ir::hir, semantic::TypeKind, test_support::compile};

fn length(expression: &str) -> u32 {
    let result = compile(
        &format!("package main\nconst A = {expression}\nfunc main() {{}}\n"),
        None,
    )
    .unwrap();
    let ty = result
        .hir
        .packages
        .iter()
        .flat_map(|p| &p.items)
        .find_map(|item| match item {
            hir::Item::Const(item) => Some(item.values[0].ty),
            _ => None,
        })
        .unwrap();
    let Some(TypeKind::Array { length, .. }) = result.hir.types.underlying_kind(ty) else {
        panic!("not an array")
    };
    *length
}

#[test]
fn length_is_maximum_index_plus_one_not_last_index() {
    assert_eq!(length("[...]u8{10, 20, 30}"), 3);
    assert_eq!(length("[...]u8{4: 10, 20}"), 6);
    assert_eq!(length("[...]u8{8: 1, 2: 2, 3}"), 9);
    assert_eq!(length("[...]u8{(1 << 3): 7}"), 9);
    assert_eq!(length("[...]u8{4294967294 as u32: 7}"), u32::MAX);
}

#[test]
fn inferred_arrays_compose_with_constants_fixed_types_and_calls() {
    let result = compile("package main\nconst C = [...][2]u8{[...]u8{4, 7}, [...]u8{2, 3}}\nvar G: [2]u8 = [...]u8{8, 9}\nfunc main() {}\nfunc id(a: [2]u8) -> [2]u8 { return a; }\nfunc result() -> u8 { var a: [2]u8 = id([...]u8{1, 2}); a = [...]u8{5, 6}; return a[1] + G[0] + C[0][1]; }\n", None).unwrap();
    assert_eq!(
        crate::memory_test_vm::execute(&result.mir, "main.result", &[]).unwrap(),
        vec![21]
    );
}

#[test]
fn runtime_elements_keep_source_order_and_holes_are_zero() {
    let result = compile("package main\nvar count: i32\nfunc main() {}\nfunc next() -> i32 { count = count + 1; return count; }\nfunc result() -> i32 { var a = [...]i32{3: next(), 0: next(), next()}; return a[3] * 100 + a[0] * 10 + a[1] + a[2]; }\n", None).unwrap();
    assert_eq!(
        crate::memory_test_vm::execute(&result.mir, "main.result", &[]).unwrap(),
        vec![123]
    );
}

#[test]
fn invalid_inference_is_a_source_error() {
    for (source, message, span) in [
        (
            "var x = [...]u8{}",
            "cannot infer array length from an empty literal",
            "[...]u8{}",
        ),
        (
            "var x = [...]u8{-1: 0}",
            "array literal index out of bounds",
            "-1: 0",
        ),
        (
            "var x = [...]u8{4294967295 as u32: 0}",
            "array literal index out of bounds",
            "4294967295 as u32: 0",
        ),
        (
            "var x = [...]u8{4294967294 as u32: 0, 1}",
            "array literal index out of bounds",
            "1",
        ),
        (
            "var x = [...]u8{2: 0, 2: 1}",
            "duplicate field or index",
            "2: 1",
        ),
        (
            "var i = 2; var x = [...]u8{i: 0}",
            "array literal key must be a compile-time integer",
            "i: 0",
        ),
        (
            "var x = [...]u8{true: 0}",
            "index must be an integer constant",
            "true",
        ),
        (
            "var x = [...]u8{256}",
            "integer literal out of range",
            "256",
        ),
        ("var x: [2]u8 = [...]u8{1}", "type mismatch", "[...]u8{1}"),
        (
            "var x: [...]u8",
            "inferred array length requires a composite literal",
            "[...]u8",
        ),
        (
            "var x = new([...]u8)",
            "inferred array length requires a composite literal",
            "[...]u8",
        ),
        (
            "var x = [...][...]u8{[...]u8{1}}",
            "inferred array length requires a composite literal",
            "[...]u8",
        ),
        ("var x = [. . .]u8{1}", "expected expression", "."),
        ("var x = [....]u8{1}", "expected `]`", "."),
    ] {
        let input = format!("package main\nfunc main() {{ {source}; }}\n");
        if source == "var x = [...][...]u8{[...]u8{1}}" || source == "var x = [. . .]u8{1}" {
            crate::test_support::reject_at(&input, None, message, span, 0);
        } else {
            crate::test_support::reject(&input, None, message, span);
        }
    }
    crate::test_support::reject(
        "package main\ntype T [...]u8\nfunc main() {}\n",
        None,
        "inferred",
        "[...]u8",
    );
    crate::test_support::reject(
        "package main\nfunc f(a: [...]u8) {}\nfunc main() {}\n",
        None,
        "inferred",
        "[...]u8",
    );
}
