//! Operation-specific contracts and resolved function symbols.
use super::*;
use crate::semantic::Intrinsic;
use std::collections::{BTreeMap, BTreeSet};

fn cast_valid(types: &TypeTable, from: TypeId, to: TypeId) -> bool {
    let numeric =
        |ty| types.is_integer(ty) || matches!(types.underlying_kind(ty), Some(TypeKind::F32));
    (types.underlying_id(from).is_some() && types.underlying_id(from) == types.underlying_id(to))
        || (numeric(from) && numeric(to))
        || (types.is_integer(from)
            && matches!(types.underlying_kind(to), Some(TypeKind::Pointer(_))))
        || (matches!(types.underlying_kind(from), Some(TypeKind::Pointer(_)))
            && types.is_integer(to))
        || (matches!(types.underlying_kind(from), Some(TypeKind::Pointer(_)))
            && matches!(types.underlying_kind(to), Some(TypeKind::Pointer(_))))
}

pub(super) fn function(types: &TypeTable, f: &Function) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let mut error = |block, message: &str| {
        errors.push(ValidationError {
            function: f.name.clone(),
            block,
            message: message.into(),
        })
    };
    let parameter_set = f.parameters.iter().copied().collect::<BTreeSet<_>>();
    if parameter_set.len() != f.parameters.len()
        || f.locals
            .iter()
            .filter(|l| l.kind == LocalKind::Parameter)
            .any(|l| !parameter_set.contains(&l.id))
    {
        error(
            None,
            "parameter locals are duplicated or missing from the signature",
        );
    }
    let ty = |id: ValueId| f.values.get(id.0 as usize).map(|v| v.ty);
    if f.blocks
        .get(f.entry.0 as usize)
        .is_some_and(|b| !b.parameters.is_empty())
    {
        error(Some(f.entry), "entry block must not have block parameters");
    }
    for block in &f.blocks {
        for i in &block.instructions {
            let valid = match &i.kind {
                InstructionKind::Constant(c) => match c {
                    Constant::Zero(_) => true,
                    Constant::Integer { bits, ty } => crate::constant::integer_range(types, *ty)
                        .is_some_and(|(min, max)| {
                            let value = if min < 0 {
                                *bits as i64 as i128
                            } else {
                                i128::from(*bits)
                            };
                            value >= min && value <= max
                        }),
                    Constant::Float32 { ty, .. } => {
                        matches!(types.underlying_kind(*ty), Some(TypeKind::F32))
                    }
                    Constant::Bool { ty, .. } => {
                        matches!(types.underlying_kind(*ty), Some(TypeKind::Bool))
                    }
                    Constant::Nil(ty) => matches!(
                        types.underlying_kind(*ty),
                        Some(TypeKind::Pointer(_) | TypeKind::Function(_))
                    ),
                    Constant::Function { ty, signature, .. } => {
                        matches!(types.underlying_kind(*ty), Some(TypeKind::Function(actual)) if actual == signature)
                    }
                },
                InstructionKind::Unary { op, operand } => ty(*operand).is_some_and(|ty| match op {
                    UnaryOp::Negate => {
                        types.is_integer(ty)
                            || matches!(types.underlying_kind(ty), Some(TypeKind::F32))
                    }
                    UnaryOp::BitNot => types.is_integer(ty),
                    UnaryOp::LogicalNot => {
                        matches!(types.underlying_kind(ty), Some(TypeKind::Bool))
                    }
                }),
                InstructionKind::Binary { op, lhs, .. } => ty(*lhs).is_some_and(|ty| match op {
                    BinaryOp::FloatAdd
                    | BinaryOp::FloatSubtract
                    | BinaryOp::FloatMultiply
                    | BinaryOp::FloatDivide
                    | BinaryOp::FloatEqual
                    | BinaryOp::FloatNotEqual
                    | BinaryOp::FloatLess
                    | BinaryOp::FloatLessEqual
                    | BinaryOp::FloatGreater
                    | BinaryOp::FloatGreaterEqual => {
                        matches!(types.underlying_kind(ty), Some(TypeKind::F32))
                    }
                    BinaryOp::Equal | BinaryOp::NotEqual => {
                        types.is_integer(ty)
                            || matches!(
                                types.underlying_kind(ty),
                                Some(TypeKind::Bool | TypeKind::Pointer(_) | TypeKind::Function(_))
                            )
                    }
                    BinaryOp::LessSigned
                    | BinaryOp::LessEqualSigned
                    | BinaryOp::GreaterSigned
                    | BinaryOp::GreaterEqualSigned
                    | BinaryOp::ShiftRightArithmetic => types.is_signed_integer(ty),
                    BinaryOp::LessUnsigned
                    | BinaryOp::LessEqualUnsigned
                    | BinaryOp::GreaterUnsigned
                    | BinaryOp::GreaterEqualUnsigned
                    | BinaryOp::ShiftRightLogical => {
                        types.is_integer(ty) && !types.is_signed_integer(ty)
                    }
                    _ => types.is_integer(ty),
                }),
                InstructionKind::Cast {
                    value,
                    to,
                    behavior,
                } => ty(*value).is_some_and(|from| {
                    cast_valid(types, from, *to)
                        && (!matches!(types.underlying_kind(from), Some(TypeKind::F32))
                            || !types.is_integer(*to)
                            || *behavior == CastBehavior::Checked)
                }),
                InstructionKind::AggregateCopy { source, .. } => matches!(
                    types.underlying_kind(source.ty),
                    Some(TypeKind::Array { .. } | TypeKind::Struct(_) | TypeKind::Interface(_))
                ),
                InstructionKind::AddressOf(Place {
                    base: PlaceBase::Local(id),
                    ..
                }) => f
                    .locals
                    .get(id.0 as usize)
                    .is_some_and(|local| local.address_taken),
                InstructionKind::Check(Check::SignedDivisionOverflow { lhs, rhs }) => {
                    ty(*lhs).is_some_and(|t| types.is_signed_integer(t) && Some(t) == ty(*rhs))
                }
                InstructionKind::Check(Check::Bounds { length, .. }) => {
                    ty(*length) == Some(TypeId::U32)
                }
                InstructionKind::Call {
                    callee: Callee::Intrinsic(op),
                    signature,
                    effects,
                    ..
                } => intrinsic(types, *op, signature) && *effects == CallEffects::Unknown,
                _ => true,
            };
            if !valid {
                error(
                    Some(block.id),
                    "instruction violates its operand/type/effect contract",
                );
            }
        }
    }
    errors
}

