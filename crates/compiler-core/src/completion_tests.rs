//! Successful compilation is complete and validated; malformed IR never passes.
use crate::{
    mir::*,
    semantic::Intrinsic,
    source::Span,
    test_support::{compile, compile_files},
};

fn project(source: &str) -> Project {
    compile(source, None).unwrap().mir
}

#[test]
fn loaded_snapshot_does_not_require_a_physical_main_file() {
    use crate::{LoadedProject, PackageId, PackageSources, source::SourceDb};
    let root = std::path::PathBuf::from("<in-memory-ond-test>");
    let mut sources = SourceDb::default();
    let file = sources.add_file(
        root.join("main.ond"),
        "package main\nfunc main() {}\n".into(),
    );
    let loaded = LoadedProject {
        root: root.clone(),
        manifest_path: root.join("ond.toml"),
        ignored_paths: Vec::new(),
        sources,
        packages: vec![PackageSources {
            id: PackageId(0),
            logical_path: ".".into(),
            directory: root,
            files: vec![file],
        }],
    };
    assert!(crate::compile_loaded_project(loaded.clone()).is_ok());
    let mut missing = loaded;
    missing.packages[0].files.clear();
    assert!(
        crate::compile_loaded_project(missing)
            .unwrap_err()
            .to_string()
            .contains("project root must contain `main.ond`")
    );
}
fn rejects(project: &Project, expected: &str) {
    let errors = validate_project(project).expect_err("malformed MIR accepted");
    assert!(
        errors.iter().any(|e| e.message.contains(expected)),
        "{errors:?}"
    );
}
fn instructions(f: &mut Function) -> impl Iterator<Item = &mut Instruction> {
    f.blocks.iter_mut().flat_map(|b| &mut b.instructions)
}

