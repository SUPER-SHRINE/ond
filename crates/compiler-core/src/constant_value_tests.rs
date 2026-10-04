//! Mandatory evaluation at runtime-value boundaries, without variable propagation.
use crate::{
    mir::{BinaryOp, InstructionKind},
    test_support::compile,
};

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
        "store32(0, 4294967295 + 1)",
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
