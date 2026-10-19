//! Outer component framing is required even when nested type nodes are framed.
use super::{canonical_generic_method_symbol, identity, operators};
use crate::{
    semantic::{TypeId, TypeTable},
    source::Span,
};

fn named(types: &mut TypeTable, name: &str) -> TypeId {
    types.reserve_defined(name.into(), Span::synthetic())
}

#[test]
fn generic_declaration_cannot_absorb_argument_encoding() {
    let mut types = TypeTable::new();
    let first = named(&mut types, "q/r.T[1:n5:a/r.Z");
    let second = named(&mut types, "a/r.Z");
    assert_eq!(
        format!("p.G[{}]", identity::arguments(&[first], &types, &[])),
        format!(
            "p.G[1:n16:q/r.T[{}]",
            identity::arguments(&[second], &types, &[])
        ),
    );
    assert_ne!(
        identity::generic_name("p.G", &[first], &types, &[]),
        identity::generic_name("p.G[1:n16:q/r.T", &[second], &types, &[]),
    );
}

#[test]
fn generic_method_owner_cannot_absorb_argument_encoding() {
    let mut types = TypeTable::new();
    let first = named(&mut types, "q/r.\u{1f}generic_method.T.M.1:n5:a/r.Z");
    let second = named(&mut types, "a/r.Z");
    let second_owner = "p.\u{1f}generic_method.G.M.1:n34:q/r";
    assert_eq!(
        format!(
            "p.\u{1f}generic_method.G.M.{}",
            identity::arguments(&[first], &types, &[])
        ),
        format!(
            "{second_owner}.\u{1f}generic_method.T.M.{}",
            identity::arguments(&[second], &types, &[])
        ),
    );
    assert_ne!(
        canonical_generic_method_symbol("p", "G", "M", &[first], &types, &[]),
        canonical_generic_method_symbol(second_owner, "T", "M", &[second], &types, &[]),
    );
}

#[test]
fn operator_owner_cannot_absorb_parameter_encoding() {
    let mut types = TypeTable::new();
    let first_receiver = named(&mut types, "p.A");
    let first_argument = named(
        &mut types,
        "q/r.\u{1f}operator.add.2:n33:p.\u{1f}operator.add.2:n3:p.An65:q/r.Bn5:a/r.Z",
    );
    let second_owner = "p.\u{1f}operator.add.2:n3:p.An65:q/r";
    let second_receiver = named(&mut types, &format!("{second_owner}.B"));
    let second_argument = named(&mut types, "a/r.Z");
    assert_eq!(
        format!(
            "p.\u{1f}operator.add.{}",
            identity::arguments(&[first_receiver, first_argument], &types, &[])
        ),
        format!(
            "{second_owner}.\u{1f}operator.add.{}",
            identity::arguments(&[second_receiver, second_argument], &types, &[])
        ),
    );
    assert_ne!(
        operators::canonical_operator_symbol(
            "p",
            crate::ir::ast::OperatorName::Add,
            &[first_receiver, first_argument],
            &types,
            &[]
        ),
        operators::canonical_operator_symbol(
            second_owner,
            crate::ir::ast::OperatorName::Add,
            &[second_receiver, second_argument],
            &types,
            &[]
        ),
    );
}

#[test]
fn generic_operator_owner_cannot_absorb_parameter_encoding() {
    let mut types = TypeTable::new();
    let first_receiver = types.reserve_generic_defined(
        "first".into(),
        "p.A".into(),
        vec![TypeId::U32],
        Span::synthetic(),
    );
    let first_argument = named(
        &mut types,
        "q/r.\u{1f}generic_operator.add.2:g44:p.\u{1f}generic_operator.add.2:g3:p.A1:un87:q/r.B1:un5:a/r.Z",
    );
    let second_owner = "p.\u{1f}generic_operator.add.2:g3:p.A1:un87:q/r";
    let second_receiver = types.reserve_generic_defined(
        "second".into(),
        format!("{second_owner}.B"),
        vec![TypeId::U32],
        Span::synthetic(),
    );
    let second_argument = named(&mut types, "a/r.Z");
    assert_eq!(
        format!(
            "p.\u{1f}generic_operator.add.{}.1:u",
            identity::arguments(&[first_receiver, first_argument], &types, &[])
        ),
        format!(
            "{second_owner}.\u{1f}generic_operator.add.{}.1:u",
            identity::arguments(&[second_receiver, second_argument], &types, &[])
        ),
    );
    assert_ne!(
        operators::canonical_generic_operator_symbol(
            "p",
            crate::ir::ast::OperatorName::Add,
            &[first_receiver, first_argument],
            &[TypeId::U32],
            &types,
            &[]
        ),
        operators::canonical_generic_operator_symbol(
            second_owner,
            crate::ir::ast::OperatorName::Add,
            &[second_receiver, second_argument],
            &[TypeId::U32],
            &types,
            &[]
        ),
    );
}

