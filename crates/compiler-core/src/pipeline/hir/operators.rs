//! Operator symbol identities and declaration-level type helpers.

use crate::ir::ast;

pub(super) fn canonical_operator_symbol(
    package_path: &str,
    operator: ast::OperatorName,
    parameters: &[crate::semantic::TypeId],
    types: &crate::semantic::TypeTable,
    universe: &[super::types::TypePackage],
) -> String {
    let package = if package_path == "." {
        "main"
    } else {
        package_path
    };
    let name = operator_symbol_name(operator);
    let parameters = super::identity::arguments(parameters, types, universe);
    format!(
        "{}:{package}.\u{1f}operator.{name}.{parameters};",
        package.len()
    )
}

pub(super) fn canonical_generic_operator_symbol(
    package_path: &str,
    operator: ast::OperatorName,
    parameters: &[crate::semantic::TypeId],
    arguments: &[crate::semantic::TypeId],
    types: &crate::semantic::TypeTable,
    universe: &[super::types::TypePackage],
) -> String {
    let package = if package_path == "." {
        "main"
    } else {
        package_path
    };
    let name = operator_symbol_name(operator);
    let parameters = super::identity::arguments(parameters, types, universe);
    let arguments = super::identity::arguments(arguments, types, universe);
    format!(
        "{}:{package}.\u{1f}generic_operator.{name}.{parameters}.{arguments};",
        package.len()
    )
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
