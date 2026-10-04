//! Typed HIR to MIR CFG construction.
mod memory;
use crate::{
    FrontendError,
    ir::{ast, hir},
    mir,
    source::Span,
};

pub(crate) fn lower_to_mir(hir: &hir::Project) -> Result<mir::Project, FrontendError> {
    let mut packages = Vec::new();
    for package in &hir.packages {
        let mut functions = Vec::new();
        for item in &package.items {
            if let hir::Item::Func(function) = item {
                functions.push(lower_function(&hir.types, function)?);
            }
        }
        packages.push(mir::Package {
            id: package.id,
            imports: package.imports.clone(),
            logical_path: package.logical_path.clone(),
            functions,
            globals: package
                .items
                .iter()
                .filter_map(|item| match item {
                    hir::Item::Var(item) => Some(&item.globals),
                    _ => None,
                })
                .flatten()
                .flatten()
                .map(|global| {
                    Ok(mir::Global {
                        symbol: global.symbol.clone(),
                        ty: global.ty,
                        initializer: global
                            .initializer
                            .as_ref()
                            .map(|value| lower_static_value(&hir.types, value))
                            .transpose()?,
                        span: global.span,
                    })
                })
                .collect::<Result<Vec<_>, FrontendError>>()?,
            statics: package
                .statics
                .iter()
                .map(|item| {
                    Ok(mir::Static {
                        symbol: item.symbol.clone(),
                        value: lower_static_value(&hir.types, &item.value)?,
                        exported: item.exported,
                        null_terminated: item.null_terminated,
                        span: item.span,
                    })
                })
                .collect::<Result<Vec<_>, FrontendError>>()?,
            initializer: package
                .initializer
                .as_ref()
                .map(|function| lower_function(&hir.types, function))
                .transpose()?,
        });
    }
    Ok(mir::Project {
        types: hir.types.clone(),
        packages,
        initialization_order: hir.initialization_order.clone(),
    })
}

fn lower_static_value(
    types: &crate::semantic::TypeTable,
    expr: &hir::Expr,
) -> Result<mir::StaticValue, FrontendError> {
    let kind = match &expr.kind {
        hir::ExprKind::Zero => mir::StaticValueKind::Zero,
        hir::ExprKind::Integer(value) => mir::StaticValueKind::Integer(*value),
        hir::ExprKind::Float32(value) => mir::StaticValueKind::Float32(*value),
        hir::ExprKind::Bool(value) => mir::StaticValueKind::Bool(*value),
        hir::ExprKind::Nil => mir::StaticValueKind::Nil,
        hir::ExprKind::Function(symbol) => {
            let Some(crate::semantic::TypeKind::Function(signature)) =
                types.underlying_kind(expr.ty)
            else {
                return Err(FrontendError::new(vec![crate::Diagnostic::error(
                    expr.span,
                    "static function value has non-function type",
                )]));
            };
            mir::StaticValueKind::Function {
                symbol: symbol.clone(),
                signature: signature.clone(),
            }
        }
        hir::ExprKind::Composite(values) => mir::StaticValueKind::Composite(
            values
                .iter()
                .map(|(index, value)| Ok((*index, lower_static_value(types, value)?)))
                .collect::<Result<Vec<_>, FrontendError>>()?,
        ),
        _ => {
            return Err(FrontendError::new(vec![crate::Diagnostic::error(
                expr.span,
                "static initializer was not fully evaluated",
            )]));
        }
    };
    Ok(mir::StaticValue { ty: expr.ty, kind })
}

