use super::*;
use kagura::{Bus, Cpu, DefaultBus};
use ram::Ram;
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};
const RAM: u32 = 0x10000000;
const OUTPUT: u32 = 0x20000100;
const EXIT: u32 = 0x20000200;
struct Fixture(std::path::PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn objects(test: bool, jump: bool) -> Vec<Object> {
    static N: AtomicU64 = AtomicU64::new(0);
    let f = Fixture(std::env::temp_dir().join(format!(
        "ond-exec-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    )));
    fs::create_dir_all(&f.0).unwrap();
    fs::write(f.0.join("ond.toml"), "[project]\nname='app'\n").unwrap();
    fs::write(f.0.join("main.ond"),"package main\nvar G:u32=7\nfunc f()->u32{return G}\nfunc main(){var p=&G;var out=536871168 as u32 as *u32;out[0]=f()+*p;out[1]=p as u32}\nfunc TestOne(){*(536871168 as u32 as *u32)=G+3}\nfunc TestTwo(){*(536871168 as u32 as *u32)=G+1}\n").unwrap();
    let (project, packages) = if test {
        fs::write(
            f.0.join("main.ond"),
            "package main\nvar Counter:u32=3\nfunc main(){}\n",
        )
        .unwrap();
        fs::write(f.0.join("main_test.ond"),"package main\nimport \"app/dep\"\nfunc TestOne(){*(536871168 as u32 as *u32)=Counter+dep.Value()}\nfunc TestTwo(){*(536871168 as u32 as *u32)=Counter+1}\n").unwrap();
        fs::create_dir_all(f.0.join("dep")).unwrap();
        fs::write(
            f.0.join("dep/dep.ond"),
            "package dep\nvar Seed:u32=7\nfunc Value()->u32{return Seed}\n",
        )
        .unwrap();
        let loaded = ond_compiler_core::load_project_with_test_inputs(
            &f.0,
            &Default::default(),
            &[],
            &Default::default(),
            &Default::default(),
        )
        .unwrap();
        let project = ond_compiler_core::compile_loaded_project(loaded)
            .unwrap()
            .mir;
        let packages = crate::mir_backend::codegen_objects(&project).unwrap();
        (project, packages)
    } else {
        let compilation = crate::pipeline::compile_project(&f.0).unwrap();
        (compilation.mir, compilation.objects)
    };
    let metadata = crate::program::ProgramMetadata::from_mir(&project).unwrap();
    let end = if jump {
        crate::kagura_encoding::absolute_thunk(EXIT).to_vec()
    } else {
        vec![crate::kagura_encoding::word(12, 0, 0, 0, -1)]
    };
    let startup = if test {
        crate::program::test_startup(
            &[
                (
                    "case1".into(),
                    metadata.initializers.clone(),
                    "main.TestOne".into(),
                ),
                (
                    "case2".into(),
                    metadata.initializers.clone(),
                    "main.TestTwo".into(),
                ),
            ],
            &end,
        )
    } else {
        crate::program::program_startup(&metadata.initializers, &metadata.entry_symbol, &end)
    };
    let mut objects = vec![startup];
    objects.extend(packages);
    crate::mir_backend::runtime::supply(objects).unwrap()
}
fn linked(
    objects: &[Object],
    mode: u8,
) -> (LinkedOutput, ond_protocol::executable::Executable, Vec<u8>) {
    let kinds = objects
        .iter()
        .flat_map(|o| &o.sections)
        .filter(|s| s.memory_size > 0)
        .map(|s| s.kind)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let prefix = if mode == 2 {
        self_boot_prefix(&kinds, 0).len() as u32
    } else {
        0
    };
    let plan = LinkPlan {
        read_only: (mode > 0).then_some(MemoryRegion {
            base: if mode == 1 { 0x1000 } else { 0 },
            size: 65536,
        }),
        memory_base: RAM,
        memory_size: 65536,
        stack_size: 256,
        entry_symbol: "__ond.entry".into(),
        heap_base_symbol: "__ond_heap_base".into(),
        stack_bottom_symbol: "__ond_stack_bottom".into(),
    };
    let debug = objects
        .iter()
        .map(|o| DebugObject {
            package: o.name.clone(),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    let pairs = objects.iter().zip(&debug).collect::<Vec<_>>();
    let (output, wire) = link_executable_pairs_with_debug(&pairs, &plan, prefix).unwrap();
    let mut bytes = if mode == 2 {
        self_boot_prefix(&kinds, prefix)
    } else {
        Vec::new()
    };
    bytes.extend(wire.write().unwrap());
    (output, wire, bytes)
}
fn machine(
    wire: &ond_protocol::executable::Executable,
    bytes: &[u8],
    mode: u8,
    entry: Option<u32>,
) -> (Cpu, DefaultBus) {
    let mut bus = DefaultBus::new();
    bus.map_device(RAM, 65536, Ram::new(65536)).unwrap();
    bus.map_device(OUTPUT, 8, Ram::new(8)).unwrap();
    bus.map_device(EXIT, 4, Ram::new(4)).unwrap();
    bus.write32(EXIT, 0xc000ffff).unwrap();
    for i in 0..65536 {
        bus.write8(RAM + i, 0xa5).unwrap();
    }
    for i in [0, 4] {
        bus.write32(OUTPUT + i, 0xdeadbeef).unwrap();
    }
    let base = if mode == 0 {
        RAM
    } else if mode == 1 {
        0x1000
    } else {
        0
    };
    if mode > 0 {
        bus.map_device(base, 65536, Ram::new(65536)).unwrap();
    }
    for (i, byte) in bytes.iter().enumerate() {
        bus.write8(base + i as u32, *byte).unwrap();
    }
    let mut cpu = Cpu::new();
    if mode != 2 || entry.is_some() {
        for s in &wire.sections {
            for (i, b) in s.data.iter().enumerate() {
                bus.write8(s.memory_address + i as u32, *b).unwrap();
            }
            for i in s.data.len() as u32..s.memory_size {
                bus.write8(s.memory_address + i, 0).unwrap();
            }
        }
        cpu.set_reg(14, wire.initial_sp);
        cpu.set_pc(entry.unwrap_or(wire.entry_address));
    }
    if mode == 2 && entry.is_none() {
        for _ in 0..200000 {
            if cpu.pc() == wire.entry_address {
                break;
            }
            cpu.step(&mut bus).unwrap();
        }
        assert_eq!(cpu.pc(), wire.entry_address);
    }
    assert_eq!(cpu.reg(14), wire.initial_sp);
    for s in &wire.sections {
        for i in 0..s.memory_size {
            assert_eq!(
                bus.read8(s.memory_address + i).unwrap(),
                s.data.get(i as usize).copied().unwrap_or(0),
                "load state {:?} offset {i}",
                s.kind
            );
        }
    }
    if mode > 0 {
        for (i, byte) in bytes.iter().enumerate() {
            assert_eq!(
                bus.read8(base + i as u32).unwrap(),
                *byte,
                "ROM was modified"
            );
        }
    }
    (cpu, bus)
}
#[test]
fn bin_03_reset_and_external_loaders_execute_final_addresses() {
    for mode in 0..3 {
        for jump in [false, true] {
            let objects = objects(false, jump);
            let (_output, wire, bytes) = linked(&objects, mode);
            let (mut cpu, mut bus) = machine(&wire, &bytes, mode, None);
            for _ in 0..200000 {
                cpu.step(&mut bus).unwrap();
                if bus.read32(OUTPUT + 4).unwrap() != 0xdeadbeef && (!jump || cpu.pc() == EXIT) {
                    break;
                }
            }
            assert_eq!(bus.read32(OUTPUT).unwrap(), 14, "mode {mode}");
            let g = bus.read32(OUTPUT + 4).unwrap();
            let bss = wire
                .sections
                .iter()
                .find(|s| s.kind == SectionKind::Bss)
                .unwrap();
            assert!(g >= bss.memory_address && g + 4 <= bss.memory_address + bss.memory_size);
            if jump {
                assert_eq!(cpu.pc(), EXIT);
            } else {
                for _ in 0..1000 {
                    cpu.step(&mut bus).unwrap();
                }
                let pc = cpu.pc();
                for _ in 0..10 {
                    cpu.step(&mut bus).unwrap();
                    assert_eq!(cpu.pc(), pc);
                }
            }
            assert_eq!(cpu.reg(14), wire.initial_sp);
        }
    }
}
#[test]
fn test_01_reset_guard_and_independent_wrappers() {
    for mode in 0..3 {
        for jump in [false, true] {
            let objects = objects(true, jump);
            let (output, wire, bytes) = linked(&objects, mode);
            let (mut cpu, mut bus) = machine(&wire, &bytes, mode, None);
            let mut trapped = false;
            for _ in 0..200000 {
                if cpu.step(&mut bus).is_err() {
                    trapped = true;
                    break;
                }
            }
            assert!(trapped);
            assert_eq!(bus.read32(OUTPUT).unwrap(), 0xdeadbeef);
            for (entry, expected) in [("case1", 10), ("case2", 4)] {
                let (mut cpu, mut bus) =
                    machine(&wire, &bytes, mode, Some(output.image.symbols[entry]));
                for _ in 0..200000 {
                    cpu.step(&mut bus).unwrap();
                    if bus.read32(OUTPUT).unwrap() != 0xdeadbeef && (!jump || cpu.pc() == EXIT) {
                        break;
                    }
                }
                assert_eq!(bus.read32(OUTPUT).unwrap(), expected);
                if jump {
                    assert_eq!(cpu.pc(), EXIT)
                } else {
                    for _ in 0..1000 {
                        cpu.step(&mut bus).unwrap();
                    }
                    let pc = cpu.pc();
                    for _ in 0..10 {
                        cpu.step(&mut bus).unwrap();
                        assert_eq!(cpu.pc(), pc);
                    }
                }
                assert_eq!(cpu.reg(14), wire.initial_sp);
            }
        }
    }
}

#[test]
fn bin_02_reservations_alignment_zero_symbols_and_exact_capacity() {
    use crate::ir::object::{Section, Symbol, SymbolBinding, SymbolKind};
    let mut object = Object {
        name: "fixture".into(),
        sections: vec![
            Section {
                kind: SectionKind::Text,
                alignment: 16,
                memory_size: 4,
                data: 0xc000ffffu32.to_le_bytes().to_vec(),
            },
            Section {
                kind: SectionKind::Rodata,
                alignment: 4,
                memory_size: 0,
                data: Vec::new(),
            },
        ],
        symbols: vec![
            Symbol {
                name: "start".into(),
                binding: SymbolBinding::Exported,
                kind: SymbolKind::Function,
                section: Some(SectionKind::Text),
                offset: 0,
                size: 4,
            },
            Symbol {
                name: "zero".into(),
                binding: SymbolBinding::Exported,
                kind: SymbolKind::Object,
                section: Some(SectionKind::Rodata),
                offset: 0,
                size: 0,
            },
        ],
        relocations: Vec::new(),
    };
    let debug = DebugObject {
        package: "fixture".into(),
        ..Default::default()
    };
    for rom in [
        None,
        Some(MemoryRegion {
            base: 4,
            size: 65536,
        }),
    ] {
        let plan = LinkPlan {
            read_only: rom,
            memory_base: if rom.is_some() { RAM } else { 4 },
            memory_size: if rom.is_some() { 65536 } else { 65532 },
            stack_size: 256,
            entry_symbol: "start".into(),
            heap_base_symbol: "heap".into(),
            stack_bottom_symbol: "stack".into(),
        };
        let (output, wire) =
            link_executable_pairs_with_debug(&[(&object, &debug)], &plan, 0).unwrap();
        assert_eq!(wire.sections.len(), 1);
        assert!(output.image.symbols.contains_key("zero"));
        assert_eq!(wire.entry_address % 16, 0);
        assert_eq!(
            wire.sections[0].memory_address,
            4 + wire.sections[0].file_offset
        );
    }
    object.sections.truncate(1);
    object.symbols.truncate(1);
    object.sections[0].alignment = 4;
    let mut plan = LinkPlan {
        read_only: None,
        memory_base: 4096,
        memory_size: 64,
        stack_size: 4,
        entry_symbol: "start".into(),
        heap_base_symbol: "heap".into(),
        stack_bottom_symbol: "stack".into(),
    };
    let (output, wire) = link_executable_pairs_with_debug(&[(&object, &debug)], &plan, 0).unwrap();
    assert_eq!(wire.write().unwrap().len(), 60);
    assert_eq!(output.image.heap_base, output.image.stack_bottom);
    plan.stack_size = 8;
    assert!(link_executable_pairs_with_debug(&[(&object, &debug)], &plan, 0).is_err());
    plan.stack_size = 4;
    plan.read_only = Some(MemoryRegion { base: 0, size: 60 });
    assert!(link_executable_pairs_with_debug(&[(&object, &debug)], &plan, 0).is_ok());
    plan.read_only.as_mut().unwrap().size = 56;
    assert!(link_executable_pairs_with_debug(&[(&object, &debug)], &plan, 0).is_err());
}

#[test]
fn bin_02_fixed_wire_load_states() {
    use ond_protocol::executable::Executable;
    let encode = |words: &[u32], payload: &[u8]| {
        let mut bytes = b"ONDX".to_vec();
        for value in words {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend(payload);
        bytes
    };
    let w = encode(
        &[
            1, 1, 1, 84, 0x10000050, 0x10000100, 2, 1, 80, 4, 0x10000050, 4, 4, 4, 0, 0,
            0x10000054, 4, 4,
        ],
        &[0, 0, 0, 0xc0],
    );
    let p = encode(
        &[
            1, 1, 1, 88, 0x10000054, 0x10000100, 2, 1, 84, 4, 0x10000054, 4, 4, 4, 0, 0,
            0x10000058, 4, 4,
        ],
        &[0, 0, 0, 0, 0, 0, 0, 0xc0],
    );
    let d = encode(
        &[
            1, 1, 1, 110, 0x10000068, 0x10000100, 3, 1, 104, 4, 0x10000068, 4, 4, 3, 108, 2,
            0x1000006c, 4, 4, 4, 0, 0, 0x10000070, 4, 4,
        ],
        &[0, 0, 0, 0xc0, 0x11, 0x22],
    );
    let mut empty = w.clone();
    empty[56..60].copy_from_slice(&3u32.to_le_bytes());
    for (bytes, zero_start, heap) in [
        (w, 84, 0x10000058),
        (p, 88, 0x1000005c),
        (d, 110, 0x10000074),
        (empty, 84, 0x10000058),
    ] {
        let wire = Executable::read(&bytes).unwrap();
        let mut bus = DefaultBus::new();
        bus.map_device(RAM, 256, Ram::new(256)).unwrap();
        for i in 0..256 {
            bus.write8(RAM + i, 0xa5).unwrap();
        }
        for (i, b) in bytes.iter().enumerate() {
            bus.write8(RAM + i as u32, *b).unwrap();
        }
        for s in &wire.sections {
            for i in s.data.len() as u32..s.memory_size {
                bus.write8(s.memory_address + i, 0).unwrap();
            }
        }
        let mut cpu = Cpu::new();
        cpu.set_reg(14, wire.initial_sp);
        cpu.set_pc(wire.entry_address);
        for (i, b) in bytes.iter().enumerate() {
            assert_eq!(bus.read8(RAM + i as u32).unwrap(), *b);
        }
        for address in RAM + zero_start..heap {
            assert_eq!(bus.read8(address).unwrap(), 0);
        }
        assert_eq!(bus.read8(heap).unwrap(), 0xa5);
        assert_eq!(cpu.reg(14), 0x10000100);
        assert_eq!(cpu.pc(), wire.sections[0].memory_address);
    }
}
