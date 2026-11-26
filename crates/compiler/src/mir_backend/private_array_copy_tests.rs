use super::{frame::Frame, tests::*, *};
use crate::{
    linker::{LinkPlan, LinkedOutput, link_objects_with_debug},
    program::ProgramMetadata,
};
use kagura::{Bus, BusFault, DefaultBus, Fault, FaultCode};
use ond_protocol::debug::{DebugOperation, FunctionDebug, ReturnAddressLocation};
use std::collections::BTreeMap;

const CALC: &str = "main.calc";
const EXIT: u32 = 0xffff8040;
const OUTPUT: u32 = 0xffff8050;
const IO: u32 = 0x20000000;
const STACK: u32 = 65536;

struct Comparison {
    mir: mir::Project,
    codes: [CodegenOutput; 2],
    metadata: ProgramMetadata,
}

impl Comparison {
    fn compile(source: &str) -> Self {
        let fixture = Fixture::new(source);
        let compiled = crate::compile_project(&fixture.0)
            .unwrap_or_else(|error| panic!("{error:?}\n{source}"));
        let paths = compiled.loaded.packages[0]
            .files
            .iter()
            .map(|id| (*id, "main.ond".to_owned()))
            .collect::<BTreeMap<_, _>>();
        let selected = compiled
            .mir
            .packages
            .iter()
            .map(|p| p.logical_path.clone())
            .collect();
        let raw = codegen_selected_using(
            &compiled.mir,
            &selected,
            Some((&compiled.loaded.sources, &paths)),
            raw_induction::lower_without_private_array_copies,
        )
        .unwrap();
        let before = runtime::supply_with_debug(raw.objects, raw.debug).unwrap();
        let after = CodegenOutput {
            objects: compiled.objects,
            debug: compiled.debug_objects,
        };
        let metadata = ProgramMetadata::from_mir(&compiled.mir).unwrap();
        Self {
            mir: compiled.mir,
            codes: [before, after],
            metadata,
        }
    }

    fn link(&self, code: &CodegenOutput, stack_size: u32) -> LinkedOutput {
        let mut objects = vec![crate::test_support::startup_object(
            &self.metadata.initializers,
            &self.metadata.entry_symbol,
        )];
        objects.extend_from_slice(&code.objects);
        link_objects_with_debug(
            &objects,
            &code.debug,
            &LinkPlan {
                read_only: None,
                memory_base: 0x1000,
                memory_size: 262144,
                stack_size,
                entry_symbol: "__ond.entry".into(),
                heap_base_symbol: "__ond_heap_base".into(),
                stack_bottom_symbol: "__ond_stack_bottom".into(),
            },
        )
        .unwrap()
    }

    fn frames(&self, name: &str) -> ([Frame; 2], private_array_copy::Plan) {
        let types = &self.mir.types;
        let f = self.mir.packages[0]
            .functions
            .iter()
            .find(|f| f.name == name)
            .unwrap();
        let signature = mir::FunctionType {
            parameters: f
                .parameters
                .iter()
                .map(|id| f.locals[id.0 as usize].ty)
                .collect(),
            returns: f.returns.clone(),
        };
        let abi = layout::abi(types, &signature, f.span).unwrap();
        let before = private_array_copy::Plan::default();
        assert!(super::local_aggregate::LocalAggregates::new(types, f, &abi).is_none());
        let after = private_array_copy::Plan::new(types, f, &abi);
        (
            [&before, &after].map(|p| Frame::new(f, types, &abi, p).unwrap()),
            after,
        )
    }

    fn local(&self, function: &str, name: &str) -> mir::LocalId {
        self.mir.packages[0]
            .functions
            .iter()
            .find(|f| f.name == function)
            .unwrap()
            .locals
            .iter()
            .find(|l| l.name.as_deref() == Some(name))
            .unwrap()
            .id
    }

    fn assert_unchanged(&self) {
        let [before, after] = &self.codes;
        assert_eq!(before.debug, after.debug);
        assert_eq!(
            format!("{:?}", before.objects),
            format!("{:?}", after.objects)
        );
    }
}

struct CountStack {
    bus: DefaultBus,
    stack: std::ops::Range<u32>,
    reads: u32,
    writes: u32,
}
macro_rules! count_stack_accesses {
    ($(($read:ident, $write:ident, $ty:ty)),+ $(,)?) => {
        impl Bus for CountStack {
            $(
                fn $read(&mut self, addr: u32) -> Result<$ty, BusFault> {
                    self.reads += u32::from(self.stack.contains(&addr));
                    self.bus.$read(addr)
                }
                fn $write(&mut self, addr: u32, value: $ty) -> Result<(), BusFault> {
                    self.writes += u32::from(self.stack.contains(&addr));
                    self.bus.$write(addr, value)
                }
            )+
        }
    };
}
count_stack_accesses!(
    (read8, write8, u8),
    (read16, write16, u16),
    (read32, write32, u32)
);