fn lower_function(
    types: &crate::semantic::TypeTable,
    function: &hir::FuncItem,
) -> Result<mir::Function, FrontendError> {
    let (locals, parameters) = match &function.body {
        hir::Body::Typed(body) => {
            let locals = body
                .locals
                .iter()
                .map(|local| mir::Local {
                    id: mir::LocalId(local.id.0),
                    name: Some(local.name.clone()),
                    ty: local.ty,
                    kind: match local.kind {
                        hir::LocalKind::Parameter => mir::LocalKind::Parameter,
                        hir::LocalKind::User => mir::LocalKind::User,
                    },
                    address_taken: false,
                    span: local.span,
                })
                .collect::<Vec<_>>();
            let parameters = body
                .locals
                .iter()
                .filter(|local| local.kind == hir::LocalKind::Parameter)
                .map(|local| mir::LocalId(local.id.0))
                .collect();
            (locals, parameters)
        }
    };

    let hir::Body::Typed(body) = &function.body;
    let mut defers = Vec::new();
    collect_defers(&body.block, &mut defers);
    defers.sort_by_key(|(id, _, _)| *id);

    let mut builder = FunctionBuilder {
        types,
        locals,
        values: Vec::new(),
        blocks: Vec::new(),
        current: None,
        next_instruction: 0,
        loops: Vec::new(),
        defer_flags: Vec::new(),
        return_block: None,
        return_values: Vec::new(),
    };
    let entry = builder.new_block(function.span);
    builder.current = Some(entry);
    if !defers.is_empty() {
        for (_, _, span) in &defers {
            let id = mir::LocalId(builder.locals.len() as u32);
            builder.locals.push(mir::Local {
                id,
                name: None,
                ty: crate::semantic::TypeId::BOOL,
                kind: mir::LocalKind::Temporary,
                address_taken: false,
                span: *span,
            });
            builder.defer_flags.push(id);
            let inactive = builder.emit_value(
                crate::semantic::TypeId::BOOL,
                mir::InstructionKind::Constant(mir::Constant::Bool {
                    value: false,
                    ty: crate::semantic::TypeId::BOOL,
                }),
                *span,
            );
            builder.store_value(
                mir::Place {
                    base: mir::PlaceBase::Local(id),
                    projections: Vec::new(),
                    ty: crate::semantic::TypeId::BOOL,
                },
                inactive,
                *span,
            );
        }
        let return_block = builder.new_block(function.span);
        builder.return_block = Some(return_block);
        for ty in &function.signature.returns {
            let value = mir::ValueId(builder.values.len() as u32);
            builder.values.push(mir::Value {
                id: value,
                ty: *ty,
                span: function.span,
            });
            builder.blocks[return_block.0 as usize]
                .parameters
                .push(value);
            builder.return_values.push(value);
        }
    }
    {
        if builder.lower_block(&body.block).is_none() {
            return Err(FrontendError::new(vec![crate::Diagnostic::error(
                function.span,
                "internal MIR lowering failure for a typed HIR body",
            )]));
        }
        if function.signature.returns.is_empty() {
            builder.route_return(Vec::new(), function.span);
        }
    }
    if let Some(return_block) = builder.return_block {
        builder.current = Some(return_block);
        for (id, block, span) in defers.into_iter().rev() {
            builder.lower_defer(id, block, span).ok_or_else(|| {
                FrontendError::new(vec![crate::Diagnostic::error(
                    span,
                    "internal MIR lowering failure for a defer block",
                )])
            })?;
        }
        builder.terminate(
            mir::TerminatorKind::Return(builder.return_values.clone()),
            function.span,
        );
    }
    Ok(mir::Function {
        name: function.canonical_name.clone(),
        parameters,
        returns: function.signature.returns.clone(),
        locals: builder.locals,
        values: builder.values,
        span: function.span,
        entry: mir::BlockId(0),
        blocks: builder.blocks,
    })
}

struct FunctionBuilder<'a> {
    types: &'a crate::semantic::TypeTable,
    locals: Vec<mir::Local>,
    values: Vec<mir::Value>,
    blocks: Vec<mir::BasicBlock>,
    current: Option<mir::BlockId>,
    next_instruction: u32,
    loops: Vec<(mir::BlockId, mir::BlockId)>,
    defer_flags: Vec<mir::LocalId>,
    return_block: Option<mir::BlockId>,
    return_values: Vec<mir::ValueId>,
}

