use super::{frame::Frame, tests::Fixture, *};
use crate::linker::{LinkPlan, LinkedOutput, link_objects_with_debug};
use kagura::{Bus, FaultCode};
use ond_compiler_core::Compilation;
use ond_protocol::debug::{DebugOperation, ReturnAddressLocation};

const EXIT: u32 = 0xffff8040;
const SOURCE: &str = r#"package main
var trace: u32
var result: u32
func mark(n:u32)->u32 { trace=trace*10+n; return n }
func many(a:u32,b:u32,c:u32,d:u32,w:u64)->(u32,u32,u32,u32,u32) {
    defer { trace=trace*10+6 }
    return a+trace,b+20,((w>>32) as u32)+c,(w as u32)+d,c+d+50
}
func main() {
    var a,b,_,d,e=many(mark(1),mark(2),mark(3),mark(4),0x1122334455667788)
    result=a+b+d+e
}
"#;

fn image(frontend: &Compilation, stack_size: u32, omit: bool) -> LinkedOutput {
    let project = &frontend.mir;
    let paths = frontend.loaded.packages[0]
        .files
        .iter()
        .map(|id| (*id, "main.ond".into()))
        .collect();
    let selected = project
        .packages
        .iter()
        .map(|package| package.logical_path.clone())
        .collect();
    let sources = Some((&frontend.loaded.sources, &paths));
    let code = if omit {
        codegen_selected_with_debug(project, &selected, sources)
    } else {
        codegen_selected_using(
            project,
            &selected,
            sources,
            raw_induction::lower_with_discarded_call_results,
        )
    }
    .unwrap();
    let code = runtime::supply_with_debug(code.objects, code.debug).unwrap();
    let metadata = crate::program::ProgramMetadata::from_mir(project).unwrap();
    let mut objects = vec![crate::test_support::startup_object(
        &metadata.initializers,
        &metadata.entry_symbol,
    )];
    objects.extend(code.objects);
    link_objects_with_debug(
        &objects,
        &code.debug,
        &LinkPlan {
            read_only: None,
            memory_base: 0x1000,
            memory_size: 1 << 20,
            stack_size,
            entry_symbol: "__ond.entry".into(),
            heap_base_symbol: "__ond_heap_base".into(),
            stack_bottom_symbol: "__ond_stack_bottom".into(),
        },
    )
    .unwrap()
}

#[derive(Debug, PartialEq, Eq)]
struct Run {
    steps: u32,
    peak: u32,
    record: u32,
    writes: Vec<u32>,
    reads: Vec<u32>,
    spills: Vec<u32>,
}

fn run(linked: &LinkedOutput, frame: &Frame, results: &[mir::ValueId]) -> Run {
    let (mut cpu, mut bus, top) = tests::machine(&linked.image);
    let symbols = &linked.image.symbols;
    let caller = symbols["main.main"];
    let callee = symbols["main.many"];
    let slots: Vec<_> = results.iter().map(|value| frame.value(*value)).collect();
    let mut run = Run {
        steps: 0,
        peak: 0,
        record: 0,
        writes: vec![],
        reads: vec![],
        spills: vec![],
    };
    let mut call = None;
    for step in 1..10_000 {
        let pc = cpu.pc();
        let sp = cpu.reg(14);
        run.peak = run.peak.max(top - sp);
        if pc == callee {
            let record = cpu.reg(1);
            run.record = record;
            assert_eq!(sp, top - frame.size);
            assert_eq!(
                (cpu.reg(2), cpu.reg(3), cpu.reg(4), cpu.reg(5)),
                (1, 2, 3, 4)
            );
            assert_eq!(cpu.reg(6), 0x55667788);
            assert_eq!(bus.read32(sp).unwrap(), 0x11223344);
            let return_pc = cpu.reg(15);
            let range = linked
                .debug
                .ranges
                .iter()
                .find(|r| {
                    r.operation == DebugOperation::Call && (r.start..r.end).contains(&return_pc)
                })
                .unwrap();
            assert_eq!(range.function, "main.main");
            assert_eq!(range.source.as_ref().unwrap().line, 10);
            call = Some((record, return_pc, range.end, sp, &range.source));
        }
        if let Some((record, return_pc, end, caller_sp, source)) = call
            && pc < end
        {
            let word = bus.read32(pc).unwrap();
            let reg = |shift| cpu.reg(((word >> shift) & 0xfu32) as usize);
            let address = reg(20)
                .wrapping_add(reg(16))
                .wrapping_add_signed(i32::from(word as i16));
            let opcode = word >> 28;
            if (return_pc..end).contains(&pc) {
                let operation = if opcode == 7 && (record..record + 20).contains(&address) {
                    run.reads.push(address - record);
                    Some(DebugOperation::MemoryRead)
                } else if opcode == 11 && slots.contains(&address.wrapping_sub(caller_sp)) {
                    run.spills.push(address - caller_sp);
                    Some(DebugOperation::MemoryWrite)
                } else {
                    None
                };
                if let Some(operation) = operation {
                    let range = linked
                        .debug
                        .ranges
                        .iter()
                        .filter(|r| (r.start..r.end).contains(&pc))
                        .min_by_key(|r| r.end - r.start)
                        .unwrap();
                    assert_eq!(range.operation, operation);
                    assert_eq!(&range.source, source);
                }
            } else if !(caller..end).contains(&pc)
                && opcode == 11
                && (record..record + 20).contains(&address)
            {
                run.writes.push(address - record);
                assert_eq!(
                    reg(24),
                    [1235, 22, 0x11223347, 0x5566778c, 57][((address - record) / 4) as usize]
                );
            }
        }
        cpu.step(&mut bus).unwrap();
        if bus.read32(EXIT).unwrap() == 0 {
            assert_eq!(cpu.reg(14), top);
            assert_eq!(bus.read32(symbols["main.result"]).unwrap(), 0x55667cae);
            assert_eq!(bus.read32(symbols["main.trace"]).unwrap(), 12346);
            run.steps = step;
            return run;
        }
    }
    panic!("CPU step limit");
}

