//! Prove checks on validated MIR without changing operands, control flow, or frame reservations.
use super::mir;
use mir::{AccessKind, BlockId, Constant, InstructionKind as I, LocalId, Place, PlaceBase, TypeId};
use std::collections::{BTreeMap, BTreeSet};

type Bound = (LocalId, u32);
type Definitions<'a> = BTreeMap<mir::ValueId, (usize, &'a I)>;

#[derive(Default)]
struct Writes {
    opaque: Option<usize>,
    locals: BTreeMap<LocalId, usize>,
}

impl Writes {
    fn invalidates(&self, local: LocalId) -> bool {
        self.opaque.is_some() || self.locals.contains_key(&local)
    }

    fn stable_since(&self, local: LocalId, index: usize, length: usize) -> bool {
        self.opaque.is_none_or(|n| n < index.min(length))
            && self.locals.get(&local).is_none_or(|n| *n < index)
    }
}

#[derive(Default)]
struct BlockProofs {
    writes: Writes,
    on_true: Option<Bound>,
    checks: Vec<(mir::InstructionId, Bound)>,
}

pub(super) fn proven_checks(
    types: &mir::TypeTable,
    f: &mir::Function,
) -> BTreeSet<mir::InstructionId> {
    let aliased: BTreeSet<_> = f
        .blocks
        .iter()
        .flat_map(|b| &b.instructions)
        .filter_map(|i| {
            if let I::AddressOf(Place {
                base: PlaceBase::Local(id),
                ..
            }) = i.kind
            {
                Some(id)
            } else {
                None
            }
        })
        .collect();
    let blocks: Vec<_> = f
        .blocks
        .iter()
        .map(|b| analyze_block(types, f, b, &aliased))
        .collect();
    if !blocks.iter().any(|b| b.on_true.is_some()) || !blocks.iter().any(|b| !b.checks.is_empty()) {
        return BTreeSet::new();
    }

    let entry = f.entry.0 as usize;
    let mut predecessors = vec![vec![]; blocks.len()];
    let mut reachable = BTreeSet::new();
    let mut queue = vec![f.entry];
    while let Some(id) = queue.pop() {
        if !reachable.insert(id.0 as usize) {
            continue;
        }
        let b = &f.blocks[id.0 as usize];
        let mut add = |target: BlockId, takes_true_edge| {
            predecessors[target.0 as usize].push((id.0 as usize, takes_true_edge));
            queue.push(target);
        };
        match &b.terminator.kind {
            mir::TerminatorKind::Jump(edge) => add(edge.target, false),
            mir::TerminatorKind::Branch {
                then_edge,
                else_edge,
                ..
            } => {
                add(then_edge.target, true);
                add(else_edge.target, false);
            }
            _ => {}
        }
    }

    // Start with no assumed facts, including for loop headers. Every reachable
    // predecessor, including a backedge or a parallel false edge, participates.
    let mut incoming = vec![BTreeSet::new(); blocks.len()];
    let mut outgoing = incoming.clone();
    loop {
        let mut changed = false;
        for &id in &reachable {
            if id == entry {
                continue;
            }
            let mut edges = predecessors[id].iter().map(|&(from, then)| {
                let mut facts = outgoing[from].clone();
                if then && let Some(bound) = blocks[from].on_true {
                    facts.insert(bound);
                }
                facts
            });
            let mut common = edges
                .next()
                .expect("reachable non-entry block has a predecessor");
            for facts in edges {
                common.retain(|bound| facts.contains(bound));
            }
            if incoming[id] != common {
                incoming[id].clone_from(&common);
                common.retain(|&(local, _)| !blocks[id].writes.invalidates(local));
                outgoing[id] = common;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    blocks
        .into_iter()
        .zip(incoming)
        .flat_map(|(block, facts)| {
            block
                .checks
                .into_iter()
                .filter_map(move |(id, bound)| facts.contains(&bound).then_some(id))
        })
        .collect()
}

fn analyze_block(
    types: &mir::TypeTable,
    f: &mir::Function,
    block: &mir::BasicBlock,
    aliased: &BTreeSet<LocalId>,
) -> BlockProofs {
    let mut proofs = BlockProofs::default();
    let mut defs = Definitions::new();
    let candidate = |defs: &Definitions<'_>, index, length| {
        let &(index_at, kind) = defs.get(&index)?;
        let I::Load {
            place,
            access: AccessKind::Local,
        } = kind
        else {
            return None;
        };
        let PlaceBase::Local(local) = place.base else {
            return None;
        };
        if place.ty != TypeId::U32
            || !place.projections.is_empty()
            || f.locals[local.0 as usize].address_taken
            || aliased.contains(&local)
        {
            return None;
        }
        let &(length_at, kind) = defs.get(&length)?;
        let I::Constant(Constant::Integer {
            bits,
            ty: TypeId::U32,
        }) = kind
        else {
            return None;
        };
        let length = u32::try_from(*bits).ok().filter(|n| *n > 0)?;
        Some(((local, length), index_at, length_at))
    };
    for (position, i) in block.instructions.iter().enumerate() {
        if let I::Check(mir::Check::Bounds { index, length }) = i.kind
            && let Some((bound, _, _)) = candidate(&defs, index, length)
            && !proofs.writes.invalidates(bound.0)
        {
            proofs.checks.push((i.id, bound));
        }
        if opaque(types, f, &i.kind) {
            proofs.writes.opaque = Some(position);
        }
        let destination = match &i.kind {
            I::Store { place, .. } => Some(place),
            I::AggregateCopy { destination, .. } => Some(destination),
            _ => None,
        };
        if let Some(Place {
            base: PlaceBase::Local(id),
            ..
        }) = destination
        {
            proofs.writes.locals.insert(*id, position);
        }
        for result in &i.results {
            defs.insert(*result, (position, &i.kind));
        }
    }
    if let mir::TerminatorKind::Branch { condition, .. } = block.terminator.kind
        && let Some((
            _,
            I::Binary {
                op: mir::BinaryOp::LessUnsigned,
                lhs,
                rhs,
                ..
            },
        )) = defs.get(&condition)
        && let Some((bound, index_at, length_at)) = candidate(&defs, *lhs, *rhs)
        && proofs.writes.stable_since(bound.0, index_at, length_at)
    {
        proofs.on_true = Some(bound);
    }
    proofs
}

fn opaque(types: &mir::TypeTable, f: &mir::Function, i: &I) -> bool {
    let float = |ty| matches!(types.underlying_kind(ty), Some(mir::TypeKind::F32));
    let value_is_float = |id: mir::ValueId| float(f.values[id.0 as usize].ty);
    // Numeric operations lowered to runtime helper calls are call barriers as well.
    match i {
        I::InlineAssembly { .. } | I::Call { .. } => true,
        I::Binary { op, lhs, .. } => {
            matches!(op, mir::BinaryOp::Divide | mir::BinaryOp::Remainder) || value_is_float(*lhs)
        }
        I::Unary { operand, .. } => value_is_float(*operand),
        I::Cast { value, to, .. } => value_is_float(*value) != float(*to),
        I::Load { place, access } | I::Store { place, access, .. } => {
            *access == AccessKind::Ordered || place.requires_ordered_access()
        }
        I::AggregateCopy {
            destination,
            source,
            access,
        } => {
            *access == AccessKind::Ordered
                || source.requires_ordered_access()
                || destination.requires_ordered_access()
        }
        I::AddressOf(place) => place.requires_ordered_access(),
        _ => false,
    }
}
