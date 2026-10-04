use super::*;

pub(super) fn rows() -> Vec<Row> {
    let mut rows = Vec::new();
    for (i, from) in TYPES.iter().enumerate() {
        for (j, to) in TYPES.iter().enumerate() {
            for (context, code) in [
                (
                    "assignment",
                    format!("func test(src: {from}) {{ var dst: {to}\ndst = src }}"),
                ),
                (
                    "argument",
                    format!("func sink(dst: {to}) {{}}\nfunc test(src: {from}) {{ sink(src) }}"),
                ),
                (
                    "return",
                    format!("func test(src: {from}) -> {to} {{ return src }}"),
                ),
            ] {
                let row = Row::new(format!("CAST-01.{context}.{i}.{j}"), &["CAST-01"], code);
                rows.push(if from == to {
                    row
                } else {
                    row.reject(IDENTICAL, "src")
                });
            }
            // All numeric pairs; integer/pointer casts; identical underlying types only otherwise.
            let integer = integer(from);
            let target_integer = super::integer(to);
            let numeric = integer || *from == "f32";
            let target_numeric = target_integer || *to == "f32";
            let pointer = matches!(*from, "*u8" | "*i32");
            let target_pointer = matches!(*to, "*u8" | "*i32");
            let valid = from == to
                || (numeric && target_numeric)
                || (integer && target_pointer)
                || (pointer && target_integer);
            let row = Row::new(
                format!("CAST-01.explicit.{i}.{j}"),
                &["CAST-01"],
                format!("func test(src: {from}) -> {to} {{ return src as {to} }}"),
            );
            rows.push(if valid {
                row
            } else {
                row.reject("invalid explicit type conversion", "src")
            });
        }
        let nullable = matches!(*from, "*u8" | "*i32" | "func()");
        for (context, code) in [
            (
                "assignment",
                format!("func test() {{ var dst: {from} = nil }}"),
            ),
            (
                "argument",
                format!("func sink(dst: {from}) {{}}\nfunc test() {{ sink(nil) }}"),
            ),
            ("return", format!("func test() -> {from} {{ return nil }}")),
        ] {
            let row = Row::new(format!("NIL-01.{context}.{i}"), &["NIL-01"], code);
            rows.push(if nullable {
                row
            } else {
                row.reject("nil requires a pointer or function type", "nil")
            });
        }
        if nullable {
            for op in ["==", "!="] {
                for (order, expr) in [
                    ("right", format!("src {op} nil")),
                    ("left", format!("nil {op} src")),
                ] {
                    rows.push(Row::new(
                        format!("NIL-01.compare.{i}.{op}.{order}"),
                        &["NIL-01", "OP-01"],
                        format!("func test(src: {from}) -> bool {{ return {expr} }}"),
                    ));
                }
            }
            rows.push(
                Row::new(
                    format!("NIL-01.integer-zero.{i}"),
                    &["NIL-01"],
                    format!("func test() {{ var dst: {from} = 0 }}"),
                )
                .reject("integer literal requires an integer type", "0"),
            );
        } else {
            for (order, expr) in [("left", "nil == src"), ("right", "src == nil")] {
                rows.push(
                    Row::new(
                        format!("NIL-01.invalid-comparison.{i}.{order}"),
                        &["NIL-01"],
                        format!("func test(src: {from}) -> bool {{ return {expr} }}"),
                    )
                    .reject("nil requires a pointer or function type", "nil"),
                );
            }
        }
    }
    for (id, code) in [
        ("var", "func test() { var x = nil }"),
        ("const", "const x = nil"),
        ("comparison", "func test() -> bool { return nil == nil }"),
    ] {
        let mut row = Row::new(format!("NIL-01.no-context.{id}"), &["NIL-01"], code).reject(
            "requires an explicit pointer or function type context",
            "nil",
        );
        row.occurrence = Some(0);
        rows.push(row);
    }
    rows
}