impl FunctionBuilder<'_> {
    fn lower_block(&mut self, block: &hir::Block) -> Option<()> {
        for statement in &block.statements {
            if self.current.is_none() {
                break;
            }
            match statement {
                hir::Stmt::LocalDecl {
                    locals,
                    values,
                    span,
                } => {
                    let evaluated = self.lower_values(values)?;
                    if locals.len() != evaluated.len() {
                        return None;
                    }
                    for (local, value) in locals.iter().zip(evaluated) {
                        let Some(local) = local else {
                            continue;
                        };
                        let ty = self.locals[local.0 as usize].ty;
                        let place = mir::Place {
                            base: mir::PlaceBase::Local(mir::LocalId(local.0)),
                            projections: Vec::new(),
                            ty,
                        };
                        self.store_value(place, value, *span);
                    }
                }
                hir::Stmt::Assign {
                    targets,
                    values,
                    op,
                    overloaded,
                    span,
                } => {
                    let targets = targets
                        .iter()
                        .map(|target| match target {
                            Some(target) => self.lower_place(target).map(Some),
                            None => Some(None),
                        })
                        .collect::<Option<Vec<_>>>()?;
                    if let Some(op) = op {
                        let [Some(place)] = targets.as_slice() else {
                            return None;
                        };
                        let place = place.clone();
                        let lhs = self.load_place_value(place.clone(), *span);
                        let evaluated = self.lower_values(values)?;
                        let [rhs] = evaluated.as_slice() else {
                            return None;
                        };
                        if let Some(call) = overloaded {
                            let hir::ExprKind::Function(symbol) = &call.callee.kind else {
                                return None;
                            };
                            let [result_ty] = call.signature.returns.as_slice() else {
                                return None;
                            };
                            if *result_ty != place.ty {
                                return None;
                            }
                            let result = mir::ValueId(self.values.len() as u32);
                            self.values.push(mir::Value {
                                id: result,
                                ty: *result_ty,
                                span: *span,
                            });
                            self.emit(
                                vec![result],
                                mir::InstructionKind::Call {
                                    callee: mir::Callee::Direct(symbol.clone()),
                                    signature: call.signature.clone(),
                                    arguments: vec![lhs, *rhs],
                                    effects: mir::CallEffects::Unknown,
                                },
                                *span,
                            );
                            self.store_value(place, result, *span);
                            continue;
                        }
                        if matches!(op, ast::BinaryOp::Div | ast::BinaryOp::Rem)
                            && self.types.is_integer(place.ty)
                        {
                            self.emit_effect(
                                mir::InstructionKind::Check(mir::Check::NonZero { value: *rhs }),
                                *span,
                            );
                            if *op == ast::BinaryOp::Div && self.types.is_signed_integer(place.ty) {
                                self.emit_effect(
                                    mir::InstructionKind::Check(
                                        mir::Check::SignedDivisionOverflow { lhs, rhs: *rhs },
                                    ),
                                    *span,
                                );
                            }
                        }
                        let result = self.emit_value(
                            place.ty,
                            mir::InstructionKind::Binary {
                                op: select_binary_op(self.types, *op, place.ty)?,
                                lhs,
                                rhs: *rhs,
                                overflow: if matches!(op, ast::BinaryOp::Div | ast::BinaryOp::Rem)
                                    && self.types.is_integer(place.ty)
                                {
                                    mir::OverflowBehavior::Checked
                                } else {
                                    mir::OverflowBehavior::Wrap
                                },
                            },
                            *span,
                        );
                        self.store_value(place, result, *span);
                        continue;
                    }
                    let evaluated = self.lower_values(values)?;
                    if targets.len() != evaluated.len() {
                        return None;
                    }
                    for (place, value) in targets.into_iter().zip(evaluated) {
                        if let Some(place) = place {
                            self.store_value(place, value, *span);
                        }
                    }
                }
                hir::Stmt::IndexAssign(statement) => {
                    let hir::IndexAssignStmt {
                        base,
                        index,
                        getter,
                        values,
                        op,
                        overloaded,
                        setter,
                        span,
                    } = statement.as_ref();
                    let base = self.lower_expr(base)?;
                    let index = self.lower_expr(index)?;
                    let got = self.emit_pre_evaluated_call(getter, vec![base, index], *span)?;
                    let [lhs] = got.as_slice() else { return None };
                    let rhs = self.lower_values(values)?;
                    let [rhs] = rhs.as_slice() else { return None };
                    let result = if let Some(call) = overloaded {
                        let result = self.emit_pre_evaluated_call(call, vec![*lhs, *rhs], *span)?;
                        let [result] = result.as_slice() else {
                            return None;
                        };
                        *result
                    } else {
                        let ty = self.values[lhs.0 as usize].ty;
                        if matches!(op, ast::BinaryOp::Div | ast::BinaryOp::Rem)
                            && self.types.is_integer(ty)
                        {
                            self.emit_effect(
                                mir::InstructionKind::Check(mir::Check::NonZero { value: *rhs }),
                                *span,
                            );
                        }
                        self.emit_value(
                            ty,
                            mir::InstructionKind::Binary {
                                op: select_binary_op(self.types, *op, ty)?,
                                lhs: *lhs,
                                rhs: *rhs,
                                overflow: mir::OverflowBehavior::Wrap,
                            },
                            *span,
                        )
                    };
                    self.emit_pre_evaluated_call(setter, vec![base, index, result], *span)?;
                }
                hir::Stmt::Expr(expr) => {
                    self.lower_expr(expr)?;
                }
                hir::Stmt::Call(call) => {
                    self.lower_call(call)?;
                }
                hir::Stmt::Trap { message, span } => {
                    self.terminate(
                        mir::TerminatorKind::Trap(mir::TrapKind::Explicit(message.clone())),
                        *span,
                    );
                }
                hir::Stmt::Return { values, span } => {
                    let values = self.lower_values(values)?;
                    self.route_return(values, *span);
                }
                hir::Stmt::Defer { id, span, .. } => {
                    let active = self.emit_value(
                        crate::semantic::TypeId::BOOL,
                        mir::InstructionKind::Constant(mir::Constant::Bool {
                            value: true,
                            ty: crate::semantic::TypeId::BOOL,
                        }),
                        *span,
                    );
                    let local = *self.defer_flags.get(*id)?;
                    self.store_value(
                        mir::Place {
                            base: mir::PlaceBase::Local(local),
                            projections: Vec::new(),
                            ty: crate::semantic::TypeId::BOOL,
                        },
                        active,
                        *span,
                    );
                }
                hir::Stmt::Block(block) => {
                    self.lower_block(block)?;
                }
                hir::Stmt::If {
                    condition,
                    then_block,
                    else_block,
                    span,
                } => {
                    let value = self.lower_expr(condition)?;
                    let then_id = self.new_block(then_block.span);
                    let else_id = self.new_block(else_block.span);
                    let merge = self.new_block(*span);
                    self.terminate(
                        mir::TerminatorKind::Branch {
                            condition: value,
                            then_edge: edge(then_id),
                            else_edge: edge(else_id),
                        },
                        condition.span,
                    );
                    self.current = Some(then_id);
                    self.lower_block(then_block)?;
                    let then_falls = self.current.is_some();
                    self.jump(merge, *span);
                    self.current = Some(else_id);
                    self.lower_block(else_block)?;
                    let else_falls = self.current.is_some();
                    self.jump(merge, *span);
                    self.current = (then_falls || else_falls).then_some(merge);
                }
                hir::Stmt::For {
                    condition,
                    body,
                    post,
                    span,
                } => {
                    let header = self.new_block(*span);
                    let body_id = self.new_block(body.span);
                    let post_id = self.new_block(post.span);
                    let exit = self.new_block(*span);
                    self.jump(header, *span);
                    self.current = Some(header);
                    if let Some(condition) = condition {
                        let value = self.lower_expr(condition)?;
                        self.terminate(
                            mir::TerminatorKind::Branch {
                                condition: value,
                                then_edge: edge(body_id),
                                else_edge: edge(exit),
                            },
                            condition.span,
                        );
                    } else {
                        self.jump(body_id, *span);
                    }
                    self.loops.push((exit, post_id));
                    self.current = Some(body_id);
                    self.lower_block(body)?;
                    self.jump(post_id, *span);
                    self.loops.pop();
                    self.current = Some(post_id);
                    self.lower_block(post)?;
                    self.jump(header, *span);
                    self.current = Some(exit);
                }
                hir::Stmt::Break(span) => {
                    self.jump(self.loops.last()?.0, *span);
                }
                hir::Stmt::Continue(span) => {
                    self.jump(self.loops.last()?.1, *span);
                }
            }
        }
        Some(())
    }

    fn lower_expr(&mut self, expr: &hir::Expr) -> Option<mir::ValueId> {
        let kind = match &expr.kind {
            hir::ExprKind::Intrinsic(_) => return None, // only legal as a call target
            hir::ExprKind::InterfaceMethod { .. } => return None, // only legal as a call target
            hir::ExprKind::Global(_)
            | hir::ExprKind::Dereference(_)
            | hir::ExprKind::AddressOf(_)
            | hir::ExprKind::Field { .. }
            | hir::ExprKind::Index { .. }
            | hir::ExprKind::Composite(_) => return self.lower_memory_expr(expr),
            hir::ExprKind::Call(call) => {
                return self.lower_call(call)?.into_iter().next();
            }
            hir::ExprKind::InterfaceEqual { lhs, rhs, negate } => {
                return self.lower_interface_equal(lhs, rhs, *negate, expr.span);
            }
            hir::ExprKind::EvaluatedLength { value, length } => {
                self.lower_expr(value)?;
                mir::InstructionKind::Constant(mir::Constant::Integer {
                    bits: u64::from(*length),
                    ty: expr.ty,
                })
            }
            hir::ExprKind::Function(symbol) => {
                let mir::TypeKind::Function(signature) = self.types.underlying_kind(expr.ty)?
                else {
                    return None;
                };
                mir::InstructionKind::Constant(mir::Constant::Function {
                    symbol: symbol.clone(),
                    signature: signature.clone(),
                    ty: expr.ty,
                })
            }
            hir::ExprKind::Zero => match self.types.underlying_kind(expr.ty)? {
                crate::semantic::TypeKind::Bool => {
                    mir::InstructionKind::Constant(mir::Constant::Bool {
                        value: false,
                        ty: expr.ty,
                    })
                }
                crate::semantic::TypeKind::F32 => {
                    mir::InstructionKind::Constant(mir::Constant::Float32 {
                        bits: 0,
                        ty: expr.ty,
                    })
                }
                crate::semantic::TypeKind::Pointer(_) | crate::semantic::TypeKind::Function(_) => {
                    mir::InstructionKind::Constant(mir::Constant::Nil(expr.ty))
                }
                kind if self.types.is_integer(expr.ty) => {
                    let _ = kind;
                    mir::InstructionKind::Constant(mir::Constant::Integer {
                        bits: 0,
                        ty: expr.ty,
                    })
                }
                _ => mir::InstructionKind::Constant(mir::Constant::Zero(expr.ty)),
            },
            hir::ExprKind::Integer(bits) => {
                mir::InstructionKind::Constant(mir::Constant::Integer {
                    bits: *bits,
                    ty: expr.ty,
                })
            }
            hir::ExprKind::Float32(bits) => {
                mir::InstructionKind::Constant(mir::Constant::Float32 {
                    bits: *bits,
                    ty: expr.ty,
                })
            }
            hir::ExprKind::Bool(value) => mir::InstructionKind::Constant(mir::Constant::Bool {
                value: *value,
                ty: expr.ty,
            }),
            hir::ExprKind::Nil => mir::InstructionKind::Constant(mir::Constant::Nil(expr.ty)),
            hir::ExprKind::Local(local) => mir::InstructionKind::Load {
                place: mir::Place {
                    base: mir::PlaceBase::Local(mir::LocalId(local.0)),
                    projections: Vec::new(),
                    ty: expr.ty,
                },
                access: mir::AccessKind::Local,
            },
            hir::ExprKind::Unary { op, operand } => {
                let operand = self.lower_expr(operand)?;
                let op = match op {
                    ast::UnaryOp::Minus => mir::UnaryOp::Negate,
                    ast::UnaryOp::BitNot => mir::UnaryOp::BitNot,
                    ast::UnaryOp::Not => mir::UnaryOp::LogicalNot,
                    ast::UnaryOp::Plus => return Some(operand),
                    _ => return None,
                };
                mir::InstructionKind::Unary { op, operand }
            }
            hir::ExprKind::Binary { op, lhs, rhs } => {
                if matches!(op, ast::BinaryOp::AndAnd | ast::BinaryOp::OrOr) {
                    let left = self.lower_expr(lhs)?;
                    let rhs_id = self.new_block(rhs.span);
                    let merge = self.new_block(expr.span);
                    let result = mir::ValueId(self.values.len() as u32);
                    self.values.push(mir::Value {
                        id: result,
                        ty: expr.ty,
                        span: expr.span,
                    });
                    self.blocks[merge.0 as usize].parameters.push(result);
                    let short_edge = mir::Edge {
                        target: merge,
                        arguments: vec![left],
                    };
                    let (then_edge, else_edge) = if *op == ast::BinaryOp::AndAnd {
                        (edge(rhs_id), short_edge)
                    } else {
                        (short_edge, edge(rhs_id))
                    };
                    self.terminate(
                        mir::TerminatorKind::Branch {
                            condition: left,
                            then_edge,
                            else_edge,
                        },
                        lhs.span,
                    );
                    self.current = Some(rhs_id);
                    let right = self.lower_expr(rhs)?;
                    self.terminate(
                        mir::TerminatorKind::Jump(mir::Edge {
                            target: merge,
                            arguments: vec![right],
                        }),
                        rhs.span,
                    );
                    self.current = Some(merge);
                    return Some(result);
                }
                let lhs = self.lower_expr(lhs)?;
                let rhs = self.lower_expr(rhs)?;
                if matches!(op, ast::BinaryOp::Div | ast::BinaryOp::Rem)
                    && self.types.is_integer(lhs_type(self, lhs)?)
                {
                    self.emit_effect(
                        mir::InstructionKind::Check(mir::Check::NonZero { value: rhs }),
                        expr.span,
                    );
                    if *op == ast::BinaryOp::Div
                        && self.types.is_signed_integer(lhs_type(self, lhs)?)
                    {
                        self.emit_effect(
                            mir::InstructionKind::Check(mir::Check::SignedDivisionOverflow {
                                lhs,
                                rhs,
                            }),
                            expr.span,
                        );
                    }
                }
                mir::InstructionKind::Binary {
                    op: select_binary_op(self.types, *op, lhs_type(self, lhs)?)?,
                    lhs,
                    rhs,
                    overflow: if matches!(op, ast::BinaryOp::Div | ast::BinaryOp::Rem)
                        && self.types.is_integer(lhs_type(self, lhs)?)
                    {
                        mir::OverflowBehavior::Checked
                    } else {
                        mir::OverflowBehavior::Wrap
                    },
                }
            }
            hir::ExprKind::Cast { value } => {
                let value = self.lower_expr(value)?;
                let behavior = if matches!(
                    self.types.underlying_kind(lhs_type(self, value)?),
                    Some(crate::semantic::TypeKind::F32)
                ) && self.types.is_integer(expr.ty)
                {
                    mir::CastBehavior::Checked
                } else {
                    mir::CastBehavior::Infallible
                };
                mir::InstructionKind::Cast {
                    value,
                    to: expr.ty,
                    behavior,
                }
            }
        };
        Some(self.emit_value(expr.ty, kind, expr.span))
    }

    fn lower_values(&mut self, values: &hir::ValueList) -> Option<Vec<mir::ValueId>> {
        match values {
            hir::ValueList::Expressions(values) => {
                values.iter().map(|value| self.lower_expr(value)).collect()
            }
            hir::ValueList::Call(call) => self.lower_call(call),
        }
    }

    fn lower_call(&mut self, call: &hir::Call) -> Option<Vec<mir::ValueId>> {
        // Evaluate the callee first, then arguments left to right. A direct
        // function reference has no evaluation effects of its own.
        let mut interface_data = None;
        let callee = match &call.callee.kind {
            hir::ExprKind::Intrinsic(op) => mir::Callee::Intrinsic(*op),
            hir::ExprKind::Function(symbol) => mir::Callee::Direct(symbol.clone()),
            hir::ExprKind::InterfaceMethod { receiver, index } => {
                let (callee, data) =
                    self.lower_interface_callee(receiver, *index, call.callee.ty, call.span)?;
                interface_data = Some(data);
                mir::Callee::Indirect(callee)
            }
            _ => mir::Callee::Indirect(self.lower_expr(&call.callee)?),
        };
        let mut arguments = call
            .arguments
            .iter()
            .map(|arg| self.lower_expr(arg))
            .collect::<Option<Vec<_>>>()?;
        if let Some(data) = interface_data {
            arguments.insert(0, data);
        }
        let results = call
            .signature
            .returns
            .iter()
            .map(|ty| {
                let id = mir::ValueId(self.values.len() as u32);
                self.values.push(mir::Value {
                    id,
                    ty: *ty,
                    span: call.span,
                });
                id
            })
            .collect::<Vec<_>>();
        self.emit(
            results.clone(),
            mir::InstructionKind::Call {
                callee,
                signature: call.signature.clone(),
                arguments,
                effects: mir::CallEffects::Unknown,
            },
            call.span,
        );
        Some(results)
    }

    fn emit_pre_evaluated_call(
        &mut self,
        call: &hir::Call,
        arguments: Vec<mir::ValueId>,
        span: Span,
    ) -> Option<Vec<mir::ValueId>> {
        let hir::ExprKind::Function(symbol) = &call.callee.kind else {
            return None;
        };
        let results = call
            .signature
            .returns
            .iter()
            .map(|ty| {
                let id = mir::ValueId(self.values.len() as u32);
                self.values.push(mir::Value { id, ty: *ty, span });
                id
            })
            .collect::<Vec<_>>();
        self.emit(
            results.clone(),
            mir::InstructionKind::Call {
                callee: mir::Callee::Direct(symbol.clone()),
                signature: call.signature.clone(),
                arguments,
                effects: mir::CallEffects::Unknown,
            },
            span,
        );
        Some(results)
    }

    fn lower_interface_callee(
        &mut self,
        receiver: &hir::Expr,
        index: u32,
        function_ty: mir::TypeId,
        span: Span,
    ) -> Option<(mir::ValueId, mir::ValueId)> {
        let interface = match self.types.underlying_kind(receiver.ty)? {
            crate::semantic::TypeKind::Interface(interface) => interface.clone(),
            _ => return None,
        };
        let value = self.lower_expr(receiver)?;
        let storage = self.temporary(receiver.ty, span);
        self.store_value(storage.clone(), value, span);
        let data = self.load_interface_field(storage.clone(), 0, interface.data_pointer, span);
        let table = self.load_interface_field(storage, 1, interface.vtable, span);
        let method_place = mir::Place {
            base: mir::PlaceBase::Pointer(table),
            projections: vec![mir::Projection::Field {
                index,
                ty: function_ty,
            }],
            ty: function_ty,
        };
        let callee = self.load_place_value(method_place, span);
        Some((callee, data))
    }

    fn load_interface_field(
        &mut self,
        mut place: mir::Place,
        index: u32,
        ty: mir::TypeId,
        span: Span,
    ) -> mir::ValueId {
        place.projections.push(mir::Projection::Field { index, ty });
        place.ty = ty;
        self.load_place_value(place, span)
    }

    fn lower_interface_equal(
        &mut self,
        lhs: &hir::Expr,
        rhs: &hir::Expr,
        negate: bool,
        span: Span,
    ) -> Option<mir::ValueId> {
        let interface = match self.types.underlying_kind(lhs.ty)? {
            crate::semantic::TypeKind::Interface(interface) => interface.clone(),
            _ => return None,
        };
        let lhs_value = self.lower_expr(lhs)?;
        let lhs_place = self.temporary(lhs.ty, lhs.span);
        self.store_value(lhs_place.clone(), lhs_value, lhs.span);
        let rhs_value = self.lower_expr(rhs)?;
        let rhs_place = self.temporary(rhs.ty, rhs.span);
        self.store_value(rhs_place.clone(), rhs_value, rhs.span);

        let lhs_data =
            self.load_interface_field(lhs_place.clone(), 0, interface.data_pointer, span);
        let rhs_data =
            self.load_interface_field(rhs_place.clone(), 0, interface.data_pointer, span);
        let data_equal = self.emit_value(
            mir::TypeId::BOOL,
            mir::InstructionKind::Binary {
                op: mir::BinaryOp::Equal,
                lhs: lhs_data,
                rhs: rhs_data,
                overflow: mir::OverflowBehavior::Wrap,
            },
            span,
        );
        let nil = self.emit_value(
            interface.data_pointer,
            mir::InstructionKind::Constant(mir::Constant::Nil(interface.data_pointer)),
            span,
        );
        let data_nil = self.emit_value(
            mir::TypeId::BOOL,
            mir::InstructionKind::Binary {
                op: mir::BinaryOp::Equal,
                lhs: lhs_data,
                rhs: nil,
                overflow: mir::OverflowBehavior::Wrap,
            },
            span,
        );
        let lhs_table = self.load_interface_field(lhs_place, 1, interface.vtable, span);
        let rhs_table = self.load_interface_field(rhs_place, 1, interface.vtable, span);
        let table_equal = self.emit_value(
            mir::TypeId::BOOL,
            mir::InstructionKind::Binary {
                op: mir::BinaryOp::Equal,
                lhs: lhs_table,
                rhs: rhs_table,
                overflow: mir::OverflowBehavior::Wrap,
            },
            span,
        );

        let false_block = self.new_block(span);
        let data_match = self.new_block(span);
        let true_block = self.new_block(span);
        let table_block = self.new_block(span);
        let merge = self.new_block(span);
        let result = mir::ValueId(self.values.len() as u32);
        self.values.push(mir::Value {
            id: result,
            ty: mir::TypeId::BOOL,
            span,
        });
        self.blocks[merge.0 as usize].parameters.push(result);
        self.terminate(
            mir::TerminatorKind::Branch {
                condition: data_equal,
                then_edge: edge(data_match),
                else_edge: edge(false_block),
            },
            span,
        );
        self.current = Some(data_match);
        self.terminate(
            mir::TerminatorKind::Branch {
                condition: data_nil,
                then_edge: edge(true_block),
                else_edge: edge(table_block),
            },
            span,
        );
        self.current = Some(false_block);
        let false_value = self.emit_value(
            mir::TypeId::BOOL,
            mir::InstructionKind::Constant(mir::Constant::Bool {
                value: negate,
                ty: mir::TypeId::BOOL,
            }),
            span,
        );
        self.terminate(
            mir::TerminatorKind::Jump(mir::Edge {
                target: merge,
                arguments: vec![false_value],
            }),
            span,
        );
        self.current = Some(true_block);
        let true_value = self.emit_value(
            mir::TypeId::BOOL,
            mir::InstructionKind::Constant(mir::Constant::Bool {
                value: !negate,
                ty: mir::TypeId::BOOL,
            }),
            span,
        );
        self.terminate(
            mir::TerminatorKind::Jump(mir::Edge {
                target: merge,
                arguments: vec![true_value],
            }),
            span,
        );
        self.current = Some(table_block);
        let table_result = if negate {
            self.emit_value(
                mir::TypeId::BOOL,
                mir::InstructionKind::Unary {
                    op: mir::UnaryOp::LogicalNot,
                    operand: table_equal,
                },
                span,
            )
        } else {
            table_equal
        };
        self.terminate(
            mir::TerminatorKind::Jump(mir::Edge {
                target: merge,
                arguments: vec![table_result],
            }),
            span,
        );
        self.current = Some(merge);
        Some(result)
    }

    fn emit_value(
        &mut self,
        ty: mir::TypeId,
        kind: mir::InstructionKind,
        span: Span,
    ) -> mir::ValueId {
        let value = mir::ValueId(self.values.len() as u32);
        self.values.push(mir::Value {
            id: value,
            ty,
            span,
        });
        self.emit(vec![value], kind, span);
        value
    }

    fn emit_effect(&mut self, kind: mir::InstructionKind, span: Span) {
        self.emit(Vec::new(), kind, span);
    }

    fn emit(&mut self, results: Vec<mir::ValueId>, kind: mir::InstructionKind, span: Span) {
        let id = mir::InstructionId(self.next_instruction);
        self.next_instruction += 1;
        self.blocks[self.current.expect("instruction requires an open block").0 as usize]
            .instructions
            .push(mir::Instruction {
                id,
                results,
                kind,
                span,
            });
    }

    fn new_block(&mut self, span: Span) -> mir::BlockId {
        let id = mir::BlockId(self.blocks.len() as u32);
        self.blocks.push(mir::BasicBlock {
            id,
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminator: mir::Terminator {
                kind: mir::TerminatorKind::Unreachable,
                span,
            },
        });
        id
    }

    fn terminate(&mut self, kind: mir::TerminatorKind, span: Span) {
        if let Some(block) = self.current.take() {
            self.blocks[block.0 as usize].terminator = mir::Terminator { kind, span };
        }
    }

    fn jump(&mut self, target: mir::BlockId, span: Span) {
        self.terminate(mir::TerminatorKind::Jump(edge(target)), span);
    }

    fn route_return(&mut self, values: Vec<mir::ValueId>, span: Span) {
        if let Some(target) = self.return_block {
            self.terminate(
                mir::TerminatorKind::Jump(mir::Edge {
                    target,
                    arguments: values,
                }),
                span,
            );
        } else {
            self.terminate(mir::TerminatorKind::Return(values), span);
        }
    }

    fn lower_defer(&mut self, id: usize, block: &hir::Block, span: Span) -> Option<()> {
        let local = *self.defer_flags.get(id)?;
        let active = self.emit_value(
            crate::semantic::TypeId::BOOL,
            mir::InstructionKind::Load {
                place: mir::Place {
                    base: mir::PlaceBase::Local(local),
                    projections: Vec::new(),
                    ty: crate::semantic::TypeId::BOOL,
                },
                access: mir::AccessKind::Local,
            },
            span,
        );
        let run = self.new_block(block.span);
        let next = self.new_block(span);
        self.terminate(
            mir::TerminatorKind::Branch {
                condition: active,
                then_edge: edge(run),
                else_edge: edge(next),
            },
            span,
        );
        self.current = Some(run);
        self.lower_block(block)?;
        self.jump(next, span);
        self.current = Some(next);
        Some(())
    }
}

