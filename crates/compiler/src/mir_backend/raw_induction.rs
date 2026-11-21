//! Induction of repeated raw addresses whose storage and trip count are proven.
mod cost;
#[cfg(test)]
mod tests;

use super::{Diagnostic, Span, emit::Emitter, frame::Frame, function, layout, mir};
use mir::{
    AccessKind, BinaryOp, BlockId, Function, InstructionKind as I, LocalId, Place, PlaceBase,
    Projection, TerminatorKind as T, TypeId, TypeTable, ValueId,
};

pub(super) fn lower(
    project: &mir::Project,
    globals: &[mir::Global],
    f: &Function,
    emitter: &mut Emitter,
) -> Result<function::PendingFunctionDebug, Diagnostic> {
    let Some(plan) = Plan::new(&project.types, globals, f) else {
        return function::lower(project, f, emitter);
    };
    let mut baseline = Emitter::default();
    let mut layout_before = function::EmittedLayout::default();
    let before = function::lower_body(project, f, &mut baseline, None, Some(&mut layout_before))?;
    let baseline = baseline.resolve()?;

    let mut candidate = Emitter::default();
    let mut layout_after = function::EmittedLayout::default();
    if let Ok(after) = function::lower_body(
        project,
        f,
        &mut candidate,
        Some(&plan),
        Some(&mut layout_after),
    ) && !baseline.has_call_since(0)
        && !candidate.has_call_since(0)
        && let Ok(candidate) = candidate.resolve()
        && cost::improves(
            &plan,
            f,
            (&baseline.words, &layout_before),
            (&candidate.words, &layout_after),
            (before.frame_size, after.frame_size),
        )
        && emitter.can_append(&candidate)
    {
        emitter.append_resolved(candidate, f.span)?;
        return Ok(after);
    }
    emitter.append_resolved(baseline, f.span)?;
    Ok(before)
}

pub(super) struct Plan {
    flow: Flow,
    trips: u32,
    base: ValueId,
    initial_offset: u32,
    byte_step: u32,
    dead_load: Vec<bool>,
}

