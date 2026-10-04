//! Local binding lifetime, redeclaration and import qualifier shadowing.
use super::*;
pub(super) const CASES: &[Case] = &[
    Case {
        id: "NAME-01.local_var_var",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test() { var Clash = 1; var Clash = 2; }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate local declaration"),
            location: Location::Source {
                file: "main.ond",
                text: "Clash",
                occurrence: 1,
            },
        }]),
    },
    Case {
        id: "NAME-01.local_var_const",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test() { var Clash = 1; const Clash = 2; }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate local declaration"),
            location: Location::Source {
                file: "main.ond",
                text: "Clash",
                occurrence: 1,
            },
        }]),
    },
    Case {
        id: "NAME-01.local_const_var",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test() { const Clash = 1; var Clash = 2; }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate local declaration"),
            location: Location::Source {
                file: "main.ond",
                text: "Clash",
                occurrence: 1,
            },
        }]),
    },
    Case {
        id: "NAME-01.local_const_const",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test() { const Clash = 1; const Clash = 2; }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate local declaration"),
            location: Location::Source {
                file: "main.ond",
                text: "Clash",
                occurrence: 1,
            },
        }]),
    },
    Case {
        id: "NAME-01.parameter_var",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test(Clash: i32) { var Clash = 1; }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate local declaration"),
            location: Location::Source {
                file: "main.ond",
                text: "Clash",
                occurrence: 1,
            },
        }]),
    },
    Case {
        id: "NAME-01.parameter_const",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test(Clash: i32) { const Clash = 1; }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate local declaration"),
            location: Location::Source {
                file: "main.ond",
                text: "Clash",
                occurrence: 1,
            },
        }]),
    },
    Case {
        id: "NAME-01.duplicate_parameter",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test(Clash: i32, Clash: i32) {}\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate local declaration"),
            location: Location::Source {
                file: "main.ond",
                text: "Clash: i32",
                occurrence: 1,
            },
        }]),
    },
    Case {
        id: "NAME-01.duplicate_local_names",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test() { var Clash, Clash = 1, 2; }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate local declaration"),
            location: Location::Source {
                file: "main.ond",
                text: "Clash",
                occurrence: 1,
            },
        }]),
    },
    Case {
        id: "NAME-01.duplicate_const_names",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test() { const Clash, Clash = 1, 2; }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate local declaration"),
            location: Location::Source {
                file: "main.ond",
                text: "Clash",
                occurrence: 1,
            },
        }]),
    },
    Case {
        id: "NAME-02.var_self",
        specs: &["NAME-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test() { var Fresh = Fresh; }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("unknown identifier"),
            location: Location::Source {
                file: "main.ond",
                text: "Fresh",
                occurrence: 1,
            },
        }]),
    },
    Case {
        id: "NAME-02.var_simultaneous",
        specs: &["NAME-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test() { var Fresh, Other = 1, Fresh; }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("unknown identifier"),
            location: Location::Source {
                file: "main.ond",
                text: "Fresh",
                occurrence: 1,
            },
        }]),
    },
    Case {
        id: "NAME-02.const_simultaneous",
        specs: &["NAME-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test() { const Fresh, Other = 1, Fresh; }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("unknown identifier"),
            location: Location::Source {
                file: "main.ond",
                text: "Fresh",
                occurrence: 1,
            },
        }]),
    },
    Case {
        id: "NAME-02.short_simultaneous",
        specs: &["NAME-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test() { Fresh, Other := 1, Fresh; }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("unknown identifier"),
            location: Location::Source {
                file: "main.ond",
                text: "Fresh",
                occurrence: 1,
            },
        }]),
    },
    Case {
        id: "NAME-02.block_outside",
        specs: &["NAME-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test() { { var Fresh = 1; }; _ = Fresh; }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("unknown identifier"),
            location: Location::Source {
                file: "main.ond",
                text: "Fresh",
                occurrence: 1,
            },
        }]),
    },
    Case {
        id: "NAME-02.if_outside",
        specs: &["NAME-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test() { if Fresh := true; Fresh {}; _ = Fresh; }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("unknown identifier"),
            location: Location::Source {
                file: "main.ond",
                text: "Fresh",
                occurrence: 2,
            },
        }]),
    },
    Case {
        id: "NAME-02.for_outside",
        specs: &["NAME-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test() { for Fresh := 0; Fresh < 1; Fresh = Fresh + 1 {}; _ = Fresh; }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("unknown identifier"),
            location: Location::Source {
                file: "main.ond",
                text: "Fresh",
                occurrence: 4,
            },
        }]),
    },
    Case {
        id: "NAME-02.then_not_else",
        specs: &["NAME-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test() { if true { var Fresh = 1; } else { _ = Fresh; } }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("unknown identifier"),
            location: Location::Source {
                file: "main.ond",
                text: "Fresh",
                occurrence: 1,
            },
        }]),
    },
    Case {
        id: "NAME-02.for_body_not_post",
        specs: &["NAME-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc test() { for i := 0; i < 1; i = Fresh { var Fresh = 1; } }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("unknown identifier"),
            location: Location::Source {
                file: "main.ond",
                text: "Fresh",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "NAME-02.shadow_import_scalar",
        specs: &["NAME-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nimport \"shared\"\nfunc main() {}\nfunc test() { var shared = 1; _ = shared.Exported; }\n",
            },
            SourceFile {
                path: "shared/a.ond",
                text: "package shared\nconst Exported = 7\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("field access requires a struct"),
            location: Location::Source {
                file: "main.ond",
                text: "shared.Exported",
                occurrence: 0,
            },
        }]),
    },
];
#[test]
fn local_var_var() {
    run(&CASES[0]).unwrap();
}
#[test]
fn local_var_const() {
    run(&CASES[1]).unwrap();
}
#[test]
fn local_const_var() {
    run(&CASES[2]).unwrap();
}
#[test]
fn local_const_const() {
    run(&CASES[3]).unwrap();
}
#[test]
fn parameter_var() {
    run(&CASES[4]).unwrap();
}
#[test]
fn parameter_const() {
    run(&CASES[5]).unwrap();
}
#[test]
fn duplicate_parameter() {
    run(&CASES[6]).unwrap();
}
#[test]
fn duplicate_local_names() {
    run(&CASES[7]).unwrap();
}
#[test]
fn duplicate_const_names() {
    run(&CASES[8]).unwrap();
}
#[test]
fn var_self() {
    run(&CASES[9]).unwrap();
}
#[test]
fn var_simultaneous() {
    run(&CASES[10]).unwrap();
}
#[test]
fn const_simultaneous() {
    run(&CASES[11]).unwrap();
}
#[test]
fn short_simultaneous() {
    run(&CASES[12]).unwrap();
}
#[test]
fn block_outside() {
    run(&CASES[13]).unwrap();
}
#[test]
fn if_outside() {
    run(&CASES[14]).unwrap();
}
#[test]
fn for_outside() {
    run(&CASES[15]).unwrap();
}
#[test]
fn then_not_else() {
    run(&CASES[16]).unwrap();
}
#[test]
fn for_body_not_post() {
    run(&CASES[17]).unwrap();
}
#[test]
fn shadow_import_scalar() {
    run(&CASES[18]).unwrap();
}