fn collect_defers<'a>(block: &'a hir::Block, output: &mut Vec<(usize, &'a hir::Block, Span)>) {
    for statement in &block.statements {
        match statement {
            hir::Stmt::Defer { id, block, span } => output.push((*id, block, *span)),
            hir::Stmt::Block(block) => collect_defers(block, output),
            hir::Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_defers(then_block, output);
                collect_defers(else_block, output);
            }
            hir::Stmt::For { body, post, .. } => {
                collect_defers(body, output);
                collect_defers(post, output);
            }
            hir::Stmt::LocalDecl { .. }
            | hir::Stmt::Assign { .. }
            | hir::Stmt::IndexAssign(_)
            | hir::Stmt::Expr(_)
            | hir::Stmt::Call(_)
            | hir::Stmt::Return { .. }
            | hir::Stmt::Break(_)
            | hir::Stmt::Continue(_) => {}
            hir::Stmt::Trap { .. } => {}
        }
    }
}

fn edge(target: mir::BlockId) -> mir::Edge {
    mir::Edge {
        target,
        arguments: Vec::new(),
    }
}

fn lhs_type(builder: &FunctionBuilder<'_>, value: mir::ValueId) -> Option<mir::TypeId> {
    builder.values.get(value.0 as usize).map(|value| value.ty)
}

