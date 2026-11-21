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
fn wide_integer_wrap_shifts_and_casts() {
    assert_eq!(execute_steps(&image("integer"), STEP_LIMIT), 1);
}

#[test]
fn wide_binary64_rounding_and_casts() {
    assert_eq!(execute_steps(&image("float"), STEP_LIMIT), 1);
}

#[test]
fn wide_fixed_rounding_remainder_and_casts() {
    assert_eq!(execute_steps(&image("fixed"), STEP_LIMIT), 1);
}

#[test]
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

#[test]
fn f32_chain_kagura_instruction_budget() {
    let (mut cpu, mut bus, _) = machine(&image("retention"));
    let mut reads = 0;
    let mut writes = 0;
    for instructions in 1..=STEP_LIMIT {
        match bus.read32(cpu.pc()).unwrap() >> 28 {
            4 | 5 | 7 => reads += 1,
            8 | 9 | 11 => writes += 1,
            _ => {}
        }
        cpu.step(&mut bus).unwrap();
        if bus.read32(EXIT).unwrap() != 0xdeadbeef {
            assert_eq!(bus.read32(OUTPUT).unwrap(), 0);
            // Baseline on the original lowering with the same image input.
            assert!(
                instructions < 5056 && reads < 676 && writes < 593,
                "retired: {instructions}, data loads: {reads}, data stores: {writes}"
            );
            return;
        }
    }
    panic!("f32 chain did not finish");
}

fn binary_vectors(ty: &str, op: &str, vectors: &[(u64, u64)], expected: impl Fn(u64, u64) -> u64) {
    const INPUT: u32 = 0x2000_0000;
    let input_b = INPUT + 8;
    let source = format!(
        "package main\nfunc main() {{ var a = *({INPUT} as u32 as *{ty}); var b = *({input_b} as u32 as *{ty}); *({OUTPUT} as u32 as *{ty}) = a {op} b }}"
    );
    let fixture = Fixture::new(&source);
    let image = compile_image(&fixture.0).unwrap();
    for &(a, b) in vectors {
        let (mut cpu, mut bus, _) = machine(&image);
        bus.map_device(OUTPUT + 4, 4, ram::Ram::new(4)).unwrap();
        bus.map_device(INPUT, 16, ram::Ram::new(16)).unwrap();
        for (offset, bits) in [(0, a), (8, b)] {
            bus.write32(INPUT + offset, bits as u32).unwrap();
            bus.write32(INPUT + offset + 4, (bits >> 32) as u32)
                .unwrap();
        }
        let finished = (0..STEP_LIMIT).any(|_| {
            cpu.step(&mut bus).unwrap();
            bus.read32(EXIT).unwrap() != 0xdeadbeef
        });
        assert!(finished, "{ty} {a:x} {op} {b:x}");
        let actual = u64::from(bus.read32(OUTPUT).unwrap())
            | (u64::from(bus.read32(OUTPUT + 4).unwrap()) << 32);
        assert_eq!(actual, expected(a, b), "{ty} {a:016x} {op} {b:016x}");
    }
}

#[test]
fn wide_binary64_cpu_vectors_match_independent_ieee_results() {
    let specials = [
        0,
        1,
        0x000f_ffff_ffff_ffff,
        0x0010_0000_0000_0000,
        1.0f64.to_bits(),
        2.0f64.to_bits(),
        (-0.0f64).to_bits(),
        (-1.0f64).to_bits(),
        f64::MAX.to_bits(),
        f64::INFINITY.to_bits(),
        f64::NEG_INFINITY.to_bits(),
        0x7ff0_0000_0000_0001,
    ];
    let mut vectors = specials
        .iter()
        .zip(specials.iter().rev())
        .map(|(&a, &b)| (a, b))
        .collect::<Vec<_>>();
    let mut seed = 19079u64;
    for _ in 0..40 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let a = seed;
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        vectors.push((a, seed));
    }
    for op in ["+", "-", "*", "/"] {
        binary_vectors("f64", op, &vectors, |a, b| {
            let (a, b) = (f64::from_bits(a), f64::from_bits(b));
            let result = match op {
                "+" => a + b,
                "-" => a - b,
                "*" => a * b,
                "/" => a / b,
                _ => unreachable!(),
            };
            if result.is_nan() {
                0x7ff8_0000_0000_0000
            } else {
                result.to_bits()
            }
        });
    }
}

