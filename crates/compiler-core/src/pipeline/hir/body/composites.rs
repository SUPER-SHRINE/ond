//! Sparse aggregate initializers, kept in source evaluation order.
use super::BodyLowerer;
use crate::{
    ir::{ast, hir},
    semantic::TypeKind,
    source::Span,
};
use std::collections::BTreeSet;

impl BodyLowerer<'_> {
    pub(super) fn lower_composite(
        &mut self,
        source: &ast::Type,
        elements: &[ast::CompositeElement],
        span: Span,
    ) -> Option<hir::Expr> {
        let inferred = matches!(source, ast::Type::Array { len: None, .. });
        let (ty, kind) = if let ast::Type::Array {
            len: None, element, ..
        } = source
        {
            if elements.is_empty() {
                return self.fail(span, "cannot infer array length from an empty literal");
            }
            let element = self.resolve_type(element)?;
            // Use the representable upper bound only for index checking; intern
            // only the final, inferred type after examining every element.
            (
                None,
                TypeKind::Array {
                    length: u32::MAX,
                    element,
                },
            )
        } else {
            let ty = self.resolve_type(source)?;
            (Some(ty), self.types.underlying_kind(ty)?.clone())
        };
        let mut next = 0u64;
        let mut inferred_length = 0u32;
        let mut seen = BTreeSet::new();
        let mut values = Vec::new();
        for element in elements {
            let (index, element_ty) = match &kind {
                TypeKind::Array {
                    length,
                    element: element_ty,
                } => {
                    let index = if let Some(key) = &element.key {
                        let key = self.lower_value(key, None)?;
                        let Some(index) = self.constant_index(&key)? else {
                            return self.fail(
                                element.span,
                                "array literal key must be a compile-time integer",
                            );
                        };
                        index
                    } else {
                        next
                    };
                    if index >= u64::from(*length) {
                        return self.fail(element.span, "array literal index out of bounds");
                    }
                    next = index + 1;
                    inferred_length = inferred_length.max(next as u32);
                    (index as u32, *element_ty)
                }
                TypeKind::Struct(_) => {
                    let Some(ast::Expr::Name(path)) = &element.key else {
                        return self.fail(element.span, "struct literal requires named fields");
                    };
                    if path.segments.len() != 1 {
                        return self.fail(element.span, "invalid struct literal field name");
                    }
                    self.field(ty?, &path.segments[0].name, element.span)?
                }
                _ => return self.fail(span, "composite literal requires an array or struct type"),
            };
            if !seen.insert(index) {
                return self.fail(
                    element.span,
                    "duplicate field or index in composite literal",
                );
            }
            values.push((index, self.lower_value(&element.value, Some(element_ty))?));
        }
        if !matches!(kind, TypeKind::Array { .. } | TypeKind::Struct(_)) {
            return self.fail(span, "composite literal requires an array or struct type");
        }
        let ty = if inferred {
            let TypeKind::Array { element, .. } = kind else {
                unreachable!()
            };
            self.types.intern(
                TypeKind::Array {
                    length: inferred_length,
                    element,
                },
                span,
            )
        } else {
            ty?
        };
        Some(hir::Expr {
            kind: hir::ExprKind::Composite(values),
            ty,
            span,
        })
    }
}
