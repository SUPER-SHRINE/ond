//! Operator symbol identities and declaration-level type helpers.

use crate::ir::ast;

pub(super) fn canonical_operator_symbol(
    package_path: &str,
    operator: ast::OperatorName,
    parameters: &[crate::semantic::TypeId],
) -> String {
    let package = if package_path == "." {
        "main"
    } else {
        package_path
    };
    let name = operator_symbol_name(operator);
    let parameters = parameters
        .iter()
        .map(|ty| ty.0.to_string())
        .collect::<Vec<_>>()
        .join("_");
    format!("{package}.\u{1f}operator.{name}.{parameters}")
}

pub(super) fn canonical_generic_operator_symbol(
    package_path: &str,
    operator: ast::OperatorName,
    declaration_index: usize,
    arguments: &[crate::semantic::TypeId],
) -> String {
    let package = if package_path == "." {
        "main"
    } else {
        package_path
    };
    let name = operator_symbol_name(operator);
    let arguments = arguments
        .iter()
        .map(|ty| ty.0.to_string())
        .collect::<Vec<_>>()
        .join("_");
    format!("{package}.\u{1f}generic_operator.{name}.{declaration_index}.{arguments}")
}

fn operator_symbol_name(operator: ast::OperatorName) -> &'static str {
    match operator {
        ast::OperatorName::Add => "add",
        ast::OperatorName::Sub => "sub",
        ast::OperatorName::Mul => "mul",
        ast::OperatorName::Div => "div",
        ast::OperatorName::Rem => "rem",
        ast::OperatorName::Index => "index",
        ast::OperatorName::IndexSet => "index_set",
        ast::OperatorName::Len => "len",
    }
}

pub(super) fn operator_type_owner(
    types: &crate::semantic::TypeTable,
    ty: crate::semantic::TypeId,
) -> Option<String> {
    if let Some(name) = types
        .get(ty)
        .and_then(|definition| definition.name.as_deref())
    {
        let owner = name.rsplit_once('.')?.0;
        return Some(if owner == "main" {
            ".".into()
        } else {
            owner.into()
        });
    }
    match types.kind(ty) {
        Some(crate::semantic::TypeKind::Pointer(pointee)) => operator_type_owner(types, *pointee),
        _ => None,
    }
}

pub(super) fn type_mentions_parameter(ty: &ast::Type, parameter: &str) -> bool {
    match ty {
        ast::Type::Named(path) => {
            matches!(path.segments.as_slice(), [name] if name.name == parameter)
        }
        ast::Type::Apply { arguments, .. } => arguments
            .iter()
            .any(|argument| type_mentions_parameter(argument, parameter)),
        ast::Type::Pointer(inner, _) => type_mentions_parameter(inner, parameter),
        ast::Type::Array { element, .. } => type_mentions_parameter(element, parameter),
        ast::Type::Struct { fields, .. } => fields
            .iter()
            .any(|field| type_mentions_parameter(&field.ty, parameter)),
        ast::Type::Interface { methods, .. } => methods.iter().any(|method| {
            method
                .signature
                .params
                .iter()
                .any(|parameter_ty| type_mentions_parameter(&parameter_ty.ty, parameter))
                || method
                    .signature
                    .results
                    .iter()
                    .any(|result| type_mentions_parameter(result, parameter))
        }),
        ast::Type::Func { signature, .. } => {
            signature
                .params
                .iter()
                .any(|parameter_ty| type_mentions_parameter(&parameter_ty.ty, parameter))
                || signature
                    .results
                    .iter()
                    .any(|result| type_mentions_parameter(result, parameter))
        }
        ast::Type::Resolved(_, _) => false,
    }
}
