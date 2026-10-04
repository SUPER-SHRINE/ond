use super::*;

pub(super) fn rows() -> Vec<Row> {
    let mut rows = Vec::new();
    for (id, code, message) in [
        (
            "local",
            "func test() { var x }",
            "variable without initializer requires a type",
        ),
        (
            "global",
            "var x",
            "global without initializer requires a type",
        ),
    ] {
        rows.push(
            Row::new(format!("DECL-01.no-type-or-value.{id}"), &["DECL-01"], code)
                .reject(message, "x"),
        );
    }
    for (i, ty) in TYPES.iter().enumerate() {
        rows.push(
            Row::new(
                format!("DECL-01.zero.{i}"),
                &["DECL-01"],
                format!("var g: {ty}\nfunc test() -> {ty} {{ var x: {ty}\nreturn x }}"),
            )
            .check(Check::Mir {
                description: "typed local zero and zero-initialized global storage",
                verify: zero_storage,
            }),
        );
        rows.push(Row::new(
            format!("DECL-01.infer.{i}"),
            &["DECL-01"],
            format!("func test(src: {ty}) -> {ty} {{ var x = src\nreturn x }}"),
        ));
        let other = if *ty == "bool" { "i32" } else { "bool" };
        rows.push(
            Row::new(
                format!("DECL-01.fixed-inference.{i}"),
                &["DECL-01"],
                format!("func test(src: {ty}, other: {other}) {{ var x = src\nx = other }}"),
            )
            .reject(IDENTICAL, "other"),
        );
    }
    for (id, code) in [
        ("nil-function", "const X: func() = nil"),
        ("nil-pointer", "const X: *i32 = nil"),
        ("defined-array", "type Array [2]i32\nconst X = Array{1, 2}"),
        ("defined-struct", "const X = S{x: 1}"),
    ] {
        rows.push(Row::new(format!("DECL-02.{id}"), &["DECL-02"], code));
    }
    for (id, setup, expr) in [
        ("call", "func f() -> i32 { return 0 }", "f()"),
        ("function", "func callable() {}", "callable"),
        ("global", "var g = 0", "g"),
        ("load", "", "load32(0)"),
        ("alloc", "", "alloc[u8](4)"),
        ("new", "", "new(i32)"),
        ("address", "var g = 0", "&g"),
        ("array-global", "var g = 0", "[2]i32{g, 0}"),
        ("struct-global", "var g = 0", "S{x: g}"),
    ] {
        rows.push(
            Row::new(
                format!("DECL-02.nonconstant.{id}"),
                &["DECL-02"],
                format!("{setup}\nconst X = {expr}"),
            )
            .reject("constant initializer requires a compile-time value", expr),
        );
    }
    for (id, declaration, expression, ty, value) in [
        (
            "function-nil-value",
            "const F: func() = nil",
            "F",
            "func() -> ()",
            Scalar::Nil,
        ),
        (
            "named-array-values",
            "type Array [2]i32\nconst A = Array{1, 2}",
            "A[0] + A[1]",
            "i32",
            Scalar::Integer(3),
        ),
        (
            "named-array-key",
            "type Array [3]i32\nconst A = Array{1 + 1: 7}",
            "A[0] + A[2]",
            "i32",
            Scalar::Integer(7),
        ),
        (
            "named-struct-values",
            "const A = S{x: 3}",
            "A.x",
            "i32",
            Scalar::Integer(3),
        ),
    ] {
        rows.push(
            Row::new(
                format!("DECL-02.{id}"),
                &["DECL-02"],
                format!("{declaration}\nconst Result = {expression}"),
            )
            .check(Check::HirConstant {
                package: ".",
                name: "Result",
                ty,
                value,
            }),
        );
    }
    rows
}

fn zero_storage(project: &mir::Project) -> Result<(), String> {
    let package = project
        .packages
        .iter()
        .find(|p| p.logical_path == ".")
        .ok_or("missing package")?;
    let f = package
        .functions
        .iter()
        .find(|f| f.name == "main.test")
        .ok_or("missing function")?;
    let x = f
        .locals
        .iter()
        .find(|l| l.name.as_deref() == Some("x"))
        .ok_or("missing local")?;
    if f.returns != [x.ty] || package.globals.len() != 1 || package.globals[0].ty != x.ty {
        return Err("local/global/return type mismatch".into());
    }
    let instructions: Vec<_> = f.blocks.iter().flat_map(|b| &b.instructions).collect();
    let zero = instructions
        .iter()
        .find(|i| matches!(&i.kind, mir::InstructionKind::Constant(c) if is_zero(project, x.ty, c)))
        .ok_or("missing typed zero")?;
    let stored = zero.results.len() == 1
        && instructions.iter().any(|i| {
            let mir::InstructionKind::Store { place, value, .. } = &i.kind else {
                return false;
            };
            if *value != zero.results[0] {
                return false;
            }
            let is_x =
                |p: &mir::Place| p.base == mir::PlaceBase::Local(x.id) && p.projections.is_empty();
            is_x(place)
                || instructions.iter().any(|i| {
                    matches!(&i.kind,
            mir::InstructionKind::AggregateCopy { destination, source, .. }
            if is_x(destination) && source == place)
                })
        });
    if !stored {
        return Err("typed zero is not stored to the local".into());
    }
    if package.initializer.is_some() {
        return Err("zero-only global unexpectedly has an initializer".into());
    }
    Ok(())
}

fn is_zero(project: &mir::Project, ty: mir::TypeId, value: &mir::Constant) -> bool {
    use mir::{Constant as C, TypeKind as T};
    if value.ty() != ty {
        return false;
    }
    match project.types.underlying_kind(ty) {
        Some(T::U8 | T::I8 | T::U16 | T::I16 | T::U32 | T::I32) => {
            matches!(value, C::Integer { bits: 0, .. })
        }
        Some(T::F32) => matches!(value, C::Float32 { bits: 0, .. }),
        Some(T::Bool) => matches!(value, C::Bool { value: false, .. }),
        Some(T::Pointer(_) | T::Function(_)) => matches!(value, C::Nil(_)),
        Some(T::Array { .. } | T::Struct { .. }) => matches!(value, C::Zero(_)),
        _ => false,
    }
}
