use super::*;
use crate::{
    linker::{LinkPlan, LinkedOutput, link_objects_with_debug},
    mir_backend::{CodegenOutput, codegen_selected_using, codegen_selected_with_debug, tests::*},
};
use kagura::{Bus, BusFault, DefaultBus, FaultCode};
use ond_protocol::debug::{DebugOperation, SourceLocation};
use std::collections::BTreeMap;

const BYTES: u32 = 2048 * 12;
const EXIT: u32 = 0xffff8040;
const EXIT_SENTINEL: u32 = 0xdead_beef;
const OUTPUT: u32 = 0xffff8050;
const VISIT: &str = "main.visit";
const THREE: &str = "var old = p[i].value; p[i].value = old + 1; sum = sum + p[i].value + i";
const FOUR: &str =
    "var old = p[i].value; p[i].value = old + 1; sum = sum + p[i].value + i; p[i].value = sum";
const SPLIT: &str = "if i < selector { sum = sum + p[i].value + i; p[i].value = sum } else { sum = p[i].value + i + sum; p[i].value = sum }";

fn source(base: &str, header: &str, body: &str, selector: u32) -> String {
    format!(
        r#"package main
type Item struct {{ first: u16; value: u32; last: u32; }}
var Data: [2048]Item
func before() -> u32 {{ return 9 }}
func visit(selector: u32) -> u32 {{
    var p = {base}
    var sum: u32
    for {header} {{
        {body}
    }}
    return sum
}}
func after() -> u32 {{ return 3 }}
func main() {{ *({OUTPUT} as u32 as *u32) = before() + visit({selector}) + after() }}
"#
    )
}

struct Outputs {
    before: CodegenOutput,
    after: CodegenOutput,
    proven: bool,
}

impl Outputs {
    fn assert_unchanged(&self, name: &str) {
        // Object's derived Debug includes all section bytes, symbol and
        // relocation fields. Compare the whole output including sidecars.
        assert!(
            format!("{:?}", self.before.objects) == format!("{:?}", self.after.objects),
            "{name}: object changed",
        );
        assert_eq!(self.before.debug, self.after.debug, "{name}");
    }
}

fn compile(name: &str, source: &str) -> Outputs {
    let fixture = Fixture::new(source);
    let loaded = ond_compiler_core::load_project(&fixture.0).unwrap();
    let compiled = ond_compiler_core::compile_loaded_project_with_generation(loaded)
        .unwrap_or_else(|err| panic!("{name}: {err:?}"))
        .0;
    let p = &compiled.mir;
    let package = &p.packages[0];
    let f = package.functions.iter().find(|f| f.name == VISIT).unwrap();
    let proven = Plan::new(&p.types, &package.globals, f).is_some();
    let selected = p.packages.iter().map(|p| p.logical_path.clone()).collect();
    let paths = package.id;
    let paths = compiled
        .loaded
        .packages
        .iter()
        .filter(|p| p.id == paths)
        .flat_map(|p| p.files.iter().map(|id| (*id, "main.ond".to_owned())))
        .collect::<BTreeMap<_, _>>();
    let debug = Some((&compiled.loaded.sources, &paths));
    let before = codegen_selected_using(p, &selected, debug, |p, _, f, emitter| {
        function::lower(p, f, emitter)
    })
    .unwrap();
    let after = codegen_selected_with_debug(p, &selected, debug).unwrap();
    Outputs {
        before,
        after,
        proven,
    }
}

