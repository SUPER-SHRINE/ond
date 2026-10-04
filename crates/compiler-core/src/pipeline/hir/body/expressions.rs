//! Expression typing and name resolution.
use super::BodyLowerer;
use crate::ir::{ast, hir};

impl BodyLowerer<'_> {
    pub(super) fn lower_expr(
        &mut self,
        expr: &ast::Expr,
        expected: Option<crate::semantic::TypeId>,
    ) -> Option<hir::Expr> {
        use crate::semantic::{TypeId, TypeKind};

        if matches!(self.builtin_name(expr), Some("len")) {
            let value = self.lower_len_builtin(expr)?;
            return self.constant_use(value, expected, expr.span());
        }
        if matches!(self.builtin_name(expr), Some("thisFile" | "thisLine")) {
            let value = self.lower_source_builtin(expr)?;
            return self.constant_use(value, expected, expr.span());
        }

        let (kind, ty) = match expr {
            ast::Expr::Literal(ast::Literal::Int(value, _)) => {
                let ty = expected.unwrap_or(TypeId::I32);
                if !self.types.is_integer(ty) {
                    return self.fail(expr.span(), "integer literal requires an integer type");
                }
                let Some(bits) = crate::numeric_literal::integer(value) else {
                    return self.fail(expr.span(), "invalid or out-of-range integer literal");
                };
                let (_, max) = crate::constant::integer_range(self.types, ty)?;
                if i128::from(bits) > max {
                    return self.fail(expr.span(), "integer literal out of range for its type");
                }
                (hir::ExprKind::Integer(bits), ty)
            }
            ast::Expr::Literal(ast::Literal::Float(value, _)) => {
                let ty = expected.unwrap_or(TypeId::F32);
                if !matches!(self.types.underlying_kind(ty), Some(TypeKind::F32)) {
                    return self.fail(expr.span(), "float literal requires f32 type");
                }
                let value = match crate::numeric_literal::float(value) {
                    Ok(value) => value,
                    Err(message) => return self.fail(expr.span(), message),
                };
                if !value.is_finite() {
                    return self.fail(expr.span(), "f32 literal out of range");
                }
                (hir::ExprKind::Float32(value.to_bits()), ty)
            }
            ast::Expr::Literal(ast::Literal::Bool(value, _)) => {
                let ty = expected.unwrap_or(TypeId::BOOL);
                if !matches!(self.types.underlying_kind(ty), Some(TypeKind::Bool)) {
                    return self.fail(expr.span(), "boolean literal requires bool type");
                }
                (hir::ExprKind::Bool(*value), ty)
            }
            ast::Expr::Literal(ast::Literal::Nil(_)) => {
                let Some(ty) = expected else {
                    return self.fail(
                        expr.span(),
                        "`nil` requires an explicit pointer or function type context",
                    );
                };
                if matches!(self.types.underlying_kind(ty), Some(TypeKind::Interface(_))) {
                    return Some(hir::Expr {
                        kind: hir::ExprKind::Zero,
                        ty,
                        span: expr.span(),
                    });
                }
                if !matches!(
                    self.types.underlying_kind(ty),
                    Some(TypeKind::Pointer(_) | TypeKind::Function(_))
                ) {
                    return self.fail(expr.span(), "nil requires a pointer or function type");
                }
                (hir::ExprKind::Nil, ty)
            }
            ast::Expr::Literal(ast::Literal::String(source, span)) => {
                let bytes = match crate::string_literal::decode(source) {
                    Ok(bytes) => bytes,
                    Err(message) => return self.fail(*span, message),
                };
                return self.lower_string_bytes(bytes, *span, expected);
            }
            ast::Expr::LayoutQuery {
                ty,
                alignment,
                span,
            } => {
                let ty = self.resolve_type(ty)?;
                let (size, align) = match self.types.size_align(ty) {
                    Ok(layout) => layout,
                    Err(message) => return self.fail(*span, message),
                };
                (
                    hir::ExprKind::Integer(u64::from(if *alignment { align } else { size })),
                    TypeId::U32,
                )
            }
            ast::Expr::Name(path) if path.segments.len() == 1 => {
                let name = &path.segments[0].name;
                if name == "_" {
                    return self.fail(path.span, "blank identifier `_` cannot be used as a value");
                }
                if let Some(value) = self.constants.get(name).cloned() {
                    let null_terminated = self.null_terminated_constants.contains(name);
                    return self.local_constant_use(value, expected, path.span, null_terminated);
                }
                let Some(local) = self.names.get(name).copied() else {
                    if self.poisoned_names.contains(name) {
                        return self.suppress_poisoned_failure();
                    }
                    return self.package_value(self.package_id, name, expected, path.span);
                };
                let ty = self.locals.get(local.0 as usize)?.ty;
                if let Some(expected) = expected
                    && expected != ty
                {
                    return self.type_mismatch(path.span, expected, ty);
                }
                (hir::ExprKind::Local(local), ty)
            }
            ast::Expr::Unary { op, expr, .. } => {
                if *op == ast::UnaryOp::Minus {
                    if let ast::Expr::Literal(ast::Literal::Int(text, _)) = expr.as_ref() {
                        let ty = expected.unwrap_or(TypeId::I32);
                        let Some((min, max)) = crate::constant::integer_range(self.types, ty)
                        else {
                            return self.fail(expr.span(), "integer literal requires integer type");
                        };
                        let Some(bits) = crate::numeric_literal::integer(text) else {
                            return self.fail(expr.span(), "integer literal out of range");
                        };
                        let value = -i128::from(bits);
                        if value < min || value > max {
                            return self
                                .fail(expr.span(), "integer literal out of range for its type");
                        }
                        return Some(hir::Expr {
                            kind: hir::ExprKind::Integer(value as u64),
                            ty,
                            span: expr.span(),
                        });
                    }
                }
                if matches!(op, ast::UnaryOp::AddrOf | ast::UnaryOp::Deref) {
                    if *op == ast::UnaryOp::AddrOf
                        && matches!(expr.as_ref(), ast::Expr::Composite { .. })
                    {
                        return self.fail(
                            expr.span(),
                            "`&T{...}` is not supported; composite literals are not addressable",
                        );
                    }
                    let value = self.lower_expr(expr, None)?;
                    let result = if *op == ast::UnaryOp::AddrOf {
                        if !self.addressable(&value) {
                            return self.fail(
                                expr.span(),
                                "address-of requires an addressable expression",
                            );
                        }
                        let ty = self.types.pointer(value.ty, expr.span());
                        hir::Expr {
                            kind: hir::ExprKind::AddressOf(Box::new(value)),
                            ty,
                            span: expr.span(),
                        }
                    } else {
                        let Some(TypeKind::Pointer(ty)) = self.types.underlying_kind(value.ty)
                        else {
                            return self.fail(expr.span(), "dereference requires a pointer");
                        };
                        hir::Expr {
                            ty: *ty,
                            span: expr.span(),
                            kind: hir::ExprKind::Dereference(Box::new(value)),
                        }
                    };
                    return self.constant_use(result, expected, expr.span());
                }
                let operand = self.lower_expr(expr, expected)?;
                let builtin = match op {
                    ast::UnaryOp::Plus | ast::UnaryOp::Minus | ast::UnaryOp::BitNot
                        if self.types.is_integer(operand.ty) =>
                    {
                        true
                    }
                    ast::UnaryOp::Plus | ast::UnaryOp::Minus
                        if matches!(
                            self.types.underlying_kind(operand.ty),
                            Some(TypeKind::F32)
                        ) =>
                    {
                        true
                    }
                    ast::UnaryOp::Not
                        if matches!(
                            self.types.underlying_kind(operand.ty),
                            Some(TypeKind::Bool)
                        ) =>
                    {
                        true
                    }
                    _ => false,
                };
                if !builtin && matches!(op, ast::UnaryOp::Plus | ast::UnaryOp::Minus) {
                    let name = if *op == ast::UnaryOp::Plus {
                        ast::OperatorName::Add
                    } else {
                        ast::OperatorName::Sub
                    };
                    let Some(call) =
                        self.matching_operator_call(name, vec![operand], expr.span())?
                    else {
                        return self.fail(expr.span(), "invalid unary operand type");
                    };
                    let [ty] = call.signature.returns.as_slice() else {
                        return self.fail(expr.span(), "unary operator must return one value");
                    };
                    let ty = *ty;
                    if expected.is_some_and(|expected| expected != ty) {
                        return self.fail(expr.span(), "expression type mismatch");
                    }
                    return Some(hir::Expr {
                        kind: hir::ExprKind::Call(Box::new(call)),
                        ty,
                        span: expr.span(),
                    });
                }
                if !builtin {
                    return self.fail(expr.span(), "invalid unary operand type");
                }
                let ty = operand.ty;
                (
                    hir::ExprKind::Unary {
                        op: *op,
                        operand: Box::new(operand),
                    },
                    ty,
                )
            }
            ast::Expr::Binary { op, lhs, rhs, .. } => {
                if matches!(op, ast::BinaryOp::AndAnd | ast::BinaryOp::OrOr) {
                    let lhs = self.lower_expr(lhs, expected)?;
                    if !matches!(self.types.underlying_kind(lhs.ty), Some(TypeKind::Bool)) {
                        return self.fail(expr.span(), "logical operands must have type bool");
                    }
                    // Keep the RHS for constant eligibility and static checks,
                    // but do not evaluate values on a provably skipped branch.
                    let saved = self.defer_value_evaluation;
                    self.defer_value_evaluation =
                        saved || self.short_circuit_result(*op, &lhs).is_some();
                    let rhs = self.lower_expr(rhs, Some(lhs.ty));
                    self.defer_value_evaluation = saved;
                    let rhs = rhs?;
                    return Some(hir::Expr {
                        ty: lhs.ty,
                        span: expr.span(),
                        kind: hir::ExprKind::Binary {
                            op: *op,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        },
                    });
                }
                let comparison = matches!(
                    op,
                    ast::BinaryOp::Eq
                        | ast::BinaryOp::Ne
                        | ast::BinaryOp::Lt
                        | ast::BinaryOp::Le
                        | ast::BinaryOp::Gt
                        | ast::BinaryOp::Ge
                );
                let (lhs, rhs) = if matches!(op, ast::BinaryOp::Eq | ast::BinaryOp::Ne)
                    && matches!(lhs.as_ref(), ast::Expr::Literal(ast::Literal::Nil(_)))
                    && !matches!(rhs.as_ref(), ast::Expr::Literal(ast::Literal::Nil(_)))
                {
                    // A nil literal has no default type. Borrow the typed operand's
                    // context in either order; keep HIR operands in source order.
                    let rhs = self.lower_expr(rhs, None)?;
                    (self.lower_expr(lhs, Some(rhs.ty))?, rhs)
                } else {
                    let lhs = self.lower_expr(lhs, (!comparison).then_some(expected).flatten())?;
                    let overloadable = matches!(
                        op,
                        ast::BinaryOp::Add
                            | ast::BinaryOp::Sub
                            | ast::BinaryOp::Mul
                            | ast::BinaryOp::Div
                            | ast::BinaryOp::Rem
                    );
                    let rhs = if matches!(op, ast::BinaryOp::ShiftLeft | ast::BinaryOp::ShiftRight)
                    {
                        self.lower_expr(rhs, Some(TypeId::U32))?
                    } else if overloadable
                        && (self.types.is_integer(lhs.ty)
                            || matches!(self.types.underlying_kind(lhs.ty), Some(TypeKind::F32)))
                    {
                        self.lower_expr(rhs, Some(lhs.ty))?
                    } else if overloadable {
                        let inferred = binary_operator_name(*op)
                            .and_then(|name| {
                                self.unique_operator_parameter_types(name, &[Some(lhs.ty), None])
                            })
                            .map(|parameters| parameters[1]);
                        let candidate = self.lower_expr(rhs, inferred)?;
                        if candidate.ty != lhs.ty
                            && matches!(
                                rhs.as_ref(),
                                ast::Expr::Literal(
                                    ast::Literal::Int(_, _) | ast::Literal::Float(_, _)
                                )
                            )
                        {
                            self.lower_expr(rhs, Some(lhs.ty))?
                        } else {
                            candidate
                        }
                    } else {
                        self.lower_expr(rhs, Some(lhs.ty))?
                    };
                    (lhs, rhs)
                };
                if matches!(op, ast::BinaryOp::Eq | ast::BinaryOp::Ne)
                    && matches!(
                        self.types.underlying_kind(lhs.ty),
                        Some(TypeKind::Interface(_))
                    )
                {
                    if lhs.ty != rhs.ty {
                        return self
                            .fail(expr.span(), "interface comparison requires identical types");
                    }
                    return Some(hir::Expr {
                        kind: hir::ExprKind::InterfaceEqual {
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                            negate: *op == ast::BinaryOp::Ne,
                        },
                        ty: TypeId::BOOL,
                        span: expr.span(),
                    });
                }
                let operand_kind = self.types.underlying_kind(lhs.ty);
                let valid = if self.types.is_integer(lhs.ty) {
                    true
                } else {
                    match operand_kind {
                        Some(TypeKind::F32) => matches!(
                            op,
                            ast::BinaryOp::Add
                                | ast::BinaryOp::Sub
                                | ast::BinaryOp::Mul
                                | ast::BinaryOp::Div
                                | ast::BinaryOp::Eq
                                | ast::BinaryOp::Ne
                                | ast::BinaryOp::Lt
                                | ast::BinaryOp::Le
                                | ast::BinaryOp::Gt
                                | ast::BinaryOp::Ge
                        ),
                        Some(TypeKind::Bool | TypeKind::Pointer(_) | TypeKind::Function(_)) => {
                            matches!(op, ast::BinaryOp::Eq | ast::BinaryOp::Ne)
                        }
                        _ => false,
                    }
                };
                if !valid
                    || (!matches!(op, ast::BinaryOp::ShiftLeft | ast::BinaryOp::ShiftRight)
                        && lhs.ty != rhs.ty)
                {
                    if let Some(name) = binary_operator_name(*op)
                        && let Some(call) =
                            self.matching_operator_call(name, vec![lhs, rhs], expr.span())?
                    {
                        let [ty] = call.signature.returns.as_slice() else {
                            return self.fail(expr.span(), "binary operator must return one value");
                        };
                        let ty = *ty;
                        if expected.is_some_and(|expected| expected != ty) {
                            return self.fail(expr.span(), "expression type mismatch");
                        }
                        return Some(hir::Expr {
                            kind: hir::ExprKind::Call(Box::new(call)),
                            ty,
                            span: expr.span(),
                        });
                    }
                    return self.fail(expr.span(), "invalid binary operand type");
                }
                let ty = if comparison { TypeId::BOOL } else { lhs.ty };
                if expected.is_some_and(|expected| expected != ty) {
                    return self.fail(expr.span(), "expression type mismatch");
                }
                (
                    hir::ExprKind::Binary {
                        op: *op,
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    },
                    ty,
                )
            }
            ast::Expr::Cast {
                expr,
                ty,
                span: cast_span,
            } => {
                let ty = self.resolve_type(ty)?;
                if matches!(self.types.underlying_kind(ty), Some(TypeKind::Interface(_))) {
                    let value = self.lower_interface_conversion(expr, ty, *cast_span)?;
                    if let Some(expected) = expected
                        && expected != ty
                    {
                        return self.type_mismatch(expr.span(), expected, ty);
                    }
                    return Some(value);
                }
                let literal_context = (self.types.is_integer(ty)
                    && matches!(expr.as_ref(), ast::Expr::Literal(ast::Literal::Int(_, _))))
                .then_some(ty);
                let value = self.lower_expr(expr, literal_context)?;
                let source = self.types.underlying_kind(value.ty);
                let destination = self.types.underlying_kind(ty);
                let numeric_source =
                    self.types.is_integer(value.ty) || matches!(source, Some(TypeKind::F32));
                let numeric_destination =
                    self.types.is_integer(ty) || matches!(destination, Some(TypeKind::F32));
                let valid = self.types.underlying_id(value.ty) == self.types.underlying_id(ty)
                    || (numeric_source && numeric_destination)
                    || (self.types.is_integer(value.ty)
                        && matches!(destination, Some(TypeKind::Pointer(_))))
                    || (matches!(source, Some(TypeKind::Pointer(_))) && self.types.is_integer(ty));
                if !valid {
                    return self.fail(expr.span(), "invalid explicit type conversion");
                }
                if let Some(expected) = expected
                    && expected != ty
                {
                    return self.type_mismatch(expr.span(), expected, ty);
                }
                (
                    hir::ExprKind::Cast {
                        value: Box::new(value),
                    },
                    ty,
                )
            }
            ast::Expr::Selector { base, field, span } => {
                if let ast::Expr::Name(path) = base.as_ref() {
                    if path.segments.len() == 1
                        && !self.names.contains_key(&path.segments[0].name)
                        && !self.constants.contains_key(&path.segments[0].name)
                        && !self
                            .universe
                            .iter()
                            .find(|package| package.id == self.package_id)
                            .is_some_and(|package| {
                                package.values.contains(&path.segments[0].name)
                                    || package.constants.contains_key(&path.segments[0].name)
                            })
                    {
                        let qualifier = &path.segments[0].name;
                        let imported = self.imported_package(qualifier, path.span);
                        if let Some(package) = imported {
                            if !field
                                .name
                                .as_bytes()
                                .first()
                                .is_some_and(u8::is_ascii_uppercase)
                            {
                                let kind = if self
                                    .universe
                                    .iter()
                                    .find(|candidate| candidate.id == package)
                                    .is_some_and(|package| {
                                        package.functions.contains_key(&field.name)
                                    }) {
                                    "function"
                                } else {
                                    "symbol"
                                };
                                return self.fail(
                                    *span,
                                    format!(
                                        "{kind} `{}.{}` is not exported",
                                        qualifier, field.name
                                    ),
                                );
                            }
                            return self.package_value(package, &field.name, expected, *span);
                        }
                    }
                }
                let value = self.lower_field(base, &field.name, *span)?;
                return self.constant_use(value, expected, *span);
            }
            ast::Expr::Name(path) => {
                if path.segments.len() >= 2 {
                    return self.lower_expr(
                        &ast::Expr::Selector {
                            base: Box::new(ast::Expr::Name(ast::Path {
                                segments: path.segments[..path.segments.len() - 1].to_vec(),
                                span: path.span,
                            })),
                            field: path.segments.last()?.clone(),
                            span: path.span,
                        },
                        expected,
                    );
                }
                return self.fail(path.span, "empty name path");
            }
            ast::Expr::Call { .. } => {
                let call = self.lower_call(expr)?;
                if call.signature.returns.len() != 1 {
                    return self.fail(
                        expr.span(),
                        format!(
                            "single-value expression requires 1 return value, found {}",
                            call.signature.returns.len()
                        ),
                    );
                }
                let ty = call.signature.returns[0];
                if expected.is_some_and(|expected| expected != ty) {
                    return self.fail(expr.span(), "call result type mismatch");
                }
                (hir::ExprKind::Call(Box::new(call)), ty)
            }
            ast::Expr::TypeApply { .. } => {
                return self.fail(expr.span(), "type arguments must be followed by a call");
            }
            ast::Expr::Index { base, index, span } => {
                let value = self.lower_index(base, index, *span)?;
                return self.constant_use(value, expected, *span);
            }
            ast::Expr::Composite { ty, elements, span } => {
                let value = self.lower_composite(ty, elements, *span)?;
                return self.constant_use(value, expected, *span);
            }
            ast::Expr::New { ty, span } => {
                let value = self.lower_new(ty, *span)?;
                return self.constant_use(value, expected, *span);
            }
        };
        let result = hir::Expr {
            kind,
            ty,
            span: expr.span(),
        };
        if matches!(result.kind, hir::ExprKind::Cast { .. })
            && !self.defer_value_evaluation
            && crate::constant::is_constant(&result, self.types)
        {
            return match crate::constant::evaluate(&result, self.types) {
                Ok(value) => Some(value),
                Err(message) => self.fail(result.span, message),
            };
        }
        Some(result)
    }
}

pub(super) fn binary_operator_name(op: ast::BinaryOp) -> Option<ast::OperatorName> {
    Some(match op {
        ast::BinaryOp::Add => ast::OperatorName::Add,
        ast::BinaryOp::Sub => ast::OperatorName::Sub,
        ast::BinaryOp::Mul => ast::OperatorName::Mul,
        ast::BinaryOp::Div => ast::OperatorName::Div,
        ast::BinaryOp::Rem => ast::OperatorName::Rem,
        _ => return None,
    })
}