fn intrinsic(types: &TypeTable, op: Intrinsic, sig: &FunctionType) -> bool {
    let pointer = |ty| matches!(types.underlying_kind(ty), Some(TypeKind::Pointer(_)));
    let ptr_to = |ty, element| matches!(types.underlying_kind(ty), Some(TypeKind::Pointer(actual)) if *actual == element);
    match op {
        Intrinsic::New(element) => {
            types.get(element).is_some()
                && sig.parameters.is_empty()
                && sig.returns.len() == 1
                && ptr_to(sig.returns[0], element)
        }
        Intrinsic::Alloc(element) => {
            sig.parameters == [TypeId::U32]
                && sig.returns.len() == 1
                && types.get(element).is_some()
                && ptr_to(sig.returns[0], element)
        }
        Intrinsic::Free => {
            sig.parameters.len() == 1 && pointer(sig.parameters[0]) && sig.returns.is_empty()
        }
        Intrinsic::Load32 | Intrinsic::Store32 => {
            let store = op == Intrinsic::Store32;
            sig.parameters.len() == if store { 2 } else { 1 }
                && (sig.parameters[0] == TypeId::U32 || pointer(sig.parameters[0]))
                && if store {
                    sig.parameters[1] == TypeId::U32 && sig.returns.is_empty()
                } else {
                    sig.returns == [TypeId::U32]
                }
        }
    }
}

pub(super) fn project(project: &Project) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let functions = project
        .packages
        .iter()
        .flat_map(|p| p.functions.iter().chain(p.initializer.iter()))
        .collect::<Vec<_>>();
    let mut symbols = BTreeMap::new();
    for f in &functions {
        let signature = FunctionType {
            parameters: f
                .parameters
                .iter()
                .filter_map(|id| f.locals.get(id.0 as usize).map(|l| l.ty))
                .collect(),
            returns: f.returns.clone(),
        };
        if symbols.insert(f.name.as_str(), signature).is_some() {
            errors.push(ValidationError {
                function: f.name.clone(),
                block: None,
                message: "duplicate function symbol".into(),
            });
        }
    }
    for f in functions {
        for block in &f.blocks {
            for i in &block.instructions {
                let reference = match &i.kind {
                    InstructionKind::Call {
                        callee: Callee::Direct(symbol),
                        signature,
                        ..
                    }
                    | InstructionKind::Constant(Constant::Function {
                        symbol, signature, ..
                    }) => Some((symbol, signature)),
                    _ => None,
                };
                if let Some((symbol, signature)) = reference {
                    if symbols.get(symbol.as_str()) != Some(signature) {
                        errors.push(ValidationError {
                            function: f.name.clone(),
                            block: Some(block.id),
                            message: format!("unknown function or signature mismatch: {symbol}"),
                        });
                    }
                }
            }
        }
    }
    errors
}
