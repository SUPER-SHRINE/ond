//! Source -> MIR -> machine code numerical tests. No float host device.
use super::tests::{Fixture, machine, run};
use kagura::Bus;
use ram::Ram;
pub(super) fn fault(source: &str) {
    let f = Fixture::new(source);
    let bytes = crate::test_support::compile_image(&f.0).unwrap();
    let (mut cpu, mut bus, _) = machine(&bytes);
    for _ in 0..2_000_000 {
        if let Err(e) = cpu.step(&mut bus) {
            assert!(format!("{e:?}").contains("InvalidInstruction"), "{e:?}");
            assert_eq!(bus.read32(0xffff8040).unwrap(), 0xdeadbeef);
            return;
        }
    }
    panic!("expected numeric fault");
}
pub(super) fn input_run(bytes: &crate::linker::LinkedImage, a: u32, b: u32) -> u32 {
    let (mut cpu, mut bus, top) = machine(bytes);
    bus.map_device(0x20000000, 8, Ram::new(8)).unwrap();
    bus.write32(0x20000000, a).unwrap();
    bus.write32(0x20000004, b).unwrap();
    for _ in 0..2_000_000 {
        cpu.step(&mut bus).unwrap();
        if bus.read32(0xffff8040).unwrap() != 0xdeadbeef {
            assert_eq!(cpu.reg(14), top);
            for r in 7..=11 {
                assert_eq!(cpu.reg(r), 0xabcd0000 + r as u32);
            }
            return bus.read32(0xffff8050).unwrap();
        }
    }
    panic!("CPU step limit");
}
const BITS: &str = "func bits(x:f32)->u32{return load32((&x) as u32)}\nfunc frombits(x:u32)->f32{var f:f32;store32((&f) as u32,x);return f}";

