//! 固定wire case由来の入力と期待。実装から期待byteを生成しない。
use super::*;
use ond_protocol::debug::{DebugObject, DebugOperation};

fn fixture() -> (Object, LinkPlan) {
    let object = Object {
        name: "input".into(),
        sections: vec![Section {
            kind: SectionKind::Text,
            alignment: 4,
            data: vec![0x34, 0x12, 0, 0xcf, 0, 0, 0, 0, 0, 0, 0, 0],
            memory_size: 12,
        }],
        symbols: vec![
            Symbol {
                name: "main.main".into(),
                binding: SymbolBinding::Exported,
                kind: SymbolKind::Function,
                section: Some(SectionKind::Text),
                offset: 0,
                size: 4,
            },
            Symbol {
                name: "target".into(),
                binding: SymbolBinding::Exported,
                kind: SymbolKind::Function,
                section: Some(SectionKind::Text),
                offset: 8,
                size: 4,
            },
        ],
        relocations: vec![Relocation {
            section: SectionKind::Text,
            offset: 0,
            kind: RelocationKind::Call16,
            target: "target".into(),
            addend: 0,
        }],
    };
    let plan = LinkPlan {
        read_only: None,
        memory_base: 4096,
        memory_size: 65536,
        stack_size: 8192,
        entry_symbol: "main.main".into(),
        heap_base_symbol: "heap".into(),
        stack_bottom_symbol: "bottom".into(),
    };
    (object, plan)
}
#[test]
fn link01_call16() {
    let (o, p) = fixture();
    let image = link_objects(&[o], &p).unwrap();
    assert_eq!(
        &image.sections[0].data[..4],
        &[1, 0, 0, 0xcf],
        "LINK-01.call16"
    );
}
#[test]
fn link01_jump16() {
    let (mut o, p) = fixture();
    o.sections[0].data[..4].copy_from_slice(&[0xaa, 0xaa, 0x20, 0xc0]);
    o.relocations[0].kind = RelocationKind::Jump16;
    let image = link_objects(&[o], &p).unwrap();
    assert_eq!(
        &image.sections[0].data[..4],
        &[1, 0, 0x20, 0xc0],
        "LINK-01.jump16"
    );
}
#[test]
fn link01_call_opcode_reject() {
    let (mut o, p) = fixture();
    o.sections[0].data[3] = 0xbf;
    assert!(
        link_objects(&[o], &p).is_err(),
        "LINK-01.call-opcode-reject"
    );
}
#[test]
fn link01_ram_end_align() {
    let (o, mut p) = fixture();
    p.memory_size = 65540;
    assert!(link_objects(&[o], &p).is_err(), "LINK-01.ram-end-align");
}
#[test]
fn link01_four_byte_stack() {
    let (o, mut p) = fixture();
    p.stack_size = 4;
    let image = link_objects(&[o], &p).unwrap();
    assert_eq!(
        (image.stack_top, image.stack_bottom),
        (69632, 69628),
        "LINK-01.four-byte-stack"
    );
}
#[test]
fn link01_local_hidden() {
    let (mut o, p) = fixture();
    o.relocations.clear();
    o.sections[0].data.fill(0);
    o.symbols.push(Symbol {
        name: "hidden".into(),
        binding: SymbolBinding::Local,
        kind: SymbolKind::Object,
        section: Some(SectionKind::Text),
        offset: 4,
        size: 4,
    });
    let image = link_objects(&[o], &p).unwrap();
    assert_eq!(
        image.symbols,
        BTreeMap::from([
            ("main.main".into(), 4096),
            ("target".into(), 4104),
            ("heap".into(), 4108),
            ("bottom".into(), 61440)
        ]),
        "LINK-01.local-hidden"
    );
}

