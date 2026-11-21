//! Forward known leaves in a single block while preserving every original frame slot.
use super::{function::Context, *};
use mir::{AccessKind, Constant, InstructionKind as I, Place, PlaceBase, Projection, ValueId};
use std::collections::BTreeMap;

const MAX_LEAVES: usize = 4;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Source {
    Zero,
    Scalar(ValueId),
}

struct Region {
    base: u32,
    leaves: Vec<(u32, layout::ScalarLayout)>,
}

pub(super) struct LocalAggregates {
    indices: BTreeMap<ValueId, u32>,
    // A known source is either zero or an immutable, normalized scalar value slot.
    slots: BTreeMap<u32, Source>,
    aliases: BTreeMap<ValueId, Source>,
    scratch: Option<(mir::TypeId, Vec<Source>)>,
    in_r1: Option<Source>,
}

impl LocalAggregates {
    pub(super) fn new(
        types: &mir::TypeTable,
        f: &mir::Function,
        abi: &layout::AbiPlan,
    ) -> Option<Self> {
        let [block] = f.blocks.as_slice() else {
            return None;
        };
        if f.values
            .iter()
            .any(|v| layout::words(types, v.ty) == 2 || types.fixed_fraction(v.ty).is_some())
            || abi.hidden.is_some()
            || f.parameters
                .iter()
                .any(|id| layout::aggregate(types, f.locals[id.0 as usize].ty))
            || !f
                .locals
                .iter()
                .any(|local| !local.address_taken && small_aggregate(types, local.ty, local.span))
        {
            return None;
        }
        let indices = block
            .instructions
            .iter()
            .filter_map(|i| match i.kind {
                I::Constant(Constant::Integer { bits, .. }) => {
                    Some((i.results[0], u32::try_from(bits).ok()?))
                }
                _ => None,
            })
            .collect::<BTreeMap<_, _>>();
        let local = |place: &Place, access| {
            access == AccessKind::Local
                && matches!(place.base, PlaceBase::Local(_))
                && place.projections.iter().all(|part| match part {
                    Projection::Field { .. } => true,
                    Projection::Index { index, .. } => indices.contains_key(index),
                    _ => false,
                })
        };
        let float = |ty| matches!(types.underlying_kind(ty), Some(mir::TypeKind::F32));
        let value_is_float = |value: ValueId| float(f.values[value.0 as usize].ty);
        if !block.instructions.iter().all(|i| match &i.kind {
            I::Load { place, access } | I::Store { place, access, .. } => local(place, *access),
            I::AggregateCopy {
                destination,
                source,
                access,
            } => local(source, *access) && local(destination, *access),
            I::InlineAssembly { .. } | I::AddressOf(_) | I::Call { .. } => false,
            I::Binary { op, lhs, .. } => {
                !matches!(op, mir::BinaryOp::Divide | mir::BinaryOp::Remainder)
                    && !value_is_float(*lhs)
            }
            I::Unary { operand, .. } => !value_is_float(*operand),
            I::Cast { value, to, .. } => value_is_float(*value) == float(*to),
            _ => true,
        }) {
            return None;
        }
        Some(Self {
            indices,
            slots: BTreeMap::new(),
            aliases: BTreeMap::new(),
            scratch: None,
            in_r1: None,
        })
    }

    pub(super) fn emit(&mut self, cx: &mut Context<'_>, i: &mir::Instruction) -> bool {
        let emitted = self.try_emit(cx, i).is_some();
        if !emitted {
            self.in_r1 = None;
            let destination = match &i.kind {
                I::Store { place, .. } => Some(place),
                I::AggregateCopy { destination, .. } => {
                    self.scratch = None;
                    Some(destination)
                }
                _ => None,
            };
            if let Some(r) = destination.and_then(|p| self.local_region(cx, p, i.span)) {
                for (offset, _) in r.leaves {
                    self.slots.remove(&(r.base + offset));
                }
            }
        }
        emitted
    }

