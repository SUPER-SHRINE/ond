//! Retain exact positive literal values with logarithmic expression depth.
use crate::{
    ir::{ast, hir},
    semantic::TypeId,
    source::Span,
};

fn literal(bits: u64, ty: TypeId, span: Span) -> hir::Expr {
    hir::Expr {
        kind: hir::ExprKind::Integer(bits),
        ty,
        span,
    }
}
fn binary(op: ast::BinaryOp, lhs: hir::Expr, rhs: hir::Expr, ty: TypeId, span: Span) -> hir::Expr {
    hir::Expr {
        kind: hir::ExprKind::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        },
        ty,
        span,
    }
}
fn radix_power(exponent: usize, ty: TypeId, span: Span) -> hir::Expr {
    if exponent <= 8 {
        return literal(1u64 << (7 * exponent), ty, span);
    }
    let middle = exponent / 2;
    binary(
        ast::BinaryOp::Mul,
        radix_power(middle, ty, span),
        radix_power(exponent - middle, ty, span),
        ty,
        span,
    )
}
pub(super) fn balanced(digits: &[u8], ty: TypeId, span: Span) -> hir::Expr {
    // Nine radix-128 digits fit a positive i64, preserving the distinction
    // between source literals and evaluated sign-extended integer bits.
    if digits.len() <= 9 {
        return literal(
            digits
                .iter()
                .fold(0u64, |bits, digit| bits * 128 + u64::from(*digit)),
            ty,
            span,
        );
    }
    let middle = digits.len() / 2;
    binary(
        ast::BinaryOp::Add,
        binary(
            ast::BinaryOp::Mul,
            balanced(&digits[..middle], ty, span),
            radix_power(digits.len() - middle, ty, span),
            ty,
            span,
        ),
        balanced(&digits[middle..], ty, span),
        ty,
        span,
    )
}
