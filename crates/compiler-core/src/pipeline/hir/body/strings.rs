//! Builtin name lookup, compile-time source locations, and array `len`.
use super::BodyLowerer;
use crate::{
    ir::{ast, hir},
    semantic::{TypeId, TypeKind},
    source::Span,
};

impl BodyLowerer<'_> {
    pub(super) fn builtin_name<'b>(&self, source: &'b ast::Expr) -> Option<&'b str> {
        let ast::Expr::Call { callee, .. } = source else {
            return None;
        };
        let ast::Expr::Name(path) = callee.as_ref() else {
            return None;
        };
        if path.segments.len() != 1 {
            return None;
        }
        let name = path.segments[0].name.as_str();
        let shadowed = self.names.contains_key(name)
            || self.constants.contains_key(name)
            || self
                .universe
                .iter()
                .find(|p| p.id == self.package_id)
                .is_some_and(|p| p.values.contains(name) || p.constants.contains_key(name));
        (!shadowed).then_some(name)
    }

    pub(super) fn lower_len_builtin(&mut self, source: &ast::Expr) -> Option<hir::Expr> {
        let ast::Expr::Call { args, span, .. } = source else {
            return None;
        };
        if self.builtin_name(source)? != "len" {
            return None;
        }
        if args.len() != 1 {
            return self.fail(*span, "len expects 1 argument");
        }
        let value = self.lower_expr(&args[0], None)?;
        if let Some(TypeKind::Array { length, .. }) = self.types.underlying_kind(value.ty) {
            return Some(hir::Expr {
                kind: hir::ExprKind::EvaluatedLength {
                    value: Box::new(value),
                    length: *length,
                },
                ty: TypeId::U32,
                span: *span,
            });
        }
        if let Some(call) =
            self.matching_operator_call(ast::OperatorName::Len, vec![value], *span)?
        {
            return Some(hir::Expr {
                kind: hir::ExprKind::Call(Box::new(call)),
                ty: TypeId::U32,
                span: *span,
            });
        }
        self.fail(*span, "len requires an array or matching operator")
    }

    pub(super) fn value_builtin(&self, source: &ast::Expr) -> bool {
        matches!(
            self.builtin_name(source),
            Some("len" | "thisFile" | "thisLine")
        )
    }

    pub(super) fn lower_source_builtin(&mut self, source: &ast::Expr) -> Option<hir::Expr> {
        let ast::Expr::Call { args, span, .. } = source else {
            return None;
        };
        let name @ ("thisFile" | "thisLine") = self.builtin_name(source)? else {
            return None;
        };
        if !args.is_empty() {
            return self.fail(*span, format!("{name} expects 0 arguments"));
        }
        let Some(sources) = self.sources else {
            return self.fail(
                *span,
                format!("{name} is not available in a type declaration"),
            );
        };
        if name == "thisLine" {
            let line = sources.file(span.file).line_col(span.start).0;
            let line = match u32::try_from(line) {
                Ok(line) => line,
                Err(_) => return self.fail(*span, "source line exceeds u32 range"),
            };
            return Some(hir::Expr {
                kind: hir::ExprKind::Integer(u64::from(line)),
                ty: TypeId::U32,
                span: *span,
            });
        }

        let package = self
            .universe
            .iter()
            .find(|package| package.id == self.package_id)?;
        let file_name = sources
            .file(span.file)
            .path()
            .file_name()
            .map(|name| name.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|| "<source>".to_string());
        let path = if package.logical_path == "." {
            file_name
        } else {
            format!("{}/{file_name}", package.logical_path)
        };
        let array = self.lower_string_bytes(path.into_bytes(), *span, None)?;
        let element = hir::Expr {
            kind: hir::ExprKind::Index {
                base: Box::new(array),
                index: Box::new(hir::Expr {
                    kind: hir::ExprKind::Integer(0),
                    ty: TypeId::U32,
                    span: *span,
                }),
            },
            ty: TypeId::U8,
            span: *span,
        };
        let pointer = self.types.pointer(TypeId::U8, *span);
        Some(hir::Expr {
            kind: hir::ExprKind::AddressOf(Box::new(element)),
            ty: pointer,
            span: *span,
        })
    }

    pub(super) fn lower_string_bytes(
        &mut self,
        bytes: Vec<u8>,
        span: Span,
        expected: Option<TypeId>,
    ) -> Option<hir::Expr> {
        let length = match u32::try_from(bytes.len()) {
            Ok(length) => length,
            Err(_) => return self.fail(span, "string literal is too large"),
        };
        let literal_ty = self.types.intern(
            TypeKind::Array {
                length,
                element: TypeId::U8,
            },
            span,
        );
        if expected.is_some_and(|expected| expected != literal_ty) {
            return self.fail(
                span,
                "string literal type must exactly match the destination array type",
            );
        }
        let values = bytes
            .into_iter()
            .enumerate()
            .map(|(index, byte)| {
                (
                    index as u32,
                    hir::Expr {
                        kind: hir::ExprKind::Integer(u64::from(byte)),
                        ty: TypeId::U8,
                        span,
                    },
                )
            })
            .collect();
        let value = hir::Expr {
            kind: hir::ExprKind::Composite(values),
            ty: literal_ty,
            span,
        };
        if self.constant_depth == 0 {
            Some(self.anonymous_static(value, span, true))
        } else {
            Some(value)
        }
    }
}
