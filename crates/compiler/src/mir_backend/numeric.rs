//! Numeric helper selection and checks; arguments in r1/r2, result in r1.
use super::{function::Context, runtime::Helper, *};
impl Context<'_> {
    pub(super) fn remainder_from_quotient(
        &mut self,
        lhs: mir::ValueId,
        rhs: mir::ValueId,
        quotient: mir::ValueId,
    ) {
        // The low bits of a - q*b agree at the operand width. The common
        // result path normalizes narrow values before storing them.
        self.load(quotient, 1);
        self.load(rhs, 2);
        self.e.emit(3, 1, 1, 2, 0);
        self.load(lhs, 3);
        self.e.emit(1, 1, 1, 1, 0);
        self.e.emit(0, 1, 3, 1, 1);
    }

    pub(super) fn float(&self, ty: mir::TypeId) -> bool {
        matches!(self.p.types.underlying_kind(ty), Some(mir::TypeKind::F32))
    }
    pub(super) fn float_positive(&mut self, ty: mir::TypeId, span: Span) {
        let wide = layout::words(&self.p.types, ty) == 2;
        let bits = if wide { 2 } else { 1 };
        let inf = if wide { 0x7ff00000 } else { 0x7f800000 };
        let done = self.e.label();
        self.e.emit(2, 3, bits, 0, 1);
        self.e.emit(2, 3, 3, 0, 0x8001u16 as i16);
        self.e.immediate(4, inf);
        if wide {
            let different = self.e.label();
            self.e.emit(13, 5, 3, 4, 0);
            self.e.near_branch(5, different, span);
            self.e.near_branch(1, done, span);
            self.e.immediate(3, inf + 1);
            self.e.bind(different);
        }
        self.e.emit(13, 5, 4, 3, 2);
        self.e.near_branch(5, done, span);
        self.e
            .immediate(bits, if wide { 0x7ff80000 } else { 0x7fc00000 });
        if wide {
            self.e.immediate(1, 0);
        }
        self.e.bind(done);
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
        let lhs_ty = self.ty(lhs);
        if self.p.types.underlying_id(lhs_ty) == Some(mir::TypeId::F64) {
            let code = match op {
                FloatAdd => 0,
                FloatSubtract => 1,
                FloatMultiply => 2,
                FloatDivide => 3,
                FloatEqual => 4,
                FloatNotEqual => 5,
                FloatLess => 6,
                FloatLessEqual => 7,
                FloatGreater => 8,
                FloatGreaterEqual => 9,
                _ => return Err(error(span, "invalid floating-point operation")),
            };
            self.load_first(lhs, 2);
            self.load_pair(rhs, 3);
            self.e.immediate(5, code);
            let helper = if code < 4 {
                Helper::Binary64
            } else {
                Helper::Compare64
            };
            self.e.call(&helper.symbol(), span)?;
            return Ok(());
        }
        if let Some(fraction) = self.p.types.fixed_fraction(lhs_ty)
            && let Some(code) = match op {
                Add => Some(0),
                Subtract => Some(1),
                Multiply => Some(2),
                Divide => Some(3),
                Remainder => Some(4),
                _ => None,
            }
        {
            self.load_wide_operand(lhs, 1, span)?;
            self.load_wide_operand(rhs, 3, span)?;
            self.e.immediate(5, fraction);
            self.e.immediate(6, code);
            self.e.call(&Helper::Fixed.symbol(), span)?;
            return Ok(());
        }
        if layout::words(&self.p.types, lhs_ty) == 2 {
            return self.wide_binary(op, lhs, rhs, span);
        }
        self.load_first(lhs, 1);
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
        let from = self.ty(value);
        if self.p.types.underlying_id(from) == self.p.types.underlying_id(to) {
            self.load_first(value, layout::words(&self.p.types, to));
            return Ok(());
        }
        let special = |ty| {
            self.p.types.fixed_fraction(ty).is_some()
                || self.p.types.underlying_id(ty) == Some(mir::TypeId::F64)
        };
        if special(from)
            || special(to)
            || (self.float(from) && layout::words(&self.p.types, to) == 2)
            || (self.float(to) && layout::words(&self.p.types, from) == 2)
        {
            let from_code = conversion_type(&self.p.types, from, span)?;
            let to_code = conversion_type(&self.p.types, to, span)?;
            self.load_wide_operand(value, 1, span)?;
            self.e.immediate(3, from_code);
            self.e.immediate(4, to_code);
            self.e.call(&Helper::ConvertWide.symbol(), span)?;
            return Ok(());
        }
        let source_words = layout::words(&self.p.types, from);
        self.load_first(value, source_words);
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
            let source = layout::scalar(&self.p.types, from, span)?;
            if layout::words(&self.p.types, to) == 2 {
                if source.size == 8 {
                    return Ok(());
                }
                if source.signed {
                    ops::sign_extend(self.e, 1, source);
                    self.e.emit(2, 2, 1, 0, 0xc01fu16 as i16);
                } else {
                    self.e.emit(0, 2, 0, 0, 0);
                }
                return Ok(());
            }
            if behavior != mir::CastBehavior::Infallible {
                return Err(error(span, "unsupported checked cast"));
            }
            ops::sign_extend(self.e, 1, layout::scalar(&self.p.types, from, span)?);
        }
        Ok(())
    }
    pub(super) fn load_wide_operand(
        &mut self,
        value: mir::ValueId,
        reg: u8,
        span: Span,
    ) -> Result<(), Diagnostic> {
        let scalar = layout::scalar(&self.p.types, self.ty(value), span)?;
        if reg == 1 {
            self.load_first(value, scalar.size.div_ceil(4));
        } else {
            self.load(value, reg);
            if scalar.size == 8 {
                self.load_word(value, 1, reg + 1);
            }
        }
        if scalar.size == 8 {
            return Ok(());
        }
        if scalar.signed {
            ops::sign_extend(self.e, reg, scalar);
            self.e.emit(2, reg + 1, reg, 0, 0xc01fu16 as i16);
        } else {
            self.e.emit(0, reg + 1, 0, 0, 0);
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
                let scalar = layout::scalar(&self.p.types, self.ty(*index), span)?;
                self.load(*index, 1);
                if scalar.size == 8 {
                    self.load_word(*index, 1, 3);
                    self.e.trap_if_nonzero(3);
                } else if scalar.signed {
                    ops::sign_extend(self.e, 1, scalar);
                    self.e.emit(13, 3, 1, 0, 1);
                    self.e.trap_if_nonzero(3);
                }
                self.load(*length, 2);
                self.e.emit(13, 1, 1, 2, 6);
                self.e.trap_if_nonzero(1);
            }
            mir::Check::NonZero { value } => {
                self.load(*value, 1);
                if layout::words(&self.p.types, self.ty(*value)) == 2 {
                    self.load_word(*value, 1, 2);
                    self.e.emit(1, 1, 1, 1, 0);
                    self.e.emit(1, 2, 2, 2, 0);
                    self.e.emit(1, 1, 1, 2, 0);
                }
                self.e.emit(13, 1, 1, 0, 0);
                self.e.trap_if_nonzero(1);
            }
            mir::Check::SignedDivisionOverflow { lhs, rhs } => {
                let scalar = layout::scalar(&self.p.types, self.ty(*lhs), span)?;
                if scalar.size == 8 {
                    let safe = self.e.label();
                    self.load_pair(*lhs, 1);
                    self.e.emit(13, 1, 1, 0, 0);
                    self.e.branch(1, safe, span);
                    self.e.immediate(3, 0x80000000);
                    self.e.emit(13, 2, 2, 3, 0);
                    self.e.branch(2, safe, span);
                    self.load_pair(*rhs, 1);
                    self.e.immediate(3, u32::MAX);
                    self.e.emit(13, 1, 1, 3, 0);
                    self.e.branch(1, safe, span);
                    self.e.emit(13, 2, 2, 3, 0);
                    self.e.trap_if_nonzero(2);
                    self.e.bind(safe);
                    return Ok(());
                }
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

fn conversion_type(types: &mir::TypeTable, ty: mir::TypeId, span: Span) -> Result<u32, Diagnostic> {
    if let Some(fraction) = types.fixed_fraction(ty) {
        return Ok(0x300 | fraction);
    }
    let scalar = layout::scalar(types, ty, span)?;
    let category = if types.is_float(ty) {
        0x200
    } else if scalar.signed {
        0x100
    } else {
        0
    };
    Ok(category | (scalar.size * 8))
}
