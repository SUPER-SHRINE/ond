use super::*;

pub(super) fn rows() -> Vec<Row> {
    let mut rows = Vec::new();
    for from in INTEGERS {
        for to in INTEGERS {
            // Every one of the 36 source/destination pairs. Include both source
            // extrema and each destination boundary reachable in the source.
            let values: std::collections::BTreeSet<_> = [
                from.min,
                from.max,
                0,
                1,
                to.min - 1,
                to.min,
                to.max,
                to.max + 1,
            ]
            .into_iter()
            .filter(|n| (from.min..=from.max).contains(n))
            .collect();
            for n in values {
                let setup = format!("const A: {} = {n}", from.name);
                let expr = format!("A as {}", to.name);
                let expected = if (to.min..=to.max).contains(&n) {
                    integer(n, to.name)
                } else {
                    error("out of range", &expr)
                };
                rows.push(Row::constant(
                    format!("CAST-02.{}.{}.{}", from.name, to.name, n),
                    &["CAST-02"],
                    &setup,
                    &expr,
                    "",
                    expected,
                ));
            }
        }
    }
    // Explicit IEEE binary32 rounded integer extrema, independent golden bits.
    for (ty, min, max) in [
        ("u8", 0, 0x437f0000),
        ("i8", 0xc3000000, 0x42fe0000),
        ("u16", 0, 0x477fff00),
        ("i16", 0xc7000000, 0x46fffe00),
        ("u32", 0, 0x4f800000),
        ("i32", 0xcf000000, 0x4f000000),
    ] {
        let t = INTEGERS.iter().find(|t| t.name == ty).unwrap();
        for (label, n, bits) in [("min", t.min, min), ("max", t.max, max)] {
            rows.push(Row::constant(
                format!("CAST-03.{}.to_f32.{}", ty, label),
                &["CAST-03"],
                &format!("const A: {ty} = {n}"),
                "A as f32",
                "",
                float(bits),
            ));
        }
    }
    for ty in ["i32", "u32"] {
        for (n, bits) in [
            (16777215, 0x4b7fffff),
            (16777216, 0x4b800000),
            (16777217, 0x4b800000),
            (16777218, 0x4b800001),
            (16777219, 0x4b800002),
        ] {
            rows.push(Row::constant(
                format!("CAST-03.{}.ties.{}", ty, n),
                &["CAST-03"],
                &format!("const A: {ty} = {n}"),
                "A as f32",
                "",
                float(bits),
            ));
            if ty == "i32" {
                rows.push(Row::constant(
                    format!("CAST-03.{}.negative_ties.{}", ty, n),
                    &["CAST-03"],
                    &format!("const A: {ty} = -{n}"),
                    "A as f32",
                    "",
                    float(bits | 0x80000000),
                ));
            }
        }
    }
    // Exact hex floats straddle each destination's representable interval.
    for (ty, lo, below, hi, above, hi_value) in [
        ("u8", "0.0", "-1.0", "255.0", "256.0", 255),
        ("i8", "-128.0", "-129.0", "127.0", "128.0", 127),
        ("u16", "0.0", "-1.0", "65535.0", "65536.0", 65535),
        ("i16", "-32768.0", "-32769.0", "32767.0", "32768.0", 32767),
        ("u32", "0.0", "-1.0", "0x1.fffffep31", "0x1p32", 4294967040),
        (
            "i32",
            "-0x1p31",
            "-0x1.000002p31",
            "0x1.fffffep30",
            "0x1p31",
            2147483520,
        ),
    ] {
        let t = INTEGERS.iter().find(|t| t.name == ty).unwrap();
        for (label, literal, result) in [
            ("min", lo, Some(t.min)),
            ("below", below, None),
            ("max", hi, Some(hi_value)),
            ("above", above, None),
            ("positive_fraction", "1.75", Some(1)),
            (
                "negative_fraction",
                "-1.75",
                if t.min < 0 { Some(-1) } else { None },
            ),
            ("negative_zero", "-0.0", Some(0)),
            ("negative_small_fraction", "-0.75", Some(0)),
            ("nan", "0.0 / 0.0", None),
            ("positive_inf", "1.0 / 0.0", None),
            ("negative_inf", "-1.0 / 0.0", None),
        ] {
            let expr = format!("A as {ty}");
            rows.push(Row::constant(
                format!("CAST-03.f32.{}.{}", ty, label),
                &["CAST-03"],
                &format!("const A: f32 = {literal}"),
                &expr,
                "",
                match result {
                    Some(n) => integer(n, ty),
                    None => error("constant", &expr),
                },
            ));
        }
    }
    rows
}
