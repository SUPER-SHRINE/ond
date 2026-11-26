//! Typed accesses. r4/r5 are addresses; r1..r3 are scratch.
use super::{emit::Emitter, function::Context, *};
pub(super) fn address(e: &mut Emitter, reg: u8, offset: u32) {
    e.stack_address(reg, offset);
}
pub(super) fn read(e: &mut Emitter, reg: u8, base: u8, offset: u32, ty: layout::ScalarLayout) {
    let (base, imm) = access_address(e, base, offset, ty);
    read_indexed(e, reg, base, 0, imm, ty);
}
fn read_indexed(e: &mut Emitter, reg: u8, base: u8, index: u8, imm: i16, ty: layout::ScalarLayout) {
    e.emit(
        match ty.size {
            1 => 4,
            2 => 5,
            _ => 7,
        },
        reg,
        base,
        index,
        imm,
    );
    if ty.size == 8 {
        e.emit(7, reg + 1, base, index, imm + 4);
    }
    ops::normalize(e, reg, ty);
}
pub(super) fn write(e: &mut Emitter, reg: u8, base: u8, offset: u32, ty: layout::ScalarLayout) {
    let (base, imm) = access_address(e, base, offset, ty);
    write_indexed(e, reg, base, 0, imm, ty);
}
fn write_indexed(
    e: &mut Emitter,
    reg: u8,
    base: u8,
    index: u8,
    imm: i16,
    ty: layout::ScalarLayout,
) {
    e.emit(store_opcode(ty), reg, base, index, imm);
    if ty.size == 8 {
        e.emit(11, if reg == 0 { 0 } else { reg + 1 }, base, index, imm + 4);
    }
}
fn access_address(e: &mut Emitter, base: u8, offset: u32, ty: layout::ScalarLayout) -> (u8, i16) {
    if offset <= max_direct_offset(ty) {
        (base, offset as i16)
    } else {
        e.immediate(3, offset);
        e.emit(0, 3, base, 3, 0);
        (3, 0)
    }
}
fn max_direct_offset(ty: layout::ScalarLayout) -> u32 {
    // Both word accesses of a 64-bit element must fit the signed immediate.
    i16::MAX as u32 - if ty.size == 8 { 4 } else { 0 }
}
pub(super) fn store_opcode(ty: layout::ScalarLayout) -> u32 {
    match ty.size {
        1 => 8,
        2 => 9,
        _ => 11,
    }
}
/// Narrow Direct-1 return slots use natural alignment and zero upper bytes.
pub(super) fn zero_upper_slot_bytes(e: &mut Emitter, ty: layout::ScalarLayout) {
    for offset in ty.size..4 {
        e.emit(8, 0, 5, 0, offset as i16);
    }
}
/// Source/destination must be disjoint or identical. Arbitrary alias uses a snapshot first.
pub(super) fn copy(
    e: &mut Emitter,
    types: &mir::TypeTable,
    ty: mir::TypeId,
    span: Span,
) -> Result<(), Diagnostic> {
    for (offset, scalar) in layout::leaves(types, ty, span)? {
        read(e, 1, 4, offset, scalar);
        write(e, 1, 5, offset, scalar);
    }
    Ok(())
}
/// Compress a transfer between private slots with the same scalar array type.
pub(super) fn copy_private_array(
    e: &mut Emitter,
    types: &mir::TypeTable,
    ty: mir::TypeId,
    span: Span,
) -> Result<(), Diagnostic> {
    use mir::TypeKind::*;
    if let Some(Array { length, element }) = types.underlying_kind(ty)
        && matches!(
            types.underlying_kind(*element),
            Some(Bool | U8 | I8 | U16 | I16 | U32 | I32 | F32 | Q16 | U64 | I64 | F64 | Q32)
        )
    {
        let scalar = layout::scalar(types, *element, span)?;
        if let Some(loop_bytes) = copy_loop_end(*length, scalar) {
            // r4/r5 keep the evaluated addresses, r1/r2 carry the payload.
            // r3 is the byte index, r12 the bound, and both are dead afterward.
            let body = e.label();
            e.immediate(3, 0);
            e.immediate(12, loop_bytes);
            e.bind(body);
            copy_indexed_elements(e, 4, scalar);
            e.emit(0, 3, 3, 0, (4 * scalar.size) as i16);
            e.emit(13, 1, 3, 12, 0);
            e.near_branch(1, body, span);
            copy_indexed_elements(e, length % 4, scalar);
            return Ok(());
        }
    }
    copy(e, types, ty, span)
}

