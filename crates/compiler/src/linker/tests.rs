use super::*;
use crate::ir::object::{Relocation, Section, Symbol, SymbolBinding, SymbolKind};

fn plan(base: u32) -> LinkPlan {
    LinkPlan {
        read_only: None,
        memory_base: base,
        memory_size: 64 * 1024 * 1024,
        stack_size: 1024 * 1024,
        entry_symbol: "start".into(),
        heap_base_symbol: "heap".into(),
        stack_bottom_symbol: "stack".into(),
    }
}

fn object() -> Object {
    Object {
        name: "test".into(),

        sections: vec![Section {
            kind: SectionKind::Text,
            alignment: 4,
            data: vec![0; 8],
            memory_size: 8,
        }],
        symbols: vec![Symbol {
            name: "start".into(),
            binding: SymbolBinding::Exported,
            kind: SymbolKind::Function,
            section: Some(SectionKind::Text),
            offset: 0,
            size: 4,
        }],
        relocations: vec![Relocation {
            section: SectionKind::Text,
            offset: 4,
            kind: RelocationKind::Abs32,
            target: "start".into(),
            addend: 0,
        }],
    }
}

#[test]
fn profile_controls_addresses_without_container_or_mir() {
    for base in [0x1000, 0x10000000] {
        let image = link_objects(&[object()], &plan(base)).unwrap();
        assert_eq!(image.entry_point, base);
        assert_eq!(read_u32(&image.sections[0], 4).unwrap(), base);
        assert_eq!(image.heap_base, base + 8);
        assert_eq!(image.stack_top, base + 64 * 1024 * 1024);
        assert_eq!(image.stack_bottom, image.stack_top - 1024 * 1024);
        assert_eq!(image.symbols["heap"], image.heap_base);
        assert_eq!(image.symbols["stack"], image.stack_bottom);
        assert_eq!(image.sections[0].data.len(), 8);
    }
}

#[test]
fn invalid_memory_plans_are_rejected_without_silent_stack_growth() {
    let mut p = plan(0xfffff000);
    assert!(
        link_objects(&[object()], &p)
            .unwrap_err()
            .message
            .contains("overflow")
    );
    p = plan(0x10000001);
    assert!(
        link_objects(&[object()], &p)
            .unwrap_err()
            .message
            .contains("word-aligned")
    );
    p = plan(0x10000000);
    p.stack_size = p.memory_size;
    assert!(
        link_objects(&[object()], &p)
            .unwrap_err()
            .message
            .contains("RAM/stack")
    );
    p.stack_size = 0;
    assert!(
        link_objects(&[object()], &p)
            .unwrap_err()
            .message
            .contains("RAM/stack")
    );
    p.memory_size = 8;
    p.stack_size = 4;
    assert!(
        link_objects(&[object()], &p)
            .unwrap_err()
            .message
            .contains("does not fit")
    );
}

