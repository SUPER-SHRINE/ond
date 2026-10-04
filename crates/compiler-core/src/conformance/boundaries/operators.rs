use super::*;

pub(super) fn rows() -> Vec<Row> {
    let mut rows = Vec::new();
    for (i, ty) in TYPES.iter().enumerate() {
        let integer = integer(ty);
        for op in [
            "+", "-", "*", "/", "%", "&", "|", "^", "&^", "<<", ">>", "==", "!=", "<", "<=", ">",
            ">=", "&&", "||",
        ] {
            let logical = matches!(op, "&&" | "||");
            let equality = matches!(op, "==" | "!=");
            let comparison = equality || matches!(op, "<" | "<=" | ">" | ">=");
            let shift = matches!(op, "<<" | ">>");
            let valid = if logical {
                *ty == "bool"
            } else {
                integer
                    || (*ty == "f32" && (comparison || matches!(op, "+" | "-" | "*" | "/")))
                    || (matches!(*ty, "bool" | "*u8" | "*i32" | "func()") && equality)
            };
            let rhs = if shift { "u32" } else { ty };
            let result = if comparison || logical { "bool" } else { ty };
            let expr = format!("lhs {op} rhs");
            let row = Row::new(
                format!("OP-01.binary.{i}.{op}"),
                &["OP-01"],
                format!("func test(lhs: {ty}, rhs: {rhs}) -> {result} {{ return {expr} }}"),
            );
            rows.push(if valid {
                row
            } else if logical {
                row.reject(IDENTICAL, "lhs")
            } else {
                row.reject("invalid binary operand type", expr)
            });
        }
        for op in ["+", "-", "^", "!"] {
            let valid = match op {
                "!" => *ty == "bool",
                "^" => integer,
                _ => integer || *ty == "f32",
            };
            let row = Row::new(
                format!("OP-01.unary.{i}.{op}"),
                &["OP-01"],
                format!("func test(src: {ty}) -> {ty} {{ return {op}src }}"),
            );
            rows.push(if valid {
                row
            } else {
                row.reject("invalid unary operand type", "src")
            });
        }
    }
    rows
}
