use super::{tests::Fixture, *};
use crate::ir::object::{Object, RelocationKind, Section, SectionKind};
use kagura::Bus;
use ond_protocol::debug::{DebugOperation, ReturnAddressLocation};

#[test]
fn numeric_helper_call_slots_have_call_ranges_and_preserve_other_classifications() {
    let fixture = Fixture::new(
        "package main\nfunc udiv(a:u32,b:u32)->u32{return a/b}\nfunc urem(a:u32,b:u32)->u32{return a%b}\nfunc sdiv(a:i32,b:i32)->i32{return a/b}\nfunc srem(a:i32,b:i32)->i32{return a%b}\nfunc fdiv(a:f32,b:f32)->f32{return a/b}\nfunc fadd(a:f32,b:f32)->f32{return a+b}\nfunc fsub(a:f32,b:f32)->f32{return a-b}\nfunc fmul(a:f32,b:f32)->f32{return a*b}\nfunc feq(a:f32,b:f32)->bool{return a==b}\nfunc fne(a:f32,b:f32)->bool{return a!=b}\nfunc flt(a:f32,b:f32)->bool{return a<b}\nfunc fle(a:f32,b:f32)->bool{return a<=b}\nfunc fgt(a:f32,b:f32)->bool{return a>b}\nfunc fge(a:f32,b:f32)->bool{return a>=b}\nfunc fneg(a:f32)->f32{return -a}\nfunc itof(a:i32)->f32{return a as f32}\nfunc utof(a:u32)->f32{return a as f32}\nfunc ftoi(a:f32)->i32{return a as i32}\nfunc ftou(a:f32)->u32{return a as u32}\nfunc add(a:i32,b:i32)->i32{return a+b}\nfunc ineg(a:i32)->i32{return -a}\nfunc cast(a:i32)->i16{return a as i16}\nfunc identity(a:f32)->f32{return a}\nfunc main(){udiv(8,2)}\n",
    );
    let project = fixture.core();
    let selected = project
        .packages
        .iter()
        .map(|package| package.logical_path.clone())
        .collect();
    let output = codegen_selected_with_debug(&project, &selected, None).unwrap();
    let object = &output.objects[0];
    let debug = &output.debug[0];
    assert_eq!(
        object
            .relocations
            .iter()
            .filter(|r| r.kind == RelocationKind::CallSlot)
            .count(),
        20
    );
    for relocation in object
        .relocations
        .iter()
        .filter(|r| r.kind == RelocationKind::CallSlot)
    {
        for offset in [relocation.offset, relocation.offset + 4] {
            let range = debug
                .ranges
                .iter()
                .filter(|r| r.start <= offset && offset < r.end)
                .min_by_key(|r| r.end - r.start)
                .unwrap();
            let expected = if matches!(range.function.as_str(), "main.ftoi" | "main.ftou") {
                DebugOperation::InvalidConversion
            } else {
                DebugOperation::Call
            };
            assert_eq!(range.operation, expected, "{range:?}");
        }
    }
    for function in ["main.add", "main.ineg", "main.cast", "main.identity"] {
        let f = debug
            .functions
            .iter()
            .find(|f| f.symbol == function)
            .unwrap();
        assert!(
            !object.relocations.iter().any(|r| {
                r.kind == RelocationKind::CallSlot
                    && r.offset >= f.offset
                    && r.offset < f.offset + f.size
            }),
            "{function}"
        );
        assert!(
            debug
                .ranges
                .iter()
                .any(|r| { r.function == function && r.operation == DebugOperation::Source }),
            "{function}"
        );
        assert!(!debug.ranges.iter().any(|range| {
            range.function == function && range.operation == DebugOperation::Call
        }));
    }
    for function in ["main.ftoi", "main.ftou"] {
        assert!(debug.ranges.iter().any(|range| {
            range.function == function && range.operation == DebugOperation::InvalidConversion
        }));
    }
    assert!(
        debug
            .ranges
            .iter()
            .any(|r| r.operation == DebugOperation::DivisionByZero)
    );
    assert!(
        debug
            .ranges
            .iter()
            .any(|r| r.operation == DebugOperation::SignedDivisionOverflow)
    );
}