impl Plan {
    fn new(types: &TypeTable, globals: &[mir::Global], f: &Function) -> Option<Self> {
        if globals.is_empty() || f.blocks.len() < 4 {
            return None;
        }
        let flow = Flow::new(f)?;
        if f.locals.iter().any(|local| local.address_taken)
            || f.blocks.iter().any(|b| !b.parameters.is_empty())
            || f.returns.len() > 4
            || f.returns.iter().any(|ty| !scalar(types, *ty, f.span))
            || f.parameters
                .iter()
                .any(|id| !scalar(types, f.locals[id.0 as usize].ty, f.span))
        {
            return None;
        }
        let mut values = Values::new(f);
        let T::Branch { condition, .. } = f.blocks[flow.header.0 as usize].terminator.kind else {
            return None;
        };
        let (comparison, condition_index, limit) = values.binary(condition)?;
        let index = values.local_load(condition_index, flow.header)?;
        let limit = values.u32(limit)?;
        let index_stores = values.stores(index);
        let [(start_block, init), (update_block, update)] = index_stores.as_slice() else {
            return None;
        };
        if *start_block != f.entry || *update_block != flow.post {
            return None;
        }
        let initial = values.u32(*init)?;
        let (BinaryOp::Add, post_index, step) = values.binary(*update)? else {
            return None;
        };
        if values.local_load(post_index, flow.post)? != index {
            return None;
        }
        let step = values.u32(step)?;
        let trips = trip_count(comparison, initial, limit, step)?;
        if trips == 0 {
            return None;
        }

        let mut base_local = None;
        let mut address_loads = vec![false; f.values.len()];
        let mut accesses = vec![0u32; f.blocks.len()];
        let mut address_of = None;
        for b in &f.blocks {
            for i in &b.instructions {
                match &i.kind {
                    I::Load {
                        access: AccessKind::Ordered,
                        place,
                    }
                    | I::Store {
                        access: AccessKind::Ordered,
                        place,
                        ..
                    } => {
                        let PlaceBase::Pointer(pointer) = place.base else {
                            return None;
                        };
                        let [Projection::Offset { index: offset }, fields @ ..] =
                            place.projections.as_slice()
                        else {
                            return None;
                        };
                        if !flow.body[b.id.0 as usize]
                            || !fields
                                .iter()
                                .all(|field| matches!(field, Projection::Field { .. }))
                            || !scalar(types, place.ty, i.span)
                            || values.local_load(*offset, b.id)? != index
                        {
                            return None;
                        }
                        let local = values.local_load(pointer, b.id)?;
                        if base_local.replace(local).is_some_and(|old| old != local) {
                            return None;
                        }
                        accesses[b.id.0 as usize] += 1;
                        address_loads[pointer.0 as usize] = true;
                        address_loads[offset.0 as usize] = true;
                        if let I::Store { value, .. } = i.kind {
                            values.keep(value);
                        }
                    }
                    I::Load {
                        access: AccessKind::Local,
                        place,
                    } if local(place).is_some() && scalar(types, place.ty, i.span) => {}
                    I::Store {
                        access: AccessKind::Local,
                        place,
                        value,
                    } if local(place).is_some() && scalar(types, place.ty, i.span) => {
                        values.keep(*value);
                    }
                    I::Constant(c) if scalar(types, c.ty(), i.span) => {}
                    I::Binary { lhs, rhs, .. } if values.binary(i.results[0]).is_some() => {
                        values.keep(*lhs);
                        values.keep(*rhs);
                    }
                    I::Check(mir::Check::Bounds { index, length }) if b.id == f.entry => {
                        if values.array_index(*index)? >= values.u32(*length)? {
                            return None;
                        }
                        values.keep(*index);
                        values.keep(*length);
                    }
                    I::AddressOf(place) if b.id == f.entry && address_of.is_none() => {
                        let [Projection::Index { index, .. }] = place.projections.as_slice() else {
                            return None;
                        };
                        values.keep(*index);
                        address_of = Some((i.results[0], place));
                    }
                    _ => return None,
                }
            }
            match &b.terminator.kind {
                T::Branch { condition, .. } => values.keep(*condition),
                T::Return(values_to_return) => {
                    for id in values_to_return {
                        values.keep(*id);
                    }
                }
                _ => {}
            }
        }
        if accesses.iter().map(|n| u64::from(*n)).sum::<u64>() < 2
            || flow.path_weight(
                f,
                &accesses.iter().map(|n| i128::from(*n)).collect::<Vec<_>>(),
                false,
            )? == 0
        {
            return None;
        }
        let base_local = base_local?;
        if [index, base_local]
            .into_iter()
            .any(|id| f.locals[id.0 as usize].kind == mir::LocalKind::Parameter)
        {
            return None;
        }
        let base_stores = values.stores(base_local);
        let [(block, base)] = base_stores.as_slice() else {
            return None;
        };
        let (address, place) = address_of?;
        if *block != f.entry || *base != address {
            return None;
        }
        let PlaceBase::Static { symbol, ty } = &place.base else {
            return None;
        };
        if !globals.iter().any(|g| &g.symbol == symbol && g.ty == *ty) {
            return None;
        }
        let TypeKindArray {
            len,
            element,
            origin,
        } = array(&values, types, *ty, place)?;
        let size = layout::data(types, element, f.span).ok()?.size;
        let (initial_offset, byte_step) = prove_offsets(len, origin, size, initial, step, trips)?;

        for (load, other_use) in address_loads.iter_mut().zip(values.retained) {
            *load &= !other_use;
        }
        Some(Self {
            flow,
            trips,
            base: address,
            initial_offset,
            byte_step,
            dead_load: address_loads,
        })
    }

    pub(super) fn omit(&self, results: &[ValueId]) -> bool {
        results.len() == 1 && self.dead_load[results[0].0 as usize]
    }

    pub(super) fn storage_available(frame: &Frame) -> bool {
        // The scalar-only proof excludes aggregate copy (the only other
        // scratch user). Frame::new reserves an aligned byte and pads it to RA.
        frame.scratch.is_multiple_of(4)
            && frame
                .ra
                .checked_sub(frame.scratch)
                .is_some_and(|bytes| bytes >= 4)
    }

    pub(super) fn emit_update(&self, e: &mut Emitter, block: BlockId, frame: &Frame) {
        if block == self.flow.entry {
            e.load(4, frame.value(self.base));
            if self.initial_offset != 0 {
                e.immediate(2, self.initial_offset);
                e.emit(0, 4, 4, 2, 0);
            }
            e.store(4, frame.scratch);
        } else if block == self.flow.post && self.byte_step != 0 {
            e.load(4, frame.scratch);
            e.immediate(2, self.byte_step);
            e.emit(0, 4, 4, 2, 0);
            e.store(4, frame.scratch);
        }
    }
}

fn scalar(types: &TypeTable, ty: TypeId, span: Span) -> bool {
    layout::scalar(types, ty, span).is_ok_and(|s| s.size <= 4)
}

fn local(place: &Place) -> Option<LocalId> {
    match place.base {
        PlaceBase::Local(id) if place.projections.is_empty() => Some(id),
        _ => None,
    }
}

struct Values<'a> {
    f: &'a Function,
    definitions: Vec<Option<(BlockId, &'a I)>>,
    retained: Vec<bool>,
}

