//! Project layout, package-wide imports and entry-point contracts.
use super::*;

pub(super) const CASES: &[Case] = &[
    Case {
        id: "PKG-01.missing_manifest",
        specs: &["PKG-01"],
        layer: Layer::Core,
        manifest: false,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("does not contain `ond.toml`"),
            location: Location::Project,
        }]),
    },
    Case {
        id: "PKG-01.empty_project",
        specs: &["PKG-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[],
        expected: Expected::Reject(&[
            ExpectedDiagnostic {
                severity: Severity::Error,
                message: Message::Exact("project root must contain `main.ond`"),
                location: Location::Project,
            },
            ExpectedDiagnostic {
                severity: Severity::Error,
                message: Message::Exact("`main` package must define `func main()`"),
                location: Location::Project,
            },
        ]),
    },
    Case {
        id: "PKG-01.root_package_mismatch",
        specs: &["PKG-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package other\nfunc main() {}\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Exact("package `.` must declare `package main`"),
            location: Location::Source {
                file: "main.ond",
                text: "package other",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "PKG-01.directory_package_mismatch",
        specs: &["PKG-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\n",
            },
            SourceFile {
                path: "lib/a.ond",
                text: "package wrong\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Exact("package `lib` must declare `package lib`"),
            location: Location::Source {
                file: "lib/a.ond",
                text: "package wrong",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "PKG-01.mixed_package_clauses",
        specs: &["PKG-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\n",
            },
            SourceFile {
                path: "lib/a.ond",
                text: "package lib\n",
            },
            SourceFile {
                path: "lib/b.ond",
                text: "package wrong\n",
            },
        ],
        expected: Expected::Reject(&[
            ExpectedDiagnostic {
                severity: Severity::Error,
                message: Message::Exact("package `lib` must declare `package lib`"),
                location: Location::Source {
                    file: "lib/b.ond",
                    text: "package wrong",
                    occurrence: 0,
                },
            },
            ExpectedDiagnostic {
                severity: Severity::Error,
                message: Message::Exact(
                    "all files in package `lib` must use the same package clause",
                ),
                location: Location::Source {
                    file: "lib/a.ond",
                    text: "package lib",
                    occurrence: 0,
                },
            },
        ]),
    },
    Case {
        id: "PKG-01.empty_source",
        specs: &["PKG-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Exact("expected `package`"),
            location: Location::EndOfFile { file: "main.ond" },
        }]),
    },
    Case {
        id: "PKG-01.declaration_free_package",
        specs: &["PKG-01"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nimport \"lib\"\nfunc main() {}\n",
            },
            SourceFile {
                path: "lib/empty.ond",
                text: "package lib\n",
            },
        ],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "PKG-02.missing_import",
        specs: &["PKG-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nimport \"missing\"\nfunc main() {}\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Exact("unknown import path `missing`"),
            location: Location::Source {
                file: "main.ond",
                text: "import \"missing\"",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "PKG-02.root_import",
        specs: &["PKG-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\n",
            },
            SourceFile {
                path: "lib/a.ond",
                text: "package lib\nimport \".\"\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Exact("project root `main` package cannot be imported"),
            location: Location::Source {
                file: "lib/a.ond",
                text: "import \".\"",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "PKG-02.self_cycle",
        specs: &["PKG-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\n",
            },
            SourceFile {
                path: "lib/a.ond",
                text: "package lib\nimport \"lib\"\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Exact("import cycle detected: lib -> lib"),
            location: Location::Project,
        }]),
    },
    Case {
        id: "PKG-02.multi_cycle",
        specs: &["PKG-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\n",
            },
            SourceFile {
                path: "a/a.ond",
                text: "package a\nimport \"b\"\n",
            },
            SourceFile {
                path: "b/b.ond",
                text: "package b\nimport \"c\"\n",
            },
            SourceFile {
                path: "c/c.ond",
                text: "package c\nimport \"a\"\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Exact("import cycle detected: a -> b -> c -> a"),
            location: Location::Project,
        }]),
    },
    Case {
        id: "PKG-02.invalid_path_parent",
        specs: &["PKG-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nimport \"../lib\"\nfunc main() {}\n",
            },
            SourceFile {
                path: "lib/a.ond",
                text: "package lib\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("unknown import path"),
            location: Location::Source {
                file: "main.ond",
                text: "import \"../lib\"",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "PKG-02.invalid_path_absolute",
        specs: &["PKG-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nimport \"/lib\"\nfunc main() {}\n",
            },
            SourceFile {
                path: "lib/a.ond",
                text: "package lib\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("unknown import path"),
            location: Location::Source {
                file: "main.ond",
                text: "import \"/lib\"",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "PKG-02.invalid_path_dot",
        specs: &["PKG-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nimport \"./lib\"\nfunc main() {}\n",
            },
            SourceFile {
                path: "lib/a.ond",
                text: "package lib\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("unknown import path"),
            location: Location::Source {
                file: "main.ond",
                text: "import \"./lib\"",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "PKG-02.invalid_path_empty",
        specs: &["PKG-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nimport \"\"\nfunc main() {}\n",
            },
            SourceFile {
                path: "lib/a.ond",
                text: "package lib\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("unknown import path"),
            location: Location::Source {
                file: "main.ond",
                text: "import \"\"",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "PKG-02.invalid_path_trailing",
        specs: &["PKG-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nimport \"lib/\"\nfunc main() {}\n",
            },
            SourceFile {
                path: "lib/a.ond",
                text: "package lib\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("unknown import path"),
            location: Location::Source {
                file: "main.ond",
                text: "import \"lib/\"",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "PKG-02.invalid_path_backslash",
        specs: &["PKG-02"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nimport \"lib\\\\sub\"\nfunc main() {}\n",
            },
            SourceFile {
                path: "lib/a.ond",
                text: "package lib\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("unknown import path"),
            location: Location::Source {
                file: "main.ond",
                text: "import \"lib\\\\sub\"",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "PKG-03.file_scoped_imports",
        specs: &["PKG-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nimport \"nested/lib\"\nvar G: lib.T = lib.C as lib.T\nconst C = lib.C\nfunc main() { G = lib.G; }\n",
            },
            SourceFile {
                path: "z.ond",
                text: "package main\nimport \"nested/lib\"\n",
            },
            SourceFile {
                path: "zz.ond",
                text: "package main\nimport \"nested/lib\"\n",
            },
            SourceFile {
                path: "zzz.ond",
                text: "package main\nimport \"nested/lib\"\nconst Later = lib.C\nfunc later() -> lib.T { return lib.G; }\n",
            },
            SourceFile {
                path: "nested/lib/a.ond",
                text: "package lib\ntype T i32\nconst C = 7\nvar G: T = 3 as T\n",
            },
        ],
        expected: Expected::Accept(&[
            Check::HirConstant {
                package: ".",
                name: "C",
                ty: "i32",
                value: Scalar::Integer(7),
            },
            Check::HirConstant {
                package: ".",
                name: "Later",
                ty: "i32",
                value: Scalar::Integer(7),
            },
        ]),
    },
    Case {
        id: "PKG-03.same_alias_in_distinct_files",
        specs: &["PKG-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nimport \"a/lib\"\nfunc main() {}\n",
            },
            SourceFile {
                path: "z.ond",
                text: "package main\nimport \"b/lib\"\n",
            },
            SourceFile {
                path: "a/lib/a.ond",
                text: "package lib\n",
            },
            SourceFile {
                path: "b/lib/b.ond",
                text: "package lib\n",
            },
        ],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "INIT-03.missing_main",
        specs: &["INIT-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Exact("`main` package must define `func main()`"),
            location: Location::Project,
        }]),
    },
    Case {
        id: "INIT-03.other_package_main_is_not_entry",
        specs: &["INIT-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\n",
            },
            SourceFile {
                path: "lib/a.ond",
                text: "package lib\nfunc main() {}\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Exact("`main` package must define `func main()`"),
            location: Location::Project,
        }]),
    },
    Case {
        id: "INIT-03.other_package_main_allowed",
        specs: &["INIT-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\n",
            },
            SourceFile {
                path: "lib/a.ond",
                text: "package lib\nfunc main(x: i32) -> i32 { return x; }\n",
            },
        ],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "INIT-03.duplicate_main",
        specs: &["INIT-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\n",
            },
            SourceFile {
                path: "z.ond",
                text: "package main\nfunc main() {}\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Exact("duplicate top-level declaration `main`"),
            location: Location::Source {
                file: "z.ond",
                text: "main",
                occurrence: 1,
            },
        }]),
    },
    Case {
        id: "INIT-03.main_type_conflict",
        specs: &["INIT-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype main i32\nfunc main() {}\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Exact("duplicate top-level declaration `main`"),
            location: Location::Source {
                file: "main.ond",
                text: "main",
                occurrence: 2,
            },
        }]),
    },
    Case {
        id: "INIT-03.main_parameter",
        specs: &["INIT-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main(x: i32) {  }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Exact("`func main()` must not have parameters or return values"),
            location: Location::Source {
                file: "main.ond",
                text: "func main(x: i32) {  }",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "INIT-03.main_return",
        specs: &["INIT-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() -> i32 { return 0; }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Exact("`func main()` must not have parameters or return values"),
            location: Location::Source {
                file: "main.ond",
                text: "func main() -> i32 { return 0; }",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "INIT-03.main_both",
        specs: &["INIT-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main(x: i32) -> i32 { return x; }\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Exact("`func main()` must not have parameters or return values"),
            location: Location::Source {
                file: "main.ond",
                text: "func main(x: i32) -> i32 { return x; }",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "INIT-03.init_empty",
        specs: &["INIT-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc init() {  }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "INIT-03.init_parameter",
        specs: &["INIT-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc init(x: i32) {  }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "INIT-03.init_return",
        specs: &["INIT-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc init() -> i32 { return 0; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "INIT-03.init_both",
        specs: &["INIT-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() {}\nfunc init(x: i32) -> i32 { return x; }\n",
        }],
        expected: Expected::Accept(&[]),
    },
    Case {
        id: "INIT-03.imported_init",
        specs: &["INIT-03"],
        layer: Layer::Core,
        manifest: true,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nfunc main() {}\n",
            },
            SourceFile {
                path: "lib/a.ond",
                text: "package lib\nfunc init() {}\n",
            },
        ],
        expected: Expected::Accept(&[]),
    },
];
#[test]
fn missing_manifest() {
    run(&CASES[0]).unwrap();
}
#[test]
fn empty_project() {
    run(&CASES[1]).unwrap();
}
#[test]
fn root_package_mismatch() {
    run(&CASES[2]).unwrap();
}
#[test]
fn directory_package_mismatch() {
    run(&CASES[3]).unwrap();
}
#[test]
fn mixed_package_clauses() {
    run(&CASES[4]).unwrap();
}
#[test]
fn empty_source() {
    run(&CASES[5]).unwrap();
}
#[test]
fn declaration_free_package() {
    run(&CASES[6]).unwrap();
}
#[test]
fn missing_import() {
    run(&CASES[7]).unwrap();
}
#[test]
fn root_import() {
    run(&CASES[8]).unwrap();
}
#[test]
fn self_cycle() {
    run(&CASES[9]).unwrap();
}
#[test]
fn multi_cycle() {
    run(&CASES[10]).unwrap();
}
#[test]
fn invalid_path_parent() {
    run(&CASES[11]).unwrap();
}
#[test]
fn invalid_path_absolute() {
    run(&CASES[12]).unwrap();
}
#[test]
fn invalid_path_dot() {
    run(&CASES[13]).unwrap();
}
#[test]
fn invalid_path_empty() {
    run(&CASES[14]).unwrap();
}
#[test]
fn invalid_path_trailing() {
    run(&CASES[15]).unwrap();
}
#[test]
fn invalid_path_backslash() {
    run(&CASES[16]).unwrap();
}
#[test]
fn file_scoped_imports() {
    run(&CASES[17]).unwrap();
}
#[test]
fn same_alias_in_distinct_files() {
    run(&CASES[18]).unwrap();
}
#[test]
fn missing_main() {
    run(&CASES[19]).unwrap();
}
#[test]
fn other_package_main_is_not_entry() {
    run(&CASES[20]).unwrap();
}
#[test]
fn other_package_main_allowed() {
    run(&CASES[21]).unwrap();
}
#[test]
fn duplicate_main() {
    run(&CASES[22]).unwrap();
}
#[test]
fn main_type_conflict() {
    run(&CASES[23]).unwrap();
}
#[test]
fn main_parameter() {
    run(&CASES[24]).unwrap();
}
#[test]
fn main_return() {
    run(&CASES[25]).unwrap();
}
#[test]
fn main_both() {
    run(&CASES[26]).unwrap();
}
#[test]
fn init_empty() {
    run(&CASES[27]).unwrap();
}
#[test]
fn init_parameter() {
    run(&CASES[28]).unwrap();
}
#[test]
fn init_return() {
    run(&CASES[29]).unwrap();
}
#[test]
fn init_both() {
    run(&CASES[30]).unwrap();
}
#[test]
fn imported_init() {
    run(&CASES[31]).unwrap();
}
