//! Global storage types and initializers; type dependencies do not reorder startup.
use super::BodyLowerer;
use crate::{
    ir::{ast, hir},
    project::PackageId,
    source::Span,
};

impl BodyLowerer<'_> {
    pub(super) fn global_value(
        &mut self,
        package_id: PackageId,
        name: &str,
        span: Span,
    ) -> Option<hir::Expr> {
        let package = self.universe.iter().find(|p| p.id == package_id)?;
        let spec = package.globals.get(name)?.clone();
        let symbol = super::super::canonical_symbol_name(&package.logical_path, name);
        let key = (package_id, name.to_string());
        if !self.global_stack.insert(key.clone()) {
            return self.fail(
                span,
                format!("global type inference cycle at `{name}`; add an explicit type"),
            );
        }
        // A package initializer cannot capture the local scope of its caller.
        let saved_package = self.package_id;
        let names = std::mem::take(&mut self.names);
        let constants = std::mem::take(&mut self.constants);
        let null_terminated_constants = std::mem::take(&mut self.null_terminated_constants);
        self.package_id = package_id;
        let ty = (|| {
            if let Some(annotation) = &spec.ty {
                return self.resolve_type(annotation);
            }
            if spec.values.is_empty() {
                return self.fail(spec.span, "global without initializer requires a type");
            }
            let values = self.lower_values(
                &spec.values,
                &vec![None; spec.names.len()],
                spec.span,
                "global declaration",
            )?;
            let index = spec
                .names
                .iter()
                .position(|candidate| candidate.name == name)?;
            values.types().get(index).copied()
        })();
        self.package_id = saved_package;
        self.names = names;
        self.constants = constants;
        self.null_terminated_constants = null_terminated_constants;
        self.global_stack.remove(&key);
        Some(hir::Expr {
            kind: hir::ExprKind::Global(symbol),
            ty: ty?,
            span,
        })
    }

    pub(in super::super) fn lower_global(&mut self, spec: &ast::VarSpec) -> hir::VarItem {
        let mut item = hir::VarItem {
            names: spec.names.iter().map(|n| n.name.clone()).collect(),
            globals: Vec::new(),
            initializer: None,
            span: spec.span,
        };
        let explicit = match &spec.ty {
            Some(ty) => match self.resolve_type(ty) {
                Some(ty) => Some(ty),
                None => return item,
            },
            None => None,
        };
        let values = if spec.values.is_empty() {
            Vec::new()
        } else {
            if spec.values.len() != spec.names.len() {
                self.fail::<()>(spec.span, "global declaration value count mismatch");
                return item;
            }
            let mut values = Vec::new();
            for value in &spec.values {
                let Some(value) = self.constant_expr(value, explicit) else {
                    return item;
                };
                values.push(value);
            }
            values
        };
        let value_types = values.iter().map(|value| value.ty).collect::<Vec<_>>();
        for (index, name) in spec.names.iter().enumerate() {
            let Some(ty) = explicit.or_else(|| value_types.get(index).copied()) else {
                self.fail::<()>(spec.span, "global without initializer requires a type");
                return item;
            };
            let global = if name.name == "_" {
                None
            } else {
                let package = self
                    .universe
                    .iter()
                    .find(|p| p.id == self.package_id)
                    .unwrap();
                Some(hir::Global {
                    symbol: super::super::canonical_symbol_name(&package.logical_path, &name.name),
                    ty,
                    initializer: values.get(index).cloned(),
                    span: name.span,
                })
            };
            item.globals.push(global);
        }
        item
    }
}