impl<'a> Values<'a> {
    fn new(f: &'a Function) -> Self {
        let mut definitions = vec![None; f.values.len()];
        for b in &f.blocks {
            for i in &b.instructions {
                for &id in &i.results {
                    definitions[id.0 as usize] = Some((b.id, &i.kind));
                }
            }
        }
        Self {
            f,
            definitions,
            retained: vec![false; f.values.len()],
        }
    }

    fn ty(&self, id: ValueId) -> TypeId {
        self.f.values[id.0 as usize].ty
    }

    fn u32(&self, id: ValueId) -> Option<u32> {
        match self.definitions[id.0 as usize]?.1 {
            I::Constant(mir::Constant::Integer {
                bits,
                ty: TypeId::U32,
            }) => u32::try_from(*bits).ok(),
            I::Constant(mir::Constant::Zero(TypeId::U32)) => Some(0),
            _ => None,
        }
    }

    fn array_index(&self, id: ValueId) -> Option<u32> {
        self.u32(id)
            .or_else(|| match self.definitions[id.0 as usize]?.1 {
                I::Constant(mir::Constant::Integer {
                    bits,
                    ty: TypeId::I32,
                }) => i32::try_from(*bits).ok().map(|n| n as u32),
                I::Constant(mir::Constant::Zero(TypeId::I32)) => Some(0),
                _ => None,
            })
    }

    fn binary(&self, id: ValueId) -> Option<(BinaryOp, ValueId, ValueId)> {
        match *self.definitions[id.0 as usize]?.1 {
            I::Binary {
                op:
                    op @ (BinaryOp::Add | BinaryOp::Equal | BinaryOp::NotEqual | BinaryOp::LessUnsigned),
                lhs,
                rhs,
                overflow: mir::OverflowBehavior::Wrap,
            } if self.ty(lhs) == TypeId::U32 && self.ty(rhs) == TypeId::U32 => Some((op, lhs, rhs)),
            _ => None,
        }
    }

    fn local_load(&self, id: ValueId, block: BlockId) -> Option<LocalId> {
        match self.definitions[id.0 as usize]? {
            (
                b,
                I::Load {
                    access: AccessKind::Local,
                    place,
                },
            ) if b == block => local(place),
            _ => None,
        }
    }

    fn stores(&self, id: LocalId) -> Vec<(BlockId, ValueId)> {
        self.f
            .blocks
            .iter()
            .flat_map(|b| {
                b.instructions.iter().filter_map(move |i| match &i.kind {
                    I::Store { place, value, .. } if local(place) == Some(id) => {
                        Some((b.id, *value))
                    }
                    _ => None,
                })
            })
            .collect()
    }

    fn keep(&mut self, id: ValueId) {
        self.retained[id.0 as usize] = true;
    }
}

struct TypeKindArray {
    len: u32,
    element: TypeId,
    origin: u32,
}

fn array(
    values: &Values<'_>,
    types: &TypeTable,
    ty: TypeId,
    place: &Place,
) -> Option<TypeKindArray> {
    let &mir::TypeKind::Array {
        element,
        length: len,
    } = types.underlying_kind(ty)?
    else {
        return None;
    };
    let [
        Projection::Index {
            index,
            element: indexed,
        },
    ] = place.projections.as_slice()
    else {
        return None;
    };
    let origin = values.array_index(*index)?;
    (origin < len && *indexed == element && place.ty == element).then_some(TypeKindArray {
        len,
        element,
        origin,
    })
}

fn trip_count(comparison: BinaryOp, first: u32, bound: u32, step: u32) -> Option<u32> {
    if step == 0 {
        return None;
    }
    match comparison {
        BinaryOp::LessUnsigned => {
            let distance = u64::from(bound.saturating_sub(first));
            let trips = distance.div_ceil(u64::from(step));
            // The monotone proof requires the terminating comparison before wrap.
            u32::try_from(u64::from(first) + trips * u64::from(step)).ok()?;
            u32::try_from(trips).ok()
        }
        BinaryOp::NotEqual => {
            let shift = step.trailing_zeros();
            let mask = u32::MAX >> shift;
            let delta = bound.wrapping_sub(first);
            if (delta >> shift) << shift != delta {
                return None;
            }
            // Divide the congruence by gcd(step, 2^32); the remaining odd
            // multiplier has an inverse modulo 2^32. Each iteration doubles
            // the number of correct low bits, starting with one.
            let odd = step >> shift;
            let mut inverse = 1u32;
            for _ in 0..5 {
                inverse = inverse.wrapping_mul(2u32.wrapping_sub(odd.wrapping_mul(inverse)));
            }
            Some((delta >> shift).wrapping_mul(inverse) & mask)
        }
        _ => None,
    }
}

