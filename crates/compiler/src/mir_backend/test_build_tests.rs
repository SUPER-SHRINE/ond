//! 通常startupから各source fileの関数を呼び出す。
use super::tests::{Fixture, execute};
#[test]
fn ordinary_startup_executes_suffix_functions_and_initializers() {
    let fixture =
        Fixture::new("package main\nfunc main(){*(4294934608 as u32 as *u32)=TestOne()}\n");
    std::fs::write(fixture.0.join("main_test.ond"), "package main\nvar Seed:u32=40\nfunc private()->u32{return Seed+2}\nfunc TestOne()->u32{return private()}\nfunc TestTwo(x:u32)->u32{return private()+x}\n").unwrap();
    let project = fixture.core();
    let metadata = crate::program::ProgramMetadata::from_mir(&project).unwrap();
    let startup = crate::program::program_startup(
        &metadata.initializers,
        &metadata.entry_symbol,
        &[
            crate::kagura_encoding::word(0, 1, 0, 0, -1),
            crate::kagura_encoding::word(2, 1, 1, 0, 15),
            crate::kagura_encoding::word(0, 1, 1, 0, 64),
            crate::kagura_encoding::word(11, 0, 1, 0, 0),
            crate::kagura_encoding::word(12, 0, 0, 0, -1),
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
    assert!(
        !image
            .symbols
            .keys()
            .any(|name| name.starts_with("__ond.test."))
    );
    assert_eq!(execute(&image), 42);
}