fn assert_stack_overflow(linked: &LinkedOutput) {
    let (mut cpu, mut bus, _) = tests::machine(&linked.image);
    for _ in 0..10_000 {
        if let Err(fault) = cpu.step(&mut bus) {
            assert_eq!(fault.code(), FaultCode::InvalidInstruction);
            let pc = fault.faulting_pc();
            let range = linked
                .debug
                .ranges
                .iter()
                .find(|r| (r.start..r.end).contains(&pc))
                .unwrap();
            assert_eq!(range.operation, DebugOperation::StackOverflow);
            assert_eq!(range.function, "main.many");
            assert_eq!(range.source.as_ref().unwrap().line, 5);
            let child = linked
                .debug
                .functions
                .iter()
                .find(|f| f.symbol == range.function)
                .unwrap();
            let row = child
                .unwind
                .iter()
                .rev()
                .find(|row| row.address <= pc)
                .unwrap();
            assert_eq!(row.cfa_sp_offset, 0);
            assert_eq!(row.return_address, ReturnAddressLocation::LinkRegister);
            let caller = linked
                .debug
                .functions
                .iter()
                .find(|f| f.symbol == "main.main")
                .unwrap();
            assert!((caller.address..caller.address + caller.size).contains(&cpu.reg(15)));
            return;
        }
    }
    panic!("expected StackOverflow");
}

#[test]
fn discarded_call_results_preserve_abi_defer_debug_and_stack() {
    let fixture = Fixture::new(SOURCE);
    let frontend = ond_compiler_core::compile_project(&fixture.0).unwrap();
    let project = &frontend.mir;
    let main = project.packages[0]
        .functions
        .iter()
        .find(|f| f.name == "main.main")
        .unwrap();
    let abi = layout::abi(
        &project.types,
        &mir::FunctionType {
            parameters: vec![],
            returns: vec![],
        },
        main.span,
    )
    .unwrap();
    let frame = Frame::new(main, &project.types, &abi, &Default::default()).unwrap();
    let call = main
        .blocks
        .iter()
        .flat_map(|b| &b.instructions)
        .find(|i| i.results.len() == 5)
        .unwrap();

    let before = image(&frontend, 8192, false);
    let after = image(&frontend, 8192, true);
    let text = |linked: &LinkedOutput| {
        linked
            .image
            .sections
            .iter()
            .find(|s| s.kind == SectionKind::Text)
            .unwrap()
            .memory_size
    };
    assert_eq!(text(&before) - text(&after), 4);
    assert_eq!(
        before
            .debug
            .functions
            .iter()
            .map(|f| f.frame_size)
            .collect::<Vec<_>>(),
        after
            .debug
            .functions
            .iter()
            .map(|f| f.frame_size)
            .collect::<Vec<_>>()
    );
    let before = run(&before, &frame, &call.results);
    let after = run(&after, &frame, &call.results);
    assert_eq!(before.steps - after.steps, 1);
    assert_eq!(before.peak, after.peak);
    assert_eq!(before.record, after.record);
    assert_eq!(before.writes, [0, 4, 8, 12, 16]);
    assert_eq!(before.writes, after.writes);
    assert_eq!(before.reads, before.writes);
    assert_eq!(before.reads, after.reads);
    assert_eq!(
        before.spills,
        call.results
            .iter()
            .map(|value| frame.value(*value))
            .collect::<Vec<_>>()
    );
    let mut live_spills = before.spills;
    live_spills.remove(2);
    assert_eq!(after.spills, live_spills);
    for omit in [false, true] {
        assert_eq!(
            run(&image(&frontend, after.peak, omit), &frame, &call.results).peak,
            after.peak
        );
        assert_stack_overflow(&image(&frontend, after.peak - 4, omit));
    }
}

