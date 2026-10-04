//! Reuse the shared source fixtures with the core MemoryVm runner.
use super::*;
use crate::test_vectors::evaluation as data;
use std::sync::LazyLock;
static STORAGE: LazyLock<Vec<(Vec<SourceFile<'static>>, [Check; 1])>> = LazyLock::new(|| {
    data::CASES
        .iter()
        .map(|case| {
            (
                case.files
                    .iter()
                    .map(|f| SourceFile {
                        path: f.path,
                        text: f.text,
                    })
                    .collect(),
                [Check::MemoryExecution {
                    description: "shared trace and values",
                    function: "main.test",
                    input: &[],
                    expected: case.expected,
                }],
            )
        })
        .collect()
});
pub(super) static CASES: LazyLock<Vec<Case<'static>>> = LazyLock::new(|| {
    data::CASES
        .iter()
        .zip(STORAGE.iter())
        .map(|(case, (files, checks))| Case {
            id: case.id,
            specs: case.specs,
            layer: Layer::MemoryVm,
            manifest: true,
            files,
            expected: Expected::Accept(checks),
        })
        .collect()
});
#[test]
fn operands() {
    run(&CASES[0]).unwrap();
}
#[test]
fn arguments() {
    run(&CASES[1]).unwrap();
}
#[test]
fn indices_distinct() {
    run(&CASES[2]).unwrap();
}
#[test]
fn indices_alias() {
    run(&CASES[3]).unwrap();
}
#[test]
fn pointers_alias() {
    run(&CASES[4]).unwrap();
}
#[test]
fn index_snapshot() {
    run(&CASES[5]).unwrap();
}
#[test]
fn pointer_snapshot() {
    run(&CASES[6]).unwrap();
}
#[test]
fn rhs_snapshot() {
    run(&CASES[7]).unwrap();
}
#[test]
fn rhs_mutates_index() {
    run(&CASES[8]).unwrap();
}
#[test]
fn aggregate_then_field() {
    run(&CASES[9]).unwrap();
}
#[test]
fn field_then_aggregate() {
    run(&CASES[10]).unwrap();
}
#[test]
fn multi_return_alias() {
    run(&CASES[11]).unwrap();
}
#[test]
fn branch_short_circuit() {
    run(&CASES[12]).unwrap();
}
#[test]
fn continue_post_break() {
    run(&CASES[13]).unwrap();
}
#[test]
fn nested_break_continue() {
    run(&CASES[14]).unwrap();
}
#[test]
fn declaration_shadow() {
    run(&CASES[15]).unwrap();
}
#[test]
fn if_scope_both_branches() {
    run(&CASES[16]).unwrap();
}
#[test]
fn for_scope_restore() {
    run(&CASES[17]).unwrap();
}
#[test]
fn pointer_base_then_index() {
    run(&CASES[18]).unwrap();
}
#[test]
fn return_values_left_to_right() {
    run(&CASES[19]).unwrap();
}
#[test]
fn early_return_stops_effects() {
    run(&CASES[20]).unwrap();
}
#[test]
fn const_shadow_initializers() {
    run(&CASES[21]).unwrap();
}
#[test]
fn short_shadow_initializers() {
    run(&CASES[22]).unwrap();
}
#[test]
fn import_shadow_and_restore() {
    run(&CASES[23]).unwrap();
}
#[test]
fn if_scope_then() {
    run(&CASES[24]).unwrap();
}
