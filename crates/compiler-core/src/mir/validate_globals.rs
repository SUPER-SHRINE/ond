//! Project-level static symbol and initialization metadata checks.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn validate(project: &Project) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let mut static_errors = Vec::new();
    let functions = project
        .packages
        .iter()
        .flat_map(|package| package.functions.iter().chain(package.initializer.iter()))
        .map(|function| {
            let signature = FunctionType {
                parameters: function
                    .parameters
                    .iter()
                    .filter_map(|local| function.locals.get(local.0 as usize).map(|local| local.ty))
                    .collect(),
                returns: function.returns.clone(),
            };
            (function.name.as_str(), signature)
        })
        .collect::<BTreeMap<_, _>>();
    let mut error = |function: &str, message: String| {
        errors.push(ValidationError {
            function: function.into(),
            block: None,
            message,
        })
    };
    let mut globals = BTreeMap::new();
    let mut package_ids = BTreeSet::new();
    for package in &project.packages {
        if !package_ids.insert(package.id) {
            error("<startup>", "duplicate package id".into());
        }
        for global in &package.globals {
            if globals.insert(global.symbol.as_str(), global.ty).is_some() {
                error("<globals>", format!("duplicate global {}", global.symbol));
            }
            if project.types.get(global.ty).is_none() {
                error(
                    "<globals>",
                    format!("global {} has unknown type", global.symbol),
                );
            }
            if let Some(initializer) = &global.initializer {
                if initializer.ty != global.ty {
                    error(
                        "<globals>",
                        format!("global {} initializer type mismatch", global.symbol),
                    );
                }
                validate_static_functions(
                    &project.types,
                    &functions,
                    &global.symbol,
                    initializer,
                    &mut static_errors,
                );
            }
        }
        for item in &package.statics {
            if globals
                .insert(item.symbol.as_str(), item.value.ty)
                .is_some()
            {
                error("<globals>", format!("duplicate static {}", item.symbol));
            }
            if project.types.get(item.value.ty).is_none() {
                error(
                    "<globals>",
                    format!("static {} has unknown type", item.symbol),
                );
            }
            validate_static_functions(
                &project.types,
                &functions,
                &item.symbol,
                &item.value,
                &mut static_errors,
            );
            if item.null_terminated
                && !matches!(
                    project.types.underlying_kind(item.value.ty),
                    Some(TypeKind::Array {
                        element: TypeId::U8,
                        ..
                    })
                )
            {
                error(
                    "<globals>",
                    format!("null-terminated static {} is not a byte array", item.symbol),
                );
            }
        }
    }
    if !project.initialization_order.is_empty() {
        error(
            "<startup>",
            "automatic package initialization is not supported".into(),
        );
    }
    for package in &project.packages {
        for function in package.functions.iter().chain(package.initializer.iter()) {
            if globals.contains_key(function.name.as_str()) {
                error(&function.name, "function and static symbols collide".into());
            }
        }
        for dependency in &package.imports {
            if !package_ids.contains(dependency) {
                error(
                    "<startup>",
                    format!("unknown imported package {dependency:?}"),
                );
            }
        }
        if let Some(init) = &package.initializer {
            error(
                &init.name,
                "automatic package initializer is not supported".into(),
            );
        }
        for f in package.functions.iter().chain(package.initializer.iter()) {
            for i in f.blocks.iter().flat_map(|b| &b.instructions) {
                let places: Vec<&Place> = match &i.kind {
                    InstructionKind::Load { place, .. }
                    | InstructionKind::Store { place, .. }
                    | InstructionKind::AddressOf(place) => vec![place],
                    InstructionKind::AggregateCopy {
                        source,
                        destination,
                        ..
                    } => vec![source, destination],
                    _ => Vec::new(),
                };
                for place in places {
                    if let PlaceBase::Static { symbol, ty } = &place.base {
                        if globals.get(symbol.as_str()) != Some(ty) {
                            error(
                                &f.name,
                                format!("unknown global or mismatched static type: {symbol}"),
                            );
                        }
                    }
                }
            }
        }
    }
    errors.extend(static_errors);
    errors
}

fn validate_static_functions(
    types: &TypeTable,
    functions: &BTreeMap<&str, FunctionType>,
    owner: &str,
    value: &StaticValue,
    errors: &mut Vec<ValidationError>,
) {
    match &value.kind {
        StaticValueKind::Function { symbol, signature } => {
            if !matches!(types.underlying_kind(value.ty), Some(TypeKind::Function(ty)) if ty == signature)
            {
                errors.push(ValidationError {
                    function: owner.into(),
                    block: None,
                    message: format!("static function {symbol} has mismatched value type"),
                });
            }
            let Some(actual) = functions.get(symbol.as_str()) else {
                errors.push(ValidationError {
                    function: owner.into(),
                    block: None,
                    message: format!("static function references unknown symbol {symbol}"),
                });
                return;
            };
            let erased_receiver = signature
                .parameters
                .first()
                .zip(actual.parameters.first())
                .is_some_and(|(expected, actual)| {
                    matches!(
                        types.underlying_kind(*expected),
                        Some(TypeKind::Pointer(TypeId::U8))
                    ) && matches!(types.underlying_kind(*actual), Some(TypeKind::Pointer(_)))
                });
            let compatible = signature == actual
                || (erased_receiver
                    && signature.parameters.len() == actual.parameters.len()
                    && signature.parameters[1..] == actual.parameters[1..]
                    && signature.returns == actual.returns);
            if !compatible {
                errors.push(ValidationError {
                    function: owner.into(),
                    block: None,
                    message: format!("static function {symbol} has incompatible signature"),
                });
            }
        }
        StaticValueKind::Composite(elements) => {
            for (_, element) in elements {
                validate_static_functions(types, functions, owner, element, errors);
            }
        }
        _ => {}
    }
}
