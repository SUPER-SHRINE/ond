//! Runtime Cast elision for the normal `ond compile` path, after constant evaluation.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
mod tests;

pub(crate) fn elide_narrow_f32_roundtrips(function: &mut Function, direct: &BTreeSet<ValueId>) {
    if direct.is_empty() {
        return;
    }
    let mut from_integer = BTreeMap::new();
    for instruction in function.blocks.iter().flat_map(|block| &block.instructions) {
        if let InstructionKind::Cast {
            value,
            to: TypeId::F32,
            behavior: CastBehavior::Infallible,
        } = instruction.kind
            && matches!(
                function.values[value.0 as usize].ty,
                TypeId::U8 | TypeId::I8 | TypeId::U16 | TypeId::I16
            )
        {
            from_integer.insert(instruction.results[0], value);
        }
    }
    if from_integer.is_empty() {
        return;
    }

    let mut replacement = vec![None; function.values.len()];
    let mut unused_float = vec![false; function.values.len()];
    for instruction in function.blocks.iter().flat_map(|block| &block.instructions) {
        if let InstructionKind::Cast {
            value,
            to,
            behavior: CastBehavior::Checked,
        } = instruction.kind
            && let Some(&original) = from_integer.get(&value)
            && function.values[original.0 as usize].ty == to
            && direct.contains(&instruction.results[0])
        {
            replacement[instruction.results[0].0 as usize] = Some(original);
            unused_float[value.0 as usize] = true;
        }
    }
    if replacement.iter().all(Option::is_none) {
        return;
    }

    // Resolve nested round trips without assuming the order of blocks or ValueIds.
    for index in 0..replacement.len() {
        if let Some(mut original) = replacement[index] {
            while let Some(next) = replacement[original.0 as usize] {
                original = next;
            }
            let mut current = index;
            while let Some(next) = replacement[current] {
                replacement[current] = Some(original);
                current = next.0 as usize;
            }
        }
    }

    for block in &mut function.blocks {
        let mut mark_used = |value: &mut ValueId| unused_float[value.0 as usize] = false;
        for instruction in &mut block.instructions {
            if !instruction
                .results
                .iter()
                .any(|result| replacement[result.0 as usize].is_some())
            {
                visit_instruction_operands(&mut instruction.kind, &mut mark_used);
            }
        }
        visit_terminator_operands(&mut block.terminator.kind, &mut mark_used);
    }

    let remove = |value: ValueId| {
        let index = value.0 as usize;
        replacement[index].is_some() || unused_float[index]
    };
    let mut new_ids = vec![ValueId(0); function.values.len()];
    let mut next_value = 0;
    function.values.retain_mut(|value| {
        if remove(value.id) {
            return false;
        }
        new_ids[value.id.0 as usize] = ValueId(next_value);
        value.id = ValueId(next_value);
        next_value += 1;
        true
    });
    let mut remap = |value: &mut ValueId| {
        let original = replacement[value.0 as usize].unwrap_or(*value);
        *value = new_ids[original.0 as usize];
    };
    let mut next_instruction = 0;
    for block in &mut function.blocks {
        block.parameters.iter_mut().for_each(&mut remap);
        block.instructions.retain_mut(|instruction| {
            if instruction.results.iter().any(|&result| remove(result)) {
                return false;
            }
            instruction.id = InstructionId(next_instruction);
            next_instruction += 1;
            instruction.results.iter_mut().for_each(&mut remap);
            visit_instruction_operands(&mut instruction.kind, &mut remap);
            true
        });
        visit_terminator_operands(&mut block.terminator.kind, &mut remap);
    }
}

fn visit_instruction_operands(kind: &mut InstructionKind, visit: &mut impl FnMut(&mut ValueId)) {
    fn place_operands(place: &mut Place, visit: &mut impl FnMut(&mut ValueId)) {
        if let PlaceBase::Pointer(value) = &mut place.base {
            visit(value);
        }
        for projection in &mut place.projections {
            if let Projection::Index { index, .. } | Projection::Offset { index } = projection {
                visit(index);
            }
        }
    }

    match kind {
        InstructionKind::InlineAssembly { inputs, .. } => {
            for (_, value) in inputs {
                visit(value);
            }
        }
        InstructionKind::Constant(_) => {}
        InstructionKind::Unary { operand, .. } | InstructionKind::Cast { value: operand, .. } => {
            visit(operand);
        }
        InstructionKind::Binary { lhs, rhs, .. }
        | InstructionKind::Check(Check::SignedDivisionOverflow { lhs, rhs }) => {
            visit(lhs);
            visit(rhs);
        }
        InstructionKind::AddressOf(place) | InstructionKind::Load { place, .. } => {
            place_operands(place, visit);
        }
        InstructionKind::Store { place, value, .. } => {
            place_operands(place, visit);
            visit(value);
        }
        InstructionKind::AggregateCopy {
            source,
            destination,
            ..
        } => {
            place_operands(source, visit);
            place_operands(destination, visit);
        }
        InstructionKind::Check(Check::Bounds { index, length }) => {
            visit(index);
            visit(length);
        }
        InstructionKind::Check(Check::NonZero { value }) => visit(value),
        InstructionKind::Call {
            callee, arguments, ..
        } => {
            if let Callee::Indirect(value) = callee {
                visit(value);
            }
            arguments.iter_mut().for_each(visit);
        }
    }
}

fn visit_terminator_operands(kind: &mut TerminatorKind, visit: &mut impl FnMut(&mut ValueId)) {
    match kind {
        TerminatorKind::Jump(edge) => edge.arguments.iter_mut().for_each(visit),
        TerminatorKind::Branch {
            condition,
            then_edge,
            else_edge,
        } => {
            visit(condition);
            then_edge.arguments.iter_mut().for_each(&mut *visit);
            else_edge.arguments.iter_mut().for_each(visit);
        }
        TerminatorKind::Return(values) => values.iter_mut().for_each(visit),
        TerminatorKind::Trap(_) | TerminatorKind::Unreachable => {}
    }
}
