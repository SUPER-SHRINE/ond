use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use super::compile_project;

#[test]
fn import_aliases_disambiguate_packages_with_the_same_declared_name() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("ond-import-alias-{}-{unique}", std::process::id()));
    fs::create_dir_all(root.join("my/json")).unwrap();
    fs::create_dir_all(root.join("json")).unwrap();
    fs::write(root.join("ond.toml"), "").unwrap();
    fs::write(
        root.join("main.ond"),
        "package main\nimport std_json \"json\"\nimport \"my/json\"\nfunc main() { var _ = std_json.Value + json.Value }\n",
    )
    .unwrap();
    fs::write(
        root.join("json/json.ond"),
        "package json\nconst Value = 1\n",
    )
    .unwrap();
    fs::write(
        root.join("my/json/json.ond"),
        "package json\nconst Value = 2\n",
    )
    .unwrap();

    let compilation = compile_project(&root).unwrap();
    let main = compilation
        .hir
        .packages
        .iter()
        .find(|package| package.logical_path == ".")
        .unwrap();
    let imports = &main.import_names[&main.files[0].file_id];
    assert_ne!(imports["std_json"], imports["json"]);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn compiles_source_through_target_neutral_mir() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("ond-compiler-core-{}-{unique}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("ond.toml"), "[target.unrelated]\nvalue = 1\n").unwrap();
    fs::write(root.join("main.ond"), "package main\n\nfunc main() {}\n").unwrap();

    let compilation = compile_project(&root).unwrap();
    crate::mir::validate_project(&compilation.mir).unwrap();
    let main = &compilation.mir.packages[0].functions[0];
    assert_eq!(main.name, "main.main");
    assert_eq!(main.blocks.len(), 1);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn resolves_function_signature_without_target_layout() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("ond-typed-hir-{}-{unique}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("ond.toml"), "").unwrap();
    fs::write(
        root.join("main.ond"),
        "package main\n\nfunc identity(value: *u8) -> *u8 {\n    return value\n}\n\nfunc main() {}\n",
    )
    .unwrap();

    let compilation = compile_project(&root).unwrap();
    let package = &compilation.hir.packages[0];
    let function = package
        .items
        .iter()
        .find_map(|item| match item {
            crate::ir::hir::Item::Func(function) if function.name == "identity" => Some(function),
            _ => None,
        })
        .unwrap();
    let pointer = function.signature.parameters[0].ty;
    assert!(matches!(
        compilation.hir.types.kind(pointer),
        Some(crate::semantic::TypeKind::Pointer(pointee))
            if *pointee == crate::semantic::TypeId::U8
    ));
    assert_eq!(function.signature.returns, vec![pointer]);

    let mir_function = compilation.mir.packages[0]
        .functions
        .iter()
        .find(|function| function.name == "main.identity")
        .unwrap();
    assert_eq!(mir_function.locals[0].ty, pointer);
    assert_eq!(mir_function.returns, vec![pointer]);
    assert!(matches!(
        mir_function.blocks[0].instructions.as_slice(),
        [crate::mir::Instruction {
            kind: crate::mir::InstructionKind::Load { .. },
            ..
        }]
    ));
    assert!(matches!(
        mir_function.blocks[0].terminator.kind,
        crate::mir::TerminatorKind::Return(ref values) if values == &[crate::mir::ValueId(0)]
    ));
    crate::mir::validate_function(&compilation.mir.types, mir_function).unwrap();

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn lowers_integer_division_checks_before_the_operation() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("ond-mir-division-{}-{unique}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("ond.toml"), "").unwrap();
    fs::write(
        root.join("main.ond"),
        "package main\n\nfunc quotient(left: i32, right: i32) -> i32 {\n    return left / right\n}\n\nfunc main() {}\n",
    )
    .unwrap();

    let compilation = compile_project(&root).unwrap();
    let function = compilation.mir.packages[0]
        .functions
        .iter()
        .find(|function| function.name == "main.quotient")
        .unwrap();
    let instructions = &function.blocks[0].instructions;
    assert!(matches!(
        instructions[2].kind,
        crate::mir::InstructionKind::Check(crate::mir::Check::NonZero { .. })
    ));
    assert!(matches!(
        instructions[3].kind,
        crate::mir::InstructionKind::Check(crate::mir::Check::SignedDivisionOverflow { .. })
    ));
    assert!(matches!(
        instructions[4].kind,
        crate::mir::InstructionKind::Binary {
            op: crate::mir::BinaryOp::Divide,
            ..
        }
    ));
    crate::mir::validate_function(&compilation.mir.types, function).unwrap();

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn resolves_an_imported_defined_type_in_a_signature() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("ond-imported-type-{}-{unique}", std::process::id()));
    fs::create_dir_all(root.join("shared")).unwrap();
    fs::write(root.join("ond.toml"), "").unwrap();
    fs::write(
        root.join("main.ond"),
        "package main\n\nimport \"shared\"\n\nfunc consume(value: shared.Value) {}\n\nfunc main() {}\n",
    )
    .unwrap();
    fs::write(
        root.join("shared").join("shared.ond"),
        "package shared\n\ntype Value u32\n",
    )
    .unwrap();

    let compilation = compile_project(&root).unwrap();
    let function = compilation
        .hir
        .packages
        .iter()
        .find(|package| package.logical_path == ".")
        .unwrap()
        .items
        .iter()
        .find_map(|item| match item {
            crate::ir::hir::Item::Func(function) if function.name == "consume" => Some(function),
            _ => None,
        })
        .unwrap();
    let ty = function.signature.parameters[0].ty;
    let definition = compilation.hir.types.get(ty).unwrap();
    assert_eq!(definition.name.as_deref(), Some("shared.Value"));
    assert_eq!(
        definition.kind,
        crate::semantic::TypeKind::Defined {
            underlying: crate::semantic::TypeId::U32,
        }
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn types_local_declarations_and_assignments_before_mir_lowering() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("ond-typed-locals-{}-{unique}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("ond.toml"), "").unwrap();
    fs::write(
        root.join("main.ond"),
        "package main\n\nfunc calculate(input: u32) -> u32 {\n    var base: u32 = 10\n    result := base + input\n    result = result + 1\n    return result\n}\n\nfunc main() {}\n",
    )
    .unwrap();

    let compilation = compile_project(&root).unwrap();
    let hir_function = compilation.hir.packages[0]
        .items
        .iter()
        .find_map(|item| match item {
            crate::ir::hir::Item::Func(function) if function.name == "calculate" => Some(function),
            _ => None,
        })
        .unwrap();
    let crate::ir::hir::Body::Typed(body) = &hir_function.body;
    assert_eq!(body.locals.len(), 3);
    assert_eq!(body.locals[0].name, "input");
    assert_eq!(body.locals[1].name, "base");
    assert_eq!(body.locals[2].name, "result");
    assert_eq!(body.block.statements.len(), 4);

    let mir_function = compilation.mir.packages[0]
        .functions
        .iter()
        .find(|function| function.name == "main.calculate")
        .unwrap();
    assert_eq!(mir_function.locals.len(), 3);
    assert!(matches!(
        mir_function.blocks[0].terminator.kind,
        crate::mir::TerminatorKind::Return(_)
    ));
    crate::mir::validate_function(&compilation.mir.types, mir_function).unwrap();

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn represents_pointer_recursive_structs_with_type_ids() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ond-recursive-type-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("ond.toml"), "").unwrap();
    fs::write(
        root.join("main.ond"),
        "package main\n\ntype Node struct {\n    next: *Node\n    value: u32\n}\n\nfunc main() {}\n",
    )
    .unwrap();

    let compilation = compile_project(&root).unwrap();
    let node = compilation.hir.packages[0]
        .items
        .iter()
        .find_map(|item| match item {
            crate::ir::hir::Item::Type(item) if item.name == "Node" => Some(item.ty),
            _ => None,
        })
        .unwrap();
    let underlying = compilation.hir.types.underlying_id(node).unwrap();
    let crate::semantic::TypeKind::Struct(structure) =
        compilation.hir.types.kind(underlying).unwrap()
    else {
        panic!("Node should resolve to a struct type");
    };
    let next = structure
        .fields
        .iter()
        .find(|field| field.name == "next")
        .unwrap();
    assert!(matches!(
        compilation.hir.types.kind(next.ty),
        Some(crate::semantic::TypeKind::Pointer(pointee)) if *pointee == node
    ));

    fs::remove_dir_all(root).unwrap();
}
