//! Small seed registry; existing regression suites stay where they are.
use super::*;

fn unnamed_parameter(ast: &ast::Project) -> Result<(), String> {
    let ast::TopLevelDecl::Type(declaration) = &ast.packages[0].files[0].decls[0] else {
        return Err("expected type declaration".into());
    };
    let ast::Type::Func { signature, .. } = &declaration.specs[0].ty else {
        return Err("expected function type".into());
    };
    if signature.params.len() == 1 && signature.params[0].name.is_none() {
        Ok(())
    } else {
        Err("expected one unnamed parameter".into())
    }
}

fn folded_return(project: &mir::Project) -> Result<(), String> {
    let function = project
        .packages
        .iter()
        .flat_map(|p| &p.functions)
        .find(|f| f.name == "main.result")
        .ok_or("missing main.result")?;
    let operations: Vec<_> = function
        .blocks
        .iter()
        .flat_map(|b| &b.instructions)
        .collect();
    if operations
        .iter()
        .any(|i| matches!(i.kind, mir::InstructionKind::Binary { .. }))
    {
        return Err("constant arithmetic survived into MIR".into());
    }
    if !operations.iter().any(|i| {
        matches!(
            i.kind,
            mir::InstructionKind::Constant(mir::Constant::Integer { bits: 42, .. })
        )
    }) {
        return Err("missing folded 42".into());
    }
    Ok(())
}

pub(super) const CASES: &[Case] = &[
    Case {
        id: "SYN-11.unnamed-parameter.ast",
        specs: &["SYN-11"],
        manifest: true,
        layer: Layer::Parse,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\ntype F func(i32) -> i32\n",
        }],
        expected: Expected::Accept(&[Check::Ast {
            description: "one unnamed parameter, not a blank binding",
            verify: unnamed_parameter,
        }]),
    },
    Case {
        id: "SYN-12.empty-results.reject",
        specs: &["SYN-12"],
        manifest: true,
        layer: Layer::Parse,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nfunc main() -> () {}\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Exact(
                "empty result list is not allowed; omit `->` for zero return values",
            ),
            location: Location::Source {
                file: "main.ond",
                text: ")",
                occurrence: 1,
            },
        }]),
    },
    Case {
        id: "NUM-02.imported-constant.core",
        specs: &["NUM-02", "PKG-03", "FP-01", "NIL-01", "STR-01"],
        manifest: true,
        layer: Layer::Core,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\nimport \"shared\"\nconst Answer = shared.Base * 2\nconst Flag = true\nconst Text = \"音\"\nconst Pointer: *u8 = nil\nconst F = -0.0\nfunc main() {}\nfunc result() -> i32 { return Answer; }\n",
            },
            SourceFile {
                path: "shared/shared.ond",
                text: "package shared\nconst Base = 21\n",
            },
        ],
        expected: Expected::Accept(&[
            Check::HirConstant {
                package: ".",
                name: "Answer",
                ty: "i32",
                value: Scalar::Integer(42),
            },
            Check::HirConstant {
                package: ".",
                name: "Flag",
                ty: "bool",
                value: Scalar::Bool(true),
            },
            Check::HirConstant {
                package: ".",
                name: "Text",
                ty: "[3]u8",
                value: Scalar::Bytes("音".as_bytes()),
            },
            Check::HirConstant {
                package: ".",
                name: "Pointer",
                ty: "*u8",
                value: Scalar::Nil,
            },
            Check::HirConstant {
                package: ".",
                name: "F",
                ty: "f32",
                value: Scalar::Float32(0x80000000),
            },
            Check::Mir {
                description: "folded main.result contains 42 and no binary operation",
                verify: folded_return,
            },
        ]),
    },
    Case {
        id: "NAME-03.unicode-multifile.reject",
        specs: &["NAME-03", "DIAG-02"],
        manifest: true,
        layer: Layer::Core,
        files: &[
            SourceFile {
                path: "main.ond",
                text: "package main\n// i32\nfunc main() {}\n",
            },
            SourceFile {
                path: "other.ond",
                text: "package main\n// 音のコメント\nfunc f() { var i32 = 1; }\n",
            },
        ],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Contains("builtin type name `i32`"),
            location: Location::Source {
                file: "other.ond",
                text: "i32",
                occurrence: 0,
            },
        }]),
    },
    Case {
        id: "PKG-01.missing-main-file.reject",
        specs: &["PKG-01"],
        manifest: true,
        layer: Layer::Core,
        files: &[SourceFile {
            path: "other.ond",
            text: "package main\nfunc main() {}\n",
        }],
        expected: Expected::Reject(&[ExpectedDiagnostic {
            severity: Severity::Error,
            message: Message::Exact("project root must contain `main.ond`"),
            location: Location::Project,
        }]),
    },
];

#[test]
fn registry_is_valid() {
    let all: Vec<_> = [
        CASES,
        scope::CASES,
        precedence::CASES,
        constant_contexts::CASES,
        &evaluation::CASES,
        flow::CASES,
        names::CASES,
        name_duplicates::CASES,
        control_headers::CASES,
        diagnostics::CASES,
    ]
    .into_iter()
    .flatten()
    .copied()
    .collect();
    validate_registry(&all).unwrap();
    let mut ids: std::collections::BTreeSet<_> = all.iter().map(|c| c.id.to_string()).collect();
    for id in numeric::ids().into_iter().chain(boundaries::ids()) {
        assert!(ids.insert(id.clone()), "duplicate generated case ID: {id}");
    }
}

macro_rules! case_test {
    ($name:ident, $index:expr) => {
        #[test]
        fn $name() {
            run(&CASES[$index]).unwrap();
        }
    };
}
case_test!(unnamed_parameter_ast, 0);
case_test!(empty_results_rejected, 1);
case_test!(imported_constants_hir_and_mir, 2);
case_test!(unicode_multifile_diagnostic, 3);
case_test!(missing_main_project_diagnostic, 4);