fn copy_loop_end(length: u32, scalar: layout::ScalarLayout) -> Option<u32> {
    length.checked_mul(scalar.size)?;
    let first_far = max_direct_offset(scalar) / scalar.size + 1;
    if length <= first_far {
        return None;
    }

    let iterations = length / 4;
    let loop_bytes = iterations * 4 * scalar.size;
    let loop_control_steps = 1 + Emitter::immediate_words(loop_bytes) + u64::from(iterations) * 3;
    // Place/slot setup and typed payload (including the tail) are unchanged.
    // For each far element the unrolled read and write independently form
    // base+offset. Count their actual immediate and ADD instructions.
    let address_steps = (first_far..length)
        .map(|index| 2 * (Emitter::immediate_words(index * scalar.size) + 1))
        .sum::<u64>();

    // Unrolled Text has one word per step. Looped Text contains the setup,
    // a single body with control, and the tail. At least two executions of
    // that body make Text strictly smaller when dynamic steps do not grow.
    (iterations >= 2 && loop_control_steps <= address_steps).then_some(loop_bytes)
}

fn copy_indexed_elements(e: &mut Emitter, count: u32, scalar: layout::ScalarLayout) {
    for element in 0..count {
        let offset = (element * scalar.size) as i16;
        read_indexed(e, 1, 4, 3, offset, scalar);
        write_indexed(e, 1, 5, 3, offset, scalar);
    }
}

impl Context<'_> {
    pub(super) fn copy_slots(
        &mut self,
        src: u32,
        dst: u32,
        ty: mir::TypeId,
        span: Span,
    ) -> Result<(), Diagnostic> {
        address(self.e, 4, src);
        if !layout::aggregate(&self.p.types, ty) {
            let scalar = layout::scalar(&self.p.types, ty, span)?;
            read(self.e, 1, 4, 0, scalar);
            self.e.store(1, dst);
            if scalar.size == 8 {
                self.e.store(2, dst + 4);
            }
            return Ok(());
        }
        address(self.e, 5, dst);
        copy(self.e, &self.p.types, ty, span)
    }
    /// Resolve the complete symbolic place into r4 without accessing its final value.
    pub(super) fn place(&mut self, p: &mir::Place, span: Span) -> Result<(), Diagnostic> {
        // The induction proof accepts only raw places whose first projection
        // is the selected offset. Field projections still use normal lowering.
        let carried = self.induction
            && matches!(p.base, mir::PlaceBase::Pointer(_))
            && matches!(p.projections.first(), Some(mir::Projection::Offset { .. }));
        let mut ty = match p.base {
            mir::PlaceBase::Local(id) => {
                address(self.e, 4, self.frame.local(id));
                self.f.locals[id.0 as usize].ty
            }
            mir::PlaceBase::Pointer(v) => {
                if carried {
                    self.e.load(4, self.frame.scratch);
                } else {
                    self.load(v, 4);
                }
                match self.p.types.underlying_kind(self.ty(v)) {
                    Some(mir::TypeKind::Pointer(t)) => *t,
                    _ => unreachable!("validated pointer"),
                }
            }
            mir::PlaceBase::Static { ref symbol, ty } => {
                self.e.symbol(4, symbol, span)?;
                ty
            }
        };
        for projection in &p.projections[usize::from(carried)..] {
            match projection {
                mir::Projection::Dereference => {
                    self.e.emit(7, 4, 4, 0, 0);
                    ty = match self.p.types.underlying_kind(ty) {
                        Some(mir::TypeKind::Pointer(t)) => *t,
                        _ => unreachable!("validated dereference"),
                    };
                }
                mir::Projection::Field { index, ty: field } => {
                    let offset = layout::data(&self.p.types, ty, span)?.fields[*index as usize];
                    self.e.immediate(2, offset);
                    self.e.emit(0, 4, 4, 2, 0);
                    ty = *field;
                }
                mir::Projection::Index { index, element } => {
                    self.offset(*index, *element, span)?;
                    ty = *element;
                }
                mir::Projection::Offset { index } => self.offset(*index, ty, span)?,
            }
        }
        Ok(())
    }
    fn offset(
        &mut self,
        index: mir::ValueId,
        element: mir::TypeId,
        span: Span,
    ) -> Result<(), Diagnostic> {
        self.load(index, 2);
        self.e
            .immediate(3, layout::data(&self.p.types, element, span)?.size);
        self.e.emit(3, 2, 2, 3, 0);
        self.e.emit(0, 4, 4, 2, 0);
        Ok(())
    }
}
