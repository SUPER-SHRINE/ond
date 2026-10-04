//! Mandatory constant evaluation at value boundaries, not an optimization pass.
use super::BodyLowerer;
use crate::{
    ir::{ast, hir},
    semantic::TypeId,
};

impl BodyLowerer<'_> {
    pub(super) fn short_circuit_result(&self, op: ast::BinaryOp, lhs: &hir::Expr) -> Option<bool> {
        if !matches!(op, ast::BinaryOp::AndAnd | ast::BinaryOp::OrOr) {
            return None;
        }
        let value = crate::constant::evaluate(lhs, self.types).ok()?;
        match (op, value.kind) {
            (ast::BinaryOp::AndAnd, hir::ExprKind::Bool(false)) => Some(false),
            (ast::BinaryOp::OrOr, hir::ExprKind::Bool(true)) => Some(true),
            _ => None,
        }
    }

    pub(super) fn lower_value(
        &mut self,
        source: &ast::Expr,
        expected: Option<TypeId>,
    ) -> Option<hir::Expr> {
        let value = self.lower_expr(source, expected)?;
        self.finish_value(value)
    }

    pub(super) fn finish_value(&mut self, mut value: hir::Expr) -> Option<hir::Expr> {
        if self.defer_value_evaluation {
            return Some(value);
        }
        // Evaluate the whole constant subtree before checking its result type.
        // Checking each arithmetic node would incorrectly reject (250 + 10) - 10:u8.
        if crate::constant::is_constant(&value, self.types) {
            return match crate::constant::evaluate(&value, self.types) {
                Ok(value) => Some(value),
                Err(message) => self.fail(value.span, message),
            };
        }
        // Value contexts may contain calls on the skipped branch. Constant
        // declarations still reject those via the separate eligibility check.
        if let hir::ExprKind::Binary { op, lhs, .. } = &value.kind {
            if let Some(result) = self.short_circuit_result(*op, lhs) {
                value.kind = hir::ExprKind::Bool(result);
                return Some(value);
            }
        }
        // A runtime expression can contain independent constant operands. Never
        // propagate a local/global's initializer: those operations must still wrap.
        let mut finish = |operand: &mut Box<hir::Expr>| -> Option<()> {
            **operand = self.finish_value((**operand).clone())?;
            Some(())
        };
        match &mut value.kind {
            hir::ExprKind::Unary { operand, .. }
            | hir::ExprKind::Cast { value: operand }
            | hir::ExprKind::Dereference(operand) => finish(operand)?,
            hir::ExprKind::Binary { lhs, rhs, .. } => {
                finish(lhs)?;
                finish(rhs)?;
            }
            hir::ExprKind::Field { base, .. } => finish(base)?,
            hir::ExprKind::Index { base, index } => {
                finish(base)?;
                finish(index)?;
            }
            hir::ExprKind::Call(call) => {
                finish(&mut call.callee)?;
                for argument in &mut call.arguments {
                    *argument = self.finish_value(argument.clone())?;
                }
            }
            hir::ExprKind::Composite(elements) => {
                for (_, element) in elements {
                    *element = self.finish_value(element.clone())?;
                }
            }
            // Keep places intact: evaluation must not turn an lvalue into an rvalue.
            _ => {}
        }
        Some(value)
    }
}