fn link(code: &CodegenOutput, stack_size: u32) -> LinkedOutput {
    let mut objects = vec![crate::test_support::startup_object(&[], "main.main")];
    objects.extend_from_slice(&code.objects);
    link_objects_with_debug(
        &objects,
        &code.debug,
        &LinkPlan {
            read_only: None,
            memory_base: 0x1000,
            memory_size: 65536,
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
    offset: u32,
    width: u32,
    written: Option<u32>,
}

struct TraceBus {
    bus: DefaultBus,
    start: u32,
    bytes: u32,
    accesses: Vec<Access>,
    fail_at: Option<usize>,
}
impl TraceBus {
    fn access(&mut self, address: u32, width: u32, written: Option<u32>) -> Result<(), BusFault> {
        if let Some(offset) = address.checked_sub(self.start).filter(|n| *n < self.bytes) {
            self.accesses.push(Access {
                offset,
                width,
                written,
            });
            if self.fail_at == Some(self.accesses.len() - 1) {
                return Err(BusFault);
            }
        }
        Ok(())
    }
}
macro_rules! read {
    ($method:ident, $ty:ty, $width:expr) => {
        fn $method(&mut self, address: u32) -> Result<$ty, BusFault> {
            self.access(address, $width, None)?;
            self.bus.$method(address)
        }
    };
}
macro_rules! write {
    ($method:ident, $ty:ty, $width:expr) => {
        fn $method(&mut self, address: u32, value: $ty) -> Result<(), BusFault> {
            self.access(address, $width, Some(u32::from(value)))?;
            self.bus.$method(address, value)
        }
    };
}
impl Bus for TraceBus {
    read!(read8, u8, 1);
    read!(read16, u16, 2);
    read!(read32, u32, 4);
    write!(write8, u8, 1);
    write!(write16, u16, 2);
    write!(write32, u32, 4);
}

#[derive(Debug, PartialEq, Eq)]
struct Fault {
    code: FaultCode,
    operation: DebugOperation,
    source: Option<SourceLocation>,
}

struct Run {
    output: u32,
    exit: u32,
    steps: usize,
    peak: u32,
    trace: Vec<Access>,
    data: Vec<u8>,
    fault: Option<Fault>,
}

fn run(linked: &LinkedOutput, fail_at: Option<usize>) -> Run {
    run_with_data_size(linked, fail_at, BYTES)
}

fn run_with_data_size(linked: &LinkedOutput, fail_at: Option<usize>, bytes: u32) -> Run {
    let (mut cpu, mut bus, top) = machine(&linked.image);
    bus.map_device(OUTPUT + 4, 12, ram::Ram::new(12)).unwrap();
    let start = linked.image.symbols["main.Data"];
    let mut bus = TraceBus {
        bus,
        start,
        bytes,
        accesses: Vec::new(),
        fail_at,
    };
    let mut low = top;
    for steps in 1..=200_000 {
        let outcome = cpu.step(&mut bus);
        low = low.min(cpu.reg(14));
        let exit = bus.bus.read32(EXIT).unwrap();
        let fault = match outcome {
            Ok(()) if exit != EXIT_SENTINEL => {
                assert_eq!(exit, 0);
                assert_eq!(cpu.reg(14), top);
                for r in 7..=11 {
                    assert_eq!(cpu.reg(r), 0xabcd0000 + r as u32);
                }
                None
            }
            Ok(()) => continue,
            Err(err) => {
                let range = linked
                    .debug
                    .ranges
                    .iter()
                    .filter(|range| {
                        range.start <= err.faulting_pc() && err.faulting_pc() < range.end
                    })
                    .min_by_key(|range| range.end - range.start)
                    .unwrap();
                Some(Fault {
                    code: err.code(),
                    operation: range.operation,
                    source: range.source.clone(),
                })
            }
        };
        return Run {
            output: bus.bus.read32(OUTPUT).unwrap(),
            exit,
            steps,
            peak: top - low,
            trace: bus.accesses,
            data: (start..start + bytes)
                .map(|address| bus.bus.read8(address).unwrap())
                .collect(),
            fault,
        };
    }
    panic!("Kagura step limit");
}

fn frame(code: &CodegenOutput) -> u32 {
    code.debug[0]
        .functions
        .iter()
        .find(|f| f.symbol == VISIT)
        .unwrap()
        .frame_size
}

fn text(linked: &LinkedOutput) -> usize {
    linked
        .image
        .sections
        .iter()
        .find(|s| s.kind == super::super::SectionKind::Text)
        .unwrap()
        .data
        .len()
}

#[test]
fn raw_induction_normal_codegen_is_faster_for_proven_paths() {
    let mixed = "var narrow = p[i].first; p[i].first = narrow; sum = sum + p[i].value + i";
    let continue_loop = format!("{THREE}; if i == selector {{ continue }}; p[i].value = sum");
    for (name, base, header, body, selector, expected, count) in [
        (
            "three",
            "&Data[0]",
            "i := 0 as u32; i < 128; i += 1",
            THREE,
            0,
            128 + 8128,
            384,
        ),
        (
            "four",
            "&Data[0]",
            "i := 0 as u32; i < 128; i += 1",
            FOUR,
            0,
            128 + 8128,
            512,
        ),
        (
            "split",
            "&Data[0]",
            "i := 0 as u32; i < 8; i += 1",
            SPLIT,
            2,
            28,
            16,
        ),
        (
            "continue",
            "&Data[0]",
            "i := 0 as u32; i < 8; i += 1",
            continue_loop.as_str(),
            2,
            36,
            31,
        ),
        (
            "mixed-width",
            "&Data[0]",
            "i := 0 as u32; i < 8; i += 1",
            mixed,
            0,
            28,
            24,
        ),
        (
            "wrap",
            "&Data[2]",
            "i := 4294967294 as u32; i != 2; i += 1",
            FOUR,
            0,
            2,
            16,
        ),
        (
            "long",
            "&Data[0]",
            "i := 0 as u32; i < 1025; i += 1",
            THREE,
            0,
            1025 + 1025 * 1024 / 2,
            3075,
        ),
    ] {
        let o = compile(name, &source(base, header, body, selector));
        assert!(o.proven, "{name}");
        let l = link(&o.before, 8192);
        let r = link(&o.after, 8192);
        let before = run(&l, None);
        let after = run(&r, None);
        assert!(before.fault.is_none() && after.fault.is_none(), "{name}");
        assert_eq!(
            (before.output, after.output),
            (expected + 12, expected + 12),
            "{name}"
        );
        assert_eq!(before.trace.len(), count, "{name}");
        assert_eq!(before.trace, after.trace, "{name}");
        if name == "mixed-width" {
            assert_eq!(after.trace.iter().filter(|a| a.width == 2).count(), 16);
            assert_eq!(after.trace.iter().filter(|a| a.width == 4).count(), 8);
        }
        assert_eq!(before.data, after.data, "{name}");
        assert!(
            after.steps < before.steps,
            "{name}: {} vs {}",
            before.steps,
            after.steps
        );
        assert_eq!(frame(&o.after), frame(&o.before), "{name}");
        assert_eq!(after.peak, before.peak, "{name}");
        let peak = before.peak;
        for code in [&o.before, &o.after] {
            let exact = run(&link(code, peak), None);
            assert!(exact.fault.is_none(), "{name}");
            assert_eq!((exact.output, exact.exit), (expected + 12, 0), "{name}");
        }
        for capacity in [peak - 4, peak - 8] {
            let short_before = run(&link(&o.before, capacity), None);
            let short_after = run(&link(&o.after, capacity), None);
            assert_eq!(
                short_before.fault, short_after.fault,
                "{name} at {capacity} B"
            );
            let fault = short_after.fault.as_ref().unwrap();
            assert_eq!(
                fault.code,
                FaultCode::InvalidInstruction,
                "{name} at {capacity} B"
            );
            assert_eq!(
                fault.operation,
                DebugOperation::StackOverflow,
                "{name} at {capacity} B"
            );
            assert!(fault.source.is_some(), "{name} at {capacity} B");
            for short in [&short_before, &short_after] {
                assert_eq!(
                    (short.output, short.exit),
                    (0, EXIT_SENTINEL),
                    "{name} at {capacity} B"
                );
                assert!(short.trace.is_empty(), "{name} at {capacity} B");
            }
        }
        eprintln!(
            "{name}: steps {} -> {}, text {} -> {}, frame {} -> {}, peak/min stack {} -> {}",
            before.steps,
            after.steps,
            text(&l),
            text(&r),
            frame(&o.before),
            frame(&o.after),
            before.peak,
            after.peak
        );
    }
}

#[test]
fn raw_induction_fallback_keeps_package_artifacts() {
    let one_per_arm = "if i == selector { sum = sum + p[i].value } else { p[i].value = sum }";
    for (name, base, header, body, proven) in [
        (
            "zero",
            "&Data[0]",
            "i := 0 as u32; i < 0; i += 1",
            FOUR,
            false,
        ),
        (
            "continue-only-path",
            "&Data[0]",
            "i := 0 as u32; i < 8; i += 1",
            "if i < selector { continue }; sum = sum + p[i].value; p[i].value = sum",
            false,
        ),
        (
            "unknown-origin",
            "&Data[selector]",
            "i := 0 as u32; i < 8; i += 1",
            FOUR,
            false,
        ),
        (
            "unknown-cap",
            "&Data[0]",
            "i := 0 as u32; i < selector; i += 1",
            FOUR,
            false,
        ),
        (
            "changed-base",
            "&Data[0]",
            "i := 0 as u32; i < 8; i += 1",
            "p = &Data[1]; sum = sum + p[i].value; p[i].value = sum",
            false,
        ),
        (
            "changed-index",
            "&Data[0]",
            "i := 0 as u32; i < 8; i += 1",
            "if i == selector { i += 1; continue }; sum = sum + p[i].value; p[i].value = sum",
            false,
        ),
        (
            "early-break",
            "&Data[0]",
            "i := 0 as u32; i < 8; i += 1",
            "if i < selector { break }; sum = sum + p[i].value; p[i].value = sum",
            false,
        ),
        (
            "early-return",
            "&Data[0]",
            "i := 0 as u32; i < 8; i += 1",
            "if i < selector { return sum }; sum = sum + p[i].value; p[i].value = sum",
            false,
        ),
        (
            "emission-tie",
            "&Data[0]",
            "i := 1 as u32; i < 2; i += 305419896",
            one_per_arm,
            true,
        ),
    ] {
        let o = compile(name, &source(base, header, body, 2));
        assert_eq!(o.proven, proven, "{name}");
        o.assert_unchanged(name);
    }
}

#[test]
fn raw_induction_fault_preserves_access_prefix_and_span() {
    let o = compile(
        "fault",
        &source("&Data[0]", "i := 0 as u32; i < 4; i += 1", FOUR, 0),
    );
    let left = link(&o.before, 8192);
    let right = link(&o.after, 8192);
    let normal_before = run(&left, None);
    let normal_after = run(&right, None);
    assert!(normal_before.fault.is_none() && normal_after.fault.is_none());
    assert!(normal_after.steps < normal_before.steps);
    for (fail_at, expected) in [
        (2, DebugOperation::MemoryRead),
        (3, DebugOperation::MemoryWrite),
    ] {
        let before = run(&left, Some(fail_at));
        let after = run(&right, Some(fail_at));
        assert_eq!((before.output, before.exit), (0, EXIT_SENTINEL));
        assert_eq!((after.output, after.exit), (0, EXIT_SENTINEL));
        assert_eq!(before.trace, after.trace);
        assert_eq!(before.trace.len(), fail_at + 1);
        assert_eq!(before.data, after.data);
        assert_eq!(before.data[4], 1);
        assert_eq!(before.fault, after.fault);
        let fault = after.fault.unwrap();
        assert_eq!(fault.code, FaultCode::BusFault);
        assert_eq!(fault.operation, expected);
        assert_eq!(fault.source.unwrap().line, 9);
    }
    let o = compile(
        "bounds",
        &source(
            "&Data[selector]",
            "i := 0 as u32; i < 4; i += 1",
            FOUR,
            2048,
        ),
    );
    o.assert_unchanged("bounds");
    let run = run(&link(&o.after, 8192), None);
    assert!(run.trace.is_empty());
    let fault = run.fault.unwrap();
    assert_eq!(fault.code, FaultCode::InvalidInstruction);
    assert_eq!(fault.operation, DebugOperation::BoundsCheck);
    assert_eq!(fault.source.unwrap().line, 6);
}

#[test]
fn raw_induction_range_and_trip_proofs_cover_modular_limits() {
    use BinaryOp::{LessUnsigned as Lt, NotEqual as Ne};
    assert_eq!(trip_count(Lt, 1, 2049, 1), Some(2048));
    assert_eq!(trip_count(Lt, 2, 9, 3), Some(3));
    assert_eq!(trip_count(Lt, u32::MAX - 1, u32::MAX, 2), None);
    assert_eq!(trip_count(Ne, u32::MAX - 1, 2, 1), Some(4));
    assert_eq!(trip_count(Ne, u32::MAX - 1, 2, 2), Some(2));
    assert_eq!(trip_count(Ne, u32::MAX - 1, 1, 2), None);
    assert_eq!(trip_count(Ne, 0, u32::MAX, u32::MAX), Some(1));
    assert_eq!(trip_count(Ne, 0, 9, 3), Some(3));
    assert_eq!(trip_count(Ne, 0, 2, u32::MAX), Some(u32::MAX - 1));
    assert!(prove_offsets(4, 2, 12, u32::MAX - 1, 1, 4).is_some());
    assert!(prove_offsets(6, 0, 12, 5, u32::MAX, 6).is_some()); // negative byte stride
    assert!(prove_offsets(3, 2, 12, u32::MAX - 1, 1, 4).is_none());
    assert!(prove_offsets(4, 0, 12, 0, 1 << 30, u32::MAX).is_some()); // modulo stride zero
    assert!(prove_offsets(4, 0, 0, 0, 1, 4).is_none());
}

#[test]
fn raw_induction_emitted_cost_skips_padding_and_declines_unmodeled_branches() {
    let word = crate::kagura_encoding::word;
    let span = Span::synthetic();
    let mut e = Emitter::default();
    e.symbol(4, "data", span).unwrap();
    e.trap_if_nonzero(2);
    let label = e.label();
    e.branch(0, label, span);
    let length = e.words.len();
    e.bind(label);
    let e = e.resolve().unwrap();
    assert_eq!(
        cost::steps(&e.words, 0..length, length, Some(length), None),
        Some(5)
    );
    let mut far = Emitter::default();
    let label = far.label();
    far.branch(0, label, span);
    far.words.resize(32770, word(0, 0, 0, 0, 0));
    far.bind(label);
    let length = far.words.len();
    let far = far.resolve().unwrap();
    assert_eq!(
        cost::steps(&far.words, 0..length, 0, Some(length), None),
        None
    );
}

#[test]
fn raw_induction_documented_global_array_case() {
    let doc = include_str!(
        "../../../../../specs/ond/conformance/cases/kagura-regression-observations.md"
    )
    .replace("\r\n", "\n");
    let (_, section) = doc
        .split_once("<!-- ond-spec-id: conformance/kagura-regression/raw-pointer-loop -->")
        .unwrap();
    let (_, code) = section.split_once("```ond\n").unwrap();
    let (source, _) = code.split_once("\n```").unwrap();
    let output = compile("documented", source);
    assert!(output.proven);
    let linked = link(&output.after, 8192);
    let baseline = run_with_data_size(&link(&output.before, 8192), None, 16);
    let normal = run_with_data_size(&linked, None, 16);
    assert!(baseline.fault.is_none() && normal.fault.is_none());
    assert!(normal.steps < baseline.steps);
    assert_eq!(normal.output, 10);
    assert_eq!(
        normal.data,
        [1u32, 3, 6, 10]
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect::<Vec<_>>()
    );
    let failed = run_with_data_size(&linked, Some(2), 16);
    assert_eq!((failed.output, failed.exit), (0, EXIT_SENTINEL));
    assert_eq!(
        failed.data,
        [1u32, 0, 0, 0]
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        failed.trace,
        [
            Access {
                offset: 0,
                width: 4,
                written: None
            },
            Access {
                offset: 0,
                width: 4,
                written: Some(1)
            },
            Access {
                offset: 0,
                width: 4,
                written: None
            },
        ]
    );
    let fault = failed.fault.unwrap();
    assert_eq!(fault.code, FaultCode::BusFault);
    assert_eq!(fault.operation, DebugOperation::MemoryRead);
    let src = fault.source.unwrap();
    let (line, statement) = source
        .lines()
        .enumerate()
        .find(|(_, text)| text.contains("sum = sum + p[i] + i"))
        .unwrap();
    assert_eq!(src.line as usize, line + 1);
    assert_eq!(src.column as usize, statement.find("p[i]").unwrap() + 1);
}

#[test]
fn raw_induction_gate_declines_a_larger_emitted_frame() {
    let fixture = Fixture::new(&source("&Data[0]", "i := 0 as u32; i < 8; i += 1", FOUR, 0));
    let project = fixture.core();
    let package = &project.packages[0];
    let original = package.functions.iter().find(|f| f.name == VISIT).unwrap();
    let mut larger = original.clone();
    // Exercise the actual frame gate with an otherwise profitable emission
    // using another private slot. The production transformation uses the
    // same MIR and reuses one word, so the ordinary cases have equal frames.
    larger.locals.push(mir::Local {
        id: LocalId(u32::try_from(larger.locals.len()).unwrap()),
        name: None,
        ty: TypeId::U64,
        kind: mir::LocalKind::Temporary,
        address_taken: false,
        span: larger.span,
    });
    mir::validate_function(&project.types, &larger).unwrap();
    let plan = Plan::new(&project.types, &package.globals, &larger).unwrap();
    let mut before = Emitter::default();
    let mut after = Emitter::default();
    let mut layout_before = function::EmittedLayout::default();
    let mut layout_after = function::EmittedLayout::default();
    let baseline = function::lower_body(
        &project,
        original,
        &mut before,
        None,
        Some(&mut layout_before),
    )
    .unwrap();
    let candidate = function::lower_body(
        &project,
        &larger,
        &mut after,
        Some(&plan),
        Some(&mut layout_after),
    )
    .unwrap();
    let before = before.resolve().unwrap();
    let after = after.resolve().unwrap();
    assert!(candidate.frame_size > baseline.frame_size);
    let emitted_before = (before.words.as_slice(), &layout_before);
    let emitted_after = (after.words.as_slice(), &layout_after);
    assert!(cost::improves(
        &plan,
        &larger,
        emitted_before,
        emitted_after,
        (candidate.frame_size, candidate.frame_size),
    ));
    assert!(!cost::improves(
        &plan,
        &larger,
        emitted_before,
        emitted_after,
        (baseline.frame_size, candidate.frame_size),
    ));
}
