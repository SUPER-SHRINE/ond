use super::{
    frame::Frame,
    tests::{Fixture, machine},
    *,
};
use crate::{
    linker::{LinkPlan, LinkedOutput, link_objects_with_debug},
    program::ProgramMetadata,
};
use kagura::{Bus, BusFault, DefaultBus};
use ond_protocol::debug::{DebugOperation, FunctionDebug, ReturnAddressLocation};
use std::{collections::BTreeSet, hint::black_box, time::Instant};

const CALC: &str = "main.calc";
const EXIT: u32 = 0xffff8040;
const OUTPUT: u32 = 0xffff8050;
const IO: u32 = 0x2000_0000;
const STACK: u32 = 1 << 20;

#[derive(Clone, Copy)]
struct Case {
    ty: &'static str,
    length: u32,
    selected: bool,
    snapshot: bool,
    multiple: bool,
    caller_first: bool,
}

impl Case {
    fn source(self) -> String {
        let Self {
            ty,
            length,
            snapshot,
            multiple,
            caller_first,
            ..
        } = self;
        // Small aggregate selection keeps the unrelated large snapshots.
        let tag = if snapshot {
            "var p:[2]u32; var q:[2]u32; p[1]=7; q=p; p[1]=9; var tag=q[1]"
        } else {
            "var tag:u32=7"
        };
        let last = length.saturating_sub(1);
        let (set, result) = if length == 0 {
            (String::new(), "input".into())
        } else {
            (format!("a[{last}]=input"), format!("b[{last}]"))
        };
        let (extra, result, tag_result) = if multiple {
            (
                format!("a[{last}]=0; a,b=b,a; a=a"),
                format!("a[{last}]"),
                format!("tag + b[{last}]"),
            )
        } else {
            (String::new(), result, "tag".into())
        };
        let callee = format!(
            "func calc(input:{ty})->({ty},u32) {{
    {tag}
    var a:[{length}]{ty}
    var b:[{length}]{ty}
    {set}
    b=a
    {extra}
    return {result}, {tag_result}
}}"
        );
        let caller = format!(
            "func main() {{
    var value, tag = calc(*({IO} as u32 as *{ty}))
    *({} as u32 as *{ty}) = value
    *({OUTPUT} as u32 as *u32) = tag
}}",
            IO + 16
        );
        if caller_first {
            format!("package main\n{caller}\n{callee}")
        } else {
            format!("package main\n{callee}\n{caller}")
        }
    }

    fn input(self) -> (u8, u64) {
        match self.ty {
            "bool" => (1, 1),
            "u8" => (1, 0xa7),
            "u16" => (2, 0xdef0),
            "u32" => (4, 0xabcd_1234),
            "u64" => (8, 0xabcd_ef01_5678_1234),
            "f32" => (4, 0x7fc1_2345),
            "f64" => (8, 0x7ff8_1234_5678_abcd),
            ty => panic!("unexpected type {ty}"),
        }
    }
}

fn generate(
    mir: &mir::Project,
    loaded: &crate::project::LoadedProject,
    enabled: bool,
) -> CodegenOutput {
    let paths = loaded.packages[0]
        .files
        .iter()
        .map(|id| (*id, "main.ond".into()))
        .collect();
    let lower = if enabled {
        raw_induction::lower
    } else {
        raw_induction::lower_without_private_copy_loops
    };
    let raw = codegen_selected_using(
        mir,
        &mir.packages
            .iter()
            .map(|p| p.logical_path.clone())
            .collect(),
        Some((&loaded.sources, &paths)),
        lower,
    )
    .unwrap();
    runtime::supply_with_debug(raw.objects, raw.debug).unwrap()
}

fn link(code: &CodegenOutput, info: &ProgramMetadata, stack_size: u32) -> LinkedOutput {
    let mut objects = vec![crate::test_support::startup_object(
        &info.initializers,
        &info.entry_symbol,
    )];
    objects.extend_from_slice(&code.objects);
    link_objects_with_debug(
        &objects,
        &code.debug,
        &LinkPlan {
            read_only: None,
            memory_base: 0x1000,
            memory_size: 8 * STACK,
            stack_size,
            entry_symbol: "__ond.entry".into(),
            heap_base_symbol: "__ond_heap_base".into(),
            stack_bottom_symbol: "__ond_stack_bottom".into(),
        },
    )
    .unwrap()
}

#[derive(Debug, PartialEq, Eq)]
struct Access {
    address: u32,
    width: u8,
    write: bool,
}

