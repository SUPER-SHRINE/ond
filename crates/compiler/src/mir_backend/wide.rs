//! Two-word scalar lowering. Values use low word first.
use super::{emit::Emitter, function::Context, runtime::Helper, *};

impl Context<'_> {
    pub(super) fn wide_binary(
        &mut self,
        op: mir::BinaryOp,
        lhs: mir::ValueId,
        rhs: mir::ValueId,
        span: Span,
    ) -> Result<(), Diagnostic> {
        use mir::BinaryOp::*;
        self.load_first(lhs, 2);
        self.load(rhs, 3);
        if !matches!(op, ShiftLeft | ShiftRightArithmetic | ShiftRightLogical) {
            self.load_word(rhs, 1, 4);
        }
        let e = &mut *self.e;
        match op {
            Add => add(e),
            Subtract => subtract(e),
            Multiply => e.call(&Helper::Mul64.symbol(), span)?,
            Divide | Remainder => {
                let signed = self
                    .p
                    .types
                    .is_signed_integer(self.f.values[lhs.0 as usize].ty);
                let helper = match (signed, op) {
                    (true, Divide) => Helper::IDiv64,
                    (true, _) => Helper::IRem64,
                    (false, Divide) => Helper::UDiv64,
                    (false, _) => Helper::URem64,
                };
                e.call(&helper.symbol(), span)?;
            }
            BitAnd | BitOr | BitXor | BitAndNot => {
                bitwise(e, op, 1, 3);
                bitwise(e, op, 2, 4);
            }
            ShiftLeft | ShiftRightArithmetic | ShiftRightLogical => shift(e, op, span),
            Equal | NotEqual => {
                e.emit(13, 1, 1, 3, 0);
                e.emit(13, 2, 2, 4, 0);
                e.emit(1, 1, 1, 2, 0);
                e.emit(1, 1, 1, 1, 0);
                if op == NotEqual {
                    e.emit(13, 1, 1, 0, 0);
                }
            }
            LessSigned | LessEqualSigned | GreaterSigned | GreaterEqualSigned | LessUnsigned
            | LessEqualUnsigned | GreaterUnsigned | GreaterEqualUnsigned => {
                let signed = matches!(
                    op,
                    LessSigned | LessEqualSigned | GreaterSigned | GreaterEqualSigned
                );
                let swap = matches!(
                    op,
                    GreaterSigned | GreaterUnsigned | LessEqualSigned | LessEqualUnsigned
                );
                let invert = matches!(
                    op,
                    LessEqualSigned | LessEqualUnsigned | GreaterEqualSigned | GreaterEqualUnsigned
                );
                let (al, ah, bl, bh) = if swap { (3, 4, 1, 2) } else { (1, 2, 3, 4) };
                let equal_high = e.label();
                let done = e.label();
                e.emit(13, 5, ah, bh, 4);
                e.near_branch(5, equal_high, span);
                e.emit(13, 1, ah, bh, if signed { 1 } else { 2 });
                e.near_branch(0, done, span);
                e.bind(equal_high);
                e.emit(13, 1, al, bl, 2);
                e.bind(done);
                if invert {
                    e.emit(13, 1, 1, 0, 0);
                }
            }
            _ => return Err(error(span, format!("unsupported 64 bit operation {op:?}"))),
        }
        Ok(())
    }

    pub(super) fn load_pair(&mut self, id: mir::ValueId, first: u8) {
        self.load(id, first);
        self.load_word(id, 1, first + 1);
    }
}

pub(super) fn negate(e: &mut Emitter) {
    e.emit(1, 1, 1, 1, 0);
    e.emit(1, 2, 2, 2, 0);
    e.emit(0, 1, 1, 0, 1);
    e.emit(13, 5, 1, 0, 0);
    e.emit(0, 2, 2, 5, 0);
}

fn add(e: &mut Emitter) {
    e.emit(0, 5, 1, 3, 0);
    e.emit(13, 6, 5, 1, 2);
    e.emit(0, 2, 2, 4, 0);
    e.emit(0, 2, 2, 6, 0);
    e.emit(0, 1, 5, 0, 0);
}

fn subtract(e: &mut Emitter) {
    e.emit(13, 6, 1, 3, 2);
    e.emit(1, 3, 3, 3, 0);
    e.emit(1, 4, 4, 4, 0);
    e.emit(1, 6, 6, 6, 0);
    e.emit(0, 6, 6, 0, 1);
    e.emit(0, 1, 1, 3, 1);
    e.emit(0, 2, 2, 4, 1);
    e.emit(0, 2, 2, 6, 0);
}

fn bitwise(e: &mut Emitter, op: mir::BinaryOp, left: u8, right: u8) {
    use mir::BinaryOp::*;
    match op {
        BitAnd => {
            e.emit(1, left, left, right, 0);
            e.emit(1, left, left, left, 0);
        }
        BitOr => {
            e.emit(1, left, left, left, 0);
            e.emit(1, right, right, right, 0);
            e.emit(1, left, left, right, 0);
        }
        BitXor => {
            e.emit(1, 5, left, right, 0);
            e.emit(1, left, left, 5, 0);
            e.emit(1, right, right, 5, 0);
            e.emit(1, left, left, right, 0);
        }
        BitAndNot => {
            e.emit(1, right, right, right, 0);
            e.emit(1, left, left, right, 0);
            e.emit(1, left, left, left, 0);
        }
        _ => unreachable!(),
    }
}

fn shift(e: &mut Emitter, op: mir::BinaryOp, span: Span) {
    use mir::BinaryOp::*;
    let done = e.label();
    let small = e.label();
    e.emit(2, 3, 3, 0, 26);
    e.emit(2, 3, 3, 0, 0x801au16 as i16);
    e.near_branch(3, done, span);
    e.emit(2, 5, 3, 0, 0x8005u16 as i16);
    e.near_branch(5, small, span);
    let logical = 0x8000u16 as i16;
    let arithmetic = 0xc000u16 as i16;
    let mode = if op == ShiftRightArithmetic {
        arithmetic
    } else {
        logical
    };
    // Kagura takes the low five bits of the count. This is count-32 here.
    if op == ShiftLeft {
        e.emit(2, 2, 1, 3, 0);
        e.emit(0, 1, 0, 0, 0);
    } else {
        e.emit(2, 1, 2, 3, mode);
        if op == ShiftRightArithmetic {
            e.emit(2, 2, 2, 0, (0xc000u16 | 31) as i16);
        } else {
            e.emit(0, 2, 0, 0, 0);
        }
    }
    e.near_branch(0, done, span);
    e.bind(small);
    e.emit(1, 5, 3, 3, 0);
    e.emit(0, 5, 5, 0, 1);
    if op == ShiftLeft {
        e.emit(2, 6, 1, 5, logical);
        e.emit(2, 1, 1, 3, 0);
        e.emit(2, 2, 2, 3, 0);
        e.emit(0, 2, 2, 6, 0);
    } else {
        e.emit(2, 6, 2, 5, 0);
        e.emit(2, 1, 1, 3, logical);
        e.emit(2, 2, 2, 3, mode);
        e.emit(0, 1, 1, 6, 0);
    }
    e.bind(done);
}
