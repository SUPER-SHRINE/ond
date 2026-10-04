//! Discovery and lowering of requested generic function specializations.

use crate::diagnostic::{Diagnostic, Diagnostics, Label};
use crate::ir::{ast, hir};
use crate::source::Span;
use std::collections::{BTreeMap, BTreeSet};

use super::body::lower_typed_body_with_substitutions;
use super::types::{
    self, TypePackage, generic_method_receiver, resolve_method_signature, resolve_signature,
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
    let mut instantiation_chains = BTreeMap::<String, Vec<Vec<Span>>>::new();
    loop {
        let mut requested = BTreeMap::<String, Vec<Vec<Span>>>::new();
        for package in packages.iter() {
            for item in &package.statics {
                collect_generic_symbols_expr(&item.value, &[], &mut requested);
            }
            for item in &package.items {
                if let hir::Item::Func(function) = item {
                    let hir::Body::Typed(body) = &function.body;
                    let parents = instantiation_chains
                        .get(&function.canonical_name)
                        .map(Vec::as_slice)
                        .unwrap_or(&[]);
                    collect_generic_symbols_block(&body.block, parents, &mut requested);
                }
            }
        }
        requested.retain(|symbol, _| !generated.contains(symbol) && !failed.contains(symbol));
        let Some((symbol, chains)) = requested.into_iter().next() else {
            break;
        };
        instantiation_chains
            .entry(symbol.clone())
            .or_default()
            .extend(chains.iter().cloned());
        if let Some((package_path, receiver_name, method_name, declaration_index, arguments)) =
            parse_generic_method_symbol(&symbol)
        {
            let Some(source_package) = universe.iter().find(|package| {
                package.logical_path == package_path
                    || package_path == "main" && package.logical_path == "."
            }) else {
                report_generic_diagnostic(
                    diagnostics,
                    Diagnostic::error(
                        Span::synthetic(),
                        format!("generic method package `{package_path}` is unavailable"),
                    ),
                    &chains,
                    "generic method",
                );
                failed.insert(symbol.clone());
                continue;
            };
            let Some(declaration) = source_package.methods.get(declaration_index) else {
                report_generic_diagnostic(
                    diagnostics,
                    Diagnostic::error(
                        Span::synthetic(),
                        "generic method declaration is unavailable",
                    ),
                    &chains,
                    "generic method",
                );
                failed.insert(symbol.clone());
                continue;
            };
            let Some(receiver) = declaration.receiver.as_ref() else {
                report_generic_diagnostic(
                    diagnostics,
                    Diagnostic::error(declaration.span, "invalid generic method specialization"),
                    &chains,
                    "generic method",
                );
                failed.insert(symbol.clone());
                continue;
            };
            let Some(generic) = generic_method_receiver(receiver) else {
                report_generic_diagnostic(
                    diagnostics,
                    Diagnostic::error(receiver.span, "invalid generic method specialization"),
                    &chains,
                    "generic method",
                );
                failed.insert(symbol.clone());
                continue;
            };
            if generic.name != receiver_name
                || declaration.name.name != method_name
                || generic.parameters.len() != arguments.len()
                || arguments
                    .iter()
                    .any(|argument| types.get(*argument).is_none())
            {
                report_generic_diagnostic(
                    diagnostics,
                    Diagnostic::error(declaration.span, "invalid generic method specialization"),
                    &chains,
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
                report_generic_diagnostics(
                    diagnostics,
                    specialization_diagnostics,
                    &chains,
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
            ) else {
                report_generic_diagnostics(
                    diagnostics,
                    specialization_diagnostics,
                    &chains,
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
        if let Some((package_path, operator, declaration_index, arguments)) =
            parse_generic_operator_symbol(&symbol)
        {
            let Some(source_package) = universe.iter().find(|package| {
                package.logical_path == package_path
                    || package_path == "main" && package.logical_path == "."
            }) else {
                report_generic_diagnostic(
                    diagnostics,
                    Diagnostic::error(
                        Span::synthetic(),
                        format!("generic operator package `{package_path}` is unavailable"),
                    ),
                    &chains,
                    "generic operator",
                );
                failed.insert(symbol.clone());
                continue;
            };
            let Some(declaration) = source_package.operators.get(declaration_index) else {
                report_generic_diagnostic(
                    diagnostics,
                    Diagnostic::error(
                        Span::synthetic(),
                        "generic operator declaration is unavailable",
                    ),
                    &chains,
                    "generic operator",
                );
                failed.insert(symbol.clone());
                continue;
            };
            if declaration.name != operator
                || declaration.type_params.len() != arguments.len()
                || arguments
                    .iter()
                    .any(|argument| types.get(*argument).is_none())
            {
                report_generic_diagnostic(
                    diagnostics,
                    Diagnostic::error(declaration.span, "invalid generic operator specialization"),
                    &chains,
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
                report_generic_diagnostics(
                    diagnostics,
                    specialization_diagnostics,
                    &chains,
                    "generic operator",
                );
                failed.insert(symbol.clone());
                continue;
            };
            if declaration.name == ast::OperatorName::Len
                && signature.returns != [crate::semantic::TypeId::U32]
            {
                report_generic_diagnostic(
                    diagnostics,
                    Diagnostic::error(declaration.signature.span, "operator len must return u32"),
                    &chains,
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
            ) else {
                report_generic_diagnostics(
                    diagnostics,
                    specialization_diagnostics,
                    &chains,
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
        let Some((package_path, name, arguments)) = parse_generic_symbol(&symbol) else {
            generated.insert(symbol);
            continue;
        };
        let Some(source_package) = universe.iter().find(|package| {
            package.logical_path == package_path
                || package_path == "main" && package.logical_path == "."
        }) else {
            report_generic_diagnostic(
                diagnostics,
                Diagnostic::error(
                    Span::synthetic(),
                    format!("generic function package `{package_path}` is unavailable"),
                ),
                &chains,
                "generic function",
            );
            failed.insert(symbol.clone());
            continue;
        };
        let Some(declaration) = source_package.generic_functions.get(name) else {
            report_generic_diagnostic(
                diagnostics,
                Diagnostic::error(
                    Span::synthetic(),
                    format!("generic function `{name}` is unavailable"),
                ),
                &chains,
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
            report_generic_diagnostic(
                diagnostics,
                Diagnostic::error(declaration.span, "invalid generic function specialization"),
                &chains,
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
            report_generic_diagnostics(
                diagnostics,
                specialization_diagnostics,
                &chains,
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
        ) else {
            report_generic_diagnostics(
                diagnostics,
                specialization_diagnostics,
                &chains,
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
}

fn extend_unique_statics(target: &mut Vec<hir::Static>, statics: Vec<hir::Static>) {
    for item in statics {
        if !target.iter().any(|existing| existing.symbol == item.symbol) {
            target.push(item);
        }
    }
}

fn report_generic_diagnostics(
    output: &mut Diagnostics,
    diagnostics: Diagnostics,
    chains: &[Vec<Span>],
    description: &str,
) {
    for diagnostic in diagnostics {
        report_generic_diagnostic(output, diagnostic, chains, description);
    }
}

fn report_generic_diagnostic(
    output: &mut Diagnostics,
    mut diagnostic: Diagnostic,
    chains: &[Vec<Span>],
    description: &str,
) {
    if chains.is_empty() {
        output.push(diagnostic);
        return;
    }
    let mut seen = Vec::new();
    if diagnostic.primary.span.is_synthetic()
        && let Some(primary) = chains.iter().find_map(|chain| chain.first()).copied()
    {
        diagnostic.primary = Label {
            span: primary,
            message: diagnostic.message.clone(),
        };
        seen.push(primary);
    }
    let previous_related = std::mem::take(&mut diagnostic.related);
    for chain in chains {
        for (index, span) in chain.iter().enumerate() {
            if *span == diagnostic.primary.span || seen.contains(span) {
                continue;
            }
            seen.push(*span);
            diagnostic.related.push(Label {
                span: *span,
                message: if index == 0 {
                    format!("{description} instantiated here")
                } else {
                    format!("nested {description} instantiation requested here")
                },
            });
        }
    }
    diagnostic.related.extend(previous_related);
    output.push(diagnostic);
}

fn parse_generic_symbol(symbol: &str) -> Option<(&str, &str, Vec<crate::semantic::TypeId>)> {
    let (package, tail) = symbol.split_once(".\u{1f}generic.")?;
    let (name, arguments) = tail.split_once('.')?;
    let arguments = arguments
        .split('.')
        .map(|argument| argument.parse::<u32>().ok().map(crate::semantic::TypeId))
        .collect::<Option<Vec<_>>>()?;
    Some((package, name, arguments))
}

fn parse_generic_method_symbol(
    symbol: &str,
) -> Option<(&str, &str, &str, usize, Vec<crate::semantic::TypeId>)> {
    let (package, tail) = symbol.split_once(".\u{1f}generic_method.")?;
    let mut parts = tail.split('.');
    let receiver = parts.next()?;
    let method = parts.next()?;
    let declaration = parts.next()?.parse().ok()?;
    let arguments = parts
        .next()?
        .split('_')
        .map(|argument| argument.parse::<u32>().ok().map(crate::semantic::TypeId))
        .collect::<Option<Vec<_>>>()?;
    Some((package, receiver, method, declaration, arguments))
}

fn parse_generic_operator_symbol(
    symbol: &str,
) -> Option<(&str, ast::OperatorName, usize, Vec<crate::semantic::TypeId>)> {
    let (package, tail) = symbol.split_once(".\u{1f}generic_operator.")?;
    let mut parts = tail.split('.');
    let operator = match parts.next()? {
        "add" => ast::OperatorName::Add,
        "sub" => ast::OperatorName::Sub,
        "mul" => ast::OperatorName::Mul,
        "div" => ast::OperatorName::Div,
        "rem" => ast::OperatorName::Rem,
        "index" => ast::OperatorName::Index,
        "index_set" => ast::OperatorName::IndexSet,
        "len" => ast::OperatorName::Len,
        _ => return None,
    };
    let declaration = parts.next()?.parse().ok()?;
    let arguments = parts
        .next()?
        .split('_')
        .map(|argument| argument.parse::<u32>().ok().map(crate::semantic::TypeId))
        .collect::<Option<Vec<_>>>()?;
    Some((package, operator, declaration, arguments))
}

fn collect_generic_symbols_block(
    block: &hir::Block,
    parents: &[Vec<Span>],
    output: &mut BTreeMap<String, Vec<Vec<Span>>>,
) {
    for statement in &block.statements {
        match statement {
            hir::Stmt::LocalDecl { values, .. } | hir::Stmt::Return { values, .. } => {
                collect_generic_symbols_values(values, parents, output)
            }
            hir::Stmt::Assign {
                targets,
                values,
                overloaded,
                ..
            } => {
                for target in targets.iter().flatten() {
                    collect_generic_symbols_expr(target, parents, output);
                }
                collect_generic_symbols_values(values, parents, output);
                if let Some(call) = overloaded {
                    collect_generic_symbols_call(call, parents, output);
                }
            }
            hir::Stmt::IndexAssign(statement) => {
                collect_generic_symbols_expr(&statement.base, parents, output);
                collect_generic_symbols_expr(&statement.index, parents, output);
                collect_generic_symbols_call(&statement.getter, parents, output);
                collect_generic_symbols_values(&statement.values, parents, output);
                if let Some(call) = &statement.overloaded {
                    collect_generic_symbols_call(call, parents, output);
                }
                collect_generic_symbols_call(&statement.setter, parents, output);
            }
            hir::Stmt::Expr(expr) => collect_generic_symbols_expr(expr, parents, output),
            hir::Stmt::Call(call) => collect_generic_symbols_call(call, parents, output),
            hir::Stmt::Defer { block, .. } | hir::Stmt::Block(block) => {
                collect_generic_symbols_block(block, parents, output)
            }
            hir::Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                collect_generic_symbols_expr(condition, parents, output);
                collect_generic_symbols_block(then_block, parents, output);
                collect_generic_symbols_block(else_block, parents, output);
            }
            hir::Stmt::For {
                condition,
                body,
                post,
                ..
            } => {
                if let Some(condition) = condition {
                    collect_generic_symbols_expr(condition, parents, output);
                }
                collect_generic_symbols_block(body, parents, output);
                collect_generic_symbols_block(post, parents, output);
            }
            hir::Stmt::Break(_) | hir::Stmt::Continue(_) | hir::Stmt::Trap { .. } => {}
        }
    }
}

fn collect_generic_symbols_values(
    values: &hir::ValueList,
    parents: &[Vec<Span>],
    output: &mut BTreeMap<String, Vec<Vec<Span>>>,
) {
    match values {
        hir::ValueList::Expressions(values) => {
            for value in values {
                collect_generic_symbols_expr(value, parents, output);
            }
        }
        hir::ValueList::Call(call) => collect_generic_symbols_call(call, parents, output),
    }
}

fn collect_generic_symbols_call(
    call: &hir::Call,
    parents: &[Vec<Span>],
    output: &mut BTreeMap<String, Vec<Vec<Span>>>,
) {
    collect_generic_symbols_expr(&call.callee, parents, output);
    for argument in &call.arguments {
        collect_generic_symbols_expr(argument, parents, output);
    }
}

fn collect_generic_symbols_expr(
    expr: &hir::Expr,
    parents: &[Vec<Span>],
    output: &mut BTreeMap<String, Vec<Vec<Span>>>,
) {
    match &expr.kind {
        hir::ExprKind::Function(symbol) => {
            if symbol.contains(".\u{1f}generic.")
                || symbol.contains(".\u{1f}generic_operator.")
                || symbol.contains(".\u{1f}generic_method.")
            {
                let chains = output.entry(symbol.clone()).or_default();
                if parents.is_empty() {
                    let chain = vec![expr.span];
                    if !chains.contains(&chain) {
                        chains.push(chain);
                    }
                } else {
                    for parent in parents {
                        let mut chain = parent.clone();
                        chain.push(expr.span);
                        if !chains.contains(&chain) {
                            chains.push(chain);
                        }
                    }
                }
            }
        }
        hir::ExprKind::Dereference(value)
        | hir::ExprKind::AddressOf(value)
        | hir::ExprKind::InterfaceMethod {
            receiver: value, ..
        }
        | hir::ExprKind::EvaluatedLength { value, .. }
        | hir::ExprKind::Unary { operand: value, .. }
        | hir::ExprKind::Cast { value } => collect_generic_symbols_expr(value, parents, output),
        hir::ExprKind::Field { base, .. } => collect_generic_symbols_expr(base, parents, output),
        hir::ExprKind::Index { base, index }
        | hir::ExprKind::InterfaceEqual {
            lhs: base,
            rhs: index,
            ..
        }
        | hir::ExprKind::Binary {
            lhs: base,
            rhs: index,
            ..
        } => {
            collect_generic_symbols_expr(base, parents, output);
            collect_generic_symbols_expr(index, parents, output);
        }
        hir::ExprKind::Composite(values) => {
            for (_, value) in values {
                collect_generic_symbols_expr(value, parents, output);
            }
        }
        hir::ExprKind::Call(call) => collect_generic_symbols_call(call, parents, output),
        hir::ExprKind::Zero
        | hir::ExprKind::Integer(_)
        | hir::ExprKind::Float32(_)
        | hir::ExprKind::Bool(_)
        | hir::ExprKind::Nil
        | hir::ExprKind::Local(_)
        | hir::ExprKind::Global(_)
        | hir::ExprKind::Intrinsic(_) => {}
    }
}
