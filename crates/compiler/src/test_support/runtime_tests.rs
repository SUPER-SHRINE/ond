//! Simulate backend helper requests at the Object boundary, then use the real
//! supply path, linker, direct image loading and Kagura CPU. No f32 MIR support implied.
use super::*;
use crate::mir_backend::runtime::{self, Helper};
use crate::{
    project::{LoadedProject, PackageId, PackageSources},
    source::SourceDb,
};
use kagura::{Bus, Cpu, DefaultBus};
use ram::Ram;
fn input(source: String) -> Vec<Object> {
    let root = std::path::PathBuf::from("<helper-caller>");
    let mut sources = SourceDb::default();
    let id = sources.add_file(root.join("main.ond"), source);
    let loaded = LoadedProject {
        root: root.clone(),
        manifest_path: root.join("ond.toml"),
        ignored_paths: Vec::new(),
        sources,
        packages: vec![PackageSources {
            id: PackageId(0),
            logical_path: ".".into(),
            directory: root,
            files: vec![id],
        }],
    };
    let p = ond_compiler_core::compile_loaded_project(loaded).unwrap();
    crate::mir_backend::codegen_objects(&p.mir).unwrap()
}
fn execute_helper(h: Helper, args: &[u32], fault: bool) -> u32 {
    let sig = h.signature();
    let parameters = sig
        .parameters
        .iter()
        .enumerate()
        .map(|(i, t)| {
            format!(
                "a{i}:{}",
                if *t == ond_compiler_core::mir::TypeId::I32 {
                    "i32"
                } else {
                    "u32"
                }
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let arguments = args
        .iter()
        .zip(sig.parameters)
        .map(|(n, t)| {
            if t == ond_compiler_core::mir::TypeId::I32 {
                format!("{}", *n as i32)
            } else {
                n.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(",");
    let mut objects = input(format!(
        "package main\nfunc stub({parameters})->u32{{return 0}}\nfunc main(){{store32(4294934608,stub({arguments}))}}"
    ));
    // The stub establishes the exact typed call/frame. A backend operation would
    // request this reserved symbol directly instead of emitting main.stub.
    let mut replaced = 0;
    for r in objects.iter_mut().flat_map(|o| &mut o.relocations) {
        if r.target == "main.stub" {
            r.target = h.symbol();
            replaced += 1;
        }
    }
    assert_eq!(replaced, 1);
    let mut complete = vec![startup_object(&[], "main.main")];
    complete.extend(runtime::supply(objects).unwrap());
    let linked = link_objects(&complete, 1024 * 1024, 65536).unwrap();
    let exe = linked;
    let mut bus = DefaultBus::new();
    bus.map_device(RAM_BASE, exe.ram_size, Ram::new(exe.ram_size as usize))
        .unwrap();
    bus.map_device(0xffff8040, 4, Ram::new(4)).unwrap();
    bus.map_device(0xffff8050, 4, Ram::new(4)).unwrap();
    bus.write32(0xffff8040, 0xdeadbeef).unwrap();
    for s in exe.sections {
        for (i, b) in s.data.iter().enumerate() {
            bus.write8(s.load_address + i as u32, *b).unwrap();
        }
    }
    let mut cpu = Cpu::new();
    let top = RAM_BASE + exe.ram_size;
    cpu.set_reg(14, top);
    cpu.set_pc(exe.entry_point);
    for r in 7..=11 {
        cpu.set_reg(r, 0xabcd0000 + r as u32);
    }
    for _ in 0..2_000_000 {
        if let Err(e) = cpu.step(&mut bus) {
            assert!(fault, "{h:?}: {e:?}");
            assert!(format!("{e:?}").contains("InvalidInstruction"));
            return 0;
        }
        if bus.read32(0xffff8040).unwrap() != 0xdeadbeef {
            assert!(!fault);
            assert_eq!(cpu.reg(14), top);
            for r in 7..=11 {
                assert_eq!(cpu.reg(r), 0xabcd0000 + r as u32);
            }
            return bus.read32(0xffff8050).unwrap();
        }
    }
    panic!("helper CPU step limit: {h:?}");
}
#[test]
fn ond_helpers_link_and_execute_without_a_host_float_device() {
    for (h, args, want) in [
        (
            Helper::F32Add,
            vec![1.0f32.to_bits(), 2.0f32.to_bits()],
            3.0f32.to_bits(),
        ),
        (
            Helper::F32Mul,
            vec![2.0f32.to_bits(), 3.0f32.to_bits()],
            6.0f32.to_bits(),
        ),
        (
            Helper::F32Div,
            vec![6.0f32.to_bits(), 2.0f32.to_bits()],
            3.0f32.to_bits(),
        ),
        (Helper::F32Neg, vec![0], 0x80000000),
        (
            Helper::F32FromI32,
            vec![(-7i32) as u32],
            (-7.0f32).to_bits(),
        ),
        (Helper::F32ToU32, vec![7.5f32.to_bits()], 7),
        (Helper::F32Eq, vec![0x7fc00000, 0x7fc00000], 0),
    ] {
        assert_eq!(execute_helper(h, &args, false), want, "{h:?}");
    }
}
#[test]
fn invalid_conversion_uses_trap_primitive_not_recursive_division_helper() {
    execute_helper(Helper::F32ToU32, &[0x7fc00000], true);
}