struct TracingBus {
    bus: DefaultBus,
    text: Vec<std::ops::Range<u32>>,
    accesses: Vec<(u32, Access)>,
    pc: u32,
    fetch: bool,
    branches: u32,
    literals: u32,
}

impl TracingBus {
    fn record(&mut self, address: u32, width: u8, write: bool) {
        if !write && self.text.iter().any(|section| section.contains(&address)) {
            self.literals += 1;
        } else {
            self.accesses.push((
                self.pc,
                Access {
                    address,
                    width,
                    write,
                },
            ));
        }
    }
}

macro_rules! trace_reads_and_writes {
    ($($read:ident, $write:ident, $ty:ty, $width:literal);+) => {
        impl Bus for TracingBus {
            $(
                fn $read(&mut self, address: u32) -> Result<$ty, BusFault> {
                    if $width == 4 && std::mem::take(&mut self.fetch) {
                        let word = self.bus.$read(address)?;
                        self.branches += u32::from(u32::from(word) >> 28 == 12);
                        return Ok(word);
                    }
                    self.record(address, $width, false);
                    self.bus.$read(address)
                }

                fn $write(&mut self, address: u32, value: $ty) -> Result<(), BusFault> {
                    self.record(address, $width, true);
                    self.bus.$write(address, value)
                }
            )+
        }
    };
}
trace_reads_and_writes!(read8, write8, u8, 1; read16, write16, u16, 2; read32, write32, u32, 4);

struct Run {
    trace: TracingBus,
    steps: u32,
    peak: u32,
}

fn run(linked: &LinkedOutput, case: Case) -> Run {
    let (mut cpu, mut bus, top) = machine(&linked.image);
    bus.map_device(IO, 64, ram::Ram::new(64)).unwrap();
    let (width, bits) = case.input();
    let input = &bits.to_le_bytes()[..usize::from(width)];
    for (offset, &byte) in input.iter().enumerate() {
        bus.write8(IO + offset as u32, byte).unwrap();
    }
    let mut trace = TracingBus {
        bus,
        text: linked
            .image
            .sections
            .iter()
            .filter(|s| s.kind == SectionKind::Text)
            .map(|s| s.load_address..s.load_address + s.memory_size)
            .collect(),
        accesses: Vec::new(),
        pc: 0,
        fetch: false,
        branches: 0,
        literals: 0,
    };
    let mut bottom = top;
    for steps in 1..=3_000_000 {
        trace.pc = cpu.pc();
        trace.fetch = true;
        cpu.step(&mut trace).unwrap();
        bottom = bottom.min(cpu.reg(14));
        if trace.bus.read32(EXIT).unwrap() == 0 {
            assert_eq!(trace.bus.read32(OUTPUT).unwrap(), 7);
            for (offset, &byte) in input.iter().enumerate() {
                assert_eq!(trace.bus.read8(IO + 16 + offset as u32).unwrap(), byte);
            }
            assert_eq!(cpu.reg(14), top);
            for register in 7..=11 {
                assert_eq!(cpu.reg(register), 0xabcd_0000 + register as u32);
            }
            return Run {
                trace,
                steps,
                peak: top - bottom,
            };
        }
    }
    panic!("guest step limit at {:#x}", cpu.pc());
}

fn calc(linked: &LinkedOutput) -> &FunctionDebug {
    linked
        .debug
        .functions
        .iter()
        .find(|f| f.symbol == CALC)
        .unwrap()
}

fn words(linked: &LinkedOutput, start: u32, end: u32) -> impl Iterator<Item = u32> + '_ {
    let text = linked
        .image
        .sections
        .iter()
        .find(|s| {
            s.kind == SectionKind::Text
                && s.load_address <= start
                && end <= s.load_address + s.memory_size
        })
        .unwrap();
    text.data[(start - text.load_address) as usize..(end - text.load_address) as usize]
        .chunks_exact(4)
        .map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()))
}

fn instruction(linked: &LinkedOutput, pc: u32) -> u32 {
    words(linked, pc, pc + 4).next().unwrap()
}

fn access_range(linked: &LinkedOutput, pc: u32) -> &ond_protocol::debug::DebugRange {
    linked
        .debug
        .ranges
        .iter()
        .filter(|r| r.start <= pc && pc < r.end)
        .min_by_key(|r| r.end - r.start)
        .unwrap()
}