// These package paths are accepted by physical Unix projects. Virtual source
// names reject colons, and Windows cannot create these directories.
// These semantic identity fixtures set explicit logical names independently of
// the project loader's namespace, preserving the exact adversarial encodings.
#[cfg(unix)]
fn compile_with_package_names(
    files: &[(&str, &str)],
) -> Result<crate::Compilation, crate::FrontendError> {
    use std::fs;

    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "ond-identity-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("ond.toml"), "[project]\nname = \"test\"\n").unwrap();
    for (name, source) in files {
        let path = root.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, source).unwrap();
    }
    let result = crate::load_project(&root).and_then(|mut loaded| {
        for package in &mut loaded.packages {
            if package.logical_path != "." {
                package.logical_path = package
                    .logical_path
                    .strip_prefix("test/")
                    .expect("fixture subpackage must have the test project prefix")
                    .to_owned();
            }
        }
        crate::compile_loaded_project(loaded)
    });
    fs::remove_dir_all(root).unwrap();
    result
}

#[cfg(unix)]
#[test]
fn colliding_generic_source_names_remain_distinct_nominal_types() {
    let error = compile_with_package_names(&[
        ("main.ond", "package main\nimport p \"p\"\nimport n1 \"q/r.T[1:n5:a/r\"\nimport n2 \"a/r\"\nimport d2 \"p.G[1:n16:q/r\"\nfunc main(){var x:p.G[n1.Z];var y:d2.T[n2.Z];x=y;_=x}"),
        ("p/p.ond", "package p\ntype G[T] struct { A:T }"),
        ("p.G[1:n16:q/r/r.ond", "package r\ntype T[U] struct { B:U }"),
        ("q/r.T[1:n5:a/r/r.ond", "package r\ntype Z u32"),
        ("a/r/r.ond", "package r\ntype Z u32"),
    ]).expect_err("different generic declarations must reject assignment");
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|d| d.message.contains("type mismatch")),
        "{error:?}"
    );
}

