//! Numeric specification data: constant values and dynamic MIR are distinct checks.
mod casts;
mod float_vectors;
mod floats;
mod integers;
mod runtime;
use super::*;
use crate::test_vectors::integers as vectors;

use crate::test_vectors::INTEGERS;
pub(super) enum Outcome {
    Value(&'static str, Scalar),
    Error { message: &'static str, span: String },
    Mir(fn(&mir::Project) -> Result<(), String>),
}
pub(super) struct Row {
    id: String,
    specs: &'static [&'static str],
    source: String,
    expected: Outcome,
}
impl Row {
    fn constant(
        id: String,
        specs: &'static [&'static str],
        setup: &str,
        expression: &str,
        annotation: &str,
        expected: Outcome,
    ) -> Self {
        let annotation = if annotation.is_empty() {
            String::new()
        } else {
            format!(": {annotation}")
        };
        Self {
            id,
            specs,
            source: format!(
                "package main\n{setup}\nconst X{annotation} = {expression}\nfunc main() {{}}\n"
            ),
            expected,
        }
    }
    fn run(&self) {
        let files = [SourceFile {
            path: "main.ond",
            text: &self.source,
        }];
        let checks;
        let diagnostics;
        let expected = match &self.expected {
            Outcome::Value(ty, value) => {
                checks = [Check::HirConstant {
                    package: ".",
                    name: "X",
                    ty,
                    value: *value,
                }];
                Expected::Accept(&checks)
            }
            Outcome::Error { message, span } => {
                diagnostics = [ExpectedDiagnostic {
                    severity: Severity::Error,
                    message: Message::Contains(message),
                    location: Location::Source {
                        file: "main.ond",
                        text: span,
                        occurrence: self.source.match_indices(span).count().saturating_sub(1),
                    },
                }];
                Expected::Reject(&diagnostics)
            }
            Outcome::Mir(verify) => {
                checks = [Check::Mir {
                    description: "numeric opcode/type/guard contract, not backend execution",
                    verify: *verify,
                }];
                Expected::Accept(&checks)
            }
        };
        run(&Case {
            id: &self.id,
            specs: self.specs,
            layer: Layer::Core,
            manifest: true,
            files: &files,
            expected,
        })
        .unwrap_or_else(|e| panic!("{e}\nsource:\n{}", self.source));
    }
}
fn integer(value: i128, ty: &'static str) -> Outcome {
    Outcome::Value(ty, Scalar::Integer(value as u64))
}
fn boolean(value: bool) -> Outcome {
    Outcome::Value("bool", Scalar::Bool(value))
}
fn float(bits: u32) -> Outcome {
    Outcome::Value("f32", Scalar::Float32(bits))
}
fn error(message: &'static str, span: impl Into<String>) -> Outcome {
    Outcome::Error {
        message,
        span: span.into(),
    }
}
fn run_rows(rows: Vec<Row>) {
    for row in rows {
        row.run();
    }
}
pub(super) fn ids() -> Vec<String> {
    [
        integers::rows(),
        casts::rows(),
        floats::rows(),
        runtime::rows(),
        float_vectors::rows(),
    ]
    .into_iter()
    .flatten()
    .map(|row| row.id)
    .collect()
}
#[test]
fn integer_constants() {
    run_rows(integers::rows());
}
#[test]
fn numeric_casts() {
    run_rows(casts::rows());
}
#[test]
fn binary32_constants() {
    run_rows(floats::rows());
}
#[test]
fn dynamic_mir_contracts() {
    run_rows(runtime::rows());
}
#[test]
fn floating_point_input_bit_vectors() {
    run_rows(float_vectors::rows());
}

#[test]
fn matrix_inventory() {
    assert_eq!(INTEGERS.len(), 6);
    for (name, rows, count) in [
        ("integers", integers::rows(), 392),
        ("casts", casts::rows(), 264),
        ("floats", floats::rows(), 69),
        ("float bit inputs", float_vectors::rows(), 25),
        ("dynamic projects", runtime::rows(), 1),
    ] {
        assert_eq!(
            rows.len(),
            count,
            "{name}: update the inventory when adding/removing cases"
        );
    }
    assert_eq!(ids().len(), 751);
    assert_eq!(vectors::rows().len(), 336);
}