/// Compare ordered data accesses and check the final linked source/operation
/// and unwind at each distinct generated indexed load/store. All execution is normal.
fn check_accesses(linked: &[LinkedOutput; 2], runs: &[Run; 2], case: Case) -> usize {
    let [before, after] = linked;
    let func = calc(after);
    let mut seen = BTreeSet::new();
    let [old, new] = runs.each_ref().map(|r| &r.trace.accesses);
    assert_eq!(old.len(), new.len());
    for ((old_pc, old), (new_pc, new)) in old.iter().zip(new) {
        assert_eq!(old, new);
        if !(func.address..func.address + func.size).contains(new_pc) || seen.contains(new_pc) {
            continue;
        }
        let word = instruction(after, *new_pc);
        if (word >> 16) & 0xf != 3 {
            continue;
        }
        seen.insert(*new_pc);
        let expected = match word >> 28 {
            4 | 5 | 7 => DebugOperation::MemoryRead,
            8 | 9 | 11 => DebugOperation::MemoryWrite,
            _ => panic!("unexpected access opcode: {word:#x}"),
        };
        let current = access_range(after, *new_pc);
        let previous = access_range(before, *old_pc);
        assert_eq!(current.operation, expected);
        assert_eq!(current.function, CALC);
        assert_eq!(current.source, previous.source);
        assert_eq!(current.operation, previous.operation);
        let source = current.source.as_ref().unwrap();
        assert_eq!(source.path, "main.ond");
        let first = if case.caller_first { 9 } else { 4 };
        assert!((first..=first + 4).contains(&source.line));
        for (image, pc) in [(before, *old_pc), (after, *new_pc)] {
            let f = calc(image);
            let row = f.unwind.iter().rfind(|row| row.address <= pc).unwrap();
            assert_eq!(row.cfa_sp_offset, f.frame_size);
            assert_eq!(
                row.return_address,
                ReturnAddressLocation::StackOffset(f.return_address_offset)
            );
        }
        if case.ty == "bool" && expected == DebugOperation::MemoryRead {
            assert_eq!(
                instruction(after, new_pc + 4),
                crate::kagura_encoding::word(13, 1, 1, 0, 4)
            );
        }
    }

    let mut memory_pcs = BTreeSet::new();
    let mut branches = 0;
    for (index, word) in words(after, func.address, func.address + func.size).enumerate() {
        let pc = func.address + index as u32 * 4;
        if word & 0xffff_8000 == 0xc010_8000 {
            let target = (i64::from(pc) + 4 + i64::from(word as i16) * 4) as u32;
            assert!((func.address..pc).contains(&target));
            assert!(matches!(instruction(after, target) >> 28, 4 | 5 | 7));
            assert_eq!(
                access_range(after, target).source,
                access_range(after, pc).source
            );
            branches += 1;
        }
        if matches!(word >> 28, 4 | 5 | 7 | 8 | 9 | 11)
            && (word >> 16) & 0xf == 3
            && matches!((word >> 20) & 0xf, 4 | 5)
        {
            memory_pcs.insert(pc);
        }
    }
    assert_eq!(seen, memory_pcs);
    // The two initializations and b=a each use one direct transfer, or
    // one staging store and both snapshot legs. b=a also loads its RHS.
    // The swap and self-assignment add three assignments with RHS loads.
    let expected = if case.selected {
        let legs = if case.snapshot { 3 } else { 1 };
        let extra = if case.multiple { 3 } else { 0 };
        (3 + extra) * legs + 1 + extra
    } else {
        0
    };
    assert_eq!(branches, expected, "{} {}", case.ty, case.length);
    branches
}

fn far_call_into_calc(linked: &LinkedOutput, code: &CodegenOutput) -> bool {
    let (main, rel) = code
        .objects
        .iter()
        .find_map(|obj| {
            let main = obj
                .symbols
                .iter()
                .find(|symbol| symbol.name == "main.main")?;
            let call = obj
                .relocations
                .iter()
                .find(|rel| rel.kind == RelocationKind::CallSlot && rel.target == CALC)?;
            Some((main, call))
        })
        .unwrap();
    let site = linked.image.symbols["main.main"] + rel.offset - main.offset;
    // The third slot word is padding for a direct call or capture-PC for a thunk.
    let far = instruction(linked, site + 8) != 0;
    let delta = (i64::from(calc(linked).address) - i64::from(site) - 4) / 4;
    assert_eq!(far, i16::try_from(delta).is_err());
    far
}

fn assert_same_guard(linked: &[LinkedOutput; 2]) {
    let guards = linked.each_ref().map(|l| {
        let range = l
            .debug
            .ranges
            .iter()
            .find(|r| r.function == CALC && r.operation == DebugOperation::StackOverflow)
            .unwrap();
        words(l, range.start, range.end).collect::<Vec<_>>()
    });
    assert_eq!(guards[0], guards[1]);
}