#[derive(Debug)]
struct Observation {
    steps: usize,
    stack_reads: u32,
    stack_writes: u32,
    peak: u32,
    sp: u32,
    ra: u32,
    exit: u32,
    result: u32,
    fault: Option<Fault>,
}

fn run(linked: &LinkedOutput, setup: impl FnOnce(&mut DefaultBus)) -> (Observation, DefaultBus) {
    let (mut cpu, mut bus, top) = machine(&linked.image);
    bus.map_device(IO, 64, ram::Ram::new(64)).unwrap();
    setup(&mut bus);
    let mut counted = CountStack {
        bus,
        stack: linked.debug.stack_bottom..top,
        reads: 0,
        writes: 0,
    };
    let mut low = top;
    for steps in 1..=200_000 {
        let fault = cpu.step(&mut counted).err();
        low = low.min(cpu.reg(14));
        let exit = counted.bus.read32(EXIT).unwrap();
        if fault.is_some() || exit == 0 {
            return (
                Observation {
                    steps,
                    stack_reads: counted.reads,
                    stack_writes: counted.writes,
                    peak: top - low,
                    sp: cpu.reg(14),
                    ra: cpu.reg(15),
                    result: counted.bus.read32(OUTPUT).unwrap(),
                    exit,
                    fault,
                },
                counted.bus,
            );
        }
    }
    panic!("CPU step limit");
}

fn assert_return(linked: &LinkedOutput, obs: &Observation, expected: u32) {
    assert_eq!(obs.fault, None);
    assert_eq!(obs.exit, 0);
    assert_eq!(obs.result, expected);
    assert_eq!(obs.sp, linked.debug.stack_top);
}

fn debug_function<'a>(linked: &'a LinkedOutput, name: &str) -> &'a FunctionDebug {
    linked
        .debug
        .functions
        .iter()
        .find(|f| f.symbol == name)
        .unwrap()
}

fn text_bytes(linked: &LinkedOutput) -> usize {
    linked
        .image
        .sections
        .iter()
        .filter(|s| s.kind == SectionKind::Text)
        .map(|s| s.data.len())
        .sum()
}

#[test]
fn private_array_copy_removes_transfers_and_stack_bytes() {
    for (length, keep_large) in [(8, false), (256, false), (256, true)] {
        let large = if keep_large {
            "var original: Big; var copy: Big; original.words[511] = 5; copy = original; return b[0] + copy.words[511]"
        } else {
            "return b[0]"
        };
        let comparison = Comparison::compile(&format!(
            r#"package main
type Big struct {{ words: [512]u32; }}
func calc(input: u32) -> u32 {{
    var a: [{length}]u32
    var b: [{length}]u32
    a[0] = input
    b = a
    a[0] = 19
    {large}
}}
func main() {{ *({OUTPUT} as u32 as *u32) = calc(42) }}"#,
        ));
        let ([base_frame, new_frame], plan) = comparison.frames(CALC);
        assert_eq!(plan.copies.len(), 3);
        assert_eq!(plan.eliminated_locals.len(), 3);
        let scratch_before = base_frame.ra - base_frame.scratch;
        let scratch_after = new_frame.ra - new_frame.scratch;
        assert_eq!(scratch_before, if keep_large { 2048 } else { length * 4 });
        assert_eq!(scratch_after, if keep_large { 2048 } else { 4 });

        let expected = if keep_large { 47 } else { 42 };
        let measure = |code, frame: Frame| {
            let linked = comparison.link(code, STACK);
            let callee = debug_function(&linked, CALC);
            assert_eq!(callee.frame_size, frame.size);
            let (loose, _) = run(&linked, |_| {});
            assert_return(&linked, &loose, expected);

            let exact = comparison.link(code, loose.peak);
            assert_return(&exact, &run(&exact, |_| {}).0, expected);
            let tight = comparison.link(code, loose.peak - 4);
            let fault = run(&tight, |_| {}).0.fault.unwrap();
            assert_eq!(fault.code(), FaultCode::InvalidInstruction);
            assert!(tight.debug.ranges.iter().any(|range| {
                range.start <= fault.faulting_pc()
                    && fault.faulting_pc() < range.end
                    && range.operation == DebugOperation::StackOverflow
                    && range.function == CALC
            }));
            (loose, text_bytes(&linked), callee.frame_size)
        };
        let (before, before_text, before_frame) = measure(&comparison.codes[0], base_frame);
        let (after, after_text, after_frame) = measure(&comparison.codes[1], new_frame);
        assert_eq!(before.stack_reads - after.stack_reads, length * 6);
        assert_eq!(before.stack_writes - after.stack_writes, length * 6);
        assert!(after.steps < before.steps);
        assert!(after_text < before_text);
        assert!(after_frame < before_frame);
        assert!(after.peak < before.peak);
        eprintln!(
            "[{length}]u32, larger snapshot={keep_large}: reads {}->{}, writes {}->{}, steps {}->{}, Text bytes {before_text}->{after_text}, debug frame {before_frame}->{after_frame}, minimum stack {}->{}, scratch {scratch_before}->{scratch_after}",
            before.stack_reads,
            after.stack_reads,
            before.stack_writes,
            after.stack_writes,
            before.steps,
            after.steps,
            before.peak,
            after.peak
        );
    }
}

