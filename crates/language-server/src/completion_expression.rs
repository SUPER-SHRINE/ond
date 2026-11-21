use compiler::ir::ast::{BinaryOp, Expr, Literal, Type, UnaryOp};
use compiler::source::Span;

use super::primitive_type;

pub(super) fn operator_type(expr: &Expr, resolve: impl Fn(&Expr) -> Option<Type>) -> Option<Type> {
    match expr {
        Expr::Unary {
            op, expr: operand, ..
        } => {
            let operand = resolve(operand)?;
            let name = scalar_name(&operand)?;
            let result = match op {
                UnaryOp::Plus | UnaryOp::Minus if is_numeric(name) => name,
                UnaryOp::BitNot if is_integer(name) => name,
                UnaryOp::Not if name == "bool" => "bool",
                _ => return None,
            };
            Some(primitive_type(result, expr.span()))
        }
        Expr::Binary { op, lhs, rhs, .. } => binary_type(*op, lhs, rhs, expr.span(), resolve),
        _ => None,
    }
}

fn binary_type(
    op: BinaryOp,
    lhs: &Expr,
    rhs: &Expr,
    span: Span,
    resolve: impl Fn(&Expr) -> Option<Type>,
) -> Option<Type> {
    use BinaryOp::*;

    if matches!(op, ShiftLeft | ShiftRight) {
        let left = resolve(lhs)?;
        let name = scalar_name(&left)?;
        is_integer(name).then_some(())?;
        let count_matches = if contextual_arithmetic(rhs, None) {
            contextual_arithmetic(rhs, Some("u32"))
        } else {
            scalar_name(&resolve(rhs)?)? == "u32"
        };
        count_matches.then_some(())?;
        return Some(primitive_type(name, span));
    }

    let right_is_contextual = contextual_arithmetic(rhs, None);
    let (known, other, contextual) = if !right_is_contextual && contextual_arithmetic(lhs, None) {
        (resolve(rhs)?, lhs, true)
    } else {
        (resolve(lhs)?, rhs, right_is_contextual)
    };
    let common = scalar_name(&known)?;
    let matches = if contextual {
        contextual_arithmetic(other, Some(common))
    } else {
        scalar_name(&resolve(other)?)? == common
    };
    matches.then_some(())?;
    let result = match op {
        op if accepts_arithmetic(op, common) => common,
        Eq | Ne if common == "bool" || is_numeric(common) => "bool",
        Lt | Le | Gt | Ge if is_numeric(common) => "bool",
        AndAnd | OrOr if common == "bool" => "bool",
        _ => return None,
    };
    Some(primitive_type(result, span))
}

fn accepts_arithmetic(op: BinaryOp, name: &str) -> bool {
    use BinaryOp::*;
    match op {
        Add | Sub | Mul | Div => is_numeric(name),
        Rem => is_integer(name) || matches!(name, "q16" | "q32"),
        BitAnd | BitOr | BitXor | BitAndNot | ShiftLeft | ShiftRight => is_integer(name),
        _ => false,
    }
}

fn scalar_name(ty: &Type) -> Option<&str> {
    let Type::Named(path) = ty else {
        return None;
    };
    let [name] = path.segments.as_slice() else {
        return None;
    };
    (name.name == "bool" || is_numeric(&name.name)).then_some(&name.name)
}

fn is_numeric(name: &str) -> bool {
    is_integer(name) || matches!(name, "f32" | "f64" | "q16" | "q32")
}

fn is_integer(name: &str) -> bool {
    matches!(
        name,
        "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64"
    )
}
fn contextual_arithmetic(expr: &Expr, name: Option<&str>) -> bool {
    match expr {
        Expr::Literal(Literal::Int(..)) => name.is_none_or(is_numeric),
        Expr::Literal(Literal::Float(..)) => {
            name.is_none_or(|name| matches!(name, "f32" | "f64" | "q16" | "q32"))
        }
        Expr::Unary {
            op: UnaryOp::Plus | UnaryOp::Minus,
            expr,
            ..
        } => contextual_arithmetic(expr, name),
        Expr::Unary {
            op: UnaryOp::BitNot,
            expr,
            ..
        } => name.is_none_or(is_integer) && contextual_arithmetic(expr, name),
        Expr::Binary { op, lhs, rhs, .. } => {
            accepts_arithmetic(*op, name.unwrap_or("i32"))
                && contextual_arithmetic(lhs, name)
                && contextual_arithmetic(
                    rhs,
                    if matches!(op, BinaryOp::ShiftLeft | BinaryOp::ShiftRight) {
                        Some("u32")
                    } else {
                        name
                    },
                )
        }
        _ => false,
    }
}
