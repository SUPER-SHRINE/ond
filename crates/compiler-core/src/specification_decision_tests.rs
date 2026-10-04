//! Regression cases for conformance checklist decisions Q-01 through Q-04.
use crate::{memory_test_vm::execute, test_support::compile};

#[test]
fn builtin_type_names_cannot_introduce_bindings() {
    for name in ["bool", "u8", "i8", "u16", "i16", "u32", "i32", "f32"] {
        for declaration in [
            format!("type {name} u8"),
            format!("var {name} = 1"),
            format!("const {name} = 1"),
            format!("func {name}() {{}}"),
            format!("func f({name}: u8) {{}}"),
            format!("func f() {{ var {name} = 1; }}"),
            format!("func f() {{ const {name} = 1; }}"),
            format!("func f() {{ {name} := 1; }}"),
            format!("func f() {{ ok, {name} := 1, 2; }}"),
            format!("var (ok = 0; {name} = 1)"),
        ] {
            let source = format!("package main\n{declaration}\nfunc main() {{}}\n");
            let error = compile(&source, None).unwrap_err();
            let diagnostic = error
                .diagnostics()
                .iter()
                .find(|d| d.message.contains("builtin type name"))
                .unwrap_or_else(|| panic!("{source}: {error:?}"));
            let span = diagnostic.primary.span;
            assert_eq!(&source[span.start..span.end], name);
        }
    }
}

#[test]
fn builtin_types_remain_usable_and_member_names_are_not_reserved() {
    compile("package main\ntype str [2]u8\ntype S struct { i32: i32; len: i32; str: str; }\nfunc identity(x: i32) -> i32 { return x; }\nfunc main() { var a: S; a.i32 = identity(2); a.len = 3; var f: func(i32: i32); var str = 2; _ = str; _ = f; }\n", None).unwrap();
}

#[test]
fn empty_result_list_is_syntax_error_in_declarations_and_types() {
    for declaration in [
        "func f() -> () {}",
        "type F func() -> ()",
        "var f: func(i32) -> ()",
        "func f(cb: func() -> ()) {}",
    ] {
        let source = format!("package main\n{declaration}\nfunc main() {{}}\n");
        let error = crate::parse_source_file(crate::source::FileId(0), &source).unwrap_err();
        assert!(
            format!("{error:?}").contains("empty result list"),
            "{source}: {error:?}"
        );
    }
    compile("package main\nfunc f() {}\nfunc g() -> (i32,) { return 1; }\nfunc main() { var cb: func() = f; cb(); }\n", None).unwrap();
}

#[test]
fn if_parentheses_are_ordinary_expression_grouping() {
    let result = compile("package main\nfunc main() {}\nfunc result() -> i32 { var x = 1; if ((x == 1)) { return 7; } else { return 0; } }\n", None).unwrap();
    assert_eq!(execute(&result.mir, "main.result", &[]).unwrap(), vec![7]);
}

#[test]
fn constants_distinguish_eligibility_typechecking_and_evaluation() {
    let result = compile("package main\nconst A = false && (1 / 0 == 0)\nconst B = true || (1 / 0 == 0)\nconst D = len(\"音\")\nfunc array(x: i32) -> [2]u8 { return [2]u8{}; }\nfunc main() {}\nfunc result() -> (bool, bool, u32) { return A, B, D; }\n", None).unwrap();
    assert_eq!(
        execute(&result.mir, "main.result", &[]).unwrap(),
        vec![0, 1, 3]
    );
    for expression in [
        "false && f()",
        "true || f()",
        "false && 1",
        "len(array(true))",
        "(256 as u8)",
    ] {
        let source = format!(
            "package main\nconst X = {expression}\nfunc f() -> bool {{ return true; }}\nfunc array(x: i32) -> [2]u8 {{ return [2]u8{{}}; }}\nfunc main() {{}}\n"
        );
        let error = compile(&source, None).unwrap_err();
        assert!(!error.diagnostics()[0].primary.span.is_synthetic());
    }
}
