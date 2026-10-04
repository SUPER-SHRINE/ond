//! Dynamic operations are checked structurally, never by the legacy AST backend.
use super::*;
use mir::{BinaryOp as B, InstructionKind as I};

// tag, spelling, signed opcode, unsigned opcode, boolean result, shift count
const OPS: &[(&str, &str, B, B, bool, bool)] = &[
    ("add", "+", B::Add, B::Add, false, false),
    ("sub", "-", B::Subtract, B::Subtract, false, false),
    ("mul", "*", B::Multiply, B::Multiply, false, false),
    ("div", "/", B::Divide, B::Divide, false, false),
    ("rem", "%", B::Remainder, B::Remainder, false, false),
    ("and", "&", B::BitAnd, B::BitAnd, false, false),
    ("or", "|", B::BitOr, B::BitOr, false, false),
    ("xor", "^", B::BitXor, B::BitXor, false, false),
    ("andnot", "&^", B::BitAndNot, B::BitAndNot, false, false),
    ("left", "<<", B::ShiftLeft, B::ShiftLeft, false, true),
    (
        "right",
        ">>",
        B::ShiftRightArithmetic,
        B::ShiftRightLogical,
        false,
        true,
    ),
    ("eq", "==", B::Equal, B::Equal, true, false),
    ("ne", "!=", B::NotEqual, B::NotEqual, true, false),
    ("lt", "<", B::LessSigned, B::LessUnsigned, true, false),
    (
        "le",
        "<=",
        B::LessEqualSigned,
        B::LessEqualUnsigned,
        true,
        false,
    ),
    ("gt", ">", B::GreaterSigned, B::GreaterUnsigned, true, false),
    (
        "ge",
        ">=",
        B::GreaterEqualSigned,
        B::GreaterEqualUnsigned,
        true,
        false,
    ),
];
const FLOAT_OPS: &[(&str, &str, B, bool)] = &[
    ("add", "+", B::FloatAdd, false),
    ("sub", "-", B::FloatSubtract, false),
    ("mul", "*", B::FloatMultiply, false),
    ("div", "/", B::FloatDivide, false),
    ("eq", "==", B::FloatEqual, true),
    ("ne", "!=", B::FloatNotEqual, true),
    ("lt", "<", B::FloatLess, true),
    ("le", "<=", B::FloatLessEqual, true),
    ("gt", ">", B::FloatGreater, true),
    ("ge", ">=", B::FloatGreaterEqual, true),
];
fn function<'a>(p: &'a mir::Project, name: &str) -> Result<&'a mir::Function, String> {
    p.packages
        .iter()
        .flat_map(|p| &p.functions)
        .find(|f| f.name == format!("main.{name}"))
        .ok_or_else(|| format!("missing {name}"))
}
fn type_name(p: &mir::Project, f: &mir::Function, id: mir::ValueId) -> String {
    p.types.display(f.values[id.0 as usize].ty).to_string()
}
fn parameter_value(f: &mir::Function, value: mir::ValueId, index: usize) -> bool {
    f.blocks.iter().flat_map(|b| &b.instructions).any(|i| {
        i.results == [value] && matches!(&i.kind, I::Load { place, .. }
            if place.projections.is_empty() && matches!(place.base, mir::PlaceBase::Local(id) if Some(&id) == f.parameters.get(index)))
    })
}
fn returns_value(f: &mir::Function, value: mir::ValueId) -> bool {
    f.blocks.iter().any(
        |b| matches!(&b.terminator.kind, mir::TerminatorKind::Return(values) if values == &[value]),
    )
}

