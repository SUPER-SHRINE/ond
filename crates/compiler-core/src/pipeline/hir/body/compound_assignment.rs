//! Resolve the binary operation of a storage compound assignment before builtin fallback.
use super::{
    BodyLowerer,
    expressions::{binary_operator_name, contextual_literal},
};
use crate::{
    ir::{ast, hir},
    semantic::{TypeId, TypeKind},
};

impl BodyLowerer<'_> {
    pub(super) fn implicit_one(
        &mut self,
        operand: TypeId,
        rhs: TypeId,
        span: crate::source::Span,
    ) -> Option<hir::Expr> {
        if !self.types.is_integer(operand)
            && !matches!(self.types.underlying_kind(operand), Some(TypeKind::F32))
        {
            return self.fail(span, "increment or decrement requires an integer or f32");
        }
        let kind = if self.types.is_integer(rhs) {
            hir::ExprKind::Integer(1)
        } else if matches!(self.types.underlying_kind(rhs), Some(TypeKind::F32)) {
            hir::ExprKind::Float32(1.0f32.to_bits())
        } else {
            return self.fail(span, "increment or decrement operand type mismatch");
        };
        Some(hir::Expr {
            kind,
            ty: rhs,
            span,
        })
    }

    pub(super) fn compound_values(
        &mut self,
        assignment: &ast::AssignStmt,
        target: &hir::Expr,
        op: ast::BinaryOp,
    ) -> Option<(hir::ValueList, Option<hir::Call>)> {
        let integer = self.types.is_integer(target.ty);
        let float = matches!(self.types.underlying_kind(target.ty), Some(TypeKind::F32));
        let builtin = integer
            || float
                && matches!(
                    op,
                    ast::BinaryOp::Add
                        | ast::BinaryOp::Sub
                        | ast::BinaryOp::Mul
                        | ast::BinaryOp::Div
                );
        let builtin_rhs = if matches!(op, ast::BinaryOp::ShiftLeft | ast::BinaryOp::ShiftRight) {
            TypeId::U32
        } else {
            target.ty
        };
        let name = binary_operator_name(op);
        let custom_rhs = name
            .and_then(|name| {
                self.unique_operator_parameter_types(
                    name,
                    target,
                    &[Some(target.ty), None],
                    assignment.span,
                )
            })
            .map(|parameters| parameters[1]);
        let values = if assignment.implicit_one {
            hir::ValueList::Expressions(vec![self.implicit_one(
                target.ty,
                custom_rhs.unwrap_or(builtin_rhs),
                assignment.span,
            )?])
        } else {
            let expected = assignment
                .values
                .first()
                .filter(|value| contextual_literal(value))
                .map(|_| custom_rhs.unwrap_or(builtin_rhs));
            self.lower_values(
                &assignment.values,
                &[expected],
                assignment.span,
                "compound assignment",
            )?
        };
        let value_types = values.types();
        let [rhs_ty] = value_types.as_slice() else {
            return self.fail(assignment.span, "compound assignment requires one value");
        };
        let overloaded = if let Some(name) = name {
            self.matching_operator_call(
                name,
                vec![
                    target.clone(),
                    hir::Expr {
                        kind: hir::ExprKind::Zero,
                        ty: *rhs_ty,
                        span: assignment.span,
                    },
                ],
                assignment.span,
            )?
        } else {
            None
        };
        if let Some(call) = &overloaded {
            let [result] = call.signature.returns.as_slice() else {
                return self.fail(assignment.span, "binary operator must return one value");
            };
            if *result != target.ty {
                return self.type_mismatch(assignment.span, target.ty, *result);
            }
        } else {
            if !builtin {
                return self.fail(assignment.span, "invalid compound assignment operand type");
            }
            if *rhs_ty != builtin_rhs {
                return self.type_mismatch(
                    assignment
                        .values
                        .first()
                        .map_or(assignment.span, ast::Expr::span),
                    builtin_rhs,
                    *rhs_ty,
                );
            }
        }
        Some((values, overloaded))
    }
}
