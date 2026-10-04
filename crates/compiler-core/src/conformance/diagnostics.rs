//! Exact source diagnostics across UTF-8, CRLF, multiple files and compiler stages.
use super::*;

pub(super) const CASES: &[Case] = &[
    Case {
        id: "DIAG-02.multifile-unicode-hir",
        specs: &["DIAG-01", "DIAG-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\n",
            },
            SourceFile {
                path: "00.ond",
                text: "package main\r\n// 日本語😀 true\r\nfunc check(値: bool) { var 数: i32 = 値 }\r\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Exact("declaration type mismatch: expected `i32`, found `bool`"),
            location: Location::Source {
                file: "00.ond",
                text: "値",
                occurrence: 1,
            },
        }]),
    },
    Case {
        id: "DIAG-02.imported-package-body",
        specs: &["DIAG-01", "DIAG-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nimport \"shared\"\nfunc main() {}\n",
            },
            SourceFile {
                path: "shared/値.ond",
                text: "package shared\n// 𐐀😀\nfunc Value() -> i32 { return 未定義 }\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Exact("unknown identifier `未定義`"),
            location: Location::Source {
                file: "shared/値.ond",
                text: "未定義",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "DIAG-01.multiple-independent-errors",
        specs: &["DIAG-01", "DIAG-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\nfunc valid() -> i32 { return 42 }\n",
            },
            SourceFile {
                path: "a.ond",
                text: "package main\n// 😀\nfunc first() { var x: i32 = true }\n",
            },
            SourceFile {
                path: "z.ond",
                text: "package main\n// 日本語\nfunc last() { var x = missing }\n",
            },
        ],
        expected: Expected::Reject(&[
            ExpectedDiagnostic {
                severity: Severity::Error,
                message: Message::Exact("boolean literal requires bool type"),
                location: Location::Source {
                    file: "a.ond",
                    text: "true",
                    occurrence: 0,
                },
            },
            ExpectedDiagnostic {
                severity: Severity::Error,
                message: Message::Exact("unknown identifier `missing`"),
                location: Location::Source {
                    file: "z.ond",
                    text: "missing",
                    occurrence: 0,
                },
            },
        ]),
    },
    Case {
        id: "DIAG-01.invalid-global-no-partial-success",
        specs: &["DIAG-01", "DIAG-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\nfunc valid() -> i32 { return 1 }\n",
            },
            SourceFile {
                path: "global.ond",
                text: "package main\n// 😀\nvar 値: bool = 1\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Exact("integer literal requires an integer type"),
            location: Location::Source {
                file: "global.ond",
                text: "1",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "DIAG-02.unicode-lexical-byte-range",
        specs: &["DIAG-01", "DIAG-02"],
        layer: Layer::Parse,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\r\n// 日本語\r\nfunc main() { 😀 }\r\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Exact("unexpected character `😀`"),
            location: Location::Source {
                file: "main.ond",
                text: "😀",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "DIAG-02.unicode-parser-byte-range",
        specs: &["DIAG-01", "DIAG-02"],
        layer: Layer::Parse,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\r\n// 日本語😀\r\nfunc main() { var 値 = 1 var 次 = 2 }\r\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Exact("expected `;` or newline"),
            location: Location::Source {
                file: "main.ond",
                text: "var",
                occurrence: 1,
            },
        }]),
    },
];

#[test]
fn source_diagnostic_contracts() {
    for case in CASES {
        run(case).unwrap();
    }
}

#[test]
fn wrong_cause_file_occurrence_and_count_cannot_pass() {
    let seed = CASES[0];
    let Expected::Reject(expected) = seed.expected else {
        unreachable!()
    };
    let diagnostic = expected[0];
    for bad in [
        ExpectedDiagnostic {
            message: Message::Exact("unknown identifier `値`"),
            ..diagnostic
        },
        ExpectedDiagnostic {
            location: Location::Source {
                file: "00.ond",
                text: "値",
                occurrence: 0,
            },
            ..diagnostic
        },
        ExpectedDiagnostic {
            location: Location::EndOfFile { file: "main.ond" },
            ..diagnostic
        },
    ] {
        assert!(
            run(&Case {
                expected: Expected::Reject(&[bad]),
                ..seed
            })
            .unwrap_err()
            .contains("message/location not found")
        );
    }
    assert!(
        run(&Case {
            expected: Expected::Reject(&[diagnostic, diagnostic]),
            ..seed
        })
        .unwrap_err()
        .contains("diagnostic count")
    );
}
