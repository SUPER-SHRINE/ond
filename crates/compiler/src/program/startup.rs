//! Shared Ond/Kagura runtime ABI startup; caller supplies termination instructions.
use crate::ir::object::{Object, RelocationKind, SectionKind, SymbolBinding};
const ENTRY_SYMBOL: &str = "__ond.entry";
const HEAP_BASE_SYMBOL: &str = "__ond_heap_base";
const STACK_BOTTOM_SYMBOL: &str = "__ond_stack_bottom";
const HEAP_CURSOR_SYMBOL: &str = "__ond_heap_cursor";
const HEAP_BASE_SLOT_SYMBOL: &str = "__ond_heap_base_slot";
const STACK_BOTTOM_SLOT_SYMBOL: &str = "__ond_stack_bottom_slot";
pub fn program_startup(
    init_symbols: &[String],
    main_symbol: &str,
    return_instructions: &[u32],
) -> Object {
    let mut words = Vec::new();
    let mut relocations = Vec::new();

    for symbol in init_symbols {
        let offset = (words.len() * 4) as u32;
        words.extend(crate::kagura_encoding::link_slot(true, 0));
        relocations.push(crate::ir::object::Relocation {
            section: SectionKind::Text,
            offset,
            kind: RelocationKind::CallSlot,
            target: symbol.clone(),
            addend: 0,
        });
    }

    let main_offset = (words.len() * 4) as u32;
    words.extend(crate::kagura_encoding::link_slot(true, 0));
    relocations.push(crate::ir::object::Relocation {
        section: SectionKind::Text,
        offset: main_offset,
        kind: RelocationKind::CallSlot,
        target: main_symbol.to_string(),
        addend: 0,
    });

    words.extend_from_slice(return_instructions);

    let mut data = vec![0u8; 8];
    let heap_base_data_offset = 0u32;
    let stack_bottom_data_offset = 4u32;
    let _ = &mut data;
    relocations.push(crate::ir::object::Relocation {
        section: SectionKind::Data,
        offset: heap_base_data_offset,
        kind: RelocationKind::Abs32,
        target: HEAP_BASE_SYMBOL.to_string(),
        addend: 0,
    });
    relocations.push(crate::ir::object::Relocation {
        section: SectionKind::Data,
        offset: stack_bottom_data_offset,
        kind: RelocationKind::Abs32,
        target: STACK_BOTTOM_SYMBOL.to_string(),
        addend: 0,
    });

    let text_size = (words.len() * 4) as u32;
    Object {
        name: "\u{0001}startup".to_string(),

        sections: vec![
            crate::ir::object::Section {
                kind: SectionKind::Text,
                alignment: 4,
                memory_size: (words.len() * 4) as u32,
                data: words.into_iter().flat_map(u32::to_le_bytes).collect(),
            },
            crate::ir::object::Section {
                kind: SectionKind::Data,
                alignment: 4,
                memory_size: 8,
                data,
            },
            crate::ir::object::Section {
                kind: SectionKind::Bss,
                alignment: 4,
                memory_size: 4,
                data: Vec::new(),
            },
        ],
        symbols: vec![
            crate::ir::object::Symbol {
                name: ENTRY_SYMBOL.to_string(),
                binding: SymbolBinding::Exported,
                kind: crate::ir::object::SymbolKind::Function,
                section: Some(SectionKind::Text),
                offset: 0,
                size: text_size,
            },
            crate::ir::object::Symbol {
                name: HEAP_BASE_SLOT_SYMBOL.to_string(),
                binding: SymbolBinding::Exported,
                kind: crate::ir::object::SymbolKind::Object,
                section: Some(SectionKind::Data),
                offset: 0,
                size: 4,
            },
            crate::ir::object::Symbol {
                name: STACK_BOTTOM_SLOT_SYMBOL.to_string(),
                binding: SymbolBinding::Exported,
                kind: crate::ir::object::SymbolKind::Object,
                section: Some(SectionKind::Data),
                offset: 4,
                size: 4,
            },
            crate::ir::object::Symbol {
                name: HEAP_CURSOR_SYMBOL.to_string(),
                binding: SymbolBinding::Exported,
                kind: crate::ir::object::SymbolKind::Object,
                section: Some(SectionKind::Bss),
                offset: 0,
                size: 4,
            },
        ],
        relocations,
    }
}

/// One shared runtime state and an independent initializer/case entry for each test.
pub fn test_startup(
    cases: &[(String, Vec<String>, String)],
    return_instructions: &[u32],
) -> Object {
    use crate::ir::object::{Relocation, Symbol, SymbolKind};
    let mut object = program_startup(&[], "unused", &[]);
    object
        .relocations
        .retain(|r| r.section != SectionKind::Text);
    object.symbols[0].size = 4;
    let mut words = vec![crate::kagura_encoding::word(6, 0, 0, 0, 0)];
    for (entry, initializers, case) in cases {
        let offset = (words.len() * 4) as u32;
        for symbol in initializers.iter().chain(std::iter::once(case)) {
            object.relocations.push(Relocation {
                section: SectionKind::Text,
                offset: (words.len() * 4) as u32,
                kind: RelocationKind::CallSlot,
                target: symbol.clone(),
                addend: 0,
            });
            words.extend(crate::kagura_encoding::link_slot(true, 0));
        }
        words.extend_from_slice(return_instructions);
        object.symbols.push(Symbol {
            name: entry.clone(),
            binding: SymbolBinding::Exported,
            kind: SymbolKind::Function,
            section: Some(SectionKind::Text),
            offset,
            size: (words.len() * 4) as u32 - offset,
        });
    }
    object.sections[0].memory_size = (words.len() * 4) as u32;
    object.sections[0].data = words.into_iter().flat_map(u32::to_le_bytes).collect();
    object
}
