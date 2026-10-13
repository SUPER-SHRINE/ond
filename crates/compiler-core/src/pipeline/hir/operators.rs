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