fn prove_offsets(
    len: u32,
    origin: u32,
    size: u32,
    first: u32,
    step: u32,
    trips: u32,
) -> Option<(u32, u32)> {
    let last_element = len.checked_mul(size)?.checked_sub(size)?;
    let initial = first.wrapping_mul(size);
    let start = origin.checked_mul(size)?.wrapping_add(initial);
    let byte_step = step.wrapping_mul(size);
    if size == 0 || start > last_element || !start.is_multiple_of(size) {
        return None;
    }
    // If either lift stays inside the array, all intermediate offsets do too.
    // This admits wrapped indices when their modulo addresses are consecutive
    // steps within the array, without imposing a trip-count cutoff.
    let lift = i128::from(byte_step);
    if trips > 1
        && ![lift, lift - (1i128 << 32)].into_iter().any(|stride| {
            let end = i128::from(start) + stride * i128::from(trips - 1);
            stride % i128::from(size) == 0 && (0..=i128::from(last_element)).contains(&end)
        })
    {
        return None;
    }
    Some((initial, byte_step))
}

struct Flow {
    entry: BlockId,
    header: BlockId,
    first: BlockId,
    post: BlockId,
    exit: BlockId,
    body: Vec<bool>,
    order: Vec<BlockId>,
}

fn successors(block: &mir::BasicBlock) -> impl Iterator<Item = BlockId> {
    match &block.terminator.kind {
        T::Jump(edge) => [Some(edge.target), None],
        T::Branch {
            then_edge,
            else_edge,
            ..
        } => [Some(then_edge.target), Some(else_edge.target)],
        _ => [None, None],
    }
    .into_iter()
    .flatten()
}

impl Flow {
    fn new(f: &Function) -> Option<Self> {
        let get = |id: BlockId| &f.blocks[id.0 as usize];
        let entry = f.entry;
        let T::Jump(to_header) = &get(entry).terminator.kind else {
            return None;
        };
        let header = to_header.target;
        let T::Branch {
            then_edge,
            else_edge,
            ..
        } = &get(header).terminator.kind
        else {
            return None;
        };
        let (first, exit) = (then_edge.target, else_edge.target);
        if [entry, header].contains(&exit) || !matches!(get(exit).terminator.kind, T::Return(_)) {
            return None;
        }
        let mut backedges = f
            .blocks
            .iter()
            .filter(|b| b.id != entry && successors(b).any(|to| to == header));
        let post = backedges.next()?.id;
        if backedges.next().is_some()
            || [header, exit].contains(&post)
            || !matches!(get(post).terminator.kind, T::Jump(_))
        {
            return None;
        }
        let mut body = vec![false; f.blocks.len()];
        let mut incoming = vec![0usize; f.blocks.len()];
        let mut pending = vec![first];
        while let Some(id) = pending.pop() {
            if id == post {
                continue;
            }
            if [entry, header, exit].contains(&id) {
                return None;
            }
            if std::mem::replace(&mut body[id.0 as usize], true) {
                continue;
            }
            let mut edges = successors(get(id)).peekable();
            edges.peek()?;
            for target in edges {
                incoming[target.0 as usize] += 1;
                pending.push(target);
            }
        }
        if incoming[post.0 as usize] == 0
            || f.blocks.iter().any(|b| {
                b.id != header
                    && !body[b.id.0 as usize]
                    && successors(b).any(|id| id == post || body[id.0 as usize])
            })
        {
            return None;
        }
        let body_len = body.iter().filter(|b| **b).count();
        let mut order = Vec::with_capacity(body_len);
        pending.push(first);
        while let Some(id) = pending.pop() {
            if id == post || incoming[id.0 as usize] != 0 {
                continue;
            }
            order.push(id);
            for target in successors(get(id)) {
                incoming[target.0 as usize] -= 1;
                if incoming[target.0 as usize] == 0 {
                    pending.push(target);
                }
            }
        }
        (order.len() == body_len).then_some(Self {
            entry,
            header,
            first,
            post,
            exit,
            body,
            order,
        })
    }

    fn path_weight(&self, f: &Function, weights: &[i128], maximum: bool) -> Option<i128> {
        let mut totals = vec![0; f.blocks.len()];
        for &id in self.order.iter().rev() {
            let choices = successors(&f.blocks[id.0 as usize]).map(|to| totals[to.0 as usize]);
            let total = if maximum {
                choices.max()?
            } else {
                choices.min()?
            };
            totals[id.0 as usize] = weights[id.0 as usize] + total;
        }
        Some(totals[self.first.0 as usize])
    }
}
