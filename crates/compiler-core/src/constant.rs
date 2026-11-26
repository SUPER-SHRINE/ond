use crate::{
    ir::{
        ast::{BinaryOp as B, UnaryOp as U},
        hir::{Expr, ExprKind},
    },
    semantic::{TypeId, TypeKind, TypeTable},
};

use num_bigint::BigInt;
use num_traits::{FromPrimitive, ToPrimitive, Zero};

enum Value {
    Int(BigInt),
    Bool(bool),
    Float(f32),
    Double(f64),
    Fixed(i128),
    Nil,
    Aggregate(Vec<(u32, Expr)>),
}

/// Eligibility is checked structurally, including short-circuited operands.
pub(crate) fn is_constant(expr: &Expr, types: &TypeTable) -> bool {
    match &expr.kind {
        ExprKind::Zero
        | ExprKind::Integer(_)
        | ExprKind::Float32(_)
        | ExprKind::Float64(_)
        | ExprKind::Fixed(_)
        | ExprKind::Bool(_)
        | ExprKind::Nil => true,
        ExprKind::Composite(values) => values.iter().all(|(_, value)| is_constant(value, types)),
        ExprKind::Field { base, .. } => is_constant(base, types),
        ExprKind::Index { base, index } => is_constant(base, types) && is_constant(index, types),
        ExprKind::EvaluatedLength { value, .. } => is_constant(value, types),
        ExprKind::Unary { operand, .. } => is_constant(operand, types),
        ExprKind::Binary { lhs, rhs, .. } => is_constant(lhs, types) && is_constant(rhs, types),
        ExprKind::Cast { value } => {
            !matches!(types.underlying_kind(expr.ty), Some(TypeKind::Pointer(_)))
                && is_constant(value, types)
        }
        _ => false,
    }
}

pub(crate) fn evaluate(expr: &Expr, types: &TypeTable) -> Result<Expr, String> {
    if !is_constant(expr, types) {
        return Err("constant initializer requires a compile-time value".into());
    }
    let value = eval(expr, types)?;
    let kind = match value {
        Value::Int(value) => {
            let (min, max) = integer_range(types, expr.ty).ok_or("constant is not an integer")?;
            if value < BigInt::from(min) || value > BigInt::from(max) {
                return Err(if matches!(expr.kind, ExprKind::Integer(_)) {
                    "integer literal out of range for its type"
                } else {
                    "constant is out of range for its declared type"
                }
                .into());
            }
            ExprKind::Integer(if min < 0 {
                value.to_i64().unwrap() as u64
            } else {
                value.to_u64().unwrap()
            })
        }
        Value::Bool(value) => ExprKind::Bool(value),
        Value::Float(value) => ExprKind::Float32(if value.is_nan() {
            0x7fc00000
        } else {
            value.to_bits()
        }),
        Value::Double(value) => ExprKind::Float64(if value.is_nan() {
            0x7ff8000000000000
        } else {
            value.to_bits()
        }),
        Value::Fixed(raw) => {
            ExprKind::Fixed(fixed_bits(raw, types.fixed_fraction(expr.ty).unwrap())?)
        }
        Value::Nil => ExprKind::Nil,
        Value::Aggregate(values) => ExprKind::Composite(values),
    };
    Ok(Expr {
        kind,
        ty: expr.ty,
        span: expr.span,
    })
}

pub(crate) fn integer_range(types: &TypeTable, ty: TypeId) -> Option<(i128, i128)> {
    Some(match types.underlying_kind(ty)? {
        TypeKind::U8 => (0, 255),
        TypeKind::I8 => (-128, 127),
        TypeKind::U16 => (0, 65535),
        TypeKind::I16 => (-32768, 32767),
        TypeKind::U32 => (0, u32::MAX as i128),
        TypeKind::I32 => (i32::MIN as i128, i32::MAX as i128),
        TypeKind::U64 => (0, u64::MAX as i128),
        TypeKind::I64 => (i64::MIN as i128, i64::MAX as i128),
        _ => return None,
    })
}

fn eval(expr: &Expr, types: &TypeTable) -> Result<Value, String> {
    let value = eval_inner(expr, types)?;
    Ok(match value {
        Value::Float(value) if value.is_nan() => Value::Float(f32::from_bits(0x7fc00000)),
        value => value,
    })
}

fn project_integer(value: BigInt, types: &TypeTable, ty: TypeId) -> Result<BigInt, String> {
    let (min, max) = integer_range(types, ty).ok_or("invalid bitwise operand")?;
    let modulus = BigInt::from(max - min + 1);
    let bits = ((value % &modulus) + &modulus) % &modulus;
    Ok(if min < 0 && bits > BigInt::from(max) {
        bits - modulus
    } else {
        bits
    })
}

