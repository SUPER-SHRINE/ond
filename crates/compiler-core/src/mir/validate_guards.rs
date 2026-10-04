//! Safety obligations for checked operations. Guards must dominate their use.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn validate(
    types: &TypeTable,
    f: &Function,
    dominators: &[BTreeSet<usize>],
    reachable: &BTreeSet<usize>,
) -> Vec<ValidationError> {
    let mut definitions = BTreeMap::new();
    let mut checks = Vec::new();
    for (b, block) in f.blocks.iter().enumerate() {
        for (i, instruction) in block.instructions.iter().enumerate() {
            for result in &instruction.results {
                definitions.insert(*result, &instruction.kind);
            }
            if let InstructionKind::Check(check) = &instruction.kind {
                checks.push((b, i, check));
            }
        }
    }
    let integer = |id: ValueId| -> Option<i128> {
        match definitions.get(&id)? {
            InstructionKind::Constant(Constant::Integer { bits, ty }) => {
                Some(if types.is_signed_integer(*ty) {
                    *bits as i64 as i128
                } else {
                    i128::from(*bits)
                })
            }
            InstructionKind::Constant(Constant::Zero(ty)) if types.is_integer(*ty) => Some(0),
            _ => None,
        }
    };
    let mut errors = Vec::new();
    for (b, block) in f.blocks.iter().enumerate() {
        for (i, instruction) in block.instructions.iter().enumerate() {
            let guarded = |predicate: &dyn Fn(&Check) -> bool| {
                checks.iter().any(|(owner, position, check)| {
                    ((*owner == b && *position < i)
                        || (*owner != b && reachable.contains(&b) && dominators[b].contains(owner)))
                        && predicate(check)
                })
            };
            let bounds = |index, length: u32| {
                integer(index).is_some_and(|index| index >= 0 && index < i128::from(length))
                    || guarded(
                        &|check| matches!(check, Check::Bounds { index: checked, length: bound } if *checked == index && integer(*bound) == Some(i128::from(length))),
                    )
            };
            let mut valid = true;
            match &instruction.kind {
                InstructionKind::Binary {
                    op: op @ (BinaryOp::Divide | BinaryOp::Remainder),
                    lhs,
                    rhs,
                    overflow,
                } => {
                    valid &= *overflow == OverflowBehavior::Checked;
                    valid &= integer(*rhs).is_some_and(|n| n != 0)
                        || guarded(&|c| matches!(c, Check::NonZero { value } if value == rhs));
                    let ty = f.values.get(lhs.0 as usize).map(|v| v.ty);
                    if *op == BinaryOp::Divide && ty.is_some_and(|t| types.is_signed_integer(t)) {
                        let min = ty
                            .and_then(|t| crate::constant::integer_range(types, t))
                            .map(|(min, _)| min);
                        valid &= integer(*rhs).is_some_and(|n| n != -1)
                            || integer(*lhs).is_some_and(|n| Some(n) != min)
                            || guarded(
                                &|c| matches!(c, Check::SignedDivisionOverflow { lhs: a, rhs: z } if a == lhs && z == rhs),
                            );
                    }
                }
                _ => {}
            }
            let places: Vec<&Place> = match &instruction.kind {
                InstructionKind::Load { place, .. }
                | InstructionKind::Store { place, .. }
                | InstructionKind::AddressOf(place) => vec![place],
                InstructionKind::AggregateCopy {
                    source,
                    destination,
                    ..
                } => vec![source, destination],
                _ => Vec::new(),
            };
            for place in places {
                let mut current = match &place.base {
                    PlaceBase::Local(id) => f.locals.get(id.0 as usize).map(|l| l.ty),
                    PlaceBase::Static { ty, .. } => Some(*ty),
                    PlaceBase::Pointer(id) => f.values.get(id.0 as usize).and_then(|v| match types
                        .underlying_kind(v.ty)
                    {
                        Some(TypeKind::Pointer(t)) => Some(*t),
                        _ => None,
                    }),
                };
                for projection in &place.projections {
                    current = match (projection, current.and_then(|ty| types.underlying_kind(ty))) {
                        (
                            Projection::Index { index, .. },
                            Some(TypeKind::Array { length, element }),
                        ) => {
                            valid &= bounds(*index, *length);
                            Some(*element)
                        }
                        (Projection::Field { index, .. }, Some(TypeKind::Struct(s))) => {
                            s.fields.get(*index as usize).map(|f| f.ty)
                        }
                        (Projection::Dereference, Some(TypeKind::Pointer(t))) => Some(*t),
                        (Projection::Offset { .. }, _) => current,
                        _ => None,
                    };
                }
            }
            if !valid {
                errors.push(ValidationError {
                    function: f.name.clone(),
                    block: Some(block.id),
                    message: "checked operation lacks a dominating safety check or static proof"
                        .into(),
                });
            }
        }
    }
    errors
}
