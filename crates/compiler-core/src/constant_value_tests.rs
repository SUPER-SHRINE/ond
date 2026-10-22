//! Mandatory evaluation at runtime-value boundaries, without variable propagation.
use crate::{
    mir::{BinaryOp, InstructionKind},
    test_support::compile,
};

#[test]
fn constant_composites_reject_skipped_calls_before_value_folding() {
    let mut accepted = Vec::new();
    for logical in ["false && flag()", "true || flag()"] {
        for expression in [
            logical.to_string(),
            format!("[1]bool{{{logical}}}"),
            format!("len([1]bool{{{logical}}})"),
            format!("S{{value: {logical}}}"),
        ] {
            for local in [false, true] {
                let declaration = format!("const Value = {expression}");
                let (package, body) = if local {
                    ("", declaration.as_str())
                } else {
                    (declaration.as_str(), "")
                };
                let source = format!(
                    "package main\ntype S struct {{ value: bool; }}\n{package}\nfunc flag() -> bool {{ return true; }}\nfunc main() {{ {body}; }}\n"
                );
                match compile(&source, None) {
                    Ok(_) => accepted.push(format!("local={local}: {expression}")),
                    Err(error) => assert!(
                        error
                            .to_string()
                            .contains("constant initializer requires a compile-time value"),
                        "local={local}: {expression}: {error}"
                    ),
                }
            }
        }
    }
    assert!(
        accepted.is_empty(),
        "accepted invalid constants: {accepted:?}"
    );
}

#[test]
fn constant_composites_skip_rhs_value_errors() {
    for rhs in ["1 / 0 == 0", "(250 + 6) as u8 == 0"] {
        for (logical, expected) in [
            (format!("false && ({rhs})"), 0),
            (format!("true || ({rhs})"), 1),
        ] {
            let source = format!(
                "package main\ntype S struct {{ value: bool; }}\nconst A = [1]bool{{{logical}}}\nconst B = S{{value: {logical}}}\nconst Size = len([1]bool{{{logical}}})\nfunc main() {{}}\nfunc result() -> (bool, bool, bool, bool, u32, u32) {{ const LocalA = [1]bool{{{logical}}}; const LocalB = S{{value: {logical}}}; const LocalSize = len([1]bool{{{logical}}}); return A[0], LocalA[0], B.value, LocalB.value, Size, LocalSize; }}\n"
            );
            let result = compile(&source, None).unwrap();
            assert_eq!(
                crate::memory_test_vm::execute(&result.mir, "main.result", &[]).unwrap(),
                [expected, expected, expected, expected, 1, 1],
                "{logical}"
            );
        }
    }
}

#[test]
fn skipped_composite_rhs_still_requires_valid_names_and_types() {
    for rhs in ["missing()", "1", "true as u8 == 0"] {
        for logical in [format!("false && ({rhs})"), format!("true || ({rhs})")] {
            for expression in [
                format!("[1]bool{{{logical}}}"),
                format!("len([1]bool{{{logical}}})"),
            ] {
                for declaration in ["const", "var"] {
                    let source = format!(
                        "package main\nfunc main() {{ {declaration} Value = {expression}; }}\n"
                    );
                    assert!(
                        compile(&source, None).is_err(),
                        "{declaration} Value = {expression}"
                    );
                }
            }
        }
    }
}

#[test]
fn runtime_composite_logical_values_preserve_effects_and_source_order() {
    let result = compile(
        r#"package main
func main() {}
func flag(trace: *u32, digit: u32, value: bool) -> bool {
    *trace = *trace * 10 + digit
    return value;
}
func result() -> (bool, bool, bool, bool, bool, bool, u32, u32, bool, bool) {
    var trace: u32
    var values = [4]bool{
        3: false && flag(&trace, 9, true),
        1: true || flag(&trace, 9, false),
        2: flag(&trace, 1, true) && flag(&trace, 2, false),
        0: flag(&trace, 3, false) || flag(&trace, 4, true),
    }
    var size = len([2]bool{
        1: flag(&trace, 5, false) && flag(&trace, 9, true),
        0: flag(&trace, 6, true) || flag(&trace, 9, false),
    })
    var skippedAnd = false && (len([1]bool{flag(&trace, 9, true)}) == 1)
    var skippedOr = true || (len([1]bool{flag(&trace, 9, false)}) == 1)
    var evaluatedAnd = true && (len([1]bool{flag(&trace, 7, true)}) == 1)
    var evaluatedOr = false || (len([1]bool{flag(&trace, 8, false)}) == 1)
    return values[0], values[1], values[2], values[3], skippedAnd, skippedOr, size, trace, evaluatedAnd, evaluatedOr;
}
"#,
        None,
    )
    .unwrap();
    assert_eq!(
        crate::memory_test_vm::execute(&result.mir, "main.result", &[]).unwrap(),
        [1, 1, 0, 0, 0, 1, 2, 12345678, 1, 1]
    );
}