#[test]
fn private_array_copy_preserves_parallel_and_self_snapshots_and_typed_bits() {
    let comparison = Comparison::compile(&format!(
        r#"package main
func calc(input: bool) -> u32 {{
    var a: [5]bool
    var b: [5]bool
    a[0], a[4], b[4] = input, true, input
    a, b = b, a
    a = a
    if !a[0] && a[4] && b[0] && b[4] {{ return 42 }}
    return 0
}}
func narrow(input: f32) -> f32 {{
    var first: [5]f32
    first[4] = input
    var second = first
    return second[4]
}}
func wide(input: f64) -> f64 {{
    var first: [2]f64
    first[1] = input
    var second = first
    return second[1]
}}
func main() {{
    *({} as u32 as *f32) = narrow(*({} as u32 as *f32))
    *({} as u32 as *f64) = wide(*({} as u32 as *f64))
    *({OUTPUT} as u32 as *u32) = calc(*({IO} as u32 as *bool))
}}"#,
        IO + 16,
        IO + 4,
        IO + 20,
        IO + 8,
    ));
    for name in [CALC, "main.narrow", "main.wide"] {
        let ([before, after], plan) = comparison.frames(name);
        assert!(!plan.copies.is_empty());
        assert!(after.size < before.size);
    }
    let (frames, _) = comparison.frames(CALC);
    let a = comparison.local(CALC, "a");
    for (code, frame) in comparison.codes.iter().zip(frames) {
        let linked = comparison.link(code, STACK);
        for (narrow, low, high) in [
            (0x80000000, 0, 0x80000000),
            (0x7fc12345, 0x5678abcd, 0x7ff81234),
        ] {
            let (obs, mut bus) = run(&linked, |bus| {
                bus.write8(IO, 0x80).unwrap();
                bus.write32(IO + 4, narrow).unwrap();
                bus.write32(IO + 8, low).unwrap();
                bus.write32(IO + 12, high).unwrap();
            });
            assert_return(&linked, &obs, 42);
            assert_eq!(bus.read32(IO + 16).unwrap(), narrow);
            assert_eq!(bus.read32(IO + 20).unwrap(), low);
            assert_eq!(bus.read32(IO + 24).unwrap(), high);
            // calc is the last call. Check the exact private destination bytes
            // left in its frame after the swap and self-copy.
            let address = obs.sp - debug_function(&linked, "main.main").frame_size - frame.size
                + frame.local(a);
            assert_eq!(bus.read8(address).unwrap(), 0);
            assert_eq!(bus.read8(address + 4).unwrap(), 1);
        }
    }
}

#[test]
fn private_array_copy_keeps_bounds_fault_and_physical_unwind() {
    let comparison = Comparison::compile(&format!(
        r#"package main
func calc(index: u32) -> u32 {{
    var a: [2][8]u32
    var b: [8]u32
    b[0] = 19
    a[0], a[index] = b, b
    return a[0][0] + a[1][0]
}}
func main() {{ *({OUTPUT} as u32 as *u32) = calc(*({IO} as u32 as *u32)) }}"#,
    ));
    let (frames, _) = comparison.frames(CALC);
    let a = comparison.local(CALC, "a");
    let mut sources = Vec::new();
    for (code, frame) in comparison.codes.iter().zip(frames) {
        let linked = comparison.link(code, STACK);
        assert_return(&linked, &run(&linked, |b| b.write32(IO, 1).unwrap()).0, 38);
        let (obs, mut bus) = run(&linked, |b| b.write32(IO, 2).unwrap());
        let fault = obs.fault.unwrap();
        assert_eq!(fault.code(), FaultCode::InvalidInstruction);
        let pc = fault.faulting_pc();
        let bounds = linked
            .debug
            .ranges
            .iter()
            .find(|range| range.start <= pc && pc < range.end)
            .unwrap();
        assert_eq!(bounds.operation, DebugOperation::BoundsCheck);
        assert_eq!(bounds.function, CALC);
        sources.push(bounds.source.clone().unwrap());

        // The first destination was committed before the second destination faulted.
        let start = obs.sp + frame.local(a);
        assert_eq!(bus.read32(start).unwrap(), 19);
        assert_eq!(bus.read32(start + 32).unwrap(), 0);
        let callee = debug_function(&linked, CALC);
        let row = callee
            .unwind
            .iter()
            .rev()
            .find(|row| row.address <= pc)
            .unwrap();
        assert_eq!(row.cfa_sp_offset, callee.frame_size);
        assert_eq!(
            row.return_address,
            ReturnAddressLocation::StackOffset(frame.ra)
        );
        let saved_ra = bus.read32(obs.sp + frame.ra).unwrap();
        assert_eq!(saved_ra, obs.ra);
        let caller = debug_function(&linked, "main.main");
        assert!((caller.address..caller.address + caller.size).contains(&saved_ra));
        assert_eq!(
            obs.sp + row.cfa_sp_offset,
            linked.debug.stack_top - caller.frame_size
        );
    }
    assert_eq!(sources[0], sources[1]);
    assert_eq!(sources[0].path, "main.ond");
    assert_eq!(sources[0].line, 6);
}

