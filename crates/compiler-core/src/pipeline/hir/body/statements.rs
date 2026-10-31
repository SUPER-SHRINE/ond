//! Statement lowering, local declarations and structured control flow.
use super::BodyLowerer;
use crate::ir::{ast, hir};

impl BodyLowerer<'_> {
    fn scoped_block(
        &mut self,
        block: &ast::Block,
        returns: &[crate::semantic::TypeId],
    ) -> Option<hir::Block> {
        let names = self.names.clone();
        let constants = self.constants.clone();
        let null_terminated_constants = self.null_terminated_constants.clone();
        let poisoned_names = self.poisoned_names.clone();
        let scope = std::mem::take(&mut self.scope_names);
        let result = self.lower_block(block, returns);
        self.names = names;
        self.constants = constants;
        self.null_terminated_constants = null_terminated_constants;
        self.poisoned_names = poisoned_names;
        self.scope_names = scope;
        result
    }

    fn condition(&mut self, expr: &ast::Expr) -> Option<hir::Expr> {
        let expr = self.lower_value(expr, None)?;
        if matches!(
            self.types.underlying_kind(expr.ty),
            Some(crate::semantic::TypeKind::Bool)
        ) {
            Some(expr)
        } else {
            self.fail(expr.span, "condition must have type bool")
        }
    }

    pub(super) fn lower_block(
        &mut self,
        block: &ast::Block,
        returns: &[crate::semantic::TypeId],
    ) -> Option<hir::Block> {
        let mut statements = Vec::new();
        for statement in &block.statements {
            let names = self.names.clone();
            let constants = self.constants.clone();
            let null_terminated_constants = self.null_terminated_constants.clone();
            let scope_names = self.scope_names.clone();
            let poisoned_names = self.poisoned_names.clone();
            let locals_len = self.locals.len();
            let statics_len = self.statics.len();
            let loop_depth = self.loop_depth;
            let defer_depth = self.defer_depth;
            let next_defer_id = self.next_defer_id;
            let statements_len = statements.len();
            let lowered = (|| -> Option<()> {
                match statement {
                    ast::Stmt::InlineAssembly(statement) => {
                        statements.push(self.lower_asm(statement)?);
                    }
                    ast::Stmt::Const(declaration) => {
                        for spec in &declaration.specs {
                            if spec.names.len() != spec.values.len() {
                                return self
                                    .fail(spec.span, "constant declaration value count mismatch");
                            }
                            let ty = match &spec.ty {
                                Some(ty) => Some(self.resolve_type(ty)?),
                                None => None,
                            };
                            let mut evaluated = Vec::new();
                            for value in &spec.values {
                                evaluated.push(self.constant_expr(value, ty)?);
                            }
                            for ((name, source), value) in
                                spec.names.iter().zip(&spec.values).zip(evaluated)
                            {
                                if name.name == "_" {
                                    continue;
                                }
                                if !self.scope_names.insert(name.name.clone()) {
                                    return self.fail(
                                        name.span,
                                        format!("duplicate local declaration `{}`", name.name),
                                    );
                                }
                                self.names.remove(&name.name);
                                let null_terminated = matches!(
                                    source,
                                    ast::Expr::Literal(ast::Literal::String(_, _))
                                ) || matches!(source, ast::Expr::Name(path)
                                if path.segments.len() == 1
                                    && self.null_terminated_constants.contains(
                                        &path.segments[0].name
                                    ));
                                if null_terminated {
                                    self.null_terminated_constants.insert(name.name.clone());
                                } else {
                                    self.null_terminated_constants.remove(&name.name);
                                }
                                self.constants.insert(name.name.clone(), value);
                            }
                        }
                    }
                    ast::Stmt::Var(declaration) => {
                        for spec in &declaration.specs {
                            let explicit = match &spec.ty {
                                Some(ty) => Some(self.resolve_type(ty)?),
                                None => None,
                            };
                            let values = if spec.values.is_empty() {
                                let Some(ty) = explicit else {
                                    return self.fail(
                                        spec.span,
                                        "variable without initializer requires a type",
                                    );
                                };
                                hir::ValueList::Expressions(
                                    spec.names
                                        .iter()
                                        .map(|name| hir::Expr {
                                            kind: hir::ExprKind::Zero,
                                            ty,
                                            span: name.span,
                                        })
                                        .collect::<Vec<_>>(),
                                )
                            } else {
                                self.lower_values(
                                    &spec.values,
                                    &vec![explicit; spec.names.len()],
                                    spec.span,
                                    "declaration",
                                )?
                            };
                            let mut locals = Vec::new();
                            for (name, ty) in spec.names.iter().zip(values.types()) {
                                if name.name == "_" {
                                    locals.push(None);
                                    continue;
                                }
                                let local = self.add_local(
                                    name.name.clone(),
                                    ty,
                                    hir::LocalKind::User,
                                    name.span,
                                )?;
                                locals.push(Some(local));
                            }
                            statements.push(hir::Stmt::LocalDecl {
                                locals,
                                values,
                                span: spec.span,
                            });
                        }
                    }
                    ast::Stmt::ShortVar(declaration) => {
                        let mut seen = std::collections::BTreeSet::new();
                        for name in &declaration.names {
                            if name.name != "_" && !seen.insert(&name.name) {
                                return self.fail(name.span, "duplicate name in short declaration");
                            }
                        }
                        if declaration.names.iter().any(|name| {
                            self.scope_names.contains(&name.name)
                                && self.constants.contains_key(&name.name)
                        }) {
                            return self.fail(declaration.span, "cannot assign to a constant");
                        }
                        if !declaration
                            .names
                            .iter()
                            .any(|name| name.name != "_" && !self.scope_names.contains(&name.name))
                        {
                            return self.fail(
                                declaration.span,
                                "short declaration requires at least one new name",
                            );
                        }
                        let expected = declaration
                            .names
                            .iter()
                            .map(|name| {
                                if !self.scope_names.contains(&name.name) {
                                    return None;
                                }
                                self.names
                                    .get(&name.name)
                                    .and_then(|id| self.locals.get(id.0 as usize))
                                    .map(|local| local.ty)
                            })
                            .collect::<Vec<_>>();
                        let values = self.lower_values(
                            &declaration.values,
                            &expected,
                            declaration.span,
                            "declaration",
                        )?;
                        let mut locals = Vec::new();
                        for ((name, expected), ty) in
                            declaration.names.iter().zip(expected).zip(values.types())
                        {
                            if name.name == "_" {
                                locals.push(None);
                                continue;
                            }
                            let local = match expected {
                                Some(_) => *self.names.get(&name.name)?,
                                None => self.add_local(
                                    name.name.clone(),
                                    ty,
                                    hir::LocalKind::User,
                                    name.span,
                                )?,
                            };
                            locals.push(Some(local));
                        }
                        statements.push(hir::Stmt::LocalDecl {
                            locals,
                            values,
                            span: declaration.span,
                        });
                    }
                    ast::Stmt::Assign(assignment) => {
                        for target in &assignment.targets {
                            if self.read_only_target(target) {
                                return self.fail(target.span(), "cannot assign to a constant");
                            }
                        }
                        if let Some(op) = assignment.op
                            && assignment.targets.len() == 1
                            && (assignment.implicit_one || assignment.values.len() == 1)
                            && let ast::Expr::Index { base, index, .. } = &assignment.targets[0]
                        {
                            statements.extend(self.lower_compound_index(
                                base,
                                index,
                                assignment.values.first(),
                                op,
                                assignment.span,
                            )?);
                            return Some(());
                        }
                        if assignment.op.is_none()
                            && assignment.targets.len() == 1
                            && assignment.values.len() == 1
                            && let ast::Expr::Index { base, index, .. } = &assignment.targets[0]
                        {
                            let base = self.lower_expr(base, None)?;
                            let known_index = if super::expressions::contextual_literal(index) {
                                None
                            } else {
                                Some(self.lower_value(index, None)?)
                            };
                            let source_value = &assignment.values[0];
                            let known_value =
                                if super::expressions::contextual_literal(source_value) {
                                    None
                                } else {
                                    Some(self.lower_value(source_value, None)?)
                                };
                            let expected = self.unique_operator_parameter_types(
                                ast::OperatorName::IndexSet,
                                &base,
                                &[
                                    Some(base.ty),
                                    known_index.as_ref().map(|value| value.ty),
                                    known_value.as_ref().map(|value| value.ty),
                                ],
                                assignment.span,
                            );
                            if let Some(expected) = expected {
                                let index = if let Some(index) = known_index {
                                    index
                                } else {
                                    self.lower_value(index, Some(expected[1]))?
                                };
                                let value = if let Some(value) = known_value {
                                    value
                                } else {
                                    self.lower_value(&assignment.values[0], Some(expected[2]))?
                                };
                                if let Some(call) = self.matching_operator_call(
                                    ast::OperatorName::IndexSet,
                                    vec![base, index, value],
                                    assignment.span,
                                )? {
                                    statements.push(hir::Stmt::Call(call));
                                    return Some(());
                                }
                            }
                        }
                        for target in &assignment.targets {
                            if let ast::Expr::Name(path) = target {
                                if path.segments.len() == 1 && path.segments[0].name == "_" {
                                    continue;
                                }
                                if path.segments.len() == 1
                                    && self.constants.contains_key(&path.segments[0].name)
                                {
                                    return self.fail(path.span, "cannot assign to a constant");
                                }
                                if path.segments.len() == 1
                                    && !self.names.contains_key(&path.segments[0].name)
                                {
                                    let name = &path.segments[0].name;
                                    if let Some(package) = self
                                        .universe
                                        .iter()
                                        .find(|package| package.id == self.package_id)
                                    {
                                        if package.constants.contains_key(name) {
                                            return self
                                                .fail(path.span, "cannot assign to a constant");
                                        }
                                        if package.globals.contains_key(name)
                                            || package.functions.contains_key(name)
                                        {
                                            continue;
                                        }
                                    }
                                    if self.poisoned_names.contains(name) {
                                        return self.suppress_poisoned_failure();
                                    }
                                    return self
                                        .fail(path.span, format!("unknown identifier `{name}`"));
                                }
                            }
                        }
                        let targets = assignment
                        .targets
                        .iter()
                        .map(|target| {
                            if matches!(target, ast::Expr::Name(path) if path.segments.len() == 1 && path.segments[0].name == "_") {
                                Some(None)
                            } else { self.lower_target(target).map(Some) }
                        })
                        .collect::<Option<Vec<_>>>()?;
                        let expected = targets
                            .iter()
                            .map(|target| target.as_ref().map(|target| target.ty))
                            .collect::<Vec<_>>();
                        let mut overloaded = None;
                        let values = if let Some(op) = assignment.op {
                            let Some(target) = targets.first().and_then(Option::as_ref) else {
                                return self.fail(
                                    assignment.span,
                                    "compound assignment target cannot be `_`",
                                );
                            };
                            let (values, call) = self.compound_values(assignment, target, op)?;
                            overloaded = call;
                            values
                        } else {
                            self.lower_values(
                                &assignment.values,
                                &expected,
                                assignment.span,
                                "assignment",
                            )?
                        };
                        statements.push(hir::Stmt::Assign {
                            targets,
                            values,
                            op: assignment.op,
                            overloaded,
                            span: assignment.span,
                        });
                    }
                    ast::Stmt::Expr(statement) => {
                        if !matches!(&statement.expr, ast::Expr::Call { .. }) {
                            return self.fail(
                                statement.span,
                                "expression statement must be a function call",
                            );
                        }
                        if matches!(&statement.expr, ast::Expr::Call { .. })
                            && !self.value_builtin(&statement.expr)
                        {
                            statements.push(hir::Stmt::Call(self.lower_call(&statement.expr)?));
                        } else {
                            statements
                                .push(hir::Stmt::Expr(self.lower_value(&statement.expr, None)?));
                        }
                    }
                    ast::Stmt::Return(statement) => {
                        if self.defer_depth > 0 {
                            return self.fail(
                                statement.span,
                                "return is not allowed inside a defer block",
                            );
                        }
                        let expected = returns.iter().copied().map(Some).collect::<Vec<_>>();
                        let values = self.lower_values(
                            &statement.values,
                            &expected,
                            statement.span,
                            "return",
                        )?;
                        statements.push(hir::Stmt::Return {
                            values,
                            span: statement.span,
                        });
                    }
                    ast::Stmt::Defer(statement) => {
                        if self.defer_depth > 0 {
                            return self.fail(statement.span, "defer blocks cannot be nested");
                        }
                        if self.loop_depth > 0 {
                            return self.fail(statement.span, "defer is not allowed inside a loop");
                        }
                        let id = self.next_defer_id;
                        self.next_defer_id += 1;
                        self.defer_depth += 1;
                        let deferred = self.scoped_block(&statement.block, returns);
                        self.defer_depth -= 1;
                        statements.push(hir::Stmt::Defer {
                            id,
                            block: deferred?,
                            span: statement.span,
                        });
                    }
                    ast::Stmt::Trap { message, span } => {
                        statements.push(hir::Stmt::Trap {
                            message: message.clone(),
                            span: *span,
                        });
                    }
                    ast::Stmt::Block(block) => {
                        statements.push(hir::Stmt::Block(self.scoped_block(block, returns)?))
                    }
                    ast::Stmt::If(stmt) => {
                        let names = self.names.clone();
                        let constants = self.constants.clone();
                        let scope = std::mem::take(&mut self.scope_names);
                        let mut outer = if let Some(init) = &stmt.init {
                            self.lower_block(
                                &ast::Block {
                                    statements: vec![(**init).clone()],
                                    span: stmt.span,
                                },
                                returns,
                            )?
                        } else {
                            hir::Block {
                                statements: Vec::new(),
                                span: stmt.span,
                            }
                        };
                        let condition = self.condition(&stmt.condition)?;
                        let then_block = self.scoped_block(&stmt.then_block, returns)?;
                        let else_block = if let Some(branch) = &stmt.else_branch {
                            self.scoped_block(
                                &ast::Block {
                                    statements: vec![(**branch).clone()],
                                    span: stmt.span,
                                },
                                returns,
                            )?
                        } else {
                            hir::Block {
                                statements: Vec::new(),
                                span: stmt.span,
                            }
                        };
                        outer.statements.push(hir::Stmt::If {
                            condition,
                            then_block,
                            else_block,
                            span: stmt.span,
                        });
                        self.names = names;
                        self.constants = constants;
                        self.scope_names = scope;
                        statements.push(hir::Stmt::Block(outer));
                    }
                    ast::Stmt::For(stmt) => {
                        let names = self.names.clone();
                        let constants = self.constants.clone();
                        let scope = std::mem::take(&mut self.scope_names);
                        let (init, condition, post) = match &stmt.kind {
                            ast::ForKind::Infinite => (None, None, None),
                            ast::ForKind::While(condition) => (None, Some(condition), None),
                            ast::ForKind::ThreeClause {
                                init,
                                condition,
                                post,
                            } => (init.as_deref(), condition.as_ref(), post.as_deref()),
                        };
                        let mut outer = if let Some(init) = init {
                            self.lower_block(
                                &ast::Block {
                                    statements: vec![init.clone()],
                                    span: stmt.span,
                                },
                                returns,
                            )?
                        } else {
                            hir::Block {
                                statements: Vec::new(),
                                span: stmt.span,
                            }
                        };
                        let condition = match condition {
                            Some(expr) => Some(self.condition(expr)?),
                            None => None,
                        };
                        let post = if let Some(post) = post {
                            self.lower_block(
                                &ast::Block {
                                    statements: vec![post.clone()],
                                    span: stmt.span,
                                },
                                returns,
                            )?
                        } else {
                            hir::Block {
                                statements: Vec::new(),
                                span: stmt.span,
                            }
                        };
                        self.loop_depth += 1;
                        let body = self.scoped_block(&stmt.body, returns)?;
                        self.loop_depth -= 1;
                        outer.statements.push(hir::Stmt::For {
                            condition,
                            body,
                            post,
                            span: stmt.span,
                        });
                        self.names = names;
                        self.constants = constants;
                        self.scope_names = scope;
                        statements.push(hir::Stmt::Block(outer));
                    }
                    ast::Stmt::Break(span) if self.loop_depth > 0 => {
                        statements.push(hir::Stmt::Break(*span))
                    }
                    ast::Stmt::Continue(span) if self.loop_depth > 0 => {
                        statements.push(hir::Stmt::Continue(*span))
                    }
                    ast::Stmt::Break(span) | ast::Stmt::Continue(span) => {
                        return self.fail(*span, "break/continue requires an enclosing loop");
                    }
                }
                Some(())
            })();
            if lowered.is_none() {
                self.names = names;
                self.constants = constants;
                self.null_terminated_constants = null_terminated_constants;
                self.scope_names = scope_names;
                self.poisoned_names = poisoned_names;
                self.locals.truncate(locals_len);
                self.statics.truncate(statics_len);
                self.loop_depth = loop_depth;
                self.defer_depth = defer_depth;
                self.next_defer_id = next_defer_id;
                statements.truncate(statements_len);
                self.recover_failure(statement_span(statement));
                poison_declared_names(statement, &mut self.poisoned_names);
            }
        }
        Some(hir::Block {
            statements,
            span: block.span,
        })
    }
}

