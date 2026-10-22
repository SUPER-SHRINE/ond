//! Discovery and lowering of requested generic function specializations.

use crate::diagnostic::{Diagnostic, Diagnostics, Label};
use crate::ir::{ast, hir};
use crate::source::Span;
use std::collections::{BTreeMap, BTreeSet};

use super::body::lower_typed_body_with_substitutions;
use super::types::{
    self, GenericSpecialization, TypePackage, generic_method_receiver, resolve_method_signature,
    resolve_signature,
};

pub(super) fn instantiate_generic_functions(
    universe: &[TypePackage],
    types: &mut crate::semantic::TypeTable,
    packages: &mut [hir::Package],
    sources: &crate::source::SourceDb,
    diagnostics: &mut Diagnostics,
) {
    let mut generated = packages
        .iter()
        .flat_map(|package| package.items.iter())
        .filter_map(|item| match item {
            hir::Item::Func(function) => Some(function.canonical_name.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mut failed = BTreeSet::new();
    let mut invalid_requests = Vec::new();
    let mut failed_diagnostics = Vec::<(String, String, Diagnostic)>::new();
    loop {
        let requests = super::generic_paths::requests(universe)
            .into_iter()
            .filter(|request| !invalid_requests.contains(request))
            .collect::<Vec<_>>();
        if let Some((request, diagnostic)) = super::generic_paths::validate(&requests, universe) {
            invalid_requests.push(request);
            diagnostics.push(diagnostic);
            continue;
        }
        let Some(symbol) = requests
            .iter()
            .map(|request| &request.symbol)
            .filter(|symbol| !failed.contains(*symbol) && !generated.contains(*symbol))
            .min()
            .cloned()
        else {
            break;
        };
        let specialization = universe.iter().find_map(|package| {
            package
                .specializations
                .borrow()
                .get(&symbol)
                .cloned()
                .map(|request| (package, request))
        });
        if let Some((
            source_package,
            GenericSpecialization::Method {
                declaration: declaration_index,
                arguments,
            },
        )) = specialization.clone()
        {
            let Some(declaration) = source_package.methods.get(declaration_index) else {
                defer_generic_diagnostic(
                    &mut failed_diagnostics,
                    symbol.clone(),
                    Diagnostic::error(
                        Span::synthetic(),
                        "generic method declaration is unavailable",
                    ),
                    "generic method",
                );
                failed.insert(symbol.clone());
                continue;
            };
            let Some(receiver) = declaration.receiver.as_ref() else {
                defer_generic_diagnostic(
                    &mut failed_diagnostics,
                    symbol.clone(),
                    Diagnostic::error(declaration.span, "invalid generic method specialization"),
                    "generic method",
                );
                failed.insert(symbol.clone());
                continue;
            };
            let Some(generic) = generic_method_receiver(receiver) else {
                defer_generic_diagnostic(
                    &mut failed_diagnostics,
                    symbol.clone(),
                    Diagnostic::error(receiver.span, "invalid generic method specialization"),
                    "generic method",
                );
                failed.insert(symbol.clone());
                continue;
            };
            if generic.parameters.len() != arguments.len()
                || arguments
                    .iter()
                    .any(|argument| types.get(*argument).is_none())
            {
                defer_generic_diagnostic(
                    &mut failed_diagnostics,
                    symbol.clone(),
                    Diagnostic::error(declaration.span, "invalid generic method specialization"),
                    "generic method",
                );
                failed.insert(symbol.clone());
                continue;
            }
            let substitutions = generic
                .parameters
                .iter()
                .map(|parameter| parameter.name.as_str())
                .zip(arguments.iter().copied())
                .collect();
            let receiver_source = ast::Field {
                name: receiver.name.clone(),
                ty: types::substitute_type(&receiver.ty, &substitutions),
                span: receiver.span,
            };
            let signature_source = types::substitute_signature(
                &declaration.signature,
                &generic.parameters,
                &arguments,
            );
            let mut specialization_diagnostics = Diagnostics::new();
            let Some(signature) = resolve_method_signature(
                source_package.id,
                &receiver_source,
                &signature_source,
                universe,
                types,
                &mut specialization_diagnostics,
            ) else {
                defer_generic_diagnostics(
                    &mut failed_diagnostics,
                    symbol.clone(),
                    specialization_diagnostics,
                    "generic method",
                );
                failed.insert(symbol.clone());
                continue;
            };
            let substitutions = generic
                .parameters
                .iter()
                .map(|parameter| parameter.name.clone())
                .zip(arguments.iter().copied())
                .collect();
            let mut specialization_diagnostics = Diagnostics::new();
            let Some((body, statics)) = lower_typed_body_with_substitutions(
                source_package.id,
                &signature,
                &declaration.body,
                universe,
                types,
                sources,
                &mut specialization_diagnostics,
                substitutions,
                Some(&symbol),
            ) else {
                defer_generic_diagnostics(
                    &mut failed_diagnostics,
                    symbol.clone(),
                    specialization_diagnostics,
                    "generic method",
                );
                failed.insert(symbol.clone());
                continue;
            };
            let target = packages
                .iter_mut()
                .find(|package| package.id == source_package.id)
                .expect("generic method owner package must have HIR output");
            extend_unique_statics(&mut target.statics, statics);
            target.items.push(hir::Item::Func(hir::FuncItem {
                name: declaration.name.name.clone(),
                receiver: signature.parameters.first().map(|parameter| parameter.ty),
                signature,
                body,
                span: declaration.span,
                canonical_name: symbol.clone(),
            }));
            generated.insert(symbol);
            continue;
        }
        if let Some((
            source_package,
            GenericSpecialization::Operator {
                declaration: declaration_index,
                arguments,
            },
        )) = specialization.clone()
        {
            let Some(declaration) = source_package.operators.get(declaration_index) else {
                defer_generic_diagnostic(
                    &mut failed_diagnostics,
                    symbol.clone(),
                    Diagnostic::error(
                        Span::synthetic(),
                        "generic operator declaration is unavailable",
                    ),
                    "generic operator",
                );
                failed.insert(symbol.clone());
                continue;
            };
            if declaration.type_params.len() != arguments.len()
                || arguments
                    .iter()
                    .any(|argument| types.get(*argument).is_none())
            {
                defer_generic_diagnostic(
                    &mut failed_diagnostics,
                    symbol.clone(),
                    Diagnostic::error(declaration.span, "invalid generic operator specialization"),
                    "generic operator",
                );
                failed.insert(symbol.clone());
                continue;
            }
            let source_signature = types::substitute_signature(
                &declaration.signature,
                &declaration.type_params,
                &arguments,
            );
            let mut specialization_diagnostics = Diagnostics::new();
            let Some(signature) = resolve_signature(
                source_package.id,
                &source_signature,
                universe,
                types,
                &mut specialization_diagnostics,
            ) else {
                defer_generic_diagnostics(
                    &mut failed_diagnostics,
                    symbol.clone(),
                    specialization_diagnostics,
                    "generic operator",
                );
                failed.insert(symbol.clone());
                continue;
            };
            if declaration.name == ast::OperatorName::Len
                && signature.returns != [crate::semantic::TypeId::U32]
            {
                defer_generic_diagnostic(
                    &mut failed_diagnostics,
                    symbol.clone(),
                    Diagnostic::error(declaration.signature.span, "operator len must return u32"),
                    "generic operator",
                );
                failed.insert(symbol.clone());
                continue;
            }
            let substitutions = declaration
                .type_params
                .iter()
                .map(|parameter| parameter.name.clone())
                .zip(arguments.iter().copied())
                .collect();
            let mut specialization_diagnostics = Diagnostics::new();
            let Some((body, statics)) = lower_typed_body_with_substitutions(
                source_package.id,
                &signature,
                &declaration.body,
                universe,
                types,
                sources,
                &mut specialization_diagnostics,
                substitutions,
                Some(&symbol),
            ) else {
                defer_generic_diagnostics(
                    &mut failed_diagnostics,
                    symbol.clone(),
                    specialization_diagnostics,
                    "generic operator",
                );
                failed.insert(symbol.clone());
                continue;
            };
            let target = packages
                .iter_mut()
                .find(|package| package.id == source_package.id)
                .expect("generic operator owner package must have HIR output");
            extend_unique_statics(&mut target.statics, statics);
            target.items.push(hir::Item::Func(hir::FuncItem {
                name: symbol.clone(),
                receiver: None,
                signature,
                body,
                span: declaration.span,
                canonical_name: symbol.clone(),
            }));
            generated.insert(symbol);
            continue;
        }
        let Some((source_package, GenericSpecialization::Function { name, arguments })) =
            specialization
        else {
            generated.insert(symbol);
            continue;
        };
        let Some(declaration) = source_package.generic_functions.get(&name) else {
            defer_generic_diagnostic(
                &mut failed_diagnostics,
                symbol.clone(),
                Diagnostic::error(
                    Span::synthetic(),
                    format!("generic function `{name}` is unavailable"),
                ),
                "generic function",
            );
            failed.insert(symbol.clone());
            continue;
        };
        if declaration.type_params.len() != arguments.len()
            || arguments
                .iter()
                .any(|argument| types.get(*argument).is_none())
        {
            defer_generic_diagnostic(
                &mut failed_diagnostics,
                symbol.clone(),
                Diagnostic::error(declaration.span, "invalid generic function specialization"),
                "generic function",
            );
            failed.insert(symbol.clone());
            continue;
        }
        let source_signature = types::substitute_signature(
            &declaration.signature,
            &declaration.type_params,
            &arguments,
        );
        let mut specialization_diagnostics = Diagnostics::new();
        let Some(signature) = resolve_signature(
            source_package.id,
            &source_signature,
            universe,
            types,
            &mut specialization_diagnostics,
        ) else {
            defer_generic_diagnostics(
                &mut failed_diagnostics,
                symbol.clone(),
                specialization_diagnostics,
                "generic function",
            );
            failed.insert(symbol.clone());
            continue;
        };
        let substitutions = declaration
            .type_params
            .iter()
            .map(|parameter| parameter.name.clone())
            .zip(arguments.iter().copied())
            .collect();
        let mut specialization_diagnostics = Diagnostics::new();
        let Some((body, statics)) = lower_typed_body_with_substitutions(
            source_package.id,
            &signature,
            &declaration.body,
            universe,
            types,
            sources,
            &mut specialization_diagnostics,
            substitutions,
            Some(&symbol),
        ) else {
            defer_generic_diagnostics(
                &mut failed_diagnostics,
                symbol.clone(),
                specialization_diagnostics,
                "generic function",
            );
            failed.insert(symbol.clone());
            continue;
        };
        let target = packages
            .iter_mut()
            .find(|package| package.id == source_package.id)
            .expect("generic owner package must have HIR output");
        extend_unique_statics(&mut target.statics, statics);
        target.items.push(hir::Item::Func(hir::FuncItem {
            name: symbol.clone(),
            receiver: None,
            signature,
            body,
            span: declaration.span,
            canonical_name: symbol.clone(),
        }));
        generated.insert(symbol);
    }
    if failed_diagnostics.is_empty() {
        return;
    }
    let uses = collect_generic_uses(universe);
    for (symbol, description, diagnostic) in failed_diagnostics {
        report_generic_diagnostic(diagnostics, diagnostic, &symbol, &uses, &description);
    }
}

fn extend_unique_statics(target: &mut Vec<hir::Static>, statics: Vec<hir::Static>) {
    for item in statics {
        if !target.iter().any(|existing| existing.symbol == item.symbol) {
            target.push(item);
        }
    }
}

struct GenericUse {
    caller: Option<String>,
    span: Span,
}

fn collect_generic_uses(universe: &[TypePackage]) -> BTreeMap<String, Vec<GenericUse>> {
    let mut uses = BTreeMap::<String, Vec<GenericUse>>::new();
    for request in super::generic_paths::requests(universe) {
        uses.entry(request.symbol).or_default().push(GenericUse {
            caller: request.caller,
            span: request.span,
        });
    }
    uses
}

fn defer_generic_diagnostic(
    failures: &mut Vec<(String, String, Diagnostic)>,
    symbol: String,
    diagnostic: Diagnostic,
    description: &str,
) {
    failures.push((symbol, description.to_owned(), diagnostic));
}

fn defer_generic_diagnostics(
    failures: &mut Vec<(String, String, Diagnostic)>,
    symbol: String,
    diagnostics: Diagnostics,
    description: &str,
) {
    for diagnostic in diagnostics {
        defer_generic_diagnostic(failures, symbol.clone(), diagnostic, description);
    }
}

fn report_generic_diagnostic(
    output: &mut Diagnostics,
    mut diagnostic: Diagnostic,
    symbol: &str,
    uses: &BTreeMap<String, Vec<GenericUse>>,
    description: &str,
) {
    // Related labels are a set of requesting sites, so visit each caller once
    // rather than enumerating paths through recursive generic instances.
    let mut pending = vec![symbol];
    let mut visited = BTreeSet::new();
    let mut roots = Vec::new();
    let mut nested = Vec::new();
    while let Some(symbol) = pending.pop() {
        if !visited.insert(symbol) {
            continue;
        }
        for site in uses.get(symbol).into_iter().flatten() {
            if let Some(caller) = &site.caller {
                nested.push(site.span);
                pending.push(caller);
            } else {
                roots.push(site.span);
            }
        }
    }
    let mut seen = Vec::new();
    if diagnostic.primary.span.is_synthetic()
        && let Some(primary) = roots.first().or_else(|| nested.first()).copied()
    {
        diagnostic.primary = Label {
            span: primary,
            message: diagnostic.message.clone(),
        };
        seen.push(primary);
    }
    let previous_related = std::mem::take(&mut diagnostic.related);
    for (spans, message) in [
        (roots, format!("{description} instantiated here")),
        (
            nested,
            format!("nested {description} instantiation requested here"),
        ),
    ] {
        for span in spans {
            if span == diagnostic.primary.span || seen.contains(&span) {
                continue;
            }
            seen.push(span);
            diagnostic.related.push(Label {
                span,
                message: message.clone(),
            });
        }
    }
    diagnostic.related.extend(previous_related);
    output.push(diagnostic);
}