fn binary(
    p: &mir::Project,
    name: &str,
    input: &str,
    rhs_type: &str,
    output: &str,
    op: B,
    division: bool,
    signed_division: bool,
) -> Result<(), String> {
    let f = function(p, name)?;
    let instructions: Vec<_> = f.blocks.iter().flat_map(|b| &b.instructions).collect();
    let operations: Vec<_> = instructions
        .iter()
        .enumerate()
        .filter(|(_, i)| matches!(i.kind, I::Binary { .. }))
        .collect();
    if operations.len() != 1 {
        return Err(format!("{name}: expected one dynamic binary"));
    }
    let (pos, i) = operations[0];
    let I::Binary {
        op: actual,
        lhs,
        rhs,
        overflow,
    } = i.kind
    else {
        unreachable!()
    };
    if actual != op
        || !parameter_value(f, lhs, 0)
        || !parameter_value(f, rhs, 1)
        || type_name(p, f, lhs) != input
        || type_name(p, f, rhs) != rhs_type
        || i.results.len() != 1
        || type_name(p, f, i.results[0]) != output
        || !returns_value(f, i.results[0])
        || overflow
            != if division {
                mir::OverflowBehavior::Checked
            } else {
                mir::OverflowBehavior::Wrap
            }
    {
        return Err(format!(
            "{name}: opcode/type/overflow mismatch: {:?}",
            i.kind
        ));
    }
    let checks: Vec<_> = instructions
        .iter()
        .enumerate()
        .filter_map(|(n, i)| {
            if let I::Check(c) = &i.kind {
                Some((n, c))
            } else {
                None
            }
        })
        .collect();
    if checks.len() != usize::from(division) + usize::from(signed_division) {
        return Err(format!("{name}: unexpected guard count"));
    }
    if division
        && !checks
            .iter()
            .any(|(n, c)| *n < pos && **c == mir::Check::NonZero { value: rhs })
    {
        return Err(format!("{name}: missing preceding nonzero guard"));
    }
    if signed_division
        && !checks
            .iter()
            .any(|(n, c)| *n < pos && **c == mir::Check::SignedDivisionOverflow { lhs, rhs })
    {
        return Err(format!("{name}: missing overflow guard"));
    }
    if !division && i.kind.effects().may_trap {
        return Err(format!("{name}: nontrapping operation marked trapping"));
    }
    Ok(())
}
fn verify(p: &mir::Project) -> Result<(), String> {
    for t in INTEGERS {
        for &(tag, _, signed, unsigned, comparison, shift) in OPS {
            binary(
                p,
                &format!("{}_{}", t.name, tag),
                t.name,
                if shift { "u32" } else { t.name },
                if comparison { "bool" } else { t.name },
                if t.min < 0 { signed } else { unsigned },
                matches!(tag, "div" | "rem"),
                tag == "div" && t.min < 0,
            )?;
        }
    }
    for &(tag, _, op, comparison) in FLOAT_OPS {
        binary(
            p,
            &format!("f32_{tag}"),
            "f32",
            "f32",
            if comparison { "bool" } else { "f32" },
            op,
            false,
            false,
        )?;
    }
    for ty in INTEGERS.iter().map(|t| t.name).chain(["f32"]) {
        for (tag, op) in [
            ("positive", None),
            ("negative", Some(mir::UnaryOp::Negate)),
            ("complement", Some(mir::UnaryOp::BitNot)),
        ] {
            if ty == "f32" && tag == "complement" {
                continue;
            }
            let f = function(p, &format!("{ty}_{tag}"))?;
            let unary: Vec<_> = f
                .blocks
                .iter()
                .flat_map(|b| &b.instructions)
                .filter(|i| matches!(i.kind, I::Unary { .. }))
                .collect();
            if unary.len() != usize::from(op.is_some()) {
                return Err(format!("{ty}_{tag}: unary count"));
            }
            if let Some(op) = op {
                let i = unary[0];
                let I::Unary {
                    op: actual,
                    operand,
                } = i.kind
                else {
                    unreachable!()
                };
                if actual != op
                    || !parameter_value(f, operand, 0)
                    || type_name(p, f, operand) != ty
                    || type_name(p, f, i.results[0]) != ty
                    || !returns_value(f, i.results[0])
                {
                    return Err(format!("{ty}_{tag}: unary contract"));
                }
            }
            if op.is_none()
                && !f
                    .values
                    .iter()
                    .any(|v| parameter_value(f, v.id, 0) && returns_value(f, v.id))
            {
                return Err(format!("{ty}_{tag}: unary plus must preserve its input"));
            }
            if f.returns.len() != 1 || p.types.display(f.returns[0]).to_string() != ty {
                return Err("unary return type".into());
            }
        }
    }
    for from in INTEGERS.iter().map(|t| t.name).chain(["f32"]) {
        for to in INTEGERS.iter().map(|t| t.name).chain(["f32"]) {
            if from == "f32" && to == "f32" {
                continue;
            }
            let name = format!("cast_{from}_{to}");
            let f = function(p, &name)?;
            let casts: Vec<_> = f
                .blocks
                .iter()
                .flat_map(|b| &b.instructions)
                .filter(|i| matches!(i.kind, I::Cast { .. }))
                .collect();
            if casts.len() != 1 {
                return Err(format!("{name}: cast count"));
            }
            let i = casts[0];
            let I::Cast {
                value,
                to: dest,
                behavior,
            } = i.kind
            else {
                unreachable!()
            };
            let expected = if from == "f32" {
                mir::CastBehavior::Checked
            } else {
                mir::CastBehavior::Infallible
            };
            if type_name(p, f, value) != from
                || !parameter_value(f, value, 0)
                || p.types.display(dest).to_string() != to
                || type_name(p, f, i.results[0]) != to
                || !returns_value(f, i.results[0])
                || behavior != expected
                || i.kind.effects().may_trap != (from == "f32")
            {
                return Err(format!("{name}: cast type/effect contract"));
            }
        }
    }
    Ok(())
}

