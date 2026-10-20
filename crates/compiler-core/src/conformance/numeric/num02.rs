use super::*;

fn row(id: &str) -> Row {
    let rows = rows();
    rows.into_iter().find(|row| row.id == id).unwrap()
}

fn rows() -> Vec<Row> {
    vec![
        Row::constant(
            "NUM-02.bitwise-project-overflow".into(),
            &["NUM-02"],
            "",
            "^(250 + 10)",
            "u8",
            integer(251, "u8"),
        ),
        Row::constant(
            "NUM-02.arithmetic-exact-before-bitwise".into(),
            &["NUM-02"],
            "",
            "(250 + 10) - 10",
            "u8",
            integer(250, "u8"),
        ),
        Row::constant(
            "NUM-02.bitwise-project-negative".into(),
            &["NUM-02"],
            "",
            "(0 - 1) & 255",
            "u8",
            integer(255, "u8"),
        ),
        Row::constant(
            "NUM-02.bitwise-project-signed".into(),
            &["NUM-02"],
            "",
            "(127 + 1) | 0",
            "i8",
            integer(-128, "i8"),
        ),
        Row::constant(
            "NUM-02.i8-parenthesized-negative".into(),
            &["NUM-02"],
            "",
            "-(128)",
            "i8",
            integer(-128, "i8"),
        ),
        Row::constant(
            "NUM-02.i8-subtraction-negative".into(),
            &["NUM-02"],
            "",
            "0 - 128",
            "i8",
            integer(-128, "i8"),
        ),
        Row::constant(
            "NUM-02.u8-reduced-constant".into(),
            &["NUM-02"],
            "",
            "256 - 1",
            "u8",
            integer(255, "u8"),
        ),
        Row::constant(
            "NUM-02.u8-complement-wide".into(),
            &["NUM-02"],
            "",
            "^256",
            "u8",
            integer(255, "u8"),
        ),
        Row::constant(
            "NUM-02.u8-static-bitwise-wide".into(),
            &["NUM-02"],
            "",
            "1 & 256",
            "u8",
            integer(0, "u8"),
        ),
    ]
}

fn run_constant(id: &str) {
    row(id).run();
}

#[test]
fn num02_bitwise_project_overflow() {
    run_constant("NUM-02.bitwise-project-overflow");
}

#[test]
fn num02_arithmetic_exact_before_bitwise() {
    run_constant("NUM-02.arithmetic-exact-before-bitwise");
}

#[test]
fn num02_bitwise_project_negative() {
    run_constant("NUM-02.bitwise-project-negative");
}

#[test]
fn num02_cast_overflow_boundary() {
    let source = "package main\nconst X:u8=(250+10) as u8\nfunc main(){}\n";
    run(&Case {
        id: "NUM-02.cast-overflow-boundary",
        specs: &["NUM-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: source,
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Any,
            location: Location::WithinSource {
                file: "main.ond",
                text: "(250+10) as u8",
                occurrence: 0,
            },
        }]),
    })
    .unwrap();
}

#[test]
fn num02_bitwise_project_signed() {
    run_constant("NUM-02.bitwise-project-signed");
}

#[test]
fn num02_i8_parenthesized_negative() {
    run_constant("NUM-02.i8-parenthesized-negative");
}

#[test]
fn num02_i8_subtraction_negative() {
    run_constant("NUM-02.i8-subtraction-negative");
}

#[test]
fn num02_u8_reduced_constant() {
    run_constant("NUM-02.u8-reduced-constant");
}

#[test]
fn num02_u8_complement_wide() {
    run_constant("NUM-02.u8-complement-wide");
}

#[test]
fn num02_u8_static_bitwise_wide() {
    run_constant("NUM-02.u8-static-bitwise-wide");
}

#[test]
fn num02_u8_runtime_add_wide() {
    let source = "package main\nfunc observe(x:u8)->u8{return x+256}\nfunc main(){}\n";
    run(&Case {
        id: "NUM-02.u8-runtime-add-wide",
        specs: &["NUM-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: source,
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Any,
            location: Location::Source {
                file: "main.ond",
                text: "256",
                occurrence: 0,
            },
        }]),
    })
    .unwrap();
}

#[test]
fn num02_u8_runtime_add_reduced() {
    let source = "package main\nfunc observe(x:u8)->u8{return x+(256-1)}\nfunc main(){}\n";
    run(&Case {
        id: "NUM-02.u8-runtime-add-reduced",
        specs: &["NUM-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: source,
        }],
        expected: Expected::Accept(&[]),
    })
    .unwrap();
}

#[test]
fn num02_u8_runtime_bitwise_wide() {
    let source = "package main\nfunc observe(x:u8)->u8{return x&256}\nfunc main(){}\n";
    run(&Case {
        id: "NUM-02.u8-runtime-bitwise-wide",
        specs: &["NUM-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: source,
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Any,
            location: Location::Source {
                file: "main.ond",
                text: "256",
                occurrence: 0,
            },
        }]),
    })
    .unwrap();
}
