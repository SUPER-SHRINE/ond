//! Forward immutable array snapshots through an unobservable staging local.
use super::{layout, mir};
use mir::{AccessKind, InstructionKind as I, Place, PlaceBase, Projection};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct Plan {
    pub eliminated_locals: BTreeSet<mir::LocalId>,
    pub eliminated_stores: BTreeSet<mir::InstructionId>,
    pub copies: BTreeMap<mir::InstructionId, mir::ValueId>,
}

impl Plan {
    pub fn new(types: &mir::TypeTable, f: &mir::Function, abi: &layout::AbiPlan) -> Self {
        let mut plan = Self::default();
        let is_array = |ty| matches!(types.underlying_kind(ty), Some(mir::TypeKind::Array { .. }));
        if !f
            .locals
            .iter()
            .any(|l| l.kind == mir::LocalKind::Temporary && is_array(l.ty))
        {
            return plan;
        }

        let mut references = vec![0usize; f.locals.len()];
        if !private_places(types, f, abi, |id| references[id.0 as usize] += 1) {
            return plan;
        }

        for pair in f.blocks.iter().flat_map(|b| b.instructions.windows(2)) {
            let [store, copy] = pair else { unreachable!() };
            let I::Store { place, value, .. } = &store.kind else {
                continue;
            };
            let I::AggregateCopy {
                source,
                destination,
                ..
            } = &copy.kind
            else {
                continue;
            };
            let (PlaceBase::Local(src), PlaceBase::Local(dst)) = (&source.base, &destination.base)
            else {
                continue;
            };
            let local = &f.locals[src.0 as usize];
            let target = &f.locals[dst.0 as usize];
            if place != source
                || !source.projections.is_empty()
                || local.kind != mir::LocalKind::Temporary
                || references[src.0 as usize] != 2
                || !is_array(source.ty)
                || !layout::data(types, source.ty, copy.span).is_ok_and(|data| data.size != 0)
                || target.kind == mir::LocalKind::Parameter
            {
                continue;
            }
            plan.eliminated_locals.insert(*src);
            plan.eliminated_stores.insert(store.id);
            plan.copies.insert(copy.id, *value);
        }
        plan
    }
}

// The whole function must keep the stack slots private, including across
// calls, asm, and raw or ordered memory accesses.
pub(super) fn private_places(
    types: &mir::TypeTable,
    f: &mir::Function,
    abi: &layout::AbiPlan,
    mut visit: impl FnMut(mir::LocalId),
) -> bool {
    if f.locals.iter().any(|l| l.address_taken)
        || abi.hidden.is_some()
        || f.parameters
            .iter()
            .any(|id| layout::aggregate(types, f.locals[id.0 as usize].ty))
    {
        return false;
    }
    let mut private = |place: &Place, access| {
        let PlaceBase::Local(id) = place.base else {
            return false;
        };
        if access != AccessKind::Local
            || !place
                .projections
                .iter()
                .all(|p| matches!(p, Projection::Field { .. } | Projection::Index { .. }))
        {
            return false;
        }
        visit(id);
        true
    };
    f.blocks
        .iter()
        .flat_map(|b| &b.instructions)
        .all(|inst| match &inst.kind {
            I::Load { place, access } | I::Store { place, access, .. } => private(place, *access),
            I::AggregateCopy {
                source,
                destination,
                access,
            } => private(source, *access) && private(destination, *access),
            I::AddressOf(_) | I::Call { .. } | I::InlineAssembly { .. } => false,
            _ => true,
        })
}