#[test]
fn wide_fixed_cpu_multiply_divide_and_remainder_match_exact_values() {
    let vectors = [
        (1, 0x8000_0000),
        (3, 0x8000_0000),
        ((-3i64) as u64, 0x8000_0000),
        (1 << 50, 11 << 30),
        (0xaaaaaaaa, 0x333333333),
        (0xffffffff, -0x77777777i64 as u64),
        (0x7fffffffffffffffu64, 0x1_0000_0000),
    ];
    for op in ["*", "/", "%"] {
        binary_vectors("q32", op, &vectors, |a, b| {
            let a = i128::from(a as i64);
            let b = i128::from(b as i64);
            let (numerator, divisor) = match op {
                "*" => (a * b, 1 << 32),
                "/" => (a << 32, b),
                "%" => return (a % b) as u64,
                _ => unreachable!(),
            };
            let (n, d) = (numerator.abs(), divisor.abs());
            let truncated = n / d;
            let remainder = n % d;
            let magnitude = truncated
                + i128::from(remainder * 2 > d || (remainder * 2 == d && truncated % 2 == 1));
            (magnitude * numerator.signum() * divisor.signum()) as u64
        });
    }
}

#[test]
fn wide_casts_to_binary32_round_once_in_constants_and_at_runtime() {
    super::tests::run(
        r#"func bits(value:f32)->u32 { return *((&value) as u32 as *u32) }
func fixed(raw:u64)->q32 { return *((&raw) as u32 as *q32) }
const FromInt:f32 = (9223372586610589697 as u64) as f32
const FromFixed:f32 = (1073741888.00000000023283064365386962890625 as q32) as f32
func check(a:u64,b:u64)->u32 {
    if bits(FromInt) != 0x5f000001 || bits(a as f32) != 0x5f000001 { return 0 }
    if bits(FromFixed) != 0x4e800001 || bits(fixed(b) as f32) != 0x4e800001 { return 0 }
    return 1
}"#,
        "check(9223372586610589697,4611686293305294849)",
        1,
    );
}

#[test]
fn signed_fixed_literal_respects_custom_unary_operator() {
    super::tests::run(
        r#"type Amount q16
operator (amount:Amount) -() -> Amount { return 42 }
func check() -> u32 { var x:Amount = -1.5; return x as u32 }"#,
        "check()",
        42,
    );
}

#[test]
fn wide_type_layout_checks_unused_array_and_pointer_element() {
    for declaration in ["type Excess [536870912]u64", "type Excess *[536870912]q32"] {
        let fixture = Fixture::new(&format!("package main\n{declaration}\nfunc main() {{}}"));
        let error = ond_compiler_core::compile_project(&fixture.0).unwrap_err();
        assert!(
            error.to_string().contains("array layout overflow"),
            "{error:?}"
        );
    }
}

#[test]
fn wide_integer_cpu_product_quotient_and_remainder_match_host_integers() {
    let mut vectors = vec![
        (0, 1),
        (u64::MAX, 0xffff_ffff),
        (u64::MAX, 0x1_0000_0001),
        (i64::MIN as u64, 1),
        (i64::MIN as u64, 3),
        (i64::MIN as u64, i64::MAX as u64),
        (-1234567891234i64 as u64, -9i64 as u64),
    ];
    let mut seed = 59135u64;
    for index in 0..30 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let a = seed;
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        vectors.push((a, (seed >> index) | 1));
    }
    for op in ["*", "/", "%"] {
        binary_vectors("u64", op, &vectors, |a, b| match op {
            "*" => a.wrapping_mul(b),
            "/" => a / b,
            "%" => a % b,
            _ => unreachable!(),
        });
        binary_vectors("i64", op, &vectors, |a, b| {
            let (a, b) = (a as i64, b as i64);
            (match op {
                "*" => a.wrapping_mul(b),
                "/" => a / b,
                "%" => a % b,
                _ => unreachable!(),
            }) as u64
        });
    }
}

#[test]
fn wide_mir_rejects_lost_checked_effects() {
    use ond_compiler_core::mir::{BinaryOp, CastBehavior, InstructionKind, OverflowBehavior};
    let fixture = Fixture::new(
        "package main\nfunc add(a:q32,b:q32)->q32{return a+b}\n\
         func int(v:f64)->u64{return v as u64}\n\
         func narrow(v:f64)->f32{return v as f32}\n\
         func fixed(v:f64)->q16{return v as q16}\nfunc main(){}",
    );
    let project = fixture.core();
    for name in ["main.add", "main.int", "main.narrow", "main.fixed"] {
        let mut mutated = project.clone();
        let f = mutated.packages[0]
            .functions
            .iter_mut()
            .find(|function| function.name == name)
            .unwrap();
        let mut changed = false;
        for instruction in f
            .blocks
            .iter_mut()
            .flat_map(|block| &mut block.instructions)
        {
            match &mut instruction.kind {
                InstructionKind::Binary {
                    op: BinaryOp::Add,
                    overflow,
                    ..
                } => {
                    assert_eq!(*overflow, OverflowBehavior::Checked);
                    *overflow = OverflowBehavior::Wrap;
                    changed = true;
                }
                InstructionKind::Cast { behavior, .. } => {
                    assert_eq!(*behavior, CastBehavior::Checked);
                    *behavior = CastBehavior::Infallible;
                    changed = true;
                }
                _ => {}
            }
        }
        assert!(changed, "{name}");
        assert!(
            ond_compiler_core::mir::validate_project(&mutated).is_err(),
            "{name}"
        );
    }
}
