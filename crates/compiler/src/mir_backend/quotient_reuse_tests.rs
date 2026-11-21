use super::{tests::*, *};
use crate::{
    linker::{LinkPlan, LinkedOutput, link_objects_with_debug},
    program::ProgramMetadata,
};
use kagura::{Bus, FaultCode};
use ond_protocol::debug::DebugOperation;
use std::collections::BTreeMap;

const CALC: &str = "main.calc";
const OUTPUT: u32 = 0xffff8050;
const BEFORE: u32 = OUTPUT + 4;
const AFTER: u32 = OUTPUT + 8;
const EXIT: u32 = 0xffff8040;
const SENTINEL: u32 = 0xdeadbeef;
const STACK: u32 = 8192;
const PAIR: &str = "var q=a/b; var r=a%b; return q+r;";

fn write(address: u32, value: &str) -> String {
    format!("*({address} as u32 as *u32) = {value}")
}

fn source(ty: &str, args: &str, body: &str, extra: &str) -> String {
    let before = write(BEFORE, "123");
    let output = write(OUTPUT, &format!("calc({args}) as u32"));
    let after = write(AFTER, "789");
    format!(
        "package main\nfunc calc(a:{ty}, b:{ty}) -> {ty} {{\n{body}\n}}\n{extra}\n\
         func main() {{ {before}; {output}; {after} }}\n"
    )
}

struct Comparison {
    before: CodegenOutput,
    after: CodegenOutput,
    metadata: ProgramMetadata,
    rewrites: usize,
}

fn compile(source: &str) -> Comparison {
    let fixture = Fixture::new(source);
    let compiled =
        crate::compile_project(&fixture.0).unwrap_or_else(|error| panic!("{error:?}\n{source}"));
    assert_eq!(compiled.mir, fixture.core());
    let paths = compiled
        .loaded
        .packages
        .iter()
        .flat_map(|p| p.files.iter().map(|id| (*id, "main.ond".to_owned())))
        .collect::<BTreeMap<_, _>>();
    let p = &compiled.mir;
    let selected = p.packages.iter().map(|p| p.logical_path.clone()).collect();
    let raw = codegen_selected_using(
        p,
        &selected,
        Some((&compiled.loaded.sources, &paths)),
        |p, _, f, e| function::lower_without_quotient_reuse(p, f, e),
    )
    .unwrap();
    let before = runtime::supply_with_debug(raw.objects, raw.debug).unwrap();
    let after = CodegenOutput {
        objects: compiled.objects,
        debug: compiled.debug_objects,
    };
    let rewrites = p
        .packages
        .iter()
        .flat_map(|p| &p.functions)
        .map(|f| quotient_reuse::plan(f).len())
        .sum();
    let metadata = ProgramMetadata::from_mir(p).unwrap();
    Comparison {
        before,
        after,
        metadata,
        rewrites,
    }
}

