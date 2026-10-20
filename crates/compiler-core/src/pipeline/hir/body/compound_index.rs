//! Compound indexing captures each operand once and resolves read/write separately.
use super::{
    BodyLowerer,
    expressions::{binary_operator_name, contextual_literal},
};
use crate::{
    ir::{ast, hir},
    semantic::{TypeId, TypeKind},
    source::Span,
};

impl BodyLowerer<'_> {
    fn index_temporary(
        &mut self,
        value: hir::Expr,
        statements: &mut Vec<hir::Stmt>,
    ) -> Option<(hir::Expr, ast::Expr)> {
        let name = format!("\u{1}index.{}", self.locals.len());
        let local = self.add_local(name.clone(), value.ty, hir::LocalKind::User, value.span)?;
        let expression = hir::Expr {
            kind: hir::ExprKind::Local(local),
            ty: value.ty,
            span: value.span,
        };
        let source = ast::Expr::Name(ast::Path {
            segments: vec![ast::Ident {
                name,
                span: value.span,
            }],
            span: value.span,
        });
        statements.push(hir::Stmt::LocalDecl {
            locals: vec![Some(local)],
            span: value.span,
            values: hir::ValueList::Expressions(vec![value]),
        });
        Some((expression, source))
    }

    fn captured_index_receiver(
        &mut self,
        receiver: &hir::Expr,
        base: &hir::Expr,
        pointer: Option<&hir::Expr>,
        statements: &mut Vec<hir::Stmt>,
    ) -> Option<hir::Expr> {
        let value = if receiver.ty == base.ty {
            base.clone()
        } else {
            match receiver.kind {
                hir::ExprKind::AddressOf(_) => pointer?.clone(),
                hir::ExprKind::Dereference(_) => hir::Expr {
                    kind: hir::ExprKind::Dereference(Box::new(base.clone())),
                    ty: receiver.ty,
                    span: receiver.span,
                },
                _ => return None,
            }
        };
        Some(self.index_temporary(value, statements)?.0)
    }

    pub(super) fn lower_compound_index(
        &mut self,
        source_base: &ast::Expr,
        source_index: &ast::Expr,
        rhs: Option<&ast::Expr>,
        op: ast::BinaryOp,
        span: Span,
    ) -> Option<Vec<hir::Stmt>> {
        let implicit_one = rhs.is_none();
        if self.read_only_target(source_base) {
            return self.fail(source_base.span(), "cannot assign to a constant");
        }
        let original_base = self.lower_expr(source_base, None)?;
        let addressable = self.addressable(&original_base);
        let mut statements = Vec::new();
        let (base_place, base_pointer) = if addressable {
            let ty = self.types.pointer(original_base.ty, span);
            let pointer = self
                .index_temporary(
                    hir::Expr {
                        ty,
                        span,
                        kind: hir::ExprKind::AddressOf(Box::new(original_base.clone())),
                    },
                    &mut statements,
                )?
                .0;
            (
                hir::Expr {
                    kind: hir::ExprKind::Dereference(Box::new(pointer.clone())),
                    ty: original_base.ty,
                    span,
                },
                Some(pointer),
            )
        } else {
            (original_base, None)
        };
        let read_parameters = self.unique_operator_parameter_types(
            ast::OperatorName::Index,
            &base_place,
            &[Some(base_place.ty), None],
            span,
        );
        let write_parameters = self.unique_operator_parameter_types(
            ast::OperatorName::IndexSet,
            &base_place,
            &[Some(base_place.ty), None, None],
            span,
        );
        let pointer_base = matches!(
            self.types.underlying_kind(base_place.ty),
            Some(TypeKind::Pointer(_))
        );
        let base_value = if pointer_base || read_parameters.is_some() || write_parameters.is_some()
        {
            self.index_temporary(base_place.clone(), &mut statements)?.0
        } else {
            base_place.clone()
        };
        let builtin_base = if pointer_base || !addressable {
            base_value.clone()
        } else {
            base_place.clone()
        };
        let read_index = read_parameters.as_ref().map(|parameters| parameters[1]);

        let contextual_index = contextual_literal(source_index);
        let index_value = if !contextual_index || read_index.is_some() {
            Some(self.lower_value(source_index, read_index.filter(|_| contextual_index))?)
        } else {
            None
        };
        let mut getter = if let Some(index) = &index_value {
            self.matching_operator_call(
                ast::OperatorName::Index,
                vec![base_place.clone(), index.clone()],
                span,
            )?
        } else {
            None
        };
        let element_ty = if let Some(getter) = &getter {
            *getter.signature.returns.first()?
        } else {
            match self.types.underlying_kind(base_place.ty) {
                Some(TypeKind::Array { element, .. } | TypeKind::Pointer(element)) => *element,
                _ => {
                    return self.fail(
                        span,
                        "indexing requires an array, pointer, or matching operator",
                    );
                }
            }
        };
        // Type the arithmetic using the same expression path as ordinary binary expressions.
        // The getter's result is bound below, after capturing both adjusted receivers.
        let old_name = format!("\u{1}index.old.{}", self.locals.len());
        let old_local = self.add_local(old_name.clone(), element_ty, hir::LocalKind::User, span)?;
        let source_place = getter.is_none() && (addressable || pointer_base);
        if !source_place {
            self.non_addressable_locals.insert(old_local);
        }
        let old_source = ast::Expr::Name(ast::Path {
            segments: vec![ast::Ident {
                name: old_name,
                span,
            }],
            span,
        });
        let rhs = if implicit_one {
            let old = hir::Expr {
                kind: hir::ExprKind::Local(old_local),
                ty: element_ty,
                span,
            };
            let rhs_ty = binary_operator_name(op)
                .and_then(|name| {
                    self.unique_operator_parameter_types(
                        name,
                        &old,
                        &[Some(element_ty), None],
                        span,
                    )
                })
                .map(|types| types[1])
                .unwrap_or(element_ty);
            let one = self.implicit_one(element_ty, rhs_ty, span)?;
            self.index_temporary(one, &mut statements)?.1
        } else {
            rhs?.clone()
        };
        let mut result = self.lower_value(
            &ast::Expr::Binary {
                op,
                lhs: Box::new(old_source),
                rhs: Box::new(rhs.clone()),
                span,
            },
            None,
        )?;
        // The write value is the arithmetic result, so only now can setter
        // applicability constrain a shared literal index.
        let applicable_write = self.unique_operator_parameter_types(
            ast::OperatorName::IndexSet,
            &base_place,
            &[Some(base_place.ty), None, Some(result.ty)],
            span,
        );
        let write_index = applicable_write.as_ref().map(|parameters| parameters[1]);
        if contextual_literal(source_index)
            && read_index
                .zip(write_index)
                .is_some_and(|(read, write)| read != write)
        {
            return self.fail(
                span,
                "index read and write require different literal type contexts",
            );
        }
        let index_value = match index_value {
            Some(index) => index,
            None => self.lower_value(
                source_index,
                write_index.or_else(|| pointer_base.then_some(TypeId::U32)),
            )?,
        };
        // Validate the final shared index on every builtin read, including when
        // the write selected a custom operator with its own index context.
        if getter.is_none() {
            self.builtin_index(builtin_base.clone(), index_value.clone(), span)?;
        }
        let placeholder = hir::Expr {
            kind: hir::ExprKind::Zero,
            ty: result.ty,
            span,
        };
        let mut setter = self.matching_operator_call(
            ast::OperatorName::IndexSet,
            vec![base_place.clone(), index_value.clone(), placeholder],
            span,
        )?;
        let builtin_write = if setter.is_none() {
            if !addressable
                && !matches!(
                    self.types.underlying_kind(base_place.ty),
                    Some(TypeKind::Pointer(_))
                )
            {
                return self.fail(span, "assignment requires an addressable index base");
            }
            let target = self.builtin_index(builtin_base.clone(), index_value.clone(), span)?;
            if target.ty != result.ty {
                return self.type_mismatch(rhs.span(), target.ty, result.ty);
            }
            Some(target)
        } else {
            None
        };
        if let Some(getter) = &mut getter {
            getter.arguments[0] = self.captured_index_receiver(
                &getter.arguments[0],
                &base_value,
                base_pointer.as_ref(),
                &mut statements,
            )?;
        }
        if let Some(setter) = &mut setter {
            setter.arguments[0] = if let Some(getter) = &getter
                && getter.arguments[0].ty == setter.arguments[0].ty
            {
                getter.arguments[0].clone()
            } else {
                self.captured_index_receiver(
                    &setter.arguments[0],
                    &base_value,
                    base_pointer.as_ref(),
                    &mut statements,
                )?
            };
        }
        let index = self.index_temporary(index_value, &mut statements)?.0;
        let read = if let Some(mut getter) = getter {
            getter.arguments[1] = index.clone();
            hir::Expr {
                kind: hir::ExprKind::Call(Box::new(getter)),
                ty: element_ty,
                span,
            }
        } else {
            hir::Expr {
                kind: hir::ExprKind::Index {
                    base: Box::new(builtin_base),
                    index: Box::new(index.clone()),
                },
                ty: element_ty,
                span,
            }
        };
        let references_operand = matches!(&result.kind, hir::ExprKind::Call(call)
            if matches!(call.arguments.first().map(|argument| &argument.kind), Some(hir::ExprKind::AddressOf(value))
                if matches!(value.kind, hir::ExprKind::Local(local) if local == old_local)));
        if source_place && references_operand {
            // Capture the source element's address (and its bounds check) before
            // evaluating the arithmetic RHS; the value snapshot is not its storage.
            let ty = self.types.pointer(element_ty, span);
            let pointer = self
                .index_temporary(
                    hir::Expr {
                        kind: hir::ExprKind::AddressOf(Box::new(read)),
                        ty,
                        span,
                    },
                    &mut statements,
                )?
                .0;
            let hir::ExprKind::Call(call) = &mut result.kind else {
                unreachable!()
            };
            call.arguments[0] = pointer;
        } else {
            statements.push(hir::Stmt::LocalDecl {
                locals: vec![Some(old_local)],
                values: hir::ValueList::Expressions(vec![read]),
                span,
            });
        }
        let result = self.index_temporary(result, &mut statements)?.0;
        if let Some(mut setter) = setter {
            setter.arguments[1] = index;
            setter.arguments[2] = result;
            statements.push(hir::Stmt::Call(setter));
        } else {
            let mut target = builtin_write?;
            let hir::ExprKind::Index {
                index: target_index,
                ..
            } = &mut target.kind
            else {
                return None;
            };
            **target_index = index;
            statements.push(hir::Stmt::Assign {
                targets: vec![Some(target)],
                values: hir::ValueList::Expressions(vec![result]),
                op: None,
                overloaded: None,
                span,
            });
        }
        Some(statements)
    }
}
