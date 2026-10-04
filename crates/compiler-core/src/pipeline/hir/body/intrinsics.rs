//! Typed memory runtime operations, without target addresses or allocation sizes.
use super::BodyLowerer;
use crate::{
    ir::{ast, hir},
    semantic::{FunctionType, Intrinsic, TypeId, TypeKind},
    source::Span,
};

impl BodyLowerer<'_> {
    fn intrinsic(
        &mut self,
        op: Intrinsic,
        arguments: Vec<hir::Expr>,
        returns: Vec<TypeId>,
        span: Span,
    ) -> hir::Call {
        let signature = FunctionType {
            parameters: arguments.iter().map(|arg| arg.ty).collect(),
            returns,
        };
        let ty = self.types.function(signature.clone(), span);
        hir::Call {
            callee: Box::new(hir::Expr {
                kind: hir::ExprKind::Intrinsic(op),
                ty,
                span,
            }),
            signature,
            arguments,
            span,
        }
    }

    pub(super) fn lower_new(&mut self, ty: &ast::Type, span: Span) -> Option<hir::Expr> {
        let element = self.resolve_type(ty)?;
        let pointer = self.types.pointer(element, span);
        let call = self.intrinsic(Intrinsic::New(element), Vec::new(), vec![pointer], span);
        Some(hir::Expr {
            kind: hir::ExprKind::Call(Box::new(call)),
            ty: pointer,
            span,
        })
    }

    pub(super) fn lower_intrinsic(
        &mut self,
        name: &str,
        args: &[ast::Expr],
        span: Span,
    ) -> Option<hir::Call> {
        let count = match name {
            "store32" => 2,
            _ => 1,
        };
        if args.len() != count {
            return self.fail(span, format!("{name} expects {count} arguments"));
        }
        let mut arguments = Vec::new();
        let (op, returns) = match name {
            "free" => {
                let expected = if matches!(args[0], ast::Expr::Literal(ast::Literal::Nil(_))) {
                    Some(self.types.pointer(TypeId::U8, span))
                } else {
                    None
                };
                let pointer = self.lower_value(&args[0], expected)?;
                if !matches!(
                    self.types.underlying_kind(pointer.ty),
                    Some(TypeKind::Pointer(_))
                ) {
                    return self.fail(span, "free requires a pointer");
                }
                arguments.push(pointer);
                (Intrinsic::Free, Vec::new())
            }
            "load32" | "store32" => {
                let expected = matches!(args[0], ast::Expr::Literal(ast::Literal::Int(_, _)))
                    .then_some(TypeId::U32);
                let address = self.lower_value(&args[0], expected)?;
                if address.ty != TypeId::U32
                    && !matches!(
                        self.types.underlying_kind(address.ty),
                        Some(TypeKind::Pointer(_))
                    )
                {
                    return self.fail(span, "memory address requires u32 or pointer");
                }
                arguments.push(address);
                if name == "store32" {
                    arguments.push(self.lower_value(&args[1], Some(TypeId::U32))?);
                    (Intrinsic::Store32, Vec::new())
                } else {
                    (Intrinsic::Load32, vec![TypeId::U32])
                }
            }
            _ => return self.fail(span, "unknown compiler intrinsic"),
        };
        Some(self.intrinsic(op, arguments, returns, span))
    }

    pub(super) fn lower_typed_alloc(
        &mut self,
        element: TypeId,
        args: &[ast::Expr],
        span: Span,
    ) -> Option<hir::Call> {
        if args.len() != 1 {
            return self.fail(span, "alloc expects 1 argument");
        }
        let count = self.lower_value(&args[0], Some(TypeId::U32))?;
        let pointer = self.types.pointer(element, span);
        Some(self.intrinsic(Intrinsic::Alloc(element), vec![count], vec![pointer], span))
    }
}
