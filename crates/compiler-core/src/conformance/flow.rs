//! Structural return/loop termination rules and control syntax diagnostics.
use super::*;
pub(super) const CASES: &[Case] = &[
    Case {
        id: "FLOW-02.if_integer",
        specs: &["FLOW-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test() { if 1 {} }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("condition must have type bool"),
            location: Location::Source {
                file: "main.ond",
                text: "1",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "FLOW-02.for_integer",
        specs: &["FLOW-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test() { for 1 {} }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("condition must have type bool"),
            location: Location::Source {
                file: "main.ond",
                text: "1",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "FLOW-02.for_clause_integer",
        specs: &["FLOW-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test() { for i := 0; 1; i = i + 1 {} }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("condition must have type bool"),
            location: Location::Source {
                file: "main.ond",
                text: "1",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "FLOW-02.break_outside",
        specs: &["FLOW-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test() { break; }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("break/continue requires an enclosing loop"),
            location: Location::Source {
                file: "main.ond",
                text: "break",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "FLOW-02.continue_outside",
        specs: &["FLOW-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test() { continue; }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("break/continue requires an enclosing loop"),
            location: Location::Source {
                file: "main.ond",
                text: "continue",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "FLOW-02.if_missing_block",
        specs: &["FLOW-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test() { if true return; }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("expected `{`"),
            location: Location::Source {
                file: "main.ond",
                text: "return",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "FLOW-02.for_missing_block",
        specs: &["FLOW-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test() { for true return; }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("expected `{`"),
            location: Location::Source {
                file: "main.ond",
                text: "return",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "FLOW-02.else_missing_block",
        specs: &["FLOW-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test() { if true {} else return; }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("expected `if` or block after `else`"),
            location: Location::Source {
                file: "main.ond",
                text: "return",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "FLOW-03.both_branches",
        specs: &["FLOW-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test(b: bool) -> i32 { if b { return 1; } else { return 2; } }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-03.one_branch",
        specs: &["FLOW-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test(b: bool) -> i32 { if b { return 1; } }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("missing return in function `test`"),
            location: Location::Source {
                file: "main.ond",
                text: "{ if b { return 1; } }",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "FLOW-03.nested_missing_branch",
        specs: &["FLOW-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test(b: bool) -> i32 { if b { if b { return 1; } } else { return 2; } }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("missing return in function `test`"),
            location: Location::Source {
                file: "main.ond",
                text: "{ if b { if b { return 1; } } else { return 2; } }",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "FLOW-03.nested_all_returns",
        specs: &["FLOW-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test(b: bool) -> i32 { if b { if b { return 1; } else { return 2; } } else { return 3; } }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-03.infinite_loop",
        specs: &["FLOW-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test(b: bool) -> i32 { for {} }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-03.infinite_clause",
        specs: &["FLOW-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test(b: bool) -> i32 { for ; ; {} }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-03.loop_break",
        specs: &["FLOW-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test(b: bool) -> i32 { for { break; } }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("missing return in function `test`"),
            location: Location::Source {
                file: "main.ond",
                text: "{ for { break; } }",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "FLOW-03.nested_break_inner",
        specs: &["FLOW-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test(b: bool) -> i32 { for { for { break; } } }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-03.nested_break_outer",
        specs: &["FLOW-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test(b: bool) -> i32 { for { for { break; }; break; } }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("missing return in function `test`"),
            location: Location::Source {
                file: "main.ond",
                text: "{ for { for { break; }; break; } }",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "FLOW-03.unreachable_break_return",
        specs: &["FLOW-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test(b: bool) -> i32 { for { return 1; break; } }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-03.unreachable_break_continue",
        specs: &["FLOW-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test(b: bool) -> i32 { for { continue; break; } }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-03.unreachable_break_after_nested_infinite",
        specs: &["FLOW-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test(b: bool) -> i32 { for { for {}; break; } }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-03.if_false_break",
        specs: &["FLOW-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test(b: bool) -> i32 { for { if false { break; } } }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("missing return in function `test`"),
            location: Location::Source {
                file: "main.ond",
                text: "{ for { if false { break; } } }",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "FLOW-03.conditional_loop",
        specs: &["FLOW-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test(b: bool) -> i32 { for b { return 1; } }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("missing return in function `test`"),
            location: Location::Source {
                file: "main.ond",
                text: "{ for b { return 1; } }",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "FLOW-03.literal_true_loop",
        specs: &["FLOW-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test(b: bool) -> i32 { for true {} }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("missing return in function `test`"),
            location: Location::Source {
                file: "main.ond",
                text: "{ for true {} }",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "FLOW-03.conditional_loop_then_return",
        specs: &["FLOW-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test(b: bool) -> i32 { for b { return 1; }; return 2; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "FLOW-03.branch_terminal_loop",
        specs: &["FLOW-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test(b: bool) -> i32 { if b { return 1; } else { for {} } }\n",
        }],
        expected: Expected::Accept(&[]),
    },
];
#[test]
fn if_integer() {
    run(&CASES[0]).unwrap();
}
#[test]
fn for_integer() {
    run(&CASES[1]).unwrap();
}
#[test]
fn for_clause_integer() {
    run(&CASES[2]).unwrap();
}
#[test]
fn break_outside() {
    run(&CASES[3]).unwrap();
}
#[test]
fn continue_outside() {
    run(&CASES[4]).unwrap();
}
#[test]
fn if_missing_block() {
    run(&CASES[5]).unwrap();
}
#[test]
fn for_missing_block() {
    run(&CASES[6]).unwrap();
}
#[test]
fn else_missing_block() {
    run(&CASES[7]).unwrap();
}
#[test]
fn both_branches() {
    run(&CASES[8]).unwrap();
}
#[test]
fn one_branch() {
    run(&CASES[9]).unwrap();
}
#[test]
fn nested_missing_branch() {
    run(&CASES[10]).unwrap();
}
#[test]
fn nested_all_returns() {
    run(&CASES[11]).unwrap();
}
#[test]
fn infinite_loop() {
    run(&CASES[12]).unwrap();
}
#[test]
fn infinite_clause() {
    run(&CASES[13]).unwrap();
}
#[test]
fn loop_break() {
    run(&CASES[14]).unwrap();
}
#[test]
fn nested_break_inner() {
    run(&CASES[15]).unwrap();
}
#[test]
fn nested_break_outer() {
    run(&CASES[16]).unwrap();
}
#[test]
fn unreachable_break_return() {
    run(&CASES[17]).unwrap();
}
#[test]
fn unreachable_break_continue() {
    run(&CASES[18]).unwrap();
}
#[test]
fn unreachable_break_after_nested_infinite() {
    run(&CASES[19]).unwrap();
}
#[test]
fn if_false_break() {
    run(&CASES[20]).unwrap();
}
#[test]
fn conditional_loop() {
    run(&CASES[21]).unwrap();
}
#[test]
fn literal_true_loop() {
    run(&CASES[22]).unwrap();
}
#[test]
fn conditional_loop_then_return() {
    run(&CASES[23]).unwrap();
}
#[test]
fn branch_terminal_loop() {
    run(&CASES[24]).unwrap();
}
