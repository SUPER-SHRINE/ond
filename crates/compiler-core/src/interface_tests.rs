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
            error.diagnostics().iter().any(|diagnostic| {
                diagnostic.severity == crate::diagnostic::Severity::Error
                    && !diagnostic.primary.span.is_synthetic()
            }),
            "{error}"
        );
    }
}

#[test]
fn interface_is_an_identifier() {
    compile(
        "package main\nfunc interface() {}\nfunc main() { interface() }\n",
        None,
    )
    .unwrap();
}
