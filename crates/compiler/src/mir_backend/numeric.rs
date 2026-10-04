//! Numeric helper selection and checks; arguments in r1/r2, result in r1.
use super::{function::Context, runtime::Helper, *};
impl Context<'_> {
    pub(super) fn float(&self, ty: mir::TypeId) -> bool {
        matches!(self.p.types.underlying_kind(ty), Some(mir::TypeKind::F32))
    }
    pub(super) fn numeric_binary(
        &mut self,
        op: mir::BinaryOp,
        lhs: mir::ValueId,
        rhs: mir::ValueId,
        overflow: mir::OverflowBehavior,
        span: Span,
    ) -> Result<(), Diagnostic> {
        use mir::BinaryOp::*;
        self.load(lhs, 1);
        self.load(rhs, 2);
        let helper = match op {
            FloatAdd => Some(Helper::F32Add),
            FloatSubtract => Some(Helper::F32Sub),
            FloatMultiply => Some(Helper::F32Mul),
            FloatDivide => Some(Helper::F32Div),
            FloatEqual => Some(Helper::F32Eq),
            FloatNotEqual => Some(Helper::F32Ne),
            FloatLess => Some(Helper::F32Lt),
            FloatLessEqual => Some(Helper::F32Le),
            FloatGreater => Some(Helper::F32Gt),
            FloatGreaterEqual => Some(Helper::F32Ge),
            Divide | Remainder => {
                let scalar = layout::scalar(&self.p.types, self.ty(lhs), span)?;
                ops::sign_extend(self.e, 1, scalar);
                ops::sign_extend(self.e, 2, scalar);
                Some(match (scalar.signed, op == Divide) {
                    (true, true) => Helper::IDiv,
                    (true, false) => Helper::IRem,
                    (false, true) => Helper::UDiv,
                    (false, false) => Helper::URem,
                })
            }
            _ => None,
        };
        if let Some(h) = helper {
            self.e.call(&h.symbol(), span)?;
            return Ok(());
        }
        if overflow != mir::OverflowBehavior::Wrap {
            return Err(error(span, "unsupported checked binary operation"));
        }
        ops::binary(
            self.e,
            op,
            layout::scalar(&self.p.types, self.ty(lhs), span)?,
            span,
        )
    }
    pub(super) fn numeric_cast(
        &mut self,
        value: mir::ValueId,
        to: mir::TypeId,
        behavior: mir::CastBehavior,
        span: Span,
    ) -> Result<(), Diagnostic> {
        self.load(value, 1);
        let from = self.ty(value);
        if self.float(from) && self.float(to) {
            return Ok(());
        }
        if self.float(to) {
            let source = layout::scalar(&self.p.types, from, span)?;
            ops::sign_extend(self.e, 1, source);
            self.e.call(
                &if source.signed {
                    Helper::F32FromI32
                } else {
                    Helper::F32FromU32
                }
                .symbol(),
                span,
            )?;
        } else if self.float(from) {
            let dest = layout::scalar(&self.p.types, to, span)?;
            self.e.call(
                &if dest.signed {
                    Helper::F32ToI32
                } else {
                    Helper::F32ToU32
                }
                .symbol(),
                span,
            )?;
            if dest.size < 4 {
                let bits = dest.size * 8;
                let max = if dest.signed {
                    (1u32 << (bits - 1)) - 1
                } else {
                    (1u32 << bits) - 1
                };
                self.e.immediate(2, max);
                self.e.emit(13, 3, 2, 1, if dest.signed { 1 } else { 2 });
                self.e.trap_if_nonzero(3);
                if dest.signed {
                    self.e.immediate(2, (0u32).wrapping_sub(1 << (bits - 1)));
                    self.e.emit(13, 3, 1, 2, 1);
                    self.e.trap_if_nonzero(3);
                }
            }
        } else {
            if behavior != mir::CastBehavior::Infallible {
                return Err(error(span, "unsupported checked cast"));
            }
            ops::sign_extend(self.e, 1, layout::scalar(&self.p.types, from, span)?);
        }
        Ok(())
    }
    pub(super) fn numeric_check(
        &mut self,
        check: &mir::Check,
        span: Span,
    ) -> Result<(), Diagnostic> {
        match check {
            mir::Check::Bounds { index, length } => {
                self.load(*index, 1);
                self.load(*length, 2);
                self.e.emit(13, 1, 1, 2, 6);
                self.e.trap_if_nonzero(1);
            }
            mir::Check::NonZero { value } => {
                self.load(*value, 1);
                self.e.emit(13, 1, 1, 0, 0);
                self.e.trap_if_nonzero(1);
            }
            mir::Check::SignedDivisionOverflow { lhs, rhs } => {
                let scalar = layout::scalar(&self.p.types, self.ty(*lhs), span)?;
                self.load(*lhs, 1);
                self.e.immediate(2, 1 << (scalar.size * 8 - 1));
                self.e.emit(13, 1, 1, 2, 0);
                let done = self.e.label();
                self.e.branch(1, done, span);
                self.load(*rhs, 1);
                self.e.immediate(
                    2,
                    if scalar.size == 4 {
                        u32::MAX
                    } else {
                        (1 << (scalar.size * 8)) - 1
                    },
                );
                self.e.emit(13, 1, 1, 2, 0);
                self.e.trap_if_nonzero(1);
                self.e.bind(done);
            }
        }
        Ok(())
    }
}
