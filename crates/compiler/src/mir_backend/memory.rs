//! Typed accesses. r4/r5 are addresses; r1..r3 are scratch.
use super::{emit::Emitter, function::Context, *};
pub(super) fn address(e: &mut Emitter, reg: u8, offset: u32) {
    e.stack_address(reg, offset);
}
pub(super) fn read(e: &mut Emitter, reg: u8, base: u8, offset: u32, ty: layout::ScalarLayout) {
    e.immediate(3, offset);
    e.emit(0, 3, base, 3, 0);
    e.emit(
        match ty.size {
            1 => 4,
            2 => 5,
            _ => 7,
        },
        reg,
        3,
        0,
        0,
    );
    ops::normalize(e, reg, ty);
}
pub(super) fn write(e: &mut Emitter, reg: u8, base: u8, offset: u32, ty: layout::ScalarLayout) {
    e.immediate(3, offset);
    e.emit(0, 3, base, 3, 0);
    e.emit(
        match ty.size {
            1 => 8,
            2 => 9,
            _ => 11,
        },
        reg,
        3,
        0,
        0,
    );
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
            read(self.e, 1, 4, 0, layout::scalar(&self.p.types, ty, span)?);
            self.e.store(1, dst);
            return Ok(());
        }
        address(self.e, 5, dst);
        copy(self.e, &self.p.types, ty, span)
    }
    /// Resolve the complete symbolic place into r4 without accessing its final value.
    pub(super) fn place(&mut self, p: &mir::Place, span: Span) -> Result<(), Diagnostic> {
        let mut ty = match p.base {
            mir::PlaceBase::Local(id) => {
                address(self.e, 4, self.frame.local(id));
                self.f.locals[id.0 as usize].ty
            }
            mir::PlaceBase::Pointer(v) => {
                self.load(v, 4);
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
        for projection in &p.projections {
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
