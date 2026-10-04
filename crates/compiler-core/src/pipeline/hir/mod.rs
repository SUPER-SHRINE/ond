//! Package assembly and declaration validation.
use crate::diagnostic::{Diagnostic, Diagnostics, FrontendError};
use crate::ir::{ast, hir};
use crate::project::{LoadedProject, PackageId};
use crate::source::Span;
use std::collections::{BTreeMap, BTreeSet};

mod body;
mod body_names;
mod control_flow;
mod generic_functions;
mod operators;
mod types;
use body::{BodyLowerer, lower_typed_body};
use body_names::validate_generic_body_names;
use control_flow::{block_can_complete_normally, return_diagnostics};
use generic_functions::instantiate_generic_functions;
use operators::{
    canonical_generic_operator_symbol, canonical_operator_symbol, operator_type_owner,
    type_mentions_parameter,
};
use types::{
    TypePackage, collect_type_universe, generic_method_receiver, method_receiver_name,
    resolve_all_defined_types, resolve_defined_type, resolve_method_signature, resolve_signature,
};

pub(crate) fn lower_to_hir(
    loaded: &LoadedProject,
    ast: &ast::Project,
) -> Result<hir::Project, FrontendError> {
    let mut diagnostics = Diagnostics::new();
    let early_return_diagnostics = return_diagnostics(ast);
    let mut logical_to_package = BTreeMap::<String, PackageId>::new();
    for package in &loaded.packages {
        logical_to_package.insert(package.logical_path.clone(), package.id);
    }
    let mut types = crate::semantic::TypeTable::new();
    let mut symbols = Vec::new();
    for package in &ast.packages {
        let package_id = logical_to_package[&package.logical_path];
        for file in &package.files {
            for declaration in &file.decls {
                let (kind, names): (_, Vec<&ast::Ident>) = match declaration {
                    ast::TopLevelDecl::Const(decl) => (
                        hir::SymbolKind::Constant,
                        decl.specs
                            .iter()
                            .flat_map(|spec| spec.names.iter())
                            .collect(),
                    ),
                    ast::TopLevelDecl::Var(decl) => (
                        hir::SymbolKind::Global,
                        decl.specs
                            .iter()
                            .flat_map(|spec| spec.names.iter())
                            .collect(),
                    ),
                    ast::TopLevelDecl::Type(decl) => (
                        hir::SymbolKind::Type,
                        decl.specs.iter().map(|spec| &spec.name).collect(),
                    ),
                    ast::TopLevelDecl::Func(decl) => (
                        hir::SymbolKind::Function,
                        decl.receiver
                            .is_none()
                            .then_some(&decl.name)
                            .into_iter()
                            .collect(),
                    ),
                    ast::TopLevelDecl::Operator(_) => (hir::SymbolKind::Function, Vec::new()),
                };
                for name in names {
                    if name.name == "_" {
                        continue;
                    }
                    symbols.push(hir::Symbol {
                        id: hir::SymbolId(symbols.len() as u32),
                        package: package_id,
                        name: name.name.clone(),
                        canonical_name: canonical_symbol_name(&package.logical_path, &name.name),
                        kind,
                        exported: name
                            .name
                            .as_bytes()
                            .first()
                            .is_some_and(u8::is_ascii_uppercase),
                        span: name.span,
                    });
                }
            }
        }
    }
    let type_universe = collect_type_universe(loaded, ast, &logical_to_package, &mut types);
    resolve_all_defined_types(&type_universe, &mut types, &mut diagnostics);
    // Invalid definitions cannot be used by item/body lowering. Stop here instead
    // of resolving the same invalid declaration again and duplicating its error.
    if !diagnostics.is_empty() {
        diagnostics.extend(early_return_diagnostics);
        return Err(FrontendError::new(diagnostics.into_vec()));
    }
    diagnostics.extend(validate_generic_body_names(ast));

    let mut packages = Vec::new();
    for package in &ast.packages {
        let source_package = loaded
            .packages
            .iter()
            .find(|candidate| candidate.logical_path == package.logical_path)
            .expect("loaded and parsed packages must align");

        let expected_name = if package.logical_path == "." {
            "main".to_string()
        } else {
            package
                .logical_path
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .to_string()
        };

        let mut package_names = BTreeSet::new();
        let mut imports = Vec::new();
        let mut import_names_by_file = BTreeMap::new();
        let mut seen_top_level = BTreeMap::<String, Span>::new();
        let mut seen_methods = BTreeMap::<(String, String), Span>::new();
        let mut seen_operators = BTreeSet::new();
        let mut files = Vec::new();
        let mut items = Vec::new();
        let mut statics = Vec::new();

        for file in &package.files {
            let mut import_names = BTreeMap::<String, PackageId>::new();
            let mut aliases_by_path = BTreeMap::<String, String>::new();
            files.push(hir::File {
                file_id: file.file_id,
                package_name: file.package.name.clone(),
            });
            package_names.insert(file.package.name.clone());
            if file.package.name != expected_name {
                diagnostics.push(Diagnostic::error(
                    file.package.span,
                    format!(
                        "package `{}` must declare `package {expected_name}`",
                        package.logical_path
                    ),
                ));
            }
            if package.logical_path == "." {
                let path = loaded.sources.file(file.file_id).path();
                if path.file_name().is_some_and(|name| name == "main.ond") {
                    // found root main file
                }
            }

            for import in &file.imports {
                let logical = match crate::string_literal::import_path(&import.path) {
                    Ok(path) => path,
                    Err(message) => {
                        diagnostics.push(Diagnostic::error(import.span, message));
                        continue;
                    }
                };
                if logical == "." {
                    diagnostics.push(Diagnostic::error(
                        import.span,
                        "project root `main` package cannot be imported",
                    ));
                    continue;
                }
                if let Some(package_id) = logical_to_package.get(&logical).copied() {
                    let name = import
                        .alias
                        .as_ref()
                        .map(|alias| alias.name.as_str())
                        .unwrap_or_else(|| logical.rsplit('/').next().unwrap_or_default());
                    if name == "_" {
                        diagnostics.push(Diagnostic::error(
                            import.span,
                            "blank identifier cannot be used as an import alias",
                        ));
                        continue;
                    }
                    if let Some(previous) = aliases_by_path.insert(logical.clone(), name.into())
                        && previous != name
                    {
                        diagnostics.push(Diagnostic::error(
                            import.span,
                            format!(
                                "import path `{logical}` uses conflicting aliases `{previous}` and `{name}`"
                            ),
                        ));
                    }
                    if let Some(previous) = import_names.insert(name.into(), package_id)
                        && previous != package_id
                    {
                        diagnostics.push(Diagnostic::error(
                            import.span,
                            format!("ambiguous imported package `{name}`"),
                        ));
                    }
                    if !imports.contains(&package_id) {
                        imports.push(package_id);
                    }
                } else {
                    let mut diagnostic =
                        Diagnostic::error(import.span, format!("unknown import path `{logical}`"));
                    if let Some(ignored) = loaded.ignored_paths.iter().find(|ignored| {
                        logical.as_str() == ignored.as_str()
                            || logical
                                .strip_prefix(ignored.as_str())
                                .is_some_and(|suffix| suffix.starts_with('/'))
                    }) {
                        diagnostic = diagnostic.with_note(format!(
                            "`{ignored}` is excluded from source discovery by ond.toml resolve.ignore"
                        ));
                    }
                    diagnostics.push(diagnostic);
                }
            }

            for decl in &file.decls {
                match decl {
                    ast::TopLevelDecl::Const(decl) => {
                        for spec in &decl.specs {
                            for name in &spec.names {
                                report_duplicate_top_level(
                                    &mut diagnostics,
                                    &mut seen_top_level,
                                    &name.name,
                                    name.span,
                                );
                            }
                            let mut lowerer = BodyLowerer::new_with_sources(
                                source_package.id,
                                &type_universe,
                                &mut types,
                                &loaded.sources,
                            );
                            let values = lowerer.lower_constant_spec(spec);
                            let diagnostic = lowerer.failure.take();
                            drop(lowerer);
                            if let Some(diagnostic) = diagnostic {
                                diagnostics.push(diagnostic);
                            }
                            if let Some(values) = values {
                                for ((name, source), value) in spec
                                    .names
                                    .iter()
                                    .zip(&spec.values)
                                    .filter(|(name, _)| name.name != "_")
                                    .zip(&values)
                                {
                                    if matches!(
                                        types.underlying_kind(value.ty),
                                        Some(
                                            crate::semantic::TypeKind::Array { .. }
                                                | crate::semantic::TypeKind::Struct(_)
                                        )
                                    ) {
                                        statics.push(hir::Static {
                                            symbol: canonical_symbol_name(
                                                &package.logical_path,
                                                &name.name,
                                            ),
                                            value: value.clone(),
                                            exported: name
                                                .name
                                                .as_bytes()
                                                .first()
                                                .is_some_and(u8::is_ascii_uppercase),
                                            null_terminated: constant_is_string_literal(
                                                source_package.id,
                                                source,
                                                &type_universe,
                                                &mut BTreeSet::new(),
                                            ),
                                            span: name.span,
                                        });
                                    }
                                }
                                items.push(hir::Item::Const(hir::ConstItem {
                                    names: spec
                                        .names
                                        .iter()
                                        .filter(|n| n.name != "_")
                                        .map(|n| n.name.clone())
                                        .collect(),
                                    values,
                                    span: spec.span,
                                }));
                            }
                        }
                    }
                    ast::TopLevelDecl::Var(decl) => {
                        for spec in &decl.specs {
                            for name in &spec.names {
                                report_duplicate_top_level(
                                    &mut diagnostics,
                                    &mut seen_top_level,
                                    &name.name,
                                    name.span,
                                );
                            }
                            let mut lowerer = BodyLowerer::new_with_sources(
                                source_package.id,
                                &type_universe,
                                &mut types,
                                &loaded.sources,
                            );
                            let item = lowerer.lower_global(spec);
                            if lowerer.failure.is_none() && item.globals.len() != spec.names.len() {
                                lowerer.failure = Some(Diagnostic::error(
                                    spec.span,
                                    "global initializer lowering is not implemented",
                                ));
                            }
                            if let Some(diagnostic) = lowerer.failure.take() {
                                diagnostics.push(diagnostic);
                            }
                            for item in lowerer.take_statics() {
                                if !statics
                                    .iter()
                                    .any(|existing: &hir::Static| existing.symbol == item.symbol)
                                {
                                    statics.push(item);
                                }
                            }
                            items.push(hir::Item::Var(item));
                        }
                    }
                    ast::TopLevelDecl::Type(decl) => {
                        for spec in &decl.specs {
                            if spec.name.name == "_" {
                                diagnostics.push(Diagnostic::error(
                                    spec.span,
                                    "blank identifier cannot name a type",
                                ));
                                continue;
                            }
                            report_duplicate_top_level(
                                &mut diagnostics,
                                &mut seen_top_level,
                                &spec.name.name,
                                spec.name.span,
                            );
                            let declaration = type_universe
                                .iter()
                                .find(|package| package.id == source_package.id)
                                .and_then(|package| package.declarations.get(&spec.name.name))
                                .expect("collected type declaration must be available");
                            if let ast::Type::Interface { methods, .. } = &declaration.ty {
                                for method in methods {
                                    if !method
                                        .name
                                        .name
                                        .as_bytes()
                                        .first()
                                        .is_some_and(u8::is_ascii_uppercase)
                                    {
                                        diagnostics.push(Diagnostic::error(
                                            method.name.span,
                                            "interface methods must be exported",
                                        ));
                                    }
                                }
                            }
                            if !declaration.type_params.is_empty() {
                                continue;
                            }
                            if let Some(ty) = resolve_defined_type(
                                source_package.id,
                                &spec.name.name,
                                &type_universe,
                                &mut types,
                                false,
                                &mut BTreeSet::new(),
                                &mut diagnostics,
                            ) {
                                items.push(hir::Item::Type(hir::TypeItem {
                                    name: spec.name.name.clone(),
                                    ty,
                                    span: spec.span,
                                }));
                            }
                        }
                    }
                    ast::TopLevelDecl::Operator(decl) => {
                        if !decl.type_params.is_empty() {
                            let parameter_count = decl.signature.params.len();
                            let valid_arity = match decl.name {
                                ast::OperatorName::Add | ast::OperatorName::Sub => {
                                    matches!(parameter_count, 1 | 2)
                                }
                                ast::OperatorName::Mul
                                | ast::OperatorName::Div
                                | ast::OperatorName::Rem
                                | ast::OperatorName::Index => parameter_count == 2,
                                ast::OperatorName::IndexSet => parameter_count == 3,
                                ast::OperatorName::Len => parameter_count == 1,
                            };
                            if !valid_arity {
                                diagnostics.push(Diagnostic::error(
                                    decl.signature.span,
                                    "operator has an invalid parameter count",
                                ));
                                continue;
                            }
                            let valid_returns = if decl.name == ast::OperatorName::IndexSet {
                                decl.signature.results.is_empty()
                            } else {
                                decl.signature.results.len() == 1
                            };
                            if !valid_returns {
                                diagnostics.push(Diagnostic::error(
                                    decl.signature.span,
                                    "operator must return exactly one value (`[]=` returns none)",
                                ));
                                continue;
                            }
                            if let Some(parameter) = decl.type_params.iter().find(|parameter| {
                                !decl.signature.params.iter().any(|operand| {
                                    type_mentions_parameter(&operand.ty, &parameter.name)
                                })
                            }) {
                                diagnostics.push(Diagnostic::error(
                                    parameter.span,
                                    format!(
                                        "operator type parameter `{}` cannot be inferred from its operands",
                                        parameter.name
                                    ),
                                ));
                                continue;
                            }
                            let placeholders =
                                vec![crate::semantic::TypeId::U8; decl.type_params.len()];
                            let validation_source = types::substitute_signature(
                                &decl.signature,
                                &decl.type_params,
                                &placeholders,
                            );
                            let Some(validation_signature) = resolve_signature(
                                source_package.id,
                                &validation_source,
                                &type_universe,
                                &mut types,
                                &mut diagnostics,
                            ) else {
                                continue;
                            };
                            if decl.name == ast::OperatorName::Len
                                && validation_signature.returns != [crate::semantic::TypeId::U32]
                            {
                                diagnostics.push(Diagnostic::error(
                                    decl.signature.span,
                                    "operator len must return u32",
                                ));
                                continue;
                            }
                            let owner = validation_signature
                                .parameters
                                .iter()
                                .find_map(|parameter| operator_type_owner(&types, parameter.ty));
                            if owner.as_deref() != Some(package.logical_path.as_str()) {
                                diagnostics.push(Diagnostic::error(
                                    decl.signature.span,
                                    "generic operator must be declared by the package owning its leftmost user-defined operand",
                                ));
                            }
                            continue;
                        }
                        let Some(signature) = resolve_signature(
                            source_package.id,
                            &decl.signature,
                            &type_universe,
                            &mut types,
                            &mut diagnostics,
                        ) else {
                            continue;
                        };
                        let parameter_count = signature.parameters.len();
                        let valid_arity = match decl.name {
                            ast::OperatorName::Add | ast::OperatorName::Sub => {
                                matches!(parameter_count, 1 | 2)
                            }
                            ast::OperatorName::Mul
                            | ast::OperatorName::Div
                            | ast::OperatorName::Rem
                            | ast::OperatorName::Index => parameter_count == 2,
                            ast::OperatorName::IndexSet => parameter_count == 3,
                            ast::OperatorName::Len => parameter_count == 1,
                        };
                        if !valid_arity {
                            diagnostics.push(Diagnostic::error(
                                decl.signature.span,
                                "operator has an invalid parameter count",
                            ));
                            continue;
                        }
                        let valid_returns = if decl.name == ast::OperatorName::IndexSet {
                            signature.returns.is_empty()
                        } else {
                            signature.returns.len() == 1
                        };
                        if !valid_returns {
                            diagnostics.push(Diagnostic::error(
                                decl.signature.span,
                                "operator must return exactly one value (`[]=` returns none)",
                            ));
                            continue;
                        }
                        if decl.name == ast::OperatorName::Len
                            && signature.returns != [crate::semantic::TypeId::U32]
                        {
                            diagnostics.push(Diagnostic::error(
                                decl.signature.span,
                                "operator len must return u32",
                            ));
                            continue;
                        }
                        let parameter_types = signature
                            .parameters
                            .iter()
                            .map(|parameter| parameter.ty)
                            .collect::<Vec<_>>();
                        let owner = parameter_types
                            .iter()
                            .find_map(|ty| operator_type_owner(&types, *ty));
                        if owner.as_deref() != Some(package.logical_path.as_str()) {
                            diagnostics.push(Diagnostic::error(
                                decl.signature.span,
                                "operator must be declared by the package owning its leftmost user-defined operand",
                            ));
                            continue;
                        }
                        let key = (decl.name, parameter_types.clone());
                        if !seen_operators.insert(key) {
                            diagnostics.push(Diagnostic::error(
                                decl.span,
                                "duplicate operator declaration for the same operand types",
                            ));
                            continue;
                        }
                        let Some((body, function_statics)) = lower_typed_body(
                            source_package.id,
                            &signature,
                            &decl.body,
                            &type_universe,
                            &mut types,
                            &loaded.sources,
                            &mut diagnostics,
                        ) else {
                            continue;
                        };
                        statics.extend(function_statics);
                        let canonical_name = canonical_operator_symbol(
                            &package.logical_path,
                            decl.name,
                            &parameter_types,
                        );
                        items.push(hir::Item::Func(hir::FuncItem {
                            name: canonical_name.clone(),
                            receiver: None,
                            signature,
                            body,
                            span: decl.span,
                            canonical_name,
                        }));
                    }
                    ast::TopLevelDecl::Func(decl) => {
                        if let Some(receiver) = &decl.receiver {
                            if decl.name.name == "_" {
                                diagnostics.push(Diagnostic::error(
                                    decl.name.span,
                                    "blank identifier cannot name a method",
                                ));
                                continue;
                            }
                            if !decl.type_params.is_empty() {
                                diagnostics.push(Diagnostic::error(
                                    decl.name.span,
                                    "methods cannot declare their own type parameters",
                                ));
                                continue;
                            }
                            let Some((receiver_name, _)) = method_receiver_name(receiver) else {
                                diagnostics.push(Diagnostic::error(
                                    receiver.span,
                                    "method receiver must be a local named type or pointer to one",
                                ));
                                continue;
                            };
                            let Some(declaration) = type_universe
                                .iter()
                                .find(|candidate| candidate.id == source_package.id)
                                .and_then(|candidate| candidate.declarations.get(receiver_name))
                            else {
                                diagnostics.push(Diagnostic::error(
                                    receiver.span,
                                    "method receiver type must be declared in the same package",
                                ));
                                continue;
                            };
                            let generic_receiver = if declaration.type_params.is_empty() {
                                if generic_method_receiver(receiver).is_some() {
                                    diagnostics.push(Diagnostic::error(
                                        receiver.span,
                                        "non-generic method receiver cannot have type arguments",
                                    ));
                                    continue;
                                }
                                None
                            } else {
                                let Some(generic) = generic_method_receiver(receiver) else {
                                    diagnostics.push(Diagnostic::error(
                                        receiver.span,
                                        "generic method receiver must apply fresh type parameter names to a local generic type",
                                    ));
                                    continue;
                                };
                                if generic.name != receiver_name
                                    || generic.parameters.len() != declaration.type_params.len()
                                {
                                    diagnostics.push(Diagnostic::error(
                                        receiver.span,
                                        format!(
                                            "generic method receiver expects {} type parameters, found {}",
                                            declaration.type_params.len(),
                                            generic.parameters.len()
                                        ),
                                    ));
                                    continue;
                                }
                                let mut names = BTreeSet::new();
                                if let Some(parameter) =
                                    generic.parameters.iter().find(|parameter| {
                                        parameter.name == "_"
                                            || matches!(
                                                parameter.name.as_str(),
                                                "bool"
                                                    | "u8"
                                                    | "i8"
                                                    | "u16"
                                                    | "i16"
                                                    | "u32"
                                                    | "i32"
                                                    | "f32"
                                            )
                                            || type_universe
                                                .iter()
                                                .find(|candidate| candidate.id == source_package.id)
                                                .is_some_and(|package| {
                                                    package
                                                        .declarations
                                                        .contains_key(&parameter.name)
                                                })
                                            || !names.insert(parameter.name.clone())
                                    })
                                {
                                    diagnostics.push(Diagnostic::error(
                                        parameter.span,
                                        "generic method receiver type parameters must be fresh, distinct non-blank identifiers",
                                    ));
                                    continue;
                                }
                                Some(generic)
                            };
                            let validation_arguments = generic_receiver.as_ref().map(|generic| {
                                vec![crate::semantic::TypeId::U8; generic.parameters.len()]
                            });
                            let validation_receiver = validation_arguments.as_ref().map_or_else(
                                || (**receiver).clone(),
                                |arguments| {
                                    let substitutions = generic_receiver
                                        .as_ref()
                                        .expect("generic receiver is present")
                                        .parameters
                                        .iter()
                                        .map(|parameter| parameter.name.as_str())
                                        .zip(arguments.iter().copied())
                                        .collect();
                                    ast::Field {
                                        name: receiver.name.clone(),
                                        ty: types::substitute_type(&receiver.ty, &substitutions),
                                        span: receiver.span,
                                    }
                                },
                            );
                            let validation_signature_source =
                                validation_arguments.as_ref().map_or_else(
                                    || decl.signature.clone(),
                                    |arguments| {
                                        types::substitute_signature(
                                            &decl.signature,
                                            &generic_receiver
                                                .as_ref()
                                                .expect("generic receiver is present")
                                                .parameters,
                                            arguments,
                                        )
                                    },
                                );
                            let Some(validation_signature) = resolve_method_signature(
                                source_package.id,
                                &validation_receiver,
                                &validation_signature_source,
                                &type_universe,
                                &mut types,
                                &mut diagnostics,
                            ) else {
                                continue;
                            };
                            let receiver_type = validation_signature.parameters[0].ty;
                            let receiver_base_type = match types.kind(receiver_type) {
                                Some(crate::semantic::TypeKind::Pointer(pointee)) => *pointee,
                                _ => receiver_type,
                            };
                            if matches!(
                                types.underlying_kind(receiver_base_type),
                                Some(crate::semantic::TypeKind::Interface(_))
                            ) {
                                diagnostics.push(Diagnostic::error(
                                    receiver.span,
                                    "methods cannot be declared on interface types",
                                ));
                                continue;
                            }
                            let key = (receiver_name.to_string(), decl.name.name.clone());
                            if seen_methods.insert(key, decl.name.span).is_some() {
                                diagnostics.push(Diagnostic::error(
                                    decl.name.span,
                                    format!(
                                        "duplicate method `{receiver_name}.{}`",
                                        decl.name.name
                                    ),
                                ));
                                continue;
                            }
                            if let Some(crate::semantic::TypeKind::Struct(structure)) =
                                types.underlying_kind(receiver_base_type)
                                && structure
                                    .fields
                                    .iter()
                                    .any(|field| field.name == decl.name.name)
                            {
                                diagnostics.push(Diagnostic::error(
                                    decl.name.span,
                                    format!(
                                        "method `{}` conflicts with a field of `{receiver_name}`",
                                        decl.name.name
                                    ),
                                ));
                                continue;
                            }
                            if !decl.signature.results.is_empty()
                                && block_can_complete_normally(&decl.body)
                            {
                                diagnostics.push(Diagnostic::error(
                                    decl.body.span,
                                    format!("missing return in method `{}`", decl.name.name),
                                ));
                            }
                            if generic_receiver.is_some() {
                                continue;
                            }
                            if let Some(signature) = resolve_method_signature(
                                source_package.id,
                                receiver,
                                &decl.signature,
                                &type_universe,
                                &mut types,
                                &mut diagnostics,
                            ) {
                                let Some((body, function_statics)) = lower_typed_body(
                                    source_package.id,
                                    &signature,
                                    &decl.body,
                                    &type_universe,
                                    &mut types,
                                    &loaded.sources,
                                    &mut diagnostics,
                                ) else {
                                    continue;
                                };
                                for item in function_statics {
                                    if !statics.iter().any(|existing: &hir::Static| {
                                        existing.symbol == item.symbol
                                    }) {
                                        statics.push(item);
                                    }
                                }
                                items.push(hir::Item::Func(hir::FuncItem {
                                    name: decl.name.name.clone(),
                                    receiver: signature
                                        .parameters
                                        .first()
                                        .map(|parameter| parameter.ty),
                                    signature,
                                    body,
                                    span: decl.span,
                                    canonical_name: canonical_method_symbol(
                                        &package.logical_path,
                                        receiver_name,
                                        &decl.name.name,
                                    ),
                                }));
                            }
                            continue;
                        }
                        if decl.name.name == "_" {
                            diagnostics.push(Diagnostic::error(
                                decl.name.span,
                                "blank identifier cannot name a function",
                            ));
                            continue;
                        }
                        report_duplicate_top_level(
                            &mut diagnostics,
                            &mut seen_top_level,
                            &decl.name.name,
                            decl.name.span,
                        );
                        if !decl.type_params.is_empty() {
                            continue;
                        }
                        if !decl.signature.results.is_empty()
                            && block_can_complete_normally(&decl.body)
                        {
                            diagnostics.push(Diagnostic::error(
                                decl.body.span,
                                format!("missing return in function `{}`", decl.name.name),
                            ));
                        }
                        if let Some(signature) = resolve_signature(
                            source_package.id,
                            &decl.signature,
                            &type_universe,
                            &mut types,
                            &mut diagnostics,
                        ) {
                            let Some((body, function_statics)) = lower_typed_body(
                                source_package.id,
                                &signature,
                                &decl.body,
                                &type_universe,
                                &mut types,
                                &loaded.sources,
                                &mut diagnostics,
                            ) else {
                                continue;
                            };
                            for item in function_statics {
                                if !statics
                                    .iter()
                                    .any(|existing: &hir::Static| existing.symbol == item.symbol)
                                {
                                    statics.push(item);
                                }
                            }
                            let canonical_name =
                                canonical_symbol_name(&package.logical_path, &decl.name.name);
                            items.push(hir::Item::Func(hir::FuncItem {
                                name: decl.name.name.clone(),
                                receiver: None,
                                signature,
                                body,
                                span: decl.span,
                                canonical_name,
                            }));
                        }
                    }
                }
            }
            import_names_by_file.insert(file.file_id, import_names);
        }

        if package_names.len() > 1 {
            diagnostics.push(Diagnostic::error(
                package
                    .files
                    .first()
                    .map(|file| file.package.span)
                    .unwrap_or(Span::synthetic()),
                format!(
                    "all files in package `{}` must use the same package clause",
                    package.logical_path
                ),
            ));
        }
        for import_names in import_names_by_file.values() {
            for name in import_names.keys() {
                if seen_top_level.contains_key(name) {
                    diagnostics.push(Diagnostic::error(
                        seen_top_level[name],
                        format!("top-level declaration conflicts with imported package `{name}`"),
                    ));
                }
            }
        }

        packages.push(hir::Package {
            id: source_package.id,
            name: expected_name,
            logical_path: package.logical_path.clone(),
            imports: imports.into_iter().collect(),
            import_names: import_names_by_file,
            files,
            items,
            statics,
            initializer: None,
        });
    }

    instantiate_generic_functions(
        &type_universe,
        &mut types,
        &mut packages,
        &loaded.sources,
        &mut diagnostics,
    );

    // Semantic analysis consumes the loaded snapshot, including in-memory inputs.
    // Filesystem discovery/validation belongs to the project loader.
    let root_main = loaded
        .packages
        .iter()
        .filter(|p| p.logical_path == ".")
        .flat_map(|p| &p.files)
        .any(|id| loaded.sources.file(*id).path() == loaded.root.join("main.ond"));
    if !root_main {
        diagnostics.push(Diagnostic::error(
            Span::synthetic(),
            "project root must contain `main.ond`",
        ));
    }

    let main_package = packages.iter().find(|package| package.logical_path == ".");
    let main_functions = main_package
        .map(|package| {
            package
                .items
                .iter()
                .filter_map(|item| match item {
                    hir::Item::Func(function) if function.name == "main" => Some(function),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if main_functions.is_empty()
        && !symbols
            .iter()
            .any(|s| s.canonical_name == "main.main" && s.kind == hir::SymbolKind::Function)
    {
        diagnostics.push(Diagnostic::error(
            Span::synthetic(),
            "`main` package must define `func main()`",
        ));
    } else {
        for function in main_functions {
            if !function.signature.parameters.is_empty() || !function.signature.returns.is_empty() {
                diagnostics.push(Diagnostic::error(
                    function.span,
                    "`func main()` must not have parameters or return values",
                ));
            }
        }
    }

    if let Some(cycle) = find_import_cycle(&packages) {
        diagnostics.push(Diagnostic::error(
            Span::synthetic(),
            format!("import cycle detected: {}", cycle.join(" -> ")),
        ));
    }

    if diagnostics.is_empty() {
        let initialization_order = Vec::new();
        Ok(hir::Project {
            types,
            symbols,
            packages,
            entry_package: PackageId(0),
            initialization_order,
        })
    } else {
        Err(FrontendError::new(diagnostics.into_vec()))
    }
}

fn canonical_symbol_name(package_path: &str, identifier: &str) -> String {
    if package_path == "." {
        format!("main.{identifier}")
    } else {
        format!("{package_path}.{identifier}")
    }
}

pub(super) fn canonical_generic_symbol(
    package_path: &str,
    name: &str,
    arguments: &[crate::semantic::TypeId],
) -> String {
    let package = if package_path == "." {
        "main"
    } else {
        package_path
    };
    format!(
        "{package}.\u{1f}generic.{name}.{}",
        arguments
            .iter()
            .map(|argument| argument.0.to_string())
            .collect::<Vec<_>>()
            .join(".")
    )
}

pub(super) fn canonical_method_symbol(package_path: &str, receiver: &str, method: &str) -> String {
    let package = if package_path == "." {
        "main"
    } else {
        package_path
    };
    format!("{package}.\u{1f}method.{receiver}.{method}")
}

pub(super) fn canonical_generic_method_symbol(
    package_path: &str,
    receiver: &str,
    method: &str,
    declaration_index: usize,
    arguments: &[crate::semantic::TypeId],
) -> String {
    let package = if package_path == "." {
        "main"
    } else {
        package_path
    };
    let arguments = arguments
        .iter()
        .map(|argument| argument.0.to_string())
        .collect::<Vec<_>>()
        .join("_");
    format!("{package}.\u{1f}generic_method.{receiver}.{method}.{declaration_index}.{arguments}")
}

fn constant_is_string_literal(
    package_id: PackageId,
    source: &ast::Expr,
    universe: &[TypePackage],
    visiting: &mut BTreeSet<(PackageId, String)>,
) -> bool {
    if matches!(source, ast::Expr::Literal(ast::Literal::String(_, _))) {
        return true;
    }
    let ast::Expr::Name(path) = source else {
        return false;
    };
    let (owner, name) = match path.segments.as_slice() {
        [name] => (package_id, name.name.as_str()),
        [qualifier, name] => {
            let Some(owner) = universe
                .iter()
                .find(|package| package.id == package_id)
                .and_then(|package| package.imports.get(&path.span.file))
                .and_then(|imports| imports.get(&qualifier.name))
                .copied()
            else {
                return false;
            };
            (owner, name.name.as_str())
        }
        _ => return false,
    };
    let key = (owner, name.to_string());
    if !visiting.insert(key.clone()) {
        return false;
    }
    let result = universe
        .iter()
        .find(|package| package.id == owner)
        .and_then(|package| package.constants.get(name))
        .is_some_and(|(_, value)| constant_is_string_literal(owner, value, universe, visiting));
    visiting.remove(&key);
    result
}

fn report_duplicate_top_level(
    diagnostics: &mut Diagnostics,
    seen: &mut BTreeMap<String, Span>,
    name: &str,
    span: Span,
) {
    if name == "_" {
        return;
    }
    if let Some(previous) = seen.insert(name.to_string(), span) {
        diagnostics.push(
            Diagnostic::error(span, format!("duplicate top-level declaration `{name}`")).with_note(
                format!(
                    "previous declaration starts at byte {} in the same package",
                    previous.start
                ),
            ),
        );
    }
}

fn find_import_cycle(packages: &[hir::Package]) -> Option<Vec<String>> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Visit {
        Visiting,
        Done,
    }

    fn dfs(
        package_id: PackageId,
        packages: &[hir::Package],
        states: &mut BTreeMap<PackageId, Visit>,
        stack: &mut Vec<PackageId>,
    ) -> Option<Vec<String>> {
        states.insert(package_id, Visit::Visiting);
        stack.push(package_id);

        let package = packages.iter().find(|package| package.id == package_id)?;
        for import in &package.imports {
            match states.get(import).copied() {
                Some(Visit::Visiting) => {
                    let start = stack.iter().position(|id| id == import).unwrap_or(0);
                    let cycle = stack[start..]
                        .iter()
                        .chain(std::iter::once(import))
                        .filter_map(|id| {
                            packages
                                .iter()
                                .find(|package| package.id == *id)
                                .map(|package| package.logical_path.clone())
                        })
                        .collect::<Vec<_>>();
                    return Some(cycle);
                }
                Some(Visit::Done) => {}
                None => {
                    if let Some(cycle) = dfs(*import, packages, states, stack) {
                        return Some(cycle);
                    }
                }
            }
        }

        stack.pop();
        states.insert(package_id, Visit::Done);
        None
    }

    let mut states = BTreeMap::<PackageId, Visit>::new();
    let mut stack = Vec::new();
    for package in packages {
        if states.contains_key(&package.id) {
            continue;
        }
        if let Some(cycle) = dfs(package.id, packages, &mut states, &mut stack) {
            return Some(cycle);
        }
    }
    None
}
