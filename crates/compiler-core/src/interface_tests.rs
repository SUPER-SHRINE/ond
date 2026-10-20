use crate::test_support::compile;

#[test]
fn removed_interface_syntax_is_rejected() {
    for declaration in [
        "type Reader interface { Read() -> u32 }",
        "type Any interface {}",
        "type List[T] interface { Get() -> T }",
    ] {
        let source = format!("package main\n{declaration}\nfunc main() {{}}\n");
        let error = compile(&source, None).expect_err("interface declarations were removed");
        assert!(
            error
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.message.contains("expected type")),
            "{error}"
        );
    }
}
