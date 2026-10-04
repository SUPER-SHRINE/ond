use crate::{ir::hir, mir::*, test_support::compile};

fn checked(source: &str, shared: Option<&str>) -> crate::Compilation {
    let compilation = compile(source, shared).unwrap();
    validate_project(&compilation.mir).unwrap();
    compilation
}

// Small test-only executor: call trace proves order and single evaluation,
// independently of the legacy AST backend. Function pointers use table indices.
fn execute(project: &Project, name: &str, input: &[u64], trace: &mut Vec<String>) -> Vec<u64> {
    assert!(trace.len() < 100, "unexpected recursion");
    trace.push(name.to_string());
    let functions = project
        .packages
        .iter()
        .flat_map(|p| &p.functions)
        .collect::<Vec<_>>();
    let function = functions.iter().find(|f| f.name == name).unwrap();
    let mut locals = vec![0; function.locals.len()];
    for (id, value) in function.parameters.iter().zip(input) {
        locals[id.0 as usize] = *value;
    }
    let mut values = vec![0; function.values.len()];
    let mut block = function.entry;
    for _ in 0..1000 {
        let current = &function.blocks[block.0 as usize];
        for instruction in &current.instructions {
            let results = match &instruction.kind {
                InstructionKind::Constant(Constant::Integer { bits, .. }) => vec![*bits],
                InstructionKind::Constant(Constant::Bool { value, .. }) => vec![u64::from(*value)],
                InstructionKind::Constant(Constant::Function { symbol, .. }) => {
                    vec![functions.iter().position(|f| f.name == *symbol).unwrap() as u64 + 1]
                }
                InstructionKind::Constant(Constant::Nil(_)) => vec![0],
                InstructionKind::Load {
                    place:
                        Place {
                            base: PlaceBase::Local(id),
                            ..
                        },
                    ..
                } => vec![locals[id.0 as usize]],
                InstructionKind::Store {
                    place:
                        Place {
                            base: PlaceBase::Local(id),
                            ..
                        },
                    value,
                    ..
                } => {
                    locals[id.0 as usize] = values[value.0 as usize];
                    vec![]
                }
                InstructionKind::Binary { op, lhs, rhs, .. } => {
                    let a = values[lhs.0 as usize];
                    let b = values[rhs.0 as usize];
                    vec![match op {
                        BinaryOp::Add => a.wrapping_add(b),
                        BinaryOp::Subtract => a.wrapping_sub(b),
                        BinaryOp::Multiply => a.wrapping_mul(b),
                        BinaryOp::Equal => u64::from(a == b),
                        BinaryOp::LessSigned => u64::from((a as i32) < b as i32),
                        other => panic!("unsupported test operator {other:?}"),
                    }]
                }
                InstructionKind::Call {
                    callee,
                    arguments,
                    effects,
                    ..
                } => {
                    assert_eq!(*effects, CallEffects::Unknown);
                    let target = match callee {
                        Callee::Intrinsic(_) => panic!("intrinsic outside test subset"),
                        Callee::Direct(name) => name.as_str(),
                        Callee::Indirect(id) => {
                            let pointer = values[id.0 as usize];
                            assert_ne!(pointer, 0, "nil function");
                            functions[pointer as usize - 1].name.as_str()
                        }
                    };
                    let arguments = arguments
                        .iter()
                        .map(|id| values[id.0 as usize])
                        .collect::<Vec<_>>();
                    execute(project, target, &arguments, trace)
                }
                other => panic!("unsupported test instruction {other:?}"),
            };
            assert_eq!(instruction.results.len(), results.len());
            for (id, result) in instruction.results.iter().zip(results) {
                values[id.0 as usize] = result;
            }
        }
        let edge = match &current.terminator.kind {
            TerminatorKind::Return(result) => {
                return result.iter().map(|id| values[id.0 as usize]).collect();
            }
            TerminatorKind::Jump(edge) => edge,
            TerminatorKind::Branch {
                condition,
                then_edge,
                else_edge,
            } => {
                if values[condition.0 as usize] != 0 {
                    then_edge
                } else {
                    else_edge
                }
            }
            other => panic!("unexpected terminator {other:?}"),
        };
        let arguments = edge
            .arguments
            .iter()
            .map(|id| values[id.0 as usize])
            .collect::<Vec<_>>();
        for (id, value) in function.blocks[edge.target.0 as usize]
            .parameters
            .iter()
            .zip(arguments)
        {
            values[id.0 as usize] = value;
        }
        block = edge.target;
    }
    panic!("step limit")
}

