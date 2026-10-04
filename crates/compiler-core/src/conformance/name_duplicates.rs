//! Cross-file, cross-kind package declarations share one namespace.
use super::*;
pub(super) const CASES: &[Case] = &[
    Case {
        id: "NAME-01.package_var_var",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\nvar Clash: i32\n",
            },
            SourceFile {
                path: "z.ond",
                text: "package main\nvar Clash: i32\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate top-level declaration"),
            location: Location::Source {
                file: "z.ond",
                text: "Clash",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "NAME-01.package_var_const",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\nvar Clash: i32\n",
            },
            SourceFile {
                path: "z.ond",
                text: "package main\nconst Clash = 1\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate top-level declaration"),
            location: Location::Source {
                file: "z.ond",
                text: "Clash",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "NAME-01.package_var_func",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\nvar Clash: i32\n",
            },
            SourceFile {
                path: "z.ond",
                text: "package main\nfunc Clash() {}\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate top-level declaration"),
            location: Location::Source {
                file: "z.ond",
                text: "Clash",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "NAME-01.package_var_type",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\nvar Clash: i32\n",
            },
            SourceFile {
                path: "z.ond",
                text: "package main\ntype Clash i32\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate top-level declaration"),
            location: Location::Source {
                file: "z.ond",
                text: "Clash",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "NAME-01.package_const_var",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\nconst Clash = 1\n",
            },
            SourceFile {
                path: "z.ond",
                text: "package main\nvar Clash: i32\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate top-level declaration"),
            location: Location::Source {
                file: "z.ond",
                text: "Clash",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "NAME-01.package_const_const",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\nconst Clash = 1\n",
            },
            SourceFile {
                path: "z.ond",
                text: "package main\nconst Clash = 1\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate top-level declaration"),
            location: Location::Source {
                file: "z.ond",
                text: "Clash",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "NAME-01.package_const_func",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\nconst Clash = 1\n",
            },
            SourceFile {
                path: "z.ond",
                text: "package main\nfunc Clash() {}\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate top-level declaration"),
            location: Location::Source {
                file: "z.ond",
                text: "Clash",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "NAME-01.package_const_type",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\nconst Clash = 1\n",
            },
            SourceFile {
                path: "z.ond",
                text: "package main\ntype Clash i32\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate top-level declaration"),
            location: Location::Source {
                file: "z.ond",
                text: "Clash",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "NAME-01.package_func_var",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\nfunc Clash() {}\n",
            },
            SourceFile {
                path: "z.ond",
                text: "package main\nvar Clash: i32\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate top-level declaration"),
            location: Location::Source {
                file: "z.ond",
                text: "Clash",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "NAME-01.package_func_const",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\nfunc Clash() {}\n",
            },
            SourceFile {
                path: "z.ond",
                text: "package main\nconst Clash = 1\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate top-level declaration"),
            location: Location::Source {
                file: "z.ond",
                text: "Clash",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "NAME-01.package_func_func",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\nfunc Clash() {}\n",
            },
            SourceFile {
                path: "z.ond",
                text: "package main\nfunc Clash() {}\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate top-level declaration"),
            location: Location::Source {
                file: "z.ond",
                text: "Clash",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "NAME-01.package_func_type",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\nfunc Clash() {}\n",
            },
            SourceFile {
                path: "z.ond",
                text: "package main\ntype Clash i32\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate top-level declaration"),
            location: Location::Source {
                file: "z.ond",
                text: "Clash",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "NAME-01.package_type_var",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\ntype Clash i32\n",
            },
            SourceFile {
                path: "z.ond",
                text: "package main\nvar Clash: i32\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate top-level declaration"),
            location: Location::Source {
                file: "z.ond",
                text: "Clash",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "NAME-01.package_type_const",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\ntype Clash i32\n",
            },
            SourceFile {
                path: "z.ond",
                text: "package main\nconst Clash = 1\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate top-level declaration"),
            location: Location::Source {
                file: "z.ond",
                text: "Clash",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "NAME-01.package_type_func",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\ntype Clash i32\n",
            },
            SourceFile {
                path: "z.ond",
                text: "package main\nfunc Clash() {}\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate top-level declaration"),
            location: Location::Source {
                file: "z.ond",
                text: "Clash",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "NAME-01.package_type_type",
        specs: &["NAME-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\ntype Clash i32\n",
            },
            SourceFile {
                path: "z.ond",
                text: "package main\ntype Clash i32\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("duplicate top-level declaration"),
            location: Location::Source {
                file: "z.ond",
                text: "Clash",
                occurrence: 0,
            },
        }]),
    },
];
#[test]
fn package_var_var() {
    run(&CASES[0]).unwrap();
}
#[test]
fn package_var_const() {
    run(&CASES[1]).unwrap();
}
#[test]
fn package_var_func() {
    run(&CASES[2]).unwrap();
}
#[test]
fn package_var_type() {
    run(&CASES[3]).unwrap();
}
#[test]
fn package_const_var() {
    run(&CASES[4]).unwrap();
}
#[test]
fn package_const_const() {
    run(&CASES[5]).unwrap();
}
#[test]
fn package_const_func() {
    run(&CASES[6]).unwrap();
}
#[test]
fn package_const_type() {
    run(&CASES[7]).unwrap();
}
#[test]
fn package_func_var() {
    run(&CASES[8]).unwrap();
}
#[test]
fn package_func_const() {
    run(&CASES[9]).unwrap();
}
#[test]
fn package_func_func() {
    run(&CASES[10]).unwrap();
}
#[test]
fn package_func_type() {
    run(&CASES[11]).unwrap();
}
#[test]
fn package_type_var() {
    run(&CASES[12]).unwrap();
}
#[test]
fn package_type_const() {
    run(&CASES[13]).unwrap();
}
#[test]
fn package_type_func() {
    run(&CASES[14]).unwrap();
}
#[test]
fn package_type_type() {
    run(&CASES[15]).unwrap();
}
