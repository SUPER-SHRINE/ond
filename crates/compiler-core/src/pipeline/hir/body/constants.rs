//! Constant lookup and evaluation in package/local scope.
use super::BodyLowerer;
use crate::{
    ir::{ast, hir},
    project::PackageId,
    source::Span,
};

impl BodyLowerer<'_> {
    fn aggregate_constant(&self, value: &hir::Expr) -> bool {
        matches!(
            self.types.underlying_kind(value.ty),
            Some(crate::semantic::TypeKind::Array { .. } | crate::semantic::TypeKind::Struct(_))
        )
    }

    pub(super) fn anonymous_static(
        &mut self,
        value: hir::Expr,
        span: Span,
        null_terminated: bool,
    ) -> hir::Expr {
        let package = self
            .universe
            .iter()
            .find(|package| package.id == self.package_id)
            .expect("body package must exist in the type universe");
        let symbol = format!(
            "{}.__ond_ro_{}_{}_{}",
            package.logical_path, span.file.0, span.start, span.end
        );
        if !self.statics.iter().any(|item| item.symbol == symbol) {
            self.statics.push(hir::Static {
                symbol: symbol.clone(),
                value: value.clone(),
                exported: false,
                null_terminated,
                span,
            });
        }
        hir::Expr {
            kind: hir::ExprKind::Global(symbol),
            ty: value.ty,
            span,
        }
    }

    fn named_static(
        &self,
        package_id: PackageId,
        name: &str,
        value: &hir::Expr,
        span: Span,
    ) -> hir::Expr {
        let package = self
            .universe
            .iter()
            .find(|package| package.id == package_id)
            .expect("constant package must exist in the type universe");
        hir::Expr {
            kind: hir::ExprKind::Global(super::super::canonical_symbol_name(
                &package.logical_path,
                name,
            )),
            ty: value.ty,
            span,
        }
    }

    pub(in super::super) fn lower_constant_spec(
        &mut self,
        spec: &ast::ConstSpec,
    ) -> Option<Vec<hir::Expr>> {
        if spec.names.len() != spec.values.len() {
            return self.fail(spec.span, "constant declaration value count mismatch");
        }
        let expected = match &spec.ty {
            Some(ty) => Some(self.resolve_type(ty)?),
            None => None,
        };
        let mut values = Vec::new();
        for (name, source) in spec.names.iter().zip(&spec.values) {
            let value = self.constant_expr(source, expected)?;
            if name.name != "_" {
                values.push(value);
            }
        }
        Some(values)
    }
    pub(in super::super) fn constant_use(
        &mut self,
        mut value: hir::Expr,
        expected: Option<crate::semantic::TypeId>,
        span: Span,
    ) -> Option<hir::Expr> {
        if let Some(expected) = expected
            && expected != value.ty
        {
            return self.type_mismatch(span, expected, value.ty);
        }
        value.span = span;
        Some(value)
    }

    pub(super) fn local_constant_use(
        &mut self,
        value: hir::Expr,
        expected: Option<crate::semantic::TypeId>,
        span: Span,
        null_terminated: bool,
    ) -> Option<hir::Expr> {
        let value = self.constant_use(value, expected, span)?;
        if self.constant_depth == 0 && self.aggregate_constant(&value) {
            return Some(self.anonymous_static(value, span, null_terminated));
        }
        Some(value)
    }

    pub(in super::super) fn package_value(
        &mut self,
        package_id: PackageId,
        name: &str,
        expected: Option<crate::semantic::TypeId>,
        span: Span,
    ) -> Option<hir::Expr> {
        let package = self
            .universe
            .iter()
            .find(|package| package.id == package_id)?;
        let Some((annotation, source)) = package.constants.get(name).cloned() else {
            if package.globals.contains_key(name) {
                let value = self.global_value(package_id, name, span)?;
                return self.constant_use(value, expected, span);
            }
            if package.functions.contains_key(name) {
                let value = self.function_value(package_id, name, span)?;
                return self.constant_use(value, expected, span);
            }
            return if package.values.contains(name) {
                self.fail(
                    span,
                    format!("package symbol `{name}` cannot be used as a value"),
                )
            } else {
                self.fail(span, format!("unknown identifier `{name}`"))
            };
        };
        let key = (package_id, name.to_string());
        if !self.const_stack.insert(key.clone()) {
            return self.fail(span, format!("constant dependency cycle at `{name}`"));
        }
        let saved_package = self.package_id;
        let saved_names = std::mem::take(&mut self.names);
        let saved_constants = std::mem::take(&mut self.constants);
        let saved_null_terminated_constants = std::mem::take(&mut self.null_terminated_constants);
        self.package_id = package_id;
        let value = (|| {
            let ty = match annotation {
                Some(ref ty) => Some(self.resolve_type(ty)?),
                None => None,
            };
            self.constant_expr(&source, ty)
        })();
        self.package_id = saved_package;
        self.names = saved_names;
        self.constants = saved_constants;
        self.null_terminated_constants = saved_null_terminated_constants;
        self.const_stack.remove(&key);
        let value = value?;
        if let Some(expected) = expected
            && expected != value.ty
        {
            return self.type_mismatch(span, expected, value.ty);
        }
        if self.constant_depth == 0 && self.aggregate_constant(&value) {
            return Some(self.named_static(package_id, name, &value, span));
        }
        self.constant_use(value, expected, span)
    }

    pub(in super::super) fn constant_expr(
        &mut self,
        source: &ast::Expr,
        expected: Option<crate::semantic::TypeId>,
    ) -> Option<hir::Expr> {
        if matches!(source, ast::Expr::Call { .. }) && self.builtin_name(source) != Some("len") {
            return self.fail(
                source.span(),
                "constant initializer requires a compile-time value",
            );
        }
        self.constant_depth += 1;
        let expr = self.lower_expr(source, expected);
        self.constant_depth -= 1;
        let expr = expr?;
        match crate::constant::evaluate(&expr, self.types) {
            Ok(value) => Some(value),
            Err(message) => self.fail(source.span(), message),
        }
    }
}
