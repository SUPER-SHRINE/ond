//! Negative tests for the harness itself: incorrect expectations must fail.
use super::*;

#[test]
fn execution_checks_require_the_vm_layer_and_compare_trace_and_writes() {
    let seed = evaluation::CASES[0];
    assert!(
        run(&Case {
            layer: Layer::Core,
            ..seed
        })
        .unwrap_err()
        .contains("MemoryVm")
    );
    assert!(
        run(&Case {
            expected: Expected::Accept(&[]),
            ..seed
        })
        .unwrap_err()
        .contains("MemoryVm")
    );
    for expected in [&[999, 14][..], &[123, 999][..]] {
        let checks = [Check::MemoryExecution {
            description: "incorrect trace or result",
            function: "main.test",
            input: &[],
            expected,
        }];
        assert!(
            run(&Case {
                expected: Expected::Accept(&checks),
                ..seed
            })
            .unwrap_err()
            .contains("expected")
        );
    }
}

#[test]
fn eof_locations_and_loader_rejections_are_not_project_wildcards() {
    let empty = Case {
        files: &[SourceFile {
            path: "main.ond",
            text: "",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Exact("expected `package`"),
            location: Location::EndOfFile { file: "main.ond" },
        }]),
        ..cases::CASES[1]
    };
    run(&empty).unwrap();
    let absent = Case {
        manifest: false,
        ..empty
    };
    assert!(
        run(&absent)
            .unwrap_err()
            .contains("message/location not found")
    );
    let wrong_eof = Case {
        files: &[SourceFile {
            path: "main.ond",
            text: "func main() {}",
        }],
        ..empty
    };
    assert!(
        run(&wrong_eof)
            .unwrap_err()
            .contains("message/location not found")
    );
    let unknown_file = Case {
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Exact("expected `package`"),
            location: Location::EndOfFile {
                file: "missing.ond",
            },
        }]),
        ..empty
    };
    assert!(
        run(&unknown_file)
            .unwrap_err()
            .contains("source file not in fixture")
    );
}

#[test]
fn rejects_wrong_constant_value_and_type() {
    for checks in [
        &[Check::HirConstant {
            package: ".",
            name: "Answer",
            ty: "i32",
            value: Scalar::Integer(41),
        }][..],
        &[Check::HirConstant {
            package: ".",
            name: "Answer",
            ty: "u32",
            value: Scalar::Integer(42),
        }][..],
    ] {
        let case = Case {
            expected: Expected::Accept(checks),
            ..cases::CASES[2]
        };
        let error = run(&case).unwrap_err();
        assert!(error.contains(case.id) && error.contains("Answer"));
    }
}

#[test]
fn rejects_wrong_diagnostic_file_message_and_count() {
    const WRONG_FILE: ExpectedDiagnostic = ExpectedDiagnostic {
        severity: Severity::Error,
        message: Message::Contains("builtin type name"),
        location: Location::Source {
            file: "main.ond",
            text: "i32",
            occurrence: 0,
        },
    };
    const WRONG_MESSAGE: ExpectedDiagnostic = ExpectedDiagnostic {
        message: Message::Exact("unrelated error"),
        location: Location::Source {
            file: "other.ond",
            text: "i32",
            occurrence: 0,
        },
        ..WRONG_FILE
    };
    for diagnostics in [
        &[WRONG_FILE][..],
        &[WRONG_MESSAGE][..],
        &[WRONG_FILE, WRONG_FILE][..],
    ] {
        let case = Case {
            expected: Expected::Reject(diagnostics),
            ..cases::CASES[3]
        };
        let reason = if diagnostics.len() == 1 {
            "message/location not found"
        } else {
            "diagnostic count"
        };
        assert!(run(&case).unwrap_err().contains(reason));
    }
}

#[test]
fn rejects_unexpected_acceptance_and_rejection() {
    let accept = Case {
        expected: Expected::Accept(&[]),
        ..cases::CASES[1]
    };
    assert!(run(&accept).unwrap_err().contains("expected acceptance"));
    let reject = Case {
        expected: cases::CASES[4].expected,
        ..cases::CASES[0]
    };
    assert!(run(&reject).unwrap_err().contains("expected rejection"));
}

#[test]
fn rejects_duplicate_ids_unsafe_paths_and_incompatible_layers() {
    assert!(
        validate_registry(&[cases::CASES[0], cases::CASES[0]])
            .unwrap_err()
            .contains("empty/duplicate case ID")
    );
    for files in [
        &[SourceFile {
            path: "../main.ond",
            text: "",
        }][..],
        &[SourceFile {
            path: "C:/main.ond",
            text: "",
        }][..],
        &[SourceFile {
            path: "/main.ond",
            text: "",
        }][..],
        &[SourceFile {
            path: "a\\main.ond",
            text: "",
        }][..],
        &[SourceFile {
            path: "a/../main.ond",
            text: "",
        }][..],
        &[SourceFile {
            path: "NUL.ond",
            text: "",
        }][..],
    ] {
        // Validation is exercised before a temporary project is ever created.
        assert!(
            run(&Case {
                files,
                ..cases::CASES[0]
            })
            .unwrap_err()
            .contains("source path")
        );
    }
    assert!(
        run(&Case {
            layer: Layer::Parse,
            ..cases::CASES[2]
        })
        .unwrap_err()
        .contains("cannot assert HIR/MIR")
    );
}

#[test]
fn rejects_absent_location_and_empty_reject_expectations() {
    assert!(
        run(&Case {
            specs: &["UNKNOWN-99"],
            ..cases::CASES[0]
        })
        .unwrap_err()
        .contains("unknown checklist spec ID")
    );
    let case = Case {
        expected: Expected::Reject(&[]),
        ..cases::CASES[1]
    };
    assert!(
        run(&case)
            .unwrap_err()
            .contains("requires expected diagnostics")
    );
    let case = Case {
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("error"),
            location: Location::Source {
                file: "main.ond",
                text: "missing text",
                occurrence: 0,
            },
        }]),
        ..cases::CASES[1]
    };
    assert!(run(&case).unwrap_err().contains("occurrence not found"));
}
