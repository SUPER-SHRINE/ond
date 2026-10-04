//! Grammar assertions are independent of type checking and constant evaluation.
use super::*;

fn shape(expr: &ast::Expr) -> String {
    use ast::{Expr, Literal};
    match expr {
        Expr::Literal(Literal::Int(s, _) | Literal::Float(s, _)) => s.clone(),
        Expr::Literal(Literal::Bool(b, _)) => b.to_string(),
        Expr::Name(p) => p
            .segments
            .iter()
            .map(|n| n.name.as_str())
            .collect::<Vec<_>>()
            .join("."),
        Expr::Unary { op, expr, .. } => format!("{op:?}({})", shape(expr)),
        Expr::Binary { op, lhs, rhs, .. } => format!("{op:?}({},{})", shape(lhs), shape(rhs)),
        Expr::Cast { expr, .. } => format!("as({})", shape(expr)),
        Expr::Call { callee, .. } => format!("call({})", shape(callee)),
        Expr::Index { base, index, .. } => format!("index({},{})", shape(base), shape(index)),
        Expr::Selector { base, field, .. } => format!("field({},{})", shape(base), field.name),
        other => format!("unexpected:{other:?}"),
    }
}

fn shapes(project: &ast::Project) -> Result<(), String> {
    let expressions: Vec<_> = project.packages[0].files[0]
        .decls
        .iter()
        .flat_map(|decl| {
            let ast::TopLevelDecl::Const(decl) = decl else {
                panic!("expected const");
            };
            decl.specs.iter().flat_map(|spec| spec.values.iter())
        })
        .collect();
    let expected = &[
        "OrOr(true,AndAnd(false,false))",
        "AndAnd(true,Eq(1,1))",
        "Eq(1,Add(2,3))",
        "Add(1,Mul(2,3))",
        "Mul(2,Minus(3))",
        "Minus(as(1))",
        "Add(1,as(2))",
        "as(as(field(index(call(f),0),X)))",
        "OrOr(OrOr(true,false),false)",
        "AndAnd(AndAnd(true,false),false)",
        "Eq(Lt(1,2),true)",
        "BitXor(BitOr(Add(Sub(8,3),2),1),4)",
        "BitAndNot(BitAnd(ShiftRight(ShiftLeft(Rem(Mul(Div(32,2),3),5),1),1),7),2)",
        "Not(Minus(BitNot(x)))",
        "Mul(Add(1,2),3)",
    ];
    if expressions.len() != expected.len() {
        return Err("expression count mismatch".into());
    }
    for (i, (expr, expected)) in expressions.iter().zip(expected).enumerate() {
        let actual = shape(expr);
        if actual != *expected {
            return Err(format!("expression {i}: expected {expected}, got {actual}"));
        }
    }
    Ok(())
}

pub(super) const CASES: &[Case] = &[
    Case {
        id: "SYN-07.precedence-associativity.ast",
        specs: &["SYN-07"],
        layer: Layer::Parse,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst E0 = true || false && false\nconst E1 = true && 1 == 1\nconst E2 = 1 == 2 + 3\nconst E3 = 1 + 2 * 3\nconst E4 = 2 * -3\nconst E5 = -1 as i32\nconst E6 = 1 + 2 as i32\nconst E7 = f()[0].X as i32 as u32\nconst E8 = true || false || false\nconst E9 = true && false && false\nconst E10 = 1 < 2 == true\nconst E11 = 8 - 3 + 2 | 1 ^ 4\nconst E12 = 32 / 2 * 3 % 5 << 1 >> 1 & 7 &^ 2\nconst E13 = !-^x\nconst E14 = (1 + 2) * 3\n",
        }],
        expected: Expected::Accept(&[Check::Ast {
            description: "seven levels, left associativity and postfix chain",
            verify: shapes,
        }]),
    },
    Case {
        id: "SYN-07.precedence-values.core",
        specs: &["SYN-07"],
        layer: Layer::Core,
        manifest: true,
        files: &[SourceFile {
            path: "main.ond",
            text: "package main\nconst E0 = true || false && false\nconst E1 = true && 1 == 1\nconst E2 = 1 == 2 + 3\nconst E3 = 1 + 2 * 3\nconst E4 = 2 * -3\nconst E5 = -1 as i32\nconst E6 = 1 + 2 as i32\nconst E7 = true || false || false\nconst E8 = true && false && false\nconst E9 = 1 < 2 == true\nconst E10 = 8 - 3 + 2 | 1 ^ 4\nconst E11 = 32 / 2 * 3 % 5 << 1 >> 1 & 7 &^ 2\nconst E12 = (1 + 2) * 3\nfunc main() {}\n",
        }],
        expected: Expected::Accept(&[
            Check::HirConstant {
                package: ".",
                name: "E0",
                ty: "bool",
                value: Scalar::Bool(true),
            },
            Check::HirConstant {
                package: ".",
                name: "E1",
                ty: "bool",
                value: Scalar::Bool(true),
            },
            Check::HirConstant {
                package: ".",
                name: "E2",
                ty: "bool",
                value: Scalar::Bool(false),
            },
            Check::HirConstant {
                package: ".",
                name: "E3",
                ty: "i32",
                value: Scalar::Integer(7),
            },
            Check::HirConstant {
                package: ".",
                name: "E4",
                ty: "i32",
                value: Scalar::Integer(18446744073709551610),
            },
            Check::HirConstant {
                package: ".",
                name: "E5",
                ty: "i32",
                value: Scalar::Integer(18446744073709551615),
            },
            Check::HirConstant {
                package: ".",
                name: "E6",
                ty: "i32",
                value: Scalar::Integer(3),
            },
            Check::HirConstant {
                package: ".",
                name: "E7",
                ty: "bool",
                value: Scalar::Bool(true),
            },
            Check::HirConstant {
                package: ".",
                name: "E8",
                ty: "bool",
                value: Scalar::Bool(false),
            },
            Check::HirConstant {
                package: ".",
                name: "E9",
                ty: "bool",
                value: Scalar::Bool(true),
            },
            Check::HirConstant {
                package: ".",
                name: "E10",
                ty: "i32",
                value: Scalar::Integer(3),
            },
            Check::HirConstant {
                package: ".",
                name: "E11",
                ty: "i32",
                value: Scalar::Integer(1),
            },
            Check::HirConstant {
                package: ".",
                name: "E12",
                ty: "i32",
                value: Scalar::Integer(9),
            },
        ]),
    },
];
#[test]
fn precedence_ast() {
    run(&CASES[0]).unwrap();
}
#[test]
fn precedence_values() {
    run(&CASES[1]).unwrap();
}
