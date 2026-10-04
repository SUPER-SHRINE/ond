use crate::{
    memory_test_vm::execute,
    mir,
    test_support::{compile, compile_files},
};

fn checked(source: &str) -> crate::Compilation {
    let compilation = compile(source, None).unwrap();
    mir::validate_project(&compilation.mir).unwrap();
    compilation
}

#[test]
fn globals_are_zeroed_or_initialized_from_compile_time_values() {
    let compilation = checked(
        r#"package main
type Pair struct { first: i32; second: i32; }
var Zero: i32
var Number = 9
var Values = Pair{first: 4, second: 5}
var Text = "abc"
func main() {}
func result() -> (i32, i32, i32, i32, u32) {
    return Zero, Number, Values.first, Values.second, len(Text)
}
"#,
    );
    assert_eq!(
        execute(&compilation.mir, "main.result", &[]).unwrap(),
        [0, 9, 4, 5, 3]
    );
    assert!(compilation.mir.initialization_order.is_empty());
    assert!(
        compilation
            .mir
            .packages
            .iter()
            .all(|package| package.initializer.is_none())
    );
}

#[test]
fn runtime_global_initializers_are_rejected() {
    for declaration in [
        "var Value = make()",
        "var A = 1\nvar B = A",
        "var Value = new(i32)",
        "var A, B = pair()",
    ] {
        let source = format!(
            "package main\n{declaration}\nfunc make() -> i32 {{ return 1; }}\nfunc pair() -> (i32, i32) {{ return 1, 2; }}\nfunc main() {{}}\n"
        );
        let error = compile(&source, None).unwrap_err();
        assert!(
            error.diagnostics().iter().any(|diagnostic| diagnostic
                .message
                .contains("compile-time value")
                || diagnostic.message.contains("value count mismatch")),
            "{declaration}: {error:?}"
        );
    }
}

#[test]
fn imported_globals_and_lexical_shadowing_work() {
    let compilation = compile(
        "package main\nimport \"shared\"\nvar Value = 1\nfunc main() {}\nfunc result() -> (i32, i32) {\nvar Value = 9\nshared.Value = Value\nreturn Value, shared.Value\n}\n",
        Some("package shared\nvar Value = 2\n"),
    )
    .unwrap();
    mir::validate_project(&compilation.mir).unwrap();
    assert_eq!(
        execute(&compilation.mir, "main.result", &[]).unwrap(),
        [9, 9]
    );
}

#[test]
fn global_type_errors_are_diagnosed() {
    for (declaration, expected) in [
        ("var A: i32 = true", "bool"),
        ("var A = Missing", "unknown identifier"),
        ("var A, B = 1", "value count mismatch"),
    ] {
        let source = format!("package main\n{declaration}\nfunc main() {{}}\n");
        let error = compile(&source, None).unwrap_err();
        assert!(
            error.to_string().contains(expected),
            "{declaration}: {error}"
        );
        assert!(!error.diagnostics()[0].primary.span.is_synthetic());
    }
    assert!(compile("package main\nvar A: i32\nfunc main() {}\n", None).is_ok());
}

#[test]
fn validator_rejects_automatic_initialization_metadata_and_bad_imports() {
    let compilation = checked(
        "package main\nvar Value = 1\nfunc main() {}\nfunc get() -> i32 { return Value; }\n",
    );

    let mut bad = compilation.mir.clone();
    bad.initialization_order.push(bad.packages[0].id);
    assert!(
        mir::validate_project(&bad)
            .unwrap_err()
            .iter()
            .any(|error| error.message.contains("automatic package initialization"))
    );

    let mut bad = compilation.mir.clone();
    bad.packages[0].imports.push(crate::project::PackageId(999));
    assert!(
        mir::validate_project(&bad)
            .unwrap_err()
            .iter()
            .any(|error| error.message.contains("unknown imported package"))
    );

    let mut bad = compilation.mir.clone();
    bad.packages[0].globals.clear();
    assert!(
        mir::validate_project(&bad)
            .unwrap_err()
            .iter()
            .any(|error| error.message.contains("static type"))
    );
}

#[test]
fn disconnected_packages_need_no_startup_order() {
    let compilation = compile_files(&[
        (
            "main.ond",
            "package main\nimport \"used\"\nfunc main() {}\n",
        ),
        ("used/a.ond", "package used\nvar Value = 1\n"),
        ("unused/a.ond", "package unused\nvar Value = 2\n"),
    ])
    .unwrap();
    assert!(compilation.mir.initialization_order.is_empty());
    mir::validate_project(&compilation.mir).unwrap();
}
