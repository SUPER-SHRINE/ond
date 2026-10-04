use super::*;

pub(super) fn rows() -> Vec<Row> {
    let mut rows = Vec::new();
    for t in INTEGERS {
        for (label, n) in [
            ("below", t.min - 1),
            ("min", t.min),
            ("zero", 0),
            ("max", t.max),
            ("above", t.max + 1),
        ] {
            let expression = n.to_string();
            let expected = if (t.min..=t.max).contains(&n) {
                integer(n, t.name)
            } else {
                error("out of range", expression.trim_start_matches('-'))
            };
            rows.push(Row::constant(
                format!("NUM-01.{}.{}", t.name, label),
                &["NUM-01"],
                "",
                &expression,
                t.name,
                expected,
            ));
        }
        for (label, expression) in [
            ("plus", "+15".into()),
            ("hex", "0xf".into()),
            ("binary", "0b1111".into()),
            ("octal", "0o17".into()),
            ("hexmax", format!("0x{:x}", t.max)),
        ] {
            let n = if label == "hexmax" { t.max } else { 15 };
            rows.push(Row::constant(
                format!("NUM-01.{}.{}", t.name, label),
                &["NUM-01"],
                "",
                &expression,
                t.name,
                integer(n, t.name),
            ));
        }
        for (label, expression) in [
            ("negative_zero", "-0".to_string()),
            (
                "hex_min",
                if t.min < 0 {
                    format!("-0x{:x}", -t.min)
                } else {
                    "0x0".into()
                },
            ),
        ] {
            rows.push(Row::constant(
                format!("NUM-01.{}.{}", t.name, label),
                &["NUM-01"],
                "",
                &expression,
                t.name,
                integer(if label == "negative_zero" { 0 } else { t.min }, t.name),
            ));
        }
        // All binary integer operators, including six comparisons, at every type.
        for (label, expression, result) in [
            ("add", "6 + 3", 9),
            ("sub", "6 - 3", 3),
            ("mul", "6 * 3", 18),
            ("div", "6 / 3", 2),
            ("rem", "6 % 3", 0),
            ("and", "6 & 3", 2),
            ("or", "6 | 3", 7),
            ("xor", "6 ^ 3", 5),
            ("andnot", "6 &^ 3", 4),
            ("shl", "6 << 1", 12),
            ("shr", "6 >> 1", 3),
            ("plus", "+6", 6),
        ] {
            rows.push(Row::constant(
                format!("NUM-05.{}.{}", t.name, label),
                &["NUM-05"],
                "",
                expression,
                t.name,
                integer(result, t.name),
            ));
        }
        let not = if t.min < 0 { -7 } else { t.max - 6 };
        rows.push(Row::constant(
            format!("NUM-05.{}.not", t.name),
            &["NUM-05"],
            "",
            "^6",
            t.name,
            integer(not, t.name),
        ));
        rows.push(Row::constant(
            format!("NUM-05.{}.neg", t.name),
            &["NUM-05"],
            "",
            "-6",
            t.name,
            if t.min < 0 {
                integer(-6, t.name)
            } else {
                error("out of range", "6")
            },
        ));
        let setup = format!(
            "const A: {} = {}\nconst B: {} = {}",
            t.name, t.min, t.name, t.max
        );
        for (label, op, expected) in [
            ("eq", "==", false),
            ("ne", "!=", true),
            ("lt", "<", true),
            ("le", "<=", true),
            ("gt", ">", false),
            ("ge", ">=", false),
        ] {
            rows.push(Row::constant(
                format!("NUM-05.{}.{}", t.name, label),
                &["NUM-05"],
                &setup,
                &format!("A {op} B"),
                "",
                boolean(expected),
            ));
        }
        for (label, expr) in [
            ("add_overflow", format!("{} + 1", t.max)),
            ("sub_overflow", format!("{} - 1", t.min)),
            ("mul_overflow", format!("{} * 2", t.max)),
        ] {
            rows.push(Row::constant(
                format!("NUM-05.{}.{}", t.name, label),
                &["NUM-05", "NUM-02"],
                "",
                &expr,
                t.name,
                error("out of range", &expr),
            ));
        }
        for count in t.counts {
            let k = count % t.bits;
            for (tag, lhs, op, value) in [
                ("left", 1, "<<", 1i128 << k),
                ("right_max", t.max, ">>", t.max >> k),
                ("right_min", t.min, ">>", t.min >> k),
            ] {
                let setup = format!("const A: {} = {lhs}", t.name);
                let expr = format!("A {op} {count}");
                let expected = if value <= t.max {
                    integer(value, t.name)
                } else {
                    error("out of range", &expr)
                };
                rows.push(Row::constant(
                    format!("NUM-03.{}.{}.{}", t.name, tag, count),
                    &["NUM-03"],
                    &setup,
                    &expr,
                    "",
                    expected,
                ));
            }
        }
        for (tag, count, span, message) in [
            ("negative_count", "-1", "1", "out of range"),
            ("large_count", "4294967296", "4294967296", "out of range"),
            ("typed_count", "K", "K", "type mismatch"),
        ] {
            let setup = if tag == "typed_count" {
                format!("const A: {} = 2\nconst K: i32 = 1", t.name)
            } else {
                format!("const A: {} = 2", t.name)
            };
            rows.push(Row::constant(
                format!("NUM-03.{}.{}", t.name, tag),
                &["NUM-03"],
                &setup,
                &format!("A << {count}"),
                "",
                error(message, span),
            ));
        }
        for (tag, a, b, q, r) in [
            ("pp", 7, 3, 2, 1),
            ("np", -7, 3, -2, -1),
            ("pn", 7, -3, -2, 1),
            ("nn", -7, -3, 2, -1),
        ] {
            if t.min == 0 && (a < 0 || b < 0) {
                continue;
            }
            assert_eq!(a, q * b + r); // Independent small literal oracle.
            let setup = format!("const A: {} = {a}\nconst B: {} = {b}", t.name, t.name);
            for (label, op, value) in [("div", "/", q), ("rem", "%", r)] {
                rows.push(Row::constant(
                    format!("NUM-04.{}.{}.{}", t.name, tag, label),
                    &["NUM-04"],
                    &setup,
                    &format!("A {op} B"),
                    "",
                    integer(value, t.name),
                ));
            }
        }
        for (tag, op) in [("div_zero", "/"), ("rem_zero", "%")] {
            let expr = format!("A {op} 0");
            rows.push(Row::constant(
                format!("NUM-04.{}.{}", t.name, tag),
                &["NUM-04"],
                &format!("const A: {} = 1", t.name),
                &expr,
                "",
                error("zero", &expr),
            ));
        }
        for (tag, n) in [("min", t.min), ("max", t.max)] {
            for (label, op, value) in [("div", "/", n), ("rem", "%", 0)] {
                rows.push(Row::constant(
                    format!("NUM-04.{}.{}_{}_one", t.name, tag, label),
                    &["NUM-04"],
                    &format!("const A: {} = {n}", t.name),
                    &format!("A {op} 1"),
                    "",
                    integer(value, t.name),
                ));
            }
        }
        if t.min < 0 {
            let setup = format!("const A: {} = {}", t.name, t.min);
            rows.push(Row::constant(
                format!("NUM-04.{}.min_div_neg_one", t.name),
                &["NUM-04"],
                &setup,
                "A / -1",
                "",
                error("overflow", "A / -1"),
            ));
            rows.push(Row::constant(
                format!("NUM-04.{}.min_rem_neg_one", t.name),
                &["NUM-04"],
                &setup,
                "A % -1",
                "",
                integer(0, t.name),
            ));
        }
    }
    rows.push(Row::constant(
        "NUM-01.default_i32".into(),
        &["NUM-01"],
        "",
        "2147483647",
        "",
        integer(2147483647, "i32"),
    ));
    rows.push(Row::constant(
        "NUM-01.default_overflow".into(),
        &["NUM-01"],
        "",
        "2147483648",
        "",
        error("out of range", "2147483648"),
    ));
    rows
}