fn eval_inner(expr: &Expr, types: &TypeTable) -> Result<Value, String> {
    Ok(match &expr.kind {
        ExprKind::Integer(bits) => Value::Int(BigInt::from(if types.is_signed_integer(expr.ty) {
            *bits as i64 as i128
        } else {
            *bits as i128
        })),
        ExprKind::Bool(value) => Value::Bool(*value),
        ExprKind::Float32(bits) => Value::Float(f32::from_bits(*bits)),
        ExprKind::Float64(bits) => Value::Double(f64::from_bits(*bits)),
        ExprKind::Fixed(bits) => Value::Fixed(if types.fixed_fraction(expr.ty) == Some(16) {
            *bits as i32 as i128
        } else {
            *bits as i64 as i128
        }),
        ExprKind::Nil => Value::Nil,
        ExprKind::Zero => match types.underlying_kind(expr.ty) {
            Some(TypeKind::Bool) => Value::Bool(false),
            Some(TypeKind::F32) => Value::Float(0.0),
            Some(TypeKind::F64) => Value::Double(0.0),
            Some(TypeKind::Q16 | TypeKind::Q32) => Value::Fixed(0),
            Some(TypeKind::Pointer(_) | TypeKind::Function(_)) => Value::Nil,
            Some(TypeKind::Array { .. } | TypeKind::Struct(_)) => Value::Aggregate(Vec::new()),
            _ if types.is_integer(expr.ty) => Value::Int(BigInt::zero()),
            _ => return Err("invalid constant zero type".into()),
        },
        ExprKind::Composite(values) => Value::Aggregate(
            values
                .iter()
                .map(|(index, value)| Ok((*index, evaluate(value, types)?)))
                .collect::<Result<_, String>>()?,
        ),
        ExprKind::Field { base, index } => member(base, *index, expr, types)?,
        ExprKind::Index { base, index } => {
            let Value::Int(index) = eval(index, types)? else {
                return Err("constant index requires an integer".into());
            };
            let index = index.to_u32().ok_or("constant index out of bounds")?;
            match types.underlying_kind(base.ty) {
                Some(TypeKind::Array { length, .. }) => {
                    if index >= *length {
                        return Err("constant index out of bounds".into());
                    }
                    member(base, index, expr, types)?
                }
                _ => return Err("constant index requires an array".into()),
            }
        }
        ExprKind::EvaluatedLength { value, length } => {
            let _ = eval(value, types)?;
            Value::Int(BigInt::from(*length))
        }
        ExprKind::Unary { op, operand } => match (op, eval(operand, types)?) {
            (U::Plus, value) => value,
            (U::Minus, Value::Int(value)) => Value::Int(-value),
            (U::Minus, Value::Float(value)) => Value::Float(-value),
            (U::Minus, Value::Double(value)) => Value::Double(-value),
            (U::Minus, Value::Fixed(value)) => {
                checked_fixed(-value, types.fixed_fraction(expr.ty).unwrap())?
            }
            (U::Not, Value::Bool(value)) => Value::Bool(!value),
            (U::BitNot, Value::Int(value)) => Value::Int(project_integer(!value, types, expr.ty)?),
            _ => return Err("invalid constant unary operand".into()),
        },
        ExprKind::Binary { op, lhs, rhs } => {
            let left = eval(lhs, types)?;
            if matches!((&left, op), (Value::Bool(false), B::AndAnd)) {
                return Ok(Value::Bool(false));
            }
            if matches!((&left, op), (Value::Bool(true), B::OrOr)) {
                return Ok(Value::Bool(true));
            }
            match (left, eval(rhs, types)?) {
                (Value::Int(a), Value::Int(b)) => match op {
                    B::Eq => Value::Bool(a == b),
                    B::Ne => Value::Bool(a != b),
                    B::Lt => Value::Bool(a < b),
                    B::Le => Value::Bool(a <= b),
                    B::Gt => Value::Bool(a > b),
                    B::Ge => Value::Bool(a >= b),
                    _ => Value::Int(match op {
                        B::Add => a + b,
                        B::Sub => a - b,
                        B::Mul => a * b,
                        B::Div | B::Rem => {
                            if b.is_zero() {
                                return Err("division by zero in constant expression".into());
                            }
                            if *op == B::Div
                                && integer_range(types, lhs.ty).is_some_and(|(min, _)| {
                                    min < 0 && a == BigInt::from(min) && b == BigInt::from(-1)
                                })
                            {
                                return Err(
                                    "signed division overflow in constant expression".into()
                                );
                            }
                            if *op == B::Div { a / b } else { a % b }
                        }
                        B::BitAnd => project_integer(a & b, types, expr.ty)?,
                        B::BitOr => project_integer(a | b, types, expr.ty)?,
                        B::BitXor => project_integer(a ^ b, types, expr.ty)?,
                        B::BitAndNot => project_integer(a & !b, types, expr.ty)?,
                        B::ShiftLeft | B::ShiftRight => {
                            let count = b
                                .to_u32()
                                .ok_or("constant shift count is out of range for u32")?;
                            let width = match types.underlying_kind(lhs.ty) {
                                Some(TypeKind::U8 | TypeKind::I8) => 8,
                                Some(TypeKind::U16 | TypeKind::I16) => 16,
                                Some(TypeKind::U64 | TypeKind::I64) => 64,
                                _ => 32,
                            };
                            let count = (count % width) as usize;
                            if *op == B::ShiftLeft {
                                a << count
                            } else {
                                a >> count
                            }
                        }
                        _ => return Err("invalid integer constant operator".into()),
                    }),
                },
                (Value::Nil, Value::Nil) => Value::Bool(match op {
                    B::Eq => true,
                    B::Ne => false,
                    _ => return Err("invalid nil constant operator".into()),
                }),
                (Value::Bool(a), Value::Bool(b)) => Value::Bool(match op {
                    B::AndAnd => a && b,
                    B::OrOr => a || b,
                    B::Eq => a == b,
                    B::Ne => a != b,
                    _ => return Err("invalid boolean constant operator".into()),
                }),
                (Value::Float(a), Value::Float(b)) => match op {
                    B::Eq => Value::Bool(a == b),
                    B::Ne => Value::Bool(a != b),
                    B::Lt => Value::Bool(a < b),
                    B::Le => Value::Bool(a <= b),
                    B::Gt => Value::Bool(a > b),
                    B::Ge => Value::Bool(a >= b),
                    _ => Value::Float(match op {
                        B::Add => a + b,
                        B::Sub => a - b,
                        B::Mul => a * b,
                        B::Div => a / b,
                        _ => return Err("invalid float constant operator".into()),
                    }),
                },
                (Value::Double(a), Value::Double(b)) => match op {
                    B::Eq => Value::Bool(a == b),
                    B::Ne => Value::Bool(a != b),
                    B::Lt => Value::Bool(a < b),
                    B::Le => Value::Bool(a <= b),
                    B::Gt => Value::Bool(a > b),
                    B::Ge => Value::Bool(a >= b),
                    _ => Value::Double(match op {
                        B::Add => a + b,
                        B::Sub => a - b,
                        B::Mul => a * b,
                        B::Div => a / b,
                        _ => return Err("invalid float constant operator".into()),
                    }),
                },
                (Value::Fixed(a), Value::Fixed(b)) => {
                    fixed_binary(*op, a, b, types.fixed_fraction(lhs.ty).unwrap())?
                }
                _ => return Err("constant operand type mismatch".into()),
            }
        }
        ExprKind::Cast { value } => {
            let source_ty = value.ty;
            let value = eval(value, types)?;
            let converted = match (value, types.underlying_kind(expr.ty)) {
                (Value::Int(value), Some(TypeKind::F32)) => Value::Float(
                    value
                        .to_f32()
                        .filter(|v| v.is_finite())
                        .ok_or("constant float conversion overflow")?,
                ),
                (Value::Int(value), Some(TypeKind::F64)) => Value::Double(
                    value
                        .to_f64()
                        .filter(|v| v.is_finite())
                        .ok_or("constant float conversion overflow")?,
                ),
                (Value::Float(value), Some(TypeKind::F64)) => Value::Double(f64::from(value)),
                (Value::Double(value), Some(TypeKind::F32)) => {
                    let result = value as f32;
                    if value.is_finite() && !result.is_finite() {
                        return Err("constant cast is out of range".into());
                    }
                    Value::Float(result)
                }
                (Value::Int(value), Some(TypeKind::Q16 | TypeKind::Q32)) => {
                    let fraction = types.fixed_fraction(expr.ty).unwrap();
                    let raw = (value << fraction)
                        .to_i128()
                        .ok_or("constant cast is out of range")?;
                    checked_fixed(raw, fraction)?
                }
                (Value::Fixed(raw), Some(TypeKind::Q16 | TypeKind::Q32)) => {
                    let from = types.fixed_fraction(source_ty).unwrap();
                    let to = types.fixed_fraction(expr.ty).unwrap();
                    let raw = if to >= from {
                        raw << (to - from)
                    } else {
                        round_ratio(raw, 1 << (from - to))
                    };
                    checked_fixed(raw, to)?
                }
                (Value::Fixed(raw), Some(TypeKind::F32)) => Value::Float(
                    (raw as i64 as f32)
                        * 2.0f32.powi(-(types.fixed_fraction(source_ty).unwrap() as i32)),
                ),
                (Value::Fixed(raw), Some(TypeKind::F64)) => Value::Double(
                    (raw as i64 as f64)
                        * 2.0f64.powi(-(types.fixed_fraction(source_ty).unwrap() as i32)),
                ),
                (Value::Fixed(raw), _) if types.is_integer(expr.ty) => Value::Int(BigInt::from(
                    raw / (1 << types.fixed_fraction(source_ty).unwrap()),
                )),
                (Value::Float(value), Some(TypeKind::Q16 | TypeKind::Q32)) => {
                    float_to_fixed(f64::from(value), types.fixed_fraction(expr.ty).unwrap())?
                }
                (Value::Double(value), Some(TypeKind::Q16 | TypeKind::Q32)) => {
                    float_to_fixed(value, types.fixed_fraction(expr.ty).unwrap())?
                }
                (Value::Double(value), _) if types.is_integer(expr.ty) => Value::Int(
                    BigInt::from_f64(value.trunc())
                        .ok_or("invalid float to integer constant conversion")?,
                ),
                (Value::Float(value), _) if types.is_integer(expr.ty) => {
                    if !value.is_finite() {
                        return Err("invalid float to integer constant conversion".into());
                    }
                    Value::Int(
                        BigInt::from_f32(value.trunc())
                            .ok_or("invalid float to integer constant conversion")?,
                    )
                }
                (value, _) => value,
            };
            if let Value::Int(ref value) = converted {
                let (min, max) =
                    integer_range(types, expr.ty).ok_or("unsupported constant cast")?;
                if value < &BigInt::from(min) || value > &BigInt::from(max) {
                    return Err("constant cast is out of range".into());
                }
            }
            converted
        }
        _ => return Err("constant initializer requires a compile-time value".into()),
    })
}

