//! Typed accesses. r4/r5 are addresses; r1..r3 are scratch.
use super::{emit::Emitter, function::Context, *};
pub(super) fn address(e: &mut Emitter, reg: u8, offset: u32) {
    e.stack_address(reg, offset);
}
pub(super) fn read(e: &mut Emitter, reg: u8, base: u8, offset: u32, ty: layout::ScalarLayout) {
    let (base, imm) = access_address(e, base, offset, ty);
    e.emit(
        match ty.size {
            1 => 4,
            2 => 5,
            _ => 7,
        },
        reg,
        base,
        0,
        imm,
    );
    if ty.size == 8 {
        e.emit(7, reg + 1, base, 0, imm + 4);
    }
    ops::normalize(e, reg, ty);
}
pub(super) fn write(e: &mut Emitter, reg: u8, base: u8, offset: u32, ty: layout::ScalarLayout) {
    let (base, imm) = access_address(e, base, offset, ty);
    e.emit(store_opcode(ty), reg, base, 0, imm);
    if ty.size == 8 {
        e.emit(11, if reg == 0 { 0 } else { reg + 1 }, base, 0, imm + 4);
    }
}
fn access_address(e: &mut Emitter, base: u8, offset: u32, ty: layout::ScalarLayout) -> (u8, i16) {
    let last = if ty.size == 8 { 4 } else { 0 };
    if offset <= i16::MAX as u32 - last {
        (base, offset as i16)
    } else {
        e.immediate(3, offset);
        e.emit(0, 3, base, 3, 0);
        (3, 0)
    }
}
pub(super) fn store_opcode(ty: layout::ScalarLayout) -> u32 {
    match ty.size {
        1 => 8,
        2 => 9,
        _ => 11,
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