#[test]
fn skipped_casts_at_value_boundaries_never_reach_mir() {
    let c = compile(
        r#"package main
var G = false
func main() {}
func f(x: u8) -> bool { return true; }
func id(x: bool) -> bool { return x; }
func result() -> (bool, bool, bool, bool) {
    var x = false && ((250 + 6) as u8 == 0)
    x = true || f((250 + 6) as u8)
    var y = id(false && f((250 + 6) as u8))
    return G, x, y, true || f((250 + 6) as u8);
}
"#,
        None,
    )
    .unwrap();
    assert_eq!(
        crate::memory_test_vm::execute(&c.mir, "main.result", &[]).unwrap(),
        [0, 1, 0, 1]
    );
    for function in c
        .mir
        .packages
        .iter()
        .flat_map(|p| p.functions.iter().chain(p.initializer.iter()))
    {
        for instruction in function.blocks.iter().flat_map(|b| &b.instructions) {
            assert!(!matches!(instruction.kind, InstructionKind::Cast { .. }));
            if let InstructionKind::Call {
                callee: crate::mir::Callee::Direct(name),
                ..
            } = &instruction.kind
            {
                assert_ne!(name, "main.f");
            }
        }
    }
}

#[test]
fn short_circuit_deferral_does_not_leak_or_propagate_variables() {
    for body in [
        "var x = false && ((250 + 6) as u8 == 0); var y = (250 + 6) as u8",
        "var flag = false; var x = flag && ((250 + 6) as u8 == 0)",
        "var x = true && ((250 + 6) as u8 == 0)",
        "var x = false || ((250 + 6) as u8 == 0)",
        "var x = ((250 + 6) as u8 == 0) && false",
    ] {
        let source = format!("package main\nfunc main() {{ {body}; }}\n");
        let error = compile(&source, None).unwrap_err();
        assert!(
            error.to_string().contains("out of range"),
            "{body}: {error}"
        );
    }
}

#[test]
fn rejects_overflow_at_all_value_boundaries() {
    for body in [
        "var x: u8 = 128 << 1",
        "var x: u8; x = 128 << 1",
        "var x: u8; x, y := 128 << 1, 0",
        "take(128 << 1)",
        "var f = take; f(128 << 1)",
        "return 128 << 1",
        "var x = [1]u8{128 << 1}",
        "var x = S{x: 128 << 1}",
        "var x = 2147483647 + 1",
        "_ = 2147483647 + 1",
        "*(0 as u32 as *u32) = 4294967295 + 1",
        "var x: u8 = 1; x = x + (128 << 1)",
    ] {
        let source = format!(
            "package main\ntype S struct {{ x: u8; }}\nfunc take(x: u8) {{}}\nfunc main() {{}}\nfunc test() -> u8 {{ {body}; return 0; }}\n"
        );
        let error = compile(&source, None).unwrap_err();
        assert!(
            error
                .diagnostics()
                .iter()
                .any(|d| d.message.contains("out of range")),
            "{body}: {error:?}"
        );
    }
    let error = compile("package main\nvar X: u8 = 128 << 1\nfunc main() {}\n", None).unwrap_err();
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|d| d.message.contains("out of range"))
    );
}

#[test]
fn exact_values_flow_through_globals_assignments_arguments_and_returns() {
    let expr = "(2000000000 * 2000000000 * 2000000000 * 2000000000 * 2000000000) / (2000000000 * 2000000000 * 2000000000 * 2000000000 * 2000000000)";
    let source = format!(
        "package main\nvar G = {expr}\nfunc main() {{}}\nfunc id(x: i32) -> i32 {{ return x; }}\nfunc result() -> i32 {{ var x: i32; x = {expr}; var y = id({expr}); var a = [1]i32{{{expr}}}; return G + x + y + a[0] + ({expr}); }}\n"
    );
    let result = compile(&source, None).unwrap();
    assert_eq!(
        crate::memory_test_vm::execute(&result.mir, "main.result", &[]).unwrap(),
        vec![5]
    );
    assert!(
        !result
            .mir
            .packages
            .iter()
            .flat_map(|p| &p.functions)
            .flat_map(|f| &f.blocks)
            .flat_map(|b| &b.instructions)
            .any(|i| matches!(
                i.kind,
                InstructionKind::Binary {
                    op: BinaryOp::Multiply,
                    ..
                }
            ))
    );
}

