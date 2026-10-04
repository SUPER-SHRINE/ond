use super::runner::*;
use kagura::FaultCode;
use ond_compiler_core::test_vectors::{INTEGERS, floats, integers};
use std::collections::BTreeMap;
const BITS: &str = "func bits(x:f32)->u32{return load32((&x) as u32)}\nfunc frombits(x:u32)->f32{var f:f32;store32((&f) as u32,x);return f}";

#[test]
fn shared_336_integer_vectors_execute_on_kagura() {
    let rows = integers::rows();
    assert_eq!(rows.len(), 336);
    let mut cache = BTreeMap::new();
    let mut ids = std::collections::BTreeSet::new();
    for v in rows {
        assert!(ids.insert(v.id.clone()));
        let comparison = matches!(v.op, "==" | "!=" | "<" | "<=" | ">" | ">=");
        let b_ty = if matches!(v.op, "<<" | ">>") {
            "u32"
        } else {
            v.ty
        };
        let expression = match v.op {
            "cast" => format!("a as {}", v.ty),
            "unary+" => "+a".into(),
            "unary-" => "-a".into(),
            "unary^" => "^a".into(),
            op => format!("a {op} b"),
        };
        let result = if comparison {
            format!("var r:u32;if {expression}{{r=1}};store32({OUTPUT},r)")
        } else {
            format!("store32({OUTPUT},({expression}) as u32)")
        };
        let source = format!(
            "package main\nfunc main(){{var a=load32({INPUT}) as {};var b=load32({}) as {b_ty};{result}}}",
            v.source_ty,
            INPUT + 4
        );
        let bytes = cache
            .entry(source.clone())
            .or_insert_with(|| compile(&[("main.ond", &source)]));
        let (end, value) = match v.expected {
            integers::ResultValue::Integer(n) => (End::Return, n as u32),
            integers::ResultValue::Fault => (End::Fault(FaultCode::InvalidInstruction), SENTINEL),
        };
        run(
            bytes,
            Case {
                id: &v.id,
                specs: &[v.spec],
                inputs: &[v.a as u32, v.b as u32],
                output: &[value],
                end,
                steps: 2_000_000,
            },
        );
    }
}

#[test]
fn shared_25_float_bit_vectors_execute_on_kagura() {
    assert_eq!(floats::VECTORS.len(), 25);
    let mut cache = BTreeMap::new();
    for v in floats::VECTORS {
        let source = format!(
            "package main\n{BITS}\nfunc main(){{var a=frombits(load32({INPUT}));var b=frombits(load32({}));store32({OUTPUT},bits(a {} b))}}",
            INPUT + 4,
            v.op
        );
        let bytes = cache
            .entry(source.clone())
            .or_insert_with(|| compile(&[("main.ond", &source)]));
        run(
            bytes,
            Case {
                id: v.id,
                specs: &["FP-01", "FP-02"],
                inputs: &[v.lhs, v.rhs],
                output: &[v.bits],
                end: End::Return,
                steps: 2_000_000,
            },
        );
    }
}

#[test]
fn all_twelve_float_conversion_directions_and_boundaries_execute() {
    // Fixed IEEE golden bit patterns, not the compiler's constant evaluator.
    for (ty, min, max) in [
        ("u8", 0, 0x437f0000),
        ("i8", 0xc3000000, 0x42fe0000),
        ("u16", 0, 0x477fff00),
        ("i16", 0xc7000000, 0x46fffe00),
        ("u32", 0, 0x4f800000),
        ("i32", 0xcf000000, 0x4f000000),
    ] {
        let t = INTEGERS.iter().find(|t| t.name == ty).unwrap();
        let source = format!(
            "package main\n{BITS}\nfunc main(){{var a=load32({INPUT}) as {ty};store32({OUTPUT},bits(a as f32))}}"
        );
        let bytes = compile(&[("main.ond", &source)]);
        let mut rows = vec![(t.min as u32, min), (t.max as u32, max)];
        if t.bits == 32 {
            for (n, bits) in [
                (16777215, 0x4b7fffff),
                (16777216, 0x4b800000),
                (16777217, 0x4b800000),
                (16777218, 0x4b800001),
                (16777219, 0x4b800002),
            ] {
                rows.push((n, bits));
                if t.min < 0 {
                    rows.push(((0u32).wrapping_sub(n), bits | 0x80000000));
                }
            }
        }
        for (input, expected) in rows {
            run(
                &bytes,
                Case {
                    id: &format!("CAST-03.{ty}.f32.{input}"),
                    specs: &["CAST-03"],
                    inputs: &[input],
                    output: &[expected],
                    end: End::Return,
                    steps: 2_000_000,
                },
            );
        }
    }
    for (ty, lo, below, hi, above, max) in [
        ("u8", 0, 0xbf800000, 0x437f0000, 0x43800000, 255),
        ("i8", 0xc3000000, 0xc3010000, 0x42fe0000, 0x43000000, 127),
        ("u16", 0, 0xbf800000, 0x477fff00, 0x47800000, 65535),
        ("i16", 0xc7000000, 0xc7000100, 0x46fffe00, 0x47000000, 32767),
        ("u32", 0, 0xbf800000, 0x4f7fffff, 0x4f800000, 4294967040),
        (
            "i32", 0xcf000000, 0xcf000001, 0x4effffff, 0x4f000000, 2147483520,
        ),
    ] {
        let t = INTEGERS.iter().find(|t| t.name == ty).unwrap();
        let source = format!(
            "package main\n{BITS}\nfunc main(){{var a=frombits(load32({INPUT}));var p={OUTPUT} as *{ty};*p=a as {ty}}}"
        );
        // Read narrow stores as raw words: the untouched upper bytes stay sentinel.
        let bytes = compile(&[("main.ond", &source)]);
        let rows = [
            (lo, Some(t.min as u32)),
            (below, None),
            (hi, Some(max)),
            (above, None),
            (0x3fe00000, Some(1)),
            (0xbfe00000, if t.min < 0 { Some(u32::MAX) } else { None }),
            (0x80000000, Some(0)),
            (0xbf400000, Some(0)),
            (0x7fc00000, None),
            (0x7f800000, None),
            (0xff800000, None),
        ];
        for (input, expected) in rows {
            let (end, value) = if let Some(n) = expected {
                let mask = u32::MAX >> (32 - t.bits);
                (End::Return, (SENTINEL & !mask) | (n & mask))
            } else {
                (End::Fault(FaultCode::InvalidInstruction), SENTINEL)
            };
            run(
                &bytes,
                Case {
                    id: &format!("CAST-03.f32.{ty}.{input:x}"),
                    specs: &["CAST-03"],
                    inputs: &[input],
                    output: &[value],
                    end,
                    steps: 2_000_000,
                },
            );
        }
    }
}