#[test]
fn entry_is_selected_by_the_caller_not_by_ond_naming() {
    let mut p = plan(0x10000000);
    p.entry_symbol = "not_present".into();
    assert!(
        link_objects(&[object()], &p)
            .unwrap_err()
            .message
            .contains("entry")
    );
    p.entry_symbol = "start".into();
    assert!(link_objects(&[object()], &p).is_ok());
}
#[test]
fn malformed_objects_are_rejected_before_patching() {
    let cases: Vec<(&str, Box<dyn Fn(&mut Object)>)> = vec![
        (
            "duplicate section",
            Box::new(|o| o.sections.push(o.sections[0].clone())),
        ),
        ("alignment", Box::new(|o| o.sections[0].alignment = 3)),
        (
            "payload exceeds",
            Box::new(|o| o.sections[0].memory_size = 4),
        ),
        (
            "complete payload",
            Box::new(|o| o.sections[0].data.truncate(4)),
        ),
        (
            "duplicate symbol",
            Box::new(|o| o.symbols.push(o.symbols[0].clone())),
        ),
        (
            "missing section",
            Box::new(|o| o.symbols[0].section = Some(SectionKind::Data)),
        ),
        ("symbol exceeds", Box::new(|o| o.symbols[0].size = 9)),
        (
            "invalid imported",
            Box::new(|o| o.symbols[0].binding = SymbolBinding::Imported),
        ),
        ("word-aligned", Box::new(|o| o.relocations[0].offset = 1)),
        ("payload bounds", Box::new(|o| o.relocations[0].offset = 8)),
        (
            "overlapping",
            Box::new(|o| o.relocations.push(o.relocations[0].clone())),
        ),
        (
            "unknown relocation",
            Box::new(|o| o.relocations[0].target = "unknown".into()),
        ),
        (
            "address overflow",
            Box::new(|o| o.relocations[0].addend = i32::MIN),
        ),
    ];
    for (expected, edit) in cases {
        let mut o = object();
        edit(&mut o);
        let err = link_objects(&[o], &plan(0x1000)).unwrap_err();
        assert!(err.message.contains(expected), "{expected}: {err}");
    }
}
#[test]
fn local_symbols_are_object_scoped_and_exports_cannot_collide() {
    let mut a = object();
    a.symbols.push(Symbol {
        name: "private".into(),
        binding: SymbolBinding::Local,
        kind: SymbolKind::Object,
        section: Some(SectionKind::Text),
        offset: 4,
        size: 4,
    });
    a.relocations[0].target = "private".into();
    let mut b = a.clone();
    b.symbols[0].name = "second".into();
    let image = link_objects(&[a.clone(), b.clone()], &plan(0x1000)).unwrap();
    assert_eq!(read_u32(&image.sections[0], 4).unwrap(), 0x1004);
    assert_eq!(read_u32(&image.sections[0], 12).unwrap(), 0x100c);
    assert!(!image.symbols.contains_key("private"));
    b.symbols.pop();
    assert!(
        link_objects(&[a.clone(), b], &plan(0x1000))
            .unwrap_err()
            .message
            .contains("unknown relocation")
    );
    assert!(
        link_objects(&[a.clone(), a.clone()], &plan(0x1000))
            .unwrap_err()
            .message
            .contains("duplicate export")
    );
    a.symbols[0].name = "heap".into();
    assert!(
        link_objects(&[a], &plan(0x1000))
            .unwrap_err()
            .message
            .contains("reserved")
    );
}
#[test]
fn readonly_region_has_separate_runtime_addresses_and_checked_capacity() {
    let mut p = plan(0x10000000);
    p.read_only = Some(MemoryRegion {
        base: 256,
        size: 3840,
    });
    let image = link_objects(&[object()], &p).unwrap();
    assert_eq!(image.entry_point, 256);
    assert_eq!(image.heap_base, 0x10000000);
    p.read_only = Some(MemoryRegion {
        base: 0x10000000,
        size: 4096,
    });
    assert!(
        link_objects(&[object()], &p)
            .unwrap_err()
            .message
            .contains("overlapping")
    );
    p.read_only = Some(MemoryRegion { base: 256, size: 4 });
    assert!(
        link_objects(&[object()], &p)
            .unwrap_err()
            .message
            .contains("does not fit")
    );
}
#[test]
fn branch_slots_reject_interior_symbols_and_resolved_targets() {
    let mut o = object();
    o.sections[0].data = crate::kagura_encoding::link_slot(true, 0)
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect();
    o.sections[0].memory_size = 24;
    o.relocations[0].offset = 0;
    o.relocations[0].kind = RelocationKind::CallSlot;
    o.relocations[0].addend = 4;
    assert!(
        link_objects(&[o.clone()], &plan(0x1000))
            .unwrap_err()
            .message
            .contains("slot interior")
    );
    o.relocations[0].addend = 0;
    o.symbols.push(Symbol {
        name: "inside".into(),
        binding: SymbolBinding::Local,
        kind: SymbolKind::Object,
        section: Some(SectionKind::Text),
        offset: 8,
        size: 4,
    });
    assert!(
        link_objects(&[o], &plan(0x1000))
            .unwrap_err()
            .message
            .contains("slot interior")
    );
}