#[test]
fn multiple_results_are_evaluated_once_and_forwarded() {
    let c = checked(
        r#"package main
func main() {}
func pair(x: i32) -> (i32, i32) { return x, x + 1; }
func forward(x: i32) -> (i32, i32) { return pair(x); }
func run() -> (i32, i32) {
    var a, b = pair(3)
    a, b = b, a
    a, c := forward(a)
    var d, e: i32 = pair(c)
    a, b = pair(d + e)
    return a, b
}
"#,
        None,
    );
    let mut trace = Vec::new();
    assert_eq!(execute(&c.mir, "main.run", &[], &mut trace), vec![11, 12]);
    assert_eq!(
        trace,
        [
            "main.run",
            "main.pair",
            "main.forward",
            "main.pair",
            "main.pair",
            "main.pair"
        ]
    );
    let f = c.hir.packages[0]
        .items
        .iter()
        .find_map(|i| match i {
            hir::Item::Func(f) if f.name == "forward" => Some(f),
            _ => None,
        })
        .unwrap();
    let hir::Body::Typed(body) = &f.body;
    assert!(matches!(
        &body.block.statements[0],
        hir::Stmt::Return {
            values: hir::ValueList::Call(_),
            ..
        }
    ));
}

#[test]
fn calls_support_imports_forward_references_and_mutual_recursion() {
    let c = checked(
        r#"package main
import "shared"
func main() {}
func run() -> i32 { return shared.Double(later(3)); }
func later(x: i32) -> i32 { return x + 1; }
func even(x: i32) -> bool {
    if x == 0 { return true; }
    return odd(x - 1)
}
func odd(x: i32) -> bool {
    if x == 0 { return false; }
    return even(x - 1)
}
"#,
        Some("package shared\nfunc Double(x: i32) -> i32 { return x * 2; }\n"),
    );
    assert_eq!(execute(&c.mir, "main.run", &[], &mut Vec::new()), vec![8]);
    assert_eq!(execute(&c.mir, "main.even", &[4], &mut Vec::new()), vec![1]);
    assert_eq!(execute(&c.mir, "main.even", &[3], &mut Vec::new()), vec![0]);
}

#[test]
fn indirect_calls_evaluate_callee_then_arguments_and_keep_short_circuit() {
    let c = checked(
        r#"package main
func main() {}
func pair(a: i32, b: i32) -> (i32, i32) { return a, b; }
func choose() -> func(a: i32, b: i32) -> (i32, i32) { return pair; }
func left() -> i32 { return 7; }
func right() -> i32 { return 9; }
func yes() -> bool { return true; }
func run() -> (i32, i32) {
    var skipped = false && yes()
    return choose()(left(), right())
}
func through(f: func(a: i32, b: i32) -> (i32, i32)) -> (i32, i32) { return f(1, 2); }
func local() -> (i32, i32) {
    var f = pair
    return through(f)
}
"#,
        None,
    );
    let mut trace = Vec::new();
    assert_eq!(execute(&c.mir, "main.run", &[], &mut trace), vec![7, 9]);
    assert_eq!(
        trace,
        [
            "main.run",
            "main.choose",
            "main.left",
            "main.right",
            "main.pair"
        ]
    );
    assert_eq!(
        execute(&c.mir, "main.local", &[], &mut Vec::new()),
        vec![1, 2]
    );
}

#[test]
fn statement_calls_discard_zero_one_or_many_results() {
    let c = checked(
        r#"package main
func nothing() {}
func one() -> i32 { return 1; }
func pair() -> (i32, bool) { return 1, true; }
func main() { nothing()
    one()
    pair()
}
"#,
        None,
    );
    let mut trace = Vec::new();
    assert!(execute(&c.mir, "main.main", &[], &mut trace).is_empty());
    assert_eq!(
        trace,
        ["main.main", "main.nothing", "main.one", "main.pair"]
    );
}

#[test]
fn call_errors_are_source_diagnostics_not_pending() {
    let prelude = "package main\nfunc pair() -> (i32, bool) { return 1, true; }\nfunc one(x: i32) -> i32 { return x; }\nfunc nothing() {}\n";
    for (body, expected) in [
        ("one()", "function expects 1 arguments, found 0"),
        ("one(true)", "argument 1 type mismatch"),
        ("var x = nothing()", "value count mismatch"),
        ("var x = pair()", "value count mismatch"),
        ("var a, b: i32 = pair()", "value 2 type mismatch"),
        ("var x = 1 + pair()", "single-value expression"),
        ("one(pair())", "single-value expression"),
        ("var pair = 1\npair()", "not a function"),
        (
            "var a: i32\nvar b: i32\na, b = pair()",
            "value 2 type mismatch",
        ),
        ("var a: bool\na, b := pair()", "value 1 type mismatch"),
        ("var a, b, c = pair(), 1, 2", "single-value expression"),
        ("missing()", "unknown identifier"),
    ] {
        let error = compile(&format!("{prelude}func main() {{\n{body}\n}}\n"), None).unwrap_err();
        assert!(error.to_string().contains(expected), "{body}: {error}");
        assert!(!error.diagnostics()[0].primary.span.is_synthetic());
    }
    let error = compile(
        &format!("{prelude}func main() {{}}\nfunc bad() -> (i32, i32) {{ return pair(); }}\n"),
        None,
    )
    .unwrap_err();
    assert!(error.to_string().contains("return value 2 type mismatch"));
}

#[test]
fn lexical_bindings_cannot_shadow_builtin_names() {
    crate::test_support::reject(
        "package main\nfunc identity(x: i32) -> i32 { return x; }\nfunc main() { var len = identity; }\n",
        None,
        "builtin name `len`",
        "len",
    );
}