#[test]
fn private_array_copy_retains_rhs_call_effects_and_destination_order() {
    let comparison = Comparison::compile(&format!(
        r#"package main
var Trace: u32
var First: [5]u32
var Later: u32
func index(tag: u32, i: u32) -> u32 {{ Trace = Trace * 10 + tag; return i }}
func rhs(tag: u32, n: u32) -> [5]u32 {{ Trace = Trace * 10 + tag; return [5]u32{{n}} }}
func calc(i: u32) {{
    var a: [2][5]u32
    First[index(1, 0)], a[index(2, i)], Later = 19, rhs(3, 21), index(4, 7)
    *({OUTPUT} as u32 as *u32) = a[i][0]
}}
func main() {{ calc(*({IO} as u32 as *u32)) }}"#,
    ));
    let (_, plan) = comparison.frames(CALC);
    assert!(plan.copies.is_empty());
    comparison.assert_unchanged();

    for code in &comparison.codes {
        let linked = comparison.link(code, STACK);
        let (obs, mut bus) = run(&linked, |b| b.write32(IO, 1).unwrap());
        assert_return(&linked, &obs, 21);
        assert_eq!(
            bus.read32(linked.image.symbols["main.Trace"]).unwrap(),
            1234
        );
        assert_eq!(bus.read32(linked.image.symbols["main.First"]).unwrap(), 19);
        assert_eq!(bus.read32(linked.image.symbols["main.Later"]).unwrap(), 7);

        let (obs, mut bus) = run(&linked, |b| b.write32(IO, 2).unwrap());
        assert_eq!(obs.fault.unwrap().code(), FaultCode::InvalidInstruction);
        assert_eq!(
            bus.read32(linked.image.symbols["main.Trace"]).unwrap(),
            1234
        );
        assert_eq!(bus.read32(linked.image.symbols["main.First"]).unwrap(), 19);
        assert_eq!(bus.read32(linked.image.symbols["main.Later"]).unwrap(), 0);
    }
}

#[test]
fn private_array_copy_retains_ordered_array_fault_sources() {
    let comparison = Comparison::compile(&format!(
        r#"package main
func calc(source: u32, destination: u32) {{
    var p = source as *[3]u8
    var q = destination as *[3]u8
    *q = *p
}}
func main() {{ calc(*({} as u32 as *u32), *({} as u32 as *u32)) }}"#,
        IO + 8,
        IO + 12,
    ));
    comparison.assert_unchanged();
    let linked = comparison.link(&comparison.codes[1], STACK);
    for (source, destination, operation, observed) in [
        (IO + 62, IO + 16, DebugOperation::MemoryRead, 0xaa),
        (IO, IO + 63, DebugOperation::MemoryWrite, 11),
    ] {
        let (obs, mut bus) = run(&linked, |bus| {
            bus.write8(IO, 11).unwrap();
            bus.write8(destination, 0xaa).unwrap();
            bus.write32(IO + 8, source).unwrap();
            bus.write32(IO + 12, destination).unwrap();
        });
        let fault = obs.fault.unwrap();
        assert_eq!(fault.code(), FaultCode::BusFault);
        let pc = fault.faulting_pc();
        let range = linked
            .debug
            .ranges
            .iter()
            .find(|range| range.start <= pc && pc < range.end)
            .unwrap();
        assert_eq!(range.operation, operation);
        assert_eq!(range.function, CALC);
        assert_eq!(range.source.as_ref().unwrap().line, 5);
        assert_eq!(bus.read8(destination).unwrap(), observed);
    }
}