#[test]
fn runtime_variables_are_not_promoted_to_constants() {
    let result = compile("package main\nvar G: u8 = 128\nfunc main() {}\nfunc test() -> u8 { var x: u8 = 128; var y = x << 1; return y + G; }\n", None).unwrap();
    assert!(
        result
            .mir
            .packages
            .iter()
            .flat_map(|p| &p.functions)
            .flat_map(|f| &f.blocks)
            .flat_map(|b| &b.instructions)
            .any(|i| matches!(
                i.kind,
                InstructionKind::Binary {
                    op: BinaryOp::ShiftLeft,
                    ..
                }
            ))
    );
    // The MIR shift remains typed u8, preserving its runtime wrap contract.
    let result = compile(
        "package main\nfunc main() {}\nfunc test() -> u8 { return (250 + 10) - 10; }\n",
        None,
    )
    .unwrap();
    assert_eq!(
        crate::memory_test_vm::execute(&result.mir, "main.test", &[]).unwrap(),
        vec![250]
    );
}

#[test]
fn short_circuit_skips_rhs_while_array_len_evaluates_its_operand() {
    let result = compile("package main\nfunc main() {}\nfunc array() -> [2]u8 { return [2]u8{}; }\nfunc test() -> bool { if (1 + 1) == 2 { return false && (1 / 0 == 0); }; return true; }\nfunc size() -> u32 { return len(array()); }\n", None).unwrap();
    assert_eq!(
        crate::memory_test_vm::execute(&result.mir, "main.test", &[]).unwrap(),
        vec![0]
    );
    assert_eq!(
        crate::memory_test_vm::execute(&result.mir, "main.size", &[]).unwrap(),
        vec![2]
    );
}

#[test]
fn array_len_evaluates_and_typechecks_nested_values() {
    for expression in ["len([1]i32{1 / 0})", "len(array(1 / 0))"] {
        let source = format!(
            "package main\nfunc main() {{}}\nfunc array(x: i32) -> [2]u8 {{ return [2]u8{{}}; }}\nconst Size = {expression}\n"
        );
        let error = compile(&source, None).unwrap_err();
        assert!(
            error
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.message.contains("division by zero"))
        );
    }
    crate::test_support::reject(
        "package main\nfunc main() { var x = len([1]i32{true}); }\n",
        None,
        "boolean literal requires bool type",
        "true",
    );
}

#[test]
fn array_len_rejects_literal_cast_overflow_at_value_boundaries() {
    for body in [
        "const Size = len([1]u8{(250 + 6) as u8})",
        "var size = len([1]u8{(250 + 6) as u8})",
        "return len([1]u8{(250 + 6) as u8})",
    ] {
        let source = format!(
            "package main\nfunc main() {{}}\nfunc result() -> u32 {{ {body}; return 0; }}\n"
        );
        let error = compile(&source, None).unwrap_err();
        assert!(
            error.to_string().contains("constant cast is out of range"),
            "{body}: {error}"
        );
    }
}

#[test]
fn short_circuit_skips_casts_inside_and_around_array_len() {
    for (expression, expected) in [
        ("len([1]bool{false && ((250 + 6) as u8 == 0)}) == 1", 1),
        ("len([1]bool{true || ((250 + 6) as u8 == 0)}) == 1", 1),
        ("false && (len([1]u8{(250 + 6) as u8}) == 1)", 0),
        ("true || (len([1]u8{(250 + 6) as u8}) == 1)", 1),
    ] {
        let source = format!(
            "package main\nconst Value = {expression}\nfunc main() {{}}\nfunc result() -> (bool, bool, bool) {{ var value = {expression}; return Value, value, {expression}; }}\n"
        );
        let result = compile(&source, None).unwrap();
        assert_eq!(
            crate::memory_test_vm::execute(&result.mir, "main.result", &[]).unwrap(),
            [expected; 3],
            "{expression}"
        );
    }
}

#[test]
fn array_len_rejects_direct_operand_calls_in_constants_even_when_len_is_skipped() {
    for expression in [
        "len(array())",
        "false && (len(array()) == 1)",
        "true || (len(array()) == 1)",
    ] {
        let source = format!(
            "package main\nconst Value = {expression}\nfunc main() {{}}\nfunc array() -> [1]u8 {{ return [1]u8{{}}; }}\n"
        );
        let error = compile(&source, None).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("constant initializer requires a compile-time value"),
            "{expression}: {error}"
        );
    }
}

#[test]
fn array_len_preserves_one_operand_side_effect_and_runtime_faults() {
    let result = compile(
        r#"package main
func main() {}
func array(calls: *u32, index: u32) -> [2]u8 {
    *calls += 1
    var values = [1]u8{}
    _ = values[index]
    return [2]u8{};
}
func result(index: u32) -> (u32, u32) {
    var calls: u32
    var size = len(array(&calls, index))
    return size, calls;
}
"#,
        None,
    )
    .unwrap();
    assert_eq!(
        crate::memory_test_vm::execute(&result.mir, "main.result", &[0]).unwrap(),
        [2, 1]
    );
    assert_eq!(
        crate::memory_test_vm::execute(&result.mir, "main.result", &[1]).unwrap_err(),
        "bounds"
    );
}
