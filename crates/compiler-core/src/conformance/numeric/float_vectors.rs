use super::*;
use crate::test_vectors::floats::VECTORS;

fn literal(bits: u32) -> String {
    let sign = if bits >> 31 != 0 { "-" } else { "" };
    let exponent = (bits >> 23) & 255;
    let fraction = bits & 0x7fffff;
    if exponent == 255 {
        assert_eq!(fraction, 0, "NaN inputs have no literal spelling");
        format!("{sign}1.0 / 0.0")
    } else if exponent == 0 {
        format!("{sign}0x{fraction:x}p-149")
    } else {
        format!(
            "{sign}0x{:x}p{}",
            fraction | 0x800000,
            exponent as i32 - 150
        )
    }
}
pub(super) fn rows() -> Vec<Row> {
    VECTORS
        .iter()
        .map(|v| {
            Row::constant(
                format!("FP-02.bits.{}", v.id),
                &["FP-01", "FP-02"],
                &format!(
                    "const A: f32 = {}\nconst B: f32 = {}",
                    literal(v.lhs),
                    literal(v.rhs)
                ),
                &format!("A {} B", v.op),
                "",
                float(v.bits),
            )
        })
        .collect()
}
