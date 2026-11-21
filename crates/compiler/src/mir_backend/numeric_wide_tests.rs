//! 数値型の適合case。sourceと表は規範文書から読み込む。
use super::tests::{Fixture, execute_steps, machine};
use crate::linker::{LinkPlan, LinkedImage, link_objects_with_debug};
use crate::test_support::{compile_image, startup_object};
use kagura::{Bus, FaultCode};

const CASES: &str = include_str!("../../../../specs/ond/conformance/cases/numeric-wide.md");
const EXIT: u32 = 0xffff8040;
const OUTPUT: u32 = 0xffff8050;
const STEP_LIMIT: usize = 2_000_000;

fn case(name: &str) -> &str {
    CASES
        .split_once(&format!("\n## WIDE.{name}\n"))
        .unwrap_or_else(|| panic!("missing WIDE.{name}"))
        .1
        .split("\n## WIDE.")
        .next()
        .unwrap()
}

pub(super) fn source(name: &str) -> &str {
    let (_, block) = case(name).split_once("\n```ond\n").unwrap();
    let (source, _) = block.split_once("```").unwrap();
    source
}

fn tables(name: &str) -> impl Iterator<Item = (&str, &str)> {
    case(name).lines().filter_map(|line| {
        let (input, tail) = line.strip_prefix("| `")?.split_once("` | ")?;
        let (want, _) = tail.split_once(" |")?;
        Some((input, want.trim_matches('`')))
    })
}

fn image(name: &str) -> LinkedImage {
    let fixture = Fixture::new(source(name));
    compile_image(&fixture.0).unwrap_or_else(|e| panic!("WIDE.{name}: {e:?}"))
}

#[test]
fn wide_case_inputs_are_available() {
    for name in [
        "integer",
        "float",
        "fixed",
        "fault",
        "reject",
        "abi",
        "memory",
        "retention",
    ] {
        let input = source(name);
        assert!(input.starts_with("package main\n"), "{name}");
        assert!(input.contains("func main()"), "{name}");
        let _ = ond_compiler_core::parse_source_file(crate::source::FileId(0), input)
            .unwrap_or_else(|error| panic!("WIDE.{name}: {error:?}"));
    }
    assert_eq!(tables("fault").count(), 8);
    assert_eq!(tables("reject").count(), 8);
}

#[test]
fn wide_retention_preserves_f32_rounding_and_escaped_storage() {
    assert_eq!(execute_steps(&image("retention"), STEP_LIMIT), 0);
}

#[test]
#[ignore = "64 bit整数が未実装"]
fn wide_integer_wrap_shifts_and_casts() {
    assert_eq!(execute_steps(&image("integer"), STEP_LIMIT), 1);
}

#[test]
#[ignore = "f64が未実装"]
fn wide_binary64_rounding_and_casts() {
    assert_eq!(execute_steps(&image("float"), STEP_LIMIT), 1);
}

#[test]
#[ignore = "固定小数点型が未実装"]
fn wide_fixed_rounding_remainder_and_casts() {
    assert_eq!(execute_steps(&image("fixed"), STEP_LIMIT), 1);
}

#[test]
#[ignore = "数値型の追加とfault分類が未実装"]
fn wide_numeric_faults_keep_committed_output() {
    for (operation, expected) in tables("fault") {
        let source = source("fault").replace("OPERATION", operation);
        let fixture = Fixture::new(&source);
        let compiled = crate::pipeline::compile_project(&fixture.0)
            .unwrap_or_else(|e| panic!("{operation}: {e:?}"));
        let mut objects = vec![startup_object(&[], "main.main")];
        objects.extend(compiled.objects);
        let linked = link_objects_with_debug(
            &objects,
            &compiled.debug_objects,
            &LinkPlan {
                read_only: None,
                memory_base: 0x1000,
                memory_size: 0x10000,
                stack_size: 0x2000,
                entry_symbol: "__ond.entry".into(),
                heap_base_symbol: "__ond_heap_base".into(),
                stack_bottom_symbol: "__ond_stack_bottom".into(),
            },
        )
        .unwrap();
        let (mut cpu, mut bus, _) = machine(&linked.image);
        let fault = (0..STEP_LIMIT)
            .find_map(|_| cpu.step(&mut bus).err())
            .unwrap_or_else(|| panic!("{operation}: expected fault"));
        assert_eq!(fault.code(), FaultCode::InvalidInstruction, "{operation}");
        let pc = fault.faulting_pc();
        assert!(
            linked.debug.ranges.iter().any(|r| {
                r.start <= pc && pc < r.end && format!("{:?}", r.operation) == expected
            }),
            "{operation}: expected {expected} at {pc:#x}"
        );
        assert_eq!(bus.read32(OUTPUT).unwrap(), 1, "{operation}");
        assert_eq!(bus.read32(EXIT).unwrap(), 0xdeadbeef, "{operation}");
    }
}