#[test]
fn memory_builtins_have_explicit_typed_intrinsics() {
    let p = project(
        "package main\nfunc main() {\nvar p = new(u32)\nstore32(p, 7)\nvar x = load32(p)\nfree(p)\nvar bytes = alloc[u8](12)\nfree(bytes)\nfree(nil)\n}\n",
    );
    let ops = p.packages[0].functions[0]
        .blocks
        .iter()
        .flat_map(|b| &b.instructions)
        .filter_map(|i| match &i.kind {
            InstructionKind::Call {
                callee: Callee::Intrinsic(op),
                effects,
                ..
            } => {
                assert_eq!(*effects, CallEffects::Unknown);
                Some(*op)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        ops,
        [
            Intrinsic::New(TypeId::U32),
            Intrinsic::Store32,
            Intrinsic::Load32,
            Intrinsic::Free,
            Intrinsic::Alloc(TypeId::U8),
            Intrinsic::Free,
            Intrinsic::Free
        ]
    );
    validate_project(&p).unwrap();
}

#[test]
fn builtin_names_cannot_be_shadowed() {
    for name in [
        "alloc", "free", "len", "trap", "sizeof", "alignof", "load32", "store32", "thisFile",
        "thisLine", "new",
    ] {
        let source = format!("package main\nfunc {name}() {{}}\nfunc main() {{}}\n");
        let error = compile(&source, None).unwrap_err();
        assert!(
            error
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.message.contains("builtin name")
                    || (name == "new" && diagnostic.message.contains("expected identifier"))),
            "{name}: {error:?}"
        );
    }
}

#[test]
fn len_evaluates_its_operand_once_and_trap_is_a_nonreturning_terminator() {
    let p = project(
        "package main\nfunc main() {}\nfunc array() -> [2]u8 { return [2]u8{}; }\nfunc length() -> u32 { return len(array()); }\nfunc fail() -> i32 { defer {}\ntrap \"failed\"; }\n",
    );
    let length = p.packages[0]
        .functions
        .iter()
        .find(|function| function.name == "main.length")
        .unwrap();
    assert_eq!(
        length
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .filter(|instruction| matches!(instruction.kind, InstructionKind::Call { .. }))
            .count(),
        1
    );
    let fail = p.packages[0]
        .functions
        .iter()
        .find(|function| function.name == "main.fail")
        .unwrap();
    assert!(fail.blocks.iter().any(|block| matches!(
        block.terminator.kind,
        TerminatorKind::Trap(TrapKind::Explicit(ref message)) if message == "failed"
    )));
}

#[test]
fn trap_requires_a_literal_reason_and_decodes_quoted_and_raw_text() {
    for (literal, expected) in [
        (r#""out\nof range""#, "out\nof range"),
        ("`raw reason`", "raw reason"),
    ] {
        let p = project(&format!("package main\nfunc main() {{ trap {literal} }}\n"));
        assert!(
            p.packages[0].functions[0]
                .blocks
                .iter()
                .any(|block| matches!(
                    &block.terminator.kind,
                    TerminatorKind::Trap(TrapKind::Explicit(message)) if message == expected
                ))
        );
    }
    for source in [
        "package main\nfunc main() { trap() }\n",
        "package main\nfunc main() { var reason = \"failed\"; trap reason }\n",
        "package main\nfunc main() { trap \"\\xff\" }\n",
    ] {
        assert!(compile(source, None).is_err(), "accepted: {source}");
    }
}

#[test]
fn invalid_literals_and_intrinsics_are_source_errors() {
    for (body, message) in [
        ("var x: u8 = 256", "out of range"),
        ("var x: i8 = -129", "out of range"),
        ("var x = 2147483648", "out of range"),
        ("var x = 1e100", "out of range"),
        ("var x = (250 + 6) as u8", "out of range"),
        ("var x = (1.0 / 0.0) as i32", "conversion"),
        ("var x = alloc[u8](true)", "type"),
        ("free(1)", "pointer"),
        ("var x = load32(false)", "address"),
        ("var x = thisFile(1)", "thisFile expects 0 arguments"),
        ("var x = thisLine(1)", "thisLine expects 0 arguments"),
        ("store32(0)", "arguments"),
        ("var x = missing()", "unknown"),
    ] {
        let error =
            compile(&format!("package main\nfunc main() {{ {body}; }}\n"), None).unwrap_err();
        assert!(
            error
                .diagnostics()
                .iter()
                .any(|d| d.message.contains(message)),
            "{body}: {error:?}"
        );
        assert!(
            error
                .diagnostics()
                .iter()
                .all(|d| !d.message.contains("internal MIR")),
            "{error:?}"
        );
    }
    project("package main\nfunc main() {\nvar x: i32 = -2147483648\nvar y: u32 = 4294967295\n}\n");
    project("package main\nfunc main() {\nvar x = 1e2\nvar y = 1.5e-2\nvar z = 0xdead\n}\n");
}

#[test]
fn ambiguous_imports_are_rejected() {
    let error = compile_files(&[
        (
            "main.ond",
            "package main\nimport \"a/util\"\nimport \"b/util\"\nfunc main() {}\n",
        ),
        ("a/util/util.ond", "package util\n"),
        ("b/util/util.ond", "package util\n"),
    ])
    .unwrap_err();
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|d| d.message.contains("ambiguous")),
        "{error:?}"
    );
}

#[test]
fn validator_rejects_use_before_definition() {
    let mut p = project("package main\nfunc main() {}\nfunc f(x: i32) -> i32 { return x + 2; }\n");
    let f = &mut p.packages[0].functions[1];
    let block = &mut f.blocks[0];
    let at = block
        .instructions
        .iter()
        .position(|i| matches!(i.kind, InstructionKind::Binary { .. }))
        .unwrap();
    let binary = block.instructions.remove(at);
    block.instructions.insert(0, binary);
    rejects(&p, "without dominance");
}

