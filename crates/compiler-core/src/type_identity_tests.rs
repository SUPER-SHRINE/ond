//! Structural identity excludes source metadata but preserves nominal/package identity.
use crate::{
    project::PackageId,
    semantic::*,
    source::{FileId, Span},
    test_support::{compile, compile_files},
};
use std::hash::{Hash, Hasher};

#[test]
fn field_equality_and_hash_ignore_origin_but_not_owner() {
    let a = StructField {
        name: "x".into(),
        ty: TypeId::U8,
        private_owner: Some(PackageId(0)),
        span: Span::new(FileId(0), 1, 2),
    };
    let b = StructField {
        span: Span::new(FileId(1), 50, 60),
        ..a.clone()
    };
    let hash = |f: &StructField| {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        f.hash(&mut h);
        h.finish()
    };
    assert_eq!(a, b);
    assert_eq!(hash(&a), hash(&b));
    assert_ne!(
        a,
        StructField {
            private_owner: Some(PackageId(1)),
            ..b.clone()
        }
    );
    let mut types = TypeTable::new();
    let left = types.intern(
        TypeKind::Struct(StructType { fields: vec![a] }),
        Span::synthetic(),
    );
    let right = types.intern(
        TypeKind::Struct(StructType { fields: vec![b] }),
        Span::new(FileId(3), 0, 5),
    );
    assert_eq!(left, right);
    assert_eq!(
        types.pointer(left, Span::synthetic()),
        types.pointer(right, Span::synthetic())
    );
}

#[test]
fn anonymous_structs_match_across_files_and_function_signatures() {
    let result = compile_files(&[
        ("main.ond", "package main\nfunc main() {}\nfunc result() -> i32 { var a: struct { x: i32; }; a.x = 7; var b: struct { x: i32; }; b = echo(a); var f: func(v: struct { x: i32; }) -> struct { x: i32; } = echo; b = f(b); return b.x; }\n"),
        ("other.ond", "package main\nfunc echo(v: struct { x: i32; }) -> struct { x: i32; } { return v; }\n"),
    ]).unwrap();
    crate::mir::validate_project(&result.mir).unwrap();
}

#[test]
fn nested_arrays_pointers_and_function_parameter_names_are_structural() {
    compile("package main\nfunc main() { var a: [2]*struct { x: [3]u8; }; var b: [2]*struct { x: [3]u8; }; b = a; var f: func(first: struct { x: i32; }) -> [2]u8; var g: func(second: struct { x: i32; }) -> [2]u8; g = f; }\n", None).unwrap();
}

#[test]
fn defined_structs_stay_distinct_but_matching_underlying_casts_work() {
    let result = compile("package main\ntype A struct { x: i32; }\ntype B struct { x: i32; }\nfunc main() {}\nfunc result() -> i32 { var a = A{x: 7}; var b = a as B; return b.x; }\n", None).unwrap();
    assert_eq!(
        crate::memory_test_vm::execute(&result.mir, "main.result", &[]).unwrap(),
        vec![7]
    );
    crate::test_support::reject(
        "package main\ntype A struct { x: i32; }\ntype B struct { x: i32; }\nfunc main() { var a: A; var b: B = a; }\n",
        None,
        "type mismatch",
        "a",
    );
}

#[test]
fn public_fields_match_across_packages_and_private_fields_do_not() {
    let shared = "package shared\nfunc Public() -> struct { X: i32; } { var v: struct { X: i32; }; v.X = 7; return v; }\nfunc Private() -> struct { x: i32; } { var v: struct { x: i32; }; return v; }\n";
    let result = compile("package main\nimport \"shared\"\nfunc main() {}\nfunc result() -> i32 { var v: struct { X: i32; } = shared.Public(); return v.X; }\n", Some(shared)).unwrap();
    assert_eq!(
        crate::memory_test_vm::execute(&result.mir, "main.result", &[]).unwrap(),
        vec![7]
    );
    for (body, message, span) in [
        (
            "var v: struct { x: i32; } = shared.Private()",
            "declaration value 1 type mismatch",
            "v: struct { x: i32; } = shared.Private()",
        ),
        (
            "var v = shared.Private(); var x = v.x",
            "field `x` is not exported",
            "v.x",
        ),
        (
            "var v = shared.Private(); var x = v as struct { x: i32; }",
            "invalid explicit type conversion",
            "v",
        ),
    ] {
        crate::test_support::reject(
            &format!("package main\nimport \"shared\"\nfunc main() {{ {body}; }}\n"),
            Some(shared),
            message,
            span,
        );
    }
}

#[test]
fn field_order_names_and_nominal_element_types_remain_significant() {
    for (left, right) in [
        ("struct { x: i32; y: u8; }", "struct { y: u8; x: i32; }"),
        ("struct { x: i32; }", "struct { y: i32; }"),
        ("struct { x: i32; }", "struct { x: u32; }"),
        ("[2]u8", "[3]u8"),
        ("*A", "*B"),
        ("[2]A", "[2]B"),
        ("struct { x: A; }", "struct { x: B; }"),
        ("func(x: A)", "func(x: B)"),
    ] {
        let source = format!(
            "package main\ntype A i32\ntype B i32\nfunc main() {{ var a: {left}; var b: {right} = a; }}\n"
        );
        crate::test_support::reject(&source, None, "type mismatch", "a");
    }
}

#[test]
fn validator_rejects_missing_and_unknown_private_owners() {
    let original = compile(
        "package main\nfunc main() { var v: struct { x: i32; }; }\n",
        None,
    )
    .unwrap()
    .mir;
    for owner in [None, Some(PackageId(usize::MAX))] {
        let mut project = original.clone();
        let definition = project
            .types
            .definitions()
            .iter()
            .find(|d| matches!(d.kind, TypeKind::Struct(_)))
            .unwrap()
            .clone();
        let TypeKind::Struct(mut structure) = definition.kind else {
            unreachable!()
        };
        structure.fields[0].private_owner = owner;
        project
            .types
            .define(definition.id, TypeKind::Struct(structure));
        assert!(
            crate::mir::validate_project(&project)
                .unwrap_err()
                .iter()
                .any(|error| error.message.contains("invalid private owner"))
        );
    }
}