#[test]
fn float_operations_match_binary32_vectors() {
    let mut values = vec![
        (0, 0x80000000),
        (1, 1),
        (0x007fffff, 1),
        (0x00800000, 0x3f000000),
        (0x7f7fffff, 0x40000000),
        (0x7f800000, 0xff800000),
        (0x7fc00001, 0),
        (0x3f800000, 0x33800000),
        (0x3f800001, 0x33800000),
        (0xbf800000, 0x40400000),
    ];
    let mut seed = 123u32;
    for _ in 0..128 {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let a = seed;
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        values.push((a, seed));
    }
    for op in ["+", "-", "*", "/"] {
        let f = Fixture::new(&format!(
            "package main\n{BITS}\nfunc main(){{var a=frombits(load32(536870912));var b=frombits(load32(536870916));store32(4294934608,bits(a {op} b))}}"
        ));
        let bytes = crate::test_support::compile_image(&f.0).unwrap();
        for &(a, b) in &values {
            let x = f32::from_bits(a);
            let y = f32::from_bits(b);
            let result = match op {
                "+" => x + y,
                "-" => x - y,
                "*" => x * y,
                _ => x / y,
            };
            let expected = if result.is_nan() {
                0x7fc00000
            } else {
                result.to_bits()
            };
            assert_eq!(input_run(&bytes, a, b), expected, "{a:08x} {op} {b:08x}");
        }
    }
}
#[test]
fn float_comparison_casts_negation_and_storage_execute() {
    for op in ["==", "!=", "<", "<=", ">", ">="] {
        let f = Fixture::new(&format!(
            "package main\n{BITS}\nfunc main(){{var a=frombits(load32(536870912));var b=frombits(load32(536870916));var result:u32;if a {op} b{{result=1}};store32(4294934608,result)}}"
        ));
        let bytes = crate::test_support::compile_image(&f.0).unwrap();
        for (a, b) in [
            (0u32, 0x80000000),
            (0x7fc00001, 0),
            (0xbf800000, 0),
            (0x7f800000, 0x7f800000),
        ] {
            let x = f32::from_bits(a);
            let y = f32::from_bits(b);
            let want = match op {
                "==" => x == y,
                "!=" => x != y,
                "<" => x < y,
                "<=" => x <= y,
                ">" => x > y,
                _ => x >= y,
            };
            assert_eq!(input_run(&bytes, a, b), u32::from(want));
        }
    }
    run(
        &format!("{BITS}\nfunc neg(x:f32)->f32{{return -x}}"),
        "bits(neg(0.0))",
        0x80000000,
    );
    run(
        &format!("{BITS}\nfunc neg(x:f32)->f32{{return -x}}"),
        "bits(neg(frombits(2143289345)))",
        0x7fc00000,
    );
    run("func conv(x:f32)->u8{return x as u8}", "conv(-0.5)", 0);
    run(
        "func conv(x:f32)->i8{return x as i8}",
        "conv(-128.75)",
        (-128i32) as u32,
    );
    run(
        "func conv(x:f32)->u16{return x as u16}",
        "conv(65535.5)",
        65535,
    );
    run(
        &format!("{BITS}\nfunc conv(x:i8)->f32{{return x as f32}}"),
        "bits(conv(-7))",
        (-7.0f32).to_bits(),
    );
    run(
        &format!("{BITS}\nfunc conv(x:u32)->f32{{return x as f32}}"),
        "bits(conv(4294967295))",
        4294967296.0f32.to_bits(),
    );
    run(
        "type S struct{x:f32;y:[2]f32;}\nvar G=S{x:1.5,y:[2]f32{2.5,3.5}}\nfunc calc(s:S)->f32{return s.x+s.y[0]+s.y[1]}",
        "calc(G) as u32",
        7,
    );
}
#[test]
fn float_cast_faults_preserve_destination_range() {
    for (ty, value) in [
        ("u8", "256.0"),
        ("u16", "65536.0"),
        ("i8", "128.0"),
        ("i16", "-32769.0"),
        ("i32", "4294967296.0"),
        ("u32", "-1.0"),
        ("i32", "2147483648.0"),
    ] {
        fault(&format!(
            "package main\nfunc conv(x:f32)->{ty}{{return x as {ty}}}\nfunc main(){{conv({value})}}"
        ));
    }
    for bits in [0x7fc00000u32, 0x7f800000, 0xff800000, 0x7f7fffff] {
        fault(&format!(
            "package main\n{BITS}\nfunc main(){{var x=frombits({bits});var y=x as i32}}"
        ));
    }
}
#[test]
fn integer_division_remainder_widths_and_boundaries_execute() {
    for ty in ["u8", "i8", "u16", "i16", "u32", "i32"] {
        for op in ["/", "%"] {
            run(
                &format!("func calc(a:{ty},b:{ty})->{ty}{{return a {op} b}}"),
                "calc(17,5)",
                if op == "/" { 3 } else { 2 },
            );
            if ty.starts_with('i') {
                run(
                    &format!("func calc(a:{ty},b:{ty})->{ty}{{return a {op} b}}"),
                    "calc(-17,5)",
                    if op == "/" {
                        (-3i32) as u32
                    } else {
                        (-2i32) as u32
                    },
                );
            }
        }
    }
    for op in ["/", "%"] {
        let f = Fixture::new(&format!(
            "package main\nfunc main(){{var a=load32(536870912);var b=load32(536870916);store32(4294934608,a {op} b)}}"
        ));
        let bytes = crate::test_support::compile_image(&f.0).unwrap();
        for (a, b) in [
            (u32::MAX, 1),
            (u32::MAX, u32::MAX),
            (u32::MAX, 0x80000000),
            (0x80000000, 3),
            (0, 7),
            (7, 19),
        ] {
            assert_eq!(
                input_run(&bytes, a, b),
                if op == "/" { a / b } else { a % b }
            );
        }
    }
    for (ty, min) in [("i8", "-128"), ("i16", "-32768"), ("i32", "-2147483648")] {
        run(
            &format!("func calc(a:{ty},b:{ty})->{ty}{{return a%b}}"),
            &format!("calc({min},-1)"),
            0,
        );
        fault(&format!(
            "package main\nfunc calc(a:{ty},b:{ty})->{ty}{{return a/b}}\nfunc main(){{calc({min},-1)}}"
        ));
    }
    for ty in ["u8", "i8", "u16", "i16", "u32", "i32"] {
        for op in ["/", "%"] {
            fault(&format!(
                "package main\nfunc calc(a:{ty},b:{ty})->{ty}{{return a {op} b}}\nfunc main(){{calc(1,0)}}"
            ));
        }
    }
}

#[test]
fn signed_division_vectors_cover_all_sign_combinations() {
    for op in ["/", "%"] {
        let f = Fixture::new(&format!(
            "package main\nfunc main(){{var a=load32(536870912) as i32;var b=load32(536870916) as i32;store32(4294934608,(a {op} b) as u32)}}"
        ));
        let bytes = crate::test_support::compile_image(&f.0).unwrap();
        let mut seed = 771u32;
        for _ in 0..128 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let a = seed as i32;
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let b = seed as i32;
            if b == 0 || (a == i32::MIN && b == -1) {
                continue;
            }
            assert_eq!(
                input_run(&bytes, a as u32, b as u32),
                if op == "/" {
                    (a / b) as u32
                } else {
                    (a % b) as u32
                },
                "{a} {op} {b}"
            );
        }
    }
}
