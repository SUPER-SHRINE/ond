//! Reuse of a completed integer quotient within the same basic block.
use super::mir;
use mir::{AccessKind, BinaryOp, InstructionKind as I, PlaceBase, TypeId};
use std::collections::BTreeMap;

pub(super) type Plan = BTreeMap<mir::InstructionId, mir::ValueId>;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Operand {
    Value(mir::ValueId),
    Local(mir::LocalId, Option<mir::InstructionId>),
}

fn supported(ty: TypeId) -> bool {
    matches!(
        ty,
        TypeId::U8 | TypeId::I8 | TypeId::U16 | TypeId::I16 | TypeId::U32 | TypeId::I32
    )
}

pub(super) fn plan(f: &mir::Function) -> Plan {
    let mut remainders = Plan::new();
    let instructions = || f.blocks.iter().flat_map(|b| &b.instructions);
    if !instructions().any(|i| {
        matches!(
            i.kind,
            I::Binary {
                op: BinaryOp::Remainder,
                ..
            }
        ) && supported(f.values[i.results[0].0 as usize].ty)
    }) {
        return remainders;
    }
    let mut address_taken: Vec<_> = f.locals.iter().map(|local| local.address_taken).collect();
    for i in instructions() {
        if let I::AddressOf(place) = &i.kind
            && let PlaceBase::Local(id) = place.base
        {
            address_taken[id.0 as usize] = true;
        }
    }

    let ty = |id: mir::ValueId| f.values[id.0 as usize].ty;
    for block in &f.blocks {
        let mut last_store = BTreeMap::new();
        let mut loads = BTreeMap::new();
        let mut quotients = BTreeMap::new();
        for i in &block.instructions {
            match &i.kind {
                I::Load {
                    place:
                        mir::Place {
                            base: PlaceBase::Local(local),
                            projections,
                            ty,
                        },
                    access: AccessKind::Local,
                } if projections.is_empty() && supported(*ty) => {
                    if !address_taken[local.0 as usize] {
                        // A load records the last write when it executes, even
                        // when its SSA value is used after a later store.
                        loads.insert(
                            i.results[0],
                            Operand::Local(*local, last_store.get(local).copied()),
                        );
                    }
                }
                I::Store {
                    place:
                        mir::Place {
                            base: PlaceBase::Local(local),
                            projections,
                            ty,
                        },
                    access: AccessKind::Local,
                    ..
                } if projections.is_empty() && supported(*ty) => {
                    last_store.insert(*local, i.id);
                }
                I::Binary {
                    op: op @ (BinaryOp::Divide | BinaryOp::Remainder),
                    lhs,
                    rhs,
                    overflow: mir::OverflowBehavior::Checked,
                } => {
                    let result = i.results[0];
                    let result_type = ty(result);
                    if !supported(result_type) || ty(*lhs) != result_type || ty(*rhs) != result_type
                    {
                        continue;
                    }
                    let operand = |id| loads.get(&id).copied().unwrap_or(Operand::Value(id));
                    let key = (result_type, operand(*lhs), operand(*rhs));
                    if *op == BinaryOp::Divide {
                        quotients.insert(key, result);
                    } else if let Some(&quotient) = quotients.get(&key) {
                        remainders.insert(i.id, quotient);
                    }
                }
                I::Constant(_)
                | I::Unary { .. }
                | I::Binary { .. }
                | I::Cast { .. }
                | I::Check(_) => {}
                // Calls, assembly and indirect/compound accesses may observe
                // or modify storage, including frame slots via raw addresses.
                I::Call { .. }
                | I::InlineAssembly { .. }
                | I::AddressOf(_)
                | I::Load { .. }
                | I::Store { .. }
                | I::AggregateCopy { .. } => {
                    last_store.clear();
                    loads.clear();
                    quotients.clear();
                }
            }
        }
    }
    remainders
}