#[test]
fn helper_stack_overflow_keeps_near_and_far_caller_unwind_metadata() {
    let fixture = Fixture::new(
        "package main\nfunc calc(x:u32,y:u32)->u32{return x/y}\nfunc main(){var z=calc(8,2)}\n",
    );
    let compilation = crate::pipeline::compile_project(&fixture.0).unwrap();
    let supplied = CodegenOutput {
        objects: compilation.objects,
        debug: compilation.debug_objects,
    };
    let main_object = supplied
        .objects
        .iter()
        .find(|object| {
            object
                .symbols
                .iter()
                .any(|symbol| symbol.name == "main.calc")
        })
        .unwrap();
    let main_symbol_offset = main_object
        .symbols
        .iter()
        .find(|symbol| symbol.name == "main.calc")
        .unwrap()
        .offset;
    let helper_slot = main_object
        .relocations
        .iter()
        .find(|relocation| {
            relocation.kind == RelocationKind::CallSlot
                && relocation.target.contains("div")
                && relocation.target != "main.main"
        })
        .unwrap();
    let startup = crate::program::program_startup(&[], "main.main", &[0x60000000]);
    let padding = || Object {
        name: "padding".into(),
        sections: vec![Section {
            kind: SectionKind::Text,
            alignment: 4,
            data: vec![0; 140_000],
            memory_size: 140_000,
        }],
        symbols: vec![],
        relocations: vec![],
    };
    for far in [false, true] {
        let mut objects = vec![startup.clone()];
        let user_objects = supplied.objects.iter().filter(|object| {
            object
                .symbols
                .iter()
                .any(|symbol| symbol.name == "main.calc")
        });
        let runtime_objects = supplied.objects.iter().filter(|object| {
            !object
                .symbols
                .iter()
                .any(|symbol| symbol.name == "main.calc")
        });
        objects.extend(user_objects.cloned());
        if far {
            objects.push(padding());
        }
        objects.extend(runtime_objects.cloned());
        let linked = crate::linker::link_objects_with_debug(
            &objects,
            &supplied.debug,
            &crate::linker::LinkPlan {
                read_only: None,
                memory_base: 0x1000,
                memory_size: 1024 * 1024,
                stack_size: 8192,
                entry_symbol: "__ond.entry".into(),
                heap_base_symbol: "__ond_heap_base".into(),
                stack_bottom_symbol: "__ond_stack_bottom".into(),
            },
        )
        .unwrap();
        let function = linked
            .debug
            .functions
            .iter()
            .find(|function| function.symbol == "main.calc")
            .unwrap();
        let caller_slot = function.address + helper_slot.offset - main_symbol_offset;
        let section = linked
            .image
            .sections
            .iter()
            .find(|section| {
                section.load_address <= caller_slot
                    && caller_slot < section.load_address + section.data.len() as u32
            })
            .unwrap();
        let first = u32::from_le_bytes(
            section.data[(caller_slot - section.load_address) as usize..][..4]
                .try_into()
                .unwrap(),
        );
        assert_eq!(first as u16 as i16 == 1, far, "far={far}");

        let (mut cpu, mut bus, _) = super::tests::machine(&linked.image);
        let mut arrived = false;
        for _ in 0..200_000 {
            if cpu.pc() == caller_slot {
                arrived = true;
                break;
            }
            cpu.step(&mut bus).unwrap();
        }
        assert!(arrived, "helper call slot not reached; far={far}");
        cpu.set_reg(14, linked.debug.stack_bottom);
        let mut fault_pc = None;
        for _ in 0..100 {
            if cpu.step(&mut bus).is_err() {
                fault_pc = Some(cpu.pc());
                break;
            }
        }
        let fault_pc = fault_pc.expect("helper stack overflow");
        let fault_range = linked
            .debug
            .ranges
            .iter()
            .filter(|range| range.start <= fault_pc && fault_pc < range.end)
            .min_by_key(|range| range.end - range.start)
            .unwrap();
        assert_eq!(fault_range.operation, DebugOperation::StackOverflow);
        let helper = linked
            .debug
            .functions
            .iter()
            .find(|function| function.symbol == fault_range.function)
            .unwrap();
        assert_eq!(helper.symbol, helper_slot.target);
        let row = helper
            .unwind
            .iter()
            .filter(|row| row.address <= fault_pc)
            .max_by_key(|row| row.address)
            .unwrap();
        assert_eq!(row.return_address, ReturnAddressLocation::LinkRegister);
        assert_eq!(row.cfa_sp_offset, 0);
        assert_eq!(cpu.reg(14), linked.debug.stack_bottom);
        let return_pc = cpu.reg(15);
        assert_eq!(return_pc, caller_slot + 4);
        let caller = linked
            .debug
            .ranges
            .iter()
            .filter(|range| range.start <= return_pc && return_pc < range.end)
            .min_by_key(|range| range.end - range.start)
            .unwrap();
        assert_eq!(caller.function, "main.calc");
        assert_eq!(caller.operation, DebugOperation::Call);
        let source = caller.source.as_ref().unwrap();
        assert_eq!(source.path, "main.ond");
        assert_eq!(source.line, 2);
        assert!((35..=37).contains(&source.column), "{source:?}");
    }
}

