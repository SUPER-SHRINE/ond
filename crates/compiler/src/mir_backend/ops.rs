//! Integer selection. r1/r2 operands, r1 result; r3 scratch.
use super::{emit::Emitter, layout::ScalarLayout, *};
pub(super) fn normalize(e: &mut Emitter, reg: u8, ty: ScalarLayout) {
    if ty.boolean {
        e.emit(13, reg, reg, 0, 4);
    } else if ty.size < 4 {
        let n = (32 - ty.size * 8) as i16;
        e.emit(2, reg, reg, 0, n);
        e.emit(2, reg, reg, 0, (0x8000u16 | n as u16) as i16);
    }
}
pub(super) fn sign_extend(e: &mut Emitter, reg: u8, ty: ScalarLayout) {
    if ty.signed && ty.size < 4 {
        let n = (32 - ty.size * 8) as i16;
        e.emit(2, reg, reg, 0, n);
        e.emit(2, reg, reg, 0, (0xc000u16 | n as u16) as i16);
    }
}
pub(super) fn unary(e: &mut Emitter, op: mir::UnaryOp) {
    match op {
        mir::UnaryOp::Negate => {
            e.emit(1, 1, 1, 1, 0);
            e.emit(0, 1, 1, 0, 1);
        }
        mir::UnaryOp::BitNot => e.emit(1, 1, 1, 1, 0),
        mir::UnaryOp::LogicalNot => e.emit(13, 1, 1, 0, 0),
    }
}
pub(super) fn binary(
    e: &mut Emitter,
    op: mir::BinaryOp,
    ty: ScalarLayout,
    span: Span,
) -> Result<(), Diagnostic> {
    use mir::BinaryOp::*;
    match op {
        Add => e.emit(0, 1, 1, 2, 0),
        Subtract => {
            e.emit(1, 2, 2, 2, 0);
            e.emit(0, 2, 2, 0, 1);
            e.emit(0, 1, 1, 2, 0);
        }
        Multiply => e.emit(3, 1, 1, 2, 0),
        BitAnd => {
            e.emit(1, 1, 1, 2, 0);
            e.emit(1, 1, 1, 1, 0);
        }
        BitOr => {
            e.emit(1, 1, 1, 1, 0);
            e.emit(1, 2, 2, 2, 0);
            e.emit(1, 1, 1, 2, 0);
        }
        BitXor => {
            e.emit(1, 3, 1, 2, 0);
            e.emit(1, 1, 1, 3, 0);
            e.emit(1, 2, 2, 3, 0);
            e.emit(1, 1, 1, 2, 0);
        }
        BitAndNot => {
            e.emit(1, 2, 2, 2, 0);
            e.emit(1, 1, 1, 2, 0);
            e.emit(1, 1, 1, 1, 0);
        }
        ShiftLeft | ShiftRightLogical | ShiftRightArithmetic => {
            if ty.size < 4 {
                let shift = if ty.size == 1 { 29 } else { 28 };
                e.emit(2, 2, 2, 0, shift);
                e.emit(2, 2, 2, 0, (0x8000u16 | shift as u16) as i16);
            }
            if op == ShiftRightArithmetic {
                sign_extend(e, 1, ty);
            }
            let mode = match op {
                ShiftLeft => 0,
                ShiftRightLogical => 0x8000u16,
                _ => 0xc000u16,
            };
            e.emit(2, 1, 1, 2, mode as i16);
        }
        Equal | NotEqual => e.emit(13, 1, 1, 2, if op == Equal { 0 } else { 4 }),
        LessSigned | LessEqualSigned | GreaterSigned | GreaterEqualSigned | LessUnsigned
        | LessEqualUnsigned | GreaterUnsigned | GreaterEqualUnsigned => {
            let signed = matches!(
                op,
                LessSigned | LessEqualSigned | GreaterSigned | GreaterEqualSigned
            );
            if signed {
                sign_extend(e, 1, ty);
                sign_extend(e, 2, ty);
            }
            let swap = matches!(
                op,
                GreaterSigned | GreaterUnsigned | LessEqualSigned | LessEqualUnsigned
            );
            let invert = matches!(
                op,
                LessEqualSigned | LessEqualUnsigned | GreaterEqualSigned | GreaterEqualUnsigned
            );
            let mode = if signed { 1 } else { 2 } | if invert { 4 } else { 0 };
            e.emit(
                13,
                1,
                if swap { 2 } else { 1 },
                if swap { 1 } else { 2 },
                mode,
            );
        }
        _ => return Err(error(span, format!("unsupported binary operation {op:?}"))),
    }
    Ok(())
}
