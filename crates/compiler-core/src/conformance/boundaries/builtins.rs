use super::*;

pub(super) fn rows() -> Vec<Row> {
    let mut rows = Vec::new();
    for (id, call, message, span) in [
        ("new-empty", "new()", "expected type", ")"),
        ("new-extra", "new(i32, u8)", "expected `)`", ","),
    ] {
        let mut row = Row::new(
            format!("BI-01.{id}"),
            &["BI-01"],
            format!("var allocated = {call}"),
        )
        .reject(message, span);
        row.occurrence = Some(0);
        rows.push(row);
    }
    for call in ["free(nil)", "store32(0, 0)"] {
        rows.push(
            Row::new(
                format!("BI-01.void-result.{call}"),
                &["BI-01"],
                format!("func test() {{ var x = {call} }}"),
            )
            .reject(
                "declaration value count mismatch: expected 1, found 0",
                format!("x = {call}"),
            ),
        );
    }
    for (name, arity, spec) in [
        ("free", 1, &["BI-01"][..]),
        ("load32", 1, &["BI-01"][..]),
        ("store32", 2, &["BI-01"][..]),
        ("len", 1, &["BI-02"][..]),
        ("thisFile", 0, &["BI-03"][..]),
        ("thisLine", 0, &["BI-03"][..]),
    ] {
        for count in 0..=3 {
            if count == arity {
                continue;
            }
            let call = format!("{name}({})", vec!["0"; count].join(", "));
            rows.push(
                Row::new(
                    format!("{name}.arity.{count}"),
                    spec,
                    format!("func test() {{ {call} }}"),
                )
                .reject(
                    match name {
                        "free" => "free expects 1 argument",
                        "load32" => "load32 expects 1 argument",
                        "store32" => "store32 expects 2 arguments",
                        "len" => "len expects 1 argument",
                        _ => {
                            if name == "thisFile" {
                                "thisFile expects 0 arguments"
                            } else {
                                "thisLine expects 0 arguments"
                            }
                        }
                    },
                    call,
                ),
            );
        }
    }
    for (i, ty) in TYPES.iter().enumerate() {
        let pointer = matches!(*ty, "*u8" | "*i32");
        for (name, call, valid, message, whole, spec) in [
            (
                "alloc",
                "alloc[u8](src)",
                *ty == "u32",
                IDENTICAL,
                false,
                &["BI-01"][..],
            ),
            (
                "free",
                "free(src)",
                pointer,
                "free requires a pointer",
                true,
                &["BI-01"][..],
            ),
            (
                "load32",
                "load32(src)",
                pointer || *ty == "u32",
                "memory address requires u32 or pointer",
                true,
                &["BI-01"][..],
            ),
            (
                "store-address",
                "store32(src, 0)",
                pointer || *ty == "u32",
                "memory address requires u32 or pointer",
                true,
                &["BI-01"][..],
            ),
            (
                "store-value",
                "store32(0, src)",
                *ty == "u32",
                IDENTICAL,
                false,
                &["BI-01"][..],
            ),
            (
                "len",
                "len(src)",
                *ty == "[2]i32",
                "len requires an array",
                true,
                &["BI-02"][..],
            ),
        ] {
            let row = Row::new(
                format!("builtin.{name}.type.{i}"),
                spec,
                format!("func test(src: {ty}) {{ {call} }}"),
            );
            rows.push(if valid {
                row
            } else {
                row.reject(message, if whole { call } else { "src" })
            });
        }
    }
    for (id, code) in [
        ("alloc-result", "func test() -> *u8 { return alloc[u8](4) }"),
        (
            "alloc-typed-result",
            "func test() -> *S { return alloc[S](4) }",
        ),
        ("load-result", "func test() -> u32 { return load32(0) }"),
        ("free-nil", "func test() { free(nil) }"),
        ("new-array", "func test() -> *[4]u8 { return new([4]u8) }"),
        ("new-defined", "func test() -> *S { return new(S) }"),
    ] {
        rows.push(Row::new(format!("BI-01.{id}"), &["BI-01"], code));
    }
    rows.push(
        Row::new(
            "BI-01.new-not-element-pointer",
            &["BI-01"],
            "func test() { var p: *u8 = new([4]u8) }",
        )
        .reject(IDENTICAL, "new([4]u8)"),
    );
    for (id, code) in [
        ("len-result", "func test(s: [2]u8) -> u32 { return len(s) }"),
        (
            "defined-array",
            "type Array [2]i32\nfunc test(s: Array) -> u32 { return len(s) }",
        ),
    ] {
        rows.push(Row::new(format!("BI-02.{id}"), &["BI-02"], code));
    }
    for (id, code) in [
        (
            "this-file-result",
            "func test() -> *u8 { return thisFile() }",
        ),
        (
            "this-line-result",
            "func test() -> u32 { return thisLine() }",
        ),
    ] {
        rows.push(Row::new(format!("BI-03.{id}"), &["BI-03"], code));
    }
    rows
}
