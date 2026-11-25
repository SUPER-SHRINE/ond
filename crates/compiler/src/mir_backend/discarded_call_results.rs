//! Omit caller-private spills of unused scalar results, after the ABI reads.
use super::{layout, mir};
use mir::{BinaryOp, InstructionKind as I, TerminatorKind as T, TypeKind, ValueId};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Plan {
    pub calls: BTreeSet<mir::InstructionId>,
    pub used: Vec<bool>,
}

impl Plan {
    pub fn new(p: &mir::Project, f: &mir::Function, abi: &layout::AbiPlan) -> Option<Self> {
        // Keep the caller's incoming ABI simple too. Only analyze functions
        // that could have a discard among at least five Direct-1 results.
        if abi.hidden.is_some() {
            return None;
        }
        let candidates: Vec<_> = f
            .blocks
            .iter()
            .flat_map(|b| &b.instructions)
            .filter(|i| {
                matches!(&i.kind, I::Call {
                    callee: mir::Callee::Direct(_), signature, ..
                } if i.results.len() >= 5
                    && signature.returns.iter().all(|&ty| {
                        scalar(&p.types, ty) && layout::words(&p.types, ty) == 1
                    }))
            })
            .collect();
        if candidates.is_empty() {
            return None;
        }
        let package = p
            .packages
            .iter()
            .find(|package| package.functions.iter().any(|body| std::ptr::eq(body, f)))?;
        let mut known = BTreeMap::new();
        let mut used = vec![false; f.values.len()];
        // A typed local or static access cannot name a ValueId slot. Exclude
        // observers throughout the caller, and require every source call to
        // resolve to a likewise private leaf emitted in this same Object.
        if !private_body(
            &p.types,
            f,
            |name| {
                *known.entry(name).or_insert_with(|| {
                    package.functions.iter().any(|body| {
                        body.name == name && private_body(&p.types, body, |_| false, |_| {})
                    })
                })
            },
            |value| used[value.0 as usize] = true,
        ) {
            return None;
        }
        let calls: BTreeSet<_> = candidates
            .into_iter()
            .filter(|i| {
                let live = i
                    .results
                    .iter()
                    .filter(|value| used[value.0 as usize])
                    .count();
                (1..=4).contains(&live)
            })
            .map(|i| i.id)
            .collect();
        (!calls.is_empty()).then_some(Self { calls, used })
    }
}

fn scalar(types: &mir::TypeTable, ty: mir::TypeId) -> bool {
    types.is_integer(ty)
        || matches!(
            types.underlying_kind(ty),
            Some(TypeKind::Bool | TypeKind::F32 | TypeKind::Q16)
        )
}

fn private_body<'a>(
    types: &mir::TypeTable,
    f: &'a mir::Function,
    mut call: impl FnMut(&'a str) -> bool,
    mut use_value: impl FnMut(ValueId),
) -> bool {
    if f.locals
        .iter()
        .any(|local| local.address_taken || !scalar(types, local.ty))
        || f.values.iter().any(|value| !scalar(types, value.ty))
    {
        return false;
    }
    let ty = |value: ValueId| f.values[value.0 as usize].ty;
    let plain_place = |place: &mir::Place, access| {
        access == mir::AccessKind::Local
            && place.projections.is_empty()
            && matches!(
                place.base,
                mir::PlaceBase::Local(_) | mir::PlaceBase::Static { .. }
            )
    };
    for block in &f.blocks {
        for i in &block.instructions {
            match &i.kind {
                I::Constant(_) => {}
                I::Load { place, access } if plain_place(place, *access) => {}
                I::Store {
                    place,
                    access,
                    value,
                } if plain_place(place, *access) => {
                    use_value(*value);
                }
                // Limit evaluated operators to lowering without opaque code
                // or helper calls. Other scalar types may still be moved.
                I::Unary { operand, .. }
                    if types.is_integer(ty(*operand))
                        || types.underlying_id(ty(*operand)) == Some(mir::TypeId::BOOL) =>
                {
                    use_value(*operand);
                }
                I::Binary { op, lhs, rhs, .. }
                    if types.is_integer(ty(*lhs))
                        && !matches!(op, BinaryOp::Divide | BinaryOp::Remainder)
                        && (*op != BinaryOp::Multiply || layout::words(types, ty(*lhs)) == 1) =>
                {
                    use_value(*lhs);
                    use_value(*rhs);
                }
                I::Cast { value, to, .. }
                    if (types.is_integer(ty(*value)) && types.is_integer(*to))
                        || types.underlying_id(ty(*value)) == types.underlying_id(*to) =>
                {
                    use_value(*value);
                }
                I::Call {
                    callee: mir::Callee::Direct(name),
                    arguments,
                    ..
                } if call(name) => {
                    arguments.iter().copied().for_each(&mut use_value);
                }
                _ => return false,
            }
        }
        match &block.terminator.kind {
            T::Jump(edge) => edge.arguments.iter().copied().for_each(&mut use_value),
            T::Branch {
                condition,
                then_edge,
                else_edge,
            } => {
                use_value(*condition);
                then_edge
                    .arguments
                    .iter()
                    .chain(&else_edge.arguments)
                    .copied()
                    .for_each(&mut use_value);
            }
            T::Return(values) => values.iter().copied().for_each(&mut use_value),
            T::Trap(_) | T::Unreachable => {}
        }
    }
    true
}