/// Preserve sparse aggregate representation, including enormous zero-filled arrays.
fn member(base: &Expr, index: u32, result: &Expr, types: &TypeTable) -> Result<Value, String> {
    let Value::Aggregate(values) = eval(base, types)? else {
        return Err("constant member requires an aggregate".into());
    };
    if let Some((_, value)) = values.iter().find(|(key, _)| *key == index) {
        eval(value, types)
    } else {
        eval(
            &Expr {
                kind: ExprKind::Zero,
                ty: result.ty,
                span: result.span,
            },
            types,
        )
    }
}

fn round_ratio(numerator: i128, denominator: i128) -> i128 {
    let quotient = numerator / denominator;
    let remainder = (numerator % denominator).abs();
    let divisor = denominator.abs();
    let other_half = divisor - remainder;
    if remainder > other_half || (remainder == other_half && quotient & 1 != 0) {
        quotient + numerator.signum() * denominator.signum()
    } else {
        quotient
    }
}

pub(crate) fn fixed_bits(raw: i128, fraction: u32) -> Result<u64, &'static str> {
    if fraction == 16 {
        i32::try_from(raw).map(|v| u64::from(v as u32))
    } else {
        i64::try_from(raw).map(|v| v as u64)
    }
    .map_err(|_| "fixed-point value out of range")
}

