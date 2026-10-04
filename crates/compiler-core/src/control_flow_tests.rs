use crate::{compile_project, ir::hir, mir::*};

// Execute the scalar MIR subset used below, independently of the AST backend.
fn execute(function: &Function, input: &[u64]) -> Result<Vec<u64>, &'static str> {
    let mut locals = vec![0; function.locals.len()];
    for (id, value) in function.parameters.iter().zip(input) {
        locals[id.0 as usize] = *value;
    }
    let mut values = vec![0; function.values.len()];
    let mut block = function.entry;
    for _ in 0..10000 {
        let current = &function.blocks[block.0 as usize];
        for instruction in &current.instructions {
            let result = match &instruction.kind {
                InstructionKind::Constant(Constant::Integer { bits, .. }) => Some(*bits),
                InstructionKind::Constant(Constant::Bool { value, .. }) => Some(u64::from(*value)),
                InstructionKind::Load {
                    place:
                        Place {
                            base: PlaceBase::Local(id),
                            ..
                        },
                    ..
                } => Some(locals[id.0 as usize]),
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
                    None
                }
                InstructionKind::Check(Check::NonZero { value }) => {
                    if values[value.0 as usize] == 0 {
                        return Err("division by zero");
                    }
                    None
                }
                InstructionKind::Check(Check::SignedDivisionOverflow { .. }) => None,
                InstructionKind::Binary { op, lhs, rhs, .. } => {
                    let a = values[lhs.0 as usize];
                    let b = values[rhs.0 as usize];
                    Some(match op {
                        BinaryOp::Add => a.wrapping_add(b),
                        BinaryOp::Multiply => a.wrapping_mul(b),
                        BinaryOp::Divide => a / b,
                        BinaryOp::Equal => u64::from(a == b),
                        BinaryOp::NotEqual => u64::from(a != b),
                        BinaryOp::LessSigned | BinaryOp::LessUnsigned => u64::from(a < b),
                        BinaryOp::GreaterSigned | BinaryOp::GreaterUnsigned => u64::from(a > b),
                        _ => panic!("unexpected operator {op:?}"),
                    })
                }
                other => panic!("unexpected instruction {other:?}"),
            };
            if let Some(result) = result {
                values[instruction.results[0].0 as usize] = result;
            }
        }
        let edge = match &current.terminator.kind {
            TerminatorKind::Return(result) => {
                return Ok(result.iter().map(|id| values[id.0 as usize]).collect());
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
            _ => return Err("unexpected unreachable/trap"),
        };
        let arguments: Vec<_> = edge
            .arguments
            .iter()
            .map(|id| values[id.0 as usize])
            .collect();
        for (parameter, value) in function.blocks[edge.target.0 as usize]
            .parameters
            .iter()
            .zip(arguments)
        {
            values[parameter.0 as usize] = value;
        }
        block = edge.target;
    }
    Err("step limit")
}

#[test]
fn executes_cfg_scopes_loops_and_short_circuit() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("ond-cfg-{}-{unique}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("ond.toml"), "").unwrap();
    std::fs::write(
        root.join("main.ond"),
        r#"package main
func main() {}
func shortAnd(x: i32) -> bool { return x != 0 && 10 / x > 1 }
func shortOr(x: i32) -> bool { return x == 0 || 10 / x > 1 }
func nested(x: i32) -> bool { return (x == 0 || 10 / x > 1) && (x == 0 || x < 5) }
func loops() -> i32 {
    sum := 0
    for i := 0; i < 5; i = i + 1 {
        if i == 1 { continue }
        for j := 0; j < 4; j = j + 1 {
            if j == 2 { break }
            sum = sum + i
        }
    }
    for sum < 22 { sum = sum + 1 }
    for { break }
    return sum
}
func scopes() -> i32 {
    x := 1
    { x := x + 2
      x = x + 4
    }
    if x := x + 10; x == 11 { x = x + 1 } else { x = 0 }
    for x := 0; x < 2; x = x + 1 { }
    return x
}
func branches(x: i32) -> i32 {
    if x == 0 { return 7 } else if x == 1 { return 8 } else { return 9 }
}
func swap(x: i32) -> i32 {
    a := false
    b := true
    a, b = b || x == 0, a && 10 / x > 1
    if a && b { return 1 }
    return 2
}
"#
        .replace(" }", "\n}"),
    )
    .unwrap();
    let compilation = compile_project(&root).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    for package in &compilation.hir.packages {
        for item in &package.items {
            if let hir::Item::Func(function) = item {
                assert!(
                    matches!(function.body, hir::Body::Typed(_)),
                    "{} remained pending",
                    function.name
                );
            }
        }
    }
    validate_project(&compilation.mir).unwrap();
    let run = |name: &str, args: &[u64]| {
        let function = compilation.mir.packages[0]
            .functions
            .iter()
            .find(|f| f.name == format!("main.{name}"))
            .unwrap();
        execute(function, args)
    };
    assert_eq!(run("shortAnd", &[0]), Ok(vec![0]));
    assert_eq!(run("shortOr", &[0]), Ok(vec![1]));
    assert_eq!(run("shortAnd", &[2]), Ok(vec![1]));
    assert_eq!(run("shortOr", &[10]), Ok(vec![0]));
    assert_eq!(run("nested", &[0]), Ok(vec![1]));
    assert_eq!(run("nested", &[5]), Ok(vec![0]));
    assert_eq!(run("loops", &[]), Ok(vec![22]));
    assert_eq!(run("scopes", &[]), Ok(vec![1]));
    for (input, result) in [(0, 7), (1, 8), (2, 9)] {
        assert_eq!(run("branches", &[input]), Ok(vec![result]));
    }
    assert_eq!(run("swap", &[0]), Ok(vec![2]));
}