#[test]
#[ignore = "新しい数値型のCore検査が未実装"]
fn wide_numeric_rejections_require_recognized_types() {
    let source = source("reject");
    Fixture::new(source).core();
    for (declaration, cause) in tables("reject") {
        let invalid = source.replace("// DECLARATION", declaration);
        let fixture = Fixture::new(&invalid);
        let error = ond_compiler_core::compile_project(&fixture.0)
            .expect_err(&format!("{cause}: {declaration}"));
        let start = invalid.rfind(declaration).unwrap();
        assert!(
            error.diagnostics().iter().any(|diagnostic| {
                let span = diagnostic.primary.span;
                !span.is_synthetic() && start <= span.start && span.end <= start + declaration.len()
            }),
            "{cause}: {declaration}: {error:?}"
        );
    }
}

#[test]
#[ignore = "Direct-2 ABIが未実装"]
fn wide_abi_observes_register_stack_split_and_both_return_paths() {
    let image = image("abi");
    let (mut cpu, mut bus, top) = machine(&image);
    let split = image.symbols["main.split"];
    let memory = image.symbols["main.memory"];
    let mut split_return = None;
    let mut memory_return = None;
    let mut seen = [false; 4];

    for _ in 0..STEP_LIMIT {
        let pc = cpu.pc();
        if pc == split {
            assert_eq!(
                (1..=6).map(|r| cpu.reg(r)).collect::<Vec<_>>(),
                [1, 2, 3, 4, 5, 0x55667788]
            );
            let cfa = cpu.reg(14);
            assert_eq!(bus.read32(cfa).unwrap(), 0x11223344);
            assert_eq!(bus.read32(cfa + 4).unwrap(), 9);
            split_return = Some(cpu.reg(15));
            seen[0] = true;
        } else if split_return == Some(pc) {
            assert_eq!(
                (1..=4).map(|r| cpu.reg(r)).collect::<Vec<_>>(),
                [0x55667788, 0x11223344, 9, 15]
            );
            split_return = None;
            seen[1] = true;
        } else if pc == memory {
            assert_eq!(cpu.reg(2), 0x55667788);
            assert_eq!(cpu.reg(3), 0x11223344);
            memory_return = Some((cpu.reg(15), cpu.reg(1)));
            seen[2] = true;
        } else if let Some((_, area)) = memory_return.filter(|(resume, _)| *resume == pc) {
            let words = (0..5)
                .map(|i| bus.read32(area + 4 * i).unwrap())
                .collect::<Vec<_>>();
            assert_eq!(words, [0x55667788, 0x11223344, 0, 0x3ff00000, 0x00020000]);
            memory_return = None;
            seen[3] = true;
        }

        cpu.step(&mut bus).unwrap();
        if bus.read32(EXIT).unwrap() != 0xdeadbeef {
            assert_eq!(seen, [true; 4]);
            assert_eq!(bus.read32(EXIT).unwrap(), 0);
            assert_eq!(bus.read32(OUTPUT).unwrap(), 1);
            assert_eq!(cpu.reg(14), top);
            for r in 7..=11 {
                assert_eq!(cpu.reg(r), 0xabcd0000 + r as u32);
            }
            return;
        }
    }
    panic!("WIDE.abi: CPU step limit");
}
