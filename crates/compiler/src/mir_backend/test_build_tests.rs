use super::tests::{Fixture, execute, machine};
use kagura::Bus;
use std::collections::BTreeMap;

#[test]
fn test_build_wrappers_execute_private_helpers_initializers_and_one_case_only() {
    let fixture = Fixture::new("package main\nfunc main(){trap \"main must not run\"}\n");
    std::fs::write(fixture.0.join("main_test.ond"), "package main\nvar Seed:u32\nfunc initialize(){Seed=40}\nfunc private()->u32{return Seed+2}\nfunc TestOne(){*(4294934608 as u32 as *u32)=private()}\nfunc TestTwo(){*(4294934608 as u32 as *u32)=private()+1}\n").unwrap();
    let loaded = ond_compiler_core::load_project_with_test_inputs(
        &fixture.0,
        &BTreeMap::new(),
        &[],
        &BTreeMap::new(),
        &BTreeMap::new(),
    )
    .unwrap();
    let project = ond_compiler_core::compile_loaded_project(loaded)
        .unwrap()
        .mir;
    // Source/MIR automatic initializers are currently rejected, so supply an
    // ordinary compiled function through the shared startup's initializer list.
    let initializers = vec!["main.initialize".into()];
    let entries = vec![
        (
            "__ond.test.0".into(),
            initializers.clone(),
            "main.TestOne".into(),
        ),
        ("__ond.test.1".into(), initializers, "main.TestTwo".into()),
    ];
    use crate::kagura_encoding::word;
    let startup = crate::program::test_startup(
        &entries,
        &[
            word(0, 1, 0, 0, -1),
            word(2, 1, 1, 0, 15),
            word(0, 1, 1, 0, 64),
            word(11, 0, 1, 0, 0),
            word(12, 0, 0, 0, -1),
        ],
    );
    let mut objects = vec![startup];
    objects.extend(super::codegen_objects(&project).unwrap());
    let image = crate::linker::link_objects(
        &objects,
        &crate::linker::LinkPlan {
            read_only: None,
            memory_base: 4096,
            memory_size: 65536,
            stack_size: 8192,
            entry_symbol: "__ond.entry".into(),
            heap_base_symbol: "__ond_heap_base".into(),
            stack_bottom_symbol: "__ond_stack_bottom".into(),
        },
    )
    .unwrap();
    let (mut cpu, mut bus, _) = machine(&image);
    assert!(cpu.step(&mut bus).is_err());
    assert_eq!(bus.read32(0xffff8050).unwrap(), 0);
    // Initializer storage must still be zero after the selection-required trap.
    let seed = image.symbols["main.Seed"];
    assert_eq!(bus.read32(seed).unwrap(), 0);
    for (entry, expected) in [
        ("__ond.test.0", 42),
        ("__ond.test.1", 43),
        ("__ond.test.0", 42),
    ] {
        let mut selected = image.clone();
        selected.entry_point = image.symbols[entry];
        assert_eq!(execute(&selected), expected);
    }
}
