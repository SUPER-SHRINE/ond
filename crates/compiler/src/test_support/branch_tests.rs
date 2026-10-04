use super::*;
use crate::linker::read_u32;
use crate::{
    kagura_encoding::{link_slot, word},
    object::{Relocation, Symbol, SymbolKind},
};
use kagura::{Bus, Cpu, DefaultBus};
use ram::Ram;

fn object(delta: i32, call: bool) -> (Object, u32, u32) {
    let at = if delta >= 0 { 0 } else { (-delta - 1) as usize };
    let target = if delta >= 0 { delta as usize + 1 } else { 0 };
    let mut words = vec![word(6, 0, 0, 0, 0); (at + 6).max(target + 1)];
    words[at..at + 6].copy_from_slice(&link_slot(call, if call { 0 } else { 1 }));
    words[target] = word(12, 0, 0, 15, 0);
    let mut object = startup_object(&[], "unused");
    object.sections.truncate(1);
    object.sections[0].memory_size = words.len() as u32 * 4;
    object.sections[0].data = words.into_iter().flat_map(u32::to_le_bytes).collect();
    object.symbols = [(ENTRY_SYMBOL, at), ("dest", target)]
        .into_iter()
        .map(|(name, offset)| Symbol {
            name: name.into(),
            binding: SymbolBinding::Exported,
            kind: SymbolKind::Function,
            section: Some(SectionKind::Text),
            offset: offset as u32 * 4,
            size: 4,
        })
        .collect();
    object.relocations = vec![Relocation {
        section: SectionKind::Text,
        offset: at as u32 * 4,
        kind: if call {
            RelocationKind::CallSlot
        } else {
            RelocationKind::JumpSlot
        },
        target: "dest".into(),
        addend: 0,
    }];
    (object, at as u32 * 4, target as u32 * 4)
}

#[test]
fn linker_slots_execute_at_signed_boundaries_preserving_call_and_jump_state() {
    for delta in [32767, 32768, -32768, -32769] {
        for (call, condition) in [(true, 0), (false, 0), (false, 1)] {
            let (object, at, target) = object(delta, call);
            let linked = link_objects(&[object], 1024 * 1024, 65536).unwrap();
            let section = &linked.sections[0];
            let near = (-32768..=32767).contains(&delta);
            assert_eq!(
                read_u32(section, at).unwrap() as u16 as i16,
                if near { delta as i16 } else { 1 }
            );
            if !near {
                assert_eq!(read_u32(section, at + 20).unwrap(), RAM_BASE + target);
            }
            let mut bus = DefaultBus::new();
            bus.map_device(RAM_BASE, 1024 * 1024, Ram::new(1024 * 1024))
                .unwrap();
            for (i, byte) in section.data.iter().enumerate() {
                bus.write8(RAM_BASE + i as u32, *byte).unwrap();
            }
            let mut cpu = Cpu::new();
            for r in 1..=15 {
                cpu.set_reg(r, 0x12340000 + r as u32);
            }
            if !call {
                cpu.set_reg(1, condition);
            }
            cpu.set_pc(RAM_BASE + at);
            let taken = call || condition == 0;
            let goal = RAM_BASE + if taken { target } else { at + 24 };
            for _ in 0..4 {
                cpu.step(&mut bus).unwrap();
                if cpu.pc() == goal {
                    break;
                }
            }
            assert_eq!(cpu.pc(), goal);
            for r in 1..=15 {
                if r == 12 && taken && !near {
                    continue;
                }
                let want = if r == 15 && call {
                    RAM_BASE + at + 4
                } else if r == 1 && !call {
                    condition
                } else {
                    0x12340000 + r as u32
                };
                assert_eq!(cpu.reg(r), want, "delta={delta}, r{r}");
            }
            if call {
                cpu.step(&mut bus).unwrap(); // Return uses the ORIGINAL link.
                assert_eq!(cpu.pc(), RAM_BASE + at + 4);
                cpu.step(&mut bus).unwrap(); // Skip island/literal after return.
                assert_eq!(cpu.pc(), RAM_BASE + at + 24);
            }
        }
    }
}

fn failure(object: Object) -> String {
    link_objects(&[object], 1024 * 1024, 65536)
        .unwrap_err()
        .diagnostics()[0]
        .message
        .clone()
}

#[test]
fn invalid_branch_slots_and_targets_are_diagnosed() {
    let (base, _, _) = object(32768, true);
    let mut bad = base.clone();
    bad.sections[0].data[4] ^= 1;
    assert!(failure(bad).contains("template"));
    let mut bad = base.clone();
    bad.relocations[0].offset = 1;
    assert!(failure(bad).contains("word-aligned"));
    let mut bad = base.clone();
    bad.relocations[0].offset = bad.sections[0].memory_size - 4;
    assert!(failure(bad).contains("input text bounds"));
    let mut bad = base.clone();
    bad.relocations[0].addend = 1;
    assert!(failure(bad).contains("word-aligned"));
    let mut bad = base.clone();
    bad.relocations[0].addend = i32::MIN;
    assert!(failure(bad).contains("address overflow"));
    let mut bad = base.clone();
    bad.relocations[0].target = "missing".into();
    assert!(failure(bad).contains("unknown relocation target"));
    let mut bad = base.clone();
    bad.relocations[0].kind = RelocationKind::Call16;
    assert!(failure(bad).contains("out of range"));
    let mut bad = base.clone();
    bad.relocations.push(bad.relocations[0].clone());
    assert!(failure(bad).contains("template"));
    assert!(
        link_objects(&[base], 65536, 8192)
            .unwrap_err()
            .diagnostics()[0]
            .message
            .contains("does not fit")
    );
}

#[test]
fn thunk_addend_and_multiple_sites_do_not_move_symbols_or_abs32_literals() {
    let (mut object, _, target) = object(32768, true);
    object.sections[0].data[24..48].copy_from_slice(
        &link_slot(true, 0)
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect::<Vec<_>>(),
    );
    let mut second = object.relocations[0].clone();
    second.offset = 24;
    second.addend = -4;
    object.relocations.push(second);
    object.relocations.push(Relocation {
        section: SectionKind::Text,
        offset: 48,
        kind: RelocationKind::Abs32,
        target: "dest".into(),
        addend: 0,
    });
    // A larger separation makes both sites far, even after the negative addend.
    object.symbols[1].offset += 128;
    object.sections[0].data.extend(vec![0; 128]);
    object.sections[0].memory_size += 128;
    let size = object.sections[0].memory_size;
    let linked = link_objects(&[object], 1024 * 1024, 65536).unwrap();
    let text = &linked.sections[0];
    assert_eq!(text.memory_size, size);
    assert_eq!(read_u32(text, 20).unwrap(), RAM_BASE + target + 128);
    assert_eq!(read_u32(text, 44).unwrap(), RAM_BASE + target + 124);
    assert_eq!(read_u32(text, 48).unwrap(), RAM_BASE + target + 128);
}
