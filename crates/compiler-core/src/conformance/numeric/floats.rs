use super::*;

// Golden results are raw IEEE binary32 bits, not host-float evaluations of the
// same source expression. Cases include both directions of ties-to-even.
pub(super) const VALUES: &[(&str, &str, u32)] = &[
    ("add", "1.5 + 2.25", 0x40700000),
    ("subtract", "1.5 - 2.25", 0xbf400000),
    ("multiply", "1.5 * 2.0", 0x40400000),
    ("divide", "1.0 / 3.0", 0x3eaaaaab),
    ("unary_plus", "+1.5", 0x3fc00000),
    ("unary_minus", "-1.5", 0xbfc00000),
    ("add_tie_even", "1.0 + 0x1p-24", 0x3f800000),
    ("add_tie_odd", "0x1.000002p0 + 0x1p-24", 0x3f800002),
    ("sub_tie_even", "0x1.000002p0 - 0x1p-24", 0x3f800000),
    ("sub_tie_odd", "0x1.000004p0 - 0x1p-24", 0x3f800002),
    ("mul_underflow_even", "0x1p-149 * 0.5", 0),
    ("mul_underflow_odd", "0x1.8p-148 * 0.5", 2),
    ("div_underflow_even", "0x1p-149 / 2.0", 0),
    ("div_underflow_odd", "0x1.8p-148 / 2.0", 2),
    ("add_subnormal", "0x1p-149 + 0x1p-149", 2),
    ("sub_subnormal", "0x1p-126 - 0x1.fffffcp-127", 1),
    ("negative_underflow", "-0x1p-149 / 2.0", 0x80000000),
    ("positive_zero", "-0.0 + 0.0", 0),
    ("negative_zero_add", "-0.0 + -0.0", 0x80000000),
    ("negative_zero_sub", "-0.0 - 0.0", 0x80000000),
    ("negative_zero_mul", "0.0 * -1.0", 0x80000000),
    ("negative_zero_div", "0.0 / -1.0", 0x80000000),
    (
        "overflow_add",
        "0x1.fffffep127 + 0x1.fffffep127",
        0x7f800000,
    ),
    (
        "overflow_sub",
        "-0x1.fffffep127 - 0x1.fffffep127",
        0xff800000,
    ),
    ("overflow_mul", "0x1.fffffep127 * 2.0", 0x7f800000),
    ("overflow_div", "0x1.fffffep127 / 0.5", 0x7f800000),
    ("positive_inf", "1.0 / 0.0", 0x7f800000),
    ("negative_inf", "-1.0 / 0.0", 0xff800000),
    ("nan_div", "0.0 / 0.0", 0x7fc00000),
    ("nan_mul", "(1.0 / 0.0) * 0.0", 0x7fc00000),
    ("nan_sub", "(1.0 / 0.0) - (1.0 / 0.0)", 0x7fc00000),
    ("nan_add", "(1.0 / 0.0) + (-1.0 / 0.0)", 0x7fc00000),
    ("nan_neg", "-(0.0 / 0.0)", 0x7fc00000),
];

pub(super) fn rows() -> Vec<Row> {
    let mut rows: Vec<_> = VALUES
        .iter()
        .map(|(id, expr, bits)| {
            Row::constant(
                format!("FP-01.{id}"),
                &["FP-01"],
                "",
                expr,
                "",
                float(*bits),
            )
        })
        .collect();
    for (label, a, b, expected) in [
        (
            "nan_left",
            "0.0 / 0.0",
            "1.0",
            [false, true, false, false, false, false],
        ),
        (
            "nan_right",
            "1.0",
            "0.0 / 0.0",
            [false, true, false, false, false, false],
        ),
        (
            "nan_both",
            "0.0 / 0.0",
            "0.0 / 0.0",
            [false, true, false, false, false, false],
        ),
        (
            "signed_zero",
            "-0.0",
            "0.0",
            [true, false, false, true, false, true],
        ),
        (
            "ordered",
            "-1.5",
            "1.5",
            [false, true, true, true, false, false],
        ),
        (
            "infinities",
            "-1.0 / 0.0",
            "1.0 / 0.0",
            [false, true, true, true, false, false],
        ),
    ] {
        for ((name, op), value) in [
            ("eq", "=="),
            ("ne", "!="),
            ("lt", "<"),
            ("le", "<="),
            ("gt", ">"),
            ("ge", ">="),
        ]
        .into_iter()
        .zip(expected)
        {
            rows.push(Row::constant(
                format!("FP-01.{label}.{name}"),
                &["FP-01"],
                &format!("const A: f32 = {a}\nconst B: f32 = {b}"),
                &format!("A {op} B"),
                "",
                boolean(value),
            ));
        }
    }
    rows
}