#[test]
fn executed_numeric_faults_match_the_linked_debug_operation() {
    for (source, expected) in [
        (
            "package main\nfunc conv(value:f32)->i32{return value as i32}\nfunc main(){var result=conv(2147483648.0)}",
            DebugOperation::InvalidConversion,
        ),
        (
            "package main\nfunc main(){var x:f32=-2147483904.0;var y=x as i32}",
            DebugOperation::InvalidConversion,
        ),
        (
            "package main\nfunc main(){var x:f32=4294967296.0;var y=x as u32}",
            DebugOperation::InvalidConversion,
        ),
        (
            "package main\nfunc main(){var x:i32=1;var z:i32=0;var y=x/z}",
            DebugOperation::DivisionByZero,
        ),
        (
            "package main\nfunc main(){var x:i32=1;var z:i32=0;var y=x%z}",
            DebugOperation::DivisionByZero,
        ),
        (
            "package main\nfunc main(){var x:u32=1;var z:u32=0;var y=x/z}",
            DebugOperation::DivisionByZero,
        ),
        (
            "package main\nfunc main(){var x:u32=1;var z:u32=0;var y=x%z}",
            DebugOperation::DivisionByZero,
        ),
    ] {
        let fixture = Fixture::new(source);
        let compilation = crate::pipeline::compile_project(&fixture.0).unwrap();
        let startup = crate::program::program_startup(
            &[],
            "main.main",
            &[
                crate::kagura_encoding::word(0, 1, 0, 0, -1),
                crate::kagura_encoding::word(2, 1, 1, 0, 15),
                crate::kagura_encoding::word(0, 1, 1, 0, 64),
                crate::kagura_encoding::word(11, 0, 1, 0, 0),
                crate::kagura_encoding::word(12, 0, 0, 0, -1),
            ],
        );
        let mut objects = vec![startup];
        objects.extend(compilation.objects.clone());
        let linked = crate::linker::link_objects_with_debug(
            &objects,
            &compilation.debug_objects,
            &crate::linker::LinkPlan {
                read_only: None,
                memory_base: 0x1000,
                memory_size: 65536,
                stack_size: 8192,
                entry_symbol: "__ond.entry".into(),
                heap_base_symbol: "__ond_heap_base".into(),
                stack_bottom_symbol: "__ond_stack_bottom".into(),
            },
        )
        .unwrap();
        let (mut cpu, mut bus, _) = super::tests::machine(&linked.image);
        let fault_pc = (0..100_000)
            .find_map(|_| cpu.step(&mut bus).err().map(|_| cpu.pc()))
            .expect("program must fault before normal exit");
        assert_eq!(bus.read32(0xffff8040).unwrap(), 0xdeadbeef);
        let range = linked
            .debug
            .ranges
            .iter()
            .filter(|range| range.start <= fault_pc && fault_pc < range.end)
            .min_by_key(|range| range.end - range.start)
            .unwrap_or_else(|| {
                panic!("fault PC {fault_pc:#x} lacks linked debug metadata: {source}")
            });
        assert_eq!(range.operation, expected, "{source}: {range:?}");
        if expected == DebugOperation::InvalidConversion {
            assert!(range.function.ends_with("__sf_invalid"), "{range:?}");
        } else {
            assert_eq!(range.function, "main.main", "{range:?}");
        }
    }
}

#[test]
fn fault01_raw_unaligned_reports_load_instruction_pc() {
    let source = "package main\nfunc main(){var p=536870913 as u32 as *u32;_=*p}\n";
    let fixture = Fixture::new(source);
    let compilation = crate::pipeline::compile_project(&fixture.0).unwrap();
    let startup = crate::program::program_startup(
        &[],
        "main.main",
        &[0x0100ffff, 0x2110000f, 0x01100040, 0xb0100000, 0xc000ffff],
    );
    let mut objects = vec![startup];
    objects.extend(compilation.objects);
    let linked = crate::linker::link_objects_with_debug(
        &objects,
        &compilation.debug_objects,
        &crate::linker::LinkPlan {
            read_only: None,
            memory_base: 0x1000,
            memory_size: 0x800000,
            stack_size: 0x100000,
            entry_symbol: "__ond.entry".into(),
            heap_base_symbol: "__ond_heap_base".into(),
            stack_bottom_symbol: "__ond_stack_bottom".into(),
        },
    )
    .unwrap();
    let (mut cpu, mut bus, _) = super::tests::machine(&linked.image);
    bus.map_device(0x20000000, 0x200, ram::Ram::new(0x200))
        .unwrap();
    for word in 0..64 {
        bus.write32(0x20000100 + word * 4, 0xdeadbeef).unwrap();
    }
    let (before_pc, fault) = (0..2_000_000)
        .find_map(|_| {
            let pc = cpu.pc();
            cpu.step(&mut bus).err().map(|fault| (pc, fault))
        })
        .expect("FAULT-01.raw-unaligned must fault");
    assert!(
        matches!(
            fault,
            kagura::Fault::UnalignedAccess {
                addr: 536870913,
                width: kagura::AccessWidth::Word,
                operation: kagura::AccessOperation::Load,
                ..
            }
        ),
        "FAULT-01.raw-unaligned: {fault:?}"
    );
    assert_eq!(
        fault.faulting_pc(),
        before_pc,
        "FAULT-01.raw-unaligned actual faulting load PC"
    );
    let range = linked
        .debug
        .ranges
        .iter()
        .filter(|r| r.start <= before_pc && before_pc < r.end)
        .min_by_key(|r| r.end - r.start)
        .expect("load debug range");
    assert_eq!(range.operation, DebugOperation::MemoryRead);
    assert_eq!(range.function, "main.main");
    assert_eq!(bus.read32(0xffff8040).unwrap(), 0xdeadbeef);
    for word in 0..64 {
        assert_eq!(bus.read32(0x20000100 + word * 4).unwrap(), 0xdeadbeef);
    }
}