fn checked_fixed(raw: i128, fraction: u32) -> Result<Value, &'static str> {
    fixed_bits(raw, fraction)?;
    Ok(Value::Fixed(raw))
}

fn float_to_fixed(value: f64, fraction: u32) -> Result<Value, &'static str> {
    let raw = (value * 2.0f64.powi(fraction as i32)).round_ties_even();
    let limit = 2.0f64.powi((2 * fraction - 1) as i32);
    if !(raw >= -limit && raw < limit) {
        return Err("invalid float to fixed-point conversion");
    }
    checked_fixed(raw as i128, fraction)
}

fn fixed_binary(op: B, a: i128, b: i128, fraction: u32) -> Result<Value, &'static str> {
    let scale = 1i128 << fraction;
    let raw = match op {
        B::Eq => return Ok(Value::Bool(a == b)),
        B::Ne => return Ok(Value::Bool(a != b)),
        B::Lt => return Ok(Value::Bool(a < b)),
        B::Le => return Ok(Value::Bool(a <= b)),
        B::Gt => return Ok(Value::Bool(a > b)),
        B::Ge => return Ok(Value::Bool(a >= b)),
        B::Add => a + b,
        B::Sub => a - b,
        B::Mul => round_ratio(a * b, scale),
        B::Div | B::Rem if b == 0 => return Err("division by zero in constant expression"),
        B::Div => round_ratio(a * scale, b),
        B::Rem => a % b,
        _ => return Err("invalid fixed-point constant operator"),
    };
    checked_fixed(raw, fraction)
}
