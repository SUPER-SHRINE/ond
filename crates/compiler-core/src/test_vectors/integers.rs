use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultValue {
    Integer(i128),
    Fault,
}
#[derive(Debug)]
pub struct Vector {
    pub id: String,
    pub spec: &'static str,
    pub ty: &'static str,
    pub source_ty: &'static str,
    pub op: &'static str,
    pub a: i128,
    pub b: i128,
    pub expected: ResultValue,
}
#[cfg(test)]
fn wrap(t: &Integer, n: i128) -> i128 {
    let modulus = 1i128 << t.bits;
    let bits = n.rem_euclid(modulus);
    if t.min < 0 && bits >= modulus / 2 {
        bits - modulus
    } else {
        bits
    }
}
#[cfg(test)]
fn oracle(v: &Vector) -> ResultValue {
    let t = INTEGERS.iter().find(|t| t.name == v.ty).unwrap();
    let (a, b) = (v.a, v.b);
    if matches!(v.op, "/" | "%") && (b == 0 || v.op == "/" && t.min < 0 && a == t.min && b == -1) {
        return ResultValue::Fault;
    }
    let n = match v.op {
        "+" => a + b,
        "-" => a - b,
        "*" => a * b,
        "/" => a / b,
        "%" => a % b,
        "&" => a & b,
        "|" => a | b,
        "^" => a ^ b,
        "&^" => a & !b,
        "<<" => a << (b % i128::from(t.bits)),
        ">>" => a >> (b % i128::from(t.bits)),
        "==" => return ResultValue::Integer(i128::from(a == b)),
        "!=" => return ResultValue::Integer(i128::from(a != b)),
        "<" => return ResultValue::Integer(i128::from(a < b)),
        "<=" => return ResultValue::Integer(i128::from(a <= b)),
        ">" => return ResultValue::Integer(i128::from(a > b)),
        ">=" => return ResultValue::Integer(i128::from(a >= b)),
        "unary+" => a,
        "unary-" => -a,
        "unary^" => !a,
        "cast" => a,
        _ => panic!("unknown vector operator"),
    };
    ResultValue::Integer(wrap(t, n))
}

pub fn rows() -> Vec<Vector> {
    use ResultValue::{Fault, Integer as V};
    let mut rows = Vec::new();
    for t in INTEGERS {
        let mut add = |tag: &str, op, a, b, expected| {
            let spec = match op {
                "<<" | ">>" => "NUM-03",
                "/" | "%" => "NUM-04",
                _ => "NUM-05",
            };
            rows.push(Vector {
                id: format!("{spec}.runtime.{}.{tag}", t.name),
                spec,
                ty: t.name,
                source_ty: t.name,
                op,
                a,
                b,
                expected,
            })
        };
        for (tag, op, result) in [
            ("add", "+", 9),
            ("sub", "-", 3),
            ("mul", "*", 18),
            ("div", "/", 2),
            ("rem", "%", 0),
            ("and", "&", 2),
            ("or", "|", 7),
            ("xor", "^", 5),
            ("andnot", "&^", 4),
            ("eq", "==", 0),
            ("ne", "!=", 1),
            ("lt", "<", 0),
            ("le", "<=", 0),
            ("gt", ">", 1),
            ("ge", ">=", 1),
        ] {
            add(tag, op, 6, 3, V(result));
        }
        add("wrap_add", "+", t.max, 1, V(t.min));
        add("wrap_sub", "-", t.min, 1, V(t.max));
        add(
            "wrap_mul",
            "*",
            t.max,
            2,
            V(if t.min < 0 { -2 } else { t.max - 1 }),
        );
        add("positive", "unary+", t.max, 0, V(t.max));
        add(
            "negative",
            "unary-",
            if t.min < 0 { t.min } else { 1 },
            0,
            V(if t.min < 0 { t.min } else { t.max }),
        );
        add("not", "unary^", 0, 0, V(if t.min < 0 { -1 } else { t.max }));
        for (tag, op, expected) in [
            ("eq", "==", 0),
            ("ne", "!=", 1),
            ("lt", "<", 1),
            ("le", "<=", 1),
            ("gt", ">", 0),
            ("ge", ">=", 0),
        ] {
            add(&format!("boundary_{tag}"), op, t.min, t.max, V(expected));
        }
        for k in t.counts {
            let effective = k % t.bits;
            let left = if t.min < 0 && effective == t.bits - 1 {
                t.min
            } else {
                1i128 << effective
            };
            add(&format!("left_{k}"), "<<", 1, k.into(), V(left));
            add(
                &format!("right_{k}"),
                ">>",
                t.min,
                k.into(),
                V(t.min >> effective),
            );
        }
        add("div_zero", "/", 1, 0, Fault);
        add("rem_zero", "%", 1, 0, Fault);
        if t.min < 0 {
            add("div_min", "/", t.min, -1, Fault);
            add("rem_min", "%", t.min, -1, V(0));
            for (tag, a, b, q, r) in [
                ("pp", 7, 3, 2, 1),
                ("np", -7, 3, -2, -1),
                ("pn", 7, -3, -2, 1),
                ("nn", -7, -3, 2, -1),
            ] {
                add(&format!("div_{tag}"), "/", a, b, V(q));
                add(&format!("rem_{tag}"), "%", a, b, V(r));
            }
        }
    }
    // All 36 integer conversion directions, narrowing/widening/sign changes.
    // Golden values are obtained from fixed-width source bit strings, not the
    // compiler's conversion helper or the oracle's modulo implementation.
    for from in INTEGERS {
        for to in INTEGERS {
            for (tag, value) in [("min", from.min), ("max", from.max)] {
                let mask = (1u64 << to.bits) - 1;
                let raw = (value as u64) & mask;
                let expected = if to.min < 0 && raw & (1 << (to.bits - 1)) != 0 {
                    (raw | !mask) as i64 as i128
                } else {
                    raw as i128
                };
                rows.push(Vector {
                    id: format!("CAST-02.runtime.{}.{}.{}", from.name, to.name, tag),
                    spec: "CAST-02",
                    ty: to.name,
                    source_ty: from.name,
                    op: "cast",
                    a: value,
                    b: 0,
                    expected: V(expected),
                });
            }
        }
    }
    rows
}

#[test]
fn runtime_goldens_match_independent_integer_oracle() {
    let rows = rows();
    let mut ids = std::collections::BTreeSet::new();
    for row in &rows {
        assert!(ids.insert(&row.id));
        assert!(row.id.starts_with(row.spec));
        assert_eq!(oracle(row), row.expected, "{row:?}");
    }
    eprintln!(
        "{} runtime integer vectors (independent oracle)",
        rows.len()
    );
}