#[test]
fn discarded_call_results_reject_opaque_calls_and_addressable_frames() {
    for source in [
        SOURCE.replace("a+trace,b+20,((w>>32) as u32)+c,", "a+trace,b+20,mark(c),"),
        SOURCE.replace("    result=a+b+d+e", "    var p=&a\n    result=*p+b+d+e"),
        SOURCE.replace(
            "    result=a+b+d+e",
            "    var p=many\n    p(1,2,3,4,0)\n    result=a+b+d+e",
        ),
        SOURCE
            .replace("var a,b,_,d,e", "var a,b,c,d,e")
            .replace("result=a+b+d+e", "result=a+b+c+d+e"),
        SOURCE.replace(
            "var a,b,_,d,e=many(mark(1),mark(2),mark(3),mark(4),0x1122334455667788)\n    result=a+b+d+e",
            "many(mark(1),mark(2),mark(3),mark(4),0x1122334455667788)",
        ),
    ] {
        let fixture = Fixture::new(&source);
        assert_fallback(&fixture);
    }
    let fixture = Fixture::new(
        r#"package main
import "test/shared"
var result: u32
func main() { var _,a,_,_,_=shared.Many(); result=a }
"#,
    );
    std::fs::create_dir(fixture.0.join("shared")).unwrap();
    std::fs::write(
        fixture.0.join("shared/main.ond"),
        "package shared\nfunc Many()->(u32,u32,u32,u32,u32) { return 1,2,3,4,5 }",
    )
    .unwrap();
    assert_fallback(&fixture);
}

fn assert_fallback(fixture: &Fixture) {
    let frontend = ond_compiler_core::compile_project(&fixture.0).unwrap();
    let before = image(&frontend, 8192, false);
    let after = image(&frontend, 8192, true);
    assert_eq!(before.debug, after.debug);
    assert_eq!(before.image.sections.len(), after.image.sections.len());
    for (left, right) in before.image.sections.iter().zip(&after.image.sections) {
        assert_eq!(left.load_address, right.load_address);
        assert_eq!(left.data, right.data);
    }
}

#[test]
fn discarded_call_results_keep_narrow_loads_and_float_bits() {
    let fixture = Fixture::new(
        r#"package main
type Count u32
var byte: u8
var narrow: i16
var bits: f32
var count: Count
func many()->(bool,u8,i16,f32,Count) { return true,171,-4660,1.25,99 as Count }
func main() {
    var _,a,b,c,d=many()
    byte=a
    narrow=b
    bits=c
    count=d
}
"#,
    );
    let frontend = ond_compiler_core::compile_project(&fixture.0).unwrap();
    let mut steps = Vec::new();
    for omit in [false, true] {
        let linked = image(&frontend, 8192, omit);
        let results = &linked.image.symbols;
        let (mut cpu, mut bus, _) = tests::machine(&linked.image);
        let mut return_pc = None;
        let mut reads = Vec::new();
        for step in 1..10_000 {
            let pc = cpu.pc();
            if pc == results["main.many"] {
                let address = cpu.reg(15);
                let end = linked
                    .debug
                    .ranges
                    .iter()
                    .find(|r| {
                        r.operation == DebugOperation::Call && (r.start..r.end).contains(&address)
                    })
                    .unwrap()
                    .end;
                return_pc = Some((address, end, cpu.reg(1)));
            }
            if let Some((address, end, record)) = return_pc {
                if pc == address {
                    // Discarded and used results alike retain their padded ABI slots.
                    let words: [u32; 5] =
                        std::array::from_fn(|i| bus.read32(record + 4 * i as u32).unwrap());
                    assert_eq!(words, [1, 171, 0xedcc, 0x3fa00000, 99]);
                }
                if (address..end).contains(&pc) {
                    let word = bus.read32(pc).unwrap();
                    let width = match word >> 28 {
                        4 => 1,
                        5 => 2,
                        7 => 4,
                        _ => 0,
                    };
                    if width > 0 {
                        let reg = |shift| cpu.reg(((word >> shift) & 0xfu32) as usize);
                        let offset = reg(20)
                            .wrapping_add(reg(16))
                            .wrapping_add_signed(i32::from(word as i16))
                            .wrapping_sub(record);
                        if offset < 20 {
                            reads.push((offset, width));
                        }
                    }
                }
            }
            cpu.step(&mut bus).unwrap();
            if bus.read32(EXIT).unwrap() == 0 {
                assert_eq!(bus.read8(results["main.byte"]).unwrap(), 171);
                assert_eq!(bus.read16(results["main.narrow"]).unwrap(), 0xedcc);
                assert_eq!(bus.read32(results["main.bits"]).unwrap(), 0x3fa00000);
                assert_eq!(bus.read32(results["main.count"]).unwrap(), 99);
                assert_eq!(reads, [(0, 1), (4, 1), (8, 2), (12, 4), (16, 4)]);
                steps.push(step);
                break;
            }
        }
    }
    let [before, after] = steps[..] else {
        panic!("CPU step limit");
    };
    assert_eq!(before - after, 1);
}