#[test]
fn validator_rejects_non_dominating_branch_value() {
    let mut p = project(
        "package main\nfunc main() {}\nfunc f(b: bool) -> i32 {\nif b { return 11; } else { return 22; }\n}\n",
    );
    let f = &mut p.packages[0].functions[1];
    let from_then = f
        .blocks
        .iter()
        .find_map(|b| {
            b.instructions.iter().find_map(|i| match i.kind {
                InstructionKind::Constant(Constant::Integer { bits: 11, .. }) => Some(i.results[0]),
                _ => None,
            })
        })
        .unwrap();
    let else_block = f
        .blocks
        .iter_mut()
        .find(|b| {
            b.instructions.iter().any(|i| {
                matches!(
                    i.kind,
                    InstructionKind::Constant(Constant::Integer { bits: 22, .. })
                )
            })
        })
        .unwrap();
    else_block.terminator.kind = TerminatorKind::Return(vec![from_then]);
    rejects(&p, "without dominance");
}

#[test]
fn validator_rejects_wrong_operation_and_direct_target() {
    let mut p =
        project("package main\nfunc main() { f(1); }\nfunc f(x: i32) -> i32 { return x + 2; }\n");
    let mut bad = p.clone();
    for i in instructions(&mut bad.packages[0].functions[1]) {
        if let InstructionKind::Binary { op, .. } = &mut i.kind {
            *op = BinaryOp::FloatAdd;
        }
    }
    rejects(&bad, "operand/type/effect");
    for i in instructions(&mut p.packages[0].functions[0]) {
        if let InstructionKind::Call { callee, .. } = &mut i.kind {
            *callee = Callee::Direct("missing".into());
        }
    }
    rejects(&p, "unknown function");
}

#[test]
fn validator_rejects_intrinsic_signature_mismatch() {
    let mut p = project("package main\nfunc main() { var p = alloc[u8](4); }\n");
    for i in instructions(&mut p.packages[0].functions[0]) {
        if let InstructionKind::Call { callee, .. } = &mut i.kind {
            *callee = Callee::Intrinsic(Intrinsic::Free);
        }
    }
    rejects(&p, "operand/type/effect");
}

#[test]
fn validator_requires_division_and_bounds_guards() {
    for source in [
        "package main\nfunc main() {}\nfunc f(a: i32, b: i32) -> i32 { return a / b; }\n",
        "package main\nfunc main() {}\nfunc f(i: u32) -> u8 { var a: [4]u8\nreturn a[i]; }\n",
        "package main\nfunc main() {}\nfunc f(s: [4]u8, i: u32) -> u8 { return s[i]; }\n",
    ] {
        let mut p = project(source);
        for f in &mut p.packages[0].functions {
            for b in &mut f.blocks {
                b.instructions
                    .retain(|i| !matches!(i.kind, InstructionKind::Check(_)));
            }
            for (index, i) in instructions(f).enumerate() {
                i.id = InstructionId(index as u32);
            }
        }
        rejects(&p, "safety check");
    }
}

#[test]
fn validator_rejects_automatic_startup_order() {
    let mut p = compile(
        "package main\nimport \"shared\"\nfunc main() {}\n",
        Some("package shared\nvar V = 1\n"),
    )
    .unwrap()
    .mir;
    p.initialization_order.push(p.packages[0].id);
    rejects(&p, "automatic package initialization");
}

#[test]
fn validator_rejects_recursive_value_layout_but_allows_pointer_recursion() {
    let mut p = project("package main\nfunc main() {}\n");
    let id = p.types.reserve_defined("Cycle".into(), Span::synthetic());
    p.types.define(id, TypeKind::Defined { underlying: id });
    rejects(&p, "recursive value layout");
    let ptr = p.types.pointer(id, Span::synthetic());
    p.types.define(id, TypeKind::Defined { underlying: ptr });
    validate_project(&p).unwrap();
    p.types.define(TypeId::U32, TypeKind::I32);
    rejects(&p, "canonical builtin type");
}