#[cfg(unix)]
#[test]
fn delimiter_bearing_owners_emit_distinct_callable_symbols() {
    for (owner, argument, first, second, expression, marker) in [
        (
            "p.\u{1f}operator.add.2:n3:p.An65:q/r",
            "q/r.\u{1f}operator.add.2:n33:p.\u{1f}operator.add.2:n3:p.An65:q/r.Bn5:a/r",
            "type A struct {}\noperator (x:A) + (y:n1.Z)->u32{return 1}",
            "type B struct {}\noperator (x:B) + (y:n2.Z)->u32{return 2}",
            "p.A{}+n1.Z{},d2.B{}+n2.Z{}",
            ".\u{1f}operator.add.",
        ),
        (
            "p.\u{1f}generic_method.G.M.1:n34:q/r",
            "q/r.\u{1f}generic_method.T.M.1:n5:a/r",
            "type G[T] struct {}\nfunc (x:G[T]) M()->u32{return 1}",
            "type T[U] struct {}\nfunc (x:T[U]) M()->u32{return 2}",
            "p.G[n1.Z]{}.M(),d2.T[n2.Z]{}.M()",
            ".\u{1f}generic_method.",
        ),
        (
            "p.\u{1f}generic_operator.add.2:g3:p.A1:un87:q/r",
            "q/r.\u{1f}generic_operator.add.2:g44:p.\u{1f}generic_operator.add.2:g3:p.A1:un87:q/r.B1:un5:a/r",
            "type A[T] struct {}\noperator (x:A[T]) + (y:n1.Z)->u32{return 1}",
            "type B[T] struct {}\noperator (x:B[T]) + (y:n2.Z)->u32{return 2}",
            "p.A[u32]{}+n1.Z{},d2.B[u32]{}+n2.Z{}",
            ".\u{1f}generic_operator.add.",
        ),
    ] {
        let escaped_owner = owner.replace('\u{1f}', "\\u001f");
        let escaped_argument = argument.replace('\u{1f}', "\\u001f");
        let imports = format!(
            "import p \"p\"\nimport n1 \"{escaped_argument}\"\nimport n2 \"a/r\"\nimport d2 \"{escaped_owner}\"\n"
        );
        let main = format!(
            "package main\n{imports}func observe()->(u32,u32){{return {expression}}}\nfunc main(){{}}"
        );
        let first_source = format!("package p\nimport n1 \"{escaped_argument}\"\n{first}");
        let second_source = format!("package r\nimport n2 \"a/r\"\n{second}");
        let compilation = compile_with_package_names(&[
            ("main.ond", &main),
            ("p/p.ond", &first_source),
            (&format!("{owner}/r.ond"), &second_source),
            (&format!("{argument}/r.ond"), "package r\ntype Z struct {}"),
            ("a/r/r.ond", "package r\ntype Z struct {}"),
        ])
        .unwrap();
        let symbols = compilation
            .hir
            .packages
            .iter()
            .flat_map(|p| &p.items)
            .filter_map(|item| match item {
                crate::ir::hir::Item::Func(f) if f.canonical_name.contains(marker) => {
                    Some(&f.canonical_name)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(symbols.len(), 2);
        assert_ne!(symbols[0], symbols[1]);
        assert_eq!(
            crate::memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
            [1, 2]
        );
    }
}

#[test]
fn generated_callable_symbols_do_not_alias_ordinary_function_namespace() {
    let mut types = TypeTable::new();
    let receiver = named(&mut types, "p.A");
    let argument = named(&mut types, "q/r.B");
    let generic_receiver = types.reserve_generic_defined(
        "generic".into(),
        "p.G".into(),
        vec![argument],
        Span::synthetic(),
    );
    for (generated, ordinary_function) in [
        (
            operators::canonical_operator_symbol(
                "p",
                crate::ir::ast::OperatorName::Add,
                &[receiver, argument],
                &types,
                &[],
            ),
            "1:p.\u{1f}operator.add.2:n3:p.An5:q/r.B",
        ),
        (
            canonical_generic_method_symbol("p", "G", "M", &[argument], &types, &[]),
            "1:p.\u{1f}generic_method.G.M.1:n5:q/r.B",
        ),
        (
            operators::canonical_generic_operator_symbol(
                "p",
                crate::ir::ast::OperatorName::Add,
                &[generic_receiver, argument],
                &[argument],
                &types,
                &[],
            ),
            "1:p.\u{1f}generic_operator.add.2:g3:p.G1:n5:q/r.Bn5:q/r.B.1:n5:q/r.B",
        ),
    ] {
        assert_eq!(generated.strip_suffix(';').unwrap(), ordinary_function);
        assert_ne!(generated, ordinary_function);
    }
}

#[cfg(unix)]
#[test]
fn generated_callables_and_delimiter_bearing_ordinary_functions_execute_separately() {
    for (declaration, expression, other_owner) in [
        (
            "type A struct {}\noperator (x:A)+(y:r.B)->u32{return 1}",
            "p.A{}+r.B{}",
            "1:p.\u{1f}operator.add.2:n3:p.An5:q/r",
        ),
        (
            "type G[T] struct {}\nfunc (x:G[T]) M()->u32{return 1}",
            "p.G[r.B]{}.M()",
            "1:p.\u{1f}generic_method.G.M.1:n5:q/r",
        ),
        (
            "type G[T] struct {}\noperator (x:G[T])+(y:r.B)->u32{return 1}",
            "p.G[r.B]{}+r.B{}",
            "1:p.\u{1f}generic_operator.add.2:g3:p.G1:n5:q/r.Bn5:q/r.B.1:n5:q/r",
        ),
    ] {
        let escaped_owner = other_owner.replace('\u{1f}', "\\u001f");
        let main = format!(
            "package main\nimport \"p\"\nimport r \"q/r\"\nimport other \"{escaped_owner}\"\nfunc observe()->(u32,u32){{return {expression},other.B()}}\nfunc main(){{}}"
        );
        let owner_source = format!("package p\nimport r \"q/r\"\n{declaration}");
        let compilation = compile_with_package_names(&[
            ("main.ond", &main),
            ("p/p.ond", &owner_source),
            ("q/r/r.ond", "package r\ntype B struct {}"),
            (
                &format!("{other_owner}/r.ond"),
                "package r\nfunc B()->u32{return 2}",
            ),
        ])
        .unwrap();
        let symbols = compilation
            .hir
            .packages
            .iter()
            .flat_map(|p| &p.items)
            .filter_map(|item| match item {
                crate::ir::hir::Item::Func(f) => Some(&f.canonical_name),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            symbols
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            symbols.len()
        );
        assert_eq!(
            crate::memory_test_vm::execute(&compilation.mir, "main.observe", &[]).unwrap(),
            [1, 2]
        );
    }
}
