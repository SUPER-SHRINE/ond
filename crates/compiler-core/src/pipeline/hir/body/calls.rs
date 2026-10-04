//! Function references, call checking and multiple-result contexts.
use super::BodyLowerer;
use crate::{
    diagnostic::Diagnostics,
    ir::{ast, hir},
    project::PackageId,
    semantic::{TypeId, TypeKind},
    source::Span,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct ResolvedMethodCandidate {
    pub(super) package_id: PackageId,
    pub(super) logical_path: String,
    pub(super) receiver_name: String,
    pub(super) signature: hir::Signature,
    pub(super) symbol: String,
    pub(super) span: Span,
}

impl BodyLowerer<'_> {
    pub(super) fn unique_operator_parameter_types(
        &mut self,
        name: ast::OperatorName,
        known: &[Option<TypeId>],
    ) -> Option<Vec<TypeId>> {
        let declarations = self
            .universe
            .iter()
            .flat_map(|package| {
                package
                    .operators
                    .iter()
                    .enumerate()
                    .filter(move |(_, operator)| operator.name == name)
                    .map(move |(index, operator)| (package.id, index, operator.clone()))
            })
            .collect::<Vec<_>>();
        let mut found = None;
        for (package_id, _, declaration) in declarations {
            let Some((signature, _)) =
                self.resolve_operator_candidate(package_id, &declaration, known)
            else {
                continue;
            };
            let parameters = signature
                .parameters
                .iter()
                .map(|parameter| parameter.ty)
                .collect::<Vec<_>>();
            if found.is_some() {
                return None;
            }
            found = Some(parameters);
        }
        found
    }

    pub(super) fn matching_operator_call(
        &mut self,
        name: ast::OperatorName,
        arguments: Vec<hir::Expr>,
        span: Span,
    ) -> Option<Option<hir::Call>> {
        let argument_types = arguments
            .iter()
            .map(|argument| argument.ty)
            .collect::<Vec<_>>();
        let declarations = self
            .universe
            .iter()
            .flat_map(|package| {
                package
                    .operators
                    .iter()
                    .enumerate()
                    .filter(move |(_, operator)| operator.name == name)
                    .map(move |(index, operator)| {
                        (
                            package.id,
                            package.logical_path.clone(),
                            index,
                            operator.clone(),
                        )
                    })
            })
            .collect::<Vec<_>>();
        let mut found = None;
        let known = argument_types.iter().copied().map(Some).collect::<Vec<_>>();
        for (package_id, logical_path, declaration_index, declaration) in declarations {
            let Some((signature, type_arguments)) =
                self.resolve_operator_candidate(package_id, &declaration, &known)
            else {
                continue;
            };
            let parameters = signature
                .parameters
                .iter()
                .map(|parameter| parameter.ty)
                .collect::<Vec<_>>();
            if parameters != argument_types {
                continue;
            }
            if found.is_some() {
                self.fail::<()>(span, "ambiguous operator declaration")?;
            }
            let function_type = signature.function_type();
            let ty = self.types.function(function_type.clone(), span);
            let symbol = if let Some(type_arguments) = type_arguments {
                super::super::canonical_generic_operator_symbol(
                    &logical_path,
                    name,
                    declaration_index,
                    &type_arguments,
                )
            } else {
                super::super::canonical_operator_symbol(&logical_path, name, &parameters)
            };
            found = Some(hir::Call {
                callee: Box::new(hir::Expr {
                    kind: hir::ExprKind::Function(symbol),
                    ty,
                    span,
                }),
                signature: function_type,
                arguments: arguments.clone(),
                span,
            });
        }
        Some(found)
    }

    fn resolve_operator_candidate(
        &mut self,
        package_id: PackageId,
        declaration: &ast::OperatorDecl,
        known: &[Option<TypeId>],
    ) -> Option<(hir::Signature, Option<Vec<TypeId>>)> {
        if declaration.signature.params.len() != known.len() {
            return None;
        }
        let type_arguments = if declaration.type_params.is_empty() {
            None
        } else {
            let mut bindings = BTreeMap::new();
            let parameter_names = declaration
                .type_params
                .iter()
                .map(|parameter| parameter.name.as_str())
                .collect::<BTreeSet<_>>();
            for (parameter, actual) in declaration.signature.params.iter().zip(known) {
                if let Some(actual) = actual
                    && !self.infer_operator_type_pattern(
                        package_id,
                        &parameter.ty,
                        *actual,
                        &parameter_names,
                        &mut bindings,
                    )
                {
                    return None;
                }
            }
            let arguments = declaration
                .type_params
                .iter()
                .map(|parameter| bindings.get(&parameter.name).copied())
                .collect::<Option<Vec<_>>>()?;
            Some(arguments)
        };
        let source_signature = type_arguments.as_ref().map_or_else(
            || declaration.signature.clone(),
            |arguments| {
                super::super::types::substitute_signature(
                    &declaration.signature,
                    &declaration.type_params,
                    arguments,
                )
            },
        );
        let mut diagnostics = Diagnostics::new();
        let signature = super::super::types::resolve_signature(
            package_id,
            &source_signature,
            self.universe,
            self.types,
            &mut diagnostics,
        )?;
        let parameters = signature
            .parameters
            .iter()
            .map(|parameter| parameter.ty)
            .collect::<Vec<_>>();
        if known
            .iter()
            .zip(&parameters)
            .any(|(known, actual)| known.is_some_and(|known| known != *actual))
        {
            return None;
        }
        if type_arguments.is_some() {
            let package = self
                .universe
                .iter()
                .find(|package| package.id == package_id)?;
            let owner = parameters
                .iter()
                .find_map(|ty| super::super::operator_type_owner(self.types, *ty));
            if owner.as_deref() != Some(package.logical_path.as_str()) {
                return None;
            }
        }
        Some((signature, type_arguments))
    }

    fn infer_operator_type_pattern(
        &self,
        package_id: PackageId,
        pattern: &ast::Type,
        actual: TypeId,
        parameters: &BTreeSet<&str>,
        bindings: &mut BTreeMap<String, TypeId>,
    ) -> bool {
        match pattern {
            ast::Type::Named(path) if matches!(path.segments.as_slice(), [name] if parameters.contains(name.name.as_str())) =>
            {
                let name = &path.segments[0].name;
                bindings.get(name).is_none_or(|bound| *bound == actual) && {
                    bindings.insert(name.clone(), actual);
                    true
                }
            }
            ast::Type::Pointer(inner, _) => {
                let Some(TypeKind::Pointer(pointee)) = self.types.kind(actual) else {
                    return false;
                };
                self.infer_operator_type_pattern(package_id, inner, *pointee, parameters, bindings)
            }
            ast::Type::Apply {
                base, arguments, ..
            } => {
                let Some(instance) = self.types.generic_instance(actual) else {
                    return false;
                };
                let declaration = instance.declaration.clone();
                let actual_arguments = instance.arguments.clone();
                if self
                    .operator_pattern_declaration(package_id, base)
                    .as_deref()
                    != Some(declaration.as_str())
                    || arguments.len() != actual_arguments.len()
                {
                    return false;
                }
                arguments
                    .iter()
                    .zip(actual_arguments)
                    .all(|(pattern, actual)| {
                        self.infer_operator_type_pattern(
                            package_id, pattern, actual, parameters, bindings,
                        )
                    })
            }
            ast::Type::Array { element, .. } => {
                let Some(TypeKind::Array {
                    element: actual, ..
                }) = self.types.kind(actual)
                else {
                    return false;
                };
                self.infer_operator_type_pattern(package_id, element, *actual, parameters, bindings)
            }
            ast::Type::Struct { fields, .. } => {
                let Some(TypeKind::Struct(actual)) = self.types.kind(actual) else {
                    return false;
                };
                let actual = actual
                    .fields
                    .iter()
                    .map(|field| field.ty)
                    .collect::<Vec<_>>();
                fields.len() == actual.len()
                    && fields.iter().zip(actual).all(|(field, actual)| {
                        self.infer_operator_type_pattern(
                            package_id, &field.ty, actual, parameters, bindings,
                        )
                    })
            }
            ast::Type::Func { signature, .. } => {
                let Some(TypeKind::Function(actual)) = self.types.kind(actual) else {
                    return false;
                };
                let actual = actual.clone();
                signature.params.len() == actual.parameters.len()
                    && signature.results.len() == actual.returns.len()
                    && signature
                        .params
                        .iter()
                        .zip(actual.parameters)
                        .all(|(parameter, actual)| {
                            self.infer_operator_type_pattern(
                                package_id,
                                &parameter.ty,
                                actual,
                                parameters,
                                bindings,
                            )
                        })
                    && signature
                        .results
                        .iter()
                        .zip(actual.returns)
                        .all(|(result, actual)| {
                            self.infer_operator_type_pattern(
                                package_id, result, actual, parameters, bindings,
                            )
                        })
            }
            _ => true,
        }
    }

    fn operator_pattern_declaration(
        &self,
        package_id: PackageId,
        base: &ast::Path,
    ) -> Option<String> {
        let package = self
            .universe
            .iter()
            .find(|package| package.id == package_id)?;
        match base.segments.as_slice() {
            [name] => Some(format!("{}.{}", package.logical_path, name.name)),
            [qualifier, name] => {
                let owner = package.imports.get(&base.span.file)?.get(&qualifier.name)?;
                let owner = self.universe.iter().find(|package| package.id == *owner)?;
                Some(format!("{}.{}", owner.logical_path, name.name))
            }
            _ => None,
        }
    }

    pub(super) fn function_value(
        &mut self,
        package_id: PackageId,
        name: &str,
        span: Span,
    ) -> Option<hir::Expr> {
        let package = self
            .universe
            .iter()
            .find(|package| package.id == package_id)?;
        let source = package.functions.get(name)?.clone();
        let symbol = super::super::canonical_symbol_name(&package.logical_path, name);
        let mut diagnostics = Diagnostics::new();
        let signature = super::super::types::resolve_signature(
            package_id,
            &source,
            self.universe,
            self.types,
            &mut diagnostics,
        );
        if let Some(diagnostic) = diagnostics.into_vec().into_iter().next() {
            self.failure = Some(diagnostic);
            return None;
        }
        let ty = self
            .types
            .intern(TypeKind::Function(signature?.function_type()), span);
        Some(hir::Expr {
            kind: hir::ExprKind::Function(symbol),
            ty,
            span,
        })
    }

    pub(super) fn lower_call(&mut self, source: &ast::Expr) -> Option<hir::Call> {
        let ast::Expr::Call { callee, args, span } = source else {
            return None;
        };
        if let ast::Expr::TypeApply {
            base,
            arguments,
            span: application_span,
        } = callee.as_ref()
        {
            let ast::Expr::Name(path) = base.as_ref() else {
                return self.fail(*span, "type arguments require a named function");
            };
            if matches!(path.segments.as_slice(), [name] if name.name == "alloc") {
                if arguments.len() != 1 {
                    return self.fail(*span, "alloc expects 1 type argument");
                }
                let element = self.resolve_type(&arguments[0])?;
                return self.lower_typed_alloc(element, args, *span);
            }
            if self.generic_function_exists(path) {
                return self.lower_generic_function_call(path, arguments, args, *span);
            }
            // `values[index]()` and `Function[Type]()` have the same token
            // shape.  The parser preserves the latter form; when the name is
            // not a generic function, reinterpret a single name argument as
            // ordinary indexing and call the indexed function value.
            if let [ast::Type::Named(index)] = arguments.as_slice() {
                let indexed_call = ast::Expr::Call {
                    callee: Box::new(ast::Expr::Index {
                        base: base.clone(),
                        index: Box::new(ast::Expr::Name(index.clone())),
                        span: *application_span,
                    }),
                    args: args.clone(),
                    span: *span,
                };
                return self.lower_call(&indexed_call);
            }
            return self.fail(*span, "type arguments require a generic function");
        }
        if let Some(name @ ("alloc" | "free" | "load32" | "store32")) = self.builtin_name(source) {
            if name == "alloc" {
                return self.fail(*span, "alloc requires a type argument: alloc[T](count)");
            }
            return self.lower_intrinsic(name, args, *span);
        }
        match callee.as_ref() {
            ast::Expr::Selector { base, field, .. } => {
                if let Some(call) = self.lower_interface_call(base, &field.name, args, *span) {
                    return call;
                }
                if let Some(call) = self.lower_method_call(base, &field.name, args, *span) {
                    return call;
                }
            }
            ast::Expr::Name(path)
                if path.segments.len() >= 2 && self.path_starts_with_value(path) =>
            {
                let base = ast::Expr::Name(ast::Path {
                    segments: path.segments[..path.segments.len() - 1].to_vec(),
                    span: path.span,
                });
                let name = &path.segments.last()?.name;
                if let Some(call) = self.lower_interface_call(&base, name, args, *span) {
                    return call;
                }
                if let Some(call) = self.lower_method_call(&base, name, args, *span) {
                    return call;
                }
            }
            _ => {}
        }
        let callee = self.lower_value(callee, None)?;
        let Some(TypeKind::Function(signature)) = self.types.underlying_kind(callee.ty) else {
            return self.fail(callee.span, "called expression is not a function");
        };
        let signature = signature.clone();
        if args.len() != signature.parameters.len() {
            return self.fail(
                *span,
                format!(
                    "function expects {} arguments, found {}",
                    signature.parameters.len(),
                    args.len()
                ),
            );
        }
        let mut arguments = Vec::new();
        for (index, (argument, ty)) in args.iter().zip(&signature.parameters).enumerate() {
            let Some(value) = self.lower_value(argument, Some(*ty)) else {
                if let Some(diagnostic) = &mut self.failure {
                    diagnostic.message = format!(
                        "argument {} type mismatch or invalid expression: {}",
                        index + 1,
                        diagnostic.message
                    );
                }
                return None;
            };
            arguments.push(value);
        }
        Some(hir::Call {
            callee: Box::new(callee),
            signature,
            arguments,
            span: *span,
        })
    }

    fn lower_generic_function_call(
        &mut self,
        path: &ast::Path,
        type_arguments: &[ast::Type],
        arguments: &[ast::Expr],
        span: Span,
    ) -> Option<hir::Call> {
        let (package_id, logical_path, declaration) = match path.segments.as_slice() {
            [name] => {
                let package = self
                    .universe
                    .iter()
                    .find(|package| package.id == self.package_id)?;
                let declaration = package.generic_functions.get(&name.name)?.clone();
                (package.id, package.logical_path.clone(), declaration)
            }
            [qualifier, name] => {
                if !name
                    .name
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_uppercase)
                {
                    return self.fail(span, format!("function `{}` is not exported", name.name));
                }
                let package_id = self.imported_package(&qualifier.name, path.span)?;
                let package = self
                    .universe
                    .iter()
                    .find(|package| package.id == package_id)?;
                let declaration = package.generic_functions.get(&name.name)?.clone();
                (package.id, package.logical_path.clone(), declaration)
            }
            _ => return self.fail(span, "invalid generic function name"),
        };
        if declaration.type_params.len() != type_arguments.len() {
            return self.fail(
                span,
                format!(
                    "generic function `{}` expects {} type arguments, got {}",
                    declaration.name.name,
                    declaration.type_params.len(),
                    type_arguments.len()
                ),
            );
        }
        let concrete_types = type_arguments
            .iter()
            .map(|argument| self.resolve_type(argument))
            .collect::<Option<Vec<_>>>()?;
        let source_signature = super::super::types::substitute_signature(
            &declaration.signature,
            &declaration.type_params,
            &concrete_types,
        );
        let mut diagnostics = Diagnostics::new();
        let signature = super::super::types::resolve_signature(
            package_id,
            &source_signature,
            self.universe,
            self.types,
            &mut diagnostics,
        );
        if let Some(diagnostic) = diagnostics.into_vec().into_iter().next() {
            self.failure = Some(diagnostic);
            return None;
        }
        let signature = signature?;
        if arguments.len() != signature.parameters.len() {
            return self.fail(
                span,
                format!(
                    "function expects {} arguments, found {}",
                    signature.parameters.len(),
                    arguments.len()
                ),
            );
        }
        let mut lowered = Vec::new();
        for (index, (argument, parameter)) in
            arguments.iter().zip(&signature.parameters).enumerate()
        {
            let Some(value) = self.lower_value(argument, Some(parameter.ty)) else {
                if let Some(diagnostic) = &mut self.failure {
                    diagnostic.message = format!(
                        "argument {} type mismatch or invalid expression: {}",
                        index + 1,
                        diagnostic.message
                    );
                }
                return None;
            };
            lowered.push(value);
        }
        let function_type = signature.function_type();
        let function_ty = self.types.function(function_type.clone(), span);
        Some(hir::Call {
            callee: Box::new(hir::Expr {
                kind: hir::ExprKind::Function(super::super::canonical_generic_symbol(
                    &logical_path,
                    &declaration.name.name,
                    &concrete_types,
                )),
                ty: function_ty,
                span,
            }),
            signature: function_type,
            arguments: lowered,
            span,
        })
    }

    fn generic_function_exists(&self, path: &ast::Path) -> bool {
        match path.segments.as_slice() {
            [name] => {
                if self.names.contains_key(&name.name) || self.constants.contains_key(&name.name) {
                    return false;
                }
                self.universe
                    .iter()
                    .find(|package| package.id == self.package_id)
                    .is_some_and(|package| package.generic_functions.contains_key(&name.name))
            }
            [qualifier, name] => self
                .universe
                .iter()
                .find(|package| package.id == self.package_id)
                .and_then(|package| package.imports.get(&path.span.file))
                .and_then(|imports| imports.get(&qualifier.name))
                .and_then(|package_id| {
                    self.universe
                        .iter()
                        .find(|package| package.id == *package_id)
                })
                .is_some_and(|package| package.generic_functions.contains_key(&name.name)),
            _ => false,
        }
    }

    fn path_starts_with_value(&self, path: &ast::Path) -> bool {
        let Some(first) = path.segments.first() else {
            return false;
        };
        self.names.contains_key(&first.name)
            || self.constants.contains_key(&first.name)
            || self
                .universe
                .iter()
                .find(|package| package.id == self.package_id)
                .is_some_and(|package| {
                    package.values.contains(&first.name)
                        || package.constants.contains_key(&first.name)
                })
    }

    fn lower_method_call(
        &mut self,
        base: &ast::Expr,
        name: &str,
        args: &[ast::Expr],
        span: Span,
    ) -> Option<Option<hir::Call>> {
        let candidates = self
            .universe
            .iter()
            .flat_map(|package| {
                package
                    .methods
                    .iter()
                    .enumerate()
                    .filter(move |(_, method)| method.name.name == name)
                    .map(move |(index, method)| (package.id, index, method.clone()))
            })
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return None;
        }

        let receiver = match self.lower_value(base, None) {
            Some(receiver) => receiver,
            None => return Some(None),
        };
        for (package_id, declaration_index, method) in candidates {
            let Some(candidate) =
                self.resolve_method_candidate(package_id, declaration_index, &method, receiver.ty)
            else {
                continue;
            };
            let signature = candidate.signature;
            if candidate.package_id != self.package_id
                && !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
            {
                return Some(self.fail(span, format!("method `{name}` is not exported")));
            }
            let expected = &signature.parameters[1..];
            if args.len() != expected.len() {
                return Some(self.fail(
                    span,
                    format!(
                        "method expects {} arguments, found {}",
                        expected.len(),
                        args.len()
                    ),
                ));
            }
            let mut arguments = vec![receiver];
            for (index, (argument, parameter)) in args.iter().zip(expected).enumerate() {
                let Some(value) = self.lower_value(argument, Some(parameter.ty)) else {
                    if let Some(diagnostic) = &mut self.failure {
                        diagnostic.message = format!(
                            "argument {} type mismatch or invalid expression: {}",
                            index + 1,
                            diagnostic.message
                        );
                    }
                    return Some(None);
                };
                arguments.push(value);
            }
            let function_type = signature.function_type();
            let function_ty = self
                .types
                .intern(TypeKind::Function(function_type.clone()), span);
            let callee = hir::Expr {
                kind: hir::ExprKind::Function(candidate.symbol),
                ty: function_ty,
                span,
            };
            return Some(Some(hir::Call {
                callee: Box::new(callee),
                signature: function_type,
                arguments,
                span,
            }));
        }
        None
    }

    pub(super) fn resolve_method_candidate(
        &mut self,
        package_id: PackageId,
        declaration_index: usize,
        method: &ast::FuncDecl,
        actual_receiver: TypeId,
    ) -> Option<ResolvedMethodCandidate> {
        let package = self
            .universe
            .iter()
            .find(|package| package.id == package_id)?;
        let receiver = method.receiver.as_ref()?;
        let (receiver_name, _) = super::super::types::method_receiver_name(receiver)?;
        let receiver_name = receiver_name.to_string();
        let logical_path = package.logical_path.clone();
        let declaration = package.declarations.get(&receiver_name)?;
        let (receiver_source, signature_source, symbol) = if declaration.type_params.is_empty() {
            (
                (**receiver).clone(),
                method.signature.clone(),
                super::super::canonical_method_symbol(
                    &logical_path,
                    &receiver_name,
                    &method.name.name,
                ),
            )
        } else {
            let generic = super::super::types::generic_method_receiver(receiver)?;
            let applied_receiver = if generic.pointer {
                match self.types.kind(actual_receiver) {
                    Some(TypeKind::Pointer(pointee)) => *pointee,
                    _ => return None,
                }
            } else {
                actual_receiver
            };
            let instance = self.types.generic_instance(applied_receiver)?;
            if instance.declaration != format!("{}.{}", logical_path, receiver_name)
                || instance.arguments.len() != generic.parameters.len()
            {
                return None;
            }
            let arguments = instance.arguments.clone();
            let substitutions = generic
                .parameters
                .iter()
                .map(|parameter| parameter.name.as_str())
                .zip(arguments.iter().copied())
                .collect();
            (
                ast::Field {
                    name: receiver.name.clone(),
                    ty: super::super::types::substitute_type(&receiver.ty, &substitutions),
                    span: receiver.span,
                },
                super::super::types::substitute_signature(
                    &method.signature,
                    &generic.parameters,
                    &arguments,
                ),
                super::super::canonical_generic_method_symbol(
                    &logical_path,
                    &receiver_name,
                    &method.name.name,
                    declaration_index,
                    &arguments,
                ),
            )
        };
        let mut diagnostics = Diagnostics::new();
        let signature = super::super::types::resolve_method_signature(
            package_id,
            &receiver_source,
            &signature_source,
            self.universe,
            self.types,
            &mut diagnostics,
        )?;
        (signature.parameters.first().map(|parameter| parameter.ty) == Some(actual_receiver))
            .then_some(ResolvedMethodCandidate {
                package_id,
                logical_path,
                receiver_name,
                signature,
                symbol,
                span: method.span,
            })
    }

    /// Only a sole call can expand into several results. All other expressions
    /// occupy exactly one slot, preserving left-to-right evaluation order.
    pub(super) fn lower_values(
        &mut self,
        source: &[ast::Expr],
        expected: &[Option<TypeId>],
        span: Span,
        context: &str,
    ) -> Option<hir::ValueList> {
        if let [call @ ast::Expr::Call { .. }] = source {
            if !self.value_builtin(call) {
                let call = self.lower_call(call)?;
                if call.signature.returns.len() != expected.len() {
                    return self.fail(
                        span,
                        format!(
                            "{context} value count mismatch: expected {}, found {}",
                            expected.len(),
                            call.signature.returns.len()
                        ),
                    );
                }
                for (index, (actual, expected)) in
                    call.signature.returns.iter().zip(expected).enumerate()
                {
                    if let Some(expected) = expected
                        && expected != actual
                    {
                        let expected_name = self.types.display(*expected).to_string();
                        let actual_name = self.types.display(*actual).to_string();
                        return self.fail(
                            span,
                            format!(
                                "{context} value {} type mismatch: expected `{expected_name}`, found `{actual_name}`",
                                index + 1
                            ),
                        );
                    }
                }
                return Some(hir::ValueList::Call(call));
            }
        }
        if source.len() != expected.len() {
            return self.fail(
                span,
                format!(
                    "{context} value count mismatch: expected {}, found {}",
                    expected.len(),
                    source.len()
                ),
            );
        }
        let previous_context = self.value_context.replace(context.to_string());
        let values = source
            .iter()
            .zip(expected)
            .map(|(value, ty)| self.lower_value(value, *ty))
            .collect::<Option<Vec<_>>>();
        self.value_context = previous_context;
        let values = values?;
        Some(hir::ValueList::Expressions(values))
    }
}