    fn try_emit(&mut self, cx: &mut Context<'_>, i: &mir::Instruction) -> Option<()> {
        match &i.kind {
            I::Constant(Constant::Zero(ty)) if small_aggregate(&cx.p.types, *ty, i.span) => {
                let dst = value_region(cx, i.results[0], i.span)?;
                self.write(cx, &dst, &vec![Source::Zero; dst.leaves.len()], None);
            }
            I::Load { place, .. } => {
                let src = self.local_region(cx, place, i.span)?;
                let values = self.read(&src)?;
                let result = i.results[0];
                if layout::aggregate(&cx.p.types, place.ty) {
                    self.write(cx, &value_region(cx, result, i.span)?, &values, None);
                } else {
                    let value = values[0];
                    let reg = self.materialize(cx, value, 1);
                    cx.e.store(reg, cx.frame.value(result));
                    self.aliases.insert(result, value);
                }
            }
            I::Store { place, value, .. } => {
                let dst = self.local_region(cx, place, i.span)?;
                let values = if layout::aggregate(&cx.p.types, place.ty) {
                    self.read(&value_region(cx, *value, i.span)?)?
                } else {
                    vec![
                        self.aliases
                            .get(value)
                            .copied()
                            .unwrap_or(Source::Scalar(*value)),
                    ]
                };
                self.write(cx, &dst, &values, None);
            }
            I::AggregateCopy {
                destination,
                source,
                ..
            } => {
                let src = self.local_region(cx, source, i.span)?;
                let dst = self.local_region(cx, destination, i.span)?;
                let values = self.read(&src)?;
                self.write(cx, &dst, &values, Some(source.ty));
                self.scratch = Some((source.ty, values));
            }
            _ => return None,
        }
        Some(())
    }

    fn local_region(&self, cx: &Context<'_>, place: &Place, span: Span) -> Option<Region> {
        let PlaceBase::Local(id) = place.base else {
            return None;
        };
        let local = &cx.f.locals[id.0 as usize];
        let mut ty = local.ty;
        let types = &cx.p.types;
        if local.address_taken || !small_aggregate(types, ty, span) {
            return None;
        }
        let mut base = cx.frame.local(id);
        for part in &place.projections {
            match *part {
                Projection::Field { index, ty: field } => {
                    base += layout::data(types, ty, span).ok()?.fields[index as usize];
                    ty = field;
                }
                Projection::Index { index, element } => {
                    let index = self.indices[&index];
                    let mir::TypeKind::Array { length, .. } = types.underlying_kind(ty)? else {
                        return None;
                    };
                    if index >= *length {
                        return None;
                    }
                    base += index * layout::data(types, element, span).ok()?.size;
                    ty = element;
                }
                _ => return None,
            }
        }
        region(types, base, ty, span)
    }

    fn read(&self, region: &Region) -> Option<Vec<Source>> {
        region
            .leaves
            .iter()
            .map(|(offset, _)| self.slots.get(&(region.base + offset)).copied())
            .collect()
    }

    fn materialize(&mut self, cx: &mut Context<'_>, source: Source, register: u8) -> u8 {
        if let Source::Scalar(value) = source {
            if register != 1 || self.in_r1 != Some(source) {
                cx.load(value, register);
            }
            if register == 1 {
                self.in_r1 = Some(source);
            }
            register
        } else {
            0
        }
    }

    fn write(
        &mut self,
        cx: &mut Context<'_>,
        dest: &Region,
        values: &[Source],
        scratch: Option<mir::TypeId>,
    ) {
        let mut writes = Vec::new();
        for (n, (&(offset, scalar), &value)) in dest.leaves.iter().zip(values).enumerate() {
            let dest_offset = dest.base + offset;
            let write_dest = self.slots.insert(dest_offset, value) != Some(value);
            let write_scratch = scratch.is_some_and(|ty| {
                !matches!(&self.scratch, Some((prev, saved)) if *prev == ty && saved[n] == value)
            });
            if write_scratch || write_dest {
                // r1..r4 hold the snapshot. Stack stores only clobber r13 on large offsets.
                let reg = self.materialize(cx, value, (n + 1) as u8);
                writes.push((reg, offset, scalar, write_scratch, write_dest));
            }
        }
        for &(reg, offset, scalar, write_scratch, _) in &writes {
            if write_scratch {
                cx.e.store_scalar(reg, cx.frame.scratch + offset, scalar);
            }
        }
        for (reg, offset, scalar, _, write_dest) in writes {
            if write_dest {
                cx.e.store_scalar(reg, dest.base + offset, scalar);
            }
        }
    }
}

fn small_aggregate(types: &mir::TypeTable, ty: mir::TypeId, span: Span) -> bool {
    layout::aggregate(types, ty)
        && layout::data(types, ty, span).is_ok_and(|data| data.size <= (MAX_LEAVES as u32) * 4)
        && region(types, 0, ty, span).is_some_and(|r| r.leaves.len() <= MAX_LEAVES)
}

fn region(types: &mir::TypeTable, base: u32, ty: mir::TypeId, span: Span) -> Option<Region> {
    let leaves = layout::leaves(types, ty, span).ok()?;
    (!leaves.is_empty() && leaves.iter().all(|(_, leaf)| leaf.size <= 4))
        .then_some(Region { base, leaves })
}

fn value_region(cx: &Context<'_>, id: ValueId, span: Span) -> Option<Region> {
    region(&cx.p.types, cx.frame.value(id), cx.ty(id), span)
}
