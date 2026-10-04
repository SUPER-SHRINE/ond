//! Explicit pointer-to-interface conversion and static method tables.
use super::BodyLowerer;
use crate::{
    ir::{ast, hir},
    semantic::{TypeId, TypeKind},
    source::Span,
};

impl BodyLowerer<'_> {
    pub(super) fn lower_interface_conversion(
        &mut self,
        source: &ast::Expr,
        target: TypeId,
        span: Span,
    ) -> Option<hir::Expr> {
        let interface = match self.types.underlying_kind(target) {
            Some(TypeKind::Interface(interface)) => interface.clone(),
            _ => return None,
        };
        let value = self.lower_value(source, None)?;
        let Some(TypeKind::Pointer(pointee)) = self.types.underlying_kind(value.ty) else {
            return self.fail(span, "interface conversion requires a pointer value");
        };
        let Some(concrete) = self.types.get(*pointee)?.name.clone() else {
            return self.fail(
                span,
                "interface conversion requires a pointer to a named type",
            );
        };
        let interface_name = self
            .types
            .get(target)?
            .name
            .clone()
            .unwrap_or_else(|| self.types.display(target).to_string());

        let mut entries = Vec::new();
        for method in &interface.methods {
            let (logical_path, receiver_name, symbol, actual) =
                self.find_interface_implementation(value.ty, method, span)?;
            let _ = logical_path;
            let _ = receiver_name;
            let function_ty = self.types.function(method.signature.clone(), span);
            entries.push(hir::Expr {
                kind: hir::ExprKind::Function(symbol),
                ty: function_ty,
                span: actual,
            });
        }

        let Some(TypeKind::Pointer(vtable_ty)) = self.types.underlying_kind(interface.vtable)
        else {
            return self.fail(span, "invalid interface method table type");
        };
        let vtable_ty = *vtable_ty;
        let symbol = format!(
            "{}.\u{1f}vtable.{}.{}",
            self.universe
                .iter()
                .find(|package| package.id == self.package_id)?
                .logical_path,
            concrete,
            interface_name
        );
        if !self.statics.iter().any(|item| item.symbol == symbol) {
            self.statics.push(hir::Static {
                symbol: symbol.clone(),
                value: hir::Expr {
                    kind: hir::ExprKind::Composite(
                        entries
                            .into_iter()
                            .enumerate()
                            .map(|(index, value)| (index as u32, value))
                            .collect(),
                    ),
                    ty: vtable_ty,
                    span,
                },
                exported: false,
                null_terminated: false,
                span,
            });
        }
        let data = hir::Expr {
            kind: hir::ExprKind::Cast {
                value: Box::new(value),
            },
            ty: interface.data_pointer,
            span,
        };
        let table = hir::Expr {
            kind: hir::ExprKind::AddressOf(Box::new(hir::Expr {
                kind: hir::ExprKind::Global(symbol),
                ty: vtable_ty,
                span,
            })),
            ty: interface.vtable,
            span,
        };
        Some(hir::Expr {
            kind: hir::ExprKind::Composite(vec![(0, data), (1, table)]),
            ty: target,
            span,
        })
    }

    fn find_interface_implementation(
        &mut self,
        receiver_ty: TypeId,
        required: &crate::semantic::InterfaceMethod,
        span: Span,
    ) -> Option<(String, String, String, Span)> {
        let candidates = self
            .universe
            .iter()
            .flat_map(|package| {
                package
                    .methods
                    .iter()
                    .enumerate()
                    .filter(move |(_, method)| method.name.name == required.name)
                    .map(move |(index, method)| (package.id, index, method.clone()))
            })
            .collect::<Vec<_>>();
        for (package_id, declaration_index, method) in candidates {
            let Some(candidate) =
                self.resolve_method_candidate(package_id, declaration_index, &method, receiver_ty)
            else {
                continue;
            };
            let actual = candidate.signature.function_type();
            if actual.parameters[1..] != required.signature.parameters[1..]
                || actual.returns != required.signature.returns
            {
                continue;
            }
            if candidate.package_id != self.package_id
                && !required
                    .name
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_uppercase)
            {
                return self.fail(span, format!("method `{}` is not exported", required.name));
            }
            return Some((
                candidate.logical_path,
                candidate.receiver_name,
                candidate.symbol,
                candidate.span,
            ));
        }
        self.fail(
            span,
            format!(
                "pointer type does not implement interface method `{}`",
                required.name
            ),
        )
    }

    pub(super) fn lower_interface_call(
        &mut self,
        base: &ast::Expr,
        name: &str,
        args: &[ast::Expr],
        span: Span,
    ) -> Option<Option<hir::Call>> {
        let receiver = match self.lower_value(base, None) {
            Some(value) => value,
            None => return Some(None),
        };
        let interface = match self.types.underlying_kind(receiver.ty) {
            Some(TypeKind::Interface(interface)) => interface.clone(),
            _ => return None,
        };
        let Some((index, method)) = interface
            .methods
            .iter()
            .enumerate()
            .find(|(_, method)| method.name == name)
        else {
            return Some(self.fail(span, format!("unknown interface method `{name}`")));
        };
        if method
            .private_owner
            .is_some_and(|owner| owner != self.package_id)
        {
            return Some(self.fail(span, format!("method `{name}` is not exported")));
        }
        let parameters = &method.signature.parameters[1..];
        if args.len() != parameters.len() {
            return Some(self.fail(
                span,
                format!(
                    "method expects {} arguments, found {}",
                    parameters.len(),
                    args.len()
                ),
            ));
        }
        let mut arguments = Vec::new();
        for (argument, ty) in args.iter().zip(parameters) {
            arguments.push(self.lower_value(argument, Some(*ty))?);
        }
        let function_ty = self.types.function(method.signature.clone(), span);
        Some(Some(hir::Call {
            callee: Box::new(hir::Expr {
                kind: hir::ExprKind::InterfaceMethod {
                    receiver: Box::new(receiver),
                    index: index as u32,
                },
                ty: function_ty,
                span,
            }),
            signature: method.signature.clone(),
            arguments,
            span,
        }))
    }
}