fn measure(fixture: &Fixture, frontend: &ond_compiler_core::Compilation) -> [[u128; 2]; 2] {
    let mut samples = [const { [Vec::new(), Vec::new()] }; 2];
    for enabled in [false, true] {
        black_box(generate(&frontend.mir, &frontend.loaded, enabled));
    }
    for trial in 0..9 {
        for enabled in [trial % 2 != 0, trial % 2 == 0] {
            let mode = usize::from(enabled);
            let start = Instant::now();
            let fresh = ond_compiler_core::compile_project(&fixture.0).unwrap();
            black_box(generate(&fresh.mir, &fresh.loaded, enabled));
            samples[mode][0].push(start.elapsed().as_nanos());
            let start = Instant::now();
            black_box(generate(&frontend.mir, &frontend.loaded, enabled));
            samples[mode][1].push(start.elapsed().as_nanos());
        }
    }
    samples.map(|mode| {
        mode.map(|mut times| {
            times.sort_unstable();
            times[times.len() / 2]
        })
    })
}

fn compare(case: Case, benchmark: bool) {
    let fixture = Fixture::new(&case.source());
    let compiled = crate::compile_project(&fixture.0).unwrap();
    let before = generate(&compiled.mir, &compiled.loaded, false);
    let after = CodegenOutput {
        objects: compiled.objects,
        debug: compiled.debug_objects,
    };
    let info = ProgramMetadata::from_mir(&compiled.mir).unwrap();
    let codes = [before, after];
    let linked = codes.each_ref().map(|code| link(code, &info, STACK));
    let runs = linked.each_ref().map(|l| run(l, case));
    let loops = check_accesses(&linked, &runs, case);
    let far = [
        far_call_into_calc(&linked[0], &codes[0]),
        far_call_into_calc(&linked[1], &codes[1]),
    ];
    if case.caller_first {
        assert_eq!(far, [false; 2]);
    }
    let functions = linked.each_ref().map(calc);
    let text = linked.each_ref().map(|l| {
        l.image
            .sections
            .iter()
            .filter(|s| s.kind == SectionKind::Text)
            .map(|s| s.data.len())
            .sum::<usize>()
    });
    if case.selected {
        assert!(text[1] < text[0]);
        assert!(functions[1].size < functions[0].size);
        assert!(
            runs[1].steps <= runs[0].steps,
            "{} {}",
            case.ty,
            case.length
        );
    } else {
        assert_eq!(text[0], text[1]);
        assert_eq!(codes[0].debug, codes[1].debug);
    }
    let peaks = runs.each_ref().map(|r| r.peak);
    assert_eq!(peaks[0], peaks[1]);
    assert_eq!(functions[0].frame_size, functions[1].frame_size);
    assert_eq!(
        functions[0].return_address_offset,
        functions[1].return_address_offset
    );
    let exact = codes.each_ref().map(|c| link(c, &info, peaks[0]));
    assert_same_guard(&exact);
    for l in &exact {
        assert_eq!(run(l, case).peak, peaks[0]);
    }

    // At the same call depth 4 B less places the unchanged allocation
    // below the guard's linked stack bottom. Check this without fault execution.
    let short = codes.each_ref().map(|c| link(c, &info, peaks[0] - 4));
    assert_same_guard(&short);
    for (fits, short) in exact.iter().zip(&short) {
        let caller = fits
            .debug
            .functions
            .iter()
            .find(|f| f.symbol == "main.main")
            .unwrap();
        let new_sp = fits.debug.stack_top - caller.frame_size - calc(fits).frame_size;
        assert_eq!(new_sp, fits.debug.stack_bottom);
        assert_eq!(new_sp + 4, short.debug.stack_bottom);
    }

    let f = compiled.mir.packages[0]
        .functions
        .iter()
        .find(|f| f.name == CALC)
        .unwrap();
    let types = &compiled.mir.types;
    let abi = layout::abi(
        types,
        &mir::FunctionType {
            parameters: f
                .parameters
                .iter()
                .map(|id| f.locals[id.0 as usize].ty)
                .collect(),
            returns: f.returns.clone(),
        },
        f.span,
    )
    .unwrap();
    let small = local_aggregate::LocalAggregates::new(types, f, &abi).is_some();
    let plan = if small {
        private_array_copy::Plan::default()
    } else {
        private_array_copy::Plan::new(types, f, &abi)
    };
    let frame = Frame::new(f, types, &abi, &plan).unwrap();
    if case.selected {
        assert_eq!(case.snapshot, small);
        assert_eq!(case.snapshot, plan.copies.is_empty());
        assert_eq!(functions[1].frame_size, frame.size);
        assert_eq!(
            frame.ra - frame.scratch,
            if case.snapshot {
                case.length * u32::from(case.input().0)
            } else {
                4
            }
        );
    }

    if benchmark {
        let frontend = ond_compiler_core::compile_project(&fixture.0).unwrap();
        let times = measure(&fixture, &frontend);
        for (mode, (l, r)) in linked.iter().zip(&runs).enumerate() {
            let reads = r.trace.accesses.iter().filter(|(_, a)| !a.write).count();
            let writes = r.trace.accesses.len() - reads;
            let scratch = frame.ra - frame.scratch;
            eprintln!(
                "copy,{},{},{},{},{},{mode},{},{},{},{},{},{},{reads},{writes},{},{},{scratch},{},{},{},{},{}",
                case.ty,
                case.length,
                case.snapshot,
                case.multiple,
                case.caller_first,
                text[mode],
                functions[mode].address,
                functions[mode].size,
                r.steps,
                r.trace.branches,
                far[mode],
                r.trace.literals,
                functions[mode].frame_size,
                r.peak,
                l.debug.ranges.len(),
                times[mode][0],
                times[mode][1],
                if mode == 0 { 0 } else { loops }
            );
        }
    }
}

