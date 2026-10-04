//! Addressability, pointer dereferences, fields and checked indexing.
use super::BodyLowerer;
use crate::{
    ir::{ast, hir},
    semantic::{TypeId, TypeKind},
    source::Span,
};

impl BodyLowerer<'_> {
    pub(super) fn read_only_target(&self, source: &ast::Expr) -> bool {
        match source {
            ast::Expr::Literal(ast::Literal::String(_, _)) => true,
            ast::Expr::Name(path) if path.segments.len() == 1 => {
                let name = &path.segments[0].name;
                self.constants.contains_key(name)
                    || (!self.names.contains_key(name)
                        && self
                            .universe
                            .iter()
                            .find(|package| package.id == self.package_id)
                            .is_some_and(|package| package.constants.contains_key(name)))
            }
            ast::Expr::Selector { base, field, .. } => {
                if let ast::Expr::Name(path) = base.as_ref()
                    && path.segments.len() == 1
                    && let Some(package_id) =
                        self.imported_package(&path.segments[0].name, path.span)
                {
                    return self
                        .universe
                        .iter()
                        .find(|package| package.id == package_id)
                        .is_some_and(|package| package.constants.contains_key(&field.name));
                }
                self.read_only_target(base)
            }
            ast::Expr::Index { base, .. } => self.read_only_target(base),
            ast::Expr::Unary {
                op: ast::UnaryOp::Deref,
                expr,
                ..
            } => self.read_only_target(expr),
            _ => false,
        }
    }

    pub(super) fn addressable(&self, expr: &hir::Expr) -> bool {
        match &expr.kind {
            hir::ExprKind::Local(_) | hir::ExprKind::Global(_) | hir::ExprKind::Dereference(_) => {
                true
            }
            hir::ExprKind::Field { base, .. } => self.addressable(base),
            hir::ExprKind::Index { base, .. } => match self.types.underlying_kind(base.ty) {
                Some(TypeKind::Pointer(_)) => true,
                Some(TypeKind::Array { .. }) => self.addressable(base),
                _ => false,
            },
            _ => false,
        }
    }

    pub(super) fn lower_target(&mut self, source: &ast::Expr) -> Option<hir::Expr> {
        let value = self.lower_expr(source, None)?;
        if !self.addressable(&value) {
            return self.fail(
                source.span(),
                "assignment target is not addressable (string bytes are read-only)",
            );
        }
        Some(value)
    }

    pub(super) fn field(&mut self, ty: TypeId, name: &str, span: Span) -> Option<(u32, TypeId)> {
        let Some(TypeKind::Struct(structure)) = self.types.underlying_kind(ty) else {
            return self.fail(span, "field access requires a struct");
        };
        let Some((index, field)) = structure
            .fields
            .iter()
            .enumerate()
            .find(|(_, f)| f.name == name)
        else {
            return self.fail(span, format!("unknown struct field `{name}`"));
        };
        if field
            .private_owner
            .is_some_and(|owner| owner != self.package_id)
        {
            return self.fail(span, format!("field `{name}` is not exported"));
        }
        Some((index as u32, field.ty))
    }

    pub(super) fn lower_field(
        &mut self,
        source: &ast::Expr,
        name: &str,
        span: Span,
    ) -> Option<hir::Expr> {
        let mut base = self.lower_expr(source, None)?;
        if let Some(TypeKind::Pointer(pointee)) = self.types.underlying_kind(base.ty) {
            base = hir::Expr {
                ty: *pointee,
                span: base.span,
                kind: hir::ExprKind::Dereference(Box::new(base)),
            };
        }
        let (index, ty) = self.field(base.ty, name, span)?;
        Some(hir::Expr {
            kind: hir::ExprKind::Field {
                base: Box::new(base),
                index,
            },
            ty,
            span,
        })
    }

    pub(super) fn lower_index(
        &mut self,
        source: &ast::Expr,
        index: &ast::Expr,
        span: Span,
    ) -> Option<hir::Expr> {
        let base = self.lower_expr(source, None)?;
        let expected_index = self
            .unique_operator_parameter_types(ast::OperatorName::Index, &[Some(base.ty), None])
            .map(|parameters| parameters[1])
            .or_else(|| {
                matches!(
                    self.types.underlying_kind(base.ty),
                    Some(TypeKind::Pointer(_))
                )
                .then_some(TypeId::U32)
            });
        let lowered_index = self.lower_value(index, expected_index)?;
        if let Some(call) = self.matching_operator_call(
            ast::OperatorName::Index,
            vec![base.clone(), lowered_index.clone()],
            span,
        )? {
            let [ty] = call.signature.returns.as_slice() else {
                return self.fail(span, "index operator must return one value");
            };
            let ty = *ty;
            return Some(hir::Expr {
                kind: hir::ExprKind::Call(Box::new(call)),
                ty,
                span,
            });
        }
        let (ty, length, raw) = match self.types.underlying_kind(base.ty) {
            Some(TypeKind::Array { length, element }) => (*element, Some(*length), false),
            Some(TypeKind::Pointer(element)) => (*element, None, true),
            _ => {
                return self.fail(
                    span,
                    "indexing requires an array, pointer, or matching operator",
                );
            }
        };
        let index = if raw && lowered_index.ty != TypeId::U32 {
            self.lower_value(index, Some(TypeId::U32))?
        } else {
            lowered_index
        };
        if !self.types.is_integer(index.ty) {
            return self.fail(index.span, "index must be an integer");
        }
        if let Some(length) = length.filter(|_| !self.defer_value_evaluation) {
            if let Some(value) = self.constant_index(&index)? {
                if value >= u64::from(length) {
                    return self.fail(index.span, "constant index out of bounds");
                }
            }
        }
        Some(hir::Expr {
            kind: hir::ExprKind::Index {
                base: Box::new(base),
                index: Box::new(index),
            },
            ty,
            span,
        })
    }

    pub(super) fn constant_index(&mut self, expr: &hir::Expr) -> Option<Option<u64>> {
        if !crate::constant::is_constant(expr, self.types) {
            return Some(None);
        }
        match crate::constant::evaluate(expr, self.types) {
            Ok(hir::Expr {
                kind: hir::ExprKind::Integer(value),
                ..
            }) => Some(Some(value)),
            Ok(_) => self.fail(expr.span, "index must be an integer constant"),
            Err(message) => self.fail(expr.span, message),
        }
    }
}
