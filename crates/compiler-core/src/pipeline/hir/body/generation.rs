//! Retain source observations before value folding and temporary storage lowering.
use super::BodyLowerer;
use crate::{
    ir::{ast, hir},
    semantic::{TypeId, TypeTable},
};

impl BodyLowerer<'_> {
    pub(super) fn record_generation_type(&self, ty: TypeId) {
        if self.sources.is_none() {
            return;
        }
        let Some(package) = self
            .universe
            .iter()
            .find(|package| package.id == self.package_id)
        else {
            return;
        };
        let mut collector = package.generation.borrow_mut();
        if collector.enabled {
            collector.types.insert(ty);
        }
    }

    pub(super) fn record_source_expression(&self, source: &ast::Expr, value: &hir::Expr) {
        if self.sources.is_none()
            || matches!(source, ast::Expr::Name(path) if path.segments.iter().any(|name| name.name.starts_with('\u{1}')))
        {
            return;
        }
        let Some(package) = self
            .universe
            .iter()
            .find(|package| package.id == self.package_id)
        else {
            return;
        };
        let mut collector = package.generation.borrow_mut();
        if !collector.enabled {
            return;
        }
        collector.types.insert(value.ty);
        let node = super::super::generation::Node {
            span: source.span(),
            caller: self.generic_caller.clone(),
            ty: value.ty,
            skipped: self.defer_value_evaluation,
            static_value: matches!(
                source,
                ast::Expr::Composite { .. } | ast::Expr::Literal(ast::Literal::String(_, _))
            ),
            value: if self.defer_value_evaluation {
                None
            } else {
                semantic_value(
                    value,
                    &collector.values,
                    &self.generic_caller,
                    self.types,
                    &self.static_values,
                )
            },
        };
        if let Some(existing) = collector
            .nodes
            .iter_mut()
            .find(|existing| existing.span == node.span && existing.caller == node.caller)
        {
            if existing.skipped || !node.skipped {
                *existing = node;
            }
        } else {
            collector.nodes.push(node);
        }
    }

    pub(super) fn record_evaluated(&self, value: &hir::Expr) {
        if self.sources.is_none() || self.defer_value_evaluation || self.source_builtin_depth != 0 {
            return;
        }
        let Some(package) = self
            .universe
            .iter()
            .find(|package| package.id == self.package_id)
        else {
            return;
        };
        let mut collector = package.generation.borrow_mut();
        if collector.enabled {
            collector
                .values
                .push((value.span, self.generic_caller.clone(), value.clone()));
        }
    }
}

fn semantic_value(
    value: &hir::Expr,
    observations: &[(crate::source::Span, Option<String>, hir::Expr)],
    caller: &Option<String>,
    types: &TypeTable,
    static_values: &std::collections::BTreeMap<String, hir::Expr>,
) -> Option<hir::Expr> {
    if evaluated(value) {
        return Some(value.clone());
    }
    if let hir::ExprKind::Composite(elements) = &value.kind {
        let elements = elements
            .iter()
            .map(|(index, child)| {
                Some((
                    *index,
                    semantic_value(child, observations, caller, types, static_values)?,
                ))
            })
            .collect::<Option<Vec<_>>>()?;
        return Some(hir::Expr {
            kind: hir::ExprKind::Composite(elements),
            ..value.clone()
        });
    }
    let member = match &value.kind {
        hir::ExprKind::Field { base, index } => Some(hir::ExprKind::Field {
            base: Box::new(semantic_value(
                base,
                observations,
                caller,
                types,
                static_values,
            )?),
            index: *index,
        }),
        hir::ExprKind::Index { base, index } => Some(hir::ExprKind::Index {
            base: Box::new(semantic_value(
                base,
                observations,
                caller,
                types,
                static_values,
            )?),
            index: Box::new(semantic_value(
                index,
                observations,
                caller,
                types,
                static_values,
            )?),
        }),
        _ => None,
    };
    if let Some(kind) = member {
        return crate::constant::evaluate(
            &hir::Expr {
                kind,
                ..value.clone()
            },
            types,
        )
        .ok();
    }
    if let hir::ExprKind::Global(symbol) = &value.kind {
        if let Some(value) = static_values.get(symbol) {
            return Some(value.clone());
        }
        return observations
            .iter()
            .rev()
            .find(|(span, owner, _)| *span == value.span && owner == caller)
            .map(|(_, _, value)| value.clone());
    }
    None
}

fn evaluated(value: &hir::Expr) -> bool {
    match &value.kind {
        hir::ExprKind::Zero
        | hir::ExprKind::Integer(_)
        | hir::ExprKind::Bool(_)
        | hir::ExprKind::Float32(_)
        | hir::ExprKind::Nil => true,
        hir::ExprKind::Composite(elements) => elements.iter().all(|(_, value)| evaluated(value)),
        _ => false,
    }
}