fn select_binary_op(
    types: &crate::semantic::TypeTable,
    op: ast::BinaryOp,
    ty: mir::TypeId,
) -> Option<mir::BinaryOp> {
    use crate::semantic::TypeKind;
    use ast::BinaryOp as Ast;
    use mir::BinaryOp as Mir;

    let signed = types.is_signed_integer(ty);
    let float = matches!(types.underlying_kind(ty), Some(TypeKind::F32));
    Some(match op {
        Ast::Add if float => Mir::FloatAdd,
        Ast::Sub if float => Mir::FloatSubtract,
        Ast::Mul if float => Mir::FloatMultiply,
        Ast::Div if float => Mir::FloatDivide,
        Ast::Add => Mir::Add,
        Ast::Sub => Mir::Subtract,
        Ast::Mul => Mir::Multiply,
        Ast::Div => Mir::Divide,
        Ast::Rem => Mir::Remainder,
        Ast::BitAnd => Mir::BitAnd,
        Ast::BitOr => Mir::BitOr,
        Ast::BitXor => Mir::BitXor,
        Ast::BitAndNot => Mir::BitAndNot,
        Ast::ShiftLeft => Mir::ShiftLeft,
        Ast::ShiftRight if signed => Mir::ShiftRightArithmetic,
        Ast::ShiftRight => Mir::ShiftRightLogical,
        Ast::Eq if float => Mir::FloatEqual,
        Ast::Ne if float => Mir::FloatNotEqual,
        Ast::Lt if float => Mir::FloatLess,
        Ast::Le if float => Mir::FloatLessEqual,
        Ast::Gt if float => Mir::FloatGreater,
        Ast::Ge if float => Mir::FloatGreaterEqual,
        Ast::Eq => Mir::Equal,
        Ast::Ne => Mir::NotEqual,
        Ast::Lt if signed => Mir::LessSigned,
        Ast::Lt => Mir::LessUnsigned,
        Ast::Le if signed => Mir::LessEqualSigned,
        Ast::Le => Mir::LessEqualUnsigned,
        Ast::Gt if signed => Mir::GreaterSigned,
        Ast::Gt => Mir::GreaterUnsigned,
        Ast::Ge if signed => Mir::GreaterEqualSigned,
        Ast::Ge => Mir::GreaterEqualUnsigned,
        Ast::AndAnd | Ast::OrOr => return None,
    })
}