fn poison_declared_names(statement: &ast::Stmt, names: &mut std::collections::BTreeSet<String>) {
    let declared = match statement {
        ast::Stmt::Const(declaration) => declaration
            .specs
            .iter()
            .flat_map(|spec| &spec.names)
            .collect::<Vec<_>>(),
        ast::Stmt::Var(declaration) => declaration
            .specs
            .iter()
            .flat_map(|spec| &spec.names)
            .collect::<Vec<_>>(),
        ast::Stmt::ShortVar(declaration) => declaration.names.iter().collect(),
        _ => Vec::new(),
    };
    for name in declared {
        if name.name != "_" {
            names.insert(name.name.clone());
        }
    }
}

fn statement_span(statement: &ast::Stmt) -> crate::source::Span {
    match statement {
        ast::Stmt::Const(statement) => statement.span,
        ast::Stmt::Var(statement) => statement.span,
        ast::Stmt::ShortVar(statement) => statement.span,
        ast::Stmt::Assign(statement) => statement.span,
        ast::Stmt::Expr(statement) => statement.span,
        ast::Stmt::Return(statement) => statement.span,
        ast::Stmt::Defer(statement) => statement.span,
        ast::Stmt::InlineAssembly(statement) => statement.span,
        ast::Stmt::Trap { span, .. } | ast::Stmt::Break(span) | ast::Stmt::Continue(span) => *span,
        ast::Stmt::Block(statement) => statement.span,
        ast::Stmt::If(statement) => statement.span,
        ast::Stmt::For(statement) => statement.span,
    }
}
