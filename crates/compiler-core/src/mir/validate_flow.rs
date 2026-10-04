//! Definition order and reachable CFG dominance, independent of block numbering.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn operands(kind: &InstructionKind) -> Vec<ValueId> {
    let mut values = Vec::new();
    fn place(place: &Place, values: &mut Vec<ValueId>) {
        if let PlaceBase::Pointer(id) = place.base {
            values.push(id);
        }
        for projection in &place.projections {
            if let Projection::Index { index, .. } | Projection::Offset { index } = projection {
                values.push(*index);
            }
        }
    }
    match kind {
        InstructionKind::Constant(_) => {}
        InstructionKind::Unary { operand, .. } | InstructionKind::Cast { value: operand, .. } => {
            values.push(*operand)
        }
        InstructionKind::Binary { lhs, rhs, .. } => values.extend([*lhs, *rhs]),
        InstructionKind::AddressOf(p) | InstructionKind::Load { place: p, .. } => {
            place(p, &mut values)
        }
        InstructionKind::Store {
            place: p, value, ..
        } => {
            place(p, &mut values);
            values.push(*value);
        }
        InstructionKind::AggregateCopy {
            source,
            destination,
            ..
        } => {
            place(source, &mut values);
            place(destination, &mut values);
        }
        InstructionKind::Check(check) => match check {
            Check::Bounds { index, length } => values.extend([*index, *length]),
            Check::NonZero { value } => values.push(*value),
            Check::SignedDivisionOverflow { lhs, rhs } => values.extend([*lhs, *rhs]),
        },
        InstructionKind::Call {
            callee, arguments, ..
        } => {
            if let Callee::Indirect(value) = callee {
                values.push(*value);
            }
            values.extend(arguments);
        }
    }
    values
}

pub(super) fn validate(types: &TypeTable, f: &Function) -> Vec<ValidationError> {
    let n = f.blocks.len();
    let mut predecessors = vec![Vec::new(); n];
    let mut successors = vec![Vec::new(); n];
    let mut definitions = BTreeMap::new();
    for (b, block) in f.blocks.iter().enumerate() {
        for id in &block.parameters {
            definitions.insert(*id, (b, 0usize));
        }
        for (i, instruction) in block.instructions.iter().enumerate() {
            for id in &instruction.results {
                definitions.insert(*id, (b, i + 1));
            }
        }
        let edges: Vec<&Edge> = match &block.terminator.kind {
            TerminatorKind::Jump(edge) => vec![edge],
            TerminatorKind::Branch {
                then_edge,
                else_edge,
                ..
            } => vec![then_edge, else_edge],
            _ => Vec::new(),
        };
        for edge in edges {
            let target = edge.target.0 as usize;
            if target < n {
                successors[b].push(target);
                predecessors[target].push(b);
            }
        }
    }
    let entry = f.entry.0 as usize;
    let mut reachable = BTreeSet::new();
    let mut work = vec![entry];
    while let Some(b) = work.pop() {
        if b < n && reachable.insert(b) {
            work.extend(&successors[b]);
        }
    }
    let mut dom = vec![reachable.clone(); n];
    if entry < n {
        dom[entry] = BTreeSet::from([entry]);
    }
    loop {
        let mut changed = false;
        for &b in &reachable {
            if b == entry {
                continue;
            }
            let mut set = reachable.clone();
            for p in predecessors[b].iter().filter(|p| reachable.contains(p)) {
                set = set.intersection(&dom[*p]).copied().collect();
            }
            set.insert(b);
            if set != dom[b] {
                dom[b] = set;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let mut errors = Vec::new();
    for (b, block) in f.blocks.iter().enumerate() {
        let mut check = |id: ValueId, position: usize| {
            let valid = match definitions.get(&id) {
                Some((owner, definition)) if *owner == b => *definition < position,
                Some((owner, _)) => !reachable.contains(&b) || dom[b].contains(owner),
                None => false,
            };
            if !valid {
                errors.push(ValidationError {
                    function: f.name.clone(),
                    block: Some(block.id),
                    message: format!(
                        "value {id:?} is used before its definition or without dominance"
                    ),
                });
            }
        };
        for (i, instruction) in block.instructions.iter().enumerate() {
            for id in operands(&instruction.kind) {
                check(id, i + 1);
            }
        }
        let mut uses = Vec::new();
        match &block.terminator.kind {
            TerminatorKind::Return(values) => uses.extend(values),
            TerminatorKind::Jump(edge) => uses.extend(&edge.arguments),
            TerminatorKind::Branch {
                condition,
                then_edge,
                else_edge,
            } => {
                uses.push(*condition);
                uses.extend(&then_edge.arguments);
                uses.extend(&else_edge.arguments);
            }
            _ => {}
        }
        for id in uses {
            check(id, usize::MAX);
        }
    }
    errors.extend(super::validate_guards::validate(types, f, &dom, &reachable));
    errors
}
