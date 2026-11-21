use super::super::tests::{Fixture, machine};
use super::runner::*;
use crate::linker::{LinkPlan, link_objects_with_debug};
use kagura::{Bus, FaultCode};
use ond_compiler_core::{mir, test_vectors::INTEGERS};
use ond_protocol::debug::DebugOperation;
use ram::Ram;

fn compile_for_driver(fixture: &Fixture) -> mir::Project {
    let loaded = ond_compiler_core::project::load_project(&fixture.0).unwrap();
    ond_compiler_core::compile_loaded_project_with_generation(loaded)
        .unwrap()
        .0
        .mir
}

#[test]
fn narrow_f32_roundtrip_executes_all_131584_values_on_kagura() {
    let mut checked = 0;
    for ty in INTEGERS.iter().filter(|ty| ty.bits <= 16) {
        let name = ty.name;
        let id = format!("CAST-04.{name}.all-values");
        let fixture = Fixture::new(&format!(
            "package main\nfunc roundtrip(x:{name})->{name}{{return (x as f32) as {name}}}\n\
            func main(){{var x=(*({INPUT} as u32 as *u32)) as {name};*({OUTPUT} as u32 as *u32)=roundtrip(x) as u32}}\n"
        ));
        let project = compile_for_driver(&fixture);
        let roundtrip = project.packages[0]
            .functions
            .iter()
            .find(|function| function.name == "main.roundtrip")
            .unwrap();
        assert!(
            roundtrip
                .blocks
                .iter()
                .flat_map(|block| &block.instructions)
                .all(|instruction| {
                    !matches!(instruction.kind, mir::InstructionKind::Cast { .. })
                }),
            "{name}"
        );
        let image = crate::test_support::build_image(
            &project,
            crate::ProjectConfig {
                ram_size: 65536,
                stack_size: 8192,
            },
        )
        .unwrap();
        for n in ty.min..=ty.max {
            let word = n as u32;
            run(
                &image,
                Case {
                    id: &id,
                    specs: &["CAST-04"],
                    inputs: &[word],
                    output: &[word],
                    end: End::Return,
                    steps: 2_000_000,
                },
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 131584);
}

#[test]
fn narrow_f32_roundtrip_leaves_other_conversions_untouched() {
    let fixture = Fixture::new(
        "package main
        type N u16
        type F f32
        func wide_signed(x:i32)->i32{return (x as f32) as i32}
        func wide_unsigned(x:u32)->u32{return (x as f32) as u32}
        func different(x:u16)->i16{return (x as f32) as i16}
        func defined_int(x:N)->N{return (x as f32) as N}
        func defined_float(x:u16)->u16{return (x as F) as u16}
        func changed(x:i16)->i16{return ((x as f32)+1.0) as i16}
        func unary_plus(x:u16)->u16{return (+(x as f32)) as u16}
        func converted(x:u16)->u16{return ((x as f32) as u32) as u16}
        func local(x:u16)->u16{var f=x as f32;return f as u16}
        func main(){}",
    );
    let original = fixture.core();
    let optimized = compile_for_driver(&fixture);
    assert_eq!(
        optimized.packages[0].functions,
        original.packages[0].functions
    );
}

#[test]
fn narrow_f32_roundtrip_preserves_order_and_faults_from_input_producers() {
    let trace = OUTPUT + 4;
    let fixture = Fixture::new(&format!(
        r#"package main
        func mark(tag:u32,value:i16)->i16{{var p={trace} as u32 as *u32;*p=(*p)*10+tag;return value}}
        func combine(a:i16,b:i16)->i16{{return a+b}}
        func main(){{
            *({trace} as u32 as *u32)=0
            var mode=*({INPUT} as u32 as *u32)
            *({OUTPUT} as u32 as *u32)=combine((mark(1,-10) as f32) as i16,(mark(2,5) as f32) as i16) as u32
            if mode==1{{var bad=536870913 as u32 as *i16;_ = ((*bad) as f32) as i16}}
            if mode==2{{trap "explicit after round trip"}}
        }}"#
    ));
    let image = crate::test_support::build_image(&compile_for_driver(&fixture), config()).unwrap();
    for (input, end) in [
        (0, End::Return),
        (1, End::Fault(FaultCode::UnalignedAccess)),
        (2, End::Fault(FaultCode::InvalidInstruction)),
    ] {
        run(
            &image,
            Case {
                id: "narrow-roundtrip.ordered-effects",
                specs: &["CAST-04", "FAULT-01"],
                inputs: &[input],
                output: &[(-5i32) as u32, 12],
                end,
                steps: 2_000_000,
            },
        );
    }
}

#[test]
fn narrow_f32_roundtrip_can_reduce_stack_demand_and_enable_following_write() {
    let fixture = Fixture::new(&format!(
        "package main\n\
        func roundtrip(x:u16)->u16{{return (x as f32) as u16}}\n\
        func main(){{var x=(*({INPUT} as u32 as *u32)) as u16;*({OUTPUT} as u32 as *u32)=roundtrip(x) as u32}}\n"
    ));
    let baseline = fixture.core();
    let optimized = compile_for_driver(&fixture);
    for (project, stack_size, expected_fault) in [
        (&baseline, 512, Some(".__ond_f32_from_u32")),
        (&optimized, 512, None),
        (&optimized, 32, Some("main.main")),
    ] {
        let generated =
            super::super::codegen_with_debug(project, &Default::default(), &Default::default())
                .unwrap();
        let mut objects = vec![crate::test_support::startup_object(&[], "main.main")];
        objects.extend(generated.objects);
        let linked = link_objects_with_debug(
            &objects,
            &generated.debug,
            &LinkPlan {
                read_only: None,
                memory_base: 0x1000,
                memory_size: config().ram_size,
                stack_size,
                entry_symbol: "__ond.entry".into(),
                heap_base_symbol: "__ond_heap_base".into(),
                stack_bottom_symbol: "__ond_stack_bottom".into(),
            },
        )
        .unwrap();

        if let Some(function_suffix) = expected_fault {
            let (mut cpu, mut bus, _) = machine(&linked.image);
            bus.map_device(INPUT, 512, Ram::new(512)).unwrap();
            bus.write32(INPUT, 1).unwrap();
            bus.write32(OUTPUT, SENTINEL).unwrap();
            let fault = (0..2_000_000)
                .find_map(|_| cpu.step(&mut bus).err())
                .expect("stack fault");
            assert_eq!(fault.code(), FaultCode::InvalidInstruction);
            let pc = fault.faulting_pc();
            let range = linked
                .debug
                .ranges
                .iter()
                .filter(|range| (range.start..range.end).contains(&pc))
                .min_by_key(|range| range.end - range.start)
                .unwrap();
            assert_eq!(range.operation, DebugOperation::StackOverflow);
            assert!(range.function.ends_with(function_suffix), "{range:?}");
            assert_eq!(bus.read32(OUTPUT).unwrap(), SENTINEL);
            assert_eq!(bus.read32(0xffff8040).unwrap(), SENTINEL);
        } else {
            run(
                &linked.image,
                Case {
                    id: "narrow-roundtrip.stack-boundary",
                    specs: &["CAST-04", "FAULT-01"],
                    inputs: &[1],
                    output: &[1],
                    end: End::Return,
                    steps: 2_000_000,
                },
            );
        }
    }
}