fn debug_fixture() -> (Object, DebugObject, LinkPlan) {
    let (mut object, mut plan) = fixture();
    object.name = "diagnostic".into();
    object.relocations.clear();
    object.sections[0].data = vec![0; 64];
    object.sections[0].memory_size = 64;
    object.symbols = vec![
        Symbol {
            name: "F".into(),
            binding: SymbolBinding::Exported,
            kind: SymbolKind::Function,
            section: Some(SectionKind::Text),
            offset: 0,
            size: 16,
        },
        Symbol {
            name: "G".into(),
            binding: SymbolBinding::Local,
            kind: SymbolKind::Function,
            section: Some(SectionKind::Text),
            offset: 0,
            size: 16,
        },
    ];
    plan.entry_symbol = "F".into();
    use ond_protocol::debug::{
        ObjectDebugRange, ObjectFunctionDebug, ObjectUnwindRow, ReturnAddressLocation::*,
    };
    let debug = DebugObject {
        package: "sample".into(),
        functions: vec![ObjectFunctionDebug {
            symbol: "F".into(),
            section: SectionKind::Text,
            offset: 0,
            size: 16,
            frame_size: 8,
            return_address_offset: 4,
            source: None,
            unwind: vec![
                ObjectUnwindRow {
                    offset: 0,
                    cfa_sp_offset: 0,
                    return_address: LinkRegister,
                },
                ObjectUnwindRow {
                    offset: 4,
                    cfa_sp_offset: 8,
                    return_address: StackOffset(4),
                },
                ObjectUnwindRow {
                    offset: 16,
                    cfa_sp_offset: 0,
                    return_address: LinkRegister,
                },
            ],
        }],
        ranges: vec![
            ObjectDebugRange {
                section: SectionKind::Text,
                start: 0,
                end: 16,
                operation: DebugOperation::Source,
                message: None,
                source: None,
                function: "G".into(),
            },
            ObjectDebugRange {
                section: SectionKind::Text,
                start: 4,
                end: 8,
                operation: DebugOperation::Call,
                message: None,
                source: None,
                function: "G".into(),
            },
            ObjectDebugRange {
                section: SectionKind::Text,
                start: 4,
                end: 8,
                operation: DebugOperation::ExplicitTrap,
                message: Some("reason".into()),
                source: None,
                function: "G".into(),
            },
        ],
    };
    (object, debug, plan)
}

#[test]
fn paired_debug_preserves_ambiguous_local_ownership() {
    let (mut first, mut first_debug, mut plan) = debug_fixture();
    first.name = "first diagnostic".into();
    first.symbols[0].name = "First".into();
    first_debug.package = "first logical".into();
    first_debug.functions[0].symbol = "G".into();
    for range in &mut first_debug.ranges {
        range.function = "G".into();
    }
    let mut second = first.clone();
    second.name = "second diagnostic".into();
    second.symbols[0].name = "Second".into();
    let mut second_debug = first_debug.clone();
    second_debug.package = "first logical".into();
    plan.entry_symbol = "First".into();
    assert!(
        super::super::link_objects_with_debug(
            &[first.clone(), second.clone()],
            &[first_debug.clone(), second_debug.clone()],
            &plan
        )
        .is_err()
    );
    let output = super::super::link_object_pairs_with_debug(
        &[(&first, &first_debug), (&second, &second_debug)],
        &plan,
    )
    .unwrap();
    assert_eq!(
        output
            .debug
            .functions
            .iter()
            .map(|function| function.address)
            .collect::<Vec<_>>(),
        vec![4096, 4160]
    );
}
#[test]
fn dbg01_relative_link_object_name_independent() {
    let (o, d, p) = debug_fixture();
    let linked = link_objects_with_debug(&[o], &[d], &p).expect(
        "DBG-01.relative-link: unique direct input association is independent of Object.name",
    );
    assert_eq!(linked.debug.functions[0].address, 4096);
    assert_eq!(
        linked.debug.functions[0]
            .unwind
            .iter()
            .map(|r| r.address)
            .collect::<Vec<_>>(),
        [4096, 4100, 4112]
    );
    assert_eq!(
        linked
            .debug
            .ranges
            .iter()
            .map(|r| (r.start, r.end))
            .collect::<Vec<_>>(),
        [(4096, 4112), (4100, 4104), (4100, 4104)]
    );
}
// 以下は所属対応の別課題が拒否検査を隠さないよう、同名の所属Objectで各条件を単独検査する。
#[test]
fn dbg01_duplicate_row() {
    let (mut o, mut d, p) = debug_fixture();
    o.name = d.package.clone();
    let duplicate = d.functions[0].unwind[1].clone();
    d.functions[0].unwind.insert(2, duplicate);
    assert!(
        link_objects_with_debug(&[o], &[d], &p).is_err(),
        "DBG-01.duplicate-row"
    );
}
#[test]
fn dbg01_range_symbol_outside() {
    let (mut o, mut d, p) = debug_fixture();
    o.name = d.package.clone();
    d.ranges[0].end = 20;
    assert!(
        link_objects_with_debug(&[o], &[d], &p).is_err(),
        "DBG-01.range-symbol-outside"
    );
}
#[test]
fn dbg01_nontrap_message() {
    let (mut o, mut d, p) = debug_fixture();
    o.name = d.package.clone();
    assert_eq!(d.ranges[0].operation, DebugOperation::Source);
    d.ranges[0].message = Some("x".into());
    assert!(
        link_objects_with_debug(&[o], &[d], &p).is_err(),
        "DBG-01.nontrap-message"
    );
}
