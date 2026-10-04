//! Q-03: constant eligibility, static checking and value evaluation are separate.
use super::*;
pub(super) const CASES: &[Case] = &[
    Case {
        id: "SYN-12.q03-short_and",
        specs: &["SYN-12", "EVAL-01"],
        manifest: true,
        layer: Layer::Core,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = false && (1 / 0 == 0)\nfunc f() -> bool { return true; }\nfunc array(x: u8) -> [2]u8 { return [2]u8{}; }\nfunc main() {}\n",
        }],
        expected: Expected::Accept(&[Check::HirConstant {
            package: ".",
            name: "X",
            ty: "bool",
            value: Scalar::Bool(false),
        }]),
    },
    Case {
        id: "SYN-12.q03-short_or",
        specs: &["SYN-12", "EVAL-01"],
        manifest: true,
        layer: Layer::Core,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = true || (1 / 0 == 0)\nfunc f() -> bool { return true; }\nfunc array(x: u8) -> [2]u8 { return [2]u8{}; }\nfunc main() {}\n",
        }],
        expected: Expected::Accept(&[Check::HirConstant {
            package: ".",
            name: "X",
            ty: "bool",
            value: Scalar::Bool(true),
        }]),
    },
    Case {
        id: "SYN-12.q03-array_call_evaluates_argument",
        specs: &["SYN-12", "EVAL-01"],
        manifest: true,
        layer: Layer::Core,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = len(array(1 / 0))\nfunc f() -> bool { return true; }\nfunc array(x: u8) -> [2]u8 { return [2]u8{}; }\nfunc main() {}\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("division by zero"),
            location: Location::Source {
                file: "main.ond",
                text: "1 / 0",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "SYN-12.q03-array_literal_evaluates_element",
        specs: &["SYN-12", "EVAL-01"],
        manifest: true,
        layer: Layer::Core,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = len([2]i32{1 / 0, 0})\nfunc f() -> bool { return true; }\nfunc array(x: u8) -> [2]u8 { return [2]u8{}; }\nfunc main() {}\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("division by zero"),
            location: Location::Source {
                file: "main.ond",
                text: "1 / 0",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "SYN-12.q03-array_len_evaluates_cast",
        specs: &["SYN-12", "EVAL-01"],
        manifest: true,
        layer: Layer::Core,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = len(array((250 + 6) as u8))\nfunc f() -> bool { return true; }\nfunc array(x: u8) -> [2]u8 { return [2]u8{}; }\nfunc main() {}\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("constant cast is out of range"),
            location: Location::Source {
                file: "main.ond",
                text: "250 + 6) as u8",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "SYN-12.q03-utf8_length",
        specs: &["SYN-12", "EVAL-01"],
        manifest: true,
        layer: Layer::Core,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = len(\"音\")\nfunc f() -> bool { return true; }\nfunc array(x: u8) -> [2]u8 { return [2]u8{}; }\nfunc main() {}\n",
        }],
        expected: Expected::Accept(&[Check::HirConstant {
            package: ".",
            name: "X",
            ty: "u32",
            value: Scalar::Integer(3),
        }]),
    },
    Case {
        id: "SYN-12.q03-short_valid_cast",
        specs: &["SYN-12", "EVAL-01"],
        manifest: true,
        layer: Layer::Core,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = false && ((250 + 5) as u8 == 0)\nfunc f() -> bool { return true; }\nfunc array(x: u8) -> [2]u8 { return [2]u8{}; }\nfunc main() {}\n",
        }],
        expected: Expected::Accept(&[Check::HirConstant {
            package: ".",
            name: "X",
            ty: "bool",
            value: Scalar::Bool(false),
        }]),
    },
    Case {
        id: "SYN-12.q03-ordinary_call",
        specs: &["SYN-12", "EVAL-01"],
        manifest: true,
        layer: Layer::Core,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = f()\nfunc f() -> bool { return true; }\nfunc array(x: u8) -> [2]u8 { return [2]u8{}; }\nfunc main() {}\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("constant initializer requires a compile-time value"),
            location: Location::Source {
                file: "main.ond",
                text: "f()",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "SYN-12.q03-skipped_call_and",
        specs: &["SYN-12", "EVAL-01"],
        manifest: true,
        layer: Layer::Core,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = false && f()\nfunc f() -> bool { return true; }\nfunc array(x: u8) -> [2]u8 { return [2]u8{}; }\nfunc main() {}\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("constant initializer requires a compile-time value"),
            location: Location::Source {
                file: "main.ond",
                text: "false && f()",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "SYN-12.q03-skipped_call_or",
        specs: &["SYN-12", "EVAL-01"],
        manifest: true,
        layer: Layer::Core,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = true || f()\nfunc f() -> bool { return true; }\nfunc array(x: u8) -> [2]u8 { return [2]u8{}; }\nfunc main() {}\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("constant initializer requires a compile-time value"),
            location: Location::Source {
                file: "main.ond",
                text: "true || f()",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "SYN-12.q03-skipped_unknown",
        specs: &["SYN-12", "EVAL-01"],
        manifest: true,
        layer: Layer::Core,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = false && missing\nfunc f() -> bool { return true; }\nfunc array(x: u8) -> [2]u8 { return [2]u8{}; }\nfunc main() {}\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("unknown"),
            location: Location::Source {
                file: "main.ond",
                text: "missing",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "SYN-12.q03-skipped_type_error",
        specs: &["SYN-12", "EVAL-01"],
        manifest: true,
        layer: Layer::Core,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = false && 1\nfunc f() -> bool { return true; }\nfunc array(x: u8) -> [2]u8 { return [2]u8{}; }\nfunc main() {}\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("integer literal requires an integer type"),
            location: Location::Source {
                file: "main.ond",
                text: "1",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "SYN-12.q03-len_argument_type_error",
        specs: &["SYN-12", "EVAL-01"],
        manifest: true,
        layer: Layer::Core,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = len(array(true))\nfunc f() -> bool { return true; }\nfunc array(x: u8) -> [2]u8 { return [2]u8{}; }\nfunc main() {}\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("boolean literal"),
            location: Location::Source {
                file: "main.ond",
                text: "true",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "SYN-12.q03-len_unknown",
        specs: &["SYN-12", "EVAL-01"],
        manifest: true,
        layer: Layer::Core,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = len(array(missing))\nfunc f() -> bool { return true; }\nfunc array(x: u8) -> [2]u8 { return [2]u8{}; }\nfunc main() {}\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("unknown"),
            location: Location::Source {
                file: "main.ond",
                text: "missing",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "SYN-12.q03-evaluated_cast_range",
        specs: &["SYN-12", "EVAL-01"],
        manifest: true,
        layer: Layer::Core,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = (250 + 6) as u8\nfunc f() -> bool { return true; }\nfunc array(x: u8) -> [2]u8 { return [2]u8{}; }\nfunc main() {}\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("out of range"),
            location: Location::Source {
                file: "main.ond",
                text: "250 + 6) as u8",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "SYN-12.q03-len_cast_type_error",
        specs: &["SYN-12", "EVAL-01"],
        manifest: true,
        layer: Layer::Core,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = len(array(true as u8))\nfunc f() -> bool { return true; }\nfunc array(x: u8) -> [2]u8 { return [2]u8{}; }\nfunc main() {}\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("conversion"),
            location: Location::Source {
                file: "main.ond",
                text: "true",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "SYN-12.q03-short_cast_range_and",
        specs: &["SYN-12", "EVAL-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = false && ((250 + 6) as u8 == 0)\nfunc f(x: u8) -> bool { return true; }\nfunc main() {}\n",
        }],
        expected: Expected::Accept(&[Check::HirConstant {
            package: ".",
            name: "X",
            ty: "bool",
            value: Scalar::Bool(false),
        }]),
    },
    Case {
        id: "SYN-12.q03-short_cast_range_or",
        specs: &["SYN-12", "EVAL-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = true || ((250 + 6) as u8 == 0)\nfunc f(x: u8) -> bool { return true; }\nfunc main() {}\n",
        }],
        expected: Expected::Accept(&[Check::HirConstant {
            package: ".",
            name: "X",
            ty: "bool",
            value: Scalar::Bool(true),
        }]),
    },
    Case {
        id: "SYN-12.q03-computed_short_cast",
        specs: &["SYN-12", "EVAL-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = (1 > 2) && ((250 + 6) as u8 == 0)\nfunc f(x: u8) -> bool { return true; }\nfunc main() {}\n",
        }],
        expected: Expected::Accept(&[Check::HirConstant {
            package: ".",
            name: "X",
            ty: "bool",
            value: Scalar::Bool(false),
        }]),
    },
    Case {
        id: "SYN-12.q03-nested_short_cast",
        specs: &["SYN-12", "EVAL-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = false && (true && ((250 + 6) as u8 == 0))\nfunc f(x: u8) -> bool { return true; }\nfunc main() {}\n",
        }],
        expected: Expected::Accept(&[Check::HirConstant {
            package: ".",
            name: "X",
            ty: "bool",
            value: Scalar::Bool(false),
        }]),
    },
    Case {
        id: "SYN-12.q03-evaluated_short_cast",
        specs: &["SYN-12", "EVAL-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = true && ((250 + 6) as u8 == 0)\nfunc f(x: u8) -> bool { return true; }\nfunc main() {}\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("constant cast is out of range"),
            location: Location::Source {
                file: "main.ond",
                text: "250 + 6) as u8",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "SYN-12.q03-evaluated_or_cast",
        specs: &["SYN-12", "EVAL-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = false || ((250 + 6) as u8 == 0)\nfunc f(x: u8) -> bool { return true; }\nfunc main() {}\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("constant cast is out of range"),
            location: Location::Source {
                file: "main.ond",
                text: "250 + 6) as u8",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "SYN-12.q03-skipped_cast_type_error",
        specs: &["SYN-12", "EVAL-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = false && (true as u8 == 0)\nfunc f(x: u8) -> bool { return true; }\nfunc main() {}\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("invalid explicit type conversion"),
            location: Location::Source {
                file: "main.ond",
                text: "true",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "SYN-12.q03-skipped_cast_unknown",
        specs: &["SYN-12", "EVAL-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = false && (missing as u8 == 0)\nfunc f(x: u8) -> bool { return true; }\nfunc main() {}\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("unknown identifier"),
            location: Location::Source {
                file: "main.ond",
                text: "missing",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "SYN-12.q03-skipped_cast_call_forbidden",
        specs: &["SYN-12", "EVAL-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst X = false && f((250 + 6) as u8)\nfunc f(x: u8) -> bool { return true; }\nfunc main() {}\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("constant initializer requires a compile-time value"),
            location: Location::Source {
                file: "main.ond",
                text: "false && f((250 + 6) as u8)",
                occurrence: 0,
            },
        }]),
    },
];
#[test]
fn short_and() {
    run(&CASES[0]).unwrap();
}
#[test]
fn short_or() {
    run(&CASES[1]).unwrap();
}
#[test]
fn array_call_skips_argument() {
    run(&CASES[2]).unwrap();
}
#[test]
fn array_literal_skips_element() {
    run(&CASES[3]).unwrap();
}
#[test]
fn array_len_skips_cast() {
    run(&CASES[4]).unwrap();
}
#[test]
fn utf8_length() {
    run(&CASES[5]).unwrap();
}
#[test]
fn short_valid_cast() {
    run(&CASES[6]).unwrap();
}
#[test]
fn ordinary_call() {
    run(&CASES[7]).unwrap();
}
#[test]
fn skipped_call_and() {
    run(&CASES[8]).unwrap();
}
#[test]
fn skipped_call_or() {
    run(&CASES[9]).unwrap();
}
#[test]
fn skipped_unknown() {
    run(&CASES[10]).unwrap();
}
#[test]
fn skipped_type_error() {
    run(&CASES[11]).unwrap();
}
#[test]
fn len_argument_type_error() {
    run(&CASES[12]).unwrap();
}
#[test]
fn len_unknown() {
    run(&CASES[13]).unwrap();
}
#[test]
fn evaluated_cast_range() {
    run(&CASES[14]).unwrap();
}
#[test]
fn len_cast_type_error() {
    run(&CASES[15]).unwrap();
}
#[test]
fn short_cast_range_and() {
    run(&CASES[16]).unwrap();
}
#[test]
fn short_cast_range_or() {
    run(&CASES[17]).unwrap();
}
#[test]
fn computed_short_cast() {
    run(&CASES[18]).unwrap();
}
#[test]
fn nested_short_cast() {
    run(&CASES[19]).unwrap();
}
#[test]
fn evaluated_short_cast() {
    run(&CASES[20]).unwrap();
}
#[test]
fn evaluated_or_cast() {
    run(&CASES[21]).unwrap();
}
#[test]
fn skipped_cast_type_error() {
    run(&CASES[22]).unwrap();
}
#[test]
fn skipped_cast_unknown() {
    run(&CASES[23]).unwrap();
}
#[test]
fn skipped_cast_call_forbidden() {
    run(&CASES[24]).unwrap();
}