fn case(ty: &'static str, length: u32, selected: bool) -> Case {
    Case {
        ty,
        length,
        selected,
        snapshot: false,
        multiple: false,
        caller_first: false,
    }
}

#[test]
fn private_copy_loop_matches_unrolled_transfers() {
    for case in [
        case("u32", 0, false),
        case("u32", 1, false),
        case("u32", 4, false),
        case("u32", 256, false),
        case("u32", 9039, false),
        case("u32", 9040, true),
        Case {
            caller_first: true,
            ..case("u32", 9040, true)
        },
        case("u32", 9041, true),
        case("u16", 18081, true),
        case("bool", 36159, true),
        case("f32", 9042, true),
        case("f64", 4523, true),
        Case {
            snapshot: true,
            multiple: true,
            ..case("u32", 9043, true)
        },
    ] {
        compare(case, false);
    }
}

#[test]
#[ignore = "timing measurement; run in release on an idle host with one test thread"]
fn measure_private_copy_loop_matrix() {
    eprintln!(
        "copy,type,length,snapshot,multiple,caller_first,mode,text,calc_address,calc_text,steps,branches,far_call,reads,writes,text_literal_reads,frame,scratch,peak,ranges,pipeline_ns,backend_ns,loop_sites"
    );
    for case in [
        case("u32", 0, false),
        case("u32", 1, false),
        case("u32", 4, false),
        case("u32", 256, false),
        case("u32", 8193, false),
        case("u32", 9039, false),
        case("u32", 9040, true),
        Case {
            caller_first: true,
            ..case("u32", 9040, true)
        },
        case("u32", 9041, true),
        Case {
            snapshot: true,
            ..case("u32", 9040, true)
        },
        case("u32", 16384, true),
        case("u16", 18080, true),
        case("u8", 36160, true),
        case("u64", 4520, false),
        case("u64", 4521, true),
        case("bool", 36159, true),
        case("f32", 9042, true),
        case("f64", 4523, true),
        Case {
            snapshot: true,
            multiple: true,
            ..case("u32", 9043, true)
        },
    ] {
        compare(case, true);
    }
}

#[test]
fn private_copy_loop_keeps_raw_ordered_and_abi_transfers_unrolled() {
    let fixture = Fixture::new(
        "package main
var Shared:[9040]u32
func raw(dst:*[9040]u32, src:*[9040]u32) { *dst = *src }
func ordered() {
    var local:[9040]u32
    local=Shared
    Shared=local
}
func record(input:[9040]u32)->[9040]u32 {
    var clone=input
    return clone
}
func scalar(input:u32)->u32 { return input }
func call(input:u32)->u32 {
    var a:[9040]u32
    a[0]=input
    var b=a
    return scalar(b[0])
}
func main() {}",
    );
    let compiled = crate::compile_project(&fixture.0).unwrap();
    let baseline = generate(&compiled.mir, &compiled.loaded, false);
    assert_eq!(baseline.debug, compiled.debug_objects);
    // Compare the full artifacts without executing any of these transfers.
    assert!(format!("{:?}", baseline.objects) == format!("{:?}", compiled.objects));
}