pub(super) fn rows() -> Vec<Row> {
    let mut source = "package main\nfunc main() {}\n".to_string();
    for t in INTEGERS {
        for &(tag, spelling, _, _, comparison, shift) in OPS {
            source.push_str(&format!(
                "func {}_{}(a: {}, b: {}) -> {} {{ return a {spelling} b; }}\n",
                t.name,
                tag,
                t.name,
                if shift { "u32" } else { t.name },
                if comparison { "bool" } else { t.name }
            ));
        }
    }
    for &(tag, spelling, _, comparison) in FLOAT_OPS {
        source.push_str(&format!(
            "func f32_{tag}(a: f32, b: f32) -> {} {{ return a {spelling} b; }}\n",
            if comparison { "bool" } else { "f32" }
        ));
    }
    for ty in INTEGERS.iter().map(|t| t.name).chain(["f32"]) {
        for (tag, op) in [("positive", "+"), ("negative", "-"), ("complement", "^")] {
            if ty == "f32" && tag == "complement" {
                continue;
            }
            source.push_str(&format!(
                "func {ty}_{tag}(a: {ty}) -> {ty} {{ return {op}a; }}\n"
            ));
        }
    }
    for from in INTEGERS.iter().map(|t| t.name).chain(["f32"]) {
        for to in INTEGERS.iter().map(|t| t.name).chain(["f32"]) {
            if from != "f32" || to != "f32" {
                source.push_str(&format!(
                    "func cast_{from}_{to}(a: {from}) -> {to} {{ return a as {to}; }}\n"
                ));
            }
        }
    }
    vec![Row {
        id: "FP-02.numeric-dynamic-mir-contracts".into(),
        specs: &["NUM-03", "NUM-04", "NUM-05", "CAST-02", "CAST-03", "FP-02"],
        source,
        expected: Outcome::Mir(verify),
    }]
}

#[test]
fn contract_checker_rejects_wrong_opcode_operands_and_fault_metadata() {
    let c = crate::test_support::compile(&rows()[0].source, None).unwrap();
    verify(&c.mir).unwrap();
    for mutation in 0..4 {
        let mut bad = c.mir.clone();
        let name = match mutation {
            0 | 1 => "main.i8_sub",
            2 => "main.f32_div",
            _ => "main.i8_div",
        };
        let f = bad
            .packages
            .iter_mut()
            .flat_map(|p| &mut p.functions)
            .find(|f| f.name == name)
            .unwrap();
        if mutation == 3 {
            for block in &mut f.blocks {
                block
                    .instructions
                    .retain(|i| !matches!(i.kind, I::Check(_)));
            }
        } else {
            let i = f
                .blocks
                .iter_mut()
                .flat_map(|b| &mut b.instructions)
                .find(|i| matches!(i.kind, I::Binary { .. }))
                .unwrap();
            let I::Binary {
                op,
                lhs,
                rhs,
                overflow,
            } = &mut i.kind
            else {
                unreachable!()
            };
            match mutation {
                0 => *op = B::Add,
                1 => std::mem::swap(lhs, rhs),
                2 => *overflow = mir::OverflowBehavior::Checked,
                _ => unreachable!(),
            }
        }
        let message = verify(&bad).unwrap_err();
        let expected = if mutation == 3 {
            "unexpected guard count"
        } else {
            "opcode/type/overflow mismatch"
        };
        assert!(message.contains(expected), "mutation {mutation}: {message}");
    }
}
