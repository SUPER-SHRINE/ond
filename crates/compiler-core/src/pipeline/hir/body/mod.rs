//! Shared function-local state, scopes and diagnostic handling.
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::ir::{ast, hir};
use crate::project::PackageId;
use crate::source::{SourceDb, Span};
use std::collections::{BTreeMap, BTreeSet};

use super::types::{TypePackage, resolve_type};
mod calls;
mod composites;
mod compound_assignment;
mod compound_index;
mod constants;
mod expressions;
pub(super) use expressions::{index_type_argument, type_argument_index};
mod generation;
mod globals;
mod integer_literal;
mod intrinsics;
mod memory;
mod statements;
mod strings;
mod values;

pub(super) fn lower_typed_body(
    package_id: PackageId,
    signature: &hir::Signature,
    source: &ast::Block,
    universe: &[TypePackage],
    types: &mut crate::semantic::TypeTable,
    sources: &SourceDb,
    diagnostics: &mut Diagnostics,
) -> Option<(hir::Body, Vec<hir::Static>)> {
    lower_typed_body_with_substitutions(
        package_id,
        signature,
        source,
        universe,
        types,
        sources,
        diagnostics,
        BTreeMap::new(),
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_typed_body_with_substitutions(
    package_id: PackageId,
    signature: &hir::Signature,
    source: &ast::Block,
    universe: &[TypePackage],
    types: &mut crate::semantic::TypeTable,
    sources: &SourceDb,
    diagnostics: &mut Diagnostics,
    type_substitutions: BTreeMap<String, crate::semantic::TypeId>,
    generic_caller: Option<&str>,
) -> Option<(hir::Body, Vec<hir::Static>)> {
    let mut lowerer = BodyLowerer {
        package_id,
        universe,
        types,
        locals: Vec::new(),
        names: BTreeMap::new(),
        scope_names: BTreeSet::new(),
        loop_depth: 0,
        defer_depth: 0,
        next_defer_id: 0,
        constants: BTreeMap::new(),
        null_terminated_constants: BTreeSet::new(),
        const_stack: BTreeSet::new(),
        global_stack: BTreeSet::new(),
        statics: Vec::new(),
        static_values: BTreeMap::new(),
        source_values: Vec::new(),
        non_addressable_locals: BTreeSet::new(),
        source_builtin_depth: 0,
        constant_depth: 0,
        failure: None,
        recovered_diagnostics: Diagnostics::new(),
        value_context: None,
        poisoned_names: BTreeSet::new(),
        suppressed_failure: false,
        defer_value_evaluation: false,
        sources: Some(sources),
        type_substitutions,
        generic_caller: generic_caller.map(str::to_owned),
    };
    for parameter in &signature.parameters {
        if lowerer
            .add_local(
                parameter.name.clone(),
                parameter.ty,
                hir::LocalKind::Parameter,
                parameter.span,
            )
            .is_none()
        {
            break;
        }
    }
    let block = lowerer
        .failure
        .is_none()
        .then(|| lowerer.lower_block(source, &signature.returns))
        .flatten();
    if let Some(diagnostic) = lowerer.failure.take() {
        lowerer.recovered_diagnostics.push(diagnostic);
    }
    let valid = lowerer.recovered_diagnostics.is_empty();
    diagnostics.extend(std::mem::take(&mut lowerer.recovered_diagnostics));
    match (block, valid) {
        (Some(mut block), true) => {
            block.statements.splice(0..0, lowerer.source_values);
            Some((
                hir::Body::Typed(hir::TypedBody {
                    locals: lowerer.locals,
                    block,
                }),
                lowerer.statics,
            ))
        }
        (None, true) => {
            diagnostics.push(Diagnostic::error(
                source.span,
                "unsupported or invalid construct in typed HIR lowering",
            ));
            None
        }
        _ => None,
    }
}

pub(super) struct BodyLowerer<'a> {
    generic_caller: Option<String>,
    defer_value_evaluation: bool,
    package_id: PackageId,
    universe: &'a [TypePackage],
    types: &'a mut crate::semantic::TypeTable,
    locals: Vec<hir::Local>,
    names: BTreeMap<String, hir::LocalId>,
    scope_names: BTreeSet<String>,
    loop_depth: usize,
    defer_depth: usize,
    next_defer_id: usize,
    constants: BTreeMap<String, hir::Expr>,
    null_terminated_constants: BTreeSet<String>,
    const_stack: BTreeSet<(PackageId, String)>,
    global_stack: BTreeSet<(PackageId, String)>,
    statics: Vec<hir::Static>,
    static_values: BTreeMap<String, hir::Expr>,
    source_values: Vec<hir::Stmt>,
    non_addressable_locals: BTreeSet<hir::LocalId>,
    source_builtin_depth: usize,
    constant_depth: usize,
    pub(super) failure: Option<Diagnostic>,
    recovered_diagnostics: Diagnostics,
    value_context: Option<String>,
    poisoned_names: BTreeSet<String>,
    suppressed_failure: bool,
    sources: Option<&'a SourceDb>,
    type_substitutions: BTreeMap<String, crate::semantic::TypeId>,
}

impl<'a> BodyLowerer<'a> {
    fn record_generic_request(&self, symbol: &str, span: Span) {
        let package = self
            .universe
            .iter()
            .find(|p| p.id == self.package_id)
            .expect("generic request has a source package");
        let request = super::generic_paths::Request {
            caller: self.generic_caller.clone(),
            symbol: symbol.to_owned(),
            span,
        };
        let mut requests = package.generic_requests.borrow_mut();
        if !requests.contains(&request) {
            requests.push(request);
        }
    }

    pub(super) fn imported_package(&self, qualifier: &str, span: Span) -> Option<PackageId> {
        self.universe
            .iter()
            .find(|package| package.id == self.package_id)?
            .imports
            .get(&span.file)?
            .get(qualifier)
            .copied()
    }

    pub(super) fn new(
        package_id: PackageId,
        universe: &'a [TypePackage],
        types: &'a mut crate::semantic::TypeTable,
    ) -> Self {
        Self {
            generic_caller: None,
            package_id,
            universe,
            types,
            locals: Vec::new(),
            names: BTreeMap::new(),
            scope_names: BTreeSet::new(),
            loop_depth: 0,
            defer_depth: 0,
            next_defer_id: 0,
            constants: BTreeMap::new(),
            null_terminated_constants: BTreeSet::new(),
            const_stack: BTreeSet::new(),
            global_stack: BTreeSet::new(),
            statics: Vec::new(),
            static_values: BTreeMap::new(),
            source_values: Vec::new(),
            non_addressable_locals: BTreeSet::new(),
            source_builtin_depth: 0,
            constant_depth: 0,
            failure: None,
            recovered_diagnostics: Diagnostics::new(),
            value_context: None,
            poisoned_names: BTreeSet::new(),
            suppressed_failure: false,
            defer_value_evaluation: false,
            sources: None,
            type_substitutions: BTreeMap::new(),
        }
    }

    pub(super) fn new_with_sources(
        package_id: PackageId,
        universe: &'a [TypePackage],
        types: &'a mut crate::semantic::TypeTable,
        sources: &'a SourceDb,
    ) -> Self {
        let mut lowerer = Self::new(package_id, universe, types);
        lowerer.sources = Some(sources);
        lowerer
    }

    pub(super) fn take_statics(&mut self) -> Vec<hir::Static> {
        std::mem::take(&mut self.statics)
    }
    fn fail<T>(&mut self, span: Span, message: impl Into<String>) -> Option<T> {
        if self.failure.is_none() {
            self.failure = Some(Diagnostic::error(span, message));
        }
        None
    }

    fn type_mismatch<T>(
        &mut self,
        span: Span,
        expected: crate::semantic::TypeId,
        actual: crate::semantic::TypeId,
    ) -> Option<T> {
        let expected_name = self.types.display(expected).to_string();
        let actual_name = self.types.display(actual).to_string();
        let context = self.value_context.as_deref().unwrap_or("value");
        let message =
            format!("{context} type mismatch: expected `{expected_name}`, found `{actual_name}`");
        self.fail(span, message)
    }

    pub(super) fn recover_failure(&mut self, fallback_span: Span) {
        if let Some(diagnostic) = self.failure.take() {
            self.recovered_diagnostics.push(diagnostic);
        } else if !self.suppressed_failure {
            self.recovered_diagnostics.push(Diagnostic::error(
                fallback_span,
                "unsupported or invalid construct in typed HIR lowering",
            ));
        }
        self.suppressed_failure = false;
    }

    pub(super) fn suppress_poisoned_failure<T>(&mut self) -> Option<T> {
        self.suppressed_failure = true;
        None
    }

    fn resolve_type(&mut self, ty: &ast::Type) -> Option<crate::semantic::TypeId> {
        let substitutions = self
            .type_substitutions
            .iter()
            .map(|(name, ty)| (name.as_str(), *ty))
            .collect::<BTreeMap<_, _>>();
        let concrete = super::types::substitute_type(ty, &substitutions);
        let mut diagnostics = Diagnostics::new();
        let result = resolve_type(
            self.package_id,
            &concrete,
            self.universe,
            self.types,
            false,
            &mut BTreeSet::new(),
            &mut diagnostics,
        );
        if let Some(diagnostic) = diagnostics.into_vec().into_iter().next() {
            self.failure = Some(diagnostic);
        }
        if let Some(ty) = result {
            self.record_generation_type(ty);
        }
        result
    }

    fn add_local(
        &mut self,
        name: String,
        ty: crate::semantic::TypeId,
        kind: hir::LocalKind,
        span: Span,
    ) -> Option<hir::LocalId> {
        if name != "_" && self.scope_names.contains(&name) {
            return self.fail(span, format!("duplicate local declaration `{name}`"));
        }
        let id = hir::LocalId(self.locals.len() as u32);
        self.locals.push(hir::Local {
            id,
            name: name.clone(),
            ty,
            kind,
            span,
        });
        if name != "_" {
            self.scope_names.insert(name.clone());
            self.constants.remove(&name);
            self.null_terminated_constants.remove(&name);
            self.names.insert(name, id);
        }
        Some(id)
    }
}