impl Comparison {
    fn linked(&self, stack_size: u32) -> [LinkedOutput; 2] {
        [&self.before, &self.after].map(|code| {
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
                    memory_size: 65536,
                    stack_size,
                    entry_symbol: "__ond.entry".into(),
                    heap_base_symbol: "__ond_heap_base".into(),
                    stack_bottom_symbol: "__ond_stack_bottom".into(),
                },
            )
            .unwrap()
        })
    }

    fn preserved_debug(&self) {
        let ranges = |code: &CodegenOutput| {
            code.debug
                .iter()
                .flat_map(|o| &o.ranges)
                .filter(|r| {
                    r.function == CALC
                        && matches!(
                            r.operation,
                            DebugOperation::MemoryRead
                                | DebugOperation::MemoryWrite
                                | DebugOperation::BoundsCheck
                                | DebugOperation::DivisionByZero
                                | DebugOperation::SignedDivisionOverflow
                        )
                })
                .map(|r| (r.operation, r.source.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(ranges(&self.before), ranges(&self.after));
        let user_frames = |code: &CodegenOutput| {
            code.debug
                .iter()
                .flat_map(|o| &o.functions)
                .filter(|f| f.symbol.starts_with("main."))
                .map(|f| (f.symbol.clone(), f.frame_size, f.return_address_offset))
                .collect::<Vec<_>>()
        };
        assert_eq!(user_frames(&self.before), user_frames(&self.after));
    }

    fn unchanged(&self) {
        assert_eq!(self.rewrites, 0);
        // The Debug representation includes all section bytes and relocations.
        assert_eq!(
            format!("{:?}", self.before.objects),
            format!("{:?}", self.after.objects)
        );
        assert_eq!(self.before.debug, self.after.debug);
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ObservedFault {
    code: FaultCode,
    operation: DebugOperation,
    source: Option<SourceLocation>,
    function: String,
}

#[derive(Debug, PartialEq, Eq)]
struct Observation {
    steps: usize,
    peak: u32,
    sp: u32,
    markers: [u32; 2],
    value: u32,
    exit: u32,
    fault: Option<ObservedFault>,
}

impl Observation {
    fn assert_success(&self, value: u32) {
        assert_eq!(self.fault, None);
        assert_eq!(self.exit, 0);
        assert_eq!(self.sp, 0x11000);
        assert_eq!(self.value, value);
        assert_eq!(self.markers, [123, 789]);
    }
}

fn run(linked: &LinkedOutput) -> Observation {
    let (mut cpu, mut bus, top) = machine(&linked.image);
    bus.map_device(BEFORE, 8, ram::Ram::new(8)).unwrap();
    for address in [OUTPUT, BEFORE, AFTER] {
        bus.write32(address, SENTINEL).unwrap();
    }
    let mut low = top;
    for steps in 1..=100_000 {
        let fault = cpu.step(&mut bus).err();
        let sp = cpu.reg(14);
        low = low.min(sp);
        let exit = bus.read32(EXIT).unwrap();
        if fault.is_some() || exit != SENTINEL {
            let fault = fault.map(|fault| {
                let pc = fault.faulting_pc();
                let range = linked
                    .debug
                    .ranges
                    .iter()
                    .filter(|r| r.start <= pc && pc < r.end)
                    .min_by_key(|r| r.end - r.start)
                    .unwrap();
                ObservedFault {
                    code: fault.code(),
                    operation: range.operation,
                    source: range.source.clone(),
                    function: range.function.clone(),
                }
            });
            return Observation {
                steps,
                peak: top - low,
                sp,
                markers: [bus.read32(BEFORE).unwrap(), bus.read32(AFTER).unwrap()],
                value: bus.read32(OUTPUT).unwrap(),
                exit,
                fault,
            };
        }
    }
    panic!("CPU step limit");
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
fn quotient_reuse_normal_source_reduces_steps_and_preserves_stack_boundaries() {
    for (ty, args, value, peak, helper) in [
        ("u8", "250,5", 50, 704, runtime::Helper::URem),
        ("i8", "-100,7", 0xfffffff0, 920, runtime::Helper::IRem),
        ("u16", "60000,13", 0x120c, 704, runtime::Helper::URem),
        ("i16", "-30000,7", 0xffffef3e, 920, runtime::Helper::IRem),
        (
            "u32",
            "4294967295,17",
            0x0f0f0f0f,
            704,
            runtime::Helper::URem,
        ),
        (
            "i32",
            "-2147483648,7",
            0xedb6db6c,
            920,
            runtime::Helper::IRem,
        ),
    ] {
        for keep_helper in [false, true] {
            let extra = if keep_helper {
                format!("func independent(a:{ty}, b:{ty}) -> {ty} {{ return a%b }}")
            } else {
                String::new()
            };
            let comparison = compile(&source(ty, args, PAIR, &extra));
            assert_eq!(comparison.rewrites, 1, "{ty}");
            comparison.preserved_debug();
            let [before, after] = comparison.linked(STACK);
            let has_helper =
                |code: &LinkedOutput| code.image.symbols.contains_key(&helper.symbol());
            assert!(has_helper(&before), "{ty}");
            assert_eq!(has_helper(&after), keep_helper, "{ty}");
            assert!(text_bytes(&after) < text_bytes(&before), "{ty}");
            let before = run(&before);
            let after = run(&after);
            assert!(after.steps < before.steps, "{ty}: {before:?} -> {after:?}");
            for observation in [before, after] {
                observation.assert_success(value);
                assert_eq!(observation.peak, peak, "{ty}");
            }
            for linked in comparison.linked(peak) {
                run(&linked).assert_success(value);
            }
            for shortfall in [4, 8] {
                let [before, after] = comparison.linked(peak - shortfall).each_ref().map(run);
                assert_eq!(before, after, "{ty}");
                assert_eq!(
                    after.fault.as_ref().unwrap().operation,
                    DebugOperation::StackOverflow
                );
                assert_eq!(after.markers, [123, SENTINEL]);
                assert_eq!(after.value, SENTINEL);
            }
        }
    }
}

#[test]
fn quotient_reuse_preserves_numeric_faults_and_signed_remainder_without_a_quotient() {
    let checked = format!(
        "{};\nvar q=a/b; var r=a%b;\n{}; return q+r;",
        write(BEFORE, "234"),
        write(AFTER, "987")
    );
    for (ty, min) in [
        ("u8", None),
        ("i8", Some(-128)),
        ("u16", None),
        ("i16", Some(-32768)),
        ("u32", None),
        ("i32", Some(-2147483648)),
    ] {
        let mut faults = vec![("7,0".to_owned(), DebugOperation::DivisionByZero)];
        if let Some(min) = min {
            faults.push((format!("{min},-1"), DebugOperation::SignedDivisionOverflow));
            let standalone = compile(&source(ty, &format!("{min},-1"), "return a%b", ""));
            standalone.unchanged();
            for linked in standalone.linked(STACK) {
                run(&linked).assert_success(0);
            }
        }
        for (args, operation) in faults {
            let comparison = compile(&source(ty, &args, &checked, ""));
            assert_eq!(comparison.rewrites, 1);
            comparison.preserved_debug();
            let frame_sum = comparison
                .after
                .debug
                .iter()
                .flat_map(|o| &o.functions)
                .filter(|f| matches!(f.symbol.as_str(), CALC | "main.main"))
                .map(|f| f.frame_size)
                .sum();
            for stack in [STACK, frame_sum] {
                let [before, after] = comparison.linked(stack).each_ref().map(run);
                assert_eq!(before, after);
                let fault = after.fault.unwrap();
                assert_eq!(fault.code, FaultCode::InvalidInstruction);
                assert_eq!(fault.operation, operation);
                assert_eq!(fault.function, CALC);
                assert_eq!(fault.source.unwrap().line, 4);
                assert_eq!(after.markers, [234, SENTINEL]);
                assert_eq!(after.exit, SENTINEL);
                assert_eq!(after.value, SENTINEL);
            }
        }
    }

    let body = format!(
        "var q=a/b; {}; b=0;\nvar r=a%b;\n{}; return q+r",
        write(BEFORE, "234"),
        write(AFTER, "987")
    );
    let comparison = compile(&source("u32", "22,7", &body, ""));
    comparison.unchanged();
    let [before, after] = comparison.linked(STACK).each_ref().map(run);
    assert_eq!(before, after);
    let fault = after.fault.unwrap();
    assert_eq!(fault.code, FaultCode::InvalidInstruction);
    assert_eq!(fault.operation, DebugOperation::DivisionByZero);
    assert_eq!(fault.function, CALC);
    assert_eq!(fault.source.unwrap().line, 4);
    assert_eq!(after.markers, [234, SENTINEL]);
}

#[test]
fn quotient_reuse_requires_matching_operands_and_an_uninterrupted_same_block_divide() {
    for (name, ty, args, body, extra, expected) in [
        (
            "lhs write",
            "u32",
            "22,7",
            "var q=a/b; a=11; return q+(a%b)",
            "",
            7,
        ),
        (
            "rhs write",
            "u32",
            "22,7",
            "var q=a/b; b=6; return q+(a%b)",
            "",
            7,
        ),
        (
            "swapped",
            "u32",
            "22,7",
            "var q=a/b; return q+(b%a)",
            "",
            10,
        ),
        (
            "branch",
            "u32",
            "22,7",
            "var q:u32; if a>0 { q=a/b }; return q+(a%b)",
            "",
            4,
        ),
        (
            "call",
            "u32",
            "22,7",
            "var q=a/b; ping(); return q+(a%b)",
            "func ping() {}",
            4,
        ),
        (
            "address-taken",
            "u32",
            "22,7",
            "var p=&a; _=p; var q=a/b; return q+(a%b)",
            "",
            4,
        ),
        (
            "raw read",
            "u32",
            "22,7",
            "var q=a/b; _=*(4294934612 as u32 as *u32); return q+(a%b)",
            "",
            4,
        ),
        (
            "raw write",
            "u32",
            "22,7",
            "var q=a/b; *(4294934612 as u32 as *u32)=123; return q+(a%b)",
            "",
            4,
        ),
        (
            "aggregate access",
            "u32",
            "22,7",
            "var items=[2]u32{a,b}; var q=a/b; var copy=items; return q+(a%b)+copy[1]",
            "",
            11,
        ),
        (
            "asm",
            "u32",
            "22,7",
            "var q=a/b; asm { ADD r7, r0, r0, 123 }; return q+(a%b)",
            "",
            4,
        ),
        (
            "other type",
            "u32",
            "22,7",
            "var q=(a as i32)/(b as i32); return (q as u32)+(a%b)",
            "",
            4,
        ),
        (
            "defined",
            "Word",
            "22 as Word,7 as Word",
            PAIR,
            "type Word u32",
            4,
        ),
        ("unsigned wide", "u64", "22,7", PAIR, "", 4),
        ("signed wide", "i64", "-22,7", PAIR, "", 0xfffffffc),
        ("fixed", "q16", "22,7", PAIR, "", 4),
    ] {
        let comparison = compile(&source(ty, args, body, extra));
        comparison.unchanged();
        let [before, after] = comparison.linked(STACK).each_ref().map(run);
        assert_eq!(before, after, "{name}");
        after.assert_success(expected);
    }

    for (args, value) in [
        ("0,7", 1),
        ("22,-7", 0),
        ("-22,-7", 2),
        ("-128,7", 0xffffffeb),
    ] {
        let body = "var extra:i8; var q=a/b; extra=1; return q+(a%b)+(a%b)+extra;";
        let comparison = compile(&source("i8", args, body, ""));
        assert_eq!(comparison.rewrites, 2);
        comparison.preserved_debug();
        for linked in comparison.linked(STACK) {
            run(&linked).assert_success(value);
        }
    }
}

#[test]
fn quotient_reuse_uses_the_original_ssa_snapshot_after_a_local_store() {
    use mir::{BinaryOp, InstructionKind as I};

    for (dividend, expected) in [("a", 4), ("a+1", 5)] {
        let body = format!("var q=({dividend})/b; a=11; return q+(a%b)");
        let output = write(OUTPUT, "calc(22,7)");
        let fixture = Fixture::new(&format!(
            "package main\nfunc calc(a:u32,b:u32) -> u32 {{ {body} }}\nfunc main() {{ {output} }}"
        ));
        let mut project = fixture.core();
        let calc = project.packages[0]
            .functions
            .iter_mut()
            .find(|f| f.name == CALC)
            .unwrap();
        assert!(quotient_reuse::plan(calc).is_empty());
        let [block] = calc.blocks.as_mut_slice() else {
            panic!("expected a straight-line function");
        };
        let (original_lhs, original_rhs, quotient) = block
            .instructions
            .iter()
            .find_map(|i| match i.kind {
                I::Binary {
                    op: BinaryOp::Divide,
                    lhs,
                    rhs,
                    ..
                } => Some((lhs, rhs, i.results[0])),
                _ => None,
            })
            .unwrap();
        let remainder = block
            .instructions
            .iter_mut()
            .find(|i| {
                matches!(
                    i.kind,
                    I::Binary {
                        op: BinaryOp::Remainder,
                        ..
                    }
                )
            })
            .unwrap();
        let I::Binary { lhs, rhs, .. } = &mut remainder.kind else {
            unreachable!();
        };
        *lhs = original_lhs;
        *rhs = original_rhs;
        let remainder = remainder.id;
        // Keep both checks. The original divisor check already dominates
        // the remainder's use of the division operands.
        assert_eq!(quotient_reuse::plan(calc), [(remainder, quotient)].into());
        let image = crate::test_support::build_image(
            &project,
            crate::ProjectConfig {
                ram_size: 65536,
                stack_size: STACK,
            },
        )
        .unwrap();
        assert_eq!(execute(&image), expected);
    }
}
